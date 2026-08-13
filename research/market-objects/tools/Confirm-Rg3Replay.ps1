param(
    [Parameter(Mandatory=$true)][string]$BaselineRoot,
    [Parameter(Mandatory=$true)][string]$ReplayRoot,
    [ValidateSet('US30','FRA40','DE40','JPN225','US500','USTEC')]
    [string]$Instrument='US30',
    [string]$Output
)

$ErrorActionPreference='Stop'
if([string]::IsNullOrWhiteSpace($Output)){$Output=Join-Path $BaselineRoot "replay-certificate-$Instrument.json"}
$baseline=Join-Path $BaselineRoot "corpus\$Instrument"
$replay=Join-Path $ReplayRoot "corpus\$Instrument"
foreach($path in @($baseline,$replay)){if(-not(Test-Path -LiteralPath $path)){throw "Missing sealed corpus: $path"}}

function Json([string]$root,[string]$name){Get-Content -Raw -LiteralPath (Join-Path $root "artifacts\$name")|ConvertFrom-Json}
function EndRow([string]$root){
    @(Get-Content -LiteralPath (Join-Path $root 'raw\measurement_runs.tsv')|ConvertFrom-Csv -Delimiter "`t"|Where-Object record_type -eq 'END')[0]
}

$baselineRaw=Json $baseline 'raw-corpus-receipt.json'
$replayRaw=Json $replay 'raw-corpus-receipt.json'
$baselinePack=Json $baseline 'packed-corpus-receipt.json'
$replayPack=Json $replay 'packed-corpus-receipt.json'
$baselineEnd=EndRow $baseline
$replayEnd=EndRow $replay

if($baselineRaw.run_key-ne$replayRaw.run_key){throw 'Replay run_key changed.'}
if($baselineEnd.invocation_id-eq$replayEnd.invocation_id){throw 'Replay invocation_id was not unique.'}
if($baselineRaw.canonical_sha256-ne$replayRaw.canonical_sha256){throw 'Canonical semantic hash mismatch.'}
if($baselineRaw.byte_sha256-eq$replayRaw.byte_sha256){throw 'Byte hash did not preserve invocation uniqueness.'}
$rowDatasets=@('measurement_runs','compression_objects','compression_samples','compression_events','expansion_objects','expansion_samples','expansion_events','object_relations','structural_samples')
foreach($dataset in $rowDatasets){
    if([int64]$baselineRaw.rows.$dataset-ne[int64]$replayRaw.rows.$dataset){throw "Replay row count changed: $dataset"}
}

$datasets=@('compression_objects','compression_samples','compression_events','expansion_objects','expansion_samples','expansion_events','object_relations','structural_samples')
$exact=[ordered]@{}
foreach($dataset in $datasets){
    $left=(Get-FileHash (Join-Path $baseline "raw\$dataset.tsv") -Algorithm SHA256).Hash.ToLowerInvariant()
    $right=(Get-FileHash (Join-Path $replay "raw\$dataset.tsv") -Algorithm SHA256).Hash.ToLowerInvariant()
    if($left-ne$right){throw "Replay dataset differs: $dataset"}
    $exact[$dataset]=$left
}

$certificate=[pscustomobject][ordered]@{
    contract='NORTHSTAR_RG3_REPLAY_DETERMINISM_CERTIFICATE_V1'
    status='PASS'
    research_generation=3
    canonical_instrument=$Instrument
    run_key=$baselineRaw.run_key
    baseline_invocation_id=$baselineEnd.invocation_id
    replay_invocation_id=$replayEnd.invocation_id
    canonical_semantic_sha256=$baselineRaw.canonical_sha256
    baseline_byte_sha256=$baselineRaw.byte_sha256
    replay_byte_sha256=$replayRaw.byte_sha256
    unique_invocation_bytes=($baselineRaw.byte_sha256-ne$replayRaw.byte_sha256)
    canonical_semantics_equal=$true
    exact_non_invocation_datasets=$exact
    baseline_packed_sha256=$baselinePack.packed_sha256
    replay_packed_sha256=$replayPack.packed_sha256
    note='Packed artifacts are lossless and therefore retain unique invocation bytes; canonical semantic identity excludes invocation_id.'
}
$certificate|ConvertTo-Json -Depth 7|Set-Content -LiteralPath $Output -Encoding UTF8
$certificate|ConvertTo-Json -Depth 7
