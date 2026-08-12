//+------------------------------------------------------------------+
//| ChartController.mqh                                              |
//| Chart-only topology, regions, stable nodes, and visual state.    |
//+------------------------------------------------------------------+
#ifndef __KITT_MASTER_STRUCTURE_CHART_CONTROLLER_MQH__
#define __KITT_MASTER_STRUCTURE_CHART_CONTROLLER_MQH__

#include "MasterTypes.mqh"
#include "MasterAdapters.mqh"
#include "MasterClusterer.mqh"
#include "MasterNodeTracker.mqh"
#include "MasterRegionalModel.mqh"
#include "ChartInteraction.mqh"

struct MSC_ControllerConfig
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
   MSC_InteractionConfig   interaction;
};

void MSC_DefaultControllerConfig(MSC_ControllerConfig &cfg)
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
   cfg.volkitt_instance = 2;
   cfg.day_swings_instance = 2;
   cfg.wayne_instance = 2;
   KVP_DefaultConfig(cfg.volkitt);
   DSW_DefaultConfig(cfg.day_swings);
   WYN_DefaultConfig(cfg.wayne);
   MST_DefaultCompatibility(cfg.compatibility);
   MST_DefaultClusterConfig(cfg.clustering);
   MST_DefaultNodeLifecycleConfig(cfg.lifecycle);
   MSC_DefaultInteractionConfig(cfg.interaction);
}

class CMasterStructureChartController
{
private:
   string                            m_symbol;
   MSC_ControllerConfig              m_config;
   bool                              m_initialized;
   int                               m_atr_handle;
   double                            m_point;
   CKittVolKittMarketProfileProducer m_volkitt;
   CKittDaySwingsProducer            m_day_swings;
   CWaynePivotProducer               m_wayne;
   CMstDensityClusterer              m_clusterer;
   CMstNodeTracker                   m_tracker;
   CMstRegionalModel                 m_regional_model;
   CMscInteractionTracker            m_interactions;
   KVP_Level                         m_kvp_levels[];
   DSW_Level                         m_day_levels[];
   WYN_Level                         m_wayne_levels[];
   MST_Level                         m_levels[];
   int                               m_labels[];
   MST_Node                          m_nodes[];
   MST_SourceRef                     m_provenance[];
   MST_NodeEvent                     m_events[];
   MST_ControllerReading             m_reading;
   ulong                             m_last_snapshot_hash;
   ulong                             m_last_input_hash;

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
      if(SymbolInfoTick(m_symbol, tick) && tick.time > 0) return tick.time;
      return TimeCurrent();
   }

   bool ReadClosedAtr(double &atr)
   {
      atr = 0.0;
      if(m_atr_handle == INVALID_HANDLE || BarsCalculated(m_atr_handle) < 2)
         return false;
      double values[1];
      if(CopyBuffer(m_atr_handle, 0, 1, 1, values) != 1) return false;
      atr = values[0];
      return MST_IsFinitePrice(atr) && atr > 0.0;
   }

   int CollectLevels(void)
   {
      int kvp_count = m_config.use_volkitt ? m_volkitt.ExportLevels(m_kvp_levels) : 0;
      int day_count = m_config.use_day_swings ? m_day_swings.ExportLevels(m_day_levels) : 0;
      int wayne_count = m_config.use_wayne ? m_wayne.ExportLevels(m_wayne_levels) : 0;
      if(!m_config.use_volkitt) ArrayResize(m_kvp_levels, 0);
      if(!m_config.use_day_swings) ArrayResize(m_day_levels, 0);
      if(!m_config.use_wayne) ArrayResize(m_wayne_levels, 0);

      int requested = MathMax(kvp_count, 0) + MathMax(day_count, 0) + MathMax(wayne_count, 0);
      if(ArrayResize(m_levels, requested,
                     MathMax(m_config.clustering.max_levels, requested)) < requested)
         return -1;
      int written = 0;
      written += MST_AdaptKvpLevels(m_kvp_levels, kvp_count,
                                    m_config.volkitt_instance, m_levels, written);
      written += MST_AdaptDayLevels(m_day_levels, day_count,
                                    m_config.day_swings_instance, m_levels, written);
      written += MST_AdaptWayneLevels(m_wayne_levels, wayne_count,
                                      m_config.wayne_instance, m_levels, written);
      if(written != ArraySize(m_levels)) ArrayResize(m_levels, written);
      return written;
   }

   int NormalizeAndFilter(const double reference_price, const double atr)
   {
      int source_count = ArraySize(m_levels);
      int write = 0;
      for(int i = 0; i < source_count; i++)
      {
         if(!MST_NormalizeLevel(m_levels[i], reference_price, atr)) continue;
         if(!MST_LevelsCompatible(m_levels[i], m_levels[i], m_config.compatibility)) continue;
         if(write != i) m_levels[write] = m_levels[i];
         write++;
      }
      if(write != source_count) ArrayResize(m_levels, write);
      MST_SortLevels(m_levels, write);
      return write;
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

   ulong FullSnapshotHash(const int level_count, const ulong node_hash) const
   {
      ulong hash = MST_HashMix(node_hash, (ulong)level_count);
      double safe_point = MathMax(m_point, 0.00000001);
      for(int i = 0; i < level_count; i++)
      {
         hash = MST_HashMix(hash, m_levels[i].source_key);
         hash = MST_HashMix(hash, (ulong)MathRound(m_levels[i].lower / safe_point));
         hash = MST_HashMix(hash, (ulong)MathRound(m_levels[i].price / safe_point));
         hash = MST_HashMix(hash, (ulong)MathRound(m_levels[i].upper / safe_point));
         hash = MST_HashMix(hash, (ulong)((int)m_levels[i].state));
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
         MST_NODE_ROLE prior = m_nodes[c].role;
         if(m_nodes[c].normalized_price < -m_config.clustering.center_role_tolerance_atr)
            m_nodes[c].role = MST_NODE_LOWER;
         else if(m_nodes[c].normalized_price > m_config.clustering.center_role_tolerance_atr)
            m_nodes[c].role = MST_NODE_UPPER;
         else
            m_nodes[c].role = MST_NODE_CENTER;
         if(prior != m_nodes[c].role) changed = true;
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
      for(int i = 0; i < count; i++) m_tracker.GetEvent(i, m_events[i]);
      m_reading.lifecycle_event_count = count;
      m_reading.tracked_node_count = m_tracker.TrackCount();
   }

   void RefreshRegionalState(const double reference_price, const double atr,
                             const datetime closed_bar_time, const bool rebuilt)
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

   bool RefreshInteractionState(const double reference_price, const double atr)
   {
      datetime closed_time = iTime(m_symbol, m_config.master_timeframe, 1);
      double closed_price = iClose(m_symbol, m_config.master_timeframe, 1);
      bool changed = m_interactions.Observe(m_nodes, ArraySize(m_nodes),
                                            reference_price, atr,
                                            closed_time, closed_price);
      m_tracker.SyncAttemptCounts(m_nodes, ArraySize(m_nodes));
      m_reading.active_attempt_count = m_interactions.ActiveCount();
      return changed;
   }

public:
   CMasterStructureChartController(void)
   {
      m_symbol = "";
      m_initialized = false;
      m_atr_handle = INVALID_HANDLE;
      m_point = 0.0;
      m_last_snapshot_hash = 0;
      m_last_input_hash = 0;
      ZeroMemory(m_reading);
   }

   bool Init(const string symbol, const MSC_ControllerConfig &config)
   {
      Deinit();
      m_symbol = StringLen(symbol) > 0 ? symbol : _Symbol;
      m_config = config;
      if(m_config.master_timeframe == PERIOD_CURRENT)
         m_config.master_timeframe = (ENUM_TIMEFRAMES)_Period;
      if(m_config.atr_period < 2 || m_config.clustering.epsilon_atr < 0.0 ||
         m_config.clustering.min_samples < 1 ||
         m_config.clustering.max_levels < 1 || m_config.clustering.max_nodes < 1 ||
         m_config.lifecycle.match_distance_atr < 0.0 ||
         m_config.lifecycle.retire_after_rebuilds < 1 ||
         m_config.interaction.approach_radius_atr < 0.0 ||
         m_config.interaction.acceptance_closes < 1)
         return false;

      m_point = SymbolInfoDouble(m_symbol, SYMBOL_POINT);
      if(m_point <= 0.0) return false;
      m_atr_handle = iATR(m_symbol, m_config.master_timeframe, m_config.atr_period);
      if(m_atr_handle == INVALID_HANDLE) return false;

      // Live history may still be synchronizing. Producer init is deliberately
      // non-fatal here; each producer retries from Update without reattachment.
      if(m_config.use_volkitt)
         m_volkitt.Init(m_symbol, m_config.volkitt_timeframe,
                        m_config.profile_timeframe, m_config.volkitt);
      if(m_config.use_day_swings)
         m_day_swings.Init(m_symbol, m_config.day_swings_timeframe,
                           m_config.day_swings);
      if(m_config.use_wayne)
         m_wayne.Init(m_symbol, m_config.wayne_timeframe, m_config.wayne);

      ArrayResize(m_levels, 0);
      ArrayResize(m_labels, 0);
      ArrayResize(m_nodes, 0);
      ArrayResize(m_provenance, 0);
      ArrayResize(m_events, 0);
      m_tracker.Init(m_config.lifecycle);
      m_regional_model.Reset();
      m_interactions.Init(m_config.interaction);
      ZeroMemory(m_reading);
      m_reading.symbol = m_symbol;
      m_reading.timeframe = m_config.master_timeframe;
      m_last_snapshot_hash = 0;
      m_last_input_hash = 0;
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
      m_reading.lifecycle_event_count = 0;

      bool any_ready = false;
      if(m_config.use_volkitt)
      {
         ulong stage = GetMicrosecondCount();
         any_ready = m_volkitt.Update(force_heavy, force_profile) || any_ready;
         m_reading.volkitt_microseconds = GetMicrosecondCount() - stage;
         KVP_PerformanceReading perf;
         m_volkitt.GetPerformance(perf);
         m_reading.volkitt_heavy_core = perf.heavy_core_due;
         m_reading.volkitt_profile_rebuilt = perf.profile_rebuild_due;
         m_reading.volkitt_core_microseconds = perf.core_microseconds;
         m_reading.volkitt_profile_microseconds = perf.profile_microseconds;
      }
      if(m_config.use_day_swings)
      {
         ulong stage = GetMicrosecondCount();
         any_ready = m_day_swings.Update() || any_ready;
         m_reading.day_swings_microseconds = GetMicrosecondCount() - stage;
      }
      if(m_config.use_wayne)
      {
         ulong stage = GetMicrosecondCount();
         any_ready = m_wayne.Update() || any_ready;
         m_reading.wayne_microseconds = GetMicrosecondCount() - stage;
      }
      if(!any_ready) return false;

      ulong stage = GetMicrosecondCount();
      double reference_price = ResolveReferencePrice();
      double atr = 0.0;
      if(!MST_IsFinitePrice(reference_price) || reference_price <= 0.0 ||
         !ReadClosedAtr(atr)) return false;
      m_reading.reference_atr_microseconds = GetMicrosecondCount() - stage;

      stage = GetMicrosecondCount();
      int collected_count = CollectLevels();
      if(collected_count < 0) return false;
      m_reading.collect_microseconds = GetMicrosecondCount() - stage;
      stage = GetMicrosecondCount();
      ulong input_hash = StructuralInputHash(collected_count, atr);
      m_reading.hash_microseconds = GetMicrosecondCount() - stage;
      stage = GetMicrosecondCount();
      int level_count = NormalizeAndFilter(reference_price, atr);
      m_reading.normalize_microseconds = GetMicrosecondCount() - stage;
      int dropped = MathMax(level_count - m_config.clustering.max_levels, 0);
      bool rebuild = !allow_cached_structure || force_heavy || force_profile ||
                     input_hash != m_last_input_hash || !m_reading.valid;

      if(!rebuild)
      {
         bool role_changed = RefreshCachedNodeState(reference_price, atr);
         datetime market_time = ResolveMarketTime();
         m_tracker.BeginCycle();
         stage = GetMicrosecondCount();
         RefreshRegionalState(reference_price, atr,
                              iTime(m_symbol, m_config.master_timeframe, 1), false);
         bool interaction_changed = RefreshInteractionState(reference_price, atr);
         CollectTrackerEvents();
         m_reading.lifecycle_microseconds = GetMicrosecondCount() - stage;
         if(role_changed || interaction_changed || m_reading.lifecycle_event_count > 0)
            m_reading.render_generation++;
         m_reading.market_time = market_time;
         m_reading.calculation_bar_time = iTime(m_symbol, m_config.master_timeframe, 0);
         m_reading.reference_price = reference_price;
         m_reading.atr = atr;
         m_reading.raw_level_count = level_count;
         m_reading.node_count = ArraySize(m_nodes);
         m_reading.dropped_level_count = dropped;
         m_reading.update_microseconds = GetMicrosecondCount() - start_us;
         return true;
      }

      m_reading.structure_rebuilt = true;
      stage = GetMicrosecondCount();
      int cluster_count = m_clusterer.Fit(m_levels, level_count,
                                          m_config.compatibility,
                                          m_config.clustering, m_labels);
      if(cluster_count < 0) return false;
      m_reading.dbscan_microseconds = GetMicrosecondCount() - stage;
      int fitted_count = MathMin(level_count, ArraySize(m_labels));
      stage = GetMicrosecondCount();
      int node_count = MST_BuildNodes(m_levels, fitted_count, m_labels,
                                      cluster_count, reference_price, atr,
                                      m_config.clustering, m_nodes, m_provenance);
      m_reading.node_build_microseconds = GetMicrosecondCount() - stage;
      datetime market_time = ResolveMarketTime();
      stage = GetMicrosecondCount();
      m_tracker.Reconcile(m_nodes, node_count, atr, market_time, reference_price);
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
      bool interaction_changed = RefreshInteractionState(reference_price, atr);
      CollectTrackerEvents();
      if(interaction_changed) m_reading.render_generation++;
      m_reading.lifecycle_microseconds = GetMicrosecondCount() - stage;
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
      return true;
   }

   void Deinit(void)
   {
      m_interactions.Reset();
      m_regional_model.Reset();
      m_tracker.Reset();
      if(m_atr_handle != INVALID_HANDLE)
      {
         IndicatorRelease(m_atr_handle);
         m_atr_handle = INVALID_HANDLE;
      }
      ArrayResize(m_levels, 0);
      ArrayResize(m_labels, 0);
      ArrayResize(m_nodes, 0);
      ArrayResize(m_provenance, 0);
      ArrayResize(m_events, 0);
      m_initialized = false;
      ZeroMemory(m_reading);
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

   bool FindNearestNodes(const double price, double &nearest,
                         double &lower, double &upper) const
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

#endif // __KITT_MASTER_STRUCTURE_CHART_CONTROLLER_MQH__
