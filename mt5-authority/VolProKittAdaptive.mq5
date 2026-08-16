//+------------------------------------------------------------------+
//| VolProKittAdaptive.mq5                                          |
//| Deterministic self-calibrating descendant of VolProKitt2.mq5.   |
//| Original geometry lineage: CC BY-NC-SA 4.0, LuxAlgo port.       |
//| https://creativecommons.org/licenses/by-nc-sa/4.0/              |
//+------------------------------------------------------------------+
#property copyright "CC BY-NC-SA 4.0 - Adaptive Northstar descendant"
#property link      "https://creativecommons.org/licenses/by-nc-sa/4.0/"
#property version   "1.00"
#property strict
#property indicator_chart_window
#property indicator_buffers 10
#property indicator_plots   10
#property tester_everytick_calculate

#include "VolProKittAdaptiveCore.mqh"

input ENUM_TIMEFRAMES InpCalcTF = PERIOD_CURRENT;

const string KVA_PREFIX_BASE = "VolProKittAdaptive_";
const int KVA_VP_WIDTH_BARS = 40;
const int KVA_VP_OFFSET_BARS = 10;
const int KVA_MAX_DOTS = 180;

double KVA_Buffer0[], KVA_Buffer1[], KVA_Buffer2[], KVA_Buffer3[], KVA_Buffer4[];
double KVA_Buffer5[], KVA_Buffer6[], KVA_Buffer7[], KVA_Buffer8[], KVA_Buffer9[];

color KVA_PALETTE[KVA_MAX_CLUSTERS];
string g_kva_prefix = KVA_PREFIX_BASE;
ENUM_TIMEFRAMES g_kva_calc_tf = PERIOD_CURRENT;
int g_kva_chart_seconds = 60;
int g_kva_calc_seconds = 60;
datetime g_kva_last_closed_bar = 0;
ulong g_kva_frames = 0;

KVA_PROFILE g_kva_active;
KVA_PROFILE g_kva_pending;
int g_kva_pending_observations = 0;
KVA_CLUSTER g_kva_previous[];
KVA_LEVEL_MEMORY g_kva_levels[KVA_MAX_CLUSTERS];

string KVA_TimeframeLabel(const ENUM_TIMEFRAMES timeframe)
{
   if(timeframe == PERIOD_CURRENT) return "CURRENT";
   string value = EnumToString(timeframe);
   if(StringFind(value, "PERIOD_") == 0)
      return StringSubstr(value, 7);
   return value;
}

ENUM_TIMEFRAMES KVA_ResolveTimeframe()
{
   if(InpCalcTF == PERIOD_CURRENT)
      return (ENUM_TIMEFRAMES)_Period;
   return InpCalcTF;
}

void KVA_SetupBuffer(const int index, double &buffer[], const string label)
{
   SetIndexBuffer(index, buffer, INDICATOR_DATA);
   ArraySetAsSeries(buffer, true);
   PlotIndexSetInteger(index, PLOT_DRAW_TYPE, DRAW_NONE);
   PlotIndexSetInteger(index, PLOT_SHOW_DATA, true);
   PlotIndexSetDouble(index, PLOT_EMPTY_VALUE, EMPTY_VALUE);
   PlotIndexSetString(index, PLOT_LABEL, label);
}

void KVA_ClearCurrentBuffers(const bool clear_all)
{
   if(clear_all)
   {
      ArrayInitialize(KVA_Buffer0, EMPTY_VALUE);
      ArrayInitialize(KVA_Buffer1, EMPTY_VALUE);
      ArrayInitialize(KVA_Buffer2, EMPTY_VALUE);
      ArrayInitialize(KVA_Buffer3, EMPTY_VALUE);
      ArrayInitialize(KVA_Buffer4, EMPTY_VALUE);
      ArrayInitialize(KVA_Buffer5, EMPTY_VALUE);
      ArrayInitialize(KVA_Buffer6, EMPTY_VALUE);
      ArrayInitialize(KVA_Buffer7, EMPTY_VALUE);
      ArrayInitialize(KVA_Buffer8, EMPTY_VALUE);
      ArrayInitialize(KVA_Buffer9, EMPTY_VALUE);
      return;
   }
   KVA_Buffer0[0] = EMPTY_VALUE;
   KVA_Buffer1[0] = EMPTY_VALUE;
   KVA_Buffer2[0] = EMPTY_VALUE;
   KVA_Buffer3[0] = EMPTY_VALUE;
   KVA_Buffer4[0] = EMPTY_VALUE;
   KVA_Buffer5[0] = EMPTY_VALUE;
   KVA_Buffer6[0] = EMPTY_VALUE;
   KVA_Buffer7[0] = EMPTY_VALUE;
   KVA_Buffer8[0] = EMPTY_VALUE;
   KVA_Buffer9[0] = EMPTY_VALUE;
}

void KVA_SetObjectBuffer(const int object_id, const double price)
{
   if(object_id == 0) KVA_Buffer0[0] = price;
   else if(object_id == 1) KVA_Buffer1[0] = price;
   else if(object_id == 2) KVA_Buffer2[0] = price;
   else if(object_id == 3) KVA_Buffer3[0] = price;
   else if(object_id == 4) KVA_Buffer4[0] = price;
   else if(object_id == 5) KVA_Buffer5[0] = price;
   else if(object_id == 6) KVA_Buffer6[0] = price;
   else if(object_id == 7) KVA_Buffer7[0] = price;
   else if(object_id == 8) KVA_Buffer8[0] = price;
   else if(object_id == 9) KVA_Buffer9[0] = price;
}

void KVA_DeleteObjects()
{
   ObjectsDeleteAll(0, g_kva_prefix);
}

color KVA_MixWithBackground(const color source, const double fade)
{
   color background = (color)ChartGetInteger(0, CHART_COLOR_BACKGROUND);
   double factor = KVA_Clamp(fade, 0.0, 1.0);
   int sr = (int)(source & 0xFF);
   int sg = (int)((source >> 8) & 0xFF);
   int sb = (int)((source >> 16) & 0xFF);
   int br = (int)(background & 0xFF);
   int bg = (int)((background >> 8) & 0xFF);
   int bb = (int)((background >> 16) & 0xFF);
   int red = (int)(sr * (1.0 - factor) + br * factor);
   int green = (int)(sg * (1.0 - factor) + bg * factor);
   int blue = (int)(sb * (1.0 - factor) + bb * factor);
   return (color)((blue << 16) | (green << 8) | red);
}

color KVA_ReadableText(const color preferred)
{
   color background = (color)ChartGetInteger(0, CHART_COLOR_BACKGROUND);
   int pr = (int)(preferred & 0xFF);
   int pg = (int)((preferred >> 8) & 0xFF);
   int pb = (int)((preferred >> 16) & 0xFF);
   int br = (int)(background & 0xFF);
   int bg = (int)((background >> 8) & 0xFF);
   int bb = (int)((background >> 16) & 0xFF);
   int distance = MathAbs(pr - br) + MathAbs(pg - bg) + MathAbs(pb - bb);
   if(distance >= 180)
      return preferred;
   int luma = (br * 30 + bg * 59 + bb * 11) / 100;
   return luma < 128 ? clrWhite : clrBlack;
}

void KVA_UpsertRectangle(const string name,
                         const datetime left,
                         const double top,
                         const datetime right,
                         const double bottom,
                         const color value,
                         const bool filled,
                         const bool behind)
{
   if(ObjectFind(0, name) < 0)
      ObjectCreate(0, name, OBJ_RECTANGLE, 0, left, top, right, bottom);
   else
   {
      ObjectMove(0, name, 0, left, top);
      ObjectMove(0, name, 1, right, bottom);
   }
   ObjectSetInteger(0, name, OBJPROP_COLOR, value);
   ObjectSetInteger(0, name, OBJPROP_FILL, filled);
   ObjectSetInteger(0, name, OBJPROP_BACK, behind);
   ObjectSetInteger(0, name, OBJPROP_HIDDEN, true);
   ObjectSetInteger(0, name, OBJPROP_SELECTABLE, false);
}

void KVA_UpsertLine(const string name,
                    const datetime left,
                    const datetime right,
                    const double price,
                    const color value,
                    const ENUM_LINE_STYLE style,
                    const int width)
{
   if(ObjectFind(0, name) < 0)
      ObjectCreate(0, name, OBJ_TREND, 0, left, price, right, price);
   else
   {
      ObjectMove(0, name, 0, left, price);
      ObjectMove(0, name, 1, right, price);
   }
   ObjectSetInteger(0, name, OBJPROP_RAY_RIGHT, false);
   ObjectSetInteger(0, name, OBJPROP_COLOR, value);
   ObjectSetInteger(0, name, OBJPROP_STYLE, style);
   ObjectSetInteger(0, name, OBJPROP_WIDTH, width);
   ObjectSetInteger(0, name, OBJPROP_BACK, true);
   ObjectSetInteger(0, name, OBJPROP_HIDDEN, true);
   ObjectSetInteger(0, name, OBJPROP_SELECTABLE, false);
}

void KVA_UpsertText(const string name,
                    const datetime time,
                    const double price,
                    const string text,
                    const color value,
                    const ENUM_ANCHOR_POINT anchor)
{
   if(ObjectFind(0, name) < 0)
      ObjectCreate(0, name, OBJ_TEXT, 0, time, price);
   else
      ObjectMove(0, name, 0, time, price);
   ObjectSetString(0, name, OBJPROP_TEXT, text);
   ObjectSetInteger(0, name, OBJPROP_COLOR, value);
   ObjectSetInteger(0, name, OBJPROP_FONTSIZE, 8);
   ObjectSetInteger(0, name, OBJPROP_ANCHOR, anchor);
   ObjectSetInteger(0, name, OBJPROP_HIDDEN, true);
   ObjectSetInteger(0, name, OBJPROP_SELECTABLE, false);
}

void KVA_UpsertDot(const string name,
                   const datetime time,
                   const double price,
                   const color value)
{
   if(ObjectFind(0, name) < 0)
      ObjectCreate(0, name, OBJ_ARROW, 0, time, price);
   else
      ObjectMove(0, name, 0, time, price);
   ObjectSetInteger(0, name, OBJPROP_ARROWCODE, 159);
   ObjectSetInteger(0, name, OBJPROP_COLOR, value);
   ObjectSetInteger(0, name, OBJPROP_WIDTH, 2);
   ObjectSetInteger(0, name, OBJPROP_BACK, true);
   ObjectSetInteger(0, name, OBJPROP_HIDDEN, true);
   ObjectSetInteger(0, name, OBJPROP_SELECTABLE, false);
}

void KVA_LevelVisual(const KVA_LEVEL_STATE state,
                     const color base,
                     color &line_color,
                     ENUM_LINE_STYLE &style,
                     int &width)
{
   line_color = base;
   style = STYLE_DASH;
   width = 2;
   if(state == KVA_LEVEL_FRESH)
      style = STYLE_SOLID;
   else if(state == KVA_LEVEL_ACCEPTED)
   {
      style = STYLE_SOLID;
      width = 4;
   }
   else if(state == KVA_LEVEL_REJECTED)
   {
      style = STYLE_DASHDOT;
      width = 3;
   }
   else if(state == KVA_LEVEL_BROKEN)
   {
      style = STYLE_DASHDOTDOT;
      width = 4;
      line_color = KVA_ReadableText(base);
   }
   else if(state == KVA_LEVEL_RECLAIMED)
   {
      style = STYLE_DASHDOTDOT;
      width = 3;
   }
}

int KVA_NearestCluster(const double price,
                       const KVA_CLUSTER &clusters[],
                       const int count)
{
   int nearest = 0;
   double distance = DBL_MAX;
   for(int index = 0; index < count; index++)
   {
      double candidate = MathAbs(price - clusters[index].centroid);
      if(candidate < distance)
      {
         distance = candidate;
         nearest = index;
      }
   }
   return nearest;
}

double KVA_RowVolume(const MqlRates &rates[],
                     const KVA_PROFILE &profile,
                     const KVA_CLUSTER &clusters[],
                     const int cluster_index,
                     const int row)
{
   const KVA_CLUSTER cluster = clusters[cluster_index];
   double bottom = cluster.low + row * cluster.bin_size;
   double top = bottom + cluster.bin_size;
   double result = 0.0;
   for(int index = 1; index <= profile.lookback; index++)
   {
      double price = (rates[index].high + rates[index].low + rates[index].close) / 3.0;
      if(KVA_NearestCluster(price, clusters, profile.clusters) != cluster_index)
         continue;
      double span = MathMax(rates[index].high - rates[index].low, _Point);
      double overlap = MathMin(rates[index].high, top)
                     - MathMax(rates[index].low, bottom);
      if(overlap > 0.0)
         result += KVA_BarVolume(rates[index]) * overlap / span;
   }
   return result;
}

void KVA_DrawClusterProfile(const MqlRates &rates[],
                            const KVA_PROFILE &profile,
                            const KVA_CLUSTER &clusters[],
                            const int cluster_index,
                            const datetime line_start,
                            const datetime profile_start)
{
   const KVA_CLUSTER cluster = clusters[cluster_index];
   int object_id = cluster.object_id;
   if(object_id < 0 || object_id >= KVA_MAX_CLUSTERS)
      return;
   color base = KVA_PALETTE[object_id];
   color faded = KVA_MixWithBackground(base, 0.84);

   double row_volumes[];
   ArrayResize(row_volumes, cluster.rows);
   double maximum = 0.0;
   for(int row = 0; row < cluster.rows; row++)
   {
      row_volumes[row] = KVA_RowVolume(rates, profile, clusters, cluster_index, row);
      maximum = MathMax(maximum, row_volumes[row]);
   }

   for(int row = 0; row < cluster.rows; row++)
   {
      if(row_volumes[row] <= 0.0)
         continue;
      double bottom = cluster.low + row * cluster.bin_size;
      double top = bottom + cluster.bin_size;
      int width_bars = maximum > 0.0
                     ? (int)MathRound(row_volumes[row] / maximum * KVA_VP_WIDTH_BARS)
                     : 1;
      datetime right = profile_start
                     + (datetime)(MathMax(width_bars, 1) * g_kva_chart_seconds);
      bool is_poc = cluster.poc >= bottom && cluster.poc <= top;
      color value = is_poc ? KVA_MixWithBackground(base, 0.15) : faded;
      string name = g_kva_prefix + "VP_" + IntegerToString(object_id)
                  + "_" + IntegerToString(row);
      KVA_UpsertRectangle(name, profile_start, top, right, bottom,
                          value, true, true);
   }

   KVA_LEVEL_MEMORY level = g_kva_levels[object_id];
   color line_color;
   ENUM_LINE_STYLE line_style;
   int line_width;
   KVA_LevelVisual(level.status, base, line_color, line_style, line_width);
   datetime line_end = profile_start
                     + (datetime)(KVA_VP_WIDTH_BARS * g_kva_chart_seconds);
   KVA_UpsertLine(g_kva_prefix + "POC_" + IntegerToString(object_id),
                  line_start, line_end, cluster.poc,
                  line_color, line_style, line_width);

   string state = StringFormat(
      "OBJ:%d %s | rows:%d mass:%.1f%% | Age:%d T:%d A:%d",
      object_id, KVA_LevelStateName(level.status), cluster.rows,
      cluster.mass_share * 100.0, level.age_bars,
      level.touch_count, level.bars_accepted);
   string velocity = StringFormat("%s | %s | %s",
                                  level.regime_dir,
                                  level.regime_auc,
                                  level.regime_vol);
   KVA_UpsertText(g_kva_prefix + "State_" + IntegerToString(object_id),
                  line_end, cluster.poc, state,
                  KVA_ReadableText(base), ANCHOR_LEFT_LOWER);
   KVA_UpsertText(g_kva_prefix + "Velocity_" + IntegerToString(object_id),
                  line_end, cluster.poc, velocity,
                  KVA_ReadableText(base), ANCHOR_LEFT_UPPER);
}

void KVA_DrawDots(const MqlRates &rates[],
                  const KVA_PROFILE &profile,
                  const KVA_CLUSTER &clusters[])
{
   int stride = MathMax(profile.lookback / KVA_MAX_DOTS, 1);
   int dot = 0;
   for(int index = 1; index <= profile.lookback && dot < KVA_MAX_DOTS; index += stride)
   {
      double price = (rates[index].high + rates[index].low + rates[index].close) / 3.0;
      int cluster_index = KVA_NearestCluster(price, clusters, profile.clusters);
      int object_id = clusters[cluster_index].object_id;
      if(object_id < 0 || object_id >= KVA_MAX_CLUSTERS)
         continue;
      KVA_UpsertDot(g_kva_prefix + "Dot_" + IntegerToString(dot),
                    rates[index].time, price, KVA_PALETTE[object_id]);
      dot++;
   }
}

void KVA_CopyClusters(const KVA_CLUSTER &source[],
                      const int count,
                      KVA_CLUSTER &destination[])
{
   ArrayResize(destination, count);
   for(int i = 0; i < count; i++)
      destination[i] = source[i];
}

int OnInit()
{
   KVA_PALETTE[0] = clrTeal;
   KVA_PALETTE[1] = clrDodgerBlue;
   KVA_PALETTE[2] = clrYellow;
   KVA_PALETTE[3] = clrRed;
   KVA_PALETTE[4] = clrDarkOrchid;
   KVA_PALETTE[5] = C'0,188,212';
   KVA_PALETTE[6] = C'255,235,59';
   KVA_PALETTE[7] = C'233,30,99';
   KVA_PALETTE[8] = C'121,85,72';
   KVA_PALETTE[9] = C'96,125,139';

   g_kva_calc_tf = KVA_ResolveTimeframe();
   g_kva_chart_seconds = MathMax(PeriodSeconds((ENUM_TIMEFRAMES)_Period), 1);
   g_kva_calc_seconds = MathMax(PeriodSeconds(g_kva_calc_tf), 1);
   g_kva_prefix = KVA_PREFIX_BASE + KVA_TimeframeLabel(g_kva_calc_tf) + "_";
   ZeroMemory(g_kva_active);
   ZeroMemory(g_kva_pending);
   for(int i = 0; i < KVA_MAX_CLUSTERS; i++)
      KVA_ResetLevel(g_kva_levels[i], i);

   IndicatorSetInteger(INDICATOR_DIGITS, _Digits);
   IndicatorSetString(INDICATOR_SHORTNAME,
      "VolProKitt Adaptive [" + KVA_TimeframeLabel(g_kva_calc_tf) + "]");
   KVA_SetupBuffer(0, KVA_Buffer0, "Adaptive Object 0 POC");
   KVA_SetupBuffer(1, KVA_Buffer1, "Adaptive Object 1 POC");
   KVA_SetupBuffer(2, KVA_Buffer2, "Adaptive Object 2 POC");
   KVA_SetupBuffer(3, KVA_Buffer3, "Adaptive Object 3 POC");
   KVA_SetupBuffer(4, KVA_Buffer4, "Adaptive Object 4 POC");
   KVA_SetupBuffer(5, KVA_Buffer5, "Adaptive Object 5 POC");
   KVA_SetupBuffer(6, KVA_Buffer6, "Adaptive Object 6 POC");
   KVA_SetupBuffer(7, KVA_Buffer7, "Adaptive Object 7 POC");
   KVA_SetupBuffer(8, KVA_Buffer8, "Adaptive Object 8 POC");
   KVA_SetupBuffer(9, KVA_Buffer9, "Adaptive Object 9 POC");
   KVA_DeleteObjects();
   return INIT_SUCCEEDED;
}

void OnDeinit(const int reason)
{
   KVA_DeleteObjects();
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
   if(rates_total < 2)
      return 0;
   ArraySetAsSeries(time, true);

   MqlRates source[];
   int copied = CopyRates(_Symbol, g_kva_calc_tf, 0, KVA_MAX_LOOKBACK + 2, source);
   if(copied < KVA_MIN_LOOKBACK + 2)
      return prev_calculated;
   ArraySetAsSeries(source, true);
   datetime closed_bar_time = source[1].time;
   if(closed_bar_time == g_kva_last_closed_bar)
      return rates_total;
   g_kva_last_closed_bar = closed_bar_time;

   KVA_PROFILE candidate;
   KVA_BuildCandidateProfile(source, copied, g_kva_active, candidate);
   bool committed = KVA_ConsiderProfile(candidate, closed_bar_time,
                                        g_kva_active, g_kva_pending,
                                        g_kva_pending_observations);
   if(!g_kva_active.valid || copied < g_kva_active.lookback + 2)
      return prev_calculated;

   KVA_CLUSTER clusters[];
   if(!KVA_FitClusters(source, g_kva_active, clusters))
      return prev_calculated;
   KVA_AssignStableObjects(g_kva_previous, ArraySize(g_kva_previous),
                           clusters, ArraySize(clusters));

   bool object_present[KVA_MAX_CLUSTERS];
   ArrayInitialize(object_present, false);
   for(int cluster_index = 0; cluster_index < ArraySize(clusters); cluster_index++)
   {
      int object_id = clusters[cluster_index].object_id;
      if(object_id < 0 || object_id >= KVA_MAX_CLUSTERS)
         continue;
      object_present[object_id] = true;
      KVA_UpdateLevel(g_kva_levels[object_id], clusters[cluster_index],
                      source[0], source[1],
                      g_kva_active.velocity_bars, g_kva_calc_seconds);
   }
   for(int object_id = 0; object_id < KVA_MAX_CLUSTERS; object_id++)
   {
      if(!object_present[object_id])
         g_kva_levels[object_id].valid = false;
   }

   KVA_ClearCurrentBuffers(prev_calculated == 0);
   KVA_DeleteObjects();
   datetime profile_start = time[0]
                           + (datetime)(KVA_VP_OFFSET_BARS * g_kva_chart_seconds);
   datetime line_start = source[g_kva_active.lookback].time;
   for(int cluster_index = 0; cluster_index < ArraySize(clusters); cluster_index++)
   {
      int object_id = clusters[cluster_index].object_id;
      if(object_id >= 0 && object_id < KVA_MAX_CLUSTERS)
         KVA_SetObjectBuffer(object_id, clusters[cluster_index].poc);
      KVA_DrawClusterProfile(source, g_kva_active, clusters,
                             cluster_index, line_start, profile_start);
   }
   KVA_DrawDots(source, g_kva_active, clusters);
   KVA_CopyClusters(clusters, ArraySize(clusters), g_kva_previous);
   g_kva_frames++;

   string short_name = StringFormat(
      "VolProKitt Adaptive [%s] L%d K%d V%d G%I64u C%.2f",
      KVA_TimeframeLabel(g_kva_calc_tf), g_kva_active.lookback,
      g_kva_active.clusters, g_kva_active.velocity_bars,
      g_kva_active.generation, g_kva_active.confidence);
   IndicatorSetString(INDICATOR_SHORTNAME, short_name);
   if(committed)
   {
      PrintFormat("KVA_COMMIT symbol=%s tf=%s generation=%I64u time=%I64d lookback=%d clusters=%d velocity=%d median_range=%.10g volatility=%.10g entropy=%.6f similarity=%.6f score=%.6f confidence=%.6f",
                  _Symbol, KVA_TimeframeLabel(g_kva_calc_tf),
                  g_kva_active.generation, (long)closed_bar_time,
                  g_kva_active.lookback, g_kva_active.clusters,
                  g_kva_active.velocity_bars, g_kva_active.median_range,
                  g_kva_active.realized_volatility,
                  g_kva_active.auction_entropy,
                  g_kva_active.structural_similarity,
                  g_kva_active.cluster_score,
                  g_kva_active.confidence);
   }
   if((g_kva_frames % 32) == 0)
   {
      PrintFormat("KVA_FRAME symbol=%s tf=%s frames=%I64u generation=%I64u lookback=%d clusters=%d velocity=%d objects=%d",
                  _Symbol, KVA_TimeframeLabel(g_kva_calc_tf), g_kva_frames,
                  g_kva_active.generation, g_kva_active.lookback,
                  g_kva_active.clusters, g_kva_active.velocity_bars,
                  ArraySize(clusters));
   }
   ChartRedraw(0);
   return rates_total;
}

//+------------------------------------------------------------------+
