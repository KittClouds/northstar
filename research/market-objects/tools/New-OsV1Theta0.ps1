[CmdletBinding()]
param(
    [string]$RepositoryRoot = (Resolve-Path (Join-Path $PSScriptRoot '..\..\..')).Path,
    [string]$OutputDirectory = (Join-Path $PSScriptRoot '..\artifacts\gate16-6-prep'),
    [string]$SourceClosurePath = ''
)

$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest

function Sha256([string]$Path) {
    (Get-FileHash -LiteralPath $Path -Algorithm SHA256).Hash.ToLowerInvariant()
}

function Sha256-Bytes([byte[]]$Bytes) {
    $sha = [Security.Cryptography.SHA256]::Create()
    try { ([BitConverter]::ToString($sha.ComputeHash($Bytes))).Replace('-', '').ToLowerInvariant() }
    finally { $sha.Dispose() }
}

function Canonical-Hash($Value) {
    $json = $Value | ConvertTo-Json -Depth 100 -Compress
    Sha256-Bytes ([Text.Encoding]::UTF8.GetBytes($json))
}

function Relative-Path([string]$Path, [string]$Root) {
    $full = ([IO.Path]::GetFullPath($Path) -replace '\\', '/')
    $base = (([IO.Path]::GetFullPath($Root) -replace '\\', '/').TrimEnd('/') + '/')
    if (!$full.StartsWith($base, [StringComparison]::OrdinalIgnoreCase)) {
        throw "path escapes repository: $full"
    }
    $full.Substring($base.Length)
}

function Parse-Ini([string]$Path) {
    $sections = [ordered]@{}
    $section = ''
    foreach ($line in Get-Content -LiteralPath $Path) {
        $trimmed = $line.Trim()
        if ($trimmed -eq '' -or $trimmed.StartsWith(';')) { continue }
        if ($trimmed -match '^\[(.+)\]$') {
            $section = $Matches[1]
            if (!$sections.Contains($section)) { $sections[$section] = [ordered]@{} }
            continue
        }
        if ($trimmed -match '^([^=]+)=(.*)$') {
            if ($section -eq '') { throw "INI value outside section: $Path" }
            $sections[$section][$Matches[1].Trim()] = $Matches[2].Trim()
        }
    }
    $sections
}

function Typed-Value([string]$Text, [string]$Type) {
    switch ($Type) {
        'bool' {
            if ($Text -eq '1' -or $Text.Equals('true', [StringComparison]::OrdinalIgnoreCase)) { return $true }
            if ($Text -eq '0' -or $Text.Equals('false', [StringComparison]::OrdinalIgnoreCase)) { return $false }
            throw "invalid boolean value: $Text"
        }
        'int' { return [long]::Parse($Text, [Globalization.CultureInfo]::InvariantCulture) }
        'double' { return [double]::Parse($Text, [Globalization.CultureInfo]::InvariantCulture) }
        default { return $Text }
    }
}

$repo = [IO.Path]::GetFullPath($RepositoryRoot)
$closurePath = if ([string]::IsNullOrWhiteSpace($SourceClosurePath)) {
    Join-Path $repo 'research\market-objects\artifacts\gate16-6-prep\osv1_source_dependency_closure.json'
} else {
    [IO.Path]::GetFullPath($SourceClosurePath)
}
$manifestPath = Join-Path $repo 'research\market-objects\artifacts\rg3-gate15-seal-final-v4\gate15-corpus-manifest.json'
if (!(Test-Path -LiteralPath $closurePath) -or !(Test-Path -LiteralPath $manifestPath)) {
    throw 'source closure and Gate 15 corpus manifest are required'
}
$closure = Get-Content -Raw -LiteralPath $closurePath | ConvertFrom-Json
$manifest = Get-Content -Raw -LiteralPath $manifestPath | ConvertFrom-Json
if ($closure.status -ne 'PASS' -or $manifest.status -ne 'SEALED' -or $manifest.admitted_run_count -ne 42) {
    throw 'Theta0 admission requires PASS source closure and exactly 42 sealed RG3 runs'
}

$sourceByLogical = @{}
foreach ($source in $closure.mt5.files) { $sourceByLogical[[string]$source.logical_path] = $source }
function Source-Provenance([string]$Logical, [string]$Marker) {
    if (!$sourceByLogical.ContainsKey($Logical)) { throw "source is outside sealed closure: $Logical" }
    $source = $sourceByLogical[$Logical]
    if ((Sha256 $source.physical_path) -ne $source.sha256) { throw "source drift: $Logical" }
    $lines = Get-Content -LiteralPath $source.physical_path
    $locations = [Collections.Generic.List[int]]::new()
    for ($i = 0; $i -lt $lines.Count; $i++) {
        if ($lines[$i].Contains($Marker)) { $locations.Add($i + 1) }
    }
    if ($locations.Count -eq 0) { throw "source marker not found: $Logical :: $Marker" }
    [ordered]@{
        kind = 'SEALED_SOURCE_CLOSURE'
        logical_path = $Logical
        sha256 = $source.sha256
        marker = $Marker
        lines = @($locations)
    }
}

$searchRoots = @(
    (Join-Path $repo 'research\market-objects\artifacts\rg3-gate15-production'),
    (Join-Path $repo 'research\market-objects\artifacts\rg3-qualification')
)
$sealIndex = @{}
foreach ($root in $searchRoots) {
    foreach ($file in Get-ChildItem -LiteralPath $root -Recurse -Filter 'run-seal.json' -File) {
        $sealIndex[(Sha256 $file.FullName)] = $file.FullName
    }
}

$runRows = [Collections.Generic.List[object]]::new()
$configHashLines = [Collections.Generic.List[string]]::new()
$runEvidenceLines = [Collections.Generic.List[string]]::new()
foreach ($admitted in @($manifest.admitted_runs | Sort-Object run_key)) {
    $sealHash = [string]$admitted.run_seal_sha256
    if (!$sealIndex.ContainsKey($sealHash)) { throw "admitted run seal not found: $sealHash" }
    $sealPath = $sealIndex[$sealHash]
    $runDirectory = Split-Path -Parent $sealPath
    $collectionRoot = Split-Path -Parent (Split-Path -Parent $runDirectory)
    $configName = '{0}_{1}_{2}.ini' -f $admitted.instrument,$admitted.window_start,$admitted.window_end
    $configPath = Join-Path $collectionRoot "configs\$configName"
    $runPath = Join-Path $runDirectory 'raw\measurement_runs.tsv'
    if (!(Test-Path -LiteralPath $configPath) -or !(Test-Path -LiteralPath $runPath)) {
        throw "missing config or measurement ledger for $($admitted.run_key)"
    }
    $ini = Parse-Ini $configPath
    $end = @(Get-Content -LiteralPath $runPath | ConvertFrom-Csv -Delimiter "`t" | Where-Object record_type -eq 'END')
    if ($end.Count -ne 1 -or $end[0].run_key -ne $admitted.run_key) {
        throw "run identity mismatch: $($admitted.run_key)"
    }
    $configHashLines.Add("$(Relative-Path $configPath $repo)`t$(Sha256 $configPath)")
    $runEvidenceLines.Add("$($admitted.run_key)`t$(Sha256 $runPath)`t$sealHash")
    $runRows.Add([ordered]@{
        run_key = [string]$admitted.run_key
        canonical_instrument = [string]$end[0].canonical_instrument
        broker_symbol = [string]$end[0].broker_symbol
        timeframe = [string]$end[0].timeframe
        window_start = [string]$end[0].window_start
        window_end = [string]$end[0].window_end
        configuration_hash = [string]$end[0].config_hash
        data_source_id = [string]$end[0].data_source_id
        data_fingerprint = [string]$end[0].data_fingerprint
        contract = [string]$end[0].contract
        research_generation = [string]$end[0].research_generation
        source_auction_generation = [string]$end[0].source_auction_generation
        raw_schema_version = [string]$end[0].raw_schema_version
        compression_grammar_version = [string]$end[0].compression_grammar_version
        expansion_grammar_version = [string]$end[0].expansion_grammar_version
        config_path = Relative-Path $configPath $repo
        config_sha256 = Sha256 $configPath
        measurement_runs_path = Relative-Path $runPath $repo
        measurement_runs_sha256 = Sha256 $runPath
        run_seal_path = Relative-Path $sealPath $repo
        run_seal_sha256 = $sealHash
        tester = $ini['Tester']
        inputs = $ini['TesterInputs']
    })
}
if ($runRows.Count -ne 42) { throw "Theta0 reconstructed $($runRows.Count), expected 42 runs" }

$configSetSha = Sha256-Bytes ([Text.Encoding]::UTF8.GetBytes((@($configHashLines) -join "`n") + "`n"))
$runSetSha = Sha256-Bytes ([Text.Encoding]::UTF8.GetBytes((@($runEvidenceLines) -join "`n") + "`n"))
$fields = [Collections.Generic.List[object]]::new()

function Value-Summary([object[]]$Values, [string]$Type) {
    $distinctText = @($Values | ForEach-Object { [string]$_ } | Sort-Object -Unique)
    $typed = @($distinctText | ForEach-Object { Typed-Value $_ $Type })
    [ordered]@{
        mode = if ($distinctText.Count -eq 1) { 'CONSTANT' } else { 'PER_RUN' }
        value = if ($distinctText.Count -eq 1) { $typed[0] } else { $null }
        distinct_count = $distinctText.Count
        distinct_values = $typed
    }
}

function Add-CampaignField(
    [string]$FieldId,
    [string]$Layer,
    [string]$Section,
    [string]$Key,
    [string]$Type,
    [string]$Consumption,
    [string]$HashMembership,
    [string]$SourceMarker = ''
) {
    $values = @($runRows | ForEach-Object {
        $sectionValue = if ($Section -eq 'Tester') { $_.tester } else { $_.inputs }
        if ($null -eq $sectionValue.PSObject.Properties[$Key] -and !$sectionValue.Contains($Key)) {
            throw "missing [$Section].$Key in $($_.config_path)"
        }
        [string]$sectionValue[$Key]
    })
    $provenance = [Collections.Generic.List[object]]::new()
    $provenance.Add([ordered]@{
        kind = 'CAMPAIGN_CONFIG_SET'
        evidence_class = 'CAMPAIGN_CONFIGURED'
        locator = "[$Section].$Key"
        admitted_run_count = 42
        config_set_sha256 = $configSetSha
    })
    if ($SourceMarker -ne '') {
        $provenance.Add((Source-Provenance 'Indicators/MarketObjectResearch/MarketObjectCollector.mq5' $SourceMarker))
    }
    $fields.Add([ordered]@{
        field_id = $FieldId
        layer = $Layer
        value_type = $Type
        value_summary = Value-Summary $values $Type
        evidence_class = 'CAMPAIGN_CONFIGURED'
        run_binding = 'CONFIG_PRESENT_FOR_ALL_42_ADMITTED_RUNS'
        consumption_status = $Consumption
        configuration_hash_membership = $HashMembership
        provenance = @($provenance)
    })
}

function Add-RunField([string]$FieldId, [string]$Column, [string]$Type, [string]$Consumption, [string]$HashMembership) {
    $values = @($runRows | ForEach-Object { [string]$_.$Column })
    $fields.Add([ordered]@{
        field_id = $FieldId
        layer = 'RUN_IDENTITY'
        value_type = $Type
        value_summary = Value-Summary $values $Type
        evidence_class = 'RUN_RECORDED'
        run_binding = 'MEASUREMENT_RUN_END_FOR_ALL_42_ADMITTED_RUNS'
        consumption_status = $Consumption
        configuration_hash_membership = $HashMembership
        provenance = @([ordered]@{
            kind = 'SEALED_RUN_LEDGER_SET'
            locator = "measurement_runs.tsv::END.$Column"
            admitted_run_count = 42
            run_ledger_set_sha256 = $runSetSha
        })
    })
}

function Add-SourceField(
    [string]$FieldId,
    [string]$Layer,
    [string]$Type,
    $Value,
    [string]$InitializerLogical,
    [string]$InitializerMarker,
    [string]$ConsumerLogical,
    [string]$ConsumerMarker,
    [string]$Consumption,
    [string]$HashMembership = 'MASTER_INTERNAL_CONFIG_HASH_NOT_RUN_RECORDED'
) {
    $provenance = [Collections.Generic.List[object]]::new()
    $provenance.Add((Source-Provenance $InitializerLogical $InitializerMarker))
    if ($ConsumerLogical -ne '') { $provenance.Add((Source-Provenance $ConsumerLogical $ConsumerMarker)) }
    $fields.Add([ordered]@{
        field_id = $FieldId
        layer = $Layer
        value_type = $Type
        value_summary = [ordered]@{ mode = 'CONSTANT'; value = $Value; distinct_count = 1; distinct_values = @($Value) }
        evidence_class = 'RECOVERED_FROM_COLLECTION_ENVIRONMENT'
        value_origin = 'COMPILED_DEFAULT_OR_COLLECTOR_OVERRIDE'
        run_binding = 'NOT_RUN_BOUND_NO_EX5_OR_EXPANDED_MASTER_CONFIG_IN_RUN_RECEIPT'
        consumption_status = $Consumption
        configuration_hash_membership = $HashMembership
        provenance = @($provenance)
    })
}

# Strategy Tester harness fields.
Add-CampaignField 'tester.indicator' 'TESTER_HARNESS' 'Tester' 'Indicator' 'string' 'HARNESS_CONSUMED' 'NOT_IN_RG3_CONFIG_HASH'
Add-CampaignField 'tester.symbol' 'TESTER_HARNESS' 'Tester' 'Symbol' 'string' 'HARNESS_CONSUMED' 'RUN_KEY_MATERIAL'
Add-CampaignField 'tester.period' 'TESTER_HARNESS' 'Tester' 'Period' 'string' 'SEMANTICALLY_CONSUMED' 'RUN_KEY_MATERIAL' 'MQLInfoInteger(MQL_TESTER)'
Add-CampaignField 'tester.model' 'TESTER_HARNESS' 'Tester' 'Model' 'int' 'HARNESS_CONSUMED' 'NOT_IN_RG3_CONFIG_HASH'
Add-CampaignField 'tester.from_date' 'TESTER_HARNESS' 'Tester' 'FromDate' 'string' 'HARNESS_CONSUMED' 'NOT_IN_RG3_CONFIG_HASH'
Add-CampaignField 'tester.to_date' 'TESTER_HARNESS' 'Tester' 'ToDate' 'string' 'HARNESS_CONSUMED' 'NOT_IN_RG3_CONFIG_HASH'
Add-CampaignField 'tester.visual' 'TESTER_HARNESS' 'Tester' 'Visual' 'bool' 'OPERATIONALLY_CONSUMED' 'NOT_IN_RG3_CONFIG_HASH'
Add-CampaignField 'tester.shutdown_terminal' 'TESTER_HARNESS' 'Tester' 'ShutdownTerminal' 'bool' 'OPERATIONALLY_CONSUMED' 'NOT_IN_RG3_CONFIG_HASH'

# Explicit collector inputs.
$campaign = @(
    @('collector.canonical_instrument','RUN_IDENTITY','InpCanonicalInstrument','string','IDENTITY_AND_CONTEXT_CONSUMED','RUN_KEY_MATERIAL'),
    @('collector.data_source_id','RUN_IDENTITY','InpDataSourceId','string','IDENTITY_AND_CONTEXT_CONSUMED','RUN_KEY_SOURCE_MATERIAL'),
    @('collector.data_fingerprint','RUN_IDENTITY','InpDataFingerprint','string','IDENTITY_AND_CONTEXT_CONSUMED','RUN_KEY_SOURCE_MATERIAL'),
    @('collector.window_start','RUN_IDENTITY','InpWindowStart','int','IDENTITY_AND_SEMANTICALLY_CONSUMED','RUN_KEY_MATERIAL'),
    @('collector.window_end','RUN_IDENTITY','InpWindowEnd','int','IDENTITY_AND_SEMANTICALLY_CONSUMED','RUN_KEY_MATERIAL'),
    @('collector.enable_raw_logging','RAW_LOGGING','InpEnableRawLogging','bool','OPERATIONALLY_CONSUMED','NOT_IN_RG3_CONFIG_HASH'),
    @('collector.flush_every','RAW_LOGGING','InpFlushEvery','int','OPERATIONALLY_CONSUMED','NOT_IN_RG3_CONFIG_HASH'),
    @('compression.range_len','COMPRESSION','InpRangeLen','int','SEMANTICALLY_CONSUMED','RG3_CONFIG_HASH'),
    @('compression.range_mult','COMPRESSION','InpRangeMult','double','SEMANTICALLY_CONSUMED','RG3_CONFIG_HASH'),
    @('compression.atr_len','COMPRESSION','InpAtrLen','int','SEMANTICALLY_CONSUMED','RG3_CONFIG_HASH'),
    @('compression.confirm_bars','COMPRESSION','InpConfirmBars','int','SEMANTICALLY_CONSUMED','RG3_CONFIG_HASH'),
    @('compression.max_active_bars','COMPRESSION','InpMaxActiveBars','int','SEMANTICALLY_CONSUMED','RG3_CONFIG_HASH'),
    @('compression.frontier_buffer_atr','COMPRESSION','InpFrontierBufferAtr','double','SEMANTICALLY_CONSUMED','RG3_CONFIG_HASH'),
    @('compression.use_frontier_body_filter','COMPRESSION','InpUseFrontierBodyFilter','bool','SEMANTICALLY_CONSUMED','RG3_CONFIG_HASH'),
    @('compression.frontier_body_atr','COMPRESSION','InpFrontierBodyAtr','double','SEMANTICALLY_CONSUMED','RG3_CONFIG_HASH'),
    @('compression.observe_reentry','COMPRESSION','InpObserveReentry','bool','SEMANTICALLY_CONSUMED','RG3_CONFIG_HASH'),
    @('compression.reentry_window','COMPRESSION','InpReentryWindow','int','SEMANTICALLY_CONSUMED','RG3_CONFIG_HASH'),
    @('compression.reclaim_needs_midline','COMPRESSION','InpReclaimNeedsMidline','bool','SEMANTICALLY_CONSUMED','RG3_CONFIG_HASH'),
    @('compression.rearm_reset_bars','COMPRESSION','InpRearmResetBars','int','SEMANTICALLY_CONSUMED','RG3_CONFIG_HASH'),
    @('compression.require_fresh_lineage_window','COMPRESSION','InpRequireFreshLineageWindow','bool','DECLARED_HASHED_NOT_OPERATIONAL','RG3_CONFIG_HASH'),
    @('expansion.max_observation_bars','EXPANSION','InpMaxObservationBars','int','SEMANTICALLY_CONSUMED','RG3_CONFIG_HASH')
)
foreach ($item in $campaign) {
    Add-CampaignField $item[0] $item[1] 'TesterInputs' $item[2] $item[3] $item[4] $item[5] $item[2]
}

# Values directly recorded by every admitted run.
Add-RunField 'run.canonical_instrument' 'canonical_instrument' 'string' 'IDENTITY_CONSUMED' 'RUN_KEY_MATERIAL'
Add-RunField 'run.broker_symbol' 'broker_symbol' 'string' 'IDENTITY_CONSUMED' 'RUN_KEY_MATERIAL'
Add-RunField 'run.timeframe' 'timeframe' 'int' 'IDENTITY_AND_SEMANTICALLY_CONSUMED' 'RUN_KEY_MATERIAL'
Add-RunField 'run.window_start' 'window_start' 'int' 'IDENTITY_AND_SEMANTICALLY_CONSUMED' 'RUN_KEY_MATERIAL'
Add-RunField 'run.window_end' 'window_end' 'int' 'IDENTITY_AND_SEMANTICALLY_CONSUMED' 'RUN_KEY_MATERIAL'
Add-RunField 'run.configuration_hash' 'configuration_hash' 'string' 'IDENTITY_CONSUMED' 'RUN_KEY_MATERIAL'
Add-RunField 'run.data_source_id' 'data_source_id' 'string' 'IDENTITY_CONSUMED' 'RUN_KEY_SOURCE_MATERIAL'
Add-RunField 'run.data_fingerprint' 'data_fingerprint' 'string' 'IDENTITY_CONSUMED' 'RUN_KEY_SOURCE_MATERIAL'

$controller = 'Include/MasterStructure/MasterController.mqh'
$masterTypes = 'Include/MasterStructure/MasterTypes.mqh'
$auctionTypes = 'Include/MasterStructure/MasterAuctionTypes.mqh'

# Master direct configuration. The collector fixes master_timeframe from _Period
# and overrides the identity/output fields; the remaining values are defaults.
$masterDirect = @(
    @('master.master_timeframe','MASTER_CLOCK','enum','PERIOD_M5','config.master_timeframe=(ENUM_TIMEFRAMES)_Period','m_config.master_timeframe','SEMANTICALLY_CONSUMED'),
    @('master.volkitt_timeframe','MASTER_CLOCK','enum','PERIOD_H1','cfg.volkitt_timeframe = PERIOD_H1','m_config.volkitt_timeframe','SEMANTICALLY_CONSUMED'),
    @('master.profile_timeframe','MASTER_CLOCK','enum','PERIOD_H1','cfg.profile_timeframe = PERIOD_H1','m_config.profile_timeframe','SEMANTICALLY_CONSUMED'),
    @('master.day_swings_timeframe','MASTER_CLOCK','enum','PERIOD_M5','cfg.day_swings_timeframe = PERIOD_M5','m_config.day_swings_timeframe','SEMANTICALLY_CONSUMED'),
    @('master.wayne_timeframe','MASTER_CLOCK','enum','PERIOD_D1','cfg.wayne_timeframe = PERIOD_D1','m_config.wayne_timeframe','SEMANTICALLY_CONSUMED'),
    @('master.atr_period','MASTER_CORE','int',100,'cfg.atr_period = 100','m_config.atr_period','SEMANTICALLY_CONSUMED'),
    @('master.use_volkitt','MASTER_CORE','bool',$true,'cfg.use_volkitt = true','m_config.use_volkitt','SEMANTICALLY_CONSUMED'),
    @('master.use_day_swings','MASTER_CORE','bool',$true,'cfg.use_day_swings = true','m_config.use_day_swings','SEMANTICALLY_CONSUMED'),
    @('master.use_wayne','MASTER_CORE','bool',$true,'cfg.use_wayne = true','m_config.use_wayne','SEMANTICALLY_CONSUMED'),
    @('master.volkitt_instance','MASTER_CORE','int',1,'cfg.volkitt_instance = 1','m_config.volkitt_instance','PROVENANCE_CONSUMED'),
    @('master.day_swings_instance','MASTER_CORE','int',1,'cfg.day_swings_instance = 1','m_config.day_swings_instance','PROVENANCE_CONSUMED'),
    @('master.wayne_instance','MASTER_CORE','int',1,'cfg.wayne_instance = 1','m_config.wayne_instance','PROVENANCE_CONSUMED')
)
foreach ($item in $masterDirect) {
    $initLogical = if ($item[4].StartsWith('config.')) { 'Indicators/MarketObjectResearch/MarketObjectCollector.mq5' } else { $controller }
    Add-SourceField $item[0] $item[1] $item[2] $item[3] $initLogical $item[4] $controller $item[5] $item[6]
}

$volkittDefaults = @(
    @('lookback','int',200),@('clusters','int',5),@('iterations','int',50),@('rows_per_cluster','int',20),
    @('identity_reset_bins','double',2.50),@('volume_type','enum','KVP_VOL_TICK'),@('atr_period','int',100),
    @('cog_bins','int',180),@('velocity_bars','int',5),@('motion_threshold_bins','double',0.15),
    @('mass_motion_pct','double',0.05),@('width_motion_pct','double',0.05),@('heavy_on_new_bar_only','bool',$false),
    @('live_heavy_refresh_ms','int',750),@('enable_sentinels','bool',$true),
    @('sentinel_trigger_mode','enum','KVP_SENTINEL_TRIGGER_HYBRID'),@('sentinel_reset_mode','enum','KVP_SENTINEL_RESET_BROKER_DAY'),
    @('sentinel_arm_distance_atr','double',0.20),@('sentinel_arm_gap_fraction','double',0.25),
    @('sentinel_min_stretch_atr','double',1.00),@('sentinel_reentry_buffer_atr','double',0.25),
    @('sentinel_reentry_bars','int',3),@('enable_profile','bool',$true),@('profile_session','enum','KVP_PROFILE_DAILY'),
    @('profiles_to_keep','int',2),@('value_area_percent','int',70),@('point_multiplier','int',0),
    @('max_profile_bins','int',12000),@('intraday_start_minute','int',0),@('intraday_end_minute','int',1440),
    @('enable_single_prints','bool',$false),@('prominent_poc_percent','double',101.0)
)
foreach ($item in $volkittDefaults) {
    Add-SourceField "master.volkitt.$($item[0])" 'VOLKITT' $item[1] $item[2] 'Include/01Marketmain.mqh' "cfg.$($item[0])" 'Include/01Marketmain.mqh' "m_cfg.$($item[0])" 'SEMANTICALLY_CONSUMED'
}

foreach ($item in @(@('days_to_keep','int',5),@('confirm_reject_break_on_close','bool',$true))) {
    Add-SourceField "master.day_swings.$($item[0])" 'DAY_SWINGS' $item[1] $item[2] 'Include/01dayswings.mqh' "cfg.$($item[0])" 'Include/01dayswings.mqh' "m_cfg.$($item[0])" 'SEMANTICALLY_CONSUMED'
}
foreach ($item in @(@('periods_to_keep','int',20),@('include_standard_pivots','bool',$true),@('include_m_pivots','bool',$true),@('include_zones','bool',$true))) {
    Add-SourceField "master.wayne.$($item[0])" 'WAYNE' $item[1] $item[2] 'Include/01wayne.mqh' "cfg.$($item[0])" 'Include/01wayne.mqh' "m_cfg.$($item[0])" 'SEMANTICALLY_CONSUMED'
}

$allFamilyMask = 2046
Add-SourceField 'master.compatibility.family_masks' 'COMPATIBILITY' 'u64[16]' (@($allFamilyMask) * 16) $masterTypes 'cfg.family_masks[i] = all_families' $masterTypes 'cfg.family_masks' 'SEMANTICALLY_CONSUMED'
foreach ($item in @(@('require_role_compatibility','bool',$true),@('include_broken','bool',$false),@('include_developing','bool',$true))) {
    Add-SourceField "master.compatibility.$($item[0])" 'COMPATIBILITY' $item[1] $item[2] $masterTypes "cfg.$($item[0])" $masterTypes "cfg.$($item[0])" 'SEMANTICALLY_CONSUMED'
}

foreach ($item in @(
    @('epsilon_atr','double',0.12),@('min_samples','int',2),@('max_levels','int',1024),
    @('max_nodes','int',256),@('use_interval_distance','bool',$true),@('center_role_tolerance_atr','double',0.15)
)) {
    Add-SourceField "master.clustering.$($item[0])" 'CLUSTERING' $item[1] $item[2] $masterTypes "cfg.$($item[0])" $controller "m_config.clustering.$($item[0])" 'SEMANTICALLY_CONSUMED'
}
foreach ($item in @(@('match_distance_atr','double',0.20),@('retire_after_rebuilds','int',3))) {
    Add-SourceField "master.lifecycle.$($item[0])" 'NODE_LIFECYCLE' $item[1] $item[2] $masterTypes "cfg.$($item[0])" $controller "m_config.lifecycle.$($item[0])" 'SEMANTICALLY_CONSUMED'
}
foreach ($item in @(
    @('enabled','bool',$true),@('approach_radius_atr','double',0.50),@('rejection_min_excursion_atr','double',0.20),
    @('break_buffer_atr','double',0.00),@('acceptance_bars','int',2),@('acceptance_min_distance_atr','double',0.00),
    @('reclaim_tolerance_atr','double',0.05),@('departure_distance_atr','double',0.25),
    @('max_attempt_bars','int',24),@('episode_gap_bars','int',12)
)) {
    Add-SourceField "master.auction.$($item[0])" 'AUCTION_GRAMMAR' $item[1] $item[2] $auctionTypes "cfg.$($item[0])" $controller "m_config.auction.$($item[0])" 'SEMANTICALLY_CONSUMED'
}

# Master operational and identity overrides selected by the collector.
$overrides = @(
    @('master.enable_logging','MASTER_RUNTIME','bool',$false,'config.enable_logging=false','m_config.enable_logging','OPERATIONALLY_CONSUMED','NOT_IN_RG3_CONFIG_HASH'),
    @('master.enable_parity_oracle','MASTER_RUNTIME','bool',$false,'config.enable_parity_oracle=false','m_config.enable_parity_oracle','OPERATIONALLY_CONSUMED','NOT_IN_RG3_CONFIG_HASH'),
    @('master.instance_tag','MASTER_RUNTIME','string','RG3_OBJECT_SOURCE','config.instance_tag="RG3_OBJECT_SOURCE"','m_config.instance_tag','OPERATIONALLY_CONSUMED','NOT_IN_RG3_CONFIG_HASH'),
    @('master.log_flush_interval','MASTER_RUNTIME','int',10,'cfg.log_flush_interval = 10','m_config.log_flush_interval','NOT_ACTIVE_LOGGING_DISABLED','NOT_IN_RG3_CONFIG_HASH'),
    @('master.deterministic_terminal_time','MASTER_RUNTIME','per_run','collector.window_end','config.deterministic_terminal_time=InpWindowEnd','m_config.deterministic_terminal_time','SEMANTICALLY_CONSUMED','NOT_IN_RG3_CONFIG_HASH'),
    @('master.canonical_instrument','MASTER_IDENTITY','per_run','collector.canonical_instrument','config.canonical_instrument=InpCanonicalInstrument','m_config.canonical_instrument','IDENTITY_CONSUMED','NOT_IN_RG3_CONFIG_HASH'),
    @('master.data_source_id','MASTER_IDENTITY','string','BROKER_MT5','config.data_source_id=InpDataSourceId','m_config.data_source_id','IDENTITY_CONSUMED','NOT_IN_RG3_CONFIG_HASH'),
    @('master.data_fingerprint','MASTER_IDENTITY','per_run','collector.data_fingerprint','config.data_fingerprint=InpDataFingerprint','m_config.data_fingerprint','IDENTITY_CONSUMED','NOT_IN_RG3_CONFIG_HASH'),
    @('master.research_window_start','MASTER_IDENTITY','per_run','collector.window_start','config.research_window_start=InpWindowStart','m_config.research_window_start','IDENTITY_AND_SEMANTICALLY_CONSUMED','NOT_IN_RG3_CONFIG_HASH'),
    @('master.research_window_end','MASTER_IDENTITY','per_run','collector.window_end','config.research_window_end=InpWindowEnd','m_config.research_window_end','IDENTITY_AND_SEMANTICALLY_CONSUMED','NOT_IN_RG3_CONFIG_HASH')
)
foreach ($item in $overrides) {
    $initLogical = if ($item[4].StartsWith('cfg.')) { $controller } else { 'Indicators/MarketObjectResearch/MarketObjectCollector.mq5' }
    Add-SourceField $item[0] $item[1] $item[2] $item[3] $initLogical $item[4] $controller $item[5] $item[6] $item[7]
}
$runDerivedMaster = @(
    'master.master_timeframe','master.deterministic_terminal_time','master.canonical_instrument',
    'master.data_source_id','master.data_fingerprint','master.research_window_start','master.research_window_end'
)
foreach ($field in $fields) {
    if ($field.field_id -in $runDerivedMaster) {
        $field.evidence_class = 'RUN_RECORDED_AND_RECOVERED_SOURCE'
        $field.run_binding = 'VALUE_DERIVED_FROM_RUN_RECORDED_COLLECTOR_FIELD'
    }
}

# Version and schema identities. MOR values are cross-recorded in every run;
# Master values are only recoverable from source and folded into the RG3 hash.
$morTypes = 'Include/MarketObjectResearch/MarketObjectTypes.mqh'
$morVersions = @(
    @('version.rg3_research_generation','MOR_RESEARCH_GENERATION','int',3,'research_generation'),
    @('version.source_auction_generation','MOR_SOURCE_AUCTION_GENERATION','int',2,'source_auction_generation'),
    @('version.raw_schema','MOR_RAW_SCHEMA_VERSION','int',1,'raw_schema_version'),
    @('version.compression_grammar','MOR_COMPRESSION_GRAMMAR_VERSION','int',2,'compression_grammar_version'),
    @('version.expansion_grammar','MOR_EXPANSION_GRAMMAR_VERSION','int',1,'expansion_grammar_version'),
    @('version.rg3_contract','MOR_CONTRACT_ID','string','NORTHSTAR_RG3_RAW_MARKET_OBJECTS_V1','contract')
)
foreach ($item in $morVersions) {
    $recordedValues = @($runRows | ForEach-Object { [string]($_.($item[4])) } | Sort-Object -Unique)
    if ($recordedValues.Count -ne 1 -or $recordedValues[0] -ne [string]$item[3]) {
        throw "run-recorded MOR identity disagrees with source: $($item[1])"
    }
    Add-SourceField $item[0] 'VERSION_IDENTITY' $item[2] $item[3] $morTypes "#define $($item[1])" 'Include/MarketObjectResearch/MarketObjectLogger.mqh' $item[1] 'SERIALIZED_IDENTITY' 'RG3_CONFIG_HASH_OR_RUN_LEDGER'
    $fields[$fields.Count - 1].evidence_class = 'RUN_RECORDED_AND_RECOVERED_SOURCE'
    $fields[$fields.Count - 1].run_binding = 'VALUE_SERIALIZED_IN_ALL_42_RUN_LEDGERS'
    $fields[$fields.Count - 1].provenance += @([ordered]@{
        kind = 'SEALED_RUN_LEDGER_SET'
        locator = "measurement_runs.tsv::END.$($item[4])"
        admitted_run_count = 42
        run_ledger_set_sha256 = $runSetSha
    })
}
$masterVersions = @(
    @('version.master_research_generation','MST_RESEARCH_GENERATION','int',2),
    @('version.master_controller','MST_CONTROLLER_VERSION','int',2),
    @('version.master_topology','MST_TOPOLOGY_VERSION','int',2),
    @('version.master_auction_grammar','MST_AUCTION_GRAMMAR_VERSION','int',1),
    @('version.master_feature_schema','MST_FEATURE_SCHEMA_VERSION','int',1),
    @('version.master_dataset_schema','MST_DATASET_SCHEMA_VERSION','int',7),
    @('version.master_producer_bundle','MST_PRODUCER_BUNDLE_VERSION','int',1),
    @('version.master_code_build','MST_CODE_BUILD_ID','string','MASTER_STRUCTURE_RG2_REPLAY_WINDOW_GATE_V1'),
    @('version.master_research_preset','MST_RESEARCH_PRESET_ID','string','RG2_M5_SIX_INDEX_V1')
)
foreach ($item in $masterVersions) {
    if ($item[1] -eq 'MST_RESEARCH_PRESET_ID') {
        Add-SourceField $item[0] 'VERSION_IDENTITY' $item[2] $item[3] $masterTypes "#define $($item[1])" $controller $item[1] 'HASHED_IDENTITY' 'MASTER_INTERNAL_CONFIG_HASH_NOT_RUN_RECORDED'
    }
    else {
        Add-SourceField $item[0] 'VERSION_IDENTITY' $item[2] $item[3] $masterTypes "#define $($item[1])" 'Indicators/MarketObjectResearch/MarketObjectCollector.mq5' $item[1] 'HASHED_IDENTITY' 'RG3_CONFIG_HASH'
    }
}

$orderedFields = @($fields | Sort-Object field_id)
$idSet = [Collections.Generic.HashSet[string]]::new([StringComparer]::Ordinal)
foreach ($field in $orderedFields) {
    if (!$idSet.Add($field.field_id)) { throw "duplicate Theta0 field: $($field.field_id)" }
}

$core = [ordered]@{
    contract = 'NORTHSTAR_GATE16_6_OSV1_THETA0_FIELD_CENSUS_V1'
    status = 'PASS_WITH_EXPLICIT_RUN_BINDING_LIMITS'
    observer_instance = 'THETA0_RG3_COMPOSITE_M5_H1_H1_M5_D1'
    admitted_run_count = 42
    configuration_hash = [ordered]@{
        distinct_count = @($runRows.configuration_hash | Sort-Object -Unique).Count
        value = @($runRows.configuration_hash | Sort-Object -Unique)[0]
        scope = 'RG3 ObjectConfigText only; does not decode or bind the expanded Master defaults'
    }
    config_set_sha256 = $configSetSha
    run_ledger_set_sha256 = $runSetSha
    source_closure_sha256 = Sha256 $closurePath
    source_closure_logical_sha256 = $closure.logical_closure_sha256
    field_count = $orderedFields.Count
    fields = $orderedFields
    invariants = [ordered]@{
        all_42_runs_resolved = $true
        all_42_configs_resolved = $true
        one_recorded_configuration_hash = (@($runRows.configuration_hash | Sort-Object -Unique).Count -eq 1)
        source_defaults_called_run_recorded = $false
        ex5_hash_available = $false
        fresh_lineage_flag_operational = $false
        confirmation_windows_opened = $false
    }
    explicit_unavailable = @(
        'exact EX5 binary used by each admitted run',
        'run-bound expanded Master ConfigText',
        'run-bound terminal build identity',
        'alternative composite clock vectors'
    )
}
$core.logical_theta0_sha256 = Canonical-Hash $core

$output = [IO.Path]::GetFullPath($OutputDirectory)
New-Item -ItemType Directory -Force -Path $output | Out-Null
$jsonPath = Join-Path $output 'osv1_theta0_fields.json'
$fieldTsvPath = Join-Path $output 'osv1_theta0_fields.tsv'
$runTsvPath = Join-Path $output 'osv1_theta0_runs.tsv'
$receiptPath = Join-Path $output 'osv1_theta0_receipt.json'
[IO.File]::WriteAllText($jsonPath, ($core | ConvertTo-Json -Depth 100) + "`n", [Text.UTF8Encoding]::new($false))

$writer = [IO.StreamWriter]::new($fieldTsvPath, $false, [Text.UTF8Encoding]::new($false), 262144)
try {
    $writer.WriteLine("field_id`tlayer`tvalue_type`tvalue_mode`tvalue_json`tevidence_class`trun_binding`tconsumption_status`tconfig_hash_membership`tprovenance_json")
    foreach ($field in $orderedFields) {
        $values = @(
            $field.field_id,$field.layer,$field.value_type,$field.value_summary.mode,
            ($field.value_summary | ConvertTo-Json -Depth 20 -Compress),$field.evidence_class,$field.run_binding,
            $field.consumption_status,$field.configuration_hash_membership,
            ($field.provenance | ConvertTo-Json -Depth 20 -Compress)
        )
        if (($values -join '') -match "[`t`r`n]") { throw "TSV control character in $($field.field_id)" }
        $writer.WriteLine($values -join "`t")
    }
}
finally { $writer.Dispose() }

$writer = [IO.StreamWriter]::new($runTsvPath, $false, [Text.UTF8Encoding]::new($false), 262144)
try {
    $writer.WriteLine("run_key`tcanonical_instrument`tbroker_symbol`ttimeframe`twindow_start`twindow_end`tconfiguration_hash`tdata_source_id`tdata_fingerprint`tconfig_path`tconfig_sha256`tmeasurement_runs_path`tmeasurement_runs_sha256`trun_seal_path`trun_seal_sha256")
    foreach ($run in $runRows) {
        $values = @($run.run_key,$run.canonical_instrument,$run.broker_symbol,$run.timeframe,$run.window_start,$run.window_end,$run.configuration_hash,$run.data_source_id,$run.data_fingerprint,$run.config_path,$run.config_sha256,$run.measurement_runs_path,$run.measurement_runs_sha256,$run.run_seal_path,$run.run_seal_sha256)
        $writer.WriteLine($values -join "`t")
    }
}
finally { $writer.Dispose() }

$receipt = [ordered]@{
    contract = 'NORTHSTAR_GATE16_6_OSV1_THETA0_RECEIPT_V1'
    status = 'PASS'
    logical_theta0_sha256 = $core.logical_theta0_sha256
    fields_sha256 = Sha256 $jsonPath
    fields_tsv_sha256 = Sha256 $fieldTsvPath
    runs_tsv_sha256 = Sha256 $runTsvPath
    field_count = $orderedFields.Count
    admitted_run_count = 42
    confirmation_windows_opened = $false
}
[IO.File]::WriteAllText($receiptPath, ($receipt | ConvertTo-Json -Depth 20) + "`n", [Text.UTF8Encoding]::new($false))
Write-Output "OSV1_THETA0 fields=$($orderedFields.Count) runs=42 logical_sha256=$($core.logical_theta0_sha256)"
