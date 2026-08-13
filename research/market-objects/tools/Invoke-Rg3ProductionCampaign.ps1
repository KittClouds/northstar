param(
    [Parameter(Mandatory=$true)][string]$Manifest,
    [Parameter(Mandatory=$true)][string]$OutputRoot,
    [string]$Qualifier='',
    [string]$Terminal='C:\Program Files\MetaTrader 5\terminal64.exe',
    [string]$StagingRoot='C:\rg3s',
    [int]$TimeoutSeconds=1800,
    [switch]$DryRun
)

$ErrorActionPreference='Stop'
if([string]::IsNullOrWhiteSpace($Qualifier)){$Qualifier=Join-Path $PSScriptRoot 'Invoke-Rg3Qualification.ps1'}
$campaign=Get-Content -Raw -LiteralPath $Manifest|ConvertFrom-Json
if($campaign.contract-ne'NORTHSTAR_RG3_GATE15_PRODUCTION_CAMPAIGN_V1' -or $campaign.research_generation-ne3){throw 'Wrong Gate 15 campaign manifest.'}
$symbols=@('US30','FRA40','DE40','JPN225','US500','USTEC')
$windows=@($campaign.windows)
if($windows.Count-lt1){throw 'Campaign contains no windows.'}
$ids=@($windows.window_id|Sort-Object -Unique)
if($ids.Count-ne$windows.Count){throw 'Duplicate campaign window_id.'}

$batches=Join-Path $OutputRoot 'batches'
$generated=Join-Path $OutputRoot 'generated-manifests'
[IO.Directory]::CreateDirectory($batches)|Out-Null
[IO.Directory]::CreateDirectory($generated)|Out-Null
$results=@()

foreach($window in $windows){
    $batchRoot=Join-Path $batches $window.window_id
    $batchManifest=Join-Path $generated "$($window.window_id).json"
    $runs=@()
    foreach($canonical in $symbols){
        $binding=[string]$campaign.canonical_symbols.$canonical
        if([string]::IsNullOrWhiteSpace($binding)){throw "Missing symbol binding: $canonical"}
        $runs+=[ordered]@{canonical_instrument=$canonical;broker_symbol=$binding;window_start=$window.window_start;window_end=$window.window_end}
    }
    [ordered]@{
        contract='NORTHSTAR_RG3_SIX_MARKET_QUALIFICATION_V1'
        research_generation=3
        source_auction_generation=2
        preset=$campaign.preset
        window_policy=$campaign.window_policy
        gate15_window_id=$window.window_id
        selection_reason=$window.selection_reason
        runs=$runs
    }|ConvertTo-Json -Depth 6|Set-Content -LiteralPath $batchManifest -Encoding UTF8

    $existing=Join-Path $batchRoot 'qualification-report.json'
    if(Test-Path -LiteralPath $existing){
        $report=Get-Content -Raw -LiteralPath $existing|ConvertFrom-Json
        if($report.status-ne'PASS'){throw "Existing batch is not PASS: $($window.window_id)"}
        Write-Host "RG3_PRODUCTION_REUSE $($window.window_id)"
    }else{
        $arguments=@('-NoProfile','-ExecutionPolicy','Bypass','-File',$Qualifier,'-Manifest',$batchManifest,'-OutputRoot',$batchRoot,'-Terminal',$Terminal,'-StagingRoot',$StagingRoot,'-TimeoutSeconds',$TimeoutSeconds)
        if($DryRun){$arguments+='-DryRun'}
        & powershell @arguments
        if($LASTEXITCODE-ne0){throw "Production batch failed: $($window.window_id)"}
    }
    if(-not$DryRun){
        $report=Get-Content -Raw -LiteralPath $existing|ConvertFrom-Json
        $results+=[pscustomobject][ordered]@{
            window_id=$window.window_id
            window_start=$window.window_start
            window_end=$window.window_end
            selection_reason=$window.selection_reason
            status=$report.status
            canonical_corpus_sha256=$report.canonical_corpus_sha256
            compressions=[int]$report.totals.compressions
            expansions=[int]$report.totals.expansions
            compression_samples=[int]$report.totals.compression_samples
            expansion_samples=[int]$report.totals.expansion_samples
        }
    }
}

if(-not$DryRun){
    $manifestHash=(Get-FileHash -LiteralPath $Manifest -Algorithm SHA256).Hash.ToLowerInvariant()
    $totals=[ordered]@{
        windows=$results.Count
        runs=$results.Count*6
        compressions=($results|Measure-Object compressions -Sum).Sum
        expansions=($results|Measure-Object expansions -Sum).Sum
        compression_samples=($results|Measure-Object compression_samples -Sum).Sum
        expansion_samples=($results|Measure-Object expansion_samples -Sum).Sum
    }
    [ordered]@{
        contract='NORTHSTAR_RG3_GATE15_COLLECTION_REPORT_V1'
        status='PASS'
        research_generation=3
        campaign_manifest_sha256=$manifestHash
        batches=$results
        totals=$totals
    }|ConvertTo-Json -Depth 7|Set-Content -LiteralPath (Join-Path $OutputRoot 'production-collection-report.json') -Encoding UTF8
}
"status=$(if($DryRun){'DRY_RUN_PASS'}else{'PASS'})"
"windows=$($windows.Count)"
