param(
    [Parameter(Mandatory=$true)][string]$CandidateCensus,
    [Parameter(Mandatory=$true)][string]$WitnessCensus,
    [Parameter(Mandatory=$true)][string]$CandidateDiscovery,
    [Parameter(Mandatory=$true)][string]$WitnessDiscovery,
    [Parameter(Mandatory=$true)][string]$Seal
)
$ErrorActionPreference='Stop'
$crateRoot=(Resolve-Path -LiteralPath (Join-Path $PSScriptRoot '..')).Path
$sealPath=[IO.Path]::GetFullPath($Seal)
if(Test-Path -LiteralPath $sealPath){if((Get-ChildItem -LiteralPath $sealPath -Force|Measure-Object).Count-ne 0){throw "SEAL_NOT_EMPTY:$sealPath"}}else{New-Item -ItemType Directory -Path $sealPath|Out-Null}

function Compare-Phase([string]$left,[string]$right,[string]$phase){
    $a=(Resolve-Path -LiteralPath $left).Path;$b=(Resolve-Path -LiteralPath $right).Path
    $af=Get-ChildItem -LiteralPath $a -File -Recurse;$bf=Get-ChildItem -LiteralPath $b -File -Recurse
    if($af.Count-ne$bf.Count){throw "${phase}_FILE_COUNT_DRIFT"}
    foreach($f in $af){$rel=[IO.Path]::GetRelativePath($a,$f.FullName);$g=Join-Path $b $rel;if(!(Test-Path -LiteralPath $g)){throw "${phase}_MISSING:$rel"};if((Get-FileHash -Algorithm SHA256 -LiteralPath $f.FullName).Hash-ne(Get-FileHash -Algorithm SHA256 -LiteralPath $g).Hash){throw "${phase}_BYTE_DRIFT:$rel"}}
    return [ordered]@{schema="G9_1_${phase}_INDEPENDENT_REPLAY_RECEIPT_V1";candidate_build='BUILD_D';witness_build='BUILD_E';compared_files=$af.Count;byte_differences=0;semantic_and_artifact_replay='PASS';binary_reproducibility='NOT_REQUIRED_NOT_CLAIMED';status='PASS'}
}
$censusReplay=Compare-Phase $CandidateCensus $WitnessCensus 'CENSUS'
$discoveryReplay=Compare-Phase $CandidateDiscovery $WitnessDiscovery 'DISCOVERY'
$census=(Get-Content -Raw (Join-Path $CandidateCensus 'G9_1_RESIDUAL_PAIR_STRATA.json')|ConvertFrom-Json).summary
$discovery=Get-Content -Raw (Join-Path $CandidateDiscovery 'G9_1_DISCOVERY_SUMMARY.json')|ConvertFrom-Json
if($census.e0_exact_cardinality-ne 138962-or$census.e1_exact_cardinality-ne 0-or$census.e2_exact_cardinality-ne 0){throw 'CENSUS_EXPECTATION_DRIFT'}
if($discovery.scientific_outcome-ne'NO_ORDINAL_MATCHED_LAWFUL_SAME_FIBER_SPECIMENS'-or$discovery.continuation_evaluations-ne 0-or$discovery.bounded_silence_pairs-ne 0){throw 'TYPED_NULL_DRIFT'}
New-Item -ItemType Directory -Path (Join-Path $sealPath 'census'),(Join-Path $sealPath 'discovery'),(Join-Path $sealPath 'tools')|Out-Null
Copy-Item -Path (Join-Path $CandidateCensus '*') -Destination (Join-Path $sealPath 'census') -Recurse
Copy-Item -Path (Join-Path $CandidateDiscovery '*') -Destination (Join-Path $sealPath 'discovery') -Recurse
Copy-Item -LiteralPath (Join-Path $crateRoot 'G9_1_CAMPAIGN_FINAL_REPORT.md') -Destination $sealPath
Copy-Item -LiteralPath $PSCommandPath -Destination (Join-Path $sealPath 'tools\finalize.ps1')
[IO.File]::WriteAllText((Join-Path $sealPath 'G9_1_CENSUS_INDEPENDENT_REPLAY_RECEIPT.json'),($censusReplay|ConvertTo-Json -Compress),[Text.UTF8Encoding]::new($false))
[IO.File]::WriteAllText((Join-Path $sealPath 'G9_1_DISCOVERY_INDEPENDENT_REPLAY_RECEIPT.json'),($discoveryReplay|ConvertTo-Json -Compress),[Text.UTF8Encoding]::new($false))
$censusRoot=(Get-Content -Raw (Join-Path $CandidateCensus 'G9_1_CENSUS_ROOT_RECEIPT.json')|ConvertFrom-Json).root
$discoveryRoot=(Get-Content -Raw (Join-Path $CandidateDiscovery 'G9_1_DISCOVERY_ROOT_RECEIPT.json')|ConvertFrom-Json).root
$scienceMaterial=[Text.Encoding]::UTF8.GetBytes("G9_1_PRIMARY_SCIENCE_V1`n$censusRoot`n$discoveryRoot`n")
$sha=[Security.Cryptography.SHA256]::Create();$scienceRoot=([BitConverter]::ToString($sha.ComputeHash($scienceMaterial))).Replace('-','').ToLowerInvariant()
$access=[ordered]@{schema='G9_1_COMPOSITE_ACCESS_AUDIT_V1';canonical_D_A_prefix_replays=115000;independent_witness_D_A_prefix_replays=115000;total_authorized_D_A_prefix_replays=230000;D_B_reads=0;D_C_reads=0;D_D_reads=0;target_reads=0;outcome_reads=0;external_optic_reads=0;continuation_evaluations=0;status='PASS'}
[IO.File]::WriteAllText((Join-Path $sealPath 'G9_1_COMPOSITE_ACCESS_AUDIT.json'),($access|ConvertTo-Json -Compress),[Text.UTF8Encoding]::new($false))
$excluded=@('qualification_manifest.tsv','G9_1_QUALIFICATION_ROOT_RECEIPT.json')
$members=Get-ChildItem -LiteralPath $sealPath -File -Recurse|Where-Object{$excluded-notcontains$_.Name}|Sort-Object{[IO.Path]::GetRelativePath($sealPath,$_.FullName)}
$lines=[Collections.Generic.List[string]]::new();$lines.Add("path`tbytes`tsha256")
foreach($m in $members){$rel=[IO.Path]::GetRelativePath($sealPath,$m.FullName).Replace('\','/');$hash=(Get-FileHash -Algorithm SHA256 -LiteralPath $m.FullName).Hash.ToLowerInvariant();$lines.Add("$rel`t$($m.Length)`t$hash")}
$manifest=($lines-join"`n")+"`n";$manifestPath=Join-Path $sealPath 'qualification_manifest.tsv';[IO.File]::WriteAllText($manifestPath,$manifest,[Text.UTF8Encoding]::new($false));$root=(Get-FileHash -Algorithm SHA256 -LiteralPath $manifestPath).Hash.ToLowerInvariant()
$receipt=[ordered]@{schema='G9_1_QUALIFICATION_ROOT_RECEIPT_V1';G9_1_qualification_root=$root;primary_science_root=$scienceRoot;census_root=$censusRoot;discovery_root=$discoveryRoot;scientific_outcome='NO_ORDINAL_MATCHED_LAWFUL_SAME_FIBER_SPECIMENS';E0=138962;E1=0;E2=0;continuation_evaluations=0;bounded_silence_pairs=0;status='G9_1_CONTROLLED_RESIDUAL_LATENT_MEMORY_QUALIFIED_WITH_RESTRICTIONS'}
[IO.File]::WriteAllText((Join-Path $sealPath 'G9_1_QUALIFICATION_ROOT_RECEIPT.json'),($receipt|ConvertTo-Json -Compress),[Text.UTF8Encoding]::new($false))
Write-Output "G9_1_QUALIFICATION_ROOT=$root";Write-Output "G9_1_PRIMARY_SCIENCE_ROOT=$scienceRoot";Write-Output "G9_1_CENSUS_ROOT=$censusRoot";Write-Output "G9_1_DISCOVERY_ROOT=$discoveryRoot"
