[CmdletBinding()]
param(
    [string]$RepositoryRoot = (Resolve-Path (Join-Path $PSScriptRoot '..\..\..')).Path,
    [string]$OutputDirectory = (Join-Path $PSScriptRoot '..\artifacts\osv1-release-v1'),
    [string]$Executable = '',
    [string]$CargoTargetDirectory = 'D:\northstar-osv1-release-target'
)

$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest

function Sha256([string]$Path) {
    (Get-FileHash -LiteralPath $Path -Algorithm SHA256).Hash.ToLowerInvariant()
}

function Sha256-Bytes([byte[]]$Bytes) {
    $sha = [Security.Cryptography.SHA256]::Create()
    try { ([BitConverter]::ToString($sha.ComputeHash($Bytes))).Replace('-', '').ToLowerInvariant() }
    finally { $sha.Dispose() }
}

function Canonical-Hash($Value) {
    $json = $Value | ConvertTo-Json -Depth 100 -Compress
    Sha256-Bytes ([Text.Encoding]::UTF8.GetBytes($json))
}

function Write-Utf8([string]$Path, [string]$Text) {
    [IO.File]::WriteAllText($Path, $Text, [Text.UTF8Encoding]::new($false))
}

function Relative-Path([string]$Path, [string]$Root) {
    $full = ([IO.Path]::GetFullPath($Path) -replace '\\', '/')
    $base = (([IO.Path]::GetFullPath($Root) -replace '\\', '/').TrimEnd('/') + '/')
    if (!$full.StartsWith($base, [StringComparison]::OrdinalIgnoreCase)) {
        throw "path escapes repository: $full"
    }
    $full.Substring($base.Length)
}

function Authority-Kind([string]$Path) {
    switch -Wildcard ($Path) {
        '*gate16-6-prep/osv1_root_*' { 'OSV1_ROOT_AUTHORITY'; break }
        '*gate16-6-prep/osv1_theta0_*' { 'OBSERVER_INSTANCE_AUTHORITY'; break }
        '*gate16-6-prep/osv1_machine_findings*' { 'FINDINGS_AUTHORITY'; break }
        '*gate16-6-prep/osv1_machine_insufficiencies*' { 'INSUFFICIENCY_AUTHORITY'; break }
        '*gate16-6-prep/osv1_stagewise_parity*' { 'PARITY_AUTHORITY'; break }
        '*gate16-6-prep/osv1_ancestry*' { 'ANCESTRY_AUTHORITY'; break }
        '*gate16-6-prep/osv1_source_dependency*' { 'SOURCE_CLOSURE_AUTHORITY'; break }
        '*gate16-6-prep/osv1_double_rebuild*' { 'DETERMINISM_PROOF'; break }
        '*gate15_5_lineage*' { 'LINEAGE_AUTHORITY'; break }
        '*gate15_5_atlas_*' { 'ATLAS_GRAPH_AUTHORITY'; break }
        '*gate15_5_structural_bridge*' { 'PROVENANCE_AUTHORITY'; break }
        '*gate16_representation_manifests*' { 'REPRESENTATION_CONTRACT_AUTHORITY'; break }
        '*gate16_neighborhood*' { 'NEIGHBORHOOD_AUTHORITY'; break }
        '*gate16*distance*' { 'DISTANCE_CONTRACT_AUTHORITY'; break }
        '*gate16_5_object_authority*' { 'OBJECT_REGISTRY_AUTHORITY'; break }
        '*gate16_5_pair_census*' { 'REPRESENTATION_LOSS_AUTHORITY'; break }
        '*gate16_5_raw_difference*' { 'REPRESENTATION_LOSS_AUTHORITY'; break }
        '*gate16_5*' { 'REPRESENTATION_LOSS_DERIVATION'; break }
        '*gate16*' { 'REPRESENTATION_GEOMETRY_DERIVATION'; break }
        '*gate15*' { 'EXPLORATORY_ATLAS_DERIVATION'; break }
        '*auction-parity*' { 'RG2_ANCESTRY_AUTHORITY'; break }
        default { 'RELEASE_SUPPORT_AUTHORITY' }
    }
}

$repo = [IO.Path]::GetFullPath($RepositoryRoot)
$out = [IO.Path]::GetFullPath($OutputDirectory)
$protocolPath = Join-Path $repo 'research\market-objects\contracts\gate16_7_osv1_release.json'
$rootReceiptPath = Join-Path $repo 'research\market-objects\artifacts\gate16-6-prep\osv1_root_receipt.json'
$thetaReceiptPath = Join-Path $repo 'research\market-objects\artifacts\gate16-6-prep\osv1_theta0_receipt.json'
$findingsReceiptPath = Join-Path $repo 'research\market-objects\artifacts\gate16-6-prep\osv1_machine_findings_receipt.json'
$parityReceiptPath = Join-Path $repo 'research\market-objects\artifacts\gate16-6-prep\osv1_stagewise_parity_matrix_receipt.json'
$dagPath = Join-Path $repo 'research\market-objects\artifacts\gate16-6-prep\osv1_ancestry_dag.json'
foreach ($path in @($protocolPath,$rootReceiptPath,$thetaReceiptPath,$findingsReceiptPath,$parityReceiptPath,$dagPath)) {
    if (!(Test-Path -LiteralPath $path)) { throw "required release authority is absent: $path" }
}
$protocol = Get-Content -Raw -LiteralPath $protocolPath | ConvertFrom-Json
$root = Get-Content -Raw -LiteralPath $rootReceiptPath | ConvertFrom-Json
$theta = Get-Content -Raw -LiteralPath $thetaReceiptPath | ConvertFrom-Json
$findings = Get-Content -Raw -LiteralPath $findingsReceiptPath | ConvertFrom-Json
$parity = Get-Content -Raw -LiteralPath $parityReceiptPath | ConvertFrom-Json
$dag = Get-Content -Raw -LiteralPath $dagPath | ConvertFrom-Json
if ($root.status -ne 'PASS' -or $root.logical_osv1_sha256 -ne $protocol.frozen_identity.osv1_root_sha256) {
    throw 'Gate 16.6 root is not the preregistered OSV1 identity'
}
if ($root.confirmation_windows_opened -ne $false) { throw 'confirmation windows are not frozen' }

$payload = [Collections.Generic.List[string]]::new()
foreach ($group in $protocol.payload_groups) {
    $native = Join-Path $repo (($group -replace '/', '\\').TrimEnd('*').TrimEnd('\'))
    if (!(Test-Path -LiteralPath $native)) { throw "payload group is absent: $group" }
    foreach ($file in Get-ChildItem -LiteralPath $native -File | Sort-Object FullName) {
        $payload.Add((Relative-Path $file.FullName $repo))
    }
}
foreach ($relative in $protocol.individual_payload_authorities) {
    $native = Join-Path $repo ($relative -replace '/', '\\')
    if (!(Test-Path -LiteralPath $native)) { throw "payload authority is absent: $relative" }
    $payload.Add([string]$relative)
}
$payload = @($payload | Sort-Object -Unique)

$entries = foreach ($relative in $payload) {
    $native = Join-Path $repo ($relative -replace '/', '\\')
    [pscustomobject][ordered]@{
        path = $relative
        authority_kind = Authority-Kind $relative
        availability = 'AVAILABLE'
        mutability = 'IMMUTABLE'
        sha256 = Sha256 $native
        bytes = (Get-Item -LiteralPath $native).Length
    }
}

$releaseId = 'OSV1_V1_0_0'
$consumer = [pscustomobject][ordered]@{
    contract = 'NORTHSTAR_GATE16_7_OSV1_CONSUMER_CONTRACT_V1'
    status = 'READ_ONLY'
    release_id = $releaseId
    osv1_root_sha256 = [string]$root.logical_osv1_sha256
    authority_namespace = 'NORTHSTAR_AUTHORITY'
    derived_namespace = 'PHOENIX_DERIVED'
    permissions = @($protocol.consumer_permissions)
    forbidden = @($protocol.consumer_forbidden)
    exported_authorities = @(
        [pscustomobject][ordered]@{ kind='OBJECT_REGISTRY'; path='research/market-objects/artifacts/rg3-gate16-5-loss-final-v1/gate16_5_object_authority.tsv' },
        [pscustomobject][ordered]@{ kind='LINEAGE_RELATIONS'; path='research/market-objects/artifacts/rg3-gate15-5-atlas-final-v5/gate15_5_lineage_coordinates.tsv' },
        [pscustomobject][ordered]@{ kind='REPRESENTATION_CONTRACTS'; path='research/market-objects/artifacts/rg3-gate16-geometry-final-v1/gate16_representation_manifests.json' },
        [pscustomobject][ordered]@{ kind='DISTANCE_CONTRACTS'; path='research/market-objects/artifacts/rg3-gate16-geometry-final-v1/gate16_representation_manifests.json' },
        [pscustomobject][ordered]@{ kind='NEIGHBORHOOD_LEDGERS'; path='research/market-objects/artifacts/rg3-gate16-geometry-final-v1/gate16_neighborhoods.tsv' },
        [pscustomobject][ordered]@{ kind='REPRESENTATION_LOSS_LEDGERS'; path='research/market-objects/artifacts/rg3-gate16-5-loss-final-v1/gate16_5_pair_census.tsv.zst' },
        [pscustomobject][ordered]@{ kind='FINDINGS_LEDGER'; path='research/market-objects/artifacts/gate16-6-prep/osv1_machine_findings.json' },
        [pscustomobject][ordered]@{ kind='INSUFFICIENCY_LEDGER'; path='research/market-objects/artifacts/gate16-6-prep/osv1_machine_insufficiencies.tsv' },
        [pscustomobject][ordered]@{ kind='PROVENANCE'; path='research/market-objects/artifacts/rg3-gate15-5-atlas-final-v5/gate15_5_structural_bridge_receipts.tsv' }
    )
    typed_states = @(
        [pscustomobject][ordered]@{ value=[pscustomobject][ordered]@{kind='NULL'}; class='MISSINGNESS'; transport='TYPED_ENUM'; evidence_path='research/market-objects/artifacts/rg3-gate15-5-atlas-final-v5/gate15_5_lineage_coordinates.tsv'; evidence_token='NULL_STRUCTURAL_CONTEXT' },
        [pscustomobject][ordered]@{ value=[pscustomobject][ordered]@{kind='CENSORED'}; class='CENSORING'; transport='TYPED_ENUM'; evidence_path='research/market-objects/artifacts/rg3-gate15-5-atlas-final-v5/gate15_5_lineage_coordinates.tsv'; evidence_token='CENSORED_DESTINATION' },
        [pscustomobject][ordered]@{ value=[pscustomobject][ordered]@{kind='NOT_APPLICABLE'}; class='AVAILABILITY'; transport='TYPED_ENUM'; evidence_path='research/market-objects/artifacts/rg3-gate15-5-atlas-final-v5/gate15_5_lineage_coordinates.tsv'; evidence_token='NOT_APPLICABLE' },
        [pscustomobject][ordered]@{ value=[pscustomobject][ordered]@{kind='NOT_EVALUABLE'}; class='AVAILABILITY'; transport='TYPED_ENUM'; evidence_path='research/market-objects/artifacts/rg3-gate15-5-atlas-final-v5/gate15_5_auction_interval_intersections.tsv'; evidence_token='NOT_EVALUABLE' },
        [pscustomobject][ordered]@{ value=[pscustomobject][ordered]@{kind='INSUFFICIENT_SUPPORT'}; class='SUPPORT'; transport='TYPED_ENUM'; evidence_path='research/market-objects/artifacts/rg3-gate15-5-atlas-final-v5/gate15_5_conditioned_support_audit.json'; evidence_token='INSUFFICIENT_SUPPORT' },
        [pscustomobject][ordered]@{ value=[pscustomobject][ordered]@{kind='OPEN'}; class='PARITY'; transport='TYPED_ENUM'; evidence_path='research/market-objects/artifacts/gate16-6-prep/osv1_stagewise_parity_matrix.json'; evidence_token='OPEN' },
        [pscustomobject][ordered]@{ value=[pscustomobject][ordered]@{kind='FROZEN_UNOPENED'}; class='CONFIRMATION'; transport='TYPED_ENUM'; evidence_path='research/market-objects/artifacts/rg3-gate15-5-atlas-final-v5/gate15_5_confirmation_protocol.json'; evidence_token='FROZEN_UNOPENED' },
        [pscustomobject][ordered]@{ value=[pscustomobject][ordered]@{kind='SOURCE_RECOVERED'}; class='PROVENANCE'; transport='TYPED_ENUM'; evidence_path='research/market-objects/artifacts/gate16-6-prep/osv1_theta0_fields.json'; evidence_token='RECOVERED_FROM_COLLECTION_ENVIRONMENT' },
        [pscustomobject][ordered]@{ value=[pscustomobject][ordered]@{kind='NOT_RUN_BOUND'}; class='PROVENANCE'; transport='TYPED_ENUM'; evidence_path='research/market-objects/artifacts/gate16-6-prep/osv1_theta0_fields.json'; evidence_token='NOT_RUN_BOUND_NO_EX5_OR_EXPANDED_MASTER_CONFIG_IN_RUN_RECEIPT' }
    )
    source_kinds = @('MACHINE_DERIVED','ARTIFACT_DECLARED','HUMAN_SUMMARY')
    human_summary_authority_count = 0
    theta0_source_recovered_count = 84
    theta0_source_recovered_authority_value = 'RECOVERED_FROM_COLLECTION_ENVIRONMENT'
    theta0_not_run_bound_authority_value = 'NOT_RUN_BOUND_NO_EX5_OR_EXPANDED_MASTER_CONFIG_IN_RUN_RECEIPT'
    source_capsule = [pscustomobject][ordered]@{
        role = 'OPTIONAL_ADJUNCT_REGENERATION_AUTHORITY'
        status = 'ABSENT'
        included_in_osv1_scientific_identity = $false
    }
}

$manifestCore = [pscustomobject][ordered]@{
    contract = 'NORTHSTAR_GATE16_7_OSV1_RELEASE_MANIFEST_V1'
    status = 'SEALED'
    release_id = $releaseId
    osv1_root_sha256 = [string]$root.logical_osv1_sha256
    theta0_sha256 = [string]$theta.logical_theta0_sha256
    ancestry_sha256 = [string]$dag.logical_ancestry_sha256
    findings_sha256 = [string]$findings.logical_findings_sha256
    parity_matrix_sha256 = [string]$parity.logical_matrix_sha256
    cold_rehydration = 'REQUIRED'
    cold_source_regeneration = 'NOT_EVALUABLE_EXTERNAL_MT5_SOURCE_CAPSULE_ABSENT'
    source_regeneration_portability = 'NOT_PORTABLE_WITHOUT_EXTERNAL_SOURCE_CAPSULE'
    source_capsule = 'ABSENT_OPTIONAL_ADJUNCT'
    entries = @($entries)
}
$manifest = [pscustomobject][ordered]@{}
foreach ($property in $manifestCore.psobject.Properties) {
    $manifest | Add-Member -NotePropertyName $property.Name -NotePropertyValue $property.Value
}
$manifest | Add-Member -NotePropertyName logical_release_manifest_sha256 -NotePropertyValue (Canonical-Hash $manifestCore)

[IO.Directory]::CreateDirectory($out) | Out-Null
$consumerPath = Join-Path $out 'osv1_consumer_contract.json'
$manifestPath = Join-Path $out 'osv1_release_manifest.json'
$packPath = Join-Path $out 'osv1_release.pack.zst'
$receiptPath = Join-Path $out 'osv1_release_pack_receipt.json'
Write-Utf8 $consumerPath (($consumer | ConvertTo-Json -Depth 100) + "`n")
Write-Utf8 $manifestPath (($manifest | ConvertTo-Json -Depth 100) + "`n")

if ([string]::IsNullOrWhiteSpace($Executable)) {
    $workspace = Join-Path $repo 'research\market-objects'
    $env:CARGO_TARGET_DIR = $CargoTargetDirectory
    & cargo build --release --manifest-path (Join-Path $workspace 'Cargo.toml') --package northstar-osv1-release --bin northstar-osv1-release
    if ($LASTEXITCODE -ne 0) { throw 'OSV1 release packer build failed' }
    $Executable = Join-Path $CargoTargetDirectory 'release\northstar-osv1-release.exe'
}
if (!(Test-Path -LiteralPath $Executable)) { throw "release packer is absent: $Executable" }
& $Executable pack $repo $manifestPath $consumerPath $packPath $receiptPath
if ($LASTEXITCODE -ne 0) { throw 'OSV1 release packing failed' }
$receipt = Get-Content -Raw -LiteralPath $receiptPath | ConvertFrom-Json
if ($receipt.status -ne 'PASS' -or $receipt.osv1_root_sha256 -ne $root.logical_osv1_sha256) {
    throw 'OSV1 release receipt failed'
}
Write-Output "OSV1_RELEASE status=PASS entries=$($receipt.entry_count) packed_bytes=$($receipt.packed_bytes) pack_sha256=$($receipt.pack_sha256) root=$($receipt.osv1_root_sha256)"
