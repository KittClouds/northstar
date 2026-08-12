use std::cmp::Ordering;

use super::input::{Level, SourceRef};

pub const HASH_SEED: u64 = 1_469_598_103_934_665_603;

#[derive(Clone, Debug, Default)]
pub struct Node {
    pub node_id: u64,
    pub evidence_id: u64,
    pub cluster_id: i32,
    pub lower: f64,
    pub price: f64,
    pub upper: f64,
    pub normalized_lower: f64,
    pub normalized_price: f64,
    pub normalized_upper: f64,
    pub width_atr: f64,
    pub distance_atr: f64,
    pub region: i32,
    pub median_distance_atr: f64,
    pub median_distance_sigma: f64,
    pub cog_distance_sigma: f64,
    pub width_sigma: f64,
    pub contains_cog: bool,
    pub role: i32,
    pub family_mask: u64,
    pub role_mask: u64,
    pub family_count: i32,
    pub member_count: i32,
    pub developing_count: i32,
    pub frozen_count: i32,
    pub broken_count: i32,
    pub corridor_up_levels: i32,
    pub corridor_down_levels: i32,
    pub corridor_up_noise: i32,
    pub corridor_down_noise: i32,
    pub oldest_source_time: u64,
    pub newest_update_time: u64,
    pub provenance_offset: i32,
    pub provenance_count: i32,
    pub existence: i32,
    pub created_at: u64,
    pub last_seen_at: u64,
    pub state_changed_at: u64,
    pub attempt_count: i32,
    pub missed_rebuilds: i32,
    pub revision: i32,
    pub primary_parent_id: u64,
    pub secondary_parent_id: u64,
}

pub fn normalize_and_sort(levels: &mut [Level], reference: f64, atr: f64) {
    for level in levels.iter_mut() {
        level.normalized_lower = (level.lower - reference) / atr;
        level.normalized_price = (level.price - reference) / atr;
        level.normalized_upper = (level.upper - reference) / atr;
        level.width_atr = (level.upper - level.lower).max(0.0) / atr;
    }
    levels.sort_by(level_order);
}

pub fn dbscan(levels: &[Level], epsilon: f64, min_samples: usize) -> (Vec<i32>, i32) {
    let count = levels.len();
    let mut labels = vec![-2; count];
    let mut cluster = 0_i32;
    let mut neighbors = Vec::with_capacity(count);
    let mut queue = Vec::with_capacity(count);
    let mut queued = vec![false; count];
    for seed in 0..count {
        if labels[seed] != -2 {
            continue;
        }
        collect_neighbors(levels, seed, epsilon, &mut neighbors);
        if neighbors.len() < min_samples {
            labels[seed] = -1;
            continue;
        }
        queued.fill(false);
        queue.clear();
        for &neighbor in &neighbors {
            if queued[neighbor] {
                continue;
            }
            queue.push(neighbor);
            queued[neighbor] = true;
            if labels[neighbor] == -2 || labels[neighbor] == -1 {
                labels[neighbor] = cluster;
            }
        }
        labels[seed] = cluster;
        let mut head = 0;
        while head < queue.len() {
            let current = queue[head];
            head += 1;
            collect_neighbors(levels, current, epsilon, &mut neighbors);
            if neighbors.len() < min_samples {
                continue;
            }
            for &neighbor in &neighbors {
                if labels[neighbor] == -1 {
                    labels[neighbor] = cluster;
                }
                if labels[neighbor] != -2 {
                    continue;
                }
                labels[neighbor] = cluster;
                if !queued[neighbor] {
                    queue.push(neighbor);
                    queued[neighbor] = true;
                }
            }
        }
        cluster += 1;
    }
    (labels, cluster)
}

fn collect_neighbors(levels: &[Level], point: usize, epsilon: f64, out: &mut Vec<usize>) {
    out.clear();
    for (index, candidate) in levels.iter().enumerate() {
        if compatible(&levels[point], candidate)
            && interval_distance(&levels[point], candidate) <= epsilon
        {
            out.push(index);
        }
    }
}

fn compatible(left: &Level, right: &Level) -> bool {
    left.family > 0
        && right.family > 0
        && left.family < 16
        && right.family < 16
        && left.state != 6
        && right.state != 6
        && roles_compatible(left.role, right.role)
}

fn roles_compatible(left: i32, right: i32) -> bool {
    if left == 0 || right == 0 {
        return true;
    }
    let left = role_side(left);
    let right = role_side(right);
    left == 0 || right == 0 || left == right
}

fn role_side(role: i32) -> i32 {
    match role {
        1..=5 => -1,
        8..=12 => 1,
        _ => 0,
    }
}

fn interval_distance(left: &Level, right: &Level) -> f64 {
    if left.normalized_upper < right.normalized_lower {
        right.normalized_lower - left.normalized_upper
    } else if right.normalized_upper < left.normalized_lower {
        left.normalized_lower - right.normalized_upper
    } else {
        0.0
    }
}

pub fn build_nodes(
    levels: &[Level],
    labels: &[i32],
    cluster_count: i32,
    reference: f64,
    atr: f64,
) -> (Vec<Node>, Vec<SourceRef>) {
    let count = usize::try_from(cluster_count).unwrap_or(0);
    let mut nodes = vec![Node::default(); count];
    let mut weights = vec![0.0; count];
    let mut prices = vec![0.0; count];
    for (index, node) in nodes.iter_mut().enumerate() {
        node.cluster_id = index as i32;
        node.lower = f64::MAX;
        node.upper = -f64::MAX;
        node.normalized_lower = f64::MAX;
        node.normalized_upper = -f64::MAX;
    }
    for (level, &label) in levels.iter().zip(labels) {
        let Ok(cluster) = usize::try_from(label) else {
            continue;
        };
        let Some(node) = nodes.get_mut(cluster) else {
            continue;
        };
        let weight = level.evidence_weight.max(0.000_001);
        node.member_count += 1;
        weights[cluster] += weight;
        prices[cluster] += level.price * weight;
        node.lower = node.lower.min(level.lower);
        node.upper = node.upper.max(level.upper);
        node.normalized_lower = node.normalized_lower.min(level.normalized_lower);
        node.normalized_upper = node.normalized_upper.max(level.normalized_upper);
        node.family_mask |= bit(level.family);
        node.role_mask |= bit(level.role);
        node.developing_count += i32::from(level.developing);
        node.frozen_count += i32::from(level.frozen);
        node.broken_count += i32::from(level.state == 6);
        if level.created_at > 0
            && (node.oldest_source_time == 0 || level.created_at < node.oldest_source_time)
        {
            node.oldest_source_time = level.created_at;
        }
        node.newest_update_time = node.newest_update_time.max(level.updated_at);
    }
    let mut sources = Vec::with_capacity(nodes.iter().map(|node| node.member_count as usize).sum());
    for (cluster, node) in nodes.iter_mut().enumerate() {
        node.provenance_offset = sources.len() as i32;
        for (level, &label) in levels.iter().zip(labels) {
            if label != cluster as i32 {
                continue;
            }
            sources.push(SourceRef {
                source_key: level.source_key,
                producer: level.producer,
                producer_instance: level.producer_instance,
                local_id: level.local_id,
                family: level.family,
                source_kind: level.source_kind,
            });
        }
        let begin = node.provenance_offset as usize;
        sources[begin..].sort_by(source_order);
        node.provenance_count = node.member_count;
        node.price = prices[cluster] / weights[cluster];
        node.normalized_price = (node.price - reference) / atr;
        node.width_atr = (node.upper - node.lower).max(0.0) / atr;
        node.distance_atr = node.normalized_price;
        node.family_count = node.family_mask.count_ones() as i32;
        node.role = if node.normalized_price < -0.15 {
            -1
        } else if node.normalized_price > 0.15 {
            1
        } else {
            0
        };
        node.node_id = build_node_id(&sources, begin, node.member_count as usize);
    }
    nodes.sort_by(|left, right| {
        left.normalized_price
            .total_cmp(&right.normalized_price)
            .then(left.node_id.cmp(&right.node_id))
    });
    (nodes, sources)
}

fn level_order(left: &Level, right: &Level) -> Ordering {
    left.normalized_price
        .total_cmp(&right.normalized_price)
        .then(left.family.cmp(&right.family))
        .then(left.producer.cmp(&right.producer))
        .then(left.producer_instance.cmp(&right.producer_instance))
        .then(left.local_id.cmp(&right.local_id))
}

fn source_order(left: &SourceRef, right: &SourceRef) -> Ordering {
    left.source_key
        .cmp(&right.source_key)
        .then(left.producer.cmp(&right.producer))
        .then(left.producer_instance.cmp(&right.producer_instance))
        .then(left.local_id.cmp(&right.local_id))
}

fn build_node_id(sources: &[SourceRef], offset: usize, count: usize) -> u64 {
    let mut hash = hash_mix(HASH_SEED, count as u64);
    for source in &sources[offset..offset + count] {
        hash = hash_mix(hash, source.source_key);
        hash = hash_mix(hash, source.family as u64);
        hash = hash_mix(hash, source.source_kind.max(0) as u64);
    }
    hash
}

fn bit(value: i32) -> u64 {
    if value <= 0 || value >= 63 {
        0
    } else {
        1_u64 << value
    }
}

pub fn hash_mix(hash: u64, value: u64) -> u64 {
    (hash ^ value).wrapping_mul(1_099_511_628_211)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn level(id: u64, family: i32, role: i32, price: f64) -> Level {
        Level {
            source_key: id,
            producer: 1,
            producer_instance: 1,
            local_id: id,
            family,
            source_kind: 1,
            role,
            lower: price - 0.02,
            price,
            upper: price + 0.02,
            evidence_weight: 1.0,
            ..Level::default()
        }
    }

    #[test]
    fn normalization_sorts_by_geometry_then_identity() {
        let mut levels = vec![level(9, 2, 0, 100.2), level(4, 1, 0, 99.8)];
        normalize_and_sort(&mut levels, 100.0, 2.0);
        assert_eq!([levels[0].local_id, levels[1].local_id], [4, 9]);
        assert!((levels[0].normalized_price + 0.1).abs() < 1e-12);
        assert!((levels[1].width_atr - 0.02).abs() < 1e-12);
    }

    #[test]
    fn dbscan_joins_interval_neighbors_and_keeps_noise() {
        let mut levels = vec![
            level(1, 1, 8, 100.00),
            level(2, 2, 9, 100.08),
            level(3, 3, 8, 102.00),
        ];
        normalize_and_sort(&mut levels, 100.0, 1.0);
        let (labels, clusters) = dbscan(&levels, 0.05, 2);
        assert_eq!(clusters, 1);
        assert_eq!(labels, vec![0, 0, -1]);
    }

    #[test]
    fn dbscan_rejects_opposed_structural_roles() {
        let mut levels = vec![level(1, 1, 1, 100.0), level(2, 2, 8, 100.0)];
        normalize_and_sort(&mut levels, 100.0, 1.0);
        let (labels, clusters) = dbscan(&levels, 0.12, 2);
        assert_eq!(clusters, 0);
        assert_eq!(labels, vec![-1, -1]);
    }

    #[test]
    fn node_identity_is_provenance_order_independent() {
        let mut levels = vec![level(7, 2, 8, 100.02), level(3, 1, 8, 100.00)];
        normalize_and_sort(&mut levels, 100.0, 1.0);
        let (labels, clusters) = dbscan(&levels, 0.12, 2);
        let (nodes, sources) = build_nodes(&levels, &labels, clusters, 100.0, 1.0);
        assert_eq!(nodes.len(), 1);
        assert_eq!(nodes[0].member_count, 2);
        assert_eq!(nodes[0].family_count, 2);
        assert_eq!(sources[0].source_key, 3);
        assert_eq!(sources[1].source_key, 7);
        assert_ne!(nodes[0].node_id, 0);
    }
}
