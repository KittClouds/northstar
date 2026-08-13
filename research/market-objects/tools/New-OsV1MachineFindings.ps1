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

function Resolve-Repo([string]$Relative) {
    $path = Join-Path $RepositoryRoot $Relative
    if (!(Test-Path -LiteralPath $path -PathType Leaf)) { throw "missing machine artifact: $Relative" }
    [IO.Path]::GetFullPath($path)
}

function Machine-Root([string]$Id, [string]$Relative, [string]$ExpectedContract, [string]$ExpectedStatus) {
    $path = Resolve-Repo $Relative
    if ([IO.Path]::GetExtension($path) -ne '.json') { throw "non-machine root rejected: $Relative" }
    $json = Get-Content -Raw -LiteralPath $path | ConvertFrom-Json
    if ($json.contract -ne $ExpectedContract -or $json.status -ne $ExpectedStatus) {
        throw "machine root contract/status mismatch: $Id"
    }
    [ordered]@{
        evidence_id = $Id
        artifact_path = ($Relative -replace '\\','/')
        artifact_sha256 = Sha256 $path
        seal_path = ($Relative -replace '\\','/')
        seal_sha256 = Sha256 $path
        seal_binding = 'ROOT_MACHINE_CERTIFICATE_OR_MANIFEST'
        contract = $json.contract
        status = $json.status
        json = $json
    }
}

function Machine-Child(
    [string]$Id,
    [string]$Relative,
    $Root,
    [string]$BindingKind,
    [string]$BindingKey,
    [string]$ExpectedContract,
    [string]$ExpectedStatus
) {
    $path = Resolve-Repo $Relative
    if ([IO.Path]::GetExtension($path) -ne '.json') { throw "non-machine child rejected: $Relative" }
    $actualHash = Sha256 $path
    if ($BindingKind -eq 'FILES_MAP') {
        $property = $Root.json.files.PSObject.Properties[$BindingKey]
        if ($null -eq $property -or $property.Value -ne $actualHash) {
            throw "child hash is not sealed by files map: $Id"
        }
    }
    elseif ($BindingKind -eq 'DIRECT_HASH') {
        $property = $Root.json.PSObject.Properties[$BindingKey]
        if ($null -eq $property -or $property.Value -ne $actualHash) {
            throw "child hash is not sealed by direct hash: $Id"
        }
    }
    else { throw "unknown seal binding: $BindingKind" }
    $json = Get-Content -Raw -LiteralPath $path | ConvertFrom-Json
    if ($json.contract -ne $ExpectedContract -or $json.status -ne $ExpectedStatus) {
        throw "machine child contract/status mismatch: $Id"
    }
    [ordered]@{
        evidence_id = $Id
        artifact_path = ($Relative -replace '\\','/')
        artifact_sha256 = $actualHash
        seal_path = $Root.artifact_path
        seal_sha256 = $Root.artifact_sha256
        seal_binding = "$BindingKind::$BindingKey"
        contract = $json.contract
        status = $json.status
        json = $json
    }
}

$phase13 = Machine-Root 'G13R_CERTIFICATE' 'research\auction-parity\artifacts\phase13r-evaluation\certificate_manifest.json' 'NORTHSTAR_PHASE13R_CERTIFICATE_V1' 'COMPLETE'
$gate15 = Machine-Root 'G15_CORPUS' 'research\market-objects\artifacts\rg3-gate15-seal-final-v4\gate15-corpus-manifest.json' 'NORTHSTAR_RG3_GATE15_SEALED_CORPUS_MANIFEST_V1' 'SEALED'
$gate15Receipt = Machine-Root 'G15_5_RECEIPT' 'research\market-objects\artifacts\rg3-gate15-5-atlas-final-v5\gate15_5_receipt.json' 'NORTHSTAR_RG3_GATE15_5_ATLAS_RECEIPT_V1' 'PASS'
$gate15Exit = Machine-Child 'G15_5_EXIT' 'research\market-objects\artifacts\rg3-gate15-5-atlas-final-v5\gate15_5_exit_report.json' $gate15Receipt 'FILES_MAP' 'gate15_5_exit_report.json' 'NORTHSTAR_RG3_GATE15_5_TRI_INSTRUMENT_ATLAS_V1' 'PASS'
$gate16Receipt = Machine-Root 'G16_SEAL' 'research\market-objects\artifacts\rg3-gate16-seal-final-v1\gate16-seal-receipt.json' 'NORTHSTAR_RG3_GATE16_SEAL_RECEIPT_V1' 'PASS'
$gate16Findings = Machine-Child 'G16_FINDINGS' 'research\market-objects\artifacts\rg3-gate16-seal-final-v1\gate16-findings.json' $gate16Receipt 'FILES_MAP' 'seal/gate16-findings.json' 'NORTHSTAR_RG3_GATE16_FINDINGS_V1' 'PASS_WITH_EXPLICIT_LIMITS'
$gate16_5Receipt = Machine-Root 'G16_5_SEAL' 'research\market-objects\artifacts\rg3-gate16-5-seal-final-v1\gate16_5-seal-receipt.json' 'NORTHSTAR_RG3_GATE16_5_SEAL_RECEIPT_V1' 'PASS_WITH_OBSERVATIONAL_FINDINGS'
$gate16_5Findings = Machine-Child 'G16_5_FINDINGS' 'research\market-objects\artifacts\rg3-gate16-5-seal-final-v1\gate16_5-findings.json' $gate16_5Receipt 'DIRECT_HASH' 'findings_sha256' 'NORTHSTAR_RG3_GATE16_5_FINDINGS_V1' 'OBSERVATIONAL_FINDINGS_ONLY'

foreach ($root in @($gate15Receipt,$gate16Receipt,$gate16_5Receipt)) {
    if ($root.json.confirmation_windows_opened) { throw "confirmation windows opened in $($root.evidence_id)" }
}

$evidence = @($phase13,$gate15,$gate15Receipt,$gate15Exit,$gate16Receipt,$gate16Findings,$gate16_5Receipt,$gate16_5Findings | ForEach-Object {
    [ordered]@{
        evidence_id = $_.evidence_id
        artifact_path = $_.artifact_path
        artifact_sha256 = $_.artifact_sha256
        seal_path = $_.seal_path
        seal_sha256 = $_.seal_sha256
        seal_binding = $_.seal_binding
        contract = $_.contract
        observed_status = $_.status
    }
})

$findings = [Collections.Generic.List[object]]::new()
$insufficiencies = [Collections.Generic.List[object]]::new()
function Add-Finding(
    [string]$Id,
    [string]$Statement,
    [ValidateSet('MACHINE_DERIVED','ARTIFACT_DECLARED','HUMAN_SUMMARY')]
    [string]$SourceKind,
    [string]$EpistemicStatus,
    [string]$Gate,
    $Population,
    $Measurements,
    [string]$EvidenceId,
    [string[]]$Pointers,
    $ArtifactLimits
) {
    $findings.Add([ordered]@{
        finding_id = $Id
        statement = $Statement
        epistemic_status = $EpistemicStatus
        source_kind = $SourceKind
        gate = $Gate
        population = $Population
        measurements = $Measurements
        evidence_id = $EvidenceId
        json_pointers = @($Pointers)
        artifact_limits = $ArtifactLimits
    })
}
function Add-Insufficiency(
    [string]$Id,
    [string]$Statement,
    [ValidateSet('MACHINE_DERIVED','ARTIFACT_DECLARED','HUMAN_SUMMARY')]
    [string]$SourceKind,
    [string]$Status,
    [string]$Gate,
    [string]$EvidenceId,
    [string]$Pointer
) {
    $insufficiencies.Add([ordered]@{
        insufficiency_id = $Id
        statement = $Statement
        epistemic_status = $Status
        source_kind = $SourceKind
        gate = $Gate
        evidence_id = $EvidenceId
        json_pointer = $Pointer
    })
}

# Gate 13R certificate facts.
foreach ($property in @($phase13.json.candidate_outcomes.PSObject.Properties | Sort-Object Name)) {
    Add-Finding "G13R_$($property.Name.ToUpperInvariant())" "candidate::$($property.Name)::$($property.Value)" 'ARTIFACT_DECLARED' 'CONFIRMED' 'GATE13R' ([ordered]@{ sealed_runs = $phase13.json.sealed_runs; quarantined_runs = $phase13.json.quarantined_runs }) ([ordered]@{ candidate = $property.Name; outcome = $property.Value; behavioral_only = $phase13.json.behavioral_only; trade_policy = $phase13.json.trade_policy }) 'G13R_CERTIFICATE' @("/candidate_outcomes/$($property.Name)",'/sealed_runs','/quarantined_runs','/behavioral_only','/trade_policy') ([ordered]@{ behavioral_only = $phase13.json.behavioral_only; trade_policy = $phase13.json.trade_policy })
}

# Gate 15 sealed corpus and the six frozen exploratory systems.
Add-Finding 'G15_RG3_CORPUS_SEALED' "$($gate15.json.contract)::$($gate15.json.status)" 'ARTIFACT_DECLARED' 'OBSERVED' 'GATE15' ([ordered]@{ runs = $gate15.json.admitted_run_count; objects = [int]$gate15.json.totals.compression_objects + [int]$gate15.json.totals.expansion_objects; instruments = @($gate15.json.objects_by_instrument.PSObject.Properties).Count }) ([ordered]@{ canonical_corpus_sha256 = $gate15.json.canonical_corpus_sha256; totals = $gate15.json.totals; objects_by_instrument = $gate15.json.objects_by_instrument }) 'G15_CORPUS' @('/admitted_run_count','/canonical_corpus_sha256','/totals','/objects_by_instrument') ([ordered]@{ discovery_v2_status = $gate15.json.discovery_v2.status })
foreach ($system in @($gate15.json.family_systems | Sort-Object object_kind,representation)) {
    $id = "G15_$($system.object_kind)_$($system.representation)".ToUpperInvariant()
    Add-Finding $id "family_system::$($system.object_kind)::$($system.representation)" 'MACHINE_DERIVED' 'SUPPORTED_EXPLORATORY' 'GATE15' ([ordered]@{ supported_objects = $system.supported_objects; noise_objects = $system.noise_objects }) ([ordered]@{ family_system_sha256 = $system.sha256; common_families = $system.common_families; mean_seed_ari = $system.mean_seed_ari; noise_fraction = $system.noise_fraction }) 'G15_CORPUS' @('/family_systems') ([ordered]@{ discovery_v2_status = $gate15.json.discovery_v2.status })
}

# Gate 15.5 sealed atlas facts.
Add-Finding 'G15_5_RELATIONAL_ATLAS' $gate15Exit.json.epistemic_status 'ARTIFACT_DECLARED' 'SUPPORTED_EXPLORATORY' 'GATE15_5' ([ordered]@{ runs = $gate15Exit.json.source_run_count; objects = $gate15Exit.json.source_object_count; lineages = $gate15Exit.json.lineage_count }) ([ordered]@{ structural_bridge_receipts = $gate15Exit.json.structural_bridge_receipts; exact_receipts = $gate15Exit.json.exact_receipts; asof_receipts = $gate15Exit.json.asof_receipts; null_structural_receipts = $gate15Exit.json.null_structural_receipts; graph_nodes = $gate15Exit.json.graph_nodes; graph_edges = $gate15Exit.json.graph_edges }) 'G15_5_EXIT' @('/epistemic_status','/source_run_count','/source_object_count','/structural_bridge_receipts','/exact_receipts','/asof_receipts','/null_structural_receipts','/graph_nodes','/graph_edges') $gate15Exit.json.limitations

# Gate 16 findings are already machine-generated structured finding objects.
for ($i = 0; $i -lt $gate16Findings.json.findings.Count; $i++) {
    $source = $gate16Findings.json.findings[$i]
    $measurements = [ordered]@{}
    foreach ($property in $source.PSObject.Properties) {
        if ($property.Name -notin @('finding','claim_limit')) { $measurements[$property.Name] = $property.Value }
    }
    $limit = if ($null -ne $source.PSObject.Properties['claim_limit']) { $source.claim_limit } else { $gate16Findings.json.forbidden_conclusions }
    Add-Finding "G16_$($source.finding.ToUpperInvariant())" $source.finding 'MACHINE_DERIVED' 'OBSERVED' 'GATE16' ([ordered]@{ objects = $gate16Findings.json.object_population; completed = $gate16Findings.json.completed_objects; censored = $gate16Findings.json.censored_objects; instruments = $gate16Findings.json.instruments }) $measurements 'G16_FINDINGS' @("/findings/$i",'/object_population','/completed_objects','/censored_objects','/instruments') $limit
}

# Gate 16.5 uses the machine-emitted observation strings verbatim.
for ($i = 0; $i -lt $gate16_5Findings.json.observations.Count; $i++) {
    Add-Finding ("G16_5_OBSERVATION_{0:D2}" -f ($i + 1)) ([string]$gate16_5Findings.json.observations[$i]) 'MACHINE_DERIVED' 'OBSERVED' 'GATE16_5' ([ordered]@{ census = $gate16_5Findings.json.census }) ([ordered]@{ census = $gate16_5Findings.json.census }) 'G16_5_FINDINGS' @("/observations/$i",'/census') $gate16_5Findings.json.forbidden_inferences
}

# Insufficiencies are copied from sealed machine fields, never prose reports.
Add-Insufficiency 'G13R_NO_TRADE_POLICY' "trade_policy::$($phase13.json.trade_policy)" 'ARTIFACT_DECLARED' 'NOT_EVALUABLE' 'GATE13R' 'G13R_CERTIFICATE' '/trade_policy'
Add-Insufficiency 'G15_5_AUCTION_INTERVAL_AUTHORITY' ([string]$gate15Exit.json.auction_interval_authority) 'ARTIFACT_DECLARED' 'NOT_EVALUABLE' 'GATE15_5' 'G15_5_EXIT' '/auction_interval_authority'
for ($i = 0; $i -lt $gate15Exit.json.limitations.Count; $i++) {
    Add-Insufficiency ("G15_5_LIMIT_{0:D2}" -f ($i + 1)) ([string]$gate15Exit.json.limitations[$i]) 'ARTIFACT_DECLARED' 'INSUFFICIENT_SUPPORT' 'GATE15_5' 'G15_5_EXIT' "/limitations/$i"
}
for ($i = 0; $i -lt $gate16Findings.json.forbidden_conclusions.Count; $i++) {
    Add-Insufficiency ("G16_FORBIDDEN_{0:D2}" -f ($i + 1)) ([string]$gate16Findings.json.forbidden_conclusions[$i]) 'ARTIFACT_DECLARED' 'NOT_EVALUABLE' 'GATE16' 'G16_FINDINGS' "/forbidden_conclusions/$i"
}
for ($i = 0; $i -lt $gate16_5Findings.json.forbidden_inferences.Count; $i++) {
    Add-Insufficiency ("G16_5_FORBIDDEN_{0:D2}" -f ($i + 1)) ([string]$gate16_5Findings.json.forbidden_inferences[$i]) 'ARTIFACT_DECLARED' 'NOT_EVALUABLE' 'GATE16_5' 'G16_5_FINDINGS' "/forbidden_inferences/$i"
}

$orderedFindings = @($findings | Sort-Object finding_id)
$orderedInsufficiencies = @($insufficiencies | Sort-Object insufficiency_id)
$evidenceIds = [Collections.Generic.HashSet[string]]::new([StringComparer]::Ordinal)
foreach ($item in $evidence) { [void]$evidenceIds.Add($item.evidence_id) }
foreach ($item in @($orderedFindings) + @($orderedInsufficiencies)) {
    if ($item.source_kind -notin @('MACHINE_DERIVED','ARTIFACT_DECLARED','HUMAN_SUMMARY') -or
        $item.source_kind -eq 'HUMAN_SUMMARY' -or !$evidenceIds.Contains($item.evidence_id)) {
        throw "non-machine or unsealed finding admitted: $($item.finding_id)$($item.insufficiency_id)"
    }
}

$core = [ordered]@{
    contract = 'NORTHSTAR_GATE16_6_OSV1_MACHINE_FINDINGS_V1'
    status = 'PASS_MACHINE_ARTIFACTS_ONLY'
    authority_rule = 'every finding and insufficiency is read from a JSON artifact bound by a machine certificate, manifest, or seal receipt'
    evidence_count = $evidence.Count
    finding_count = $orderedFindings.Count
    insufficiency_count = $orderedInsufficiencies.Count
    evidence = $evidence
    findings = $orderedFindings
    insufficiencies = $orderedInsufficiencies
    invariants = [ordered]@{
        human_summary_sources = 0
        markdown_sources = 0
        unsealed_machine_sources = 0
        confirmation_windows_opened = $false
        economic_authority = $false
        trading_authority = $false
    }
}
$core.logical_findings_sha256 = Canonical-Hash $core

$output = [IO.Path]::GetFullPath($OutputDirectory)
New-Item -ItemType Directory -Force -Path $output | Out-Null
$jsonPath = Join-Path $output 'osv1_machine_findings.json'
$tsvPath = Join-Path $output 'osv1_machine_findings.tsv'
$limitsPath = Join-Path $output 'osv1_machine_insufficiencies.tsv'
$receiptPath = Join-Path $output 'osv1_machine_findings_receipt.json'
[IO.File]::WriteAllText($jsonPath, ($core | ConvertTo-Json -Depth 100) + "`n", [Text.UTF8Encoding]::new($false))

$writer = [IO.StreamWriter]::new($tsvPath, $false, [Text.UTF8Encoding]::new($false), 262144)
try {
    $writer.WriteLine("finding_id`tstatement`tsource_kind`tepistemic_status`tgate`tpopulation_json`tmeasurements_json`tevidence_id`tjson_pointers`tartifact_limits_json")
    foreach ($item in $orderedFindings) {
        $values = @($item.finding_id,$item.statement,$item.source_kind,$item.epistemic_status,$item.gate,($item.population|ConvertTo-Json -Depth 30 -Compress),($item.measurements|ConvertTo-Json -Depth 30 -Compress),$item.evidence_id,($item.json_pointers -join ','),($item.artifact_limits|ConvertTo-Json -Depth 30 -Compress))
        if (($values -join '') -match "[`t`r`n]") { throw "TSV control character in finding $($item.finding_id)" }
        $writer.WriteLine($values -join "`t")
    }
}
finally { $writer.Dispose() }

$writer = [IO.StreamWriter]::new($limitsPath, $false, [Text.UTF8Encoding]::new($false), 262144)
try {
    $writer.WriteLine("insufficiency_id`tstatement`tsource_kind`tepistemic_status`tgate`tevidence_id`tjson_pointer")
    foreach ($item in $orderedInsufficiencies) {
        $values = @($item.insufficiency_id,$item.statement,$item.source_kind,$item.epistemic_status,$item.gate,$item.evidence_id,$item.json_pointer)
        if (($values -join '') -match "[`t`r`n]") { throw "TSV control character in insufficiency $($item.insufficiency_id)" }
        $writer.WriteLine($values -join "`t")
    }
}
finally { $writer.Dispose() }

$receipt = [ordered]@{
    contract = 'NORTHSTAR_GATE16_6_OSV1_MACHINE_FINDINGS_RECEIPT_V1'
    status = 'PASS'
    logical_findings_sha256 = $core.logical_findings_sha256
    findings_json_sha256 = Sha256 $jsonPath
    findings_tsv_sha256 = Sha256 $tsvPath
    insufficiencies_tsv_sha256 = Sha256 $limitsPath
    evidence_count = $evidence.Count
    finding_count = $orderedFindings.Count
    insufficiency_count = $orderedInsufficiencies.Count
    confirmation_windows_opened = $false
}
[IO.File]::WriteAllText($receiptPath, ($receipt | ConvertTo-Json -Depth 20) + "`n", [Text.UTF8Encoding]::new($false))
Write-Output "OSV1_MACHINE_FINDINGS findings=$($orderedFindings.Count) insufficiencies=$($orderedInsufficiencies.Count) logical_sha256=$($core.logical_findings_sha256)"
