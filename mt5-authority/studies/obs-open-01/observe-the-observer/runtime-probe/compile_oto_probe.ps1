param(
    [Parameter(Mandatory = $false)]
    [string]$RepositoryRoot = (Resolve-Path "$PSScriptRoot\..\..\..\..\..").Path
)

$ErrorActionPreference = 'Stop'

$allowedEditor = 'C:\Program Files\MetaTrader 5\metaeditor64.exe'
$allowedVersion = '5.0.0.6116'
$allowedSha256 = 'f6e48c5f1ab83729bbf9580e90f4acf466e05bb8a725a63d0c205dcce2bb3778'
$forbiddenEditor = 'C:\Program Files\Trading.com Markets MT5\metaeditor64.exe'
$forbiddenSha256 = '06c85acc269f52c9eec63bca51e33c8b57f9940ba95d8a5ef03105d230b18440'

if (-not (Test-Path -LiteralPath $allowedEditor -PathType Leaf)) {
    throw "OTO compiler unavailable: $allowedEditor"
}

$resolvedEditor = (Resolve-Path -LiteralPath $allowedEditor).Path
$resolvedForbidden = if (Test-Path -LiteralPath $forbiddenEditor -PathType Leaf) {
    (Resolve-Path -LiteralPath $forbiddenEditor).Path
} else {
    $forbiddenEditor
}
$actualHash = (Get-FileHash -LiteralPath $resolvedEditor -Algorithm SHA256).Hash.ToLowerInvariant()
$actualVersion = (Get-Item -LiteralPath $resolvedEditor).VersionInfo.FileVersion

if ($resolvedEditor -eq $resolvedForbidden -or $actualHash -eq $forbiddenSha256) {
    throw 'OTO_COMPILER_QUARANTINE_VIOLATION: Trading.com MetaEditor is forbidden'
}
if ($resolvedEditor -ne $allowedEditor) {
    throw "OTO compiler origin mismatch: $resolvedEditor"
}
if ($actualVersion -ne $allowedVersion -or $actualHash -ne $allowedSha256) {
    throw "OTO compiler identity changed: version=$actualVersion sha256=$actualHash"
}

$source = Join-Path $PSScriptRoot 'OBS_OPEN_OTO_RuntimeParameterProbe.mq5'
$log = Join-Path $PSScriptRoot 'compile-runtime-parameter-probe.log'
if (-not (Test-Path -LiteralPath $source -PathType Leaf)) {
    throw "OTO probe source missing: $source"
}

$process = Start-Process -FilePath $resolvedEditor `
    -ArgumentList @("/compile:$source", "/log:$log") `
    -Wait -PassThru -WindowStyle Hidden

if (-not (Test-Path -LiteralPath $log -PathType Leaf)) {
    throw 'OTO compiler produced no log'
}
$logText = Get-Content -LiteralPath $log -Raw
if ($logText -notmatch 'Result:\s+0 errors, 0 warnings') {
    throw "OTO compile failed; inspect $log"
}

[pscustomobject]@{
    schema = 'OTO_QUARANTINED_COMPILE_RECEIPT_V1'
    editor_path = $resolvedEditor
    editor_version = $actualVersion
    editor_sha256 = $actualHash
    forbidden_editor_path = $resolvedForbidden
    forbidden_editor_sha256 = $forbiddenSha256
    quarantine_status = 'PASS'
    process_exit_code = $process.ExitCode
    source_sha256 = (Get-FileHash -LiteralPath $source -Algorithm SHA256).Hash.ToLowerInvariant()
    ex5_sha256 = (Get-FileHash -LiteralPath ([IO.Path]::ChangeExtension($source, '.ex5')) -Algorithm SHA256).Hash.ToLowerInvariant()
    log_sha256 = (Get-FileHash -LiteralPath $log -Algorithm SHA256).Hash.ToLowerInvariant()
} | ConvertTo-Json -Depth 3
