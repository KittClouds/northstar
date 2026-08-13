param(
    [Parameter(Mandatory=$true)][string]$QualificationRoot,
    [Parameter(Mandatory=$true)][string]$ProductionRoot,
    [Parameter(Mandatory=$true)][string]$DiscoveryV1,
    [Parameter(Mandatory=$true)][string]$DiscoveryV2,
    [Parameter(Mandatory=$true)][string]$DiscoveryV2Replay,
    [Parameter(Mandatory=$true)][string]$ContractsRoot,
    [Parameter(Mandatory=$true)][string]$ProductionManifest,
    [Parameter(Mandatory=$true)][string]$ConfirmationManifest,
    [Parameter(Mandatory=$true)][string]$PerformanceProof,
    [Parameter(Mandatory=$true)][string]$OutputRoot
)

$ErrorActionPreference='Stop'
[IO.Directory]::CreateDirectory($OutputRoot)|Out-Null

function Sha256Text([string]$Text){
    $algorithm=[Security.Cryptography.SHA256]::Create()
    try{return -join($algorithm.ComputeHash([Text.Encoding]::UTF8.GetBytes($Text))|ForEach-Object{$_.ToString('x2')})}
    finally{$algorithm.Dispose()}
}
function FileHash([string]$Path){
    if(-not(Test-Path -LiteralPath $Path -PathType Leaf)){throw "Missing file: $Path"}
    return (Get-FileHash -LiteralPath $Path -Algorithm SHA256).Hash.ToLowerInvariant()
}
function WriteJson([string]$Path,$Value){
    $Value|ConvertTo-Json -Depth 12|Set-Content -LiteralPath $Path -Encoding UTF8
}
function VerifyDiscovery([string]$Root,[int]$ExpectedRuns){
    $reportPath=Join-Path $Root 'gate15-discovery-report.json'
    $assignmentPath=Join-Path $Root 'candidate-assignments.tsv'
    $receiptPath=Join-Path $Root 'discovery-receipt.json'
    $receipt=Get-Content -Raw -LiteralPath $receiptPath|ConvertFrom-Json
    if($receipt.source_run_count-ne$ExpectedRuns){throw "Discovery run count mismatch: $Root"}
    $reportHash=FileHash $reportPath;$assignmentHash=FileHash $assignmentPath
    if($reportHash-ne$receipt.report_sha256 -or $assignmentHash-ne$receipt.assignments_sha256){throw "Discovery receipt mismatch: $Root"}
    return [pscustomobject][ordered]@{
        status=$receipt.status
        source_run_count=[int]$receipt.source_run_count
        recipe_sha256=[string]$receipt.recipe_sha256
        report_sha256=$reportHash
        assignments_sha256=$assignmentHash
        receipt_sha256=FileHash $receiptPath
    }
}

$runDirectories=@()
$runDirectories+=@(Get-ChildItem -LiteralPath (Join-Path $QualificationRoot 'corpus') -Directory)
$runDirectories+=@(Get-ChildItem -LiteralPath (Join-Path $ProductionRoot 'batches') -Recurse -Filter 'run-seal.json' -File|ForEach-Object{$_.Directory})
$runDirectories=@($runDirectories|Sort-Object FullName)
if($runDirectories.Count-ne42){throw "Expected 42 admitted runs, found $($runDirectories.Count)"}

$runRows=@();$canonicalLines=@();$verifiedFiles=0;$keys=@{};$totals=[ordered]@{
    compression_objects=0;expansion_objects=0;compression_samples=0;expansion_samples=0
    object_relations=0;structural_samples=0
}
$instrumentCounts=[ordered]@{DE40=0;FRA40=0;JPN225=0;US30=0;US500=0;USTEC=0}
foreach($directory in $runDirectories){
    $sealPath=Join-Path $directory.FullName 'run-seal.json'
    $seal=Get-Content -Raw -LiteralPath $sealPath|ConvertFrom-Json
    if($seal.contract-ne'NORTHSTAR_RG3_IMMUTABLE_RUN_SEAL_V1' -or $seal.status-ne'SEALED'){throw "Invalid run seal: $sealPath"}
    if($keys.ContainsKey([string]$seal.run_key)){throw "Duplicate run key: $($seal.run_key)"};$keys[[string]$seal.run_key]=$true
    foreach($property in $seal.files.psobject.Properties){
        $sealedPath=Join-Path $directory.FullName $property.Name
        if((FileHash $sealedPath)-ne[string]$property.Value){throw "Sealed file changed: $sealedPath"}
        $verifiedFiles++
    }
    $rawPath=Join-Path $directory.FullName 'artifacts\raw-corpus-receipt.json'
    $derivedPath=Join-Path $directory.FullName 'artifacts\derived-recipe-receipt.json'
    $raw=Get-Content -Raw -LiteralPath $rawPath|ConvertFrom-Json
    $derived=Get-Content -Raw -LiteralPath $derivedPath|ConvertFrom-Json
    if(-not$raw.balanced -or $raw.run_key-ne$seal.run_key -or $raw.canonical_sha256-ne$seal.raw_corpus_sha256){throw "Raw receipt mismatch: $rawPath"}
    $end=(Import-Csv -LiteralPath (Join-Path $directory.FullName 'raw\measurement_runs.tsv') -Delimiter "`t"|Where-Object record_type -eq 'END')
    if(@($end).Count-ne1 -or $end.research_generation-ne'3'){throw "Invalid measurement END row: $($directory.FullName)"}
    $instrument=[string]$end.canonical_instrument
    if(-not$instrumentCounts.Contains($instrument)){throw "Unexpected instrument: $instrument"}
    $instrumentCounts[$instrument]++
    foreach($name in @('compression_objects','expansion_objects','compression_samples','expansion_samples','object_relations','structural_samples')){$totals[$name]+=[int64]$raw.rows.$name}
    $canonicalLines+="$($seal.run_key)`t$($raw.canonical_sha256)`t$($derived.recipe_sha256)"
    $runRows+=[pscustomobject][ordered]@{
        run_key=[string]$seal.run_key;instrument=$instrument;window_start=[int64]$end.window_start;window_end=[int64]$end.window_end
        raw_corpus_sha256=[string]$raw.canonical_sha256;derived_recipe_sha256=[string]$derived.recipe_sha256
        run_seal_sha256=FileHash $sealPath
    }
}
foreach($entry in $instrumentCounts.GetEnumerator()){if($entry.Value-ne7){throw "Expected 7 runs for $($entry.Key), found $($entry.Value)"}}
$canonicalLines=@($canonicalLines|Sort-Object)
$canonicalCorpusHash=Sha256Text (($canonicalLines-join"`n")+"`n")

$v1=VerifyDiscovery $DiscoveryV1 42
$v2=VerifyDiscovery $DiscoveryV2 42
$v2Replay=VerifyDiscovery $DiscoveryV2Replay 42
if($v1.status-ne'COLLECT_MORE'){throw 'V1 failure evidence was not preserved.'}
if($v2.report_sha256-ne$v2Replay.report_sha256 -or $v2.assignments_sha256-ne$v2Replay.assignments_sha256 -or $v2.receipt_sha256-ne$v2Replay.receipt_sha256){throw 'V2 derived replay is not byte deterministic.'}
$report=Get-Content -Raw -LiteralPath (Join-Path $DiscoveryV2 'gate15-discovery-report.json')|ConvertFrom-Json

$commonSupport=$true;$stable=$true;$maxInstrumentShare=0.0;$maxWindowShare=0.0;$familySystems=@()
foreach($kind in $report.kinds){
    foreach($view in $kind.views){
        $families=@($view.families|Where-Object common_support)
        if($families.Count-lt2){$commonSupport=$false}
        if([double]$view.mean_seed_ari-lt0.65){$stable=$false}
        foreach($family in $families){
            $maxInstrumentShare=[math]::Max($maxInstrumentShare,[double]$family.largest_instrument_fraction)
            $maxWindowShare=[math]::Max($maxWindowShare,[double]$family.largest_window_fraction)
        }
        $familySystems+=[pscustomobject][ordered]@{object_kind=$kind.object_kind;representation=$view.representation;sha256=$view.family_system_sha256;supported_objects=$view.supported_objects;noise_objects=$view.noise_objects;noise_fraction=$view.noise_fraction;common_families=$families.Count;mean_seed_ari=$view.mean_seed_ari}
    }
}

$confirmation=Get-Content -Raw -LiteralPath $ConfirmationManifest|ConvertFrom-Json
if($confirmation.status-ne'FROZEN_UNOPENED' -or $confirmation.authorization-ne'NOT_AUTHORIZED_IN_GATE15'){throw 'Confirmation windows are not fail-closed.'}
$frozenSystems=@($confirmation.family_systems|ForEach-Object{"$($_.object_kind)|$($_.representation)|$($_.sha256)"}|Sort-Object)
$observedSystems=@($familySystems|ForEach-Object{"$($_.object_kind)|$($_.representation)|$($_.sha256)"}|Sort-Object)
if(($frozenSystems-join"`n")-ne($observedSystems-join"`n")){throw 'Frozen family-system identities do not match canonical V2 output.'}
foreach($window in $confirmation.windows){
    $start=[DateTimeOffset]::Parse($window.window_start).ToUnixTimeSeconds();$end=[DateTimeOffset]::Parse($window.window_end).ToUnixTimeSeconds()
    if($runRows|Where-Object{$_.window_start-lt$end -and $_.window_end-gt$start}){throw "Confirmation window was consumed: $($window.window_id)"}
}

$contractFiles=@('rg3_identity.json','raw_authority.json','schema_dictionary.json','gate15_discovery_protocol.json','gate15_discovery_adjudication_v2.json')
$contractHashes=[ordered]@{}
foreach($name in $contractFiles){$contractHashes[$name]=FileHash (Join-Path $ContractsRoot $name)}
$quarantineFiles=@()
foreach($root in @($QualificationRoot,$ProductionRoot)){
    $rootLabel=if($root-eq$QualificationRoot){'qualification'}else{'production'}
    $rootPrefix=$root.TrimEnd('\','/')+'\'
    $quarantineFiles+=@(Get-ChildItem -LiteralPath $root -Recurse -File|Where-Object{$_.FullName -match '[\\/]quarantine[\\/]'}|ForEach-Object{[pscustomobject][ordered]@{path="$rootLabel/$($_.FullName.Substring($rootPrefix.Length).Replace('\','/'))";sha256=FileHash $_.FullName}})
}

$manifest=[pscustomobject][ordered]@{
    contract='NORTHSTAR_RG3_GATE15_SEALED_CORPUS_MANIFEST_V1';status='SEALED';research_generation=3
    admitted_run_count=$runRows.Count;admitted_runs=@($runRows|Sort-Object run_key);quarantined_artifacts=$quarantineFiles
    verified_sealed_files=$verifiedFiles;canonical_corpus_sha256=$canonicalCorpusHash;totals=$totals;objects_by_instrument=$report.objects_by_instrument
    contract_sha256=$contractHashes;production_manifest_sha256=FileHash $ProductionManifest;confirmation_manifest_sha256=FileHash $ConfirmationManifest
    performance_proof_sha256=FileHash $PerformanceProof
    discovery_v1=$v1;discovery_v2=$v2;derived_replay=$v2Replay;family_systems=$familySystems
}
$manifestPath=Join-Path $OutputRoot 'gate15-corpus-manifest.json';WriteJson $manifestPath $manifest

$exitChecks=[ordered]@{
    enough_objects=([int]$report.total_objects-ge600)
    all_six_instruments=(@($report.objects_by_instrument.psobject.Properties).Count-eq6 -and [int]$report.minimum_instrument_objects-ge70)
    common_families_supported=$commonSupport
    within_view_reproducible=$stable
    support_not_instrument_concentrated=($maxInstrumentShare-lt0.35)
    support_not_window_concentrated=($maxWindowShare-lt0.35)
    censoring_and_missingness_explicit=(@($report.kinds).Count-eq2 -and @($report.missingness.psobject.Properties).Count-eq2)
    null_is_not_a_family=(@($familySystems|Where-Object{$_.common_families-lt2}).Count-eq0)
    cross_view_disagreement_retained=(@($report.kinds|Where-Object{[double]$_.mean_cross_view_ari-lt0.35}).Count-gt0)
    v1_failure_preserved=($v1.status-eq'COLLECT_MORE')
    v2_exploratory_only=($report.epistemic_status-eq'CANDIDATE_ONLY_NOT_MARKET_TRUTH')
    derived_replay_byte_identical=$true
    confirmation_windows_frozen_unopened=$true
    raw_corpus_and_schema_sealed=$true
}
$passed=@($exitChecks.Values|Where-Object{$_-ne$true}).Count-eq0
$compression=$report.kinds|Where-Object object_kind -eq 'COMPRESSION';$expansion=$report.kinds|Where-Object object_kind -eq 'EXPANSION'
$exitReport=[pscustomobject][ordered]@{
    contract='NORTHSTAR_RG3_GATE15_EXIT_REPORT_V1';status=$(if($passed){'PASS'}else{'FAIL'});research_generation=3
    corpus=[pscustomobject][ordered]@{runs=$report.source_run_count;objects=$report.total_objects;instruments=$report.objects_by_instrument;minimum_per_instrument=$report.minimum_instrument_objects;compression_objects=$compression.total;expansion_objects=$expansion.total}
    censoring=[pscustomobject][ordered]@{compression_count=$compression.censored;compression_rate=([double]$compression.censored/[double]$compression.total);expansion_count=$expansion.censored;expansion_rate=([double]$expansion.censored/[double]$expansion.total);terminal_reason_counts=[ordered]@{compression=$compression.terminal_reason_counts;expansion=$expansion.terminal_reason_counts}}
    missingness=[pscustomobject][ordered]@{counts=$report.missingness;interpretation='destination_compression_id is unavailable before a handoff and remains null for censored expansions; event rows retain availability-time nulls'}
    family_systems=$familySystems;maximum_family_instrument_share=$maxInstrumentShare;maximum_family_window_share=$maxWindowShare
    representation_sensitivity=@($report.kinds|ForEach-Object{[pscustomobject][ordered]@{object_kind=$_.object_kind;mean_raw_cross_view_ari=$_.mean_cross_view_ari;pairwise=$_.cross_view_agreement}})
    epistemic_boundary='representation-specific exploratory candidate systems; NULL is not a family; no consensus taxonomy or market truth is claimed'
    confirmation='12 exact instrument-window runs frozen and unopened; no Gate 15 confirmation claim'
    exit_checks=$exitChecks;canonical_corpus_sha256=$canonicalCorpusHash;corpus_manifest_sha256=FileHash $manifestPath
}
$exitPath=Join-Path $OutputRoot 'gate15-exit-report.json';WriteJson $exitPath $exitReport
$receipt=[pscustomobject][ordered]@{contract='NORTHSTAR_RG3_GATE15_SEAL_RECEIPT_V1';status=$exitReport.status;research_generation=3;corpus_manifest_sha256=FileHash $manifestPath;exit_report_sha256=FileHash $exitPath;canonical_corpus_sha256=$canonicalCorpusHash}
WriteJson (Join-Path $OutputRoot 'gate15-seal-receipt.json') $receipt
$receipt|ConvertTo-Json -Depth 6
if(-not$passed){exit 2}
