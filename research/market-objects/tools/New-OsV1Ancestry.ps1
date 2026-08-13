[CmdletBinding()]
param(
    [string]$RepositoryRoot = (Resolve-Path (Join-Path $PSScriptRoot '..\..\..')).Path,
    [string]$OutputDirectory = (Join-Path $PSScriptRoot '..\artifacts\gate16-6-prep')
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

function Relative-Path([string]$Path, [string]$Root) {
    $full = ([IO.Path]::GetFullPath($Path) -replace '\\', '/')
    $base = (([IO.Path]::GetFullPath($Root) -replace '\\', '/').TrimEnd('/') + '/')
    if (!$full.StartsWith($base, [StringComparison]::OrdinalIgnoreCase)) {
        throw "path escapes repository: $full"
    }
    $full.Substring($base.Length)
}

function Write-Utf8([string]$Path, [string]$Text) {
    [IO.File]::WriteAllText($Path, $Text, [Text.UTF8Encoding]::new($false))
}

function Read-Evidence([string]$RelativePath, [string]$ExpectedContract, [string[]]$AllowedStatus) {
    $path = Join-Path $repo ($RelativePath -replace '/', '\\')
    if (!(Test-Path -LiteralPath $path)) { throw "missing ancestry evidence: $RelativePath" }
    $json = Get-Content -Raw -LiteralPath $path | ConvertFrom-Json
    if ([string]$json.contract -ne $ExpectedContract) {
        throw "ancestry contract mismatch for ${RelativePath}: $($json.contract)"
    }
    if ($AllowedStatus -notcontains [string]$json.status) {
        throw "ancestry status mismatch for ${RelativePath}: $($json.status)"
    }
    [pscustomobject][ordered]@{
        path = $RelativePath
        sha256 = Sha256 $path
        contract = [string]$json.contract
        status = [string]$json.status
    }
}

function New-Node(
    [string]$Id,
    [string]$Branch,
    [string]$Stage,
    [string]$Role,
    [object[]]$Evidence
) {
    [pscustomobject][ordered]@{
        node_id = $Id
        branch = $Branch
        stage = $Stage
        authority_role = $Role
        evidence = @($Evidence)
    }
}

function Assert-Dag([object[]]$Nodes, [object[]]$Edges) {
    $ids = @($Nodes | ForEach-Object { [string]$_.node_id })
    if (($ids | Sort-Object -Unique).Count -ne $ids.Count) { throw 'duplicate ancestry node identity' }
    $known = @{}; foreach ($id in $ids) { $known[$id] = $true }
    $indegree = @{}; $outgoing = @{}
    foreach ($id in $ids) { $indegree[$id] = 0; $outgoing[$id] = [Collections.Generic.List[string]]::new() }
    foreach ($edge in $Edges) {
        $from = [string]$edge.from_node; $to = [string]$edge.to_node
        if (!$known.ContainsKey($from) -or !$known.ContainsKey($to)) { throw "edge references unknown node: $from -> $to" }
        $indegree[$to] = [int]$indegree[$to] + 1
        $outgoing[$from].Add($to)
    }
    $queue = [Collections.Generic.Queue[string]]::new()
    foreach ($id in ($ids | Sort-Object)) { if ([int]$indegree[$id] -eq 0) { $queue.Enqueue($id) } }
    $visited = 0
    while ($queue.Count -gt 0) {
        $id = $queue.Dequeue(); $visited++
        foreach ($to in $outgoing[$id]) {
            $indegree[$to] = [int]$indegree[$to] - 1
            if ([int]$indegree[$to] -eq 0) { $queue.Enqueue($to) }
        }
    }
    if ($visited -ne $ids.Count) { throw 'ancestry graph contains a cycle' }
}

$repo = [IO.Path]::GetFullPath($RepositoryRoot)
$out = [IO.Path]::GetFullPath($OutputDirectory)
[IO.Directory]::CreateDirectory($out) | Out-Null

$rg2Corpus = Read-Evidence 'research/auction-parity/proof/rg2_corpus_parity.json' 'NORTHSTAR_MT5_CORPUS_PARITY_V1' @('PASS')
$rg2Interface = Read-Evidence 'research/auction-parity/proof/phase10_5_interface_parity.json' 'NORTHSTAR_PHASE10_5_PARITY_V1' @('PASS')
$rg2Models = Read-Evidence 'research/auction-parity/artifacts/phase12-freeze/frozen_model_registry.json' 'MST_PHASE11_FROZEN_MODEL_REGISTRY_V1' @('PASS')
$rg2Inference = Read-Evidence 'research/auction-parity/proof/model_inference_parity.json' 'NORTHSTAR_PHASE11_MODEL_INFERENCE_PARITY_V1' @('PASS')
$rg2Gate13 = Read-Evidence 'research/auction-parity/artifacts/phase13r-evaluation/certificate_manifest.json' 'NORTHSTAR_PHASE13R_CERTIFICATE_V1' @('COMPLETE')

$rg3Gate14 = Read-Evidence 'research/market-objects/artifacts/gate14-build-proof.json' 'NORTHSTAR_RG3_GATE14_BUILD_PROOF_V1' @('PASS')
$rg3Gate15 = Read-Evidence 'research/market-objects/artifacts/rg3-gate15-seal-final-v4/gate15-seal-receipt.json' 'NORTHSTAR_RG3_GATE15_SEAL_RECEIPT_V1' @('PASS')
$rg3Gate155 = Read-Evidence 'research/market-objects/artifacts/rg3-gate15-5-atlas-final-v5/gate15_5_receipt.json' 'NORTHSTAR_RG3_GATE15_5_ATLAS_RECEIPT_V1' @('PASS')
$rg3Gate16 = Read-Evidence 'research/market-objects/artifacts/rg3-gate16-seal-final-v1/gate16-seal-receipt.json' 'NORTHSTAR_RG3_GATE16_SEAL_RECEIPT_V1' @('PASS')
$rg3Gate165 = Read-Evidence 'research/market-objects/artifacts/rg3-gate16-5-seal-final-v1/gate16_5-seal-receipt.json' 'NORTHSTAR_RG3_GATE16_5_SEAL_RECEIPT_V1' @('PASS_WITH_OBSERVATIONAL_FINDINGS')

$nodes = @(
    New-Node 'RG2_PHASE10' 'RG2' 'PHASE_10_CORPUS_SEAL' 'EMPIRICAL_CORPUS_AND_PARITY_FOUNDATION' @($rg2Corpus)
    New-Node 'RG2_PHASE10_5' 'RG2' 'PHASE_10_5_INTERFACE' 'CAUSAL_RESEARCH_INTERFACE' @($rg2Interface)
    New-Node 'RG2_PHASE11' 'RG2' 'PHASE_11_BEHAVIORAL_MODELS' 'FROZEN_MODEL_AND_INFERENCE_EVIDENCE' @($rg2Models, $rg2Inference)
    New-Node 'RG2_GATE13R' 'RG2' 'GATE_13R_HOLDOUT' 'BEHAVIORAL_HOLDOUT_CERTIFICATE' @($rg2Gate13)
    New-Node 'RG3_GATE14' 'RG3' 'GATE_14_OBJECT_INSTRUMENTS' 'DETERMINISTIC_MARKET_OBJECT_BUILD' @($rg3Gate14)
    New-Node 'RG3_GATE15' 'RG3' 'GATE_15_DISCOVERY' 'EXPLORATORY_FAMILY_SYSTEM_SEAL' @($rg3Gate15)
    New-Node 'RG3_GATE15_5' 'RG3' 'GATE_15_5_ATLAS' 'MULTIVIEW_ATLAS_SEAL' @($rg3Gate155)
    New-Node 'RG3_GATE16' 'RG3' 'GATE_16_GEOMETRY' 'REPRESENTATION_AND_DISTANCE_SEAL' @($rg3Gate16)
    New-Node 'RG3_GATE16_5' 'RG3' 'GATE_16_5_LOSS_CENSUS' 'REPRESENTATION_LOSS_SEAL' @($rg3Gate165)
    New-Node 'OSV1_ROOT' 'OSV1' 'GATE_16_6_CLOSURE' 'TWO_BRANCH_CLOSURE_ROOT' @()
)

$edges = @(
    [pscustomobject][ordered]@{ edge_id='E01'; from_node='RG2_PHASE10'; to_node='RG2_PHASE10_5'; relation='PRECEDES' }
    [pscustomobject][ordered]@{ edge_id='E02'; from_node='RG2_PHASE10_5'; to_node='RG2_PHASE11'; relation='PRECEDES' }
    [pscustomobject][ordered]@{ edge_id='E03'; from_node='RG2_PHASE11'; to_node='RG2_GATE13R'; relation='PRECEDES' }
    [pscustomobject][ordered]@{ edge_id='E04'; from_node='RG3_GATE14'; to_node='RG3_GATE15'; relation='PRECEDES' }
    [pscustomobject][ordered]@{ edge_id='E05'; from_node='RG3_GATE15'; to_node='RG3_GATE15_5'; relation='PRECEDES' }
    [pscustomobject][ordered]@{ edge_id='E06'; from_node='RG3_GATE15_5'; to_node='RG3_GATE16'; relation='PRECEDES' }
    [pscustomobject][ordered]@{ edge_id='E07'; from_node='RG3_GATE16'; to_node='RG3_GATE16_5'; relation='PRECEDES' }
    [pscustomobject][ordered]@{ edge_id='E08'; from_node='RG2_GATE13R'; to_node='OSV1_ROOT'; relation='CONVERGES_INTO' }
    [pscustomobject][ordered]@{ edge_id='E09'; from_node='RG3_GATE16_5'; to_node='OSV1_ROOT'; relation='CONVERGES_INTO' }
)
Assert-Dag $nodes $edges

$dagCore = [pscustomobject][ordered]@{
    contract = 'NORTHSTAR_GATE16_6_OSV1_ANCESTRY_DAG_V1'
    status = 'PASS'
    shape = 'TWO_BRANCH_DAG'
    branch_count = 2
    root_node_id = 'OSV1_ROOT'
    nodes = $nodes
    edges = $edges
    invariants = [pscustomobject][ordered]@{
        acyclic = $true
        all_evidence_present = $true
        all_evidence_contracts_match = $true
        rg2_leaf = 'RG2_GATE13R'
        rg3_leaf = 'RG3_GATE16_5'
        root_indegree = 2
    }
}
$dag = [pscustomobject][ordered]@{}
foreach ($p in $dagCore.psobject.Properties) { $dag | Add-Member -NotePropertyName $p.Name -NotePropertyValue $p.Value }
$dag | Add-Member -NotePropertyName logical_ancestry_sha256 -NotePropertyValue (Canonical-Hash $dagCore)

$dagPath = Join-Path $out 'osv1_ancestry_dag.json'
$nodesPath = Join-Path $out 'osv1_ancestry_nodes.tsv'
$edgesPath = Join-Path $out 'osv1_ancestry_edges.tsv'
Write-Utf8 $dagPath (($dag | ConvertTo-Json -Depth 100) + "`n")
$nodeRows = foreach ($node in $nodes) {
    [pscustomobject][ordered]@{
        node_id = $node.node_id
        branch = $node.branch
        stage = $node.stage
        authority_role = $node.authority_role
        evidence_count = @($node.evidence).Count
        evidence_paths = (@($node.evidence | ForEach-Object { $_.path }) -join '|')
        evidence_sha256 = (@($node.evidence | ForEach-Object { $_.sha256 }) -join '|')
    }
}
$edgeRows = $edges | Select-Object edge_id,from_node,to_node,relation
Write-Utf8 $nodesPath (($nodeRows | ConvertTo-Csv -Delimiter "`t" -NoTypeInformation) -join "`n")
Write-Utf8 $nodesPath ((Get-Content -Raw $nodesPath).TrimEnd() + "`n")
Write-Utf8 $edgesPath ((($edgeRows | ConvertTo-Csv -Delimiter "`t" -NoTypeInformation) -join "`n").TrimEnd() + "`n")

$childNames = @(
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
    'osv1_ancestry_edges.tsv'
)
$children = foreach ($name in $childNames) {
    $path = Join-Path $out $name
    if (!(Test-Path -LiteralPath $path)) { throw "missing OSV1 child artifact: $name" }
    [pscustomobject][ordered]@{ name=$name; sha256=Sha256 $path; bytes=(Get-Item -LiteralPath $path).Length }
}
$protocolPath = Join-Path $repo 'research\market-objects\contracts\gate16_6_osv1_closure.json'
if (!(Test-Path -LiteralPath $protocolPath)) { throw 'missing OSV1 closure protocol' }
$rootCore = [pscustomobject][ordered]@{
    contract = 'NORTHSTAR_GATE16_6_OSV1_ROOT_MANIFEST_V1'
    status = 'SEALED'
    observer_system = 'OSV1'
    protocol = [pscustomobject][ordered]@{
        path = Relative-Path $protocolPath $repo
        sha256 = Sha256 $protocolPath
        contract = 'NORTHSTAR_GATE16_6_OSV1_CLOSURE_V1'
    }
    ancestry = [pscustomobject][ordered]@{
        contract = $dag.contract
        logical_sha256 = $dag.logical_ancestry_sha256
        branch_count = 2
        rg2_leaf = 'RG2_GATE13R'
        rg3_leaf = 'RG3_GATE16_5'
    }
    child_artifacts = @($children)
    preserved_limits = @(
        'C1_INDEPENDENT_RUST_PRODUCER_PARITY_OPEN',
        'AUCTION_INTERVAL_CONDITIONING_NOT_EVALUABLE',
        'CROSS_SCALE_TRANSPORT_NOT_EVALUABLE',
        'GATE15_CONFIRMATION_WINDOWS_FROZEN_UNOPENED',
        'NO_ECONOMIC_OR_TRADING_AUTHORITY'
    )
    closure_invariants = [pscustomobject][ordered]@{
        source_kind_attached_to_every_finding = $true
        human_summary_authority_count = 0
        two_branch_ancestry_acyclic = $true
        root_receipt_written_last = $true
    }
}
$rootManifest = [pscustomobject][ordered]@{}
foreach ($p in $rootCore.psobject.Properties) { $rootManifest | Add-Member -NotePropertyName $p.Name -NotePropertyValue $p.Value }
$rootManifest | Add-Member -NotePropertyName logical_osv1_sha256 -NotePropertyValue (Canonical-Hash $rootCore)
$rootManifestPath = Join-Path $out 'osv1_root_manifest.json'
Write-Utf8 $rootManifestPath (($rootManifest | ConvertTo-Json -Depth 100) + "`n")

$physicalChildSet = Canonical-Hash @($children | ForEach-Object { [pscustomobject][ordered]@{ name=$_.name; sha256=$_.sha256 } })
$receipt = [pscustomobject][ordered]@{
    contract = 'NORTHSTAR_GATE16_6_OSV1_ROOT_RECEIPT_V1'
    status = 'PASS'
    root_manifest = 'osv1_root_manifest.json'
    root_manifest_sha256 = Sha256 $rootManifestPath
    logical_osv1_sha256 = $rootManifest.logical_osv1_sha256
    logical_ancestry_sha256 = $dag.logical_ancestry_sha256
    physical_child_set_sha256 = $physicalChildSet
    child_artifact_count = @($children).Count
    rg2_leaf = 'RG2_GATE13R'
    rg3_leaf = 'RG3_GATE16_5'
    receipt_written_last = $true
    confirmation_windows_opened = $false
}
$receiptPath = Join-Path $out 'osv1_root_receipt.json'
Write-Utf8 $receiptPath (($receipt | ConvertTo-Json -Depth 20) + "`n")

Write-Output "OSV1_ANCESTRY PASS nodes=$($nodes.Count) edges=$($edges.Count) logical=$($dag.logical_ancestry_sha256) root=$($rootManifest.logical_osv1_sha256)"
