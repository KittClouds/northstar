param(
    [Parameter(Mandatory=$true)][string]$Protocol,
    [Parameter(Mandatory=$true)][string]$OutputRoot,
    [string]$Terminal = 'C:\Program Files\MetaTrader 5\terminal64.exe',
    [string]$CommonFiles = "$env:APPDATA\MetaQuotes\Terminal\Common\Files",
    [int]$TimeoutSeconds = 900,
    [switch]$DryRun
)

$ErrorActionPreference='Stop'
$protocolRecord=Get-Content -Raw -LiteralPath $Protocol|ConvertFrom-Json
if($protocolRecord.status -ne 'PREAUTHORIZED_NOT_AUTHORIZED' -or $protocolRecord.holdout_authorized){throw 'Protocol is not frozen preauthorization.'}
$runsRoot=Join-Path $OutputRoot 'runs';$stagingRoot=Join-Path $OutputRoot 'staging';$quarantineRoot=Join-Path $OutputRoot 'quarantine';$configsRoot=Join-Path $OutputRoot 'configs'
foreach($directory in @($OutputRoot,$runsRoot,$stagingRoot,$quarantineRoot,$configsRoot)){[IO.Directory]::CreateDirectory($directory)|Out-Null}
$lockPath=Join-Path $OutputRoot 'single_writer.lock';$lockStream=$null

function Read-TsvShared([string]$Path){
    if(-not(Test-Path -LiteralPath $Path)){return @()}
    try{$stream=[IO.File]::Open($Path,[IO.FileMode]::Open,[IO.FileAccess]::Read,[IO.FileShare]::ReadWrite)} catch [IO.IOException] {return @()}
    try{$reader=[IO.StreamReader]::new($stream);try{$text=$reader.ReadToEnd()}finally{$reader.Dispose()}}finally{$stream.Dispose()}
    if([string]::IsNullOrWhiteSpace($text)){return @()};@($text|ConvertFrom-Csv -Delimiter "`t")
}

function New-Config($row,[string]$tag){
@"
[Tester]
Indicator=BuiltMasterStructure\MasterStructureController.ex5
Symbol=$($row.broker_symbol)
Period=M5
Model=0
FromDate=$([DateTimeOffset]::FromUnixTimeSeconds([long]$row.window_start).UtcDateTime.ToString('yyyy.MM.dd'))
ToDate=$([DateTimeOffset]::FromUnixTimeSeconds([long]$row.window_end_exclusive).UtcDateTime.ToString('yyyy.MM.dd'))
Visual=0
ShutdownTerminal=1
[TesterInputs]
InpInstanceTag=$tag
InpMasterTimeframe=5
InpATRPeriod=100
InpTimerMilliseconds=750
InpTesterCacheUnchangedStructure=true
InpVolKittTimeframe=16385
InpProfileTimeframe=16385
InpDaySwingsTimeframe=5
InpWayneTimeframe=16408
InpVolKittLookback=200
InpVolKittClusters=5
InpVolKittIterations=50
InpEnableProfile=true
InpProfilesToKeep=3
InpEnableSinglePrints=false
InpDaysToKeep=5
InpWaynePeriodsToKeep=5
InpWayneStandardPivots=true
InpWayneMidPivots=true
InpWayneZones=true
InpEpsilonATR=0.12
InpMinimumSamples=2
InpUseIntervalDistance=true
InpRequireRoleCompatibility=true
InpMaximumLevels=1024
InpMaximumNodes=256
InpCenterRoleToleranceATR=0.15
InpNodeMatchDistanceATR=0.20
InpRetireAfterRebuilds=3
InpEnableAuctionEngine=true
InpApproachDistanceATR=0.50
InpRejectionDistanceATR=0.20
InpBreakBufferATR=0.00
InpAcceptanceCloses=2
InpAcceptanceDistanceATR=0.00
InpReclaimToleranceATR=0.05
InpDepartureDistanceATR=0.25
InpMaxAttemptBars=24
InpEpisodeGapBars=12
InpEnableLogging=true
InpLogFlushSnapshots=10
InpTesterFinalizeAt=$($row.window_end_exclusive)
InpCanonicalInstrument=$($row.canonical_instrument)
InpDataSourceId=$($row.data_source_id)
InpDataFingerprint=PHASE13_$($row.canonical_instrument)_$($row.holdout_id)_RG2
InpResearchWindowStart=$($row.window_start)
InpResearchWindowEnd=$($row.window_end_exclusive)
InpShowNodes=false
InpShowLabels=false
InpShowProvenance=false
InpMaximumRenderedNodes=64
InpHistoryBars=150
InpFutureBars=30
InpLabelGapBars=6
InpEnablePerformanceTelemetry=true
InpPerformanceWindowUpdates=100
InpSlowFrameThresholdUs=250000
"@
}

function Quarantine([string]$stage,[string]$reason){
    if(-not(Test-Path -LiteralPath $stage)){[IO.Directory]::CreateDirectory($stage)|Out-Null}
    [pscustomobject][ordered]@{contract='NORTHSTAR_PHASE13_COLLECTION_FAILURE_V1';status='QUARANTINED';reason=$reason;deletion_performed=$false}|ConvertTo-Json|Set-Content -LiteralPath (Join-Path $stage 'failure.json') -Encoding UTF8
    $target=Join-Path $quarantineRoot "$(Get-Date -Format 'yyyyMMddTHHmmssfffZ')__$(Split-Path $stage -Leaf)";Move-Item -LiteralPath $stage -Destination $target;$target
}

$lockStream=[IO.File]::Open($lockPath,[IO.FileMode]::OpenOrCreate,[IO.FileAccess]::ReadWrite,[IO.FileShare]::None)
try{
    if(-not $DryRun -and @(Get-Process terminal64,metatester64 -ErrorAction SilentlyContinue).Count -gt 0){throw 'A user terminal/tester is running. Collection refuses to attach to or close it.'}
    $reservations=@($protocolRecord.reservations|Sort-Object canonical_instrument,holdout_id)
    if($reservations.Count -ne 12){throw "Expected 12 reservations, found $($reservations.Count)"}
    $sealed=0
    foreach($row in $reservations){
        $tag="P13_$($row.canonical_instrument)_$($row.holdout_id)"
        $auctionStem="MasterAuction_$($row.broker_symbol)_PERIOD_M5_${tag}_RG2_v7"
        $structureStem="MasterStructure_$($row.broker_symbol)_PERIOD_M5_${tag}_v6"
        $stage=Join-Path $stagingRoot $tag;$config=Join-Path $configsRoot "$tag.ini"
        if(Test-Path -LiteralPath $stage){throw "Existing stage detected: $stage"};[IO.Directory]::CreateDirectory($stage)|Out-Null
        [IO.File]::WriteAllText($config,(New-Config $row $tag),[Text.Encoding]::ASCII)
        if($DryRun){Write-Host "P13_DRY_RUN $tag";Remove-Item -LiteralPath $stage;continue}
        $process=$null;$before=@(Get-Process metatester64 -ErrorAction SilentlyContinue|ForEach-Object Id)
        try{
            $process=Start-Process -FilePath $Terminal -ArgumentList "/config:`"$config`"" -WindowStyle Hidden -PassThru
            $runPath=Join-Path $CommonFiles "${auctionStem}_runs.tsv";$deadline=(Get-Date).AddSeconds($TimeoutSeconds);$end=@()
            do{Start-Sleep -Milliseconds 500;$end=@(Read-TsvShared $runPath|Where-Object record_type -eq 'END'|Select-Object -Last 1)}while($end.Count -eq 0 -and (Get-Date)-lt $deadline)
            if($end.Count -eq 0){throw "Timeout without END receipt: $tag"}
            if($null-ne$process -and -not $process.HasExited){[void]$process.WaitForExit(15000)}
            $sealArgs=@{Protocol=$Protocol;Instrument=$row.canonical_instrument;HoldoutId=$row.holdout_id;AuctionStem=$auctionStem;StructureStem=$structureStem;StagingDirectory=$stage;RunsDirectory=$runsRoot;CommonFiles=$CommonFiles}
            $result=& "$PSScriptRoot\Seal-Phase13HoldoutRun.ps1" @sealArgs 2>&1
            if('status=SEALED' -notin @($result)){throw "Sealer failed: $($result -join '; ')"}
            $sealed++;Write-Host "P13_SEALED $tag"
        }catch{
            if($null-ne$process -and -not $process.HasExited){Stop-Process -Id $process.Id;[void]$process.WaitForExit(5000)}
            $owned=@(Get-Process metatester64 -ErrorAction SilentlyContinue|Where-Object Id -notin $before)
            if($owned.Count){$owned|Stop-Process}
            $q=Quarantine $stage $_.Exception.Message;throw "Phase 13 collection stopped; quarantined=$q; reason=$($_.Exception.Message)"
        }
    }
    if(-not $DryRun -and $sealed -ne 12){throw "Only $sealed/12 runs sealed"}
    Write-Output "status=$(if($DryRun){'DRY_RUN_PASS'}else{'PASS'})";Write-Output "sealed=$sealed"
}finally{if($null-ne$lockStream){$lockStream.Dispose()}}
