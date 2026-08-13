[CmdletBinding()]
param(
    [string]$RepositoryRoot = (Resolve-Path (Join-Path $PSScriptRoot '..\..\..')).Path,
    [string]$CanonicalDirectory = (Join-Path $PSScriptRoot '..\artifacts\gate16-6-prep'),
    [string]$ProofPath = (Join-Path $PSScriptRoot '..\artifacts\gate16-6-prep\osv1_double_rebuild_proof.json')
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

$repo = [IO.Path]::GetFullPath($RepositoryRoot)
$canonical = [IO.Path]::GetFullPath($CanonicalDirectory)
$toolRoot = Join-Path $repo 'research\market-objects\tools'
$artifactNames = @(
    'osv1_source_dependency_closure.json',
    'osv1_stagewise_parity_matrix.json',
    'osv1_stagewise_parity_matrix.tsv',
    'osv1_stagewise_parity_matrix_receipt.json',
    'osv1_theta0_fields.json',
    'osv1_theta0_fields.tsv',
    'osv1_theta0_runs.tsv',
    'osv1_theta0_receipt.json',
    'osv1_machine_findings.json',
    'osv1_machine_findings.tsv',
    'osv1_machine_insufficiencies.tsv',
    'osv1_machine_findings_receipt.json',
    'osv1_ancestry_dag.json',
    'osv1_ancestry_nodes.tsv',
    'osv1_ancestry_edges.tsv',
    'osv1_root_manifest.json',
    'osv1_root_receipt.json'
)

function Invoke-Rebuild([string]$OutputDirectory) {
    [IO.Directory]::CreateDirectory($OutputDirectory) | Out-Null
    $closure = Join-Path $OutputDirectory 'osv1_source_dependency_closure.json'
    & (Join-Path $toolRoot 'New-OsV1SourceClosure.ps1') -RepositoryRoot $repo -OutputPath $closure | Out-Null
    & (Join-Path $toolRoot 'New-OsV1ParityMatrix.ps1') -RepositoryRoot $repo -OutputDirectory $OutputDirectory | Out-Null
    & (Join-Path $toolRoot 'New-OsV1Theta0.ps1') -RepositoryRoot $repo -OutputDirectory $OutputDirectory -SourceClosurePath $closure | Out-Null
    & (Join-Path $toolRoot 'New-OsV1MachineFindings.ps1') -RepositoryRoot $repo -OutputDirectory $OutputDirectory | Out-Null
    & (Join-Path $toolRoot 'New-OsV1Ancestry.ps1') -RepositoryRoot $repo -OutputDirectory $OutputDirectory | Out-Null
    foreach ($name in $artifactNames) {
        if (!(Test-Path -LiteralPath (Join-Path $OutputDirectory $name))) {
            throw "rebuild omitted closure artifact: $name"
        }
    }
}

function Compare-Directories([string]$Left, [string]$Right, [string]$Label) {
    $rows = foreach ($name in $artifactNames) {
        $leftPath = Join-Path $Left $name
        $rightPath = Join-Path $Right $name
        if (!(Test-Path -LiteralPath $leftPath) -or !(Test-Path -LiteralPath $rightPath)) {
            throw "missing comparison artifact in ${Label}: $name"
        }
        $leftHash = Sha256 $leftPath
        $rightHash = Sha256 $rightPath
        if ($leftHash -ne $rightHash) { throw "byte mismatch in ${Label}: $name" }
        [pscustomobject][ordered]@{ name=$name; sha256=$leftHash; bytes=(Get-Item -LiteralPath $leftPath).Length }
    }
    @($rows)
}

function Assert-Root([string]$Directory) {
    $dag = Get-Content -Raw -LiteralPath (Join-Path $Directory 'osv1_ancestry_dag.json') | ConvertFrom-Json
    $manifestPath = Join-Path $Directory 'osv1_root_manifest.json'
    $manifest = Get-Content -Raw -LiteralPath $manifestPath | ConvertFrom-Json
    $receipt = Get-Content -Raw -LiteralPath (Join-Path $Directory 'osv1_root_receipt.json') | ConvertFrom-Json
    if ($dag.status -ne 'PASS' -or $dag.branch_count -ne 2 -or $dag.invariants.acyclic -ne $true) {
        throw 'ancestry DAG invariants failed'
    }
    if ($receipt.status -ne 'PASS' -or $receipt.receipt_written_last -ne $true) {
        throw 'root receipt invariants failed'
    }
    if ($receipt.root_manifest_sha256 -ne (Sha256 $manifestPath)) { throw 'root manifest physical hash mismatch' }
    if ($receipt.logical_osv1_sha256 -ne $manifest.logical_osv1_sha256) { throw 'root logical identity mismatch' }
    if ($receipt.logical_ancestry_sha256 -ne $dag.logical_ancestry_sha256) { throw 'ancestry logical identity mismatch' }
}

if (!(Test-Path -LiteralPath $canonical)) { throw 'canonical OSV1 directory is absent' }
$tempBase = [IO.Path]::GetFullPath([IO.Path]::GetTempPath())
$runRoot = Join-Path $tempBase ("northstar-osv1-rebuild-" + [Guid]::NewGuid().ToString('N'))
$runA = Join-Path $runRoot 'A'
$runB = Join-Path $runRoot 'B'
[IO.Directory]::CreateDirectory($runRoot) | Out-Null

try {
    Invoke-Rebuild $runA
    Invoke-Rebuild $runB
    $matching = Compare-Directories $runA $runB 'rebuild-A-vs-B'
    [void](Compare-Directories $runA $canonical 'rebuild-A-vs-canonical')
    Assert-Root $runA
    Assert-Root $runB
    Assert-Root $canonical

    $proofCore = [pscustomobject][ordered]@{
        contract = 'NORTHSTAR_GATE16_6_OSV1_DOUBLE_REBUILD_PROOF_V1'
        status = 'PASS'
        execution_count = 2
        compared_artifact_count = $artifactNames.Count
        byte_mismatch_count = 0
        canonical_match = $true
        closure_artifacts = $matching
        invariants = [pscustomobject][ordered]@{
            rebuild_a_equals_rebuild_b = $true
            rebuild_a_equals_canonical = $true
            ancestry_is_two_branch_dag = $true
            root_receipt_verified = $true
            source_kind_required = $true
            human_summary_authority_count = 0
        }
    }
    $proof = [pscustomobject][ordered]@{}
    foreach ($p in $proofCore.psobject.Properties) { $proof | Add-Member -NotePropertyName $p.Name -NotePropertyValue $p.Value }
    $proof | Add-Member -NotePropertyName logical_proof_sha256 -NotePropertyValue (Canonical-Hash $proofCore)
    Write-Utf8 ([IO.Path]::GetFullPath($ProofPath)) (($proof | ConvertTo-Json -Depth 100) + "`n")
    Write-Output "OSV1_DOUBLE_REBUILD PASS artifacts=$($artifactNames.Count) logical=$($proof.logical_proof_sha256)"
}
finally {
    $resolvedRunRoot = [IO.Path]::GetFullPath($runRoot)
    if ($resolvedRunRoot.StartsWith($tempBase, [StringComparison]::OrdinalIgnoreCase) -and (Split-Path -Leaf $resolvedRunRoot).StartsWith('northstar-osv1-rebuild-')) {
        [IO.Directory]::Delete($resolvedRunRoot, $true)
    }
}
