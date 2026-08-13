param(
    [Parameter(Mandatory=$true)][string]$Protocol,
    [Parameter(Mandatory=$true)][string]$Instrument,
    [Parameter(Mandatory=$true)][string]$HoldoutId,
    [Parameter(Mandatory=$true)][string]$AuctionStem,
    [Parameter(Mandatory=$true)][string]$StructureStem,
    [Parameter(Mandatory=$true)][string]$StagingDirectory,
    [Parameter(Mandatory=$true)][string]$RunsDirectory,
    [string]$CommonFiles = "$env:APPDATA\MetaQuotes\Terminal\Common\Files",
    [string]$CertificateTool = "$env:APPDATA\MetaQuotes\Terminal\D0E8209F77C8CF37AD8BF550E51FF075\MQL5\Scripts\Research\Certify-MasterStructureReplay.ps1"
)

$ErrorActionPreference = 'Stop'
$auctionDatasets = @('events','attempts','episodes','context','features','transits','runs')
$structureDatasets = @('inputs','nodes','events')

function Read-Tsv([string]$Path) {
    if (-not (Test-Path -LiteralPath $Path)) { throw "Missing TSV: $Path" }
    $stream = [IO.File]::Open($Path,[IO.FileMode]::Open,[IO.FileAccess]::Read,[IO.FileShare]::ReadWrite)
    try {
        $reader = [IO.StreamReader]::new($stream)
        try { $text = $reader.ReadToEnd() } finally { $reader.Dispose() }
    } finally { $stream.Dispose() }
    if ([string]::IsNullOrWhiteSpace($text)) { return @() }
    return @($text | ConvertFrom-Csv -Delimiter "`t")
}

function File-Receipt([string]$Root,[IO.FileInfo]$File) {
    [pscustomobject][ordered]@{
        path=$File.FullName.Substring($Root.TrimEnd('\').Length+1).Replace('\','/')
        bytes=$File.Length
        sha256=(Get-FileHash -LiteralPath $File.FullName -Algorithm SHA256).Hash.ToLowerInvariant()
    }
}

function Scan-Identity([string]$Path,[string]$RunKey,[string]$Invocation) {
    $stream = [IO.File]::Open($Path,[IO.FileMode]::Open,[IO.FileAccess]::Read,[IO.FileShare]::Read)
    try {
        $reader = [IO.StreamReader]::new($stream,[Text.Encoding]::UTF8,$true,1048576)
        try {
            $header = $reader.ReadLine().Split("`t")
            $runIndex = [Array]::IndexOf($header,'run_key')
            $invIndex = [Array]::IndexOf($header,'invocation_id')
            if ($runIndex -lt 0 -or $invIndex -lt 0) { throw "Identity columns missing: $Path" }
            $rows=0; $foreign=0
            while (-not $reader.EndOfStream) {
                $line=$reader.ReadLine(); if ([string]::IsNullOrWhiteSpace($line)) { continue }
                $cells=$line.Split("`t"); $rows++
                if ($cells.Count -le [Math]::Max($runIndex,$invIndex) -or $cells[$runIndex] -ne $RunKey -or $cells[$invIndex] -ne $Invocation) { $foreign++ }
            }
            [pscustomobject]@{rows=$rows;foreign=$foreign}
        } finally { $reader.Dispose() }
    } finally { $stream.Dispose() }
}

$protocolRecord = Get-Content -Raw -LiteralPath $Protocol | ConvertFrom-Json
if ($protocolRecord.contract -ne 'NORTHSTAR_RG2_HOLDOUT_PREAUTHORIZATION_V1' -or
    $protocolRecord.status -ne 'PREAUTHORIZED_NOT_AUTHORIZED' -or $protocolRecord.holdout_authorized) {
    throw 'Phase 13 preauthorization identity is not frozen and unconsumed.'
}
$reservation = @($protocolRecord.reservations | Where-Object {
    $_.canonical_instrument -eq $Instrument -and $_.holdout_id -eq $HoldoutId
})
if ($reservation.Count -ne 1 -or $reservation[0].status -ne 'RESERVED_UNTOUCHED') {
    throw "Reservation is not uniquely preauthorized: $Instrument/$HoldoutId"
}
$reservation = $reservation[0]

$auctionRaw=Join-Path $StagingDirectory 'raw\auction'
$structureRaw=Join-Path $StagingDirectory 'raw\structure'
$receipts=Join-Path $StagingDirectory 'receipts'
foreach($directory in @($auctionRaw,$structureRaw,$receipts)){[IO.Directory]::CreateDirectory($directory)|Out-Null}
foreach($dataset in $auctionDatasets){
    $source=Join-Path $CommonFiles "${AuctionStem}_${dataset}.tsv"
    if(-not(Test-Path -LiteralPath $source)){throw "Missing auction output: $source"}
    [IO.File]::Copy($source,(Join-Path $auctionRaw "${AuctionStem}_${dataset}.tsv"),$false)
}
foreach($dataset in $structureDatasets){
    $source=Join-Path $CommonFiles "${StructureStem}_${dataset}.tsv"
    if(-not(Test-Path -LiteralPath $source)){throw "Missing structure output: $source"}
    [IO.File]::Copy($source,(Join-Path $structureRaw "${StructureStem}_${dataset}.tsv"),$false)
}

$certificateOutput=& $CertificateTool -Stem $AuctionStem -CommonFiles $auctionRaw -OutputDirectory $receipts -MinimumRuns 1 2>&1
$certificatePath=Join-Path $receipts "${AuctionStem}_replay_certificate.json"
if(-not(Test-Path -LiteralPath $certificatePath)){throw "Certificate missing: $($certificateOutput -join '; ')"}
$certificate=Get-Content -Raw -LiteralPath $certificatePath|ConvertFrom-Json
if($certificate.status -ne 'PASS' -or @($certificate.invocations).Count -ne 1 -or @($certificate.failures).Count -ne 0){throw 'Canonical certificate failed.'}

$runs=@(Read-Tsv (Join-Path $auctionRaw "${AuctionStem}_runs.tsv"))
$starts=@($runs|Where-Object record_type -eq 'START');$ends=@($runs|Where-Object record_type -eq 'END')
if($starts.Count -ne 1 -or $ends.Count -ne 1){throw "Expected one START/END, got $($starts.Count)/$($ends.Count)"}
$end=$ends[0];$runKey=[string]$end.run_key;$invocation=[string]$end.invocation_id
$required=[ordered]@{
    dataset_schema='7';contract_version='1';contract_id='MST_AUCTION_RELATIONAL_V1';research_generation='2'
    run_status='COMPLETE';run_complete='1';contract_valid='1';contract_violations='0';balanced='1';visual_mode='0';tester='1'
    controller_version='2';topology_version='2';auction_grammar_version='1';feature_schema_version='1';dataset_schema_version='7';producer_bundle_version='1'
    canonical_instrument=[string]$reservation.canonical_instrument;symbol=[string]$reservation.broker_symbol;data_source_id=[string]$reservation.data_source_id
    preset_id='RG2_M5_SIX_INDEX_V1';build_id='MASTER_STRUCTURE_RG2_REPLAY_WINDOW_GATE_V1';config_hash='4284550518493202411'
    window_start=[string]$reservation.window_start;window_end=[string]$reservation.window_end_exclusive
}
foreach($field in $required.Keys){if([string]$end.$field -ne [string]$required[$field]){throw "Terminal mismatch $field expected=$($required[$field]) actual=$($end.$field)"}}
if($certificate.invocations[0].run_key -ne $runKey -or $certificate.invocations[0].invocation_id -ne $invocation -or -not $certificate.invocations[0].counts_match_manifest){throw 'Certificate identity mismatch.'}

$structureCounts=[ordered]@{}
foreach($dataset in $structureDatasets){
    $scan=Scan-Identity (Join-Path $structureRaw "${StructureStem}_${dataset}.tsv") $runKey $invocation
    if($scan.foreign -gt 0){throw "Foreign identity in structure $dataset"}
    if($dataset -ne 'events' -and $scan.rows -eq 0){throw "Empty structure $dataset"}
    $structureCounts[$dataset]=$scan.rows
}

$terminalReceipt=Join-Path $receipts 'terminal_run_receipt.json'
$end|ConvertTo-Json -Depth 5|Set-Content -LiteralPath $terminalReceipt -Encoding UTF8
$admission=[pscustomobject][ordered]@{
    contract='MST_HOLDOUT_ADMISSION_V1';status='SEALED';protocol_sha256=$protocolRecord.protocol_sha256
    canonical_instrument=$reservation.canonical_instrument;broker_symbol=$reservation.broker_symbol;data_source_id=$reservation.data_source_id
    holdout_id=$reservation.holdout_id;window_start=[long]$reservation.window_start;window_end_exclusive=[long]$reservation.window_end_exclusive
    run_key=$runKey;invocation_id=$invocation;auction_stem=$AuctionStem;structure_stem=$StructureStem
    canonical_dataset_hash=$certificate.invocations[0].canonical_dataset_hash;dataset_hashes=$certificate.invocations[0].dataset_hashes
    row_counts=$certificate.invocations[0].row_counts;structure_row_counts=$structureCounts
    terminal_hash=$end.terminal_hash;receipt_hash=$end.receipt_hash;contract_receipt_hash=$end.contract_receipt_hash
    isolation='unique_instance_tag_plus_single_writer_lock';visual_rendering=$false;research_logging=$true;unexpected_data_gaps=0
}
$admission|ConvertTo-Json -Depth 8|Set-Content -LiteralPath (Join-Path $receipts 'admission_receipt.json') -Encoding UTF8

$files=@(Get-ChildItem -LiteralPath $StagingDirectory -Recurse -File|Sort-Object FullName)
$fileReceipts=@($files|ForEach-Object{File-Receipt $StagingDirectory $_})
$material=($fileReceipts|ForEach-Object{"$($_.path)`t$($_.bytes)`t$($_.sha256)"})-join "`n"
$algorithm=[Security.Cryptography.SHA256]::Create()
try{$payload=(($algorithm.ComputeHash([Text.Encoding]::UTF8.GetBytes($material))|ForEach-Object{$_.ToString('x2')})-join '')}finally{$algorithm.Dispose()}
$seal=[pscustomobject][ordered]@{
    contract='MST_IMMUTABLE_RUN_SEAL_V1';status='SEALED';run_key=$runKey;invocation_id=$invocation
    holdout_id=$HoldoutId;protocol_sha256=$protocolRecord.protocol_sha256;canonical_dataset_hash=$certificate.invocations[0].canonical_dataset_hash
    sealed_payload_sha256=$payload;file_count=$fileReceipts.Count;files=$fileReceipts
    mutation_rule='never overwrite; one-shot holdout evidence is immutable'
}
$seal|ConvertTo-Json -Depth 8|Set-Content -LiteralPath (Join-Path $StagingDirectory 'seal.json') -Encoding UTF8
[IO.Directory]::CreateDirectory($RunsDirectory)|Out-Null
$target=Join-Path $RunsDirectory $runKey
if(Test-Path -LiteralPath $target){throw "Holdout run already exists: $target"}
Get-ChildItem -LiteralPath $StagingDirectory -Recurse -File|ForEach-Object{[IO.File]::SetAttributes($_.FullName,($_.Attributes -bor [IO.FileAttributes]::ReadOnly))}
Move-Item -LiteralPath $StagingDirectory -Destination $target
Write-Output 'status=SEALED'
Write-Output "run_key=$runKey"
Write-Output "corpus_path=$target"
