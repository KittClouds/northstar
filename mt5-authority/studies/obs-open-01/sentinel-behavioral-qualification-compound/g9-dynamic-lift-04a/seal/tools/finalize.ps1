param(
    [Parameter(Mandatory = $true)][string]$Candidate,
    [Parameter(Mandatory = $true)][string]$Witness,
    [Parameter(Mandatory = $true)][string]$Seal
)

$ErrorActionPreference = 'Stop'
$candidatePath = (Resolve-Path -LiteralPath $Candidate).Path
$witnessPath = (Resolve-Path -LiteralPath $Witness).Path
$sealPath = [System.IO.Path]::GetFullPath($Seal)

if (Test-Path -LiteralPath $sealPath) {
    if ((Get-ChildItem -LiteralPath $sealPath -Force | Measure-Object).Count -ne 0) {
        throw "SEAL_DIRECTORY_NOT_EMPTY:$sealPath"
    }
} else {
    New-Item -ItemType Directory -Path $sealPath | Out-Null
}

$candidateFiles = Get-ChildItem -LiteralPath $candidatePath -File -Recurse
$witnessFiles = Get-ChildItem -LiteralPath $witnessPath -File -Recurse
if ($candidateFiles.Count -ne $witnessFiles.Count) {
    throw 'INDEPENDENT_REPLAY_FILE_COUNT_MISMATCH'
}
foreach ($file in $candidateFiles) {
    $relative = [System.IO.Path]::GetRelativePath($candidatePath, $file.FullName)
    $other = Join-Path $witnessPath $relative
    if (!(Test-Path -LiteralPath $other)) {
        throw "INDEPENDENT_REPLAY_MEMBER_MISSING:$relative"
    }
    $leftHash = (Get-FileHash -Algorithm SHA256 -LiteralPath $file.FullName).Hash
    $rightHash = (Get-FileHash -Algorithm SHA256 -LiteralPath $other).Hash
    if ($leftHash -ne $rightHash) {
        throw "INDEPENDENT_REPLAY_MEMBER_DRIFT:$relative"
    }
}

Copy-Item -Path (Join-Path $candidatePath '*') -Destination $sealPath -Recurse -Force
$toolsPath = Join-Path $sealPath 'tools'
New-Item -ItemType Directory -Path $toolsPath | Out-Null
Copy-Item -LiteralPath $PSCommandPath -Destination (Join-Path $toolsPath 'finalize.ps1')

$candidateRoot = Get-Content (Join-Path $candidatePath 'G9_ROOT_RECEIPT.json') -Raw | ConvertFrom-Json
$witnessRoot = Get-Content (Join-Path $witnessPath 'G9_ROOT_RECEIPT.json') -Raw | ConvertFrom-Json
$replayReceipt = [ordered]@{
    schema = 'G9_INDEPENDENT_REPLAY_RECEIPT_V1'
    candidate_build = 'BUILD_D'
    witness_build = 'BUILD_E'
    candidate_primary_science_root = $candidateRoot.primary_science_root
    witness_primary_science_root = $witnessRoot.primary_science_root
    candidate_complete_run_root = $candidateRoot.G9_root
    witness_complete_run_root = $witnessRoot.G9_root
    compared_files = $candidateFiles.Count
    byte_differences = 0
    binary_reproducibility = 'NOT_REQUIRED_NOT_CLAIMED'
    semantic_and_artifact_replay = 'PASS'
    status = 'PASS'
}
$replayJson = $replayReceipt | ConvertTo-Json -Depth 4 -Compress
[System.IO.File]::WriteAllText(
    (Join-Path $sealPath 'G9_INDEPENDENT_REPLAY_RECEIPT.json'),
    $replayJson,
    [System.Text.UTF8Encoding]::new($false)
)

$excluded = @('qualification_manifest.tsv', 'G9_QUALIFICATION_ROOT_RECEIPT.json')
$members = Get-ChildItem -LiteralPath $sealPath -File -Recurse |
    Where-Object { $excluded -notcontains $_.Name } |
    Sort-Object { [System.IO.Path]::GetRelativePath($sealPath, $_.FullName) }
$lines = [System.Collections.Generic.List[string]]::new()
$lines.Add("path`tbytes`tsha256")
foreach ($member in $members) {
    $relative = [System.IO.Path]::GetRelativePath($sealPath, $member.FullName).Replace('\', '/')
    $hash = (Get-FileHash -Algorithm SHA256 -LiteralPath $member.FullName).Hash.ToLowerInvariant()
    $lines.Add("$relative`t$($member.Length)`t$hash")
}
$manifestText = ($lines -join "`n") + "`n"
$manifestPath = Join-Path $sealPath 'qualification_manifest.tsv'
[System.IO.File]::WriteAllText($manifestPath, $manifestText, [System.Text.UTF8Encoding]::new($false))
$qualificationRoot = (Get-FileHash -Algorithm SHA256 -LiteralPath $manifestPath).Hash.ToLowerInvariant()
$rootReceipt = [ordered]@{
    schema = 'G9_QUALIFICATION_ROOT_RECEIPT_V1'
    G9_qualification_root = $qualificationRoot
    primary_science_root = $candidateRoot.primary_science_root
    complete_run_root = $candidateRoot.G9_root
    independent_replay_receipt = 'G9_INDEPENDENT_REPLAY_RECEIPT.json'
    qualification_manifest_members = $members.Count
    status = 'G9_DYNAMIC_LIFT_04A_QUALIFIED_WITH_RESTRICTIONS'
}
$rootJson = $rootReceipt | ConvertTo-Json -Depth 4 -Compress
[System.IO.File]::WriteAllText(
    (Join-Path $sealPath 'G9_QUALIFICATION_ROOT_RECEIPT.json'),
    $rootJson,
    [System.Text.UTF8Encoding]::new($false)
)
Write-Output "G9_QUALIFICATION_ROOT=$qualificationRoot"
Write-Output "G9_PRIMARY_SCIENCE_ROOT=$($candidateRoot.primary_science_root)"
Write-Output "G9_COMPLETE_RUN_ROOT=$($candidateRoot.G9_root)"
