use crate::gate155::{ProcessObject, object_key};
use crate::gate155_types::{AtlasEdge, AtlasNode, LineageCoordinate, StructuralBridgeReceipt};
use crate::{CandidateAssignment, FittedFamilySystem};
use hashbrown::HashMap;
use std::collections::BTreeMap;

pub(crate) fn build_graph(
    objects: &[ProcessObject],
    receipts: &[StructuralBridgeReceipt],
    assignments: &[CandidateAssignment],
    fitted: &[FittedFamilySystem],
    lineages: &[LineageCoordinate],
    contact_counts: &HashMap<(String, i64), usize>,
) -> (Vec<AtlasNode>, Vec<AtlasEdge>) {
    let mut nodes = BTreeMap::<String, AtlasNode>::new();
    let mut edges = Vec::new();
    for object in objects {
        let key = object_key(&object.kind, &object.run_key, object.object_id);
        nodes.insert(
            key.clone(),
            AtlasNode {
                node_key: key,
                node_type: format!("{}_OBJECT", object.kind),
                local_label: object.object_id.to_string(),
                authority: "RG3_RAW_OBJECT".into(),
            },
        );
    }
    for system in fitted {
        let system_key = format!(
            "family_system::{}::{}::{}",
            system.object_kind, system.representation, system.family_system_sha256
        );
        nodes.insert(
            system_key.clone(),
            AtlasNode {
                node_key: system_key.clone(),
                node_type: "TRAJECTORY_FAMILY_SYSTEM".into(),
                local_label: system.representation.clone(),
                authority: "GATE15_EXPLORATORY".into(),
            },
        );
        for (index, label) in system
            .centroid_labels
            .iter()
            .enumerate()
            .filter(|(index, _)| system.supported[*index])
        {
            let family_key = format!(
                "trajectory_family::{}::{}::{label}",
                system.object_kind, system.representation
            );
            nodes.insert(
                family_key.clone(),
                AtlasNode {
                    node_key: family_key.clone(),
                    node_type: "TRAJECTORY_FAMILY".into(),
                    local_label: label.clone(),
                    authority: "GATE15_EXPLORATORY".into(),
                },
            );
            edges.push(AtlasEdge {
                edge_key: format!("edge::system::{system_key}::{index}"),
                edge_type: "SYSTEM_CONTAINS_TRAJECTORY_FAMILY".into(),
                source_key: system_key.clone(),
                destination_key: family_key,
                event_time: None,
                weight: 1,
                availability_code: "AVAILABLE".into(),
            });
        }
    }
    for row in assignments
        .iter()
        .filter(|row| row.eligible && row.candidate_id.is_some())
    {
        let family_key = format!(
            "trajectory_family::{}::{}::{}",
            row.object_kind,
            row.representation,
            row.candidate_id.as_deref().unwrap()
        );
        edges.push(AtlasEdge {
            edge_key: format!(
                "edge::assignment::{}::{}::{}::{}",
                row.run_key, row.object_kind, row.object_id, row.representation
            ),
            edge_type: "OBJECT_ASSIGNED_TO_TRAJECTORY_FAMILY".into(),
            source_key: object_key(&row.object_kind, &row.run_key, row.object_id),
            destination_key: family_key,
            event_time: None,
            weight: 1,
            availability_code: "AVAILABLE".into(),
        });
    }
    for receipt in receipts
        .iter()
        .filter(|receipt| receipt.availability_code == "AVAILABLE")
    {
        let snapshot_key = format!(
            "master_snapshot::{}::{}::{}",
            receipt.run_key,
            receipt.master_snapshot_time.unwrap(),
            receipt.master_snapshot_hash.as_deref().unwrap()
        );
        nodes
            .entry(snapshot_key.clone())
            .or_insert_with(|| AtlasNode {
                node_key: snapshot_key.clone(),
                node_type: "MASTER_STRUCTURAL_SNAPSHOT".into(),
                local_label: receipt.master_snapshot_time.unwrap().to_string(),
                authority: "RG3_MASTER_POINT_STATE".into(),
            });
        edges.push(AtlasEdge {
            edge_key: format!("edge::{}", receipt.receipt_id),
            edge_type: "OBJECT_EVENT_CAUSALLY_ATTACHED_TO_MASTER_SNAPSHOT".into(),
            source_key: object_key(&receipt.object_kind, &receipt.run_key, receipt.object_id),
            destination_key: snapshot_key.clone(),
            event_time: Some(receipt.object_event_time),
            weight: 1,
            availability_code: receipt.join_mode.clone(),
        });
        if let Some(node_id) = &receipt.nearest_node_id {
            let node_key = format!("master_node::{}::{node_id}", receipt.run_key);
            nodes.entry(node_key.clone()).or_insert_with(|| AtlasNode {
                node_key: node_key.clone(),
                node_type: "MASTER_STRUCTURAL_NODE".into(),
                local_label: node_id.clone(),
                authority: "RG3_MASTER_POINT_STATE".into(),
            });
            edges.push(AtlasEdge {
                edge_key: format!("edge::snapshot_node::{}::{node_id}", receipt.receipt_id),
                edge_type: if receipt.node_contact == Some(true) {
                    "MASTER_SNAPSHOT_CONTACTS_NODE"
                } else {
                    "MASTER_SNAPSHOT_NEAREST_NODE"
                }
                .into(),
                source_key: snapshot_key,
                destination_key: node_key,
                event_time: receipt.master_snapshot_time,
                weight: 1,
                availability_code: "AVAILABLE".into(),
            });
        }
    }
    for lineage in lineages {
        let expansion = object_key("EXPANSION", &lineage.run_key, lineage.expansion_id);
        edges.push(AtlasEdge {
            edge_key: format!("edge::lineage_origin::{}", lineage.lineage_id),
            edge_type: "EXPANSION_ORIGINATES_FROM_COMPRESSION".into(),
            source_key: object_key(
                "COMPRESSION",
                &lineage.run_key,
                lineage.origin_compression_id,
            ),
            destination_key: expansion.clone(),
            event_time: None,
            weight: 1,
            availability_code: "AVAILABLE".into(),
        });
        if let Some(destination) = lineage.destination_compression_id {
            edges.push(AtlasEdge {
                edge_key: format!("edge::lineage_destination::{}", lineage.lineage_id),
                edge_type: "EXPANSION_HANDOFF_TO_COMPRESSION".into(),
                source_key: expansion.clone(),
                destination_key: object_key("COMPRESSION", &lineage.run_key, destination),
                event_time: None,
                weight: 1,
                availability_code: "AVAILABLE".into(),
            });
        }
        if let Some(&count) = contact_counts.get(&(lineage.run_key.clone(), lineage.expansion_id)) {
            edges.push(AtlasEdge {
                edge_key: format!("edge::contact_count::{}", lineage.lineage_id),
                edge_type: "EXPANSION_STRUCTURAL_NODE_CONTACT_COUNT".into(),
                source_key: expansion.clone(),
                destination_key: expansion,
                event_time: None,
                weight: count,
                availability_code: "AGGREGATED_RELATION_RECEIPTS".into(),
            });
        }
    }
    edges.sort_by(|a, b| a.edge_key.cmp(&b.edge_key));
    (nodes.into_values().collect(), edges)
}
