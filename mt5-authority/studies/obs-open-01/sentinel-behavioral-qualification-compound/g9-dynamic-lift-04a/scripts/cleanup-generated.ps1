$ErrorActionPreference = 'Stop'
$g9Root = (Resolve-Path (Join-Path $PSScriptRoot '..')).Path
$targets = [System.Collections.Generic.List[string]]::new()

foreach ($name in @('build-a', 'build-b', 'build-c', 'build-d', 'build-e', 'target')) {
    $path = Join-Path $g9Root $name
    if (Test-Path -LiteralPath $path) {
        $resolved = (Resolve-Path -LiteralPath $path).Path
        if (!$resolved.StartsWith($g9Root + '\', [System.StringComparison]::OrdinalIgnoreCase)) {
            throw "UNSAFE_G9_CLEANUP_TARGET:$resolved"
        }
        $targets.Add($resolved)
    }
}

foreach ($path in @(
    'D:\northstar-g9-target-a',
    'D:\northstar-g9-target-b',
    'D:\northstar-g9-target-c',
    'D:\northstar-g9-target-d',
    'D:\northstar-g9-target-e'
)) {
    if (Test-Path -LiteralPath $path) {
        $resolved = (Resolve-Path -LiteralPath $path).Path
        if (!$resolved.StartsWith('D:\northstar-g9-target-', [System.StringComparison]::OrdinalIgnoreCase)) {
            throw "UNSAFE_D_CLEANUP_TARGET:$resolved"
        }
        $targets.Add($resolved)
    }
}

foreach ($target in $targets) {
    Remove-Item -LiteralPath $target -Recurse -Force
}

[pscustomobject]@{
    removed = $targets.Count
    seal_preserved = Test-Path -LiteralPath (Join-Path $g9Root 'seal\G9_QUALIFICATION_ROOT_RECEIPT.json')
}
