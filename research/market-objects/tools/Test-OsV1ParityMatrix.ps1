[CmdletBinding()]
param(
    [string]$RepositoryRoot = (Resolve-Path (Join-Path $PSScriptRoot '..\..\..')).Path,
    [string]$ArtifactDirectory = (Join-Path $PSScriptRoot '..\artifacts\gate16-6-prep')
)

$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest
$generator = Join-Path $PSScriptRoot 'New-OsV1ParityMatrix.ps1'
$temp = Join-Path ([IO.Path]::GetTempPath()) ("osv1-parity-{0}" -f [Guid]::NewGuid().ToString('N'))
try {
    & $generator -RepositoryRoot $RepositoryRoot -OutputDirectory $temp | Out-Null
    foreach ($name in @('osv1_stagewise_parity_matrix.json','osv1_stagewise_parity_matrix.tsv','osv1_stagewise_parity_matrix_receipt.json')) {
        $left = Join-Path $ArtifactDirectory $name
        $right = Join-Path $temp $name
        if ((Get-FileHash $left -Algorithm SHA256).Hash -ne (Get-FileHash $right -Algorithm SHA256).Hash) {
            throw "non-deterministic parity matrix artifact: $name"
        }
    }
    $matrix = Get-Content -Raw (Join-Path $ArtifactDirectory 'osv1_stagewise_parity_matrix.json') | ConvertFrom-Json
    $c1 = @($matrix.stages | Where-Object stage_id -eq 'C1_RAW_STRUCTURAL_PRODUCERS')
    if ($c1.Count -ne 1 -or $c1[0].status -ne 'OPEN') { throw 'C1 must remain singular and OPEN' }
    $parity = @($matrix.stages | Where-Object claim_class -eq 'INDEPENDENT_RECONSTRUCTION_PARITY')
    if (@($parity | Where-Object { $_.stage_id -ne 'C1_RAW_STRUCTURAL_PRODUCERS' -and $_.status -notlike 'PASS*' }).Count -ne 0) {
        throw 'a certified C2-D reconstruction row does not pass'
    }
    $expectedParityIds = @(
        'C1_RAW_STRUCTURAL_PRODUCERS',
        'C2_NORMALIZATION_AND_CANONICAL_ORDERING',
        'C3_COMPATIBILITY_AND_DBSCAN',
        'C4_NODE_PROVENANCE_AND_LIFECYCLE',
        'C5_CORRIDOR_AND_REGIONAL_STATE',
        'D1_CAUSAL_FROZEN_FEATURES',
        'D2_AUCTION_STATE_AND_RELATIONAL_LEDGER',
        'D3_TRANSIT_RECEIPTS'
    )
    $actualParityIds = @($parity.stage_id)
    if (@(Compare-Object $expectedParityIds $actualParityIds).Count -ne 0) {
        throw 'independent reconstruction parity lane has unexpected membership'
    }
    $confirmation = @($matrix.stages | Where-Object stage_id -eq 'G13R_BEHAVIORAL_HOLDOUT')
    if ($confirmation.Count -ne 1 -or $confirmation[0].claim_class -ne 'CONFIRMATION_NOT_IMPLEMENTATION_PARITY') {
        throw 'Gate 13R confirmation must remain explicitly outside implementation parity'
    }
    $derived = @($matrix.stages | Where-Object stage_id -eq 'G15_TO_G16_5_DERIVED_SCIENCE')
    if ($derived.Count -ne 1 -or $derived[0].claim_class -ne 'DERIVED_REBUILD_DETERMINISM_NOT_MT5_PARITY') {
        throw 'derived science must remain explicitly outside MT5 implementation parity'
    }
    if ($matrix.invariants.economic_authority -or $matrix.invariants.trading_authority) { throw 'authority firewall failed' }
    if ($matrix.invariants.confirmation_windows_opened) { throw 'confirmation windows were opened' }
    Write-Output "OSV1_PARITY_MATRIX_TEST PASS logical_sha256=$($matrix.logical_matrix_sha256)"
}
finally {
    Remove-Item -LiteralPath $temp -Recurse -Force -ErrorAction SilentlyContinue
}
