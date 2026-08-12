use super::{MarketLocation, StructuralMarketSnapshot, StructuralMutation, StructuralObject};

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct StructuralFingerprint(pub [u8; 32]);

pub fn fingerprint_snapshot(snapshot: &StructuralMarketSnapshot) -> StructuralFingerprint {
    let mut hasher = blake3::Hasher::new();
    hasher.update(b"northstar-structural-snapshot-v1");
    u32_field(&mut hasher, snapshot.instrument_id.get());
    u8_field(&mut hasher, snapshot.price_space as u8);
    i64_field(&mut hasher, snapshot.as_of_ns);
    u64_field(&mut hasher, snapshot.journal_sequence.get());
    u64_field(&mut hasher, snapshot.derivation_generation);
    i64_field(&mut hasher, snapshot.reference_price);
    u64_field(&mut hasher, snapshot.levels.len() as u64);
    for object in snapshot.levels.iter() {
        object_field(&mut hasher, object);
    }
    u64_field(&mut hasher, snapshot.graph.generation);
    u64_field(&mut hasher, snapshot.graph.nodes.len() as u64);
    for node in snapshot.graph.nodes.iter() {
        u64_field(&mut hasher, node.id.get());
        u32_field(&mut hasher, node.instrument_id.get());
        u8_field(&mut hasher, node.price_space as u8);
        i64_field(&mut hasher, node.lower);
        i64_field(&mut hasher, node.upper);
        i64_field(&mut hasher, node.center);
        i64_field(&mut hasher, node.created_at_ns);
        i64_field(&mut hasher, node.updated_at_ns);
        u64_field(&mut hasher, node.roles.len() as u64);
        for role in &node.roles {
            u8_field(&mut hasher, *role as u8);
        }
        u64_field(&mut hasher, node.families.len() as u64);
        for family in &node.families {
            u8_field(&mut hasher, *family as u8);
        }
        u64_field(&mut hasher, node.contributors.len() as u64);
        for contributor in &node.contributors {
            u64_field(&mut hasher, contributor.get());
        }
    }
    u64_field(&mut hasher, snapshot.graph.corridors.len() as u64);
    for corridor in snapshot.graph.corridors.iter() {
        u64_field(&mut hasher, corridor.lower_node.get());
        u64_field(&mut hasher, corridor.upper_node.get());
        i64_field(&mut hasher, corridor.lower_edge);
        i64_field(&mut hasher, corridor.upper_edge);
        i64_field(&mut hasher, corridor.width);
        i64_field(&mut hasher, corridor.width_atr_ppm);
    }
    u64_field(&mut hasher, snapshot.graph.genealogy.len() as u64);
    for genealogy in snapshot.graph.genealogy.iter() {
        genealogy_field(&mut hasher, genealogy);
    }
    location_field(&mut hasher, snapshot.location.location);
    optional_i64(&mut hasher, snapshot.location.distance_to_lower);
    optional_i64(&mut hasher, snapshot.location.distance_to_upper);
    optional_i64(&mut hasher, snapshot.location.distance_to_lower_atr_ppm);
    optional_i64(&mut hasher, snapshot.location.distance_to_upper_atr_ppm);
    u8_field(&mut hasher, snapshot.quality.price_fresh as u8);
    u8_field(&mut hasher, snapshot.quality.volume_quality as u8);
    u8_field(&mut hasher, snapshot.quality.adaptive_value_available as u8);
    u8_field(&mut hasher, snapshot.quality.volume_profile_available as u8);
    u8_field(&mut hasher, snapshot.quality.tpo_profile_available as u8);
    u8_field(&mut hasher, snapshot.quality.calendar_valid as u8);
    u8_field(&mut hasher, snapshot.quality.gaps_present as u8);
    u8_field(&mut hasher, snapshot.quality.revisions_pending as u8);
    StructuralFingerprint(*hasher.finalize().as_bytes())
}

pub fn fingerprint_mutations(mutations: &[StructuralMutation]) -> StructuralFingerprint {
    let mut hasher = blake3::Hasher::new();
    hasher.update(b"northstar-structural-mutations-v1");
    u64_field(&mut hasher, mutations.len() as u64);
    for mutation in mutations {
        match mutation {
            StructuralMutation::Created(object) => {
                u8_field(&mut hasher, 1);
                object_field(&mut hasher, object);
            }
            StructuralMutation::Updated(object) => {
                u8_field(&mut hasher, 2);
                object_field(&mut hasher, object);
            }
            StructuralMutation::Frozen { id, at_ns } => {
                u8_field(&mut hasher, 3);
                u64_field(&mut hasher, id.get());
                i64_field(&mut hasher, *at_ns);
            }
            StructuralMutation::Superseded { old, new, at_ns } => {
                u8_field(&mut hasher, 4);
                u64_field(&mut hasher, old.get());
                object_field(&mut hasher, new);
                i64_field(&mut hasher, *at_ns);
            }
            StructuralMutation::Expired { id, at_ns } => {
                u8_field(&mut hasher, 5);
                u64_field(&mut hasher, id.get());
                i64_field(&mut hasher, *at_ns);
            }
        }
    }
    StructuralFingerprint(*hasher.finalize().as_bytes())
}

fn object_field(hasher: &mut blake3::Hasher, object: &StructuralObject) {
    match object {
        StructuralObject::Level(level) => {
            u8_field(hasher, 1);
            common_object_fields(
                hasher,
                object,
                level.created_at_ns,
                level.effective_at_ns,
                level.updated_at_ns,
            );
            i64_field(hasher, level.price);
        }
        StructuralObject::Band(band) => {
            u8_field(hasher, 2);
            common_object_fields(
                hasher,
                object,
                band.created_at_ns,
                band.effective_at_ns,
                band.updated_at_ns,
            );
            i64_field(hasher, band.lower);
            i64_field(hasher, band.upper);
            i64_field(hasher, band.reference_price);
        }
    }
}

fn common_object_fields(
    hasher: &mut blake3::Hasher,
    object: &StructuralObject,
    created_at_ns: i64,
    effective_at_ns: i64,
    updated_at_ns: i64,
) {
    u64_field(hasher, object.id().get());
    u32_field(hasher, object.instrument_id().get());
    u8_field(hasher, object.price_space() as u8);
    u16_field(hasher, object.kind() as u16);
    u8_field(hasher, object.family() as u8);
    u8_field(hasher, object.role() as u8);
    u8_field(hasher, object.state() as u8);
    i64_field(hasher, created_at_ns);
    i64_field(hasher, effective_at_ns);
    i64_field(hasher, updated_at_ns);
    let provenance = object.provenance();
    u16_field(hasher, provenance.producer_id.get());
    u64_field(hasher, provenance.first_sequence.get());
    u64_field(hasher, provenance.last_sequence.get());
    u32_field(hasher, provenance.derivation_version.get());
    u32_field(hasher, provenance.calendar_id.get());
    u16_field(hasher, provenance.timeframe.map_or(0, |value| value as u16));
    i64_field(hasher, provenance.epoch_ns);
    u64_field(hasher, provenance.exclusive_group);
    u16_field(hasher, provenance.ordinal);
}

fn location_field(hasher: &mut blake3::Hasher, location: MarketLocation) {
    match location {
        MarketLocation::Unavailable => u8_field(hasher, 0),
        MarketLocation::InsideNode(id) => {
            u8_field(hasher, 1);
            u64_field(hasher, id.get());
        }
        MarketLocation::Between { lower, upper } => {
            u8_field(hasher, 2);
            u64_field(hasher, lower.get());
            u64_field(hasher, upper.get());
        }
        MarketLocation::BelowKnownStructure { nearest } => {
            u8_field(hasher, 3);
            u64_field(hasher, nearest.get());
        }
        MarketLocation::AboveKnownStructure { nearest } => {
            u8_field(hasher, 4);
            u64_field(hasher, nearest.get());
        }
    }
}

fn genealogy_field(hasher: &mut blake3::Hasher, genealogy: &super::NodeGenealogy) {
    match genealogy {
        super::NodeGenealogy::Created(id) => {
            u8_field(hasher, 1);
            u64_field(hasher, id.get());
        }
        super::NodeGenealogy::Preserved(id) => {
            u8_field(hasher, 2);
            u64_field(hasher, id.get());
        }
        super::NodeGenealogy::Merged { into, from } => {
            u8_field(hasher, 3);
            u64_field(hasher, into.get());
            u64_field(hasher, from.len() as u64);
            for id in from {
                u64_field(hasher, id.get());
            }
        }
        super::NodeGenealogy::Split { from, into } => {
            u8_field(hasher, 4);
            u64_field(hasher, from.get());
            u64_field(hasher, into.len() as u64);
            for id in into {
                u64_field(hasher, id.get());
            }
        }
        super::NodeGenealogy::Expired(id) => {
            u8_field(hasher, 5);
            u64_field(hasher, id.get());
        }
    }
}

fn optional_i64(hasher: &mut blake3::Hasher, value: Option<i64>) {
    match value {
        Some(value) => {
            u8_field(hasher, 1);
            i64_field(hasher, value);
        }
        None => u8_field(hasher, 0),
    }
}

#[inline]
fn u8_field(hasher: &mut blake3::Hasher, value: u8) {
    hasher.update(&[value]);
}

#[inline]
fn u16_field(hasher: &mut blake3::Hasher, value: u16) {
    hasher.update(&value.to_le_bytes());
}

#[inline]
fn u32_field(hasher: &mut blake3::Hasher, value: u32) {
    hasher.update(&value.to_le_bytes());
}

#[inline]
fn u64_field(hasher: &mut blake3::Hasher, value: u64) {
    hasher.update(&value.to_le_bytes());
}

#[inline]
fn i64_field(hasher: &mut blake3::Hasher, value: i64) {
    hasher.update(&value.to_le_bytes());
}
