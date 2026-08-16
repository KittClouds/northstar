use northstar_operating_surface::*;
use smallvec::smallvec;
use std::sync::{Arc, Barrier};
use std::thread;

fn root(name: &str) -> Root {
    Root::hash(name.as_bytes())
}

fn binding(gate: &str) -> ExecutionBinding {
    ExecutionBinding {
        scheduler_receipt_root: root("scheduler"),
        gate_id: gate.to_owned(),
        gate_spec_root: root("spec"),
        parent_roots: smallvec![root("parent")],
        input_roots: smallvec![root("input")],
        authority_roots: smallvec![root("authority")],
    }
}

fn record(
    id: &str,
    class: ArtifactClass,
    parents: smallvec::SmallVec<[Root; 4]>,
) -> ArtifactRecord {
    ArtifactRecord {
        id: id.to_owned(),
        root: root(id),
        class,
        parent_roots: parents,
        input_roots: smallvec![],
        result_sealed: true,
        creation_authorized: true,
        execution_authorized: true,
        inputs_valid: true,
        scope_valid: true,
        blocked: false,
        scheduler_controlled: false,
        chronology: ArtifactChronology {
            creation_authority_sealed_at: 1,
            inputs_frozen_at: 2,
            scheduler_selected_at: None,
            execution_authorized_at: 3,
            execution_started_at: 4,
            result_sealed_at: 5,
            first_consumed_at: None,
        },
        authority_ceiling: "GOVERNANCE_ONLY".to_owned(),
    }
}

fn receipt(record: &ArtifactRecord) -> InsertionReceipt {
    InsertionReceipt {
        target_root: record.root,
        target_class: record.class,
        authority_ceiling: record.authority_ceiling.clone(),
        receipt_root: root("receipt"),
    }
}

#[test]
fn lifecycle_requires_every_ignition_transition() {
    let state = LifecycleState::Declared
        .transition(LifecycleState::Eligible)
        .unwrap()
        .transition(LifecycleState::Selected)
        .unwrap()
        .transition(LifecycleState::ExecutionAuthorized)
        .unwrap()
        .transition(LifecycleState::Executing)
        .unwrap()
        .transition(LifecycleState::Executed)
        .unwrap()
        .transition(LifecycleState::ResultSealed)
        .unwrap()
        .transition(LifecycleState::ConsumabilityEstablished)
        .unwrap()
        .transition(LifecycleState::DagRecomputed)
        .unwrap();
    assert_eq!(state, LifecycleState::DagRecomputed);
    assert!(LifecycleState::Selected
        .transition(LifecycleState::Executing)
        .is_err());
    assert!(LifecycleState::Executed
        .transition(LifecycleState::ConsumabilityEstablished)
        .is_err());
}

#[test]
fn capability_is_exact_single_use() {
    let expected = binding("GATE-A");
    let capability = ExecutionCapability::new(expected.clone());
    let mut changed = expected.clone();
    changed.input_roots[0] = root("changed");
    assert_eq!(
        capability.consume(&changed, root("x")),
        Err(CapabilityError::BindingMismatch)
    );
    capability.consume(&expected, root("execution-1")).unwrap();
    assert_eq!(
        capability.consume(&expected, root("execution-2")),
        Err(CapabilityError::AlreadyConsumed)
    );
}

#[test]
fn capability_rejects_sibling_gate() {
    let capability = ExecutionCapability::new(binding("GATE-A"));
    assert_eq!(
        capability.consume(&binding("GATE-B"), root("execution")),
        Err(CapabilityError::BindingMismatch)
    );
}

#[test]
fn historical_selection_cannot_authorize_recovery_descendant() {
    let capability = ExecutionCapability::new(binding("FC01-FQB-C1"));
    assert_eq!(
        capability.consume(&binding("FC01-FQB-C1-R1"), root("execution")),
        Err(CapabilityError::BindingMismatch)
    );
}

#[test]
fn changed_authority_root_cannot_consume_capability() {
    let expected = binding("GATE-A");
    let capability = ExecutionCapability::new(expected.clone());
    let mut changed = expected;
    changed.authority_roots[0] = root("different-authority");
    assert_eq!(
        capability.consume(&changed, root("execution")),
        Err(CapabilityError::BindingMismatch)
    );
}

#[test]
fn capability_atomic_race_has_exactly_one_winner() {
    let binding = binding("RACE-GATE");
    let capability = Arc::new(ExecutionCapability::new(binding.clone()));
    let barrier = Arc::new(Barrier::new(32));
    let handles: Vec<_> = (0..32)
        .map(|index| {
            let capability = Arc::clone(&capability);
            let binding = binding.clone();
            let barrier = Arc::clone(&barrier);
            thread::spawn(move || {
                barrier.wait();
                capability
                    .consume(&binding, root(&format!("execution-{index}")))
                    .is_ok()
            })
        })
        .collect();
    assert_eq!(
        handles
            .into_iter()
            .map(|handle| handle.join().unwrap())
            .filter(|won| *won)
            .count(),
        1
    );
}

#[test]
fn capability_revocation_precedes_execution() {
    let binding = binding("GATE-A");
    let capability = ExecutionCapability::new(binding.clone());
    capability.revoke().unwrap();
    assert_eq!(
        capability.consume(&binding, root("execution")),
        Err(CapabilityError::Revoked)
    );
}

#[test]
fn blocked_ancestor_collapses_descendant_consumability() {
    let mut registry = ArtifactRegistry::default();
    let mut parent = record("PARENT", ArtifactClass::AuthorityGrant, smallvec![]);
    parent.blocked = true;
    let child = record("CHILD", ArtifactClass::Result, smallvec![parent.root]);
    registry
        .insert_authoritative(parent.clone(), &receipt(&parent))
        .unwrap();
    registry
        .insert_authoritative(child.clone(), &receipt(&child))
        .unwrap();
    let status = registry.derive_consumability(child.root).unwrap();
    assert!(!status.consumable);
    assert!(status.blocked_ancestors.contains(&parent.root));
}

#[test]
fn consumability_cycle_fails_closed() {
    let mut registry = ArtifactRegistry::default();
    let mut a = record("A", ArtifactClass::Result, smallvec![root("B")]);
    let b = record("B", ArtifactClass::Result, smallvec![a.root]);
    a.parent_roots[0] = b.root;
    registry
        .insert_authoritative(a.clone(), &receipt(&a))
        .unwrap();
    registry
        .insert_authoritative(b.clone(), &receipt(&b))
        .unwrap();
    assert!(!registry.derive_consumability(a.root).unwrap().consumable);
}

#[test]
fn unauthorized_bytes_cannot_enter_registry() {
    let mut registry = ArtifactRegistry::default();
    let artifact = record("UNAUTHORIZED", ArtifactClass::Result, smallvec![]);
    let wrong = InsertionReceipt {
        target_root: root("other"),
        target_class: ArtifactClass::Result,
        authority_ceiling: "GOVERNANCE_ONLY".into(),
        receipt_root: root("wrong-receipt"),
    };
    assert_eq!(
        registry.insert_authoritative(artifact, &wrong),
        Err(RegistryError::UnauthorizedInsertion)
    );
}

#[test]
fn reference_monitor_denies_unknown_pair_and_protected_data() {
    let mut registry = ArtifactRegistry::default();
    let actor = record("ACTOR", ArtifactClass::AuthorityGrant, smallvec![]);
    registry
        .insert_authoritative(actor.clone(), &receipt(&actor))
        .unwrap();
    let kernel = AuthorityKernel::new(
        [(OperationClass::Create, ArtifactClass::AuditFinding)],
        ["GOVERNANCE_ONLY".to_owned()],
        [DataCapability::SealedGovernance],
    );
    let mut request = OperationRequest {
        operation: OperationClass::Execute,
        actor_authority_root: actor.root,
        target_class: ArtifactClass::Result,
        target_id: "OSA-FINDING-0001".into(),
        parent_roots: smallvec![],
        input_roots: smallvec![],
        authority_roots: smallvec![],
        data_capabilities: smallvec![],
        expected_output_class: ArtifactClass::AuditFinding,
        expected_authority_ceiling: "GOVERNANCE_ONLY".into(),
    };
    assert_eq!(
        kernel.authorize(&request, &registry),
        Err(MonitorError::UnknownOperationPair)
    );
    request.operation = OperationClass::Create;
    request.target_class = ArtifactClass::AuditFinding;
    request.data_capabilities.push(DataCapability::Population);
    assert_eq!(
        kernel.authorize(&request, &registry),
        Err(MonitorError::ProtectedDataForbidden)
    );
}

#[test]
fn reference_monitor_allows_only_explicit_data_capability() {
    let mut registry = ArtifactRegistry::default();
    let actor = record("ACTOR", ArtifactClass::AuthorityGrant, smallvec![]);
    registry
        .insert_authoritative(actor.clone(), &receipt(&actor))
        .unwrap();
    let kernel = AuthorityKernel::new(
        [(OperationClass::Create, ArtifactClass::AuditFinding)],
        ["GOVERNANCE_ONLY".to_owned()],
        [DataCapability::SealedGovernance],
    );
    let request = OperationRequest {
        operation: OperationClass::Create,
        actor_authority_root: actor.root,
        target_class: ArtifactClass::AuditFinding,
        target_id: "OSA-FINDING-0009".into(),
        parent_roots: smallvec![],
        input_roots: smallvec![],
        authority_roots: smallvec![],
        data_capabilities: smallvec![DataCapability::SealedGovernance],
        expected_output_class: ArtifactClass::AuditFinding,
        expected_authority_ceiling: "GOVERNANCE_ONLY".into(),
    };
    assert!(kernel.authorize(&request, &registry).is_ok());
}

#[test]
fn denied_operation_cannot_mutate_registry() {
    let mut registry = ArtifactRegistry::default();
    let actor = record("BLOCKED-ACTOR", ArtifactClass::AuthorityGrant, smallvec![]);
    registry
        .insert_authoritative(actor.clone(), &receipt(&actor))
        .unwrap();
    let before = registry.len();
    let kernel = AuthorityKernel::new([], ["GOVERNANCE_ONLY".to_owned()], []);
    let request = OperationRequest {
        operation: OperationClass::Create,
        actor_authority_root: actor.root,
        target_class: ArtifactClass::Result,
        target_id: "FORBIDDEN".into(),
        parent_roots: smallvec![],
        input_roots: smallvec![],
        authority_roots: smallvec![],
        data_capabilities: smallvec![],
        expected_output_class: ArtifactClass::Result,
        expected_authority_ceiling: "GOVERNANCE_ONLY".into(),
    };
    assert_eq!(
        kernel.authorize(&request, &registry),
        Err(MonitorError::UnknownOperationPair)
    );
    assert_eq!(registry.len(), before);
}

#[test]
fn scheduler_controlled_chronology_requires_selection_before_authorization() {
    let mut registry = ArtifactRegistry::default();
    let mut artifact = record("BAD-CHRONOLOGY", ArtifactClass::Result, smallvec![]);
    artifact.scheduler_controlled = true;
    artifact.chronology.scheduler_selected_at = None;
    registry
        .insert_authoritative(artifact.clone(), &receipt(&artifact))
        .unwrap();
    assert!(
        !registry
            .derive_consumability(artifact.root)
            .unwrap()
            .consumable
    );
}

#[test]
fn canonical_json_sorts_maps_but_preserves_sequences() {
    let a = serde_json::json!({"z": [3, 1, 2], "a": 1});
    let b = serde_json::json!({"a": 1, "z": [3, 1, 2]});
    assert_eq!(
        canonical_json_bytes(&a).unwrap(),
        canonical_json_bytes(&b).unwrap()
    );
    assert_eq!(
        canonical_json_bytes(&a).unwrap(),
        b"{\"a\":1,\"z\":[3,1,2]}\n"
    );
    assert_eq!(
        canonical_json_bytes(&serde_json::json!(1.5)),
        Err(CanonicalError::FloatForbidden)
    );
}

#[test]
fn mmap_hash_reads_immutable_artifact_without_copying() {
    use std::io::Write;
    let mut file = tempfile::NamedTempFile::new().unwrap();
    file.write_all(b"sealed-artifact").unwrap();
    assert_eq!(
        mmap_hash(file.path()).unwrap(),
        Root::hash(b"sealed-artifact")
    );
}
