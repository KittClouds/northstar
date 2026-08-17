param([Parameter(Mandatory = $true)][string]$Seal)

$ErrorActionPreference = 'Stop'
$sealPath = (Resolve-Path -LiteralPath $Seal).Path
$oldReceiptPath = Join-Path $sealPath 'G9_QUALIFICATION_ROOT_RECEIPT.json'
$oldReceipt = Get-Content $oldReceiptPath -Raw | ConvertFrom-Json
$predecessor = $oldReceipt.G9_qualification_root
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
$receipt = [ordered]@{
    schema = 'G9_QUALIFICATION_ROOT_RECEIPT_V1'
    G9_qualification_root = $qualificationRoot
    predecessor_qualification_root = $predecessor
    amendment = 'ADD_UNIFIED_CAMPAIGN_FINAL_REPORT_ONLY'
    scientific_root_changed = $false
    complete_run_root_changed = $false
    primary_science_root = $oldReceipt.primary_science_root
    complete_run_root = $oldReceipt.complete_run_root
    independent_replay_receipt = 'G9_INDEPENDENT_REPLAY_RECEIPT.json'
    qualification_manifest_members = $members.Count
    status = 'G9_DYNAMIC_LIFT_04A_QUALIFIED_WITH_RESTRICTIONS'
}
$json = $receipt | ConvertTo-Json -Depth 4 -Compress
[System.IO.File]::WriteAllText($oldReceiptPath, $json, [System.Text.UTF8Encoding]::new($false))
Write-Output "G9_QUALIFICATION_ROOT=$qualificationRoot"
Write-Output "PREDECESSOR_QUALIFICATION_ROOT=$predecessor"
