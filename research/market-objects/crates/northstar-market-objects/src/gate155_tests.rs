use super::*;

fn sample(time: i64, hash: &str) -> MasterSample {
    MasterSample {
        time,
        snapshot_hash: hash.into(),
        regional_basis_hash: "b".into(),
        generation: 1,
        reference_price: 1.0,
        reference_atr: 1.0,
        median_price: 1.0,
        mean_price: 1.0,
        sigma: 1.0,
        cog_price: 1.0,
        price_region_code: 0,
        node_id: "0".into(),
        node_lower: None,
        node_price: None,
        node_upper: None,
        node_region_code: None,
        node_contact: false,
    }
}

fn object() -> ProcessObject {
    ProcessObject {
        kind: "EXPANSION".into(),
        run_key: "R".into(),
        instrument: "US30".into(),
        object_id: 1,
        start_time: 100,
        confirm_time: None,
        terminal_time: 100,
        terminal_reason_code: 4,
        censored: false,
        origin_compression_id: Some(1),
        destination_compression_id: None,
    }
}

#[test]
fn asof_join_never_reads_future_and_fails_after_gap() {
    let samples = vec![sample(90, "old"), sample(110, "future")];
    let receipt = bridge_receipt(&object(), "ORIGIN", 100, &samples, 15);
    assert_eq!(receipt.master_snapshot_time, Some(90));
    assert_eq!(receipt.join_mode, "ASOF");
    let failed = bridge_receipt(&object(), "ORIGIN", 100, &samples, 5);
    assert_eq!(failed.availability_code, "NULL_STRUCTURAL_CONTEXT");
}

#[test]
fn bridge_is_monotonic_under_future_append() {
    let prefix = vec![sample(90, "old"), sample(100, "exact")];
    let mut extended = prefix.clone();
    extended.push(sample(110, "future"));
    assert_eq!(
        bridge_receipt(&object(), "ORIGIN", 100, &prefix, 10),
        bridge_receipt(&object(), "ORIGIN", 100, &extended, 10)
    );
}
