//+------------------------------------------------------------------+
//| WaynePivotStateMachine.mq5                                      |
//| Standalone Wayne pivot observer with end-to-end zone grammar.    |
//| Fibot geometry is intentionally removed.                        |
//+------------------------------------------------------------------+
#property copyright "Northstar standalone observer"
#property version   "1.00"
#property strict
#property indicator_chart_window
#property indicator_buffers 17
#property indicator_plots   17
#property tester_everytick_calculate

#include "WaynePivotStateMachineCore.mqh"

input int             InpCountPeriods = 20;
input ENUM_TIMEFRAMES InpTimePeriod = PERIOD_D1;
input string          InpInstanceID = "E2E";
input int             InpInteractionBufferPoints = 2;
input int             InpAcceptanceBars = 3;
input bool            InpShowFuturePreview = true;
input datetime        InpTesterFinalizeAt = 0;

double g_buffer_s3[], g_buffer_m0[], g_buffer_s2[], g_buffer_m1[], g_buffer_s1[];
double g_buffer_m2[], g_buffer_pp[], g_buffer_m3[], g_buffer_r1[], g_buffer_m4[];
double g_buffer_r2[], g_buffer_m5[], g_buffer_r3[];
double g_buffer_lower_zone_low[], g_buffer_lower_zone_high[];
double g_buffer_upper_zone_low[], g_buffer_upper_zone_high[];

ENUM_TIMEFRAMES g_timeframe = PERIOD_D1;
int g_period_seconds = 86400;
int g_chart_seconds = 60;
string g_prefix = "WaynePivotE2E_";
datetime g_current_period_start = 0;
datetime g_last_market_bar = 0;
ulong g_frame = 0;
bool g_ready = false;
bool g_finalized = false;
WPE2E_CONFIG g_config;
WPE2E_LEVEL g_current_levels[];
CWaynePivotE2ELogger g_logger;

void SetupBuffer(const int index, double &buffer[], const string label)
{
   SetIndexBuffer(index, buffer, INDICATOR_DATA);
   ArraySetAsSeries(buffer, true);
   PlotIndexSetInteger(index, PLOT_DRAW_TYPE, DRAW_NONE);
   PlotIndexSetInteger(index, PLOT_SHOW_DATA, true);
   PlotIndexSetDouble(index, PLOT_EMPTY_VALUE, EMPTY_VALUE);
   PlotIndexSetString(index, PLOT_LABEL, label);
}

void ClearBuffers(void)
{
   ArrayInitialize(g_buffer_s3, EMPTY_VALUE);
   ArrayInitialize(g_buffer_m0, EMPTY_VALUE);
   ArrayInitialize(g_buffer_s2, EMPTY_VALUE);
   ArrayInitialize(g_buffer_m1, EMPTY_VALUE);
   ArrayInitialize(g_buffer_s1, EMPTY_VALUE);
   ArrayInitialize(g_buffer_m2, EMPTY_VALUE);
   ArrayInitialize(g_buffer_pp, EMPTY_VALUE);
   ArrayInitialize(g_buffer_m3, EMPTY_VALUE);
   ArrayInitialize(g_buffer_r1, EMPTY_VALUE);
   ArrayInitialize(g_buffer_m4, EMPTY_VALUE);
   ArrayInitialize(g_buffer_r2, EMPTY_VALUE);
   ArrayInitialize(g_buffer_m5, EMPTY_VALUE);
   ArrayInitialize(g_buffer_r3, EMPTY_VALUE);
   ArrayInitialize(g_buffer_lower_zone_low, EMPTY_VALUE);
   ArrayInitialize(g_buffer_lower_zone_high, EMPTY_VALUE);
   ArrayInitialize(g_buffer_upper_zone_low, EMPTY_VALUE);
   ArrayInitialize(g_buffer_upper_zone_high, EMPTY_VALUE);
}

void SetBufferValue(double &buffer[], const double value)
{
   if(ArraySize(buffer) > 0) buffer[0] = value;
}

void PublishBuffers(const WPE2E_LEVEL &levels[])
{
   ClearBuffers();
   if(ArraySize(levels) < WPE2E_LEVEL_COUNT) return;
   SetBufferValue(g_buffer_s3, levels[0].price);
   SetBufferValue(g_buffer_m0, levels[1].price);
   SetBufferValue(g_buffer_s2, levels[2].price);
   SetBufferValue(g_buffer_m1, levels[3].price);
   SetBufferValue(g_buffer_s1, levels[4].price);
   SetBufferValue(g_buffer_m2, levels[5].price);
   SetBufferValue(g_buffer_pp, levels[6].price);
   SetBufferValue(g_buffer_m3, levels[7].price);
   SetBufferValue(g_buffer_r1, levels[8].price);
   SetBufferValue(g_buffer_m4, levels[9].price);
   SetBufferValue(g_buffer_r2, levels[10].price);
   SetBufferValue(g_buffer_m5, levels[11].price);
   SetBufferValue(g_buffer_r3, levels[12].price);
   SetBufferValue(g_buffer_lower_zone_low, levels[13].lower);
   SetBufferValue(g_buffer_lower_zone_high, levels[13].upper);
   SetBufferValue(g_buffer_upper_zone_low, levels[14].lower);
   SetBufferValue(g_buffer_upper_zone_high, levels[14].upper);
}

color LineColor(const WPE2E_KIND kind)
{
   if(kind == WPE2E_KIND_PP) return clrGold;
   if(kind == WPE2E_KIND_M2) return clrAqua;
   if(kind == WPE2E_KIND_M4) return clrOrange;
   if(kind == WPE2E_KIND_M3 || kind == WPE2E_KIND_M5) return clrMagenta;
   if(kind == WPE2E_KIND_M0 || kind == WPE2E_KIND_M1) return clrLimeGreen;
   if(kind == WPE2E_KIND_LOWER_ZONE) return clrDeepSkyBlue;
   if(kind == WPE2E_KIND_UPPER_ZONE) return clrOrangeRed;
   if(kind == WPE2E_KIND_S3 || kind == WPE2E_KIND_S2 || kind == WPE2E_KIND_S1) return clrLimeGreen;
   return clrRed;
}

ENUM_LINE_STYLE LineStyle(const WPE2E_KIND kind)
{
   if(kind == WPE2E_KIND_PP || kind == WPE2E_KIND_M2 || kind == WPE2E_KIND_M4)
      return STYLE_SOLID;
   return STYLE_DASH;
}

void DeleteObjects(void)
{
   ObjectsDeleteAll(0, g_prefix, -1, -1);
}

void DrawTrend(const string name,
               const datetime first_time,
               const double first_price,
               const datetime second_time,
               const double second_price,
               const color line_color,
               const ENUM_LINE_STYLE style,
               const int width)
{
   if(!ObjectCreate(0, name, OBJ_TREND, 0, first_time, first_price, second_time, second_price))
      return;
   ObjectSetInteger(0, name, OBJPROP_COLOR, line_color);
   ObjectSetInteger(0, name, OBJPROP_STYLE, style);
   ObjectSetInteger(0, name, OBJPROP_WIDTH, width);
   ObjectSetInteger(0, name, OBJPROP_RAY_RIGHT, false);
   ObjectSetInteger(0, name, OBJPROP_BACK, true);
   ObjectSetInteger(0, name, OBJPROP_SELECTABLE, false);
}

void DrawText(const string name,
              const datetime time_value,
              const double price,
              const string text,
              const color text_color)
{
   if(!ObjectCreate(0, name, OBJ_TEXT, 0, time_value, price)) return;
   ObjectSetString(0, name, OBJPROP_TEXT, text);
   ObjectSetString(0, name, OBJPROP_FONT, "Arial");
   ObjectSetInteger(0, name, OBJPROP_FONTSIZE, 8);
   ObjectSetInteger(0, name, OBJPROP_COLOR, text_color);
   ObjectSetInteger(0, name, OBJPROP_ANCHOR, ANCHOR_RIGHT_UPPER);
   ObjectSetInteger(0, name, OBJPROP_SELECTABLE, false);
}

void DrawZone(const string name,
              const datetime first_time,
              const double lower,
              const datetime second_time,
              const double upper,
              const color zone_color)
{
   if(!ObjectCreate(0, name, OBJ_RECTANGLE, 0, first_time, upper, second_time, lower)) return;
   ObjectSetInteger(0, name, OBJPROP_COLOR, zone_color);
   ObjectSetInteger(0, name, OBJPROP_STYLE, STYLE_SOLID);
   ObjectSetInteger(0, name, OBJPROP_WIDTH, 1);
   ObjectSetInteger(0, name, OBJPROP_FILL, true);
   ObjectSetInteger(0, name, OBJPROP_BACK, true);
   ObjectSetInteger(0, name, OBJPROP_SELECTABLE, false);
}

void DrawLevel(const WPE2E_LEVEL &level, const bool labels)
{
   if(!level.valid) return;
   string key = g_prefix + WPE2E_KindName(level.kind) + "_" + IntegerToString(level.period_age);
   if(level.family == WPE2E_FAMILY_ZONE)
   {
      DrawZone(key, level.effective_start, level.lower, level.effective_end,
               level.upper, level.kind == WPE2E_KIND_LOWER_ZONE ? C'0,90,110' : C'110,70,0');
      if(labels)
         DrawText(key + "_LABEL", level.effective_end, level.center,
                  WPE2E_KindName(level.kind) + " " + WPE2E_StateName(level.state), LineColor(level.kind));
      return;
   }
   int width = (level.kind == WPE2E_KIND_PP || level.kind == WPE2E_KIND_M2 || level.kind == WPE2E_KIND_M4) ? 2 : 1;
   DrawTrend(key, level.effective_start, level.price, level.effective_end, level.price,
             LineColor(level.kind), LineStyle(level.kind), width);
   if(labels)
      DrawText(key + "_LABEL", level.effective_end, level.price,
               WPE2E_KindName(level.kind) + " " + WPE2E_StateName(level.state), LineColor(level.kind));
}

void DrawBlock(const WPE2E_PIVOT_BLOCK &block, const bool labels)
{
   if(!block.valid) return;
   WPE2E_LEVEL levels[];
   WPE2E_ExportLevels(block, g_config, levels);
   for(int i = 0; i < ArraySize(levels); i++)
      DrawLevel(levels[i], labels && (block.period_age == 0 || levels[i].family == WPE2E_FAMILY_ZONE));
}

bool BuildCurrentLevels(WPE2E_LEVEL &levels[], WPE2E_PIVOT_BLOCK &block)
{
   MqlRates source[];
   int requested = MathMax(MathMin(InpCountPeriods, WPE2E_MAX_PERIODS), 1);
   int copied = CopyRates(_Symbol, g_timeframe, 1, requested, source);
   if(copied < 1) return false;
   ArraySetAsSeries(source, false);
   int period_seconds = MathMax(PeriodSeconds(g_timeframe), 1);
   WPE2E_CalculateBlock(source[copied - 1], 0, g_timeframe, period_seconds, block);
   if(!block.valid) return false;
   WPE2E_ExportLevels(block, g_config, levels);
   return ArraySize(levels) == WPE2E_LEVEL_COUNT;
}

bool BuildBlockHistory(WPE2E_PIVOT_BLOCK &blocks[])
{
   MqlRates source[];
   int requested = MathMax(MathMin(InpCountPeriods, WPE2E_MAX_PERIODS), 1);
   int copied = CopyRates(_Symbol, g_timeframe, 1, requested, source);
   if(copied < 1) return false;
   ArraySetAsSeries(source, false);
   ArrayResize(blocks, copied);
   for(int age = 0; age < copied; age++)
   {
      int source_index = copied - 1 - age;
      WPE2E_CalculateBlock(source[source_index], age, g_timeframe, g_period_seconds, blocks[age]);
   }
   return true;
}

bool HasPeriodRolled(const datetime period_start)
{
   return period_start > 0 && period_start != g_current_period_start;
}

int EmitCreateEvents(const datetime market_time)
{
   int emitted = 0;
   for(int i = 0; i < ArraySize(g_current_levels); i++)
   {
      if(!g_current_levels[i].valid) continue;
      WPE2E_EVENT_ROW event;
      WPE2E_MakeEvent(g_current_levels[i], WPE2E_EVENT_CREATE, market_time, event);
      g_logger.WriteEvent(event);
      emitted++;
   }
   return emitted;
}

int ProcessClosedBar(const datetime market_time,
                     const double bar_high,
                     const double bar_low,
                     const double bar_close)
{
   int event_count = 0;
   datetime observed_period_start = iTime(_Symbol, g_timeframe, 0);
   if(HasPeriodRolled(observed_period_start))
   {
      g_current_period_start = observed_period_start;
      WPE2E_PIVOT_BLOCK block;
      WPE2E_LEVEL next_levels[];
      if(BuildCurrentLevels(next_levels, block))
      {
         ArrayCopy(g_current_levels, next_levels);
         event_count += EmitCreateEvents(market_time);
         PrintFormat("WPE2E_ROLLOVER symbol=%s tf=%s period=%I64d source=%I64d levels=%d",
                     _Symbol, EnumToString(g_timeframe), (long)g_current_period_start,
                     (long)block.source_start, ArraySize(g_current_levels));
      }
   }

   for(int i = 0; i < ArraySize(g_current_levels); i++)
   {
      WPE2E_EVENT_ROW event;
      if(WPE2E_ObserveLevel(g_current_levels[i], market_time, bar_high, bar_low,
                            bar_close, _Point, g_config, event))
      {
         g_logger.WriteEvent(event);
         PrintFormat("WPE2E_EVENT time=%I64d kind=%s event=%s previous=%s current=%s lower=%.10g price=%.10g upper=%.10g touches=%d attempts=%d",
                     (long)market_time, WPE2E_KindName(event.kind), WPE2E_EventName(event.event),
                     WPE2E_StateName(event.previous_state), WPE2E_StateName(event.current_state),
                     event.lower, event.price, event.upper, event.touches, event.attempts);
         event_count++;
      }
   }
   return event_count;
}

void FinalizeRun(const string reason)
{
   if(g_finalized) return;
   g_logger.Finalize(reason);
   g_finalized = true;
   PrintFormat("WPE2E_FINAL frames=%I64u levels=%I64u zones=%I64u events=%I64u root=%I64u reason=%s",
               g_logger.FrameCount(), g_logger.LevelCount(), g_logger.ZoneCount(),
               g_logger.EventCount(), g_logger.RootHash(), reason);
}

void DrawAll(void)
{
   DeleteObjects();
   WPE2E_PIVOT_BLOCK blocks[];
   if(BuildBlockHistory(blocks))
      for(int i = 0; i < ArraySize(blocks); i++)
         DrawBlock(blocks[i], i == 0);

   if(InpShowFuturePreview)
   {
      MqlRates current[];
      if(CopyRates(_Symbol, g_timeframe, 0, 1, current) == 1)
      {
         MqlRates preview = current[0];
         double chart_close[];
         ArraySetAsSeries(chart_close, true);
         if(CopyClose(_Symbol, (ENUM_TIMEFRAMES)_Period, 0, 1, chart_close) == 1)
            preview.close = chart_close[0];
         WPE2E_PIVOT_BLOCK future;
         WPE2E_CalculateBlock(preview, -1, g_timeframe, g_period_seconds, future);
         future.effective_start = preview.time;
         future.effective_end = preview.time + g_period_seconds;
         DrawBlock(future, false);
      }
   }
}

int OnInit()
{
   WPE2E_DefaultConfig(g_config);
   g_config.interaction_buffer_points = MathMax(InpInteractionBufferPoints, 1);
   g_config.acceptance_bars = MathMax(InpAcceptanceBars, 1);
   g_config.periods_to_keep = MathMax(MathMin(InpCountPeriods, WPE2E_MAX_PERIODS), 1);
   g_timeframe = InpTimePeriod == PERIOD_CURRENT ? (ENUM_TIMEFRAMES)_Period : InpTimePeriod;
   g_period_seconds = MathMax(PeriodSeconds(g_timeframe), 1);
   g_chart_seconds = MathMax(PeriodSeconds((ENUM_TIMEFRAMES)_Period), 1);
   g_prefix = "WaynePivotE2E_" + InpInstanceID + "_";
   g_current_period_start = iTime(_Symbol, g_timeframe, 0);
   g_last_market_bar = 0;
   g_frame = 0;
   g_ready = false;
   g_finalized = false;
   ArrayResize(g_current_levels, WPE2E_LEVEL_COUNT);
   for(int i = 0; i < WPE2E_LEVEL_COUNT; i++) WPE2E_ResetLevel(g_current_levels[i]);

   SetupBuffer(0, g_buffer_s3, "S3");
   SetupBuffer(1, g_buffer_m0, "M0");
   SetupBuffer(2, g_buffer_s2, "S2");
   SetupBuffer(3, g_buffer_m1, "M1");
   SetupBuffer(4, g_buffer_s1, "S1");
   SetupBuffer(5, g_buffer_m2, "M2");
   SetupBuffer(6, g_buffer_pp, "PP");
   SetupBuffer(7, g_buffer_m3, "M3");
   SetupBuffer(8, g_buffer_r1, "R1");
   SetupBuffer(9, g_buffer_m4, "M4");
   SetupBuffer(10, g_buffer_r2, "R2");
   SetupBuffer(11, g_buffer_m5, "M5");
   SetupBuffer(12, g_buffer_r3, "R3");
   SetupBuffer(13, g_buffer_lower_zone_low, "M1_S2_ZONE_LOW");
   SetupBuffer(14, g_buffer_lower_zone_high, "M1_S2_ZONE_HIGH");
   SetupBuffer(15, g_buffer_upper_zone_low, "M4_R2_ZONE_LOW");
   SetupBuffer(16, g_buffer_upper_zone_high, "M4_R2_ZONE_HIGH");
   ClearBuffers();

   string run_key = StringFormat("%s_%I64d", InpInstanceID, (long)GetTickCount());
   string invocation_id = StringFormat("%s_%I64d", _Symbol, (long)TimeLocal());
   if(!g_logger.Init(true, _Symbol, g_timeframe, InpInstanceID, run_key, invocation_id, 10))
   {
      Print("WPE2E_INIT_FAILED logger");
      return INIT_FAILED;
   }
   IndicatorSetString(INDICATOR_SHORTNAME, "Wayne Pivot E2E [" + EnumToString(g_timeframe) + "]");
   return INIT_SUCCEEDED;
}

void OnDeinit(const int reason)
{
   FinalizeRun(StringFormat("DEINIT_%d", reason));
   g_logger.Close();
   DeleteObjects();
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
   if(rates_total < 2) return 0;
   if(g_finalized) return rates_total;
   ArraySetAsSeries(time, true);
   ArraySetAsSeries(high, true);
   ArraySetAsSeries(low, true);
   ArraySetAsSeries(close, true);

   WPE2E_PIVOT_BLOCK current_block;
   if(!g_ready)
   {
      if(!BuildCurrentLevels(g_current_levels, current_block)) return prev_calculated;
      g_current_period_start = iTime(_Symbol, g_timeframe, 0);
      g_ready = true;
      EmitCreateEvents(time[1]);
   }
   PublishBuffers(g_current_levels);

   if(time[1] != g_last_market_bar)
   {
      g_last_market_bar = time[1];
      int frame_events = ProcessClosedBar(time[1], high[1], low[1], close[1]);
      PublishBuffers(g_current_levels);
      g_frame++;
      ulong frame_hash = WPE2E_HashLevels(g_current_levels);
      int active_levels = 0;
      int active_zones = 0;
      for(int i = 0; i < ArraySize(g_current_levels); i++)
      {
         if(!g_current_levels[i].valid) continue;
         if(g_current_levels[i].family == WPE2E_FAMILY_ZONE) active_zones++;
         else active_levels++;
      }
      g_logger.WriteFrame(g_frame, time[1], g_current_period_start,
                          g_current_levels[6].source_start,
                          g_current_levels[6].price, active_levels, active_zones,
                          frame_events, frame_hash, g_current_levels);
      if((g_frame % 32) == 0)
         PrintFormat("WPE2E_FRAME frame=%I64u market=%I64d period=%I64d active_levels=%d active_zones=%d events=%d hash=%I64u",
                     g_frame, (long)time[1], (long)g_current_period_start,
                     active_levels, active_zones, frame_events, frame_hash);
      DrawAll();
      ChartRedraw(0);
      if(InpTesterFinalizeAt > 0 && time[1] >= InpTesterFinalizeAt)
         FinalizeRun("DECLARED_TEST_BOUNDARY");
   }
   else if(prev_calculated == 0)
   {
      DrawAll();
      ChartRedraw(0);
   }
   return rates_total;
}

//+------------------------------------------------------------------+
