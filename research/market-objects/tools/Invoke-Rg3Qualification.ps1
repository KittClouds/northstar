param(
    [Parameter(Mandatory=$true)][string]$Manifest,
    [Parameter(Mandatory=$true)][string]$OutputRoot,
    [string]$Terminal='C:\Program Files\MetaTrader 5\terminal64.exe',
    [string]$CommonFiles="$env:APPDATA\MetaQuotes\Terminal\Common\Files",
    [string]$Sealer='D:\northstar-target-gate14\release\northstar-market-object-seal.exe',
    [string]$StagingRoot="$env:LOCALAPPDATA\Northstar\rg3-staging",
    [int]$TimeoutSeconds=1200,
    [ValidateSet('','US30','FRA40','DE40','JPN225','US500','USTEC')]
    [string]$OnlyInstrument='',
    [switch]$DryRun
)

$ErrorActionPreference='Stop'
$manifestRecord=Get-Content -Raw -LiteralPath $Manifest|ConvertFrom-Json
if($manifestRecord.contract -ne 'NORTHSTAR_RG3_SIX_MARKET_QUALIFICATION_V1' -or $manifestRecord.research_generation -ne 3){throw 'Wrong RG3 qualification manifest.'}
if(@($manifestRecord.runs).Count -ne 6){throw 'Qualification requires exactly six canonical instruments.'}
$expected=@('US30','FRA40','DE40','JPN225','US500','USTEC')
$actualSet=@($manifestRecord.runs.canonical_instrument|Sort-Object) -join ','
$expectedSet=@($expected|Sort-Object) -join ','
if($actualSet -ne $expectedSet){throw "Canonical instrument set mismatch: $actualSet"}
$selectedRuns=@($manifestRecord.runs|Where-Object{$OnlyInstrument-eq'' -or $_.canonical_instrument-eq$OnlyInstrument})
$expectedCount=$selectedRuns.Count
if($expectedCount-eq0){throw "No campaign row selected for $OnlyInstrument"}

$staging=Join-Path $StagingRoot 'active';$externalQuarantine=Join-Path $StagingRoot 'quarantine';$corpus=Join-Path $OutputRoot 'corpus';$quarantine=Join-Path $OutputRoot 'quarantine';$configs=Join-Path $OutputRoot 'configs'
foreach($path in @($OutputRoot,$staging,$externalQuarantine,$corpus,$quarantine,$configs)){[IO.Directory]::CreateDirectory($path)|Out-Null}
$lock=[IO.File]::Open((Join-Path $OutputRoot 'single-writer.lock'),[IO.FileMode]::OpenOrCreate,[IO.FileAccess]::ReadWrite,[IO.FileShare]::None)

function Unix([string]$value){[DateTimeOffset]::Parse($value).ToUnixTimeSeconds()}
function Read-TsvShared([string]$path){
    try{$stream=[IO.File]::Open($path,[IO.FileMode]::Open,[IO.FileAccess]::Read,[IO.FileShare]::ReadWrite)}catch{return @()}
    try{$reader=[IO.StreamReader]::new($stream);try{$text=$reader.ReadToEnd()}finally{$reader.Dispose()}}finally{$stream.Dispose()}
    if([string]::IsNullOrWhiteSpace($text)){return @()};@($text|ConvertFrom-Csv -Delimiter "`t")
}
function Config($row,[long]$start,[long]$end,[string]$fingerprint){
    $from=[DateTimeOffset]::FromUnixTimeSeconds($start).UtcDateTime.AddDays(-2).ToString('yyyy.MM.dd')
    $to=[DateTimeOffset]::FromUnixTimeSeconds($end).UtcDateTime.AddDays(2).ToString('yyyy.MM.dd')
@"
[Tester]
Indicator=MarketObjectResearch\MarketObjectCollector.ex5
Symbol=$($row.broker_symbol)
Period=M5
Model=0
FromDate=$from
ToDate=$to
Visual=0
ShutdownTerminal=1
[TesterInputs]
InpCanonicalInstrument=$($row.canonical_instrument)
InpDataSourceId=BROKER_MT5
InpDataFingerprint=$fingerprint
InpWindowStart=$start
InpWindowEnd=$end
InpEnableRawLogging=true
InpFlushEvery=32
InpRangeLen=20
InpRangeMult=1.0
InpAtrLen=500
InpConfirmBars=1
InpMaxActiveBars=240
InpFrontierBufferAtr=0.0
InpUseFrontierBodyFilter=false
InpFrontierBodyAtr=0.20
InpObserveReentry=true
InpReentryWindow=5
InpReclaimNeedsMidline=false
InpRearmResetBars=1
InpRequireFreshLineageWindow=true
InpMaxObservationBars=500
"@
}
function Quarantine([string]$stage,[string]$reason){
    [pscustomobject]@{contract='NORTHSTAR_RG3_QUARANTINE_V1';reason=$reason;deletion_performed=$false}|ConvertTo-Json|Set-Content (Join-Path $stage 'failure.json')
    $name="$(Get-Date -Format yyyyMMddTHHmmssfffZ)__$(Split-Path $stage -Leaf)";$target=Join-Path $externalQuarantine $name;Move-Item $stage $target
    [pscustomobject]@{contract='NORTHSTAR_RG3_QUARANTINE_POINTER_V1';reason=$reason;preserved_path=$target}|ConvertTo-Json|Set-Content (Join-Path $quarantine "$name.json")
    return $target
}
function Sha256Text([string]$text){
    $algorithm=[Security.Cryptography.SHA256]::Create()
    try{return -join($algorithm.ComputeHash([Text.Encoding]::UTF8.GetBytes($text))|ForEach-Object{$_.ToString('x2')})}finally{$algorithm.Dispose()}
}

try{
    if(-not $DryRun -and @(Get-Process terminal64,metatester64 -ErrorAction SilentlyContinue).Count){throw 'A user terminal/tester is running. RG3 collection refuses to attach to or close it.'}
    if(-not $DryRun -and -not(Test-Path -LiteralPath $Sealer)){throw "Missing Rust sealer: $Sealer"}
    $admitted=0
    foreach($row in $selectedRuns){
        $start=Unix $row.window_start;$end=Unix $row.window_end
        $tag="$($row.canonical_instrument)_$start`_$end";$stage=Join-Path $staging $tag
        if(Test-Path $stage){throw "Existing stage: $stage"};[IO.Directory]::CreateDirectory($stage)|Out-Null
        $raw=Join-Path $stage 'raw';$artifacts=Join-Path $stage 'artifacts';[IO.Directory]::CreateDirectory($raw)|Out-Null
        $config=Join-Path $configs "$tag.ini";$fingerprint="RG3Q1_$($row.canonical_instrument)_$start`_$end"
        [IO.File]::WriteAllText($config,(Config $row $start $end $fingerprint),[Text.Encoding]::ASCII)
        if($DryRun){Write-Host "RG3_DRY_RUN $tag";Remove-Item -Recurse $stage;continue}
        $launched=$null;$before=@(Get-Process metatester64 -ErrorAction SilentlyContinue|ForEach-Object Id);$launchedAt=Get-Date
        try{
            $launched=Start-Process -FilePath $Terminal -ArgumentList "/config:`"$config`"" -WindowStyle Hidden -PassThru
            $deadline=(Get-Date).AddSeconds($TimeoutSeconds);$runFile=$null;$endRows=@()
            do{
                Start-Sleep -Milliseconds 500
                $candidate=Get-ChildItem $CommonFiles -Filter "MarketObjects_$($row.canonical_instrument)_M5_RG3_*_measurement_runs.tsv" -ErrorAction SilentlyContinue|Where-Object LastWriteTime -ge $launchedAt|Sort-Object LastWriteTime -Descending|Select-Object -First 1
                if($candidate){$endRows=@(Read-TsvShared $candidate.FullName|Where-Object record_type -eq 'END');if($endRows.Count){$runFile=$candidate}}
            }while(-not$runFile -and (Get-Date)-lt$deadline)
            if(-not$runFile){throw "Timeout without RG3 END: $tag"}
            if($launched -and -not$launched.HasExited){[void]$launched.WaitForExit(15000)}
            $stem=$runFile.BaseName -replace '_measurement_runs$','';$datasets=@('measurement_runs','compression_objects','compression_samples','compression_events','expansion_objects','expansion_samples','expansion_events','object_relations','structural_samples')
            foreach($dataset in $datasets){$source=Join-Path $CommonFiles "${stem}_${dataset}.tsv";if(-not(Test-Path $source)){throw "Missing $dataset"};Copy-Item $source $raw}
            & $Sealer $raw $stem $artifacts;if($LASTEXITCODE -ne 0){throw "Rust sealer failed: $LASTEXITCODE"}
            $receipt=Get-Content -Raw (Join-Path $artifacts 'raw-corpus-receipt.json')|ConvertFrom-Json
            foreach($dataset in $datasets){Move-Item (Join-Path $raw "${stem}_${dataset}.tsv") (Join-Path $raw "${dataset}.tsv")}
            $runKey=[string]$receipt.run_key;$target=Join-Path $corpus $row.canonical_instrument;if(Test-Path $target){throw "Duplicate instrument corpus slot: $($row.canonical_instrument)"}
            $fileHashes=[ordered]@{};Get-ChildItem $stage -Recurse -File|ForEach-Object{$fileHashes[$_.FullName.Substring($stage.Length+1)]=(Get-FileHash $_.FullName -Algorithm SHA256).Hash.ToLowerInvariant()}
            [pscustomobject][ordered]@{contract='NORTHSTAR_RG3_IMMUTABLE_RUN_SEAL_V1';status='SEALED';run_key=$runKey;raw_corpus_sha256=$receipt.canonical_sha256;files=$fileHashes}|ConvertTo-Json -Depth 6|Set-Content (Join-Path $stage 'run-seal.json') -Encoding UTF8
            Move-Item $stage $target;$admitted++;Write-Host "RG3_SEALED $($row.canonical_instrument) $runKey"
        }catch{
            if($launched -and -not$launched.HasExited){Stop-Process -Id $launched.Id;[void]$launched.WaitForExit(5000)}
            @(Get-Process metatester64 -ErrorAction SilentlyContinue|Where-Object Id -notin $before)|Stop-Process -ErrorAction SilentlyContinue
            $q=Quarantine $stage $_.Exception.Message;throw "RG3 stopped; quarantine=$q; reason=$($_.Exception.Message)"
        }
    }
    if(-not$DryRun -and $admitted -ne $expectedCount){throw "Only $admitted/$expectedCount runs admitted"}
    if(-not$DryRun){
        $rows=@();$canonicalLines=@()
        foreach($directory in @(Get-ChildItem $corpus -Directory|Sort-Object Name)){
            $seal=Get-Content -Raw (Join-Path $directory.FullName 'run-seal.json')|ConvertFrom-Json
            $rawReceipt=Get-Content -Raw (Join-Path $directory.FullName 'artifacts\raw-corpus-receipt.json')|ConvertFrom-Json
            $packReceipt=Get-Content -Raw (Join-Path $directory.FullName 'artifacts\packed-corpus-receipt.json')|ConvertFrom-Json
            $derivedReceipt=Get-Content -Raw (Join-Path $directory.FullName 'artifacts\derived-recipe-receipt.json')|ConvertFrom-Json
            $runsFile=Get-Item (Join-Path $directory.FullName 'raw\measurement_runs.tsv')
            $end=@(Get-Content $runsFile.FullName|ConvertFrom-Csv -Delimiter "`t"|Where-Object record_type -eq 'END')[0]
            $rows+=[pscustomobject][ordered]@{canonical_instrument=$end.canonical_instrument;broker_symbol=$end.broker_symbol;run_key=$seal.run_key;balanced=($end.balanced-eq'1');compression_objects=[int]$rawReceipt.rows.compression_objects;compression_samples=[int]$rawReceipt.rows.compression_samples;expansion_objects=[int]$rawReceipt.rows.expansion_objects;expansion_samples=[int]$rawReceipt.rows.expansion_samples;relations=[int]$rawReceipt.rows.object_relations;structural_samples=[int]$rawReceipt.rows.structural_samples;raw_corpus_sha256=$rawReceipt.canonical_sha256;packed_sha256=$packReceipt.packed_sha256;derived_recipe_sha256=$derivedReceipt.recipe_sha256;status='PASS'}
            # Corpus identity binds semantic observations and the derived
            # recipe, while lossless pack byte hashes remain run receipts.
            $canonicalLines+="$($seal.run_key)`t$($rawReceipt.canonical_sha256)`t$($derivedReceipt.recipe_sha256)"
        }
        if($rows.Count-ne$expectedCount){throw "Corpus census is $($rows.Count), expected $expectedCount"}
        $corpusHash=Sha256Text (($canonicalLines -join "`n")+"`n")
        [pscustomobject][ordered]@{contract='NORTHSTAR_RG3_QUALIFICATION_REPORT_V1';status='PASS';scope=$(if($OnlyInstrument-eq''){'SIX_MARKET'}else{'REPLAY_SAMPLE'});research_generation=3;instruments=$rows;totals=[pscustomobject]@{compressions=($rows|Measure-Object compression_objects -Sum).Sum;compression_samples=($rows|Measure-Object compression_samples -Sum).Sum;expansions=($rows|Measure-Object expansion_objects -Sum).Sum;expansion_samples=($rows|Measure-Object expansion_samples -Sum).Sum;relations=($rows|Measure-Object relations -Sum).Sum;structural_samples=($rows|Measure-Object structural_samples -Sum).Sum};canonical_corpus_sha256=$corpusHash}|ConvertTo-Json -Depth 7|Set-Content (Join-Path $OutputRoot 'qualification-report.json') -Encoding UTF8
        [pscustomobject][ordered]@{contract='NORTHSTAR_RG3_IMMUTABLE_CORPUS_MANIFEST_V1';status='SEALED';research_generation=3;source_manifest_sha256=(Get-FileHash $Manifest -Algorithm SHA256).Hash.ToLowerInvariant();run_keys=@($rows.run_key|Sort-Object);canonical_corpus_sha256=$corpusHash}|ConvertTo-Json -Depth 5|Set-Content (Join-Path $OutputRoot 'corpus-manifest.json') -Encoding UTF8
    }
    "status=$(if($DryRun){'DRY_RUN_PASS'}else{'PASS'})";"admitted=$admitted"
}finally{$lock.Dispose()}
