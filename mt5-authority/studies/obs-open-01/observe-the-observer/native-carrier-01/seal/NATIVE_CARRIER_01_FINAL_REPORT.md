# NATIVE-CARRIER-01 Final Report

## Outcome

Two probes executed over the five sealed REAL-LIGHT-02 native objects on the exact 154-specimen common carrier. Per-eye signature construction was isolated; the relation layer consumed opaque class IDs only. No native optic was changed and no horizontal comparison was opened.

## Carrier

Carrier status: `CARRIER_NONEMPTY`; specimens: `154`; arm membership equality: `TRUE`.

## Probe 1

Probe 1 produced exact L0 outcome-signature and L1 complete-payload signature partitions for all five eyes. L0 and L1 are separate levels; class identity was verified by exact canonical-byte equality before opaque class IDs were emitted.

### L0

Realized five-bit distinction types: `8` of `9` patterns structurally admissible under proven relation constraints.
Unordered pair carrier size: `11781`; realized pair-type counts: `{"00000": 2328, "00001": 121, "00010": 2579, "00011": 1080, "10000": 1347, "10001": 522, "10010": 1443, "10011": 2361}`.
Full meet: `{'block_count': 10, 'block_size_multiset': [57, 21, 21, 17, 15, 11, 5, 3, 3, 1], 'singleton_count': 1, 'non_singleton_count': 9, 'collapsed_unordered_pair_count': 2328, 'max_block_size': 57, 'discrete': False}`.
Inclusion-minimal meet bases: `['SOL-P1+SOL-P4+SOL-P5']`.

- `SOL-P1 vs SOL-P2`: `LEFT_STRICTLY_REFINES_RIGHT`.
- `SOL-P1 vs SOL-P3`: `LEFT_STRICTLY_REFINES_RIGHT`.
- `SOL-P1 vs SOL-P4`: `INCOMPARABLE_NATIVE_PARTITIONS`.
- `SOL-P1 vs SOL-P5`: `INCOMPARABLE_NATIVE_PARTITIONS`.
- `SOL-P2 vs SOL-P3`: `EQUAL_NATIVE_SIGNATURE_PARTITIONS`.
- `SOL-P2 vs SOL-P4`: `RIGHT_STRICTLY_REFINES_LEFT`.
- `SOL-P2 vs SOL-P5`: `RIGHT_STRICTLY_REFINES_LEFT`.
- `SOL-P3 vs SOL-P4`: `RIGHT_STRICTLY_REFINES_LEFT`.
- `SOL-P3 vs SOL-P5`: `RIGHT_STRICTLY_REFINES_LEFT`.
- `SOL-P4 vs SOL-P5`: `INCOMPARABLE_NATIVE_PARTITIONS`.

### L1

Realized five-bit distinction types: `5` of `6` patterns structurally admissible under proven relation constraints.
Unordered pair carrier size: `11781`; realized pair-type counts: `{"00001": 3, "00101": 3, "00111": 1506, "10111": 8218, "11111": 2051}`.
Full meet: `{'block_count': 154, 'block_size_multiset': [1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1], 'singleton_count': 154, 'non_singleton_count': 0, 'collapsed_unordered_pair_count': 0, 'max_block_size': 1, 'discrete': True}`.
Inclusion-minimal meet bases: `['SOL-P5']`.

- `SOL-P1 vs SOL-P2`: `LEFT_STRICTLY_REFINES_RIGHT`.
- `SOL-P1 vs SOL-P3`: `RIGHT_STRICTLY_REFINES_LEFT`.
- `SOL-P1 vs SOL-P4`: `RIGHT_STRICTLY_REFINES_LEFT`.
- `SOL-P1 vs SOL-P5`: `RIGHT_STRICTLY_REFINES_LEFT`.
- `SOL-P2 vs SOL-P3`: `RIGHT_STRICTLY_REFINES_LEFT`.
- `SOL-P2 vs SOL-P4`: `RIGHT_STRICTLY_REFINES_LEFT`.
- `SOL-P2 vs SOL-P5`: `RIGHT_STRICTLY_REFINES_LEFT`.
- `SOL-P3 vs SOL-P4`: `LEFT_STRICTLY_REFINES_RIGHT`.
- `SOL-P3 vs SOL-P5`: `RIGHT_STRICTLY_REFINES_LEFT`.
- `SOL-P4 vs SOL-P5`: `RIGHT_STRICTLY_REFINES_LEFT`.

## Probe 2

Probe 2 computed exact join anatomy and meet/join trigger states from the sealed opaque Probe 1 partitions. It did not read native payloads.

- `L0` full join: `{'block_count': 1, 'block_size_multiset': [154], 'singleton_count': 0, 'non_singleton_count': 1, 'collapsed_unordered_pair_count': 11781, 'max_block_size': 154, 'discrete': False}`; inclusion-minimal join bases: `['SOL-P1+SOL-P4', 'SOL-P1+SOL-P5', 'SOL-P2', 'SOL-P3', 'SOL-P4+SOL-P5']`.
  Full meet: `{'block_count': 10, 'block_size_multiset': [57, 21, 21, 17, 15, 11, 5, 3, 3, 1], 'singleton_count': 1, 'non_singleton_count': 9, 'collapsed_unordered_pair_count': 2328, 'max_block_size': 57, 'discrete': False}`; meet-discrete/join-indiscrete subsets: `[]`; bi-extreme subsets reproducing all-five meet and join: `['SOL-P1+SOL-P4+SOL-P5', 'SOL-P1+SOL-P2+SOL-P4+SOL-P5', 'SOL-P1+SOL-P3+SOL-P4+SOL-P5', 'SOL-P1+SOL-P2+SOL-P3+SOL-P4+SOL-P5']`.
- `L1` full join: `{'block_count': 15, 'block_size_multiset': [140, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1], 'singleton_count': 14, 'non_singleton_count': 1, 'collapsed_unordered_pair_count': 9730, 'max_block_size': 140, 'discrete': False}`; inclusion-minimal join bases: `['SOL-P2']`.
  Full meet: `{'block_count': 154, 'block_size_multiset': [1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1], 'singleton_count': 154, 'non_singleton_count': 0, 'collapsed_unordered_pair_count': 0, 'max_block_size': 1, 'discrete': True}`; meet-discrete/join-indiscrete subsets: `[]`; bi-extreme subsets reproducing all-five meet and join: `['SOL-P2+SOL-P5', 'SOL-P1+SOL-P2+SOL-P5', 'SOL-P2+SOL-P3+SOL-P5', 'SOL-P2+SOL-P4+SOL-P5', 'SOL-P1+SOL-P2+SOL-P3+SOL-P5', 'SOL-P1+SOL-P2+SOL-P4+SOL-P5', 'SOL-P2+SOL-P3+SOL-P4+SOL-P5', 'SOL-P1+SOL-P2+SOL-P3+SOL-P4+SOL-P5']`.

## Access and firewalls

```text
parent_REAL_LIGHT_root_verified = True
parent_NATIVE_MORPH_root_verified = True
sealed_native_streams_read = 5
common_carrier_constructed = True
signature_products_constructed = 5
relation_layer_native_payload_reads = 0
relation_layer_opaque_signature_reads = 5
probe_1_native_payload_reads = 0
probe_2_native_payload_reads = 0
native_payload_fields_read_for_signature = L0_declared_native_outcome_projection_and_L1_complete_payload
D_A_raw_reads = 0
D_B_reads = 0
D_C_reads = 0
D_D_reads = 0
Sentinel_values_consumed = 0
sibling_values_consumed_by_arm = 0
targets_read = 0
outcomes_read = 0
external_outcome_surface_reads = 0
confirmation_sessions_opened = 0
semantic_translators_created = 0
horizontal_science_performed = 0
Thing_2 = UNBOUND
firewall_status = PASS
```

No Sentinel, D_B/D_C/D_D, external target/outcome surface, confirmation, translator, or horizontal relation data were consumed. Native outcome fields were read only as the contract-frozen L0 signature surface. Thing 2 remains `UNBOUND`. Discovery counts are not prevalence or incidence claims. No redundancy claim is made without an explicit qualifier.

## Closure

The carrier, five opaque signature products, Probe 1 partition/cube object, Probe 2 join/complement object, access ledger, and root receipt are sealed. This report records exact relational structure only; interpretation and any future real-history confirmation remain outside this campaign.
