use crate::gate16_types::{EventToken, ProcessGeometryObject};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;

#[derive(Debug, Clone)]
pub(crate) struct GraphSignature {
    pub typed_multiset: BTreeMap<String, u32>,
    pub wl2_multiset: BTreeMap<String, u32>,
}

#[derive(Clone)]
struct Edge {
    from: usize,
    to: usize,
    kind: &'static str,
}

fn add(multiset: &mut BTreeMap<String, u32>, value: String) {
    *multiset.entry(value).or_insert(0) += 1;
}

fn short_hash(value: &str) -> String {
    let digest = format!("{:x}", Sha256::digest(value.as_bytes()));
    digest[..16].to_owned()
}

fn token_label(token: &EventToken) -> String {
    format!(
        "EVENT:{}:{}:{}:{}",
        token.event_code, token.direction, token.state_code, token.terminal_reason_code
    )
}

pub(crate) fn graph_signature(object: &ProcessGeometryObject, canonical: bool) -> GraphSignature {
    let events = if canonical {
        &object.events_canonical
    } else {
        &object.events_raw
    };
    let mut labels = Vec::with_capacity(events.len() * 2 + 2);
    labels.push(format!("OBJECT:{}", object.kind.name()));
    let object_node = 0usize;
    let mut edges = Vec::with_capacity(events.len() * 4 + 2);
    let mut previous_event = None;
    for token in events {
        let event_node = labels.len();
        labels.push(token_label(token));
        let state_node = labels.len();
        labels.push(format!("STATE:{}", token.state_code));
        edges.push(Edge {
            from: object_node,
            to: event_node,
            kind: "EMITS",
        });
        edges.push(Edge {
            from: event_node,
            to: state_node,
            kind: "OBSERVES_STATE",
        });
        if let Some(previous) = previous_event {
            edges.push(Edge {
                from: previous,
                to: event_node,
                kind: "PRECEDES",
            });
        }
        previous_event = Some(event_node);
    }
    let terminal_node = labels.len();
    let terminal = if canonical && object.direction < 0 {
        crate::gate16_collect::reflect_terminal(object.terminal_reason_code)
    } else {
        object.terminal_reason_code
    };
    labels.push(format!("TERMINAL:{terminal}:CENSORED_{}", object.censored));
    edges.push(Edge {
        from: object_node,
        to: terminal_node,
        kind: "HAS_TERMINAL_STATUS",
    });
    if let Some(previous) = previous_event {
        edges.push(Edge {
            from: previous,
            to: terminal_node,
            kind: "TERMINATES_OR_CENSORS",
        });
    }

    let mut typed = BTreeMap::new();
    for label in &labels {
        add(&mut typed, format!("NODE:{label}"));
    }
    for edge in &edges {
        add(
            &mut typed,
            format!(
                "EDGE:{}:{}>{}",
                edge.kind, labels[edge.from], labels[edge.to]
            ),
        );
    }

    let mut current: Vec<_> = labels.iter().map(|label| short_hash(label)).collect();
    let mut wl2 = BTreeMap::new();
    for round in 0..=2 {
        for label in &current {
            add(&mut wl2, format!("WL{round}:{label}"));
        }
        if round == 2 {
            break;
        }
        let mut next = Vec::with_capacity(current.len());
        for node in 0..current.len() {
            let mut neighborhood = Vec::new();
            for edge in &edges {
                if edge.from == node {
                    neighborhood.push(format!("OUT:{}:{}", edge.kind, current[edge.to]));
                }
                if edge.to == node {
                    neighborhood.push(format!("IN:{}:{}", edge.kind, current[edge.from]));
                }
            }
            neighborhood.sort();
            next.push(short_hash(&format!(
                "{}|{}",
                current[node],
                neighborhood.join("|")
            )));
        }
        current = next;
    }
    GraphSignature {
        typed_multiset: typed,
        wl2_multiset: wl2,
    }
}

pub(crate) fn multiset_jaccard(left: &BTreeMap<String, u32>, right: &BTreeMap<String, u32>) -> f64 {
    let mut intersection = 0u64;
    let mut union = 0u64;
    for (key, &left_count) in left {
        let right_count = right.get(key).copied().unwrap_or(0);
        intersection += left_count.min(right_count) as u64;
        union += left_count.max(right_count) as u64;
    }
    for (key, &right_count) in right {
        if !left.contains_key(key) {
            union += right_count as u64;
        }
    }
    if union == 0 {
        0.0
    } else {
        1.0 - intersection as f64 / union as f64
    }
}
