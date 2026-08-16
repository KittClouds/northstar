# G1-M Final Partner Report — Find the Wall Anchor

## Sealed result

```text
EXECUTION_STATE = SEALED
QUESTION_STATUS  = CLOSED
RESULT           = G1_SEALED_CALLABLE_MATERIALIZATION_NOT_EVALUABLE
DISPOSITION      = HOLD
```

G1-M identified the sealed G1 authority object and recovered highly plausible historical compiled candidates. It did not establish the cryptographic authority-to-artifact binding required to call either candidate the exact historical callable. A1 Attempt 2 is therefore not eligible.

The decisive conjunction remains:

```text
sealed source identity                          PASS
sealed callable interface identity              PASS at source level
historical compiled-artifact identity           NOT EVALUABLE
historical output transport into frozen A1 ABI  NOT EVALUABLE
```

A1 Attempt 1 remains immutable:

```text
PROJECTION_ONLY_PATH = QUALIFIED
END_TO_END_G1_PATH   = NOT_EVALUABLE
RESULT               = NOT_EVALUABLE
```

No score, partial pass, or retrospective repair is issued.

---

## To Sol — positive archaeological identity

You found the authority object.

The sealed G1 package is present and internally coherent:

```text
authority = OBS_OPEN_G1_SENTINEL_SEMANTIC_KERNEL_V1
G1 root   = 65cbe177fd5dc510ab9dc19e203a912a107ab8ac740b55a64926a1164edceafd
source closure = a7346e722d35085f5ff97f76cda4c4de97c519facc937509ad3dce4a11ad0473
source files = 10
```

The manifest hashes back to the declared G1 root. All ten live source members are byte-identical to the sealed source copies. The source-level callable boundary is exact and recovered:

```text
init(&KernelContext)
    -> Result<KernelState, KernelError>

step(&KernelState, &CompletedObservation, &KernelContext)
    -> Result<StepResult, KernelError>
```

Initialization, context, observation cadence, gap failure, session boundaries, ordered emissions, and the ancestor projection are all recoverable from sealed contracts and exact source.

You also found two strong candidate materializations in the historical target directory:

```text
D:/codex-target-obs-open-g1/release/obs-open-04a-g1.exe
SHA256 = CEC0CA267B1F42B9B9173C8A40A43F0AC28ACB70D569500209FBA2C3D0145FA1

D:/codex-target-obs-open-g1/release/libobs_open_04a_g1.rlib
SHA256 = 564CA86530748F26FD300CB89CEF092266528D45341DB4AFCEAB00325F74205D
```

Their Cargo dep-info points to the exact historical G1 source path. Their compiler metadata records Rust 1.96.0, Windows MSVC, and LLVM 22.1.2. Their timestamps precede the two sealed G1 receipts by seconds.

That is excellent archaeological corroboration. It is not exact identity authority. The historical G1 root contains no executable hash, library hash, compiler identity, toolchain identity, or environment-contract member. Sol's positive binding therefore stops at:

```text
G1_AUTHORITY_OBJECT_IDENTIFIED
CANDIDATE_ARTIFACTS_LOCATED
AUTHORITY_TO_ARTIFACT_BINDING = NOT_ESTABLISHED
```

You found the wall and the likely anchor. The original constitution did not stamp the anchor's serial number into the deed.

---

## To Kammi — assume we found the wrong relic

Your attack succeeded on the exact seam that mattered.

Five substitution routes were tested:

1. **Source match presented as build identity.** Rejected. Source identity is exact; compiled identity was not historically bound.
2. **Timestamp and directory proximity presented as identity.** Rejected. The temporal chain is compelling but not cryptographic authority.
3. **Historical G1 output presented as the frozen FC-01A ABI.** Rejected. Historical G1 returns `StepResult { state, emissions }` and optionally projects into the authorized 04A parity surface. `G1_SEMANTIC_OUTPUT_V1` does not occur in the historical G1 seal.
4. **Fresh rebuild presented as the original callable.** Rejected. That would begin reconstruction or transport, not archaeology.
5. **A new Rust-to-Python wrapper presented as the historical boundary.** Rejected. Such a bridge may be lawful descendant work later, but it is not a recovered G1 identity.

The winning witness is compact:

```text
exact source and plausible original binaries exist
        but
no sealed member binds those binary hashes to G1 authority
```

Your disposition is therefore:

```text
IDENTITY_BINDING_NOT_ESTABLISHED
STOP_IDENTITY_TRANSITION
```

This is not evidence that the candidate is wrong. It is evidence that exactness cannot be earned from the preserved chain of custody.

---

## To Kage — maintain the chain of custody

The closure conjunction is mechanical:

```text
SEALED_G1_AUTHORITY_OBJECT_IDENTIFIED          = TRUE
MATERIALIZED_CANDIDATES_RECOVERED             = TRUE
EVERY_REQUIRED_IDENTITY_DIMENSION_SATISFIED   = FALSE
SOL_POSITIVE_BINDING_PASSED                   = FALSE
KAMMI_FOUND_NO_IDENTITY_COUNTEREXAMPLE        = FALSE
ACCESS_AUDIT_PASSED                           = TRUE
SUBSTITUTIONS_USED                            = FALSE
PROTECTED_POPULATION_CAPABILITIES_EXERCISED   = FALSE
```

Therefore:

```text
G1-M RESULT            = G1_SEALED_CALLABLE_MATERIALIZATION_NOT_EVALUABLE
A1_ATTEMPT_2_ELIGIBLE  = FALSE
FC01A                  = HOLD
FC01B+                 = FROZEN
```

No semantic-equivalence judgment was used. No candidate was promoted because it looked right. No protected population capability was exercised.

Kage's chain-of-custody ruling is that the authority object is identified, the likely compiled relics are located, and the link between them is not strong enough for the positive state.

---

## Shared access audit

```text
REAL_04A_HISTORY_READS             = 0
POPULATION_VALUE_READS             = 0
POPULATION_HISTORY_READS           = 0
POPULATION_DERIVED_CONTENT_READS   = 0
POPULATION_DERIVED_METADATA_READS  = 0
PAIR_READS                         = 0
PAIR_COHORTS_INSTANTIATED          = 0
QUESTION_TOKENS_REALIZED           = 0
ARM_EXECUTIONS                     = 0
TARGET_READS                       = 0
OUTCOME_READS                      = 0
D_B_READS                          = 0
D_C_READS                          = 0
D_D_READS                          = 0
```

Only sealed contracts, source, manifests, receipts, opaque hashes, candidate binary metadata, Cargo fingerprints, and compiler metadata were inspected.

---

## What G1-M bought

G1-M eliminated the earlier false binary choice between “G1 is gone” and “the nearby executable must be G1.” The actual state is narrower:

```text
sealed semantic/source authority      EXISTS
exact source-level callable interface EXISTS
likely historical binaries            EXIST
exact authority-to-binary binding     NOT EVALUABLE
frozen A1 output bridge               NOT HISTORICALLY PRESENT
```

That is an authority transport discontinuity, not a semantic failure.

## Frozen state

```text
LIVE:       none after G1-M seal

HOLD:       FC01A

FROZEN:     FC01B, FC01C, FC01D, FC01E
            P1, P2, P3, P4, P5
            pair parameters and cohorts
            question-token realization
            population admission and execution

BLOCKED:    P6

UNAUTHORIZED:
            G1-R reconstruction
            A1 Attempt 2
```

G1-R is not an automatic fallback. It would require a new protocol, new root, and explicit reconstruction/transport authority.

## Maximum authority

```text
G1_AUTHORITY_OBJECT_AND_CANDIDATE_ARTIFACTS_IDENTIFIED_WITHOUT_EXACT_CALLABLE_BINDING
```

The wall anchor was found. Its chain of custody was not.
