# Adapter instance qualification protocol

The only input is `fixtures/synthetic_g1_output.json`, classified as synthetic nonpopulation content. The implementation invokes the sealed A1 projector and performs direct field selection and packaging.

The output is bounded by `G8_04A_ADAPTER_PRODUCER_AUTHORITY_V2`. It is not a historical adapter identity, not a G6 fiber artifact, and not a G8 verifier input authorization.

Required terminal checks:

```text
A1_PROJECTOR_ROOT_MATCH = PASS
G1_ROOT_MATCH = PASS
DIRECT_FIELD_MAPPING_ONLY = PASS
SEQUENCE_ORDER_PRESERVED = PASS
NO_NEW_SEMANTIC_RULES = PASS
NO_DOWNSTREAM_CONSTRUCTION = PASS
POPULATION_READS = 0
```
