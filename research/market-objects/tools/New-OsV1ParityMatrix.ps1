[CmdletBinding()]
param(
    [string]$RepositoryRoot = (Resolve-Path (Join-Path $PSScriptRoot '..\..\..')).Path,
    [string]$OutputDirectory = (Join-Path $PSScriptRoot '..\artifacts\gate16-6-prep')
)

$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest

function Relative-Path([string]$Path, [string]$Root) {
    $full = ([IO.Path]::GetFullPath($Path) -replace '\\', '/')
    $base = (([IO.Path]::GetFullPath($Root) -replace '\\', '/').TrimEnd('/') + '/')
    if (!$full.StartsWith($base, [StringComparison]::OrdinalIgnoreCase)) {
        throw "evidence escapes repository: $full"
    }
    return $full.Substring($base.Length)
}

function Sha256([string]$Path) {
    return (Get-FileHash -LiteralPath $Path -Algorithm SHA256).Hash.ToLowerInvariant()
}

function Sha256-Bytes([byte[]]$Bytes) {
    $sha = [Security.Cryptography.SHA256]::Create()
    try { return ([BitConverter]::ToString($sha.ComputeHash($Bytes))).Replace('-', '').ToLowerInvariant() }
    finally { $sha.Dispose() }
}

function Canonical-Hash($Value) {
    $json = $Value | ConvertTo-Json -Depth 100 -Compress
    return Sha256-Bytes ([Text.Encoding]::UTF8.GetBytes($json))
}

function Evidence([string]$Id, [string]$Relative, [string]$ExpectedStatus, [string]$Claim) {
    $path = Join-Path $RepositoryRoot $Relative
    if (!(Test-Path -LiteralPath $path -PathType Leaf)) { throw "missing evidence: $path" }
    $extension = [IO.Path]::GetExtension($path).ToLowerInvariant()
    $contract = $null
    $actualStatus = $null
    if ($extension -eq '.json') {
        $value = Get-Content -Raw -LiteralPath $path | ConvertFrom-Json
        if ($null -ne $value.PSObject.Properties['contract']) { $contract = $value.contract }
        if ($null -ne $value.PSObject.Properties['status']) { $actualStatus = $value.status }
        if ($actualStatus -ne $ExpectedStatus) {
            throw "evidence $Id expected status $ExpectedStatus but found $actualStatus"
        }
    }
    else {
        $text = Get-Content -Raw -LiteralPath $path
        if (!$text.Contains($ExpectedStatus)) {
            throw "text evidence $Id does not contain required marker: $ExpectedStatus"
        }
        $actualStatus = "TEXT_MARKER:$ExpectedStatus"
    }
    return [ordered]@{
        evidence_id = $Id
        path = Relative-Path $path $RepositoryRoot
        sha256 = Sha256 $path
        contract = $contract
        observed_status = $actualStatus
        supports = $Claim
    }
}

function Row(
    [int]$Order,
    [string]$StageId,
    [string]$ClaimClass,
    [string]$InputBoundary,
    [string]$Mt5Role,
    [string]$RustRole,
    [string]$Status,
    [string]$ComparedOutputs,
    [string]$Scope,
    [string]$ExplicitExclusion,
    [string]$EvidenceCsv
) {
    return [ordered]@{
        order = $Order
        stage_id = $StageId
        claim_class = $ClaimClass
        input_boundary = $InputBoundary
        mt5_role = $Mt5Role
        rust_role = $RustRole
        status = $Status
        compared_outputs = $ComparedOutputs
        certified_scope = $Scope
        explicit_exclusion = $ExplicitExclusion
        evidence_ids = @($EvidenceCsv.Split(',', [StringSplitOptions]::RemoveEmptyEntries))
    }
}

$evidence = @(
    (Evidence 'P12_BD_MACHINE' 'research\auction-parity\proof\phase12_bd_transit_parity_receipt.json' 'PASS' 'exact C2-C5 topology and D reconstruction'),
    (Evidence 'P12_BD_CERTIFICATE' 'research\auction-parity\PHASE12_BD_CERTIFICATE.md' 'Phase 12C.1 remains explicitly open' 'human-readable proven boundary and C1 exclusion'),
    (Evidence 'P12_CHECKPOINT' 'research\auction-parity\PHASE12_CHECKPOINT.md' 'C1 OPEN' 'aggregate Phase 12 checkpoint'),
    (Evidence 'MT5_ORACLE' 'research\auction-parity\proof\mt5_bd_transit_oracle_receipt.json' 'PASS' 'MT5 comparison-oracle population and hashes'),
    (Evidence 'MT5_ORACLE_DETERMINISM' 'research\auction-parity\proof\mt5_fresh_oracle_determinism.json' 'MATCH' 'fresh repeated MT5 capture semantic determinism'),
    (Evidence 'P11_INFERENCE' 'research\auction-parity\proof\model_inference_parity.json' 'PASS' 'exact Rust inference parity for four frozen behavioral candidates'),
    (Evidence 'G13R_CONFIRMATION' 'research\auction-parity\artifacts\phase13r-evaluation\certificate_manifest.json' 'COMPLETE' 'one-shot behavioral holdout transport'),
    (Evidence 'RG3_CORPUS' 'research\market-objects\artifacts\rg3-gate15-seal-final-v4\gate15-corpus-manifest.json' 'SEALED' '42-run RG3 raw corpus and derived replay identity'),
    (Evidence 'G15_5_ATLAS' 'research\market-objects\artifacts\rg3-gate15-5-atlas-final-v5\gate15_5_receipt.json' 'PASS' 'deterministic multiview atlas and causal structural bridge'),
    (Evidence 'G16_REPLAY' 'research\market-objects\artifacts\rg3-gate16-seal-final-v1\gate16-replay-proof.json' 'PASS' 'representation and distance rebuild determinism'),
    (Evidence 'G16_5_REPLAY' 'research\market-objects\artifacts\rg3-gate16-5-seal-final-v1\gate16_5-replay-proof.json' 'PASS' 'representation-loss census rebuild determinism')
)

$matrix = @(
    (Row 10 'C1_RAW_STRUCTURAL_PRODUCERS' 'INDEPENDENT_RECONSTRUCTION_PARITY' 'raw historical bars plus producer warmup/state' 'AUTHORITATIVE_PRODUCER' 'NO_INDEPENDENT_IMPLEMENTATION_ADMITTED' 'OPEN' 'none' 'VolKitt, profile, day-swings, and Wayne producer semantics' 'no claim of Rust C1 reconstruction or parity' 'P12_BD_MACHINE,P12_BD_CERTIFICATE,P12_CHECKPOINT'),
    (Row 20 'C2_NORMALIZATION_AND_CANONICAL_ORDERING' 'INDEPENDENT_RECONSTRUCTION_PARITY' 'MT5 raw producer-output tape' 'COMPARISON_ORACLE' 'INDEPENDENT_RECONSTRUCTION' 'PASS_EXACT' '487662 normalized levels over 5155 frames' 'bounded US30 M5 development oracle' 'does not cover producer calculations before the tape boundary' 'P12_BD_MACHINE,MT5_ORACLE'),
    (Row 30 'C3_COMPATIBILITY_AND_DBSCAN' 'INDEPENDENT_RECONSTRUCTION_PARITY' 'canonical normalized levels' 'COMPARISON_ORACLE' 'INDEPENDENT_RECONSTRUCTION' 'PASS_EXACT' '1554 DBSCAN rebuilds' 'compatibility-gated one-dimensional clustering on bounded US30 M5 oracle' 'no alternative epsilon, compatibility, or observer configuration coverage' 'P12_BD_MACHINE,MT5_ORACLE'),
    (Row 40 'C4_NODE_PROVENANCE_AND_LIFECYCLE' 'INDEPENDENT_RECONSTRUCTION_PARITY' 'cluster memberships and normalized producer provenance' 'COMPARISON_ORACLE' 'INDEPENDENT_RECONSTRUCTION' 'PASS_EXACT' '40459 stable-node rows and 283419 provenance rows' 'weighted nodes, provenance, stable identity and lifecycle on bounded US30 M5 oracle' 'no claim beyond admitted node/lifecycle semantics' 'P12_BD_MACHINE,MT5_ORACLE'),
    (Row 50 'C5_CORRIDOR_AND_REGIONAL_STATE' 'INDEPENDENT_RECONSTRUCTION_PARITY' 'stable topology and reference ATR' 'COMPARISON_ORACLE' 'INDEPENDENT_RECONSTRUCTION' 'PASS_EXACT' '5155 regional snapshots' 'corridors, median, COG, and population-sigma regional state on bounded US30 M5 oracle' 'no auction-interval bridge and no cross-scale transport claim' 'P12_BD_MACHINE,MT5_ORACLE'),
    (Row 60 'D1_CAUSAL_FROZEN_FEATURES' 'INDEPENDENT_RECONSTRUCTION_PARITY' 'causal frame before auction observation' 'COMPARISON_ORACLE' 'INDEPENDENT_RECONSTRUCTION' 'PASS_EXACT_CAUSAL_FRAME' '5155 causal feature snapshots' 'feature/frame bijection and frozen context on bounded US30 M5 oracle' 'no future information and no C1 reconstruction claim' 'P12_BD_MACHINE,MT5_ORACLE'),
    (Row 70 'D2_AUCTION_STATE_AND_RELATIONAL_LEDGER' 'INDEPENDENT_RECONSTRUCTION_PARITY' 'reconstructed topology, region, and causal features' 'BEHAVIORAL_ORACLE' 'INDEPENDENT_RECONSTRUCTION' 'PASS_EXACT' '643 events, 145 attempts, and 52 episodes' 'event order, relational rows, censoring, and terminal hash on bounded US30 M5 oracle' 'behavioral ontology only; no economic or trading authority' 'P12_BD_MACHINE'),
    (Row 80 'D3_TRANSIT_RECEIPTS' 'INDEPENDENT_RECONSTRUCTION_PARITY' 'resolved auction departure and adjacent-node topology' 'BEHAVIORAL_ORACLE' 'INDEPENDENT_RECONSTRUCTION' 'PASS_EXACT' '7 completed transit receipts' 'transit-bearing five-session bounded US30 M5 oracle' 'small certified population; no general transition law claimed' 'P12_BD_MACHINE'),
    (Row 90 'P0_MT5_CAPTURE_REPLAY' 'DETERMINISM_NOT_PARITY' 'same bounded market window and observer instance' 'AUTHORITATIVE_CAPTURE' 'VALIDATOR_ONLY' 'PASS_MATCH' 'canonical semantic capture and frame prefixes' 'two fresh headless invocations' 'does not independently reconstruct producer semantics' 'MT5_ORACLE_DETERMINISM'),
    (Row 100 'P11_FROZEN_MODEL_INFERENCE' 'IMPLEMENTATION_PARITY_NOT_MARKET_CONFIRMATION' 'sealed Phase 11 rows and frozen model artifacts' 'REFERENCE_ARTIFACT' 'INDEPENDENT_INFERENCE' 'PASS_EXACT' 'scores and probabilities for four frozen candidates' 'maximum score and probability error 0.0 on sealed development rows' 'does not establish holdout transport, economics, or policy' 'P11_INFERENCE'),
    (Row 110 'G13R_BEHAVIORAL_HOLDOUT' 'CONFIRMATION_NOT_IMPLEMENTATION_PARITY' 'twelve prospectively authorized recovery windows' 'SEALED_DATA_SOURCE' 'FROZEN_EVALUATOR' 'PASS_FOUR_CANDIDATES' 'initial acceptance, reclaim, rejection, and retest hold' 'one-shot behavioral transport under frozen contracts' 'not mechanism, economic usefulness, policy, or deployment authority' 'G13R_CONFIRMATION'),
    (Row 120 'RG3_RAW_MARKET_OBJECT_CORPUS' 'OBSERVATIONAL_AUTHORITY_AND_SEALING' 'closed-bar MT5 compression, expansion, and structural observations' 'RAW_OBSERVATIONAL_AUTHORITY' 'VALIDATION_PACKING_AND_DERIVATION' 'PASS_SEALED' '42 runs and 1091 objects' 'sealed RG3 corpus under one admitted configuration hash' 'not an independent Rust implementation of MT5 object sensors' 'RG3_CORPUS'),
    (Row 130 'G15_TO_G16_5_DERIVED_SCIENCE' 'DERIVED_REBUILD_DETERMINISM_NOT_MT5_PARITY' 'sealed RG3 corpus plus frozen representation contracts' 'NO_RUNTIME_ROLE' 'DERIVED_SCIENTIFIC_AUTHORITY' 'PASS_DETERMINISTIC' 'atlas, representation/distance laboratory, and loss census' 'deterministic derived artifacts through Gate 16.5' 'no universal taxonomy, preferred representation, mechanism, economics, or trading meaning' 'RG3_CORPUS,G15_5_ATLAS,G16_REPLAY,G16_5_REPLAY')
)

$evidenceIds = [Collections.Generic.HashSet[string]]::new([StringComparer]::Ordinal)
foreach ($item in $evidence) {
    if (!$evidenceIds.Add($item.evidence_id)) { throw "duplicate evidence id: $($item.evidence_id)" }
}
foreach ($row in $matrix) {
    foreach ($id in $row.evidence_ids) {
        if (!$evidenceIds.Contains($id)) { throw "stage $($row.stage_id) cites missing evidence $id" }
    }
}

$core = [ordered]@{
    contract = 'NORTHSTAR_GATE16_6_OSV1_STAGEWISE_PARITY_MATRIX_V1'
    status = 'PASS'
    scope = 'stage-qualified implementation parity plus explicitly separated adjacent evidence'
    primary_parity_boundary = 'raw structural producer output'
    primary_population = 'bounded US30 M5 five-session oracle with 5155 frames'
    stage_count = $matrix.Count
    evidence_count = $evidence.Count
    stages = $matrix
    evidence = $evidence
    invariants = [ordered]@{
        c1_status = 'OPEN'
        c2_d_independent_reconstruction = 'PASS'
        adjacent_confirmation_not_called_parity = $true
        rg3_determinism_not_called_mt5_parity = $true
        economic_authority = $false
        trading_authority = $false
        confirmation_windows_opened = $false
    }
}
$core.logical_matrix_sha256 = Canonical-Hash $core

$output = [IO.Path]::GetFullPath($OutputDirectory)
New-Item -ItemType Directory -Force -Path $output | Out-Null
$jsonPath = Join-Path $output 'osv1_stagewise_parity_matrix.json'
$tsvPath = Join-Path $output 'osv1_stagewise_parity_matrix.tsv'
$receiptPath = Join-Path $output 'osv1_stagewise_parity_matrix_receipt.json'

[IO.File]::WriteAllText($jsonPath, ($core | ConvertTo-Json -Depth 100) + "`n", [Text.UTF8Encoding]::new($false))

$writer = [IO.StreamWriter]::new($tsvPath, $false, [Text.UTF8Encoding]::new($false), 262144)
try {
    $writer.WriteLine('order' + "`t" + 'stage_id' + "`t" + 'claim_class' + "`t" + 'input_boundary' + "`t" + 'mt5_role' + "`t" + 'rust_role' + "`t" + 'status' + "`t" + 'compared_outputs' + "`t" + 'certified_scope' + "`t" + 'explicit_exclusion' + "`t" + 'evidence_ids')
    foreach ($row in $matrix) {
        $values = @($row.order,$row.stage_id,$row.claim_class,$row.input_boundary,$row.mt5_role,$row.rust_role,$row.status,$row.compared_outputs,$row.certified_scope,$row.explicit_exclusion,($row.evidence_ids -join ','))
        if (($values -join '') -match "[`t`r`n]") { throw "TSV control character in stage $($row.stage_id)" }
        $writer.WriteLine($values -join "`t")
    }
}
finally { $writer.Dispose() }

$receipt = [ordered]@{
    contract = 'NORTHSTAR_GATE16_6_OSV1_STAGEWISE_PARITY_MATRIX_RECEIPT_V1'
    status = 'PASS'
    logical_matrix_sha256 = $core.logical_matrix_sha256
    json_sha256 = Sha256 $jsonPath
    tsv_sha256 = Sha256 $tsvPath
    stage_count = $matrix.Count
    evidence_count = $evidence.Count
    c1_open = $true
    c2_d_pass = $true
    confirmation_windows_opened = $false
}
[IO.File]::WriteAllText($receiptPath, ($receipt | ConvertTo-Json -Depth 20) + "`n", [Text.UTF8Encoding]::new($false))
Write-Output "OSV1_PARITY_MATRIX stages=$($matrix.Count) evidence=$($evidence.Count) logical_sha256=$($core.logical_matrix_sha256)"
