[CmdletBinding()]
param(
    [string]$RepositoryRoot = (Resolve-Path (Join-Path $PSScriptRoot '..\..\..')).Path,
    [string]$ArtifactDirectory = (Join-Path $PSScriptRoot '..\artifacts\gate16-6-prep')
)

$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest

function Assert-Same([string]$Left, [string]$Right) {
    if ((Get-FileHash -LiteralPath $Left -Algorithm SHA256).Hash -ne
        (Get-FileHash -LiteralPath $Right -Algorithm SHA256).Hash) {
        throw "non-deterministic artifact: $(Split-Path -Leaf $Left)"
    }
}

$thetaGenerator = Join-Path $PSScriptRoot 'New-OsV1Theta0.ps1'
$findingsGenerator = Join-Path $PSScriptRoot 'New-OsV1MachineFindings.ps1'
$temp = Join-Path ([IO.Path]::GetTempPath()) ("osv1-authority-{0}" -f [Guid]::NewGuid().ToString('N'))
try {
    New-Item -ItemType Directory -Force -Path $temp | Out-Null
    & $thetaGenerator -RepositoryRoot $RepositoryRoot -OutputDirectory $temp | Out-Null
    & $findingsGenerator -RepositoryRoot $RepositoryRoot -OutputDirectory $temp | Out-Null

    $files = @(
        'osv1_theta0_fields.json','osv1_theta0_fields.tsv','osv1_theta0_runs.tsv','osv1_theta0_receipt.json',
        'osv1_machine_findings.json','osv1_machine_findings.tsv','osv1_machine_insufficiencies.tsv','osv1_machine_findings_receipt.json'
    )
    foreach ($name in $files) {
        Assert-Same (Join-Path $ArtifactDirectory $name) (Join-Path $temp $name)
    }

    $theta = Get-Content -Raw -LiteralPath (Join-Path $ArtifactDirectory 'osv1_theta0_fields.json') | ConvertFrom-Json
    $thetaReceipt = Get-Content -Raw -LiteralPath (Join-Path $ArtifactDirectory 'osv1_theta0_receipt.json') | ConvertFrom-Json
    $runs = @(Get-Content -LiteralPath (Join-Path $ArtifactDirectory 'osv1_theta0_runs.tsv') | ConvertFrom-Csv -Delimiter "`t")
    if ($theta.status -ne 'PASS_WITH_EXPLICIT_RUN_BINDING_LIMITS' -or $thetaReceipt.status -ne 'PASS') {
        throw 'Theta0 census did not pass'
    }
    if ($theta.field_count -ne 134 -or $thetaReceipt.field_count -ne 134) { throw 'Theta0 field census drifted' }
    if ($runs.Count -ne 42 -or @($runs.run_key | Sort-Object -Unique).Count -ne 42) { throw 'Theta0 run census is not 42 unique runs' }
    if (@($runs.configuration_hash | Sort-Object -Unique).Count -ne 1) { throw 'Theta0 has multiple recorded configuration hashes' }
    if ($theta.configuration_hash.value -ne '11558041990222457072') { throw 'unexpected RG3 configuration hash' }

    $fieldIds = @($theta.fields.field_id)
    if (@($fieldIds | Sort-Object -Unique).Count -ne $fieldIds.Count) { throw 'Theta0 field IDs are not unique' }
    $fresh = @($theta.fields | Where-Object field_id -eq 'compression.require_fresh_lineage_window')
    if ($fresh.Count -ne 1 -or $fresh[0].consumption_status -ne 'DECLARED_HASHED_NOT_OPERATIONAL') {
        throw 'fresh-lineage input was promoted to operational behavior'
    }
    $recovered = @($theta.fields | Where-Object evidence_class -eq 'RECOVERED_FROM_COLLECTION_ENVIRONMENT')
    if ($recovered.Count -eq 0 -or @($recovered | Where-Object run_binding -notlike 'NOT_RUN_BOUND*').Count -ne 0) {
        throw 'source-recovered values were called run-bound'
    }
    if ($theta.invariants.ex5_hash_available -or $theta.invariants.source_defaults_called_run_recorded -or $theta.invariants.confirmation_windows_opened) {
        throw 'Theta0 authority limit failed'
    }

    $findings = Get-Content -Raw -LiteralPath (Join-Path $ArtifactDirectory 'osv1_machine_findings.json') | ConvertFrom-Json
    $findingsReceipt = Get-Content -Raw -LiteralPath (Join-Path $ArtifactDirectory 'osv1_machine_findings_receipt.json') | ConvertFrom-Json
    if ($findings.status -ne 'PASS_MACHINE_ARTIFACTS_ONLY' -or $findingsReceipt.status -ne 'PASS') {
        throw 'machine findings did not pass'
    }
    if ($findings.finding_count -ne 24 -or $findings.insufficiency_count -ne 18 -or $findings.evidence_count -ne 8) {
        throw 'machine finding census drifted'
    }
    if ($findings.invariants.human_summary_sources -ne 0 -or $findings.invariants.markdown_sources -ne 0 -or $findings.invariants.unsealed_machine_sources -ne 0) {
        throw 'non-machine or unsealed authority entered findings'
    }
    if ($findings.invariants.economic_authority -or $findings.invariants.trading_authority -or $findings.invariants.confirmation_windows_opened) {
        throw 'findings authority firewall failed'
    }
    foreach ($evidence in $findings.evidence) {
        if ([IO.Path]::GetExtension($evidence.artifact_path) -ne '.json' -or [IO.Path]::GetExtension($evidence.seal_path) -ne '.json') {
            throw "non-JSON evidence admitted: $($evidence.evidence_id)"
        }
        $artifact = Join-Path $RepositoryRoot $evidence.artifact_path
        $seal = Join-Path $RepositoryRoot $evidence.seal_path
        if ((Get-FileHash -LiteralPath $artifact -Algorithm SHA256).Hash.ToLowerInvariant() -ne $evidence.artifact_sha256 -or
            (Get-FileHash -LiteralPath $seal -Algorithm SHA256).Hash.ToLowerInvariant() -ne $evidence.seal_sha256) {
            throw "machine evidence drift: $($evidence.evidence_id)"
        }
    }
    $evidenceIds = @($findings.evidence.evidence_id)
    foreach ($item in @($findings.findings) + @($findings.insufficiencies)) {
        if ($item.source_kind -notin @('MACHINE_DERIVED','ARTIFACT_DECLARED','HUMAN_SUMMARY') -or
            $item.source_kind -eq 'HUMAN_SUMMARY' -or $item.evidence_id -notin $evidenceIds) {
            throw 'finding lacks sealed machine authority'
        }
    }
    $sourceKinds = @($findings.findings | Group-Object source_kind)
    if (@($sourceKinds | Where-Object Name -eq 'MACHINE_DERIVED').Count -ne 1 -or
        @($sourceKinds | Where-Object Name -eq 'ARTIFACT_DECLARED').Count -ne 1 -or
        @($sourceKinds | Where-Object Name -eq 'HUMAN_SUMMARY').Count -ne 0) {
        throw 'finding source-kind census is incomplete or admits human summary authority'
    }

    Write-Output "OSV1_AUTHORITY_TEST PASS theta0=$($theta.logical_theta0_sha256) findings=$($findings.logical_findings_sha256)"
}
finally {
    Remove-Item -LiteralPath $temp -Recurse -Force -ErrorAction SilentlyContinue
}
