//+------------------------------------------------------------------+
//| KittDailySwingStateMachine.mq5                                   |
//| Standalone daily-swing chart eyeball over a logged state machine. |
//|                                                                  |
//| This is deliberately a new indicator.  It does not edit or        |
//| include the existing KittDailySwingZones indicator.               |
//+------------------------------------------------------------------+
#property strict
#property indicator_chart_window
#property indicator_plots 0
#property indicator_buffers 0
#property tester_everytick_calculate
#property version "1.00"

#include "KittDailySwingStateMachine.mqh"

input group "Swing Source"
input ENUM_TIMEFRAMES InpCalcTF = PERIOD_CURRENT;
input int             InpDaysToKeep = 5;
input bool            InpConfirmRejectBreakOnClose = true;

input group "Projection"
input int             InpExtendBarsRight = 24;
input bool            InpShowLabels = true;
input int             InpLabelFontSize = 8;
input bool            InpPutZonesBehindPrice = true;

input group "ROYGB Day Colors"
input color           InpDay0Color = C'255,0,0';
input color           InpDay1Color = C'255,128,0';
input color           InpDay2Color = C'255,235,0';
input color           InpDay3Color = C'0,230,0';
input color           InpDay4Color = C'0,128,255';

input group "Interaction Colors"
input color           InpTouchColor = clrWhite;
input color           InpHighRejectColor = C'255,64,128';
input color           InpLowRejectColor = clrAqua;
input color           InpHighBreakColor = clrLime;
input color           InpLowBreakColor = clrMagenta;
input int             InpFreshWidth = 1;
input int             InpTouchWidth = 2;
input int             InpRejectWidth = 3;
input int             InpBreakWidth = 3;

input group "Fill"
input bool            InpPreserveRainbowFill = true;
input double          InpRainbowFade = 0.48;
input double          InpFreshFade = 0.48;
input double          InpTouchedFade = 0.52;
input double          InpRejectedFade = 0.44;
input double          InpBrokenFade = 0.62;

input group "Runtime"
input bool            InpEnableLiveTimer = true;
input uint            InpLiveTimerMs = 500;

input group "Research Receipts"
input bool            InpEnableLogging = false;
input bool            InpLogEveryUpdate = false;
input string          InpRunKey = "KITT_DAILY_SWING";
input string          InpInvocationId = "AUTO";
input datetime        InpTesterFinalizeAt = 0;

const string KDSW_PREFIX = "KittDailySwingState_";

CKittDailySwingStateMachine g_machine;
CKittDailySwingLogger       g_logger;
bool                        g_is_tester = false;
int                         g_chart_period_sec = 60;

double Clamp01(const double value)
{
   if(value < 0.0) return 0.0;
   if(value > 1.0) return 1.0;
   return value;
}

color DayColor(const int age)
{
   if(age <= 0) return InpDay0Color;
   if(age == 1) return InpDay1Color;
   if(age == 2) return InpDay2Color;
   if(age == 3) return InpDay3Color;
   return InpDay4Color;
}

color MixWithBackground(const color source, const double fade)
{
   double f = Clamp01(fade);
   color bg = (color)ChartGetInteger(0, CHART_COLOR_BACKGROUND);

   int sr = (int)(source & 0xFF);
   int sg = (int)((source >> 8) & 0xFF);
   int sb = (int)((source >> 16) & 0xFF);
   int br = (int)(bg & 0xFF);
   int bgc = (int)((bg >> 8) & 0xFF);
   int bb = (int)((bg >> 16) & 0xFF);

   int r = (int)MathRound(sr * (1.0 - f) + br * f);
   int g = (int)MathRound(sg * (1.0 - f) + bgc * f);
   int b = (int)MathRound(sb * (1.0 - f) + bb * f);
   return (color)((b << 16) | (g << 8) | r);
}

color ReadableTextColor(const color preferred)
{
   color bg = (color)ChartGetInteger(0, CHART_COLOR_BACKGROUND);
   int pr = (int)(preferred & 0xFF);
   int pg = (int)((preferred >> 8) & 0xFF);
   int pb = (int)((preferred >> 16) & 0xFF);
   int br = (int)(bg & 0xFF);
   int bgc = (int)((bg >> 8) & 0xFF);
   int bb = (int)((bg >> 16) & 0xFF);

   int delta = MathAbs(pr - br) + MathAbs(pg - bgc) + MathAbs(pb - bb);
   if(delta >= 180)
      return preferred;

   int luma = (br * 30 + bgc * 59 + bb * 11) / 100;
   return (luma < 128) ? clrWhite : clrBlack;
}

string ZoneBase(const KDSW_ZONE &zone)
{
   return KDSW_PREFIX
        + IntegerToString((long)zone.day_start)
        + "_"
        + ((zone.side == KDSW_HIGH) ? "H" : "L");
}

void UpsertRectangle(const string name,
                     const datetime t1,
                     const double p1,
                     const datetime t2,
                     const double p2,
                     const color value,
                     const bool fill,
                     const bool back,
                     const ENUM_LINE_STYLE style,
                     const int width)
{
   if(ObjectFind(0, name) < 0)
      ObjectCreate(0, name, OBJ_RECTANGLE, 0, t1, p1, t2, p2);
   else
   {
      ObjectMove(0, name, 0, t1, p1);
      ObjectMove(0, name, 1, t2, p2);
   }

   ObjectSetInteger(0, name, OBJPROP_COLOR, value);
   ObjectSetInteger(0, name, OBJPROP_FILL, fill);
   ObjectSetInteger(0, name, OBJPROP_BACK, back);
   ObjectSetInteger(0, name, OBJPROP_STYLE, style);
   ObjectSetInteger(0, name, OBJPROP_WIDTH, width);
   ObjectSetInteger(0, name, OBJPROP_SELECTABLE, false);
   ObjectSetInteger(0, name, OBJPROP_SELECTED, false);
   ObjectSetInteger(0, name, OBJPROP_HIDDEN, true);
}

void UpsertText(const string name,
                const datetime t,
                const double price,
                const string text,
                const color value)
{
   if(ObjectFind(0, name) < 0)
      ObjectCreate(0, name, OBJ_TEXT, 0, t, price);
   else
      ObjectMove(0, name, 0, t, price);

   ObjectSetString(0, name, OBJPROP_TEXT, text);
   ObjectSetInteger(0, name, OBJPROP_COLOR, value);
   ObjectSetInteger(0, name, OBJPROP_FONTSIZE, MathMax(InpLabelFontSize, 6));
   ObjectSetInteger(0, name, OBJPROP_ANCHOR, ANCHOR_LEFT);
   ObjectSetInteger(0, name, OBJPROP_SELECTABLE, false);
   ObjectSetInteger(0, name, OBJPROP_SELECTED, false);
   ObjectSetInteger(0, name, OBJPROP_HIDDEN, true);
}

void ResolveStateVisual(const KDSW_ZONE &zone,
                        color &fill_color,
                        color &edge_color,
                        ENUM_LINE_STYLE &style,
                        int &width)
{
   double fade = InpRainbowFade;
   if(!InpPreserveRainbowFill)
   {
      if(zone.state == KDSW_TOUCHED) fade = InpTouchedFade;
      else if(zone.state == KDSW_REJECTED) fade = InpRejectedFade;
      else if(zone.state == KDSW_BROKEN) fade = InpBrokenFade;
      else fade = InpFreshFade;
   }

   color age_color = DayColor(zone.day_age);
   fill_color = MixWithBackground(age_color, fade);
   edge_color = age_color;
   style = STYLE_SOLID;
   width = MathMax(InpFreshWidth, 1);

   if(zone.state == KDSW_TOUCHED)
   {
      edge_color = InpTouchColor;
      style = STYLE_DASH;
      width = MathMax(InpTouchWidth, 1);
   }
   else if(zone.state == KDSW_REJECTED)
   {
      edge_color = (zone.side == KDSW_HIGH)
                 ? InpHighRejectColor
                 : InpLowRejectColor;
      width = MathMax(InpRejectWidth, 1);
   }
   else if(zone.state == KDSW_BROKEN)
   {
      edge_color = (zone.side == KDSW_HIGH)
                 ? InpHighBreakColor
                 : InpLowBreakColor;
      style = STYLE_DASHDOT;
      width = MathMax(InpBreakWidth, 1);
   }
}

void DeleteOwnedObjects()
{
   ObjectsDeleteAll(0, KDSW_PREFIX);
   ChartRedraw(0);
}

void CleanupExpiredObjects()
{
   int total = ObjectsTotal(0, -1, -1);
   for(int i = total - 1; i >= 0; i--)
   {
      string name = ObjectName(0, i, -1, -1);
      if(StringFind(name, KDSW_PREFIX) != 0)
         continue;

      bool keep = false;
      for(int d = 0; d < g_machine.DaysToKeep(); d++)
      {
         datetime day_start = iTime(_Symbol, PERIOD_D1, d);
         string token = KDSW_PREFIX + IntegerToString((long)day_start) + "_";
         if(day_start > 0 && StringFind(name, token) == 0)
         {
            keep = true;
            break;
         }
      }

      if(!keep)
         ObjectDelete(0, name);
   }
}

void RenderZone(const KDSW_ZONE &zone)
{
   if(!zone.valid)
      return;

   KDSW_READING reading;
   g_machine.GetReading(reading);

   color fill_color, edge_color;
   ENUM_LINE_STYLE style;
   int width;
   ResolveStateVisual(zone, fill_color, edge_color, style, width);

   datetime right_time = reading.market_time
                       + (datetime)(MathMax(InpExtendBarsRight, 1)
                       * g_chart_period_sec);
   string base = ZoneBase(zone);

   UpsertRectangle(base + "_FILL",
                   zone.source_time, zone.zone_high,
                   right_time, zone.zone_low,
                   fill_color, true, InpPutZonesBehindPrice,
                   STYLE_SOLID, 1);
   UpsertRectangle(base + "_EDGE",
                   zone.source_time, zone.zone_high,
                   right_time, zone.zone_low,
                   edge_color, false, false, style, width);

   if(InpShowLabels)
   {
      double mid = (zone.zone_high + zone.zone_low) * 0.5;
      string text = "D" + IntegerToString(zone.day_age)
                  + " " + KDSW_SideName(zone.side)
                  + " | " + KDSW_StateName(zone.state)
                  + " | T:" + IntegerToString(zone.touches)
                  + " R:" + IntegerToString(zone.rejections);
      UpsertText(base + "_TXT", right_time, mid, text,
                 ReadableTextColor(edge_color));
   }
   else
      ObjectDelete(0, base + "_TXT");
}

void RenderAll()
{
   CleanupExpiredObjects();
   for(int d = 0; d < g_machine.DaysToKeep(); d++)
   {
      KDSW_ZONE high_zone;
      KDSW_ZONE low_zone;
      if(g_machine.GetZone(d, KDSW_HIGH, high_zone))
         RenderZone(high_zone);
      if(g_machine.GetZone(d, KDSW_LOW, low_zone))
         RenderZone(low_zone);
   }
   ChartRedraw(0);
}

bool RefreshSurface()
{
   if(!g_machine.Update())
      return false;

   if(g_machine.Dirty())
      RenderAll();
   g_machine.Log(g_logger, InpLogEveryUpdate);

   // Direct indicator runs in Strategy Tester do not consistently provide a
   // durable OnDeinit receipt before the tester process exits.  An explicit
   // end timestamp gives campaigns the same deterministic sealing seam used
   // by the master controller.
   if(g_is_tester && InpTesterFinalizeAt > 0 && g_logger.Enabled())
   {
      KDSW_READING reading;
      g_machine.GetReading(reading);
      if(reading.market_time >= InpTesterFinalizeAt)
         g_logger.Close();
   }

   return true;
}

int OnInit()
{
   g_is_tester = (bool)MQLInfoInteger(MQL_TESTER);
   g_chart_period_sec = MathMax(PeriodSeconds((ENUM_TIMEFRAMES)_Period), 1);

   KDSW_CONFIG cfg;
   KDSW_DefaultConfig(cfg);
   cfg.days_to_keep = InpDaysToKeep;
   cfg.confirm_reject_break_on_close = InpConfirmRejectBreakOnClose;

   IndicatorSetString(INDICATOR_SHORTNAME,
      "Kitt Daily Swing State [" + EnumToString(InpCalcTF) + "]");
   DeleteOwnedObjects();

   if(!g_machine.Init(_Symbol, InpCalcTF, cfg))
   {
      Print("Kitt daily swing state: initial data unavailable. Error=",
            GetLastError());
      return INIT_FAILED;
   }

   if(!g_logger.Init(InpEnableLogging, InpRunKey, InpInvocationId))
   {
      Print("Kitt daily swing state: logger initialization failed. Error=",
            GetLastError());
      return INIT_FAILED;
   }

   RenderAll();
   g_machine.Log(g_logger, true);

   if(InpEnableLiveTimer && !g_is_tester)
   {
      uint interval = MathMax(InpLiveTimerMs, 100);
      if(!EventSetMillisecondTimer((int)interval))
         Print("Kitt daily swing state: timer setup failed. Error=",
               GetLastError());
   }
   return INIT_SUCCEEDED;
}

void OnDeinit(const int reason)
{
   EventKillTimer();
   g_logger.Close();
   DeleteOwnedObjects();
}

void OnTimer()
{
   if(g_is_tester || !InpEnableLiveTimer)
      return;
   RefreshSurface();
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

   // The hot path is CopyRates(2), D0 maintenance, and at most ten
   // interaction checks. Full history replay occurs only at init/rollover.
   RefreshSurface();
   return rates_total;
}

//+------------------------------------------------------------------+
