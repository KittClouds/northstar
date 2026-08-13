[CmdletBinding()]
param(
    [string]$RepositoryRoot = (Resolve-Path (Join-Path $PSScriptRoot '..\..\..')).Path,
    [string]$Mt5Root = 'C:\Users\shuga\AppData\Roaming\MetaQuotes\Terminal\D0E8209F77C8CF37AD8BF550E51FF075\MQL5',
    [string]$ArtifactPath = (Join-Path $PSScriptRoot '..\artifacts\gate16-6-prep\osv1_source_dependency_closure.json')
)

$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest

$generator = Join-Path $PSScriptRoot 'New-OsV1SourceClosure.ps1'
$temp = Join-Path ([IO.Path]::GetTempPath()) ("osv1-source-closure-{0}.json" -f [Guid]::NewGuid().ToString('N'))
try {
    & $generator -RepositoryRoot $RepositoryRoot -Mt5Root $Mt5Root -OutputPath $temp | Out-Null
    $expected = Get-Content -Raw -LiteralPath $ArtifactPath | ConvertFrom-Json
    $actual = Get-Content -Raw -LiteralPath $temp | ConvertFrom-Json

    if ((Get-FileHash -LiteralPath $ArtifactPath -Algorithm SHA256).Hash -ne
        (Get-FileHash -LiteralPath $temp -Algorithm SHA256).Hash) {
        throw 'source closure rebuild is not byte-identical'
    }
    if ($actual.status -ne 'PASS') { throw 'source closure did not pass' }
    if ($actual.mt5.unresolved_count -ne 0) { throw 'MT5 includes remain unresolved' }
    if ($actual.mt5.file_count -lt 1 -or $actual.mt5.edge_count -lt 1) { throw 'MT5 closure is empty' }
    if ($actual.rust_workspaces.Count -ne 2) { throw 'expected exactly two Rust workspaces' }
    if ($actual.mirror_comparison.selected_absolute_dependency_count -ne 1) {
        throw 'expected one preserved absolute MasterParityOracle dependency'
    }
    if ($actual.mirror_comparison.substitution_performed) { throw 'source substitution occurred' }
    if ($actual.invariants.confirmation_windows_opened) { throw 'confirmation-window invariant failed' }
    if ($actual.logical_closure_sha256 -ne $expected.logical_closure_sha256) {
        throw 'logical closure hash changed'
    }
    Write-Output ("OSV1_SOURCE_CLOSURE_TEST PASS byte_sha256={0} logical_sha256={1}" -f
        (Get-FileHash -LiteralPath $ArtifactPath -Algorithm SHA256).Hash.ToLowerInvariant(),
        $actual.logical_closure_sha256)
}
finally {
    Remove-Item -LiteralPath $temp -Force -ErrorAction SilentlyContinue
}
