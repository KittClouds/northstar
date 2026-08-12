use super::{
    LevelFamily, LevelId, LevelRole, NodeGenealogy, PriceSpace, RoleSide, StructuralCorridor,
    StructuralGraphSnapshot, StructuralNode, StructuralNodeId, StructuralObject,
};
use crate::data_plane::ids::InstrumentId;
use smallvec::SmallVec;
use std::cmp::Reverse;
use std::sync::Arc;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct NodeMergeSpec {
    /// Fraction of ATR accepted as merge distance, in parts per million.
    pub atr_fraction_ppm: u32,
    pub noise_multiplier: u32,
    pub tick_multiplier: u32,
}

impl Default for NodeMergeSpec {
    fn default() -> Self {
        Self {
            atr_fraction_ppm: 80_000,
            noise_multiplier: 1,
            tick_multiplier: 2,
        }
    }
}

impl NodeMergeSpec {
    pub fn threshold(self, atr: i64, noise: i64, tick: i64) -> i64 {
        let atr_component = ((atr.max(0) as i128 * self.atr_fraction_ppm as i128) / 1_000_000)
            .min(i64::MAX as i128) as i64;
        let noise_component = noise.max(0).saturating_mul(self.noise_multiplier as i64);
        let tick_component = tick.max(0).saturating_mul(self.tick_multiplier as i64);
        atr_component.max(noise_component).max(tick_component)
    }
}

struct Candidate<'a> {
    members: SmallVec<[&'a StructuralObject; 8]>,
    lower: i64,
    upper: i64,
    center: i64,
}

impl<'a> Candidate<'a> {
    fn new(object: &'a StructuralObject) -> Self {
        Self {
            members: SmallVec::from_slice(&[object]),
            lower: object.lower(),
            upper: object.upper(),
            center: object.reference_price(),
        }
    }

    fn accepts(&self, object: &StructuralObject, threshold: i64) -> bool {
        geometry_distance(self.lower, self.upper, object.lower(), object.upper()) <= threshold
            && self.members.iter().all(|member| compatible(member, object))
    }

    fn push(&mut self, object: &'a StructuralObject) {
        self.members.push(object);
        self.lower = self.lower.min(object.lower());
        self.upper = self.upper.max(object.upper());
        self.center = self.lower + (self.upper - self.lower) / 2;
    }

    fn contributor_ids(&self) -> SmallVec<[LevelId; 8]> {
        let mut ids: SmallVec<[LevelId; 8]> = self.members.iter().map(|item| item.id()).collect();
        ids.sort_unstable();
        ids.dedup();
        ids
    }
}

pub struct NodeBook {
    generation: u64,
    next_id: u64,
    previous: Vec<StructuralNode>,
    snapshot: Arc<StructuralGraphSnapshot>,
}

impl Default for NodeBook {
    fn default() -> Self {
        Self::new()
    }
}

impl NodeBook {
    pub fn new() -> Self {
        Self {
            generation: 0,
            next_id: 1,
            previous: Vec::new(),
            snapshot: Arc::new(StructuralGraphSnapshot {
                generation: 0,
                nodes: Arc::from([]),
                corridors: Arc::from([]),
                genealogy: Arc::from([]),
            }),
        }
    }

    #[allow(clippy::too_many_arguments)]
    pub fn rebuild(
        &mut self,
        objects: &[StructuralObject],
        instrument: InstrumentId,
        price_space: PriceSpace,
        now_ns: i64,
        atr: i64,
        noise: i64,
        tick: i64,
        spec: NodeMergeSpec,
    ) -> Arc<StructuralGraphSnapshot> {
        let threshold = spec.threshold(atr, noise, tick);
        let mut active: Vec<_> = objects
            .iter()
            .filter(|object| {
                object.is_active()
                    && object.instrument_id() == instrument
                    && object.price_space() == price_space
            })
            .collect();
        active.sort_unstable_by_key(|object| {
            (
                object.lower(),
                object.upper(),
                object.reference_price(),
                object.id(),
            )
        });

        let mut candidates: Vec<Candidate<'_>> = Vec::with_capacity(active.len());
        for object in active {
            let target = candidates
                .iter()
                .enumerate()
                .rev()
                .find_map(|(index, candidate)| {
                    candidate.accepts(object, threshold).then_some(index)
                });
            match target {
                Some(index) => candidates[index].push(object),
                None => candidates.push(Candidate::new(object)),
            }
        }
        candidates
            .sort_unstable_by_key(|candidate| (candidate.center, candidate.lower, candidate.upper));
        let relations = candidate_relations(&candidates, &self.previous, threshold);
        let (mut nodes, genealogy) =
            self.assign_nodes(&candidates, &relations, instrument, price_space, now_ns);
        nodes.sort_unstable_by_key(|node| (node.center, node.lower, node.upper, node.id));
        let corridors = corridors(&nodes, atr);
        self.generation = self.generation.saturating_add(1);
        self.previous.clone_from(&nodes);
        self.snapshot = Arc::new(StructuralGraphSnapshot {
            generation: self.generation,
            nodes: nodes.into(),
            corridors: corridors.into(),
            genealogy: genealogy.into(),
        });
        Arc::clone(&self.snapshot)
    }

    pub fn snapshot(&self) -> Arc<StructuralGraphSnapshot> {
        Arc::clone(&self.snapshot)
    }

    fn assign_nodes(
        &mut self,
        candidates: &[Candidate<'_>],
        relations: &[SmallVec<[usize; 4]>],
        instrument: InstrumentId,
        price_space: PriceSpace,
        now_ns: i64,
    ) -> (Vec<StructuralNode>, Vec<NodeGenealogy>) {
        let mut used_previous = vec![false; self.previous.len()];
        let mut assigned_previous = vec![None; candidates.len()];
        for (candidate_index, related) in relations.iter().enumerate() {
            let candidate = &candidates[candidate_index];
            let mut ranked: Vec<_> = related
                .iter()
                .copied()
                .filter(|index| !used_previous[*index])
                .collect();
            ranked.sort_unstable_by_key(|index| {
                let prior = &self.previous[*index];
                (
                    Reverse(contributor_overlap(candidate, prior)),
                    (candidate.center - prior.center).unsigned_abs(),
                    prior.id,
                )
            });
            if let Some(index) = ranked.first().copied() {
                used_previous[index] = true;
                assigned_previous[candidate_index] = Some(index);
            }
        }

        let mut nodes = Vec::with_capacity(candidates.len());
        for (candidate_index, candidate) in candidates.iter().enumerate() {
            let previous = assigned_previous[candidate_index].map(|index| &self.previous[index]);
            let id = previous.map_or_else(
                || {
                    let id = StructuralNodeId(
                        ((instrument.get() as u64) << 32) | (self.next_id & 0xFFFF_FFFF),
                    );
                    self.next_id = self.next_id.saturating_add(1);
                    id
                },
                |node| node.id,
            );
            nodes.push(node_from_candidate(
                candidate,
                id,
                instrument,
                price_space,
                previous.map_or(now_ns, |node| node.created_at_ns),
                now_ns,
            ));
        }

        let mut genealogy = Vec::new();
        for (index, node) in nodes.iter().enumerate() {
            let related = &relations[index];
            if related.is_empty() {
                genealogy.push(NodeGenealogy::Created(node.id));
            } else if related.len() > 1 {
                let mut from: SmallVec<[StructuralNodeId; 4]> = related
                    .iter()
                    .map(|prior| self.previous[*prior].id)
                    .filter(|id| *id != node.id)
                    .collect();
                from.sort_unstable();
                from.dedup();
                genealogy.push(NodeGenealogy::Merged {
                    into: node.id,
                    from,
                });
            } else {
                genealogy.push(NodeGenealogy::Preserved(node.id));
            }
        }
        for (prior_index, prior) in self.previous.iter().enumerate() {
            let into: SmallVec<[StructuralNodeId; 4]> = relations
                .iter()
                .enumerate()
                .filter_map(|(candidate, related)| {
                    related
                        .contains(&prior_index)
                        .then_some(nodes[candidate].id)
                })
                .collect();
            if into.len() > 1 {
                genealogy.push(NodeGenealogy::Split {
                    from: prior.id,
                    into,
                });
            } else if into.is_empty() {
                genealogy.push(NodeGenealogy::Expired(prior.id));
            }
        }
        (nodes, genealogy)
    }
}

fn candidate_relations(
    candidates: &[Candidate<'_>],
    previous: &[StructuralNode],
    threshold: i64,
) -> Vec<SmallVec<[usize; 4]>> {
    candidates
        .iter()
        .map(|candidate| {
            previous
                .iter()
                .enumerate()
                .filter_map(|(index, node)| {
                    let overlap = contributor_overlap(candidate, node) > 0;
                    let nearby =
                        geometry_distance(candidate.lower, candidate.upper, node.lower, node.upper)
                            <= threshold;
                    (overlap || (nearby && candidate_node_compatible(candidate, node)))
                        .then_some(index)
                })
                .collect()
        })
        .collect()
}

fn node_from_candidate(
    candidate: &Candidate<'_>,
    id: StructuralNodeId,
    instrument: InstrumentId,
    price_space: PriceSpace,
    created_at_ns: i64,
    updated_at_ns: i64,
) -> StructuralNode {
    let mut roles: SmallVec<[LevelRole; 4]> = candidate
        .members
        .iter()
        .map(|object| object.role())
        .collect();
    roles.sort_unstable();
    roles.dedup();
    let mut families: SmallVec<[LevelFamily; 4]> = candidate
        .members
        .iter()
        .map(|object| object.family())
        .collect();
    families.sort_unstable();
    families.dedup();
    StructuralNode {
        id,
        instrument_id: instrument,
        price_space,
        lower: candidate.lower,
        upper: candidate.upper,
        center: candidate.center,
        roles,
        contributors: candidate.contributor_ids(),
        families,
        created_at_ns,
        updated_at_ns,
    }
}

fn compatible(left: &StructuralObject, right: &StructuralObject) -> bool {
    let left_group = left.provenance().exclusive_group;
    let right_group = right.provenance().exclusive_group;
    if left_group != 0 && left_group == right_group {
        return false;
    }
    !matches!(
        (left.role().side(), right.role().side()),
        (RoleSide::Lower, RoleSide::Upper) | (RoleSide::Upper, RoleSide::Lower)
    )
}

fn candidate_node_compatible(candidate: &Candidate<'_>, node: &StructuralNode) -> bool {
    candidate.members.iter().all(|object| {
        node.roles.iter().all(|role| {
            !matches!(
                (object.role().side(), role.side()),
                (RoleSide::Lower, RoleSide::Upper) | (RoleSide::Upper, RoleSide::Lower)
            )
        })
    })
}

fn contributor_overlap(candidate: &Candidate<'_>, node: &StructuralNode) -> usize {
    candidate
        .members
        .iter()
        .filter(|object| node.contributors.contains(&object.id()))
        .count()
}

fn geometry_distance(left_lower: i64, left_upper: i64, right_lower: i64, right_upper: i64) -> i64 {
    if left_upper < right_lower {
        right_lower.saturating_sub(left_upper)
    } else if right_upper < left_lower {
        left_lower.saturating_sub(right_upper)
    } else {
        0
    }
}

fn corridors(nodes: &[StructuralNode], atr: i64) -> Vec<StructuralCorridor> {
    nodes
        .windows(2)
        .map(|pair| {
            let lower_edge = pair[0].upper;
            let upper_edge = pair[1].lower;
            let width = (upper_edge - lower_edge).max(0);
            StructuralCorridor {
                lower_node: pair[0].id,
                upper_node: pair[1].id,
                lower_edge,
                upper_edge,
                width,
                width_atr_ppm: ratio_ppm(width, atr),
            }
        })
        .collect()
}

fn ratio_ppm(value: i64, denominator: i64) -> i64 {
    if denominator <= 0 {
        return 0;
    }
    ((value as i128 * 1_000_000) / denominator as i128).clamp(i64::MIN as i128, i64::MAX as i128)
        as i64
}
