//+------------------------------------------------------------------+
//| MasterStructureController.mq5                                    |
//| Non-trading live map over headless structural producers.         |
//+------------------------------------------------------------------+
#property version   "1.00"
#property indicator_chart_window
#property indicator_buffers 4
#property indicator_plots   4

#include <MasterStructure\MasterController.mqh>

input group "Master Runtime"
#ifdef MST_VISUAL_ONLY
input string          InpInstanceTag          = "MAP";
#else
input string          InpInstanceTag          = "MASTER";
#endif
input ENUM_TIMEFRAMES InpMasterTimeframe      = PERIOD_M5;
input int             InpATRPeriod            = 100;
input uint            InpTimerMilliseconds    = 750;
input bool            InpTesterCacheUnchangedStructure = true;

input group "Producer Timeframes"
input ENUM_TIMEFRAMES InpVolKittTimeframe     = PERIOD_H1;
input ENUM_TIMEFRAMES InpProfileTimeframe     = PERIOD_H1;
input ENUM_TIMEFRAMES InpDaySwingsTimeframe   = PERIOD_M5;
input ENUM_TIMEFRAMES InpWayneTimeframe       = PERIOD_D1;

input group "VolKitt and Profile"
input int             InpVolKittLookback      = 200;
input int             InpVolKittClusters      = 5;
input int             InpVolKittIterations    = 50;
input bool            InpEnableProfile        = true;
input int             InpProfilesToKeep       = 3;
input bool            InpEnableSinglePrints   = false;

input group "Day Swings and Wayne"
input int             InpDaysToKeep           = 5;
input int             InpWaynePeriodsToKeep   = 5;
input bool            InpWayneStandardPivots  = true;
input bool            InpWayneMidPivots       = true;
input bool            InpWayneZones           = true;

input group "Master Density Clustering"
input double          InpEpsilonATR            = 0.12;
input int             InpMinimumSamples        = 2;
input bool            InpUseIntervalDistance   = true;
input bool            InpRequireRoleCompatibility = true;
input int             InpMaximumLevels         = 1024;
input int             InpMaximumNodes          = 256;
input double          InpCenterRoleToleranceATR = 0.15;

input group "Stable Node Existence"
input double          InpNodeMatchDistanceATR   = 0.20;
input int             InpRetireAfterRebuilds    = 3;

input group "Auction Episode Grammar"
input bool            InpEnableAuctionEngine    = true;
input double          InpApproachDistanceATR    = 0.50;
input double          InpRejectionDistanceATR   = 0.20;
input double          InpBreakBufferATR         = 0.00;
input int             InpAcceptanceCloses       = 2;
input double          InpAcceptanceDistanceATR  = 0.00;
input double          InpReclaimToleranceATR    = 0.05;
input double          InpDepartureDistanceATR   = 0.25;
input int             InpMaxAttemptBars         = 24;
input int             InpEpisodeGapBars         = 12;

#ifndef MST_VISUAL_ONLY
input group "Research Receipts"
input bool            InpEnableLogging         = false;
input bool            InpEnableParityOracle    = false;
input int             InpLogFlushSnapshots     = 10;
input datetime        InpTesterFinalizeAt      = 0;

input group "RG Research Identity"
input string          InpCanonicalInstrument  = "AUTO";
input string          InpDataSourceId          = "BROKER_MT5";
input string          InpDataFingerprint       = "AUTO";
input datetime        InpResearchWindowStart   = 0;
input datetime        InpResearchWindowEnd     = 0;
#else
#define InpEnableLogging false
#define InpEnableParityOracle false
#define InpLogFlushSnapshots 0
#define InpTesterFinalizeAt 0
#define InpCanonicalInstrument "LIVE_MAP"
#define InpDataSourceId "BROKER_MT5"
#define InpDataFingerprint "VISUAL_ONLY"
#define InpResearchWindowStart 0
#define InpResearchWindowEnd 0
#endif

input group "Live Map"
input bool            InpShowNodes             = true;
input bool            InpShowLabels            = true;
input bool            InpShowProvenance         = true;
input int             InpMaximumRenderedNodes  = 64;
input int             InpHistoryBars           = 150;
input int             InpFutureBars            = 30;
input int             InpLabelGapBars          = 6;

input group "Performance Telemetry"
input bool            InpEnablePerformanceTelemetry = false;
input int             InpPerformanceWindowUpdates = 100000;
input int             InpSlowFrameThresholdUs = 5000;

double NearestNodeBuffer[];
double LowerNodeBuffer[];
double UpperNodeBuffer[];
double NodeCountBuffer[];

CMasterStructureController g_master;
MST_ControllerConfig g_pending_config;
bool  g_ready = false;
bool  g_is_tester = false;
ulong g_rendered_generation = 0;
bool  g_has_rendered_snapshot = false;
string g_prefix = "MST_";
string g_owner_prefix = "MST_";
ulong g_rendered_node_ids[];
datetime g_replay_checkpoint_bar = 0;
int g_replay_checkpoint_bars = 0;
bool g_tester_cutoff_finalized = false;
bool g_refresh_in_progress = false;
bool g_live_primed = false;
ulong g_live_next_prime_ms = 0;
int g_live_prime_failures = 0;

#define MST_PERF_STAGE_COUNT 18
enum MST_PERF_STAGE
{
   MST_PERF_TOTAL = 0,
   MST_PERF_VOLKITT,
   MST_PERF_VOLKITT_CORE,
   MST_PERF_VOLKITT_PROFILE,
   MST_PERF_DAY_SWINGS,
   MST_PERF_WAYNE,
   MST_PERF_REFERENCE_ATR,
   MST_PERF_COLLECT,
   MST_PERF_NORMALIZE,
   MST_PERF_DBSCAN,
   MST_PERF_NODE_BUILD,
   MST_PERF_LIFECYCLE,
   MST_PERF_HASH,
   MST_PERF_LOGGER,
   MST_PERF_RENDER,
   MST_PERF_DELETE,
   MST_PERF_DRAW,
   MST_PERF_REDRAW
};

struct MST_RenderPerformance
{
   bool  ran;
   int   rendered_nodes;
   ulong delete_microseconds;
   ulong draw_microseconds;
   ulong redraw_microseconds;
   ulong total_microseconds;
};

ulong g_perf_sum[MST_PERF_STAGE_COUNT];
ulong g_perf_max[MST_PERF_STAGE_COUNT];
ulong g_perf_count = 0;
ulong g_perf_render_count = 0;
ulong g_perf_heavy_count = 0;
ulong g_perf_profile_count = 0;
ulong g_perf_structure_rebuild_count = 0;
bool  g_perf_first_frame_pending = true;

void ResetPerformanceWindow(void)
{
   ArrayInitialize(g_perf_sum, 0);
   ArrayInitialize(g_perf_max, 0);
   g_perf_count = 0;
   g_perf_render_count = 0;
   g_perf_heavy_count = 0;
   g_perf_profile_count = 0;
   g_perf_structure_rebuild_count = 0;
}

void RecordPerformanceStage(const int stage, const ulong microseconds)
{
   if(stage < 0 || stage >= MST_PERF_STAGE_COUNT) return;
   g_perf_sum[stage] += microseconds;
   if(microseconds > g_perf_max[stage])
      g_perf_max[stage] = microseconds;
}

double PerformanceMean(const int stage, const ulong divisor)
{
   if(stage < 0 || stage >= MST_PERF_STAGE_COUNT || divisor == 0) return 0.0;
   return (double)g_perf_sum[stage] / (double)divisor;
}

void SetupHiddenBuffer(const int index, double &buffer[], const string label)
{
   SetIndexBuffer(index, buffer, INDICATOR_DATA);
   ArraySetAsSeries(buffer, true);
   PlotIndexSetInteger(index, PLOT_DRAW_TYPE, DRAW_NONE);
   PlotIndexSetInteger(index, PLOT_SHOW_DATA, true);
   PlotIndexSetDouble(index, PLOT_EMPTY_VALUE, EMPTY_VALUE);
   PlotIndexSetString(index, PLOT_LABEL, label);
}

color NodeRegionColor(const MST_Node &node, const bool is_median_anchor)
{
   if(is_median_anchor) return clrYellow;
   if(node.contains_cog) return clrLimeGreen;
   if(node.structural_region == MST_REGION_EXTREME_BELOW) return clrViolet;
   if(node.structural_region == MST_REGION_FAR_BELOW) return clrIndigo;
   if(node.structural_region == MST_REGION_BELOW) return clrDeepSkyBlue;
   if(node.structural_region == MST_REGION_MEDIAN_CORE)
      return node.median_distance_sigma < 0.0 ? clrTurquoise : clrGoldenrod;
   if(node.structural_region == MST_REGION_ABOVE)
   {
      if(node.median_distance_sigma < 1.0) return clrOrange;
      if(node.median_distance_sigma < 1.25) return clrDarkOrange;
      return clrOrangeRed;
   }
   if(node.structural_region == MST_REGION_FAR_ABOVE) return clrTomato;
   if(node.structural_region == MST_REGION_EXTREME_ABOVE) return clrRed;
   return clrSilver;
}

string NodeVisualRegionName(const MST_Node &node, const bool is_median_anchor)
{
   if(is_median_anchor) return "MEDIAN";
   if(node.structural_region == MST_REGION_MEDIAN_CORE)
      return node.median_distance_sigma < 0.0 ? "LOWER_CORE" : "UPPER_CORE";
   return MST_RegionName(node.structural_region);
}

ENUM_LINE_STYLE NodeLifecycleStyle(const MST_Node &node)
{
   MST_AUCTION_STATE state = (MST_AUCTION_STATE)node.interaction_state;
   if(state == MST_AUCTION_BROKEN || state == MST_AUCTION_PROVISIONAL_ACCEPTANCE)
      return STYLE_DASH;
   if(state == MST_AUCTION_CONTACT || state == MST_AUCTION_PENETRATION ||
      state == MST_AUCTION_RETEST) return STYLE_DASHDOT;
   return node.family_count > 1 ? STYLE_SOLID : STYLE_DOT;
}

string NodeName(const ulong node_id, const string suffix)
{
   return g_prefix + StringFormat("%I64u", node_id) + suffix;
}

void ConfigureObject(const string name, const color base_color)
{
   ObjectSetInteger(0, name, OBJPROP_COLOR, base_color);
   ObjectSetInteger(0, name, OBJPROP_SELECTABLE, false);
   ObjectSetInteger(0, name, OBJPROP_SELECTED, false);
   ObjectSetInteger(0, name, OBJPROP_HIDDEN, true);
}

string NodeProvenance(const MST_Node &node)
{
   if(!InpShowProvenance) return "";
   string result = "";
   ulong seen = 0;
   int end = node.provenance_offset + node.provenance_count;
   for(int i = node.provenance_offset; i < end; i++)
   {
      MST_SourceRef source;
      if(!g_master.GetProvenance(i, source)) continue;
      ulong bit = ((ulong)1 << (int)source.producer);
      if((seen & bit) != 0) continue;
      if(StringLen(result) > 0) result += "+";
      result += MST_ProducerName(source.producer);
      seen |= bit;
   }
   return result;
}

void DeleteNodeObjects(const ulong node_id)
{
   ObjectDelete(0, NodeName(node_id, "_LINE"));
   ObjectDelete(0, NodeName(node_id, "_BAND"));
   ObjectDelete(0, NodeName(node_id, "_TEXT"));
}

void DeleteOwnedObjects(void)
{
   for(int i = 0; i < ArraySize(g_rendered_node_ids); i++)
      DeleteNodeObjects(g_rendered_node_ids[i]);
   ArrayResize(g_rendered_node_ids, 0);
   ObjectsDeleteAll(0, g_prefix);
   if(g_owner_prefix != g_prefix)
      ObjectsDeleteAll(0, g_owner_prefix);
   ChartRedraw(0);
}

#ifdef MST_VISUAL_ONLY
void SetMapBootStatus(const string text)
{
   string name = g_prefix + "BOOT";
   if(ObjectFind(0, name) < 0)
      ObjectCreate(0, name, OBJ_LABEL, 0, 0, 0);
   ConfigureObject(name, clrSilver);
   // Initialization feedback must remain visible while producer histories
   // synchronize. Ordinary map objects stay hidden from manual object lists.
   ObjectSetInteger(0, name, OBJPROP_HIDDEN, false);
   ObjectSetInteger(0, name, OBJPROP_BACK, false);
   ObjectSetInteger(0, name, OBJPROP_ZORDER, 1000);
   ObjectSetInteger(0, name, OBJPROP_CORNER, CORNER_LEFT_UPPER);
   ObjectSetInteger(0, name, OBJPROP_XDISTANCE, 10);
   ObjectSetInteger(0, name, OBJPROP_YDISTANCE, 36);
   ObjectSetInteger(0, name, OBJPROP_FONTSIZE, 9);
   ObjectSetString(0, name, OBJPROP_TEXT, text);
   ChartRedraw(0);
}
#endif

bool ValidConfiguredTimeframes(void)
{
   ENUM_TIMEFRAMES frames[5] =
   {
      InpMasterTimeframe,
      InpVolKittTimeframe,
      InpProfileTimeframe,
      InpDaySwingsTimeframe,
      InpWayneTimeframe
   };
   for(int i = 0; i < ArraySize(frames); i++)
   {
      if(frames[i] == PERIOD_CURRENT) continue;
      if(PeriodSeconds(frames[i]) > 0) continue;
      PrintFormat("MST INIT FAILED: unsupported timeframe code %d at slot %d",
                  (int)frames[i], i);
      return false;
   }
   return true;
}

bool ContainsNodeId(const ulong &ids[], const ulong id)
{
   for(int i = 0; i < ArraySize(ids); i++)
      if(ids[i] == id) return true;
   return false;
}

void DrawNode(const MST_Node &node,
              const datetime left_time,
              const datetime line_right_time,
              const datetime band_right_time,
              const datetime label_time,
              const double point,
              const bool is_median_anchor)
{
   color base_color = NodeRegionColor(node, is_median_anchor);
   string line_name = NodeName(node.node_id, "_LINE");
   if(ObjectFind(0, line_name) >= 0 &&
      (ENUM_OBJECT)ObjectGetInteger(0, line_name, OBJPROP_TYPE) != OBJ_TREND)
      ObjectDelete(0, line_name);
   if(ObjectFind(0, line_name) < 0)
      ObjectCreate(0, line_name, OBJ_TREND, 0,
                   left_time, node.price, line_right_time, node.price);
   ObjectMove(0, line_name, 0, left_time, node.price);
   ObjectMove(0, line_name, 1, line_right_time, node.price);
   ConfigureObject(line_name, base_color);
   ObjectSetInteger(0, line_name, OBJPROP_RAY_LEFT, false);
   ObjectSetInteger(0, line_name, OBJPROP_RAY_RIGHT, false);
   ObjectSetInteger(0, line_name, OBJPROP_WIDTH,
                    MathMin(MathMax(node.family_count, 1), 5));
   ObjectSetInteger(0, line_name, OBJPROP_STYLE,
                    NodeLifecycleStyle(node));

   if(node.upper - node.lower > point * 2.0)
   {
      string band_name = NodeName(node.node_id, "_BAND");
      if(ObjectFind(0, band_name) < 0)
         ObjectCreate(0, band_name, OBJ_RECTANGLE, 0,
                      left_time, node.lower, band_right_time, node.upper);
      ObjectMove(0, band_name, 0, left_time, node.lower);
      ObjectMove(0, band_name, 1, band_right_time, node.upper);
      ConfigureObject(band_name, (color)ColorToARGB(base_color, 36));
      ObjectSetInteger(0, band_name, OBJPROP_FILL, true);
      ObjectSetInteger(0, band_name, OBJPROP_BACK, true);
   }
   else
      ObjectDelete(0, NodeName(node.node_id, "_BAND"));

   if(InpShowLabels)
   {
      string text_name = NodeName(node.node_id, "_TEXT");
      if(ObjectFind(0, text_name) < 0)
         ObjectCreate(0, text_name, OBJ_TEXT, 0, label_time, node.price);
      ObjectMove(0, text_name, 0, label_time, node.price);
      ConfigureObject(text_name, base_color);
      ObjectSetInteger(0, text_name, OBJPROP_ANCHOR, ANCHOR_LEFT);
      ObjectSetInteger(0, text_name, OBJPROP_FONTSIZE, 8);
      string provenance = NodeProvenance(node);
      if(StringLen(provenance) > 0) provenance = " " + provenance;
      string region_name = NodeVisualRegionName(node, is_median_anchor);
      ObjectSetString(0, text_name, OBJPROP_TEXT,
                       StringFormat("%s/%s %s z%.2f A%d N%d F%d %.2fATR%s",
                                    MST_NodeStateName(node.existence),
                                    MST_AuctionStateName((MST_AUCTION_STATE)node.interaction_state),
                                    region_name,
                                    node.median_distance_sigma,
                                    node.attempt_count, node.member_count,
                                   node.family_count, node.distance_atr,
                                   provenance));
   }
   else
      ObjectDelete(0, NodeName(node.node_id, "_TEXT"));
}

void DrawStatus(const MST_ControllerReading &reading)
{
   string name = g_prefix + "STATUS";
   if(ObjectCreate(0, name, OBJ_LABEL, 0, 0, 0))
      ConfigureObject(name, clrSilver);
   ObjectSetInteger(0, name, OBJPROP_CORNER, CORNER_LEFT_UPPER);
   ObjectSetInteger(0, name, OBJPROP_XDISTANCE, 10);
   ObjectSetInteger(0, name, OBJPROP_YDISTANCE, 18);
   ObjectSetInteger(0, name, OBJPROP_FONTSIZE, 9);
   ObjectSetString(0, name, OBJPROP_TEXT,
                     StringFormat("MASTER L%d C%d N%d A%d EP%d TR%d E%d noise=%d %.0fus",
                                  reading.raw_level_count, reading.cluster_count,
                                  reading.node_count, reading.active_attempt_count,
                                  reading.active_episode_count, reading.active_transit_count,
                                  reading.auction_event_count, reading.noise_count,
                                  (double)reading.update_microseconds));
}

void RenderSnapshot(const MST_ControllerReading &reading,
                    MST_RenderPerformance &performance)
{
   ZeroMemory(performance);
   performance.ran = true;
   ulong render_start_us = GetMicrosecondCount();
   ulong stage_start_us = render_start_us;
   if(!InpShowNodes)
   {
      for(int i = 0; i < ArraySize(g_rendered_node_ids); i++)
         DeleteNodeObjects(g_rendered_node_ids[i]);
      ArrayResize(g_rendered_node_ids, 0);
      performance.delete_microseconds = GetMicrosecondCount() - stage_start_us;
      stage_start_us = GetMicrosecondCount();
      DrawStatus(reading);
      performance.draw_microseconds = GetMicrosecondCount() - stage_start_us;
      performance.total_microseconds = GetMicrosecondCount() - render_start_us;
      return;
   }

   int node_count = g_master.NodeCount();
   int render_count = MathMin(node_count, MathMax(InpMaximumRenderedNodes, 0));
   bool used[];
   ArrayResize(used, node_count);
   ArrayInitialize(used, false);

   int period_seconds = MathMax(PeriodSeconds((ENUM_TIMEFRAMES)_Period), 1);
   datetime chart_time = iTime(_Symbol, (ENUM_TIMEFRAMES)_Period, 0);
   datetime left_time = chart_time - (datetime)(period_seconds * MathMax(InpHistoryBars, 1));
   datetime right_time = chart_time + (datetime)(period_seconds * MathMax(InpFutureBars, 1));
   datetime label_time = right_time;
   int label_gap_bars = MathMin(MathMax(InpLabelGapBars, 0),
                                MathMax(InpFutureBars, 1));
   datetime line_right_time = label_time -
      (datetime)(period_seconds * label_gap_bars);
   double point = SymbolInfoDouble(_Symbol, SYMBOL_POINT);
   ulong current_ids[];
   ArrayResize(current_ids, 0);
   MST_Node render_nodes[];
   ArrayResize(render_nodes, render_count);
   int selected_count = 0;
   stage_start_us = GetMicrosecondCount();

   for(int rendered = 0; rendered < render_count; rendered++)
   {
      int best_index = -1;
      double best_distance = DBL_MAX;
      MST_Node best_node;
      ZeroMemory(best_node);
      for(int i = 0; i < node_count; i++)
      {
         if(used[i]) continue;
         MST_Node candidate;
         if(!g_master.GetNode(i, candidate)) continue;
         double distance = MathAbs(candidate.price - reading.reference_price);
         if(distance < best_distance)
         {
            best_distance = distance;
            best_index = i;
            best_node = candidate;
         }
      }
      if(best_index < 0) break;
      used[best_index] = true;
      render_nodes[selected_count++] = best_node;
   }

   ulong median_anchor_id = 0;
   if(reading.regional.valid && selected_count > 0)
   {
      double nearest_median = DBL_MAX;
      for(int i = 0; i < selected_count; i++)
      {
         double distance = MathAbs(render_nodes[i].price - reading.regional.median_price);
         if(distance < nearest_median ||
            (distance == nearest_median && render_nodes[i].node_id < median_anchor_id))
         {
            nearest_median = distance;
            median_anchor_id = render_nodes[i].node_id;
         }
      }
   }

   for(int i = 0; i < selected_count; i++)
   {
      MST_Node best_node = render_nodes[i];
      bool is_median_anchor = best_node.node_id == median_anchor_id;
      DrawNode(best_node, left_time, line_right_time, right_time,
               label_time, point, is_median_anchor);
      int id_slot = ArraySize(current_ids);
      ArrayResize(current_ids, id_slot + 1, MathMax(render_count, 1));
      current_ids[id_slot] = best_node.node_id;
      performance.rendered_nodes++;
   }
   DrawStatus(reading);
   performance.draw_microseconds = GetMicrosecondCount() - stage_start_us;
   stage_start_us = GetMicrosecondCount();
   for(int i = 0; i < ArraySize(g_rendered_node_ids); i++)
      if(!ContainsNodeId(current_ids, g_rendered_node_ids[i]))
         DeleteNodeObjects(g_rendered_node_ids[i]);
   ArrayResize(g_rendered_node_ids, ArraySize(current_ids));
   for(int i = 0; i < ArraySize(current_ids); i++)
      g_rendered_node_ids[i] = current_ids[i];
   performance.delete_microseconds = GetMicrosecondCount() - stage_start_us;
   stage_start_us = GetMicrosecondCount();
   ChartRedraw(0);
   performance.redraw_microseconds = GetMicrosecondCount() - stage_start_us;
   performance.total_microseconds = GetMicrosecondCount() - render_start_us;
}

void RecordAndReportPerformance(const MST_ControllerReading &reading,
                                const MST_RenderPerformance &rendering)
{
   if(!InpEnablePerformanceTelemetry) return;

   ulong frame_us = reading.update_microseconds + rendering.total_microseconds;
   RecordPerformanceStage(MST_PERF_TOTAL, frame_us);
   RecordPerformanceStage(MST_PERF_VOLKITT, reading.volkitt_microseconds);
   RecordPerformanceStage(MST_PERF_VOLKITT_CORE, reading.volkitt_core_microseconds);
   RecordPerformanceStage(MST_PERF_VOLKITT_PROFILE, reading.volkitt_profile_microseconds);
   RecordPerformanceStage(MST_PERF_DAY_SWINGS, reading.day_swings_microseconds);
   RecordPerformanceStage(MST_PERF_WAYNE, reading.wayne_microseconds);
   RecordPerformanceStage(MST_PERF_REFERENCE_ATR, reading.reference_atr_microseconds);
   RecordPerformanceStage(MST_PERF_COLLECT, reading.collect_microseconds);
   RecordPerformanceStage(MST_PERF_NORMALIZE, reading.normalize_microseconds);
   RecordPerformanceStage(MST_PERF_DBSCAN, reading.dbscan_microseconds);
   RecordPerformanceStage(MST_PERF_NODE_BUILD, reading.node_build_microseconds);
   RecordPerformanceStage(MST_PERF_LIFECYCLE, reading.lifecycle_microseconds);
   RecordPerformanceStage(MST_PERF_HASH, reading.hash_microseconds);
   RecordPerformanceStage(MST_PERF_LOGGER, reading.logger_microseconds);
   if(rendering.ran)
   {
      RecordPerformanceStage(MST_PERF_RENDER, rendering.total_microseconds);
      RecordPerformanceStage(MST_PERF_DELETE, rendering.delete_microseconds);
      RecordPerformanceStage(MST_PERF_DRAW, rendering.draw_microseconds);
      RecordPerformanceStage(MST_PERF_REDRAW, rendering.redraw_microseconds);
      g_perf_render_count++;
   }
   g_perf_count++;
   if(reading.volkitt_heavy_core) g_perf_heavy_count++;
   if(reading.volkitt_profile_rebuilt) g_perf_profile_count++;
   if(reading.structure_rebuilt) g_perf_structure_rebuild_count++;

   ulong slow_threshold = (ulong)MathMax(InpSlowFrameThresholdUs, 1);
   bool slow = frame_us >= slow_threshold;
   bool structural_event = reading.volkitt_heavy_core || reading.volkitt_profile_rebuilt;
   if(g_perf_first_frame_pending || slow || structural_event)
   {
      PrintFormat("MST_PERF_FRAME slow=%d frame=%I64uus update=%I64u vk=%I64u core=%I64u heavy=%d profile=%I64u rebuilt=%d day=%I64u wayne=%I64u",
                  (int)slow, frame_us, reading.update_microseconds,
                  reading.volkitt_microseconds, reading.volkitt_core_microseconds,
                  (int)reading.volkitt_heavy_core,
                  reading.volkitt_profile_microseconds,
                  (int)reading.volkitt_profile_rebuilt,
                  reading.day_swings_microseconds, reading.wayne_microseconds);
      PrintFormat("MST_PERF_DETAIL structure_rebuilt=%d events=%d refatr=%I64u collect=%I64u normalize=%I64u dbscan=%I64u nodes=%I64u lifecycle=%I64u hash=%I64u logger=%I64u render=%I64u delete=%I64u draw=%I64u redraw=%I64u rendered_nodes=%d",
                   (int)reading.structure_rebuilt,
                   reading.lifecycle_event_count,
                   reading.reference_atr_microseconds, reading.collect_microseconds,
                   reading.normalize_microseconds, reading.dbscan_microseconds,
                   reading.node_build_microseconds, reading.lifecycle_microseconds,
                   reading.hash_microseconds,
                  reading.logger_microseconds, rendering.total_microseconds,
                   rendering.delete_microseconds, rendering.draw_microseconds,
                   rendering.redraw_microseconds, rendering.rendered_nodes);
      g_perf_first_frame_pending = false;
   }

   ulong window = (ulong)MathMax(InpPerformanceWindowUpdates, 1);
   if(g_perf_count < window) return;
   ulong render_divisor = MathMax(g_perf_render_count, (ulong)1);
   PrintFormat("MST_PERF_SUM_A n=%I64u structure_rebuilds=%I64u heavy=%I64u profile=%I64u frame_mean=%.1f/max=%I64u update_components: vk=%.1f/%I64u core=%.1f/%I64u profile=%.1f/%I64u day=%.1f/%I64u wayne=%.1f/%I64u",
               g_perf_count, g_perf_structure_rebuild_count,
               g_perf_heavy_count, g_perf_profile_count,
               PerformanceMean(MST_PERF_TOTAL, g_perf_count), g_perf_max[MST_PERF_TOTAL],
               PerformanceMean(MST_PERF_VOLKITT, g_perf_count), g_perf_max[MST_PERF_VOLKITT],
               PerformanceMean(MST_PERF_VOLKITT_CORE, g_perf_count), g_perf_max[MST_PERF_VOLKITT_CORE],
               PerformanceMean(MST_PERF_VOLKITT_PROFILE, g_perf_count), g_perf_max[MST_PERF_VOLKITT_PROFILE],
               PerformanceMean(MST_PERF_DAY_SWINGS, g_perf_count), g_perf_max[MST_PERF_DAY_SWINGS],
               PerformanceMean(MST_PERF_WAYNE, g_perf_count), g_perf_max[MST_PERF_WAYNE]);
   PrintFormat("MST_PERF_SUM_B refatr=%.1f/%I64u collect=%.1f/%I64u normalize=%.1f/%I64u dbscan=%.1f/%I64u nodes=%.1f/%I64u lifecycle=%.1f/%I64u hash=%.1f/%I64u logger=%.1f/%I64u renders=%I64u render=%.1f/%I64u delete=%.1f/%I64u draw=%.1f/%I64u redraw=%.1f/%I64u",
               PerformanceMean(MST_PERF_REFERENCE_ATR, g_perf_count), g_perf_max[MST_PERF_REFERENCE_ATR],
               PerformanceMean(MST_PERF_COLLECT, g_perf_count), g_perf_max[MST_PERF_COLLECT],
               PerformanceMean(MST_PERF_NORMALIZE, g_perf_count), g_perf_max[MST_PERF_NORMALIZE],
               PerformanceMean(MST_PERF_DBSCAN, g_perf_count), g_perf_max[MST_PERF_DBSCAN],
               PerformanceMean(MST_PERF_NODE_BUILD, g_perf_count), g_perf_max[MST_PERF_NODE_BUILD],
               PerformanceMean(MST_PERF_LIFECYCLE, g_perf_count), g_perf_max[MST_PERF_LIFECYCLE],
               PerformanceMean(MST_PERF_HASH, g_perf_count), g_perf_max[MST_PERF_HASH],
               PerformanceMean(MST_PERF_LOGGER, g_perf_count), g_perf_max[MST_PERF_LOGGER],
               g_perf_render_count,
               PerformanceMean(MST_PERF_RENDER, render_divisor), g_perf_max[MST_PERF_RENDER],
               PerformanceMean(MST_PERF_DELETE, render_divisor), g_perf_max[MST_PERF_DELETE],
               PerformanceMean(MST_PERF_DRAW, render_divisor), g_perf_max[MST_PERF_DRAW],
               PerformanceMean(MST_PERF_REDRAW, render_divisor), g_perf_max[MST_PERF_REDRAW]);
   ResetPerformanceWindow();
}

void PublishBuffers(const MST_ControllerReading &reading)
{
   // Indicator buffers are terminal-sized by the first OnCalculate call.
   // A timer or synchronous initialization may arrive before that allocation,
   // particularly during timeframe changes or sparse higher-timeframe loads.
   if(ArraySize(NearestNodeBuffer) < 1 || ArraySize(LowerNodeBuffer) < 1 ||
      ArraySize(UpperNodeBuffer) < 1 || ArraySize(NodeCountBuffer) < 1)
      return;
   double nearest = EMPTY_VALUE;
   double lower = EMPTY_VALUE;
   double upper = EMPTY_VALUE;
   g_master.FindNearestNodes(reading.reference_price, nearest, lower, upper);
   NearestNodeBuffer[0] = nearest;
   LowerNodeBuffer[0] = lower;
   UpperNodeBuffer[0] = upper;
   NodeCountBuffer[0] = (double)reading.node_count;
}

bool RefreshMaster(const bool force_heavy,
                   const bool force_profile,
                   const bool allow_cached_structure = false)
{
   if(g_tester_cutoff_finalized) return true;
   if(g_refresh_in_progress) return true;
   if(g_is_tester && InpResearchWindowStart > 0)
   {
      datetime calculation_bar = iTime(_Symbol, InpMasterTimeframe, 0);
      if(calculation_bar <= 0 || calculation_bar < InpResearchWindowStart)
         return true;
   }
   if(!g_ready) return false;
   g_refresh_in_progress = true;
   bool updated = g_master.Update(force_heavy, force_profile, allow_cached_structure);
   g_refresh_in_progress = false;
   if(!updated)
      return false;
   MST_ControllerReading reading;
   g_master.GetReading(reading);
   if(g_is_tester && reading.calculation_bar_time > g_replay_checkpoint_bar)
   {
      g_replay_checkpoint_bar = reading.calculation_bar_time;
      g_replay_checkpoint_bars++;
      if((g_replay_checkpoint_bars % 100) == 0)
         PrintFormat("MST_AUCTION_CHECKPOINT bars=%d hash=%I64u sequence=%I64u attempts=%d episodes=%d transits=%d",
                     g_replay_checkpoint_bars, reading.auction_terminal_hash,
                     reading.auction_event_sequence, reading.active_attempt_count,
                     reading.active_episode_count, reading.active_transit_count);
   }
   if(g_is_tester && InpTesterFinalizeAt > 0 &&
      reading.calculation_bar_time >= InpTesterFinalizeAt)
   {
      g_master.Finalize(10001);
      g_tester_cutoff_finalized = true;
      return true;
   }
   if(!reading.valid) return false;
   PublishBuffers(reading);
   MST_RenderPerformance rendering;
   ZeroMemory(rendering);
   if(!g_has_rendered_snapshot || reading.render_generation != g_rendered_generation)
   {
      RenderSnapshot(reading, rendering);
      g_rendered_generation = reading.render_generation;
      g_has_rendered_snapshot = true;
   }
   RecordAndReportPerformance(reading, rendering);
   return true;
}

int OnInit(void)
{
   g_ready = false;
   g_is_tester = (bool)(MQLInfoInteger(MQL_TESTER) || MQLInfoInteger(MQL_OPTIMIZATION));
   g_refresh_in_progress = false;
   g_live_primed = false;
   g_live_next_prime_ms = 0;
   g_live_prime_failures = 0;
   g_owner_prefix = "MST_" + InpInstanceTag + "_" + _Symbol + "_";
   g_prefix = g_owner_prefix + IntegerToString(_Period) + "_";
#ifdef MST_VISUAL_ONLY
   // One-time migration cleanup for chart objects stranded by controller
   // builds that used timeframe-specific MASTER ownership.
   ObjectsDeleteAll(0, "MST_MASTER_" + _Symbol + "_");
#endif
   DeleteOwnedObjects();
   if(!ValidConfiguredTimeframes())
      return INIT_PARAMETERS_INCORRECT;

   SetupHiddenBuffer(0, NearestNodeBuffer, "Nearest Structural Node");
   SetupHiddenBuffer(1, LowerNodeBuffer, "Lower Structural Node");
   SetupHiddenBuffer(2, UpperNodeBuffer, "Upper Structural Node");
   SetupHiddenBuffer(3, NodeCountBuffer, "Structural Node Count");
   IndicatorSetInteger(INDICATOR_DIGITS, _Digits);
#ifdef MST_VISUAL_ONLY
   IndicatorSetString(INDICATOR_SHORTNAME, "Master Structure Map");
#else
   IndicatorSetString(INDICATOR_SHORTNAME, "Master Structure Controller");
#endif

   MST_ControllerConfig config;
   MST_DefaultControllerConfig(config);
   config.master_timeframe = InpMasterTimeframe;
   config.volkitt_timeframe = InpVolKittTimeframe;
   config.profile_timeframe = InpProfileTimeframe;
   config.day_swings_timeframe = InpDaySwingsTimeframe;
   config.wayne_timeframe = InpWayneTimeframe;
   config.atr_period = InpATRPeriod;
   config.instance_tag = InpInstanceTag;
   config.enable_logging = InpEnableLogging;
   config.enable_parity_oracle = InpEnableParityOracle;
   config.log_flush_interval = InpLogFlushSnapshots;
   config.deterministic_terminal_time = InpTesterFinalizeAt;
   config.canonical_instrument = InpCanonicalInstrument;
   config.data_source_id = InpDataSourceId;
   config.data_fingerprint = InpDataFingerprint;
   config.research_window_start = InpResearchWindowStart;
   config.research_window_end = InpResearchWindowEnd;

   config.volkitt.lookback = InpVolKittLookback;
   config.volkitt.clusters = InpVolKittClusters;
   config.volkitt.iterations = InpVolKittIterations;
   config.volkitt.atr_period = InpATRPeriod;
   config.volkitt.enable_profile = InpEnableProfile;
   config.volkitt.profiles_to_keep = InpProfilesToKeep;
   config.volkitt.enable_single_prints = InpEnableSinglePrints;
   config.volkitt.live_heavy_refresh_ms = InpTimerMilliseconds;
   config.day_swings.days_to_keep = InpDaysToKeep;
   config.wayne.periods_to_keep = InpWaynePeriodsToKeep;
   config.wayne.include_standard_pivots = InpWayneStandardPivots;
   config.wayne.include_m_pivots = InpWayneMidPivots;
   config.wayne.include_zones = InpWayneZones;

   config.clustering.epsilon_atr = InpEpsilonATR;
   config.clustering.min_samples = InpMinimumSamples;
   config.clustering.max_levels = InpMaximumLevels;
   config.clustering.max_nodes = InpMaximumNodes;
   config.clustering.use_interval_distance = InpUseIntervalDistance;
   config.clustering.center_role_tolerance_atr = InpCenterRoleToleranceATR;
   config.compatibility.require_role_compatibility = InpRequireRoleCompatibility;
   config.lifecycle.match_distance_atr = InpNodeMatchDistanceATR;
   config.lifecycle.retire_after_rebuilds = InpRetireAfterRebuilds;
   config.auction.enabled = InpEnableAuctionEngine;
   config.auction.approach_radius_atr = InpApproachDistanceATR;
   config.auction.rejection_min_excursion_atr = InpRejectionDistanceATR;
   config.auction.break_buffer_atr = InpBreakBufferATR;
   config.auction.acceptance_bars = InpAcceptanceCloses;
   config.auction.acceptance_min_distance_atr = InpAcceptanceDistanceATR;
   config.auction.reclaim_tolerance_atr = InpReclaimToleranceATR;
   config.auction.departure_distance_atr = InpDepartureDistanceATR;
   config.auction.max_attempt_bars = InpMaxAttemptBars;
   config.auction.episode_gap_bars = InpEpisodeGapBars;

   g_pending_config = config;
   if(g_is_tester)
   {
      if(!g_master.Init(_Symbol, g_pending_config))
         return INIT_FAILED;
      g_ready = true;
   }
   g_rendered_generation = 0;
   g_has_rendered_snapshot = false;
   g_replay_checkpoint_bar = 0;
   g_replay_checkpoint_bars = 0;
   g_tester_cutoff_finalized = false;
   ArrayResize(g_rendered_node_ids, 0);
   g_perf_first_frame_pending = true;
   ResetPerformanceWindow();

   if(!g_is_tester)
   {
      uint timer_ms = (uint)MathMin(MathMax((int)InpTimerMilliseconds, 100), 5000);
      if(!EventSetMillisecondTimer(timer_ms))
      {
         g_master.Deinit();
         g_ready = false;
         return INIT_FAILED;
      }
   }
#ifdef MST_VISUAL_ONLY
   SetMapBootStatus("MAP LOADING STRUCTURE");
#endif
   // Live charts initialize asynchronously on the timer. Performing the full
   // producer/profile/DBSCAN rebuild inside OnInit blocks chart loading and
   // multiplies the load when the visual producer indicators are also present.
   // Tester semantics remain unchanged and continue to prime synchronously.
   if(g_is_tester)
      RefreshMaster(true, true, false);
   return INIT_SUCCEEDED;
}

void OnDeinit(const int reason)
{
   g_ready = false;
   EventKillTimer();
   // Visual ownership is released first. Dataset finalization must never be
   // able to strand chart objects when an indicator is removed or reloaded.
   DeleteOwnedObjects();
   MST_ControllerReading reading;
   g_master.GetReading(reading);
   if(g_is_tester)
      PrintFormat("MST_AUCTION_FINAL bars=%d hash=%I64u sequence=%I64u reason=%d",
                  g_replay_checkpoint_bars, reading.auction_terminal_hash,
                  reading.auction_event_sequence, reason);
   g_master.Deinit();
   g_refresh_in_progress = false;
}

void OnTimer(void)
{
   if(g_is_tester)
      return;

   // Keep the chart-loading path cheap. Producer handles and research identity
   // can require broker history synchronization, so live initialization owns
   // the same bounded timer/backoff lane as the first structural snapshot.
   if(!g_ready)
   {
      ulong init_now_ms = GetTickCount64();
      if(init_now_ms < g_live_next_prime_ms)
         return;
      if(!g_master.Init(_Symbol, g_pending_config))
      {
         g_live_prime_failures++;
         ulong init_retry_ms = (ulong)MathMin(2000 * g_live_prime_failures, 10000);
         g_live_next_prime_ms = init_now_ms + init_retry_ms;
#ifdef MST_VISUAL_ONLY
         SetMapBootStatus(StringFormat("MAP WAITING FOR PRODUCERS (%d)",
                                      g_live_prime_failures));
#endif
         return;
      }
      g_ready = true;
      g_live_next_prime_ms = 0;
      g_live_prime_failures = 0;
   }

   // Live initialization must not block OnInit, but a fresh controller still
   // needs one complete structural snapshot before the cached fast path can
   // take ownership. History synchronization can be transient, so failed
   // primes back off instead of hammering the chart thread every 750 ms.
   if(!g_live_primed)
   {
      ulong now_ms = GetTickCount64();
      if(now_ms < g_live_next_prime_ms)
         return;

      if(RefreshMaster(true, true, false))
      {
         g_live_primed = true;
         g_live_prime_failures = 0;
         ObjectDelete(0, g_prefix + "BOOT");
         Print("MST LIVE READY: initial structural snapshot published");
         return;
      }

      g_live_prime_failures++;
      ulong retry_ms = (ulong)MathMin(2000 * g_live_prime_failures, 10000);
      g_live_next_prime_ms = now_ms + retry_ms;
#ifdef MST_VISUAL_ONLY
      SetMapBootStatus(StringFormat("MAP WAITING FOR HISTORY (%d)", g_live_prime_failures));
#endif
      if(g_live_prime_failures == 1 || (g_live_prime_failures % 5) == 0)
         PrintFormat("MST LIVE WAIT: producer history not ready; retry=%dms failures=%d",
                     (int)retry_ms, g_live_prime_failures);
      return;
   }
   RefreshMaster(false, false, true);
}

int OnCalculate(const int rates_total,
                const int prev_calculated,
                const datetime &time[],
                const double &open[],
                const double &high[],
                const double &low[],
                const double &close[],
                const long &tick_volume[],
                const long &volume[],
                const int &spread[])
{
   if(rates_total <= 0) return 0;
   if(prev_calculated == 0)
   {
      ArrayInitialize(NearestNodeBuffer, EMPTY_VALUE);
      ArrayInitialize(LowerNodeBuffer, EMPTY_VALUE);
      ArrayInitialize(UpperNodeBuffer, EMPTY_VALUE);
      ArrayInitialize(NodeCountBuffer, 0.0);
   }
   // Live ownership belongs exclusively to OnTimer. Running the same embedded
   // producers once per tick and again on the timer starves charts that also
   // host their visual indicator counterparts, especially on fast timeframes.
   if(g_is_tester)
      RefreshMaster(false, false, InpTesterCacheUnchangedStructure);
   return rates_total;
}
