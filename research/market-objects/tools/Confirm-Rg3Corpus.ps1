param(
    [Parameter(Mandatory=$true)][string]$CorpusRoot,
    [Parameter(Mandatory=$true)][string]$Manifest,
    [int]$ExpectedInstruments=6,
    [string]$Output
)

$ErrorActionPreference='Stop'
if([string]::IsNullOrWhiteSpace($Output)){$Output=Join-Path $CorpusRoot 'corpus-verification.json'}
$corpus=Join-Path $CorpusRoot 'corpus'
$report=Get-Content -Raw -LiteralPath (Join-Path $CorpusRoot 'qualification-report.json')|ConvertFrom-Json
$corpusManifest=Get-Content -Raw -LiteralPath (Join-Path $CorpusRoot 'corpus-manifest.json')|ConvertFrom-Json
$directories=@(Get-ChildItem -LiteralPath $corpus -Directory|Sort-Object Name)
if($directories.Count-ne$ExpectedInstruments){throw "Corpus count $($directories.Count) != $ExpectedInstruments"}

function Sha256Text([string]$text){
    $algorithm=[Security.Cryptography.SHA256]::Create()
    try{return -join($algorithm.ComputeHash([Text.Encoding]::UTF8.GetBytes($text))|ForEach-Object{$_.ToString('x2')})}finally{$algorithm.Dispose()}
}

$canonicalLines=@();$verifiedFiles=0;$runKeys=@()
foreach($directory in $directories){
    $seal=Get-Content -Raw -LiteralPath (Join-Path $directory.FullName 'run-seal.json')|ConvertFrom-Json
    if($seal.contract-ne'NORTHSTAR_RG3_IMMUTABLE_RUN_SEAL_V1' -or $seal.status-ne'SEALED'){throw "Invalid run seal: $($directory.Name)"}
    foreach($property in $seal.files.psobject.Properties){
        $path=Join-Path $directory.FullName $property.Name
        if(-not(Test-Path -LiteralPath $path)){throw "Sealed file missing: $path"}
        $actual=(Get-FileHash -LiteralPath $path -Algorithm SHA256).Hash.ToLowerInvariant()
        if($actual-ne[string]$property.Value){throw "Sealed file changed: $path"}
        $verifiedFiles++
    }
    $raw=Get-Content -Raw -LiteralPath (Join-Path $directory.FullName 'artifacts\raw-corpus-receipt.json')|ConvertFrom-Json
    $derived=Get-Content -Raw -LiteralPath (Join-Path $directory.FullName 'artifacts\derived-recipe-receipt.json')|ConvertFrom-Json
    if(-not$raw.balanced -or $raw.run_key-ne$seal.run_key -or $raw.canonical_sha256-ne$seal.raw_corpus_sha256){throw "Receipt mismatch: $($directory.Name)"}
    $canonicalLines+="$($seal.run_key)`t$($raw.canonical_sha256)`t$($derived.recipe_sha256)"
    $runKeys+=$seal.run_key
}
$canonicalHash=Sha256Text (($canonicalLines-join"`n")+"`n")
if($canonicalHash-ne$report.canonical_corpus_sha256 -or $canonicalHash-ne$corpusManifest.canonical_corpus_sha256){throw 'Canonical corpus hash mismatch.'}
$sourceHash=(Get-FileHash -LiteralPath $Manifest -Algorithm SHA256).Hash.ToLowerInvariant()
if($sourceHash-ne$corpusManifest.source_manifest_sha256){throw 'Campaign manifest hash mismatch.'}
if((@($runKeys|Sort-Object)-join'|')-ne(@($corpusManifest.run_keys|Sort-Object)-join'|')){throw 'Admitted run-key census mismatch.'}

$certificate=[pscustomobject][ordered]@{
    contract='NORTHSTAR_RG3_CORPUS_VERIFICATION_V1'
    status='PASS'
    research_generation=3
    instruments=$directories.Count
    verified_sealed_files=$verifiedFiles
    source_manifest_sha256=$sourceHash
    canonical_corpus_sha256=$canonicalHash
    run_keys=@($runKeys|Sort-Object)
}
$certificate|ConvertTo-Json -Depth 5|Set-Content -LiteralPath $Output -Encoding UTF8
$certificate|ConvertTo-Json -Depth 5
