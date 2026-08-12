//+------------------------------------------------------------------+
//| MasterController.mqh                                             |
//| Owns producer cadence, normalization, clustering, nodes, logging. |
//+------------------------------------------------------------------+
#ifndef __KITT_MASTER_STRUCTURE_CONTROLLER_MQH__
#define __KITT_MASTER_STRUCTURE_CONTROLLER_MQH__

#include "MasterTypes.mqh"
#include "MasterAdapters.mqh"
#include "MasterClusterer.mqh"
#include "MasterNodeTracker.mqh"
#include "MasterLogger.mqh"
#include "MasterAuctionEngine.mqh"
#include "MasterAuctionLogger.mqh"
#include "MasterAuctionContext.mqh"
#include "MasterRegionalModel.mqh"

struct MST_ControllerConfig
{
   ENUM_TIMEFRAMES         master_timeframe;
   ENUM_TIMEFRAMES         volkitt_timeframe;
   ENUM_TIMEFRAMES         profile_timeframe;
   ENUM_TIMEFRAMES         day_swings_timeframe;
   ENUM_TIMEFRAMES         wayne_timeframe;
   int                     atr_period;

   bool                    use_volkitt;
   bool                    use_day_swings;
   bool                    use_wayne;
   int                     volkitt_instance;
   int                     day_swings_instance;
   int                     wayne_instance;

   KVP_Config              volkitt;
   DSW_Config              day_swings;
   WYN_Config              wayne;
   MST_CompatibilityConfig compatibility;
   MST_ClusterConfig       clustering;
   MST_NodeLifecycleConfig lifecycle;
   MST_AuctionGrammarConfig auction;

   bool                    enable_logging;
   string                  instance_tag;
   int                     log_flush_interval;
   datetime                deterministic_terminal_time;
};

void MST_DefaultControllerConfig(MST_ControllerConfig &cfg)
{
   ZeroMemory(cfg);
   cfg.master_timeframe = PERIOD_M5;
   cfg.volkitt_timeframe = PERIOD_H1;
   cfg.profile_timeframe = PERIOD_H1;
   cfg.day_swings_timeframe = PERIOD_M5;
   cfg.wayne_timeframe = PERIOD_D1;
   cfg.atr_period = 100;
   cfg.use_volkitt = true;
   cfg.use_day_swings = true;
   cfg.use_wayne = true;
   cfg.volkitt_instance = 1;
   cfg.day_swings_instance = 1;
   cfg.wayne_instance = 1;
   KVP_DefaultConfig(cfg.volkitt);
   DSW_DefaultConfig(cfg.day_swings);
   WYN_DefaultConfig(cfg.wayne);
   MST_DefaultCompatibility(cfg.compatibility);
   MST_DefaultClusterConfig(cfg.clustering);
   MST_DefaultNodeLifecycleConfig(cfg.lifecycle);
   MST_DefaultAuctionGrammarConfig(cfg.auction);
   cfg.enable_logging = false;
   cfg.instance_tag = "MASTER";
   cfg.log_flush_interval = 10;
   cfg.deterministic_terminal_time = 0;
}

class CMasterStructureController
{
private:
   string                            m_symbol;
   MST_ControllerConfig              m_config;
   bool                              m_initialized;
   int                               m_atr_handle;
   double                            m_point;

   CKittVolKittMarketProfileProducer m_volkitt;
   CKittDaySwingsProducer            m_day_swings;
   CWaynePivotProducer               m_wayne;
   CMstDensityClusterer              m_clusterer;
   CMstNodeTracker                   m_tracker;
   CMstResearchLogger                m_logger;
   CMstAuctionEngine                 m_auction;
   CMstAuctionLogger                 m_auction_logger;
   CMstAuctionContextBuilder         m_context_builder;
   CMstRegionalModel                 m_regional_model;

   KVP_Level                         m_kvp_levels[];
   DSW_Level                         m_day_levels[];
   WYN_Level                         m_wayne_levels[];
   MST_Level                         m_levels[];
   int                               m_labels[];
   MST_Node                          m_nodes[];
   MST_SourceRef                     m_provenance[];
   MST_NodeEvent                     m_events[];
   MST_AuctionEvent                  m_auction_events[];
   MST_AuctionAttempt                m_completed_attempts[];
   MST_AuctionEpisode                m_completed_episodes[];
   MST_AuctionContextReceipt         m_auction_context[];
   MST_TransitTrack                  m_completed_transits[];
   MST_AuctionFeatureSnapshot        m_feature_snapshot;
   MST_AuctionRunManifest            m_run_manifest;
   MST_ControllerReading             m_reading;
   ulong                             m_last_snapshot_hash;
   ulong                             m_last_input_hash;
   datetime                          m_last_market_time;
   datetime                          m_last_closed_time;
   double                            m_last_reference_price;
   double                            m_last_atr;
   bool                              m_finalized;

   ulong HashText(const string value) const
   {
      ulong hash = 1469598103934665603;
      for(int i = 0; i < StringLen(value); i++)
         hash = MST_HashMix(hash, (ulong)StringGetCharacter(value, i));
      return hash;
   }

   string ConfigText(void) const
   {
      string text = StringFormat(
         "master_tf=%d;volkitt_tf=%d;profile_tf=%d;days_tf=%d;wayne_tf=%d;atr=%d;"+
         "use_v=%d;use_d=%d;use_w=%d;v_inst=%d;d_inst=%d;w_inst=%d;"+
         "v_lookback=%d;v_clusters=%d;v_iterations=%d;v_rows=%d;v_atr=%d;"+
         "v_cog_bins=%d;v_velocity=%d;v_profile=%d;v_profiles=%d;v_va=%d;v_bins=%d;"+
         "d_keep=%d;d_close=%d;w_keep=%d;w_std=%d;w_m=%d;w_zones=%d;"+
         "epsilon=%.8f;min_samples=%d;max_levels=%d;max_nodes=%d;interval=%d;"+
         "center_tol=%.8f;match=%.8f;retire=%d;auction=%d;approach=%.8f;"+
         "reject=%.8f;break=%.8f;accept_bars=%d;accept_dist=%.8f;reclaim=%.8f;"+
         "departure=%.8f;attempt_bars=%d;episode_gap=%d;terminal_time=%I64d",
         (int)m_config.master_timeframe, (int)m_config.volkitt_timeframe,
         (int)m_config.profile_timeframe, (int)m_config.day_swings_timeframe,
         (int)m_config.wayne_timeframe, m_config.atr_period,
         (int)m_config.use_volkitt, (int)m_config.use_day_swings, (int)m_config.use_wayne,
         m_config.volkitt_instance, m_config.day_swings_instance, m_config.wayne_instance,
         m_config.volkitt.lookback, m_config.volkitt.clusters, m_config.volkitt.iterations,
         m_config.volkitt.rows_per_cluster, m_config.volkitt.atr_period,
         m_config.volkitt.cog_bins, m_config.volkitt.velocity_bars,
         (int)m_config.volkitt.enable_profile, m_config.volkitt.profiles_to_keep,
         m_config.volkitt.value_area_percent, m_config.volkitt.max_profile_bins,
         m_config.day_swings.days_to_keep, (int)m_config.day_swings.confirm_reject_break_on_close,
         m_config.wayne.periods_to_keep, (int)m_config.wayne.include_standard_pivots,
         (int)m_config.wayne.include_m_pivots, (int)m_config.wayne.include_zones,
         m_config.clustering.epsilon_atr, m_config.clustering.min_samples,
         m_config.clustering.max_levels, m_config.clustering.max_nodes,
         (int)m_config.clustering.use_interval_distance,
         m_config.clustering.center_role_tolerance_atr,
         m_config.lifecycle.match_distance_atr, m_config.lifecycle.retire_after_rebuilds,
         (int)m_config.auction.enabled, m_config.auction.approach_radius_atr,
         m_config.auction.rejection_min_excursion_atr, m_config.auction.break_buffer_atr,
         m_config.auction.acceptance_bars, m_config.auction.acceptance_min_distance_atr,
         m_config.auction.reclaim_tolerance_atr, m_config.auction.departure_distance_atr,
         m_config.auction.max_attempt_bars, m_config.auction.episode_gap_bars,
         (long)m_config.deterministic_terminal_time);
      text += ";regional=ACTIVE_NODE_MEDIAN_POPSIGMA_ROUND3_V1";
      return text;
   }

   double ResolveReferencePrice(void)
   {
      MqlTick tick;
      if(SymbolInfoTick(m_symbol, tick))
      {
         if(tick.last > 0.0) return tick.last;
         if(tick.bid > 0.0 && tick.ask > 0.0) return (tick.bid + tick.ask) * 0.5;
         if(tick.bid > 0.0) return tick.bid;
      }
      return iClose(m_symbol, m_config.master_timeframe, 0);
   }

   datetime ResolveMarketTime(void)
   {
      MqlTick tick;
      if(SymbolInfoTick(m_symbol, tick) && tick.time > 0)
         return tick.time;
      return TimeCurrent();
   }

   bool ReadClosedAtr(double &atr)
   {
      atr = 0.0;
      if(m_atr_handle == INVALID_HANDLE || BarsCalculated(m_atr_handle) < 2)
         return false;
      double values[1];
      if(CopyBuffer(m_atr_handle, 0, 1, 1, values) != 1)
         return false;
      atr = values[0];
      return MST_IsFinitePrice(atr) && atr > 0.0;
   }

   int CollectLevels(void)
   {
      int kvp_count = 0;
      int day_count = 0;
      int wayne_count = 0;
      if(m_config.use_volkitt)
         kvp_count = m_volkitt.ExportLevels(m_kvp_levels);
      else
         ArrayResize(m_kvp_levels, 0);
      if(m_config.use_day_swings)
         day_count = m_day_swings.ExportLevels(m_day_levels);
      else
         ArrayResize(m_day_levels, 0);
      if(m_config.use_wayne)
         wayne_count = m_wayne.ExportLevels(m_wayne_levels);
      else
         ArrayResize(m_wayne_levels, 0);

      int requested = MathMax(kvp_count, 0) + MathMax(day_count, 0) + MathMax(wayne_count, 0);
      if(ArrayResize(m_levels, requested, MathMax(m_config.clustering.max_levels, requested)) < requested)
         return -1;

      int written = 0;
      written += MST_AdaptKvpLevels(m_kvp_levels, kvp_count,
                                    m_config.volkitt_instance, m_levels, written);
      written += MST_AdaptDayLevels(m_day_levels, day_count,
                                    m_config.day_swings_instance, m_levels, written);
      written += MST_AdaptWayneLevels(m_wayne_levels, wayne_count,
                                      m_config.wayne_instance, m_levels, written);
      if(written != ArraySize(m_levels))
         ArrayResize(m_levels, written);
      return written;
   }

   int NormalizeAndFilter(const double reference_price, const double atr)
   {
      int source_count = ArraySize(m_levels);
      int write_index = 0;
      for(int i = 0; i < source_count; i++)
      {
         if(!MST_NormalizeLevel(m_levels[i], reference_price, atr))
            continue;
         if(!MST_LevelsCompatible(m_levels[i], m_levels[i], m_config.compatibility))
            continue;
         if(write_index != i)
            m_levels[write_index] = m_levels[i];
         write_index++;
      }
      if(write_index != source_count)
         ArrayResize(m_levels, write_index);
      MST_SortLevels(m_levels, write_index);
      return write_index;
   }

   ulong StructuralInputHash(const int requested_count, const double atr) const
   {
      int count = MathMin(requested_count, ArraySize(m_levels));
      double safe_point = MathMax(m_point, 0.00000001);
      double fine_quantum = safe_point * 0.000001;
      ulong hash = 1469598103934665603;
      hash = MST_HashMix(hash, (ulong)count);
      hash = MST_HashMix(hash, (ulong)MathRound(atr / fine_quantum));
      for(int i = 0; i < count; i++)
      {
         const MST_Level level = m_levels[i];
         hash = MST_HashMix(hash, level.source_key);
         hash = MST_HashMix(hash, (ulong)((int)level.producer));
         hash = MST_HashMix(hash, (ulong)level.producer_instance);
         hash = MST_HashMix(hash, level.local_id);
         hash = MST_HashMix(hash, (ulong)((int)level.family));
         hash = MST_HashMix(hash, (ulong)level.source_kind);
         hash = MST_HashMix(hash, (ulong)((int)level.role));
         hash = MST_HashMix(hash, (ulong)MathRound(level.lower / safe_point));
         hash = MST_HashMix(hash, (ulong)MathRound(level.price / safe_point));
         hash = MST_HashMix(hash, (ulong)MathRound(level.upper / safe_point));
         hash = MST_HashMix(hash, (ulong)level.created_at);
         hash = MST_HashMix(hash, (ulong)level.developing);
         hash = MST_HashMix(hash, (ulong)level.frozen_geometry);
         hash = MST_HashMix(hash, (ulong)((int)level.state));
         hash = MST_HashMix(hash, (ulong)MathRound(level.evidence_weight / fine_quantum));
      }
      return hash;
   }

   bool RefreshCachedNodeState(const double reference_price, const double atr)
   {
      bool changed = false;
      int node_count = ArraySize(m_nodes);
      for(int c = 0; c < node_count; c++)
      {
         if(!m_nodes[c].valid) continue;
         m_nodes[c].normalized_lower = (m_nodes[c].lower - reference_price) / atr;
         m_nodes[c].normalized_price = (m_nodes[c].price - reference_price) / atr;
         m_nodes[c].normalized_upper = (m_nodes[c].upper - reference_price) / atr;
         m_nodes[c].width_atr = m_nodes[c].width / atr;
         m_nodes[c].distance_atr = m_nodes[c].normalized_price;
         MST_NODE_ROLE previous_role = m_nodes[c].role;
         if(m_nodes[c].normalized_price < -m_config.clustering.center_role_tolerance_atr)
            m_nodes[c].role = MST_NODE_LOWER;
         else if(m_nodes[c].normalized_price > m_config.clustering.center_role_tolerance_atr)
            m_nodes[c].role = MST_NODE_UPPER;
         else
            m_nodes[c].role = MST_NODE_CENTER;
         if(previous_role != m_nodes[c].role) changed = true;
         m_nodes[c].newest_update_time = 0;
      }

      int level_count = MathMin(ArraySize(m_levels), ArraySize(m_labels));
      for(int i = 0; i < level_count; i++)
      {
         int cluster = m_labels[i];
         if(cluster < 0 || cluster >= node_count || !m_nodes[cluster].valid) continue;
         if(m_levels[i].updated_at > m_nodes[cluster].newest_update_time)
            m_nodes[cluster].newest_update_time = m_levels[i].updated_at;
      }
      return changed;
   }

   void CollectTrackerEvents(void)
   {
      int count = m_tracker.EventCount();
      ArrayResize(m_events, count);
      for(int i = 0; i < count; i++)
         m_tracker.GetEvent(i, m_events[i]);
      m_reading.lifecycle_event_count = count;
      m_reading.tracked_node_count = m_tracker.TrackCount();
   }

   void CollectAuctionOutputs(void)
   {
      int count = m_auction.EventCount();
      ArrayResize(m_auction_events, count);
      for(int i = 0; i < count; i++) m_auction.GetEvent(i, m_auction_events[i]);
      count = m_auction.CompletedAttemptCount();
      ArrayResize(m_completed_attempts, count);
      for(int i = 0; i < count; i++)
         m_auction.GetCompletedAttempt(i, m_completed_attempts[i]);
      count = m_auction.CompletedEpisodeCount();
      ArrayResize(m_completed_episodes, count);
      for(int i = 0; i < count; i++)
         m_auction.GetCompletedEpisode(i, m_completed_episodes[i]);
      count = m_auction.ContextCount();
      ArrayResize(m_auction_context, count);
      for(int i = 0; i < count; i++) m_auction.GetContext(i, m_auction_context[i]);
      count = m_auction.CompletedTransitCount();
      ArrayResize(m_completed_transits, count);
      for(int i = 0; i < count; i++)
         m_auction.GetCompletedTransit(i, m_completed_transits[i]);

      for(int i = 0; i < ArraySize(m_nodes); i++)
         m_nodes[i].interaction_state = (int)m_auction.StateForNode(m_nodes[i].node_id);
      MST_AuctionReading auction;
      m_auction.GetReading(auction);
      m_reading.auction_event_count = auction.event_count;
      m_reading.active_attempt_count = auction.active_attempts;
      m_reading.active_episode_count = auction.active_episodes;
      m_reading.active_transit_count = auction.active_transits;
      m_reading.completed_attempt_count = auction.completed_attempts;
      m_reading.completed_episode_count = auction.completed_episodes;
      m_reading.auction_event_sequence = auction.event_sequence;
      m_reading.auction_terminal_hash = auction.terminal_hash;
   }

   void ObserveAuctions(const double reference_price,
                        const double atr,
                        const datetime market_time)
   {
      datetime closed_time = iTime(m_symbol, m_config.master_timeframe, 1);
      double closed_price = iClose(m_symbol, m_config.master_timeframe, 1);
      m_context_builder.Build(m_symbol, m_config.master_timeframe,
                              m_levels, ArraySize(m_levels), m_clusterer.NoiseCount(),
                              m_nodes, ArraySize(m_nodes), reference_price, atr,
                              m_reading.regional,
                              m_feature_snapshot);
      m_auction.SetFeatureSnapshot(m_feature_snapshot);
      m_auction.Observe(m_nodes, ArraySize(m_nodes), m_provenance,
                        ArraySize(m_provenance), reference_price, atr,
                        market_time, closed_time, closed_price);
      m_tracker.SyncAttemptCounts(m_nodes, ArraySize(m_nodes));
      CollectAuctionOutputs();
      CollectTrackerEvents();
      m_last_market_time = market_time;
      m_last_closed_time = closed_time;
      m_last_reference_price = reference_price;
      m_last_atr = atr;
   }

   void RefreshRegionalState(const double reference_price,
                             const double atr,
                             const datetime closed_bar_time,
                             const bool rebuilt)
   {
      if(rebuilt)
         m_regional_model.RebuildBasis(m_nodes, ArraySize(m_nodes),
                                       m_levels, ArraySize(m_levels),
                                       m_reading.snapshot_hash,
                                       m_reading.generation, m_point);
      m_regional_model.Observe(m_symbol, m_config.master_timeframe,
                               reference_price, atr, closed_bar_time, rebuilt);
      m_regional_model.ApplyToNodes(m_nodes, ArraySize(m_nodes),
                                    m_provenance, ArraySize(m_provenance), atr);
      m_regional_model.GetSnapshot(m_reading.regional);
   }

   bool WriteAuctionCycle(void)
   {
      return m_auction_logger.WriteCycle(m_auction_events, ArraySize(m_auction_events),
                                         m_completed_attempts, ArraySize(m_completed_attempts),
                                         m_completed_episodes, ArraySize(m_completed_episodes),
                                         m_auction_context, ArraySize(m_auction_context),
                                         m_completed_transits, ArraySize(m_completed_transits));
   }

   ulong FullSnapshotHash(const int level_count, const ulong node_hash) const
   {
      ulong hash = MST_HashMix(node_hash, (ulong)level_count);
      double safe_point = MathMax(m_point, 0.00000001);
      for(int i = 0; i < level_count; i++)
      {
         long lower_ticks = (long)MathRound(m_levels[i].lower / safe_point);
         long price_ticks = (long)MathRound(m_levels[i].price / safe_point);
         long upper_ticks = (long)MathRound(m_levels[i].upper / safe_point);
         hash = MST_HashMix(hash, m_levels[i].source_key);
         hash = MST_HashMix(hash, (ulong)lower_ticks);
         hash = MST_HashMix(hash, (ulong)price_ticks);
         hash = MST_HashMix(hash, (ulong)upper_ticks);
         hash = MST_HashMix(hash, (ulong)((int)m_levels[i].state));
      }
      return hash;
   }

public:
   CMasterStructureController(void)
   {
      m_symbol = "";
      m_initialized = false;
      m_atr_handle = INVALID_HANDLE;
      m_point = 0.0;
      m_last_snapshot_hash = 0;
      m_last_input_hash = 0;
      m_last_market_time = 0;
      m_last_closed_time = 0;
      m_last_reference_price = 0.0;
      m_last_atr = 0.0;
      m_finalized = true;
      ZeroMemory(m_reading);
   }

   bool Init(const string symbol, const MST_ControllerConfig &config)
   {
      Deinit();
      m_symbol = symbol;
      if(StringLen(m_symbol) <= 0) m_symbol = _Symbol;
      m_config = config;
      if(m_config.master_timeframe == PERIOD_CURRENT)
         m_config.master_timeframe = (ENUM_TIMEFRAMES)_Period;
      if(m_config.atr_period < 2 || m_config.clustering.epsilon_atr < 0.0 ||
         m_config.clustering.min_samples < 1 ||
         m_config.clustering.max_levels < 1 || m_config.clustering.max_nodes < 1)
         return false;
      if(m_config.lifecycle.match_distance_atr < 0.0 ||
         m_config.lifecycle.retire_after_rebuilds < 1)
         return false;
      if(m_config.auction.approach_radius_atr < 0.0 ||
         m_config.auction.rejection_min_excursion_atr < 0.0 ||
         m_config.auction.break_buffer_atr < 0.0 ||
         m_config.auction.acceptance_bars < 1 ||
         m_config.auction.departure_distance_atr < 0.0 ||
         m_config.auction.max_attempt_bars < 1 ||
         m_config.auction.episode_gap_bars < 1)
         return false;

      m_point = SymbolInfoDouble(m_symbol, SYMBOL_POINT);
      if(m_point <= 0.0) return false;
      m_atr_handle = iATR(m_symbol, m_config.master_timeframe, m_config.atr_period);
      if(m_atr_handle == INVALID_HANDLE) return false;

      if(m_config.use_volkitt &&
         !m_volkitt.Init(m_symbol, m_config.volkitt_timeframe,
                         m_config.profile_timeframe, m_config.volkitt))
      {
         Deinit();
         return false;
      }
      if(m_config.use_day_swings &&
         !m_day_swings.Init(m_symbol, m_config.day_swings_timeframe,
                            m_config.day_swings))
      {
         Deinit();
         return false;
      }
      if(m_config.use_wayne &&
         !m_wayne.Init(m_symbol, m_config.wayne_timeframe, m_config.wayne))
      {
         Deinit();
         return false;
      }
      if(!m_logger.Init(m_config.enable_logging, m_symbol,
                        m_config.master_timeframe, m_config.instance_tag,
                        m_config.log_flush_interval))
      {
         Deinit();
         return false;
      }
      if(!m_auction_logger.Init(m_config.enable_logging, m_symbol,
                                m_config.master_timeframe, m_config.instance_tag,
                                m_config.log_flush_interval))
      {
         Deinit();
         return false;
      }

      ArrayResize(m_levels, 0);
      ArrayResize(m_labels, 0);
      ArrayResize(m_nodes, 0);
      ArrayResize(m_provenance, 0);
      ArrayResize(m_events, 0);
      ArrayResize(m_auction_events, 0);
      ArrayResize(m_completed_attempts, 0);
      ArrayResize(m_completed_episodes, 0);
      ArrayResize(m_auction_context, 0);
      ArrayResize(m_completed_transits, 0);
      m_tracker.Init(m_config.lifecycle);
      m_auction.Init(m_config.auction);
      m_context_builder.Reset();
      m_regional_model.Reset();
      ZeroMemory(m_feature_snapshot);
      ZeroMemory(m_run_manifest);
      ZeroMemory(m_reading);
      m_reading.symbol = m_symbol;
      m_reading.timeframe = m_config.master_timeframe;
      m_last_snapshot_hash = 0;
      m_last_input_hash = 0;
      m_last_market_time = ResolveMarketTime();
      m_last_closed_time = iTime(m_symbol, m_config.master_timeframe, 1);
      m_last_reference_price = ResolveReferencePrice();
      m_last_atr = 0.0;
      m_finalized = false;
      m_run_manifest.build_id = MST_BUILD_ID;
      m_run_manifest.session_id = m_auction_logger.SessionId();
      m_run_manifest.symbol = m_symbol;
      m_run_manifest.account_server = AccountInfoString(ACCOUNT_SERVER);
      m_run_manifest.account_company = AccountInfoString(ACCOUNT_COMPANY);
      m_run_manifest.config_text = ConfigText();
      m_run_manifest.config_hash = HashText(m_run_manifest.config_text);
      m_run_manifest.timeframe = m_config.master_timeframe;
      m_run_manifest.digits = (int)SymbolInfoInteger(m_symbol, SYMBOL_DIGITS);
      m_run_manifest.point = m_point;
      m_run_manifest.tick_size = SymbolInfoDouble(m_symbol, SYMBOL_TRADE_TICK_SIZE);
      m_run_manifest.tester = (bool)MQLInfoInteger(MQL_TESTER);
      m_run_manifest.visual_mode = (bool)MQLInfoInteger(MQL_VISUAL_MODE);
      m_run_manifest.started_at = m_last_market_time;
      if(!m_auction_logger.WriteRun("START", m_run_manifest))
      {
         Deinit();
         return false;
      }
      m_initialized = true;
      return true;
   }

   bool Update(const bool force_heavy = false,
               const bool force_profile = false,
               const bool allow_cached_structure = false)
   {
      if(!m_initialized) return false;
      ulong start_us = GetMicrosecondCount();

      m_reading.volkitt_heavy_core = false;
      m_reading.volkitt_profile_rebuilt = false;
      m_reading.structure_rebuilt = false;
      m_reading.volkitt_microseconds = 0;
      m_reading.volkitt_core_microseconds = 0;
      m_reading.volkitt_profile_microseconds = 0;
      m_reading.day_swings_microseconds = 0;
      m_reading.wayne_microseconds = 0;
      m_reading.reference_atr_microseconds = 0;
      m_reading.collect_microseconds = 0;
      m_reading.normalize_microseconds = 0;
      m_reading.dbscan_microseconds = 0;
      m_reading.node_build_microseconds = 0;
      m_reading.lifecycle_microseconds = 0;
      m_reading.hash_microseconds = 0;
      m_reading.logger_microseconds = 0;
      m_reading.lifecycle_event_count = 0;
      m_reading.auction_event_count = 0;
      m_reading.completed_attempt_count = 0;
      m_reading.completed_episode_count = 0;

      bool any_ready = false;
      if(m_config.use_volkitt)
      {
         ulong stage_us = GetMicrosecondCount();
         any_ready = m_volkitt.Update(force_heavy, force_profile) || any_ready;
         m_reading.volkitt_microseconds = GetMicrosecondCount() - stage_us;
         KVP_PerformanceReading volkitt_perf;
         m_volkitt.GetPerformance(volkitt_perf);
         m_reading.volkitt_heavy_core = volkitt_perf.heavy_core_due;
         m_reading.volkitt_profile_rebuilt = volkitt_perf.profile_rebuild_due;
         m_reading.volkitt_core_microseconds = volkitt_perf.core_microseconds;
         m_reading.volkitt_profile_microseconds = volkitt_perf.profile_microseconds;
      }
      if(m_config.use_day_swings)
      {
         ulong stage_us = GetMicrosecondCount();
         any_ready = m_day_swings.Update() || any_ready;
         m_reading.day_swings_microseconds = GetMicrosecondCount() - stage_us;
      }
      if(m_config.use_wayne)
      {
         ulong stage_us = GetMicrosecondCount();
         any_ready = m_wayne.Update() || any_ready;
         m_reading.wayne_microseconds = GetMicrosecondCount() - stage_us;
      }
      if(!any_ready) return false;

      ulong stage_us = GetMicrosecondCount();
      double reference_price = ResolveReferencePrice();
      double atr = 0.0;
      if(!MST_IsFinitePrice(reference_price) || reference_price <= 0.0 ||
          !ReadClosedAtr(atr))
         return false;
      m_reading.reference_atr_microseconds = GetMicrosecondCount() - stage_us;

      stage_us = GetMicrosecondCount();
      int collected_count = CollectLevels();
      if(collected_count < 0) return false;
      m_reading.collect_microseconds = GetMicrosecondCount() - stage_us;
      stage_us = GetMicrosecondCount();
      ulong input_hash = StructuralInputHash(collected_count, atr);
      m_reading.hash_microseconds = GetMicrosecondCount() - stage_us;
      stage_us = GetMicrosecondCount();
      int level_count = NormalizeAndFilter(reference_price, atr);
      m_reading.normalize_microseconds = GetMicrosecondCount() - stage_us;
      int dropped = MathMax(level_count - m_config.clustering.max_levels, 0);

      bool rebuild_structure = !allow_cached_structure || force_heavy || force_profile ||
                               input_hash != m_last_input_hash || !m_reading.valid;
      if(!rebuild_structure)
      {
         bool role_changed = RefreshCachedNodeState(reference_price, atr);
         datetime market_time = ResolveMarketTime();
         m_tracker.BeginCycle();
         stage_us = GetMicrosecondCount();
         RefreshRegionalState(reference_price, atr,
                              iTime(m_symbol, m_config.master_timeframe, 1), false);
         ObserveAuctions(reference_price, atr, market_time);
         m_reading.lifecycle_microseconds = GetMicrosecondCount() - stage_us;
         if(role_changed || m_reading.lifecycle_event_count > 0)
            m_reading.render_generation++;
         m_reading.symbol = m_symbol;
         m_reading.timeframe = m_config.master_timeframe;
         m_reading.market_time = market_time;
         m_reading.calculation_bar_time = iTime(m_symbol, m_config.master_timeframe, 0);
         m_reading.reference_price = reference_price;
         m_reading.atr = atr;
         m_reading.raw_level_count = level_count;
         m_reading.node_count = ArraySize(m_nodes);
         m_reading.dropped_level_count = dropped;
         m_reading.update_microseconds = GetMicrosecondCount() - start_us;
         stage_us = GetMicrosecondCount();
         if(!m_logger.WriteEvents(m_reading, m_events, ArraySize(m_events)))
            return false;
         if(!WriteAuctionCycle()) return false;
         m_reading.logger_microseconds = GetMicrosecondCount() - stage_us;
         m_reading.update_microseconds = GetMicrosecondCount() - start_us;
         return true;
      }

      m_reading.structure_rebuilt = true;
      stage_us = GetMicrosecondCount();
      int cluster_count = m_clusterer.Fit(m_levels, level_count,
                                          m_config.compatibility,
                                          m_config.clustering, m_labels);
      if(cluster_count < 0) return false;
      m_reading.dbscan_microseconds = GetMicrosecondCount() - stage_us;
      int fitted_count = MathMin(level_count, ArraySize(m_labels));
      stage_us = GetMicrosecondCount();
      int node_count = MST_BuildNodes(m_levels, fitted_count, m_labels,
                                      cluster_count, reference_price, atr,
                                      m_config.clustering, m_nodes, m_provenance);
      m_reading.node_build_microseconds = GetMicrosecondCount() - stage_us;

      datetime market_time = ResolveMarketTime();
      stage_us = GetMicrosecondCount();
      m_tracker.Reconcile(m_nodes, node_count, atr, market_time, reference_price);
      m_context_builder.EnrichCorridors(m_nodes, node_count, m_levels, m_labels,
                                        fitted_count);
      ulong node_hash = MST_SnapshotHash(m_nodes, node_count, m_point);
      ulong snapshot_hash = FullSnapshotHash(fitted_count, node_hash);
      m_last_input_hash = input_hash;
      if(snapshot_hash != m_last_snapshot_hash)
      {
         m_reading.generation++;
         m_reading.render_generation++;
         m_last_snapshot_hash = snapshot_hash;
      }
      m_reading.snapshot_hash = snapshot_hash;
      RefreshRegionalState(reference_price, atr,
                           iTime(m_symbol, m_config.master_timeframe, 1), true);
      ObserveAuctions(reference_price, atr, market_time);
      m_reading.lifecycle_microseconds = GetMicrosecondCount() - stage_us;

      m_reading.valid = true;
      m_reading.symbol = m_symbol;
      m_reading.timeframe = m_config.master_timeframe;
      m_reading.market_time = market_time;
      m_reading.calculation_bar_time = iTime(m_symbol, m_config.master_timeframe, 0);
      m_reading.reference_price = reference_price;
      m_reading.atr = atr;
      m_reading.raw_level_count = fitted_count;
      m_reading.cluster_count = cluster_count;
      m_reading.node_count = node_count;
      m_reading.noise_count = m_clusterer.NoiseCount();
      m_reading.dropped_level_count = dropped;
      m_reading.update_microseconds = GetMicrosecondCount() - start_us;
      stage_us = GetMicrosecondCount();
      if(!m_logger.WriteSnapshot(m_reading, m_levels, fitted_count, m_labels,
                                 m_nodes, node_count))
         return false;
      if(!m_logger.WriteEvents(m_reading, m_events, ArraySize(m_events)))
         return false;
      if(!WriteAuctionCycle()) return false;
      m_reading.logger_microseconds = GetMicrosecondCount() - stage_us;
      m_reading.update_microseconds = GetMicrosecondCount() - start_us;
      return true;
   }

   void Finalize(const int terminal_reason)
   {
      if(!m_initialized || m_finalized) return;
      datetime market_time = m_last_market_time > 0 ? m_last_market_time : ResolveMarketTime();
      datetime bar_time = m_last_closed_time;
      double price = m_last_reference_price > 0.0 ? m_last_reference_price : ResolveReferencePrice();
      double atr = m_last_atr > 0.0 ? m_last_atr : MathMax(m_point, 0.00000001);
      MST_CENSOR_REASON censor = (bool)MQLInfoInteger(MQL_TESTER) ?
                                 MST_CENSOR_TEST_END : MST_CENSOR_SHUTDOWN;
      m_auction.Finalize(market_time, bar_time, price, atr, censor);
      CollectAuctionOutputs();
      WriteAuctionCycle();
      m_run_manifest.ended_at = market_time;
      m_run_manifest.terminal_reason = terminal_reason;
      m_run_manifest.terminal_hash = m_auction.TerminalHash();
      m_run_manifest.event_sequence = m_auction.EventSequence();
      m_auction.GetInvariantReading(m_run_manifest.invariants);
      m_auction_logger.WriteRun("END", m_run_manifest);
      m_auction_logger.Flush();
      PrintFormat("MST_DATASET_FINAL hash=%I64u receipt=%I64u events=%I64u " +
                  "attempts=%I64d/%I64d/%I64d episodes=%I64d/%I64d/%I64d " +
                  "transits=%I64d/%I64d/%I64d active=%d/%d/%d balanced=%d reason=%d",
                  m_run_manifest.terminal_hash, m_run_manifest.invariants.receipt_hash,
                  m_run_manifest.event_sequence,
                  m_run_manifest.invariants.attempts_started,
                  m_run_manifest.invariants.attempts_resolved,
                  m_run_manifest.invariants.attempts_censored,
                  m_run_manifest.invariants.episodes_started,
                  m_run_manifest.invariants.episodes_resolved,
                  m_run_manifest.invariants.episodes_censored,
                  m_run_manifest.invariants.transits_started,
                  m_run_manifest.invariants.transits_resolved,
                  m_run_manifest.invariants.transits_censored,
                  m_run_manifest.invariants.active_attempts,
                  m_run_manifest.invariants.active_episodes,
                  m_run_manifest.invariants.active_transits,
                  (int)m_run_manifest.invariants.balanced, terminal_reason);
      m_finalized = true;
   }

   void Deinit(void)
   {
      if(m_initialized && !m_finalized) Finalize(-1);
      m_logger.Close();
      m_auction_logger.Close();
      m_auction.Reset();
      m_tracker.Reset();
      if(m_atr_handle != INVALID_HANDLE)
      {
         IndicatorRelease(m_atr_handle);
         m_atr_handle = INVALID_HANDLE;
      }
      m_initialized = false;
      m_finalized = true;
   }

   bool IsInitialized(void) const { return m_initialized; }
   int LevelCount(void) const { return ArraySize(m_levels); }
   int NodeCount(void) const { return ArraySize(m_nodes); }
   int ProvenanceCount(void) const { return ArraySize(m_provenance); }
   int EventCount(void) const { return ArraySize(m_events); }

   bool GetLevel(const int index, MST_Level &out) const
   {
      if(index < 0 || index >= ArraySize(m_levels)) return false;
      out = m_levels[index];
      return out.valid;
   }

   bool GetNode(const int index, MST_Node &out) const
   {
      if(index < 0 || index >= ArraySize(m_nodes)) return false;
      out = m_nodes[index];
      return out.valid;
   }

   bool GetProvenance(const int index, MST_SourceRef &out) const
   {
      if(index < 0 || index >= ArraySize(m_provenance)) return false;
      out = m_provenance[index];
      return true;
   }

   bool GetEvent(const int index, MST_NodeEvent &out) const
   {
      if(index < 0 || index >= ArraySize(m_events)) return false;
      out = m_events[index];
      return true;
   }

   void GetReading(MST_ControllerReading &out) const { out = m_reading; }

   bool FindNearestNodes(const double price,
                         double &nearest,
                         double &lower,
                         double &upper) const
   {
      nearest = EMPTY_VALUE;
      lower = EMPTY_VALUE;
      upper = EMPTY_VALUE;
      double best = DBL_MAX;
      double lower_gap = DBL_MAX;
      double upper_gap = DBL_MAX;
      for(int i = 0; i < ArraySize(m_nodes); i++)
      {
         if(!m_nodes[i].valid) continue;
         double gap = MathAbs(price - m_nodes[i].price);
         if(gap < best) { best = gap; nearest = m_nodes[i].price; }
         if(m_nodes[i].price <= price && price - m_nodes[i].price < lower_gap)
         {
            lower_gap = price - m_nodes[i].price;
            lower = m_nodes[i].price;
         }
         if(m_nodes[i].price >= price && m_nodes[i].price - price < upper_gap)
         {
            upper_gap = m_nodes[i].price - price;
            upper = m_nodes[i].price;
         }
      }
      return nearest != EMPTY_VALUE;
   }
};

#endif // __KITT_MASTER_STRUCTURE_CONTROLLER_MQH__
