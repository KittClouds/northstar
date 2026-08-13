[CmdletBinding()]
param(
    [string]$RepositoryRoot = (Resolve-Path (Join-Path $PSScriptRoot '..\..\..')).Path,
    [string]$Mt5Root = 'C:\Users\shuga\AppData\Roaming\MetaQuotes\Terminal\D0E8209F77C8CF37AD8BF550E51FF075\MQL5',
    [string]$Mt5Entry = 'Indicators\MarketObjectResearch\MarketObjectCollector.mq5',
    [string]$OutputPath = (Join-Path $PSScriptRoot '..\artifacts\gate16-6-prep\osv1_source_dependency_closure.json')
)

$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest

function Normalize-PathText([string]$Path) {
    return ([IO.Path]::GetFullPath($Path) -replace '\\', '/')
}

function Relative-OrExternal([string]$Path, [string]$Root) {
    $full = Normalize-PathText $Path
    $base = (Normalize-PathText $Root).TrimEnd('/') + '/'
    if ($full.StartsWith($base, [StringComparison]::OrdinalIgnoreCase)) {
        return $full.Substring($base.Length)
    }
    return "external://$full"
}

function Sha256([string]$Path) {
    return (Get-FileHash -LiteralPath $Path -Algorithm SHA256).Hash.ToLowerInvariant()
}

function Canonical-JsonBytes($Value) {
    $json = $Value | ConvertTo-Json -Depth 100 -Compress
    return [Text.Encoding]::UTF8.GetBytes($json)
}

function Sha256-Bytes([byte[]]$Bytes) {
    $sha = [Security.Cryptography.SHA256]::Create()
    try {
        return ([BitConverter]::ToString($sha.ComputeHash($Bytes))).Replace('-', '').ToLowerInvariant()
    }
    finally {
        $sha.Dispose()
    }
}

function Resolve-MqlInclude([string]$From, [string]$IncludeText, [string]$Delimiter, [string]$IncludeRoot) {
    $candidates = [Collections.Generic.List[string]]::new()
    if ([IO.Path]::IsPathRooted($IncludeText)) {
        $candidates.Add($IncludeText)
    }
    else {
        if ($Delimiter -eq '"') {
            $candidates.Add((Join-Path (Split-Path -Parent $From) $IncludeText))
        }
        $candidates.Add((Join-Path $IncludeRoot $IncludeText))
    }
    foreach ($candidate in $candidates) {
        if (Test-Path -LiteralPath $candidate -PathType Leaf) {
            return [IO.Path]::GetFullPath($candidate)
        }
    }
    return $null
}

function Get-MqlClosure([string]$Root, [string]$EntryRelative) {
    $entry = Join-Path $Root $EntryRelative
    $includeRoot = Join-Path $Root 'Include'
    if (!(Test-Path -LiteralPath $entry -PathType Leaf)) {
        throw "MT5 entry does not exist: $entry"
    }

    $queue = [Collections.Generic.Queue[string]]::new()
    $queue.Enqueue([IO.Path]::GetFullPath($entry))
    $seen = [Collections.Generic.HashSet[string]]::new([StringComparer]::OrdinalIgnoreCase)
    $files = [Collections.Generic.List[object]]::new()
    $edges = [Collections.Generic.List[object]]::new()
    $unresolved = [Collections.Generic.List[object]]::new()

    while ($queue.Count -gt 0) {
        $path = [IO.Path]::GetFullPath($queue.Dequeue())
        if (!$seen.Add($path)) { continue }

        $logical = Relative-OrExternal $path $Root
        $files.Add([ordered]@{
            logical_path = $logical
            physical_path = Normalize-PathText $path
            sha256 = Sha256 $path
            bytes = (Get-Item -LiteralPath $path).Length
            authority_role = if ($logical -like 'Include/01*') { 'MT5_C1_PRODUCER_AUTHORITY' }
                elseif ($logical -like '*MasterParityOracle.mqh') { 'MT5_C2_D_ORACLE_IMPLEMENTATION' }
                elseif ($logical -like 'Include/MasterStructure/*') { 'MT5_C2_D_ORACLE_IMPLEMENTATION' }
                elseif ($logical -like 'Include/MarketObjectResearch/*' -or $logical -like 'Include/ExpansionTrajectoryObserver/*' -or $logical -eq 'Include/RCMExpansion.mqh') { 'MT5_RG3_OBJECT_AUTHORITY' }
                elseif ($logical -like 'Indicators/*') { 'MT5_RG3_ENTRYPOINT' }
                else { 'MT5_TRANSITIVE_SOURCE' }
        })

        foreach ($line in Get-Content -LiteralPath $path) {
            if ($line -notmatch '^\s*#include\s*([<"])([^>"]+)[>"]') { continue }
            $delimiter = $Matches[1]
            $includeText = $Matches[2]
            $resolved = Resolve-MqlInclude $path $includeText $delimiter $includeRoot
            if ($null -eq $resolved) {
                $unresolved.Add([ordered]@{
                    from = $logical
                    include = $includeText
                })
                continue
            }
            $target = Relative-OrExternal $resolved $Root
            $edges.Add([ordered]@{
                from = $logical
                include = $includeText
                to = $target
                resolution = if ($target.StartsWith('external://')) { 'ABSOLUTE_EXTERNAL' } else { 'MT5_INCLUDE_ROOT' }
            })
            $queue.Enqueue($resolved)
        }
    }

    $orderedFiles = @($files | Sort-Object logical_path)
    $orderedEdges = @($edges | Sort-Object from, include, to)
    $orderedUnresolved = @($unresolved | Sort-Object from, include)
    return [ordered]@{
        entrypoint = Relative-OrExternal $entry $Root
        root = Normalize-PathText $Root
        file_count = $orderedFiles.Count
        edge_count = $orderedEdges.Count
        unresolved_count = $orderedUnresolved.Count
        files = $orderedFiles
        edges = $orderedEdges
        unresolved = $orderedUnresolved
    }
}

function Cargo-Packages([string]$LockPath) {
    $packages = [Collections.Generic.List[object]]::new()
    $current = $null
    foreach ($line in Get-Content -LiteralPath $LockPath) {
        if ($line -eq '[[package]]') {
            if ($null -ne $current) { $packages.Add($current) }
            $current = [ordered]@{}
            continue
        }
        if ($null -eq $current) { continue }
        if ($line -match '^(name|version|source|checksum) = "(.*)"$') {
            $current[$Matches[1]] = $Matches[2]
        }
    }
    if ($null -ne $current) { $packages.Add($current) }
    return @($packages | Sort-Object name, version, source)
}

function Get-RustWorkspace([string]$Root, [string]$RepoRoot, [string]$Role) {
    $manifest = Join-Path $Root 'Cargo.toml'
    $lock = Join-Path $Root 'Cargo.lock'
    if (!(Test-Path -LiteralPath $manifest -PathType Leaf) -or !(Test-Path -LiteralPath $lock -PathType Leaf)) {
        throw "Rust workspace is missing Cargo.toml or Cargo.lock: $Root"
    }
    $sourceFiles = @(
        Get-ChildItem -LiteralPath $Root -Recurse -File |
            Where-Object {
                $_.Name -eq 'Cargo.toml' -or $_.Name -eq 'Cargo.lock' -or
                $_.Extension -eq '.rs'
            } |
            Sort-Object FullName
    )
    $files = @($sourceFiles | ForEach-Object {
        [ordered]@{
            logical_path = Relative-OrExternal $_.FullName $RepoRoot
            sha256 = Sha256 $_.FullName
            bytes = $_.Length
        }
    })
    $metadataText = & cargo metadata --format-version 1 --no-deps --manifest-path $manifest
    if ($LASTEXITCODE -ne 0) { throw "cargo metadata failed for $Root" }
    $metadata = $metadataText | ConvertFrom-Json
    $workspaceIds = [Collections.Generic.HashSet[string]]::new([StringComparer]::Ordinal)
    foreach ($id in $metadata.workspace_members) { [void]$workspaceIds.Add([string]$id) }
    $packages = @($metadata.packages |
        Where-Object { $workspaceIds.Contains([string]$_.id) } |
        Sort-Object name |
        ForEach-Object {
            $package = $_
            [ordered]@{
                name = $package.name
                version = $package.version
                manifest = Relative-OrExternal $package.manifest_path $RepoRoot
                local_dependencies = @($package.dependencies |
                    Where-Object { $null -ne $_.PSObject.Properties['path'] -and $null -ne $_.path } |
                    Sort-Object name |
                    ForEach-Object { [ordered]@{
                        name = $_.name
                        path = Relative-OrExternal $_.path $RepoRoot
                    } })
                targets = @($package.targets |
                    Sort-Object name, src_path |
                    ForEach-Object { [ordered]@{
                        name = $_.name
                        kind = @($_.kind)
                        src_path = Relative-OrExternal $_.src_path $RepoRoot
                    } })
            }
        })
    return [ordered]@{
        workspace = Relative-OrExternal $Root $RepoRoot
        authority_role = $Role
        file_count = $files.Count
        files = $files
        local_packages = $packages
        locked_packages = @(Cargo-Packages $lock)
    }
}

$repo = [IO.Path]::GetFullPath($RepositoryRoot)
$mt5 = Get-MqlClosure $Mt5Root $Mt5Entry
if ($mt5.unresolved_count -ne 0) {
    throw "MT5 closure contains $($mt5.unresolved_count) unresolved includes"
}

$auction = Get-RustWorkspace (Join-Path $repo 'research\auction-parity') $repo 'RUST_C2_D_PARITY_AND_GATE13R'
$objects = Get-RustWorkspace (Join-Path $repo 'research\market-objects') $repo 'RUST_RG3_DERIVATION_GATES14_TO16_5'
$protocolPath = Join-Path $repo 'research\market-objects\contracts\gate16_6_osv1_closure.json'
$authorityEvidence = @(
    'research\auction-parity\PHASE12_BD_CERTIFICATE.md',
    'research\auction-parity\PHASE12_CHECKPOINT.md',
    'research\auction-parity\artifacts\phase13r-evaluation\certificate_manifest.json',
    'research\market-objects\artifacts\rg3-gate15-seal-final-v4\gate15-corpus-manifest.json',
    'research\market-objects\artifacts\rg3-gate16-5-seal-final-v1\gate16_5-seal-receipt.json'
) | ForEach-Object {
    $path = Join-Path $repo $_
    if (!(Test-Path -LiteralPath $path -PathType Leaf)) { throw "missing authority evidence: $path" }
    [ordered]@{ path = Relative-OrExternal $path $repo; sha256 = Sha256 $path }
}

$mirrorPath = Join-Path $repo 'research\auction-parity\mql5\MasterParityOracle.mqh'
$selectedOracle = @($mt5.files | Where-Object { $_.logical_path -like 'external://*MasterParityOracle.mqh' })
$mirrorComparison = [ordered]@{
    selected_absolute_dependency_count = $selectedOracle.Count
    repository_mirror_path = Relative-OrExternal $mirrorPath $repo
    repository_mirror_sha256 = if (Test-Path -LiteralPath $mirrorPath) { Sha256 $mirrorPath } else { $null }
    selected_dependency_sha256 = if ($selectedOracle.Count -eq 1) { $selectedOracle[0].sha256 } else { $null }
    byte_identical = if ($selectedOracle.Count -eq 1 -and (Test-Path -LiteralPath $mirrorPath)) {
        $selectedOracle[0].sha256 -eq (Sha256 $mirrorPath)
    } else { $null }
    substitution_performed = $false
}

$content = [ordered]@{
    contract = 'NORTHSTAR_GATE16_6_OSV1_SOURCE_DEPENDENCY_CLOSURE_V1'
    status = 'PASS'
    authority = [ordered]@{
        mt5_c1 = 'AUTHORITATIVE_OPEN_RUST_PARITY'
        rust_c2_d = 'PARITY_VERIFIED_FROM_RAW_PRODUCER_OUTPUT_BOUNDARY'
        rust_rg3 = 'DETERMINISTIC_DERIVATION_AND_SEALING'
    }
    protocol = [ordered]@{
        path = Relative-OrExternal $protocolPath $repo
        sha256 = Sha256 $protocolPath
    }
    authority_evidence = @($authorityEvidence)
    mt5 = $mt5
    rust_workspaces = @($auction, $objects)
    mirror_comparison = $mirrorComparison
    invariants = [ordered]@{
        mt5_unresolved_includes_zero = ($mt5.unresolved_count -eq 0)
        absolute_dependencies_preserved = ($selectedOracle.Count -eq 1)
        mirror_substitution_forbidden = $true
        rust_lockfiles_present = $true
        predecessor_artifacts_mutated = $false
        confirmation_windows_opened = $false
    }
}

$logical = [ordered]@{
    contract = $content.contract
    authority = $content.authority
    protocol = $content.protocol
    authority_evidence = $content.authority_evidence
    mt5 = $content.mt5
    rust_workspaces = $content.rust_workspaces
    mirror_comparison = $content.mirror_comparison
    invariants = $content.invariants
}
$content.logical_closure_sha256 = Sha256-Bytes (Canonical-JsonBytes $logical)

$out = [IO.Path]::GetFullPath($OutputPath)
$parent = Split-Path -Parent $out
New-Item -ItemType Directory -Force -Path $parent | Out-Null
$json = $content | ConvertTo-Json -Depth 100
[IO.File]::WriteAllText($out, $json + "`n", [Text.UTF8Encoding]::new($false))
Write-Output "OSV1_SOURCE_CLOSURE path=$out mt5_files=$($mt5.file_count) rust_files=$($auction.file_count + $objects.file_count) logical_sha256=$($content.logical_closure_sha256)"
