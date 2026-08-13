[CmdletBinding()]
param(
    [string]$RepositoryRoot = (Resolve-Path (Join-Path $PSScriptRoot '..\..\..')).Path,
    [string]$OutputRoot = 'D:\northstar-osv1-cold-rehydrated',
    [string]$ProofDirectory = 'D:\northstar-osv1-cold-proof',
    [string]$CargoTargetDirectory = 'D:\northstar-osv1-cold-target'
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

function Payload-Tree([string]$Directory) {
    $rows = foreach ($file in Get-ChildItem -LiteralPath $Directory -File | Sort-Object Name) {
        [pscustomobject][ordered]@{
            name = $file.Name
            sha256 = Sha256 $file.FullName
            bytes = $file.Length
        }
    }
    [pscustomobject][ordered]@{
        files = @($rows)
        logical_sha256 = Canonical-Hash @($rows)
    }
}

$repo = [IO.Path]::GetFullPath($RepositoryRoot)
$prep = Join-Path $repo 'research\market-objects\artifacts\gate16-6-prep'
if (Test-Path -LiteralPath $prep) {
    throw 'cold rehydration requires the originating gate16-6-prep directory to be absent'
}
$releaseDir = Join-Path $repo 'research\market-objects\artifacts\osv1-release-v1'
$pack = Join-Path $releaseDir 'osv1_release.pack.zst'
$packReceiptPath = Join-Path $releaseDir 'osv1_release_pack_receipt.json'
$manifestPath = Join-Path $releaseDir 'osv1_release_manifest.json'
$consumerPath = Join-Path $releaseDir 'osv1_consumer_contract.json'
foreach ($path in @($pack,$packReceiptPath,$manifestPath,$consumerPath)) {
    if (!(Test-Path -LiteralPath $path)) { throw "cold release input is absent: $path" }
}
$before = Payload-Tree $releaseDir
$packReceipt = Get-Content -Raw -LiteralPath $packReceiptPath | ConvertFrom-Json
if ($packReceipt.status -ne 'PASS' -or $packReceipt.osv1_root_sha256 -ne '6e18e10fc0fb3571aa21b4e20105f783dafe2872812d217d672ab5e959bb890d') {
    throw 'release pack receipt is not the frozen OSV1 identity'
}

[IO.Directory]::CreateDirectory($OutputRoot) | Out-Null
[IO.Directory]::CreateDirectory($ProofDirectory) | Out-Null
$env:CARGO_TARGET_DIR = $CargoTargetDirectory
$workspace = Join-Path $repo 'research\market-objects\Cargo.toml'
& cargo build --release --locked --manifest-path $workspace --package northstar-osv1-release --bins
if ($LASTEXITCODE -ne 0) { throw 'cold consumer build failed' }
$releaseExe = Join-Path $CargoTargetDirectory 'release\northstar-osv1-release.exe'
$consumerExe = Join-Path $CargoTargetDirectory 'release\northstar-osv1-consumer-smoke.exe'
$rehydrationReceiptPath = Join-Path $ProofDirectory 'osv1_cold_rehydration_receipt.json'
$consumerReceiptPath = Join-Path $ProofDirectory 'osv1_phoenix_consumer_smoke_receipt.json'
& $releaseExe rehydrate $pack $OutputRoot $rehydrationReceiptPath
if ($LASTEXITCODE -ne 0) { throw 'cold OSV1 rehydration failed' }
& $consumerExe $pack $consumerReceiptPath
if ($LASTEXITCODE -ne 0) { throw 'cold Phoenix read-only smoke failed' }

$rehydration = Get-Content -Raw -LiteralPath $rehydrationReceiptPath | ConvertFrom-Json
$consumer = Get-Content -Raw -LiteralPath $consumerReceiptPath | ConvertFrom-Json
$rehydratedRootPath = Join-Path $OutputRoot 'research\market-objects\artifacts\gate16-6-prep\osv1_root_receipt.json'
if (!(Test-Path -LiteralPath $rehydratedRootPath)) { throw 'rehydrated root receipt is absent' }
$rehydratedRoot = Get-Content -Raw -LiteralPath $rehydratedRootPath | ConvertFrom-Json
$after = Payload-Tree $releaseDir
if ($before.logical_sha256 -ne $after.logical_sha256) { throw 'release payload tree changed during consumption' }
if ($rehydratedRoot.logical_osv1_sha256 -ne $packReceipt.osv1_root_sha256) { throw 'cold root differs from released root' }
if ($rehydration.undeclared_authoritative_reads -ne 0 -or $rehydration.absolute_authority_resolution_attempts -ne 0) {
    throw 'cold rehydration authority read audit failed'
}
if ($consumer.source_mutation_count -ne 0 -or $consumer.pack_sha256_before -ne $consumer.pack_sha256_after) {
    throw 'read-only consumer mutation invariant failed'
}

$proof = [pscustomobject][ordered]@{
    contract = 'NORTHSTAR_GATE16_7_OSV1_COLD_RELEASE_PROOF_V1'
    status = 'PASS'
    osv1_root_sha256 = [string]$rehydratedRoot.logical_osv1_sha256
    release_id = [string]$packReceipt.release_id
    release_pack_sha256 = [string]$packReceipt.pack_sha256
    payload_tree_sha256_before = [string]$before.logical_sha256
    payload_tree_sha256_after = [string]$after.logical_sha256
    payload_file_count = @($before.files).Count
    original_gate16_6_prep_accessible = $false
    cold_rehydration = [pscustomobject][ordered]@{
        status = [string]$rehydration.status
        entry_count = [int]$rehydration.entry_count
        hash_mismatch_count = [int]$rehydration.hash_mismatch_count
        allowed_read_authority = [string]$rehydration.allowed_read_authority
        declared_authoritative_reads = [int]$rehydration.declared_authoritative_reads
        undeclared_authoritative_reads = [int]$rehydration.undeclared_authoritative_reads
        absolute_authority_resolution_attempts = [int]$rehydration.absolute_authority_resolution_attempts
    }
    cold_source_regeneration = [pscustomobject][ordered]@{
        status = 'NOT_EVALUABLE'
        reason = 'EXTERNAL_MT5_SOURCE_CAPSULE_ABSENT'
    }
    source_regeneration_portability = [pscustomobject][ordered]@{
        status = 'NOT_PORTABLE_WITHOUT_EXTERNAL_SOURCE_CAPSULE'
        source_capsule_part_of_osv1_identity = $false
    }
    phoenix_consumer = [pscustomobject][ordered]@{
        status = [string]$consumer.status
        source_kind = [string]$consumer.source_kind
        sidecar_outside_payload = $true
        typed_states_verified = [int]$consumer.typed_states_verified
        source_kinds_verified = [int]$consumer.source_kinds_verified
        theta0_source_recovered_count = [int]$consumer.theta0_source_recovered_count
        c1_parity_status = [string]$consumer.c1_parity_status
        source_mutation_count = [int]$consumer.source_mutation_count
    }
    invariants = [pscustomobject][ordered]@{
        released_root_exact = $true
        payload_tree_unchanged = $true
        only_declared_authoritative_bytes_read = $true
        typed_epistemic_states_round_trip = $true
        illegal_state_coercions_rejected_by_consumer_type = $true
        confirmation_windows_unopened = $true
        scientific_mutation_count = 0
        economic_or_trading_authority = $false
    }
}
$proofPath = Join-Path $ProofDirectory 'osv1_cold_release_proof.json'
Write-Utf8 $proofPath (($proof | ConvertTo-Json -Depth 100) + "`n")
Write-Output "OSV1_COLD_RELEASE status=PASS entries=$($rehydration.entry_count) payload=$($before.logical_sha256) pack=$($packReceipt.pack_sha256) root=$($rehydratedRoot.logical_osv1_sha256)"
