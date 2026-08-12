use super::{
    MarketLocation, MarketLocationSnapshot, Price, StructuralGraphSnapshot, StructuralNode,
};

pub fn locate_market(
    graph: &StructuralGraphSnapshot,
    price: Price,
    atr: i64,
) -> MarketLocationSnapshot {
    let nodes = graph.nodes.as_ref();
    if nodes.is_empty() {
        return MarketLocationSnapshot::UNAVAILABLE;
    }
    if let Some(node) = containing_node(nodes, price) {
        return MarketLocationSnapshot {
            location: MarketLocation::InsideNode(node.id),
            distance_to_lower: Some((price - node.lower).max(0)),
            distance_to_upper: Some((node.upper - price).max(0)),
            distance_to_lower_atr_ppm: ratio((price - node.lower).max(0), atr),
            distance_to_upper_atr_ppm: ratio((node.upper - price).max(0), atr),
        };
    }
    let upper_index = nodes.partition_point(|node| node.center < price);
    if upper_index == 0 {
        let nearest = &nodes[0];
        let distance = (nearest.lower - price).max(0);
        return MarketLocationSnapshot {
            location: MarketLocation::BelowKnownStructure {
                nearest: nearest.id,
            },
            distance_to_lower: None,
            distance_to_upper: Some(distance),
            distance_to_lower_atr_ppm: None,
            distance_to_upper_atr_ppm: ratio(distance, atr),
        };
    }
    if upper_index == nodes.len() {
        let nearest = &nodes[nodes.len() - 1];
        let distance = (price - nearest.upper).max(0);
        return MarketLocationSnapshot {
            location: MarketLocation::AboveKnownStructure {
                nearest: nearest.id,
            },
            distance_to_lower: Some(distance),
            distance_to_upper: None,
            distance_to_lower_atr_ppm: ratio(distance, atr),
            distance_to_upper_atr_ppm: None,
        };
    }
    let lower = &nodes[upper_index - 1];
    let upper = &nodes[upper_index];
    let lower_distance = (price - lower.upper).max(0);
    let upper_distance = (upper.lower - price).max(0);
    MarketLocationSnapshot {
        location: MarketLocation::Between {
            lower: lower.id,
            upper: upper.id,
        },
        distance_to_lower: Some(lower_distance),
        distance_to_upper: Some(upper_distance),
        distance_to_lower_atr_ppm: ratio(lower_distance, atr),
        distance_to_upper_atr_ppm: ratio(upper_distance, atr),
    }
}

fn containing_node(nodes: &[StructuralNode], price: i64) -> Option<&StructuralNode> {
    nodes
        .iter()
        .filter(|node| price >= node.lower && price <= node.upper)
        .min_by_key(|node| (node.upper - node.lower, node.id))
}

fn ratio(value: i64, atr: i64) -> Option<i64> {
    (atr > 0).then(|| {
        ((value as i128 * 1_000_000) / atr as i128).clamp(i64::MIN as i128, i64::MAX as i128) as i64
    })
}
