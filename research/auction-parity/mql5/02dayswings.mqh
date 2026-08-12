//+------------------------------------------------------------------+
//| 02dayswings.mqh                                                  |
//| Headless 5-day body-to-wick daily extreme zone producer         |
//|                                                                  |
//| Extracted from:                                                  |
//|   Kitt_DailySwingZones_5Day_ROYGB_Optimized.mq5 v1.10           |
//|                                                                  |
//| Purpose                                                          |
//|   - NO chart objects                                             |
//|   - NO indicator buffers                                         |
//|   - NO input declarations                                        |
//|   - NO OnInit/OnCalculate/OnTimer ownership                      |
//|   - Preserves the original daily swing-zone geometry/state       |
//|   - Emits typed structural bands for a future master controller  |
//|                                                                  |
//| Geometry                                                         |
//|   HIGH zone = [max(open,close), high] of the bar making day high |
//|   LOW  zone = [low, min(open,close)] of the bar making day low   |
//|                                                                  |
//| Runtime                                                          |
//|   - Full 1..5 day rebuild only on Init / broker-day rollover     |
//|   - Incremental D0 maintenance on normal updates                 |
//|   - Intrabar touch tracking                                      |
//|   - Closed-bar rejection/break state                             |
//+------------------------------------------------------------------+
#ifndef __KITT_02DAYSWINGS_MQH__
#define __KITT_02DAYSWINGS_MQH__

#define DSW_MAX_DAYS   5
#define DSW_MAX_ZONES 10

// -----------------------------------------------------------------------------
// Public contracts
// -----------------------------------------------------------------------------

enum DSW_ZONE_STATE
{
   DSW_ZONE_FRESH    = 0,
   DSW_ZONE_TOUCHED  = 1,
   DSW_ZONE_REJECTED = 2,
   DSW_ZONE_BROKEN   = 3
};

enum DSW_ZONE_SIDE
{
   DSW_ZONE_LOW  = 0,
   DSW_ZONE_HIGH = 1
};

enum DSW_LEVEL_FAMILY
{
   DSW_FAMILY_DAILY_EXTREME = 1
};

enum DSW_LEVEL_KIND
{
   DSW_KIND_DAILY_LOW_BAND  = 1,
   DSW_KIND_DAILY_HIGH_BAND = 2
};

enum DSW_LEVEL_ROLE
{
   DSW_ROLE_LOWER_BOUNDARY = 1,
   DSW_ROLE_UPPER_BOUNDARY = 2
};

struct DSW_Config
{
   int  days_to_keep;                    // clamped 1..5
   bool confirm_reject_break_on_close;   // preserves source default semantics
};

struct DSW_Zone
{
   bool           valid;
   DSW_ZONE_SIDE  side;
   DSW_ZONE_STATE state;
   int            day_age;

   datetime       day_start;
   datetime       source_time;

   double         zone_low;
   double         zone_high;
   double         extreme_price;

   int            touches;
   int            rejections;

   datetime       first_touch_time;
   datetime       last_touch_time;
   datetime       last_touch_bar_time;
   datetime       last_rejection_bar_time;
   datetime       break_time;
};

// Headless structural output.  Its shape intentionally mirrors the useful
// geometry/provenance fields emitted by the other MT5 structural producers,
// while remaining independent so the future master controller can normalize
// all producer families into one common market-level contract.
struct DSW_Level
{
   bool              valid;
   ulong             local_id;

   DSW_LEVEL_FAMILY  family;
   DSW_LEVEL_KIND    kind;
   DSW_LEVEL_ROLE    role;
   DSW_ZONE_SIDE     side;
   DSW_ZONE_STATE    state;

   double            lower;
   double            price;          // semantic anchor = actual daily extreme
   double            center;         // geometric center of the band
   double            upper;
   double            width;

   ENUM_TIMEFRAMES   timeframe;
   int               day_age;

   datetime          day_start;
   datetime          created_at;     // source candle making the extreme
   datetime          updated_at;     // latest producer observation time

   bool              developing;     // true only for D0
   bool              frozen_geometry;// true for completed prior days

   int               touches;
   int               rejections;

   datetime          first_touch_time;
   datetime          last_touch_time;
   datetime          break_time;
};

struct DSW_Reading
{
   bool             valid;
   string           symbol;
   ENUM_TIMEFRAMES  timeframe;
   int              days_to_keep;
   datetime         market_time;
   datetime         current_day_start;
   datetime         current_bar_time;
   ulong            update_count;
   int              valid_zone_count;
};

void DSW_DefaultConfig(DSW_Config &cfg)
{
   ZeroMemory(cfg);
   cfg.days_to_keep = 5;
   cfg.confirm_reject_break_on_close = true;
}

string DSW_StateName(const DSW_ZONE_STATE state)
{
   if(state == DSW_ZONE_TOUCHED)  return "TOUCHED";
   if(state == DSW_ZONE_REJECTED) return "REJECTED";
   if(state == DSW_ZONE_BROKEN)   return "BROKEN";
   return "FRESH";
}

string DSW_SideName(const DSW_ZONE_SIDE side)
{
   return (side == DSW_ZONE_HIGH) ? "HIGH" : "LOW";
}

// Stable inside one broker day and side. Decimal arithmetic is deliberately
// used instead of C/C++ integer-literal suffixes so MetaEditor accepts it.
ulong DSW_MakeLocalId(const datetime day_start, const DSW_ZONE_SIDE side)
{
   ulong base = (day_start > 0) ? (ulong)day_start : 0;
   return base * 4 + (ulong)((int)side + 1);
}

// -----------------------------------------------------------------------------
// Producer
// -----------------------------------------------------------------------------

class CKittDaySwingsProducer
{
private:
   string           m_symbol;
   ENUM_TIMEFRAMES  m_calc_tf;
   int              m_calc_period_sec;

   DSW_Config       m_cfg;
   DSW_Zone         m_zones[DSW_MAX_ZONES];

   int              m_days;
   bool             m_initialized;
   bool             m_is_tester;

   datetime         m_current_day_start;
   datetime         m_last_calc_bar_time;
   datetime         m_market_time;
   ulong            m_update_count;

   int ClampDays(const int value)
   {
      if(value < 1) return 1;
      if(value > DSW_MAX_DAYS) return DSW_MAX_DAYS;
      return value;
   }

   bool ResolveMarketTime(datetime &market_time)
   {
      market_time = 0;

      MqlTick tick;
      bool got_tick = SymbolInfoTick(m_symbol, tick);

      if(m_is_tester)
      {
         if(got_tick && tick.time > 0)
            market_time = tick.time;
         else
            market_time = TimeCurrent();

         return market_time > 0;
      }

      if(got_tick && tick.time > 0)
         market_time = tick.time;

      datetime server_now = TimeTradeServer();
      if(server_now > market_time)
         market_time = server_now;
      else if(market_time <= 0)
         market_time = TimeCurrent();

      return market_time > 0;
   }

   datetime DayStartByAge(const int day_age)
   {
      return iTime(m_symbol, PERIOD_D1, day_age);
   }

   void ResetZone(DSW_Zone &zone,
                  const DSW_ZONE_SIDE side,
                  const int day_age,
                  const datetime day_start,
                  const MqlRates &bar)
   {
      ZeroMemory(zone);

      zone.valid       = true;
      zone.side        = side;
      zone.state       = DSW_ZONE_FRESH;
      zone.day_age     = day_age;
      zone.day_start   = day_start;
      zone.source_time = bar.time;

      if(side == DSW_ZONE_HIGH)
      {
         zone.zone_low      = MathMax(bar.open, bar.close);
         zone.zone_high     = bar.high;
         zone.extreme_price = bar.high;
      }
      else
      {
         zone.zone_low      = bar.low;
         zone.zone_high     = MathMin(bar.open, bar.close);
         zone.extreme_price = bar.low;
      }
   }

   bool BuildDayPair(const int day_age,
                     const datetime market_time,
                     DSW_Zone &high_zone,
                     DSW_Zone &low_zone)
   {
      ZeroMemory(high_zone);
      ZeroMemory(low_zone);

      datetime day_start = DayStartByAge(day_age);
      if(day_start <= 0)
         return false;

      datetime day_end = market_time;

      if(day_age > 0)
      {
         datetime newer_day_start = DayStartByAge(day_age - 1);
         if(newer_day_start <= 0)
            return false;
         day_end = newer_day_start - 1;
      }

      if(day_end <= day_start)
         return false;

      MqlRates rates[];
      int copied = CopyRates(m_symbol, m_calc_tf, day_start, day_end, rates);
      if(copied <= 0)
         return false;

      double best_high = -DBL_MAX;
      double best_low  =  DBL_MAX;
      int high_index = -1;
      int low_index  = -1;

      // CopyRates physical order is oldest -> newest. >= / <= intentionally
      // choose the latest source candle when the exact extreme repeats.
      for(int i = 0; i < copied; i++)
      {
         if(rates[i].high >= best_high)
         {
            best_high = rates[i].high;
            high_index = i;
         }

         if(rates[i].low <= best_low)
         {
            best_low = rates[i].low;
            low_index = i;
         }
      }

      if(high_index >= 0)
         ResetZone(high_zone, DSW_ZONE_HIGH, day_age, day_start, rates[high_index]);

      if(low_index >= 0)
         ResetZone(low_zone, DSW_ZONE_LOW, day_age, day_start, rates[low_index]);

      return high_zone.valid || low_zone.valid;
   }

   bool RegisterTouch(DSW_Zone &zone, const MqlRates &bar)
   {
      if(zone.last_touch_bar_time == bar.time)
         return false;

      zone.last_touch_bar_time = bar.time;
      zone.touches++;

      if(zone.first_touch_time == 0)
         zone.first_touch_time = bar.time;

      zone.last_touch_time = bar.time;

      if(zone.state == DSW_ZONE_FRESH)
         zone.state = DSW_ZONE_TOUCHED;

      return true;
   }

   bool ApplyClosedBar(DSW_Zone &zone, const MqlRates &bar)
   {
      if(!zone.valid || zone.state == DSW_ZONE_BROKEN || bar.time <= zone.source_time)
         return false;

      bool changed = false;
      bool overlaps = (bar.high >= zone.zone_low && bar.low <= zone.zone_high);

      if(zone.side == DSW_ZONE_HIGH)
      {
         if(bar.close > zone.zone_high)
         {
            zone.state = DSW_ZONE_BROKEN;
            zone.break_time = bar.time;
            return true;
         }

         if(overlaps)
         {
            if(RegisterTouch(zone, bar))
               changed = true;

            if(bar.close < zone.zone_low && zone.last_rejection_bar_time != bar.time)
            {
               zone.last_rejection_bar_time = bar.time;
               zone.rejections++;
               zone.state = DSW_ZONE_REJECTED;
               changed = true;
            }
         }
      }
      else
      {
         if(bar.close < zone.zone_low)
         {
            zone.state = DSW_ZONE_BROKEN;
            zone.break_time = bar.time;
            return true;
         }

         if(overlaps)
         {
            if(RegisterTouch(zone, bar))
               changed = true;

            if(bar.close > zone.zone_high && zone.last_rejection_bar_time != bar.time)
            {
               zone.last_rejection_bar_time = bar.time;
               zone.rejections++;
               zone.state = DSW_ZONE_REJECTED;
               changed = true;
            }
         }
      }

      return changed;
   }

   bool ApplyIntrabarTouch(DSW_Zone &zone, const MqlRates &bar)
   {
      if(!zone.valid || zone.state == DSW_ZONE_BROKEN || bar.time <= zone.source_time)
         return false;

      bool overlaps = (bar.high >= zone.zone_low && bar.low <= zone.zone_high);
      if(!overlaps)
         return false;

      return RegisterTouch(zone, bar);
   }

   void ReplayHistoryOnce(const datetime market_time)
   {
      datetime oldest_source = 0;

      for(int i = 0; i < m_days * 2; i++)
      {
         if(!m_zones[i].valid)
            continue;

         if(oldest_source == 0 || m_zones[i].source_time < oldest_source)
            oldest_source = m_zones[i].source_time;
      }

      if(oldest_source <= 0 || market_time <= oldest_source)
         return;

      MqlRates history[];
      int copied = CopyRates(m_symbol, m_calc_tf,
                             oldest_source + m_calc_period_sec,
                             market_time,
                             history);
      if(copied <= 0)
         return;

      // One historical pass at initialization/day rollover only.
      for(int b = 0; b < copied; b++)
      {
         for(int z = 0; z < m_days * 2; z++)
         {
            if(m_cfg.confirm_reject_break_on_close)
            {
               ApplyClosedBar(m_zones[z], history[b]);
            }
            else
            {
               // Preserves the source indicator's practical behavior: touch can
               // be observed before close while rejection/break state itself is
               // still resolved from bar OHLC/close.
               ApplyIntrabarTouch(m_zones[z], history[b]);
               ApplyClosedBar(m_zones[z], history[b]);
            }
         }
      }
   }

   bool UpdateCurrentDayExtremeFromBar(const MqlRates &bar)
   {
      bool changed = false;

      // Fixed index contract: D0 HIGH = 0, D0 LOW = 1.
      // Do not use C++-style reference aliases to array elements in MQL5.
      if(!m_zones[0].valid ||
         bar.high > m_zones[0].extreme_price ||
         (bar.high == m_zones[0].extreme_price && bar.time > m_zones[0].source_time))
      {
         ResetZone(m_zones[0], DSW_ZONE_HIGH, 0, m_current_day_start, bar);
         changed = true;
      }

      if(!m_zones[1].valid ||
         bar.low < m_zones[1].extreme_price ||
         (bar.low == m_zones[1].extreme_price && bar.time > m_zones[1].source_time))
      {
         ResetZone(m_zones[1], DSW_ZONE_LOW, 0, m_current_day_start, bar);
         changed = true;
      }

      return changed;
   }

   bool ProcessClosedBarAcrossZones(const MqlRates &closed_bar)
   {
      bool changed = false;

      for(int i = 0; i < m_days * 2; i++)
      {
         if(ApplyClosedBar(m_zones[i], closed_bar))
            changed = true;
      }

      return changed;
   }

   bool ProcessIntrabarTouches(const MqlRates &current_bar)
   {
      bool changed = false;

      for(int i = 0; i < m_days * 2; i++)
      {
         if(ApplyIntrabarTouch(m_zones[i], current_bar))
            changed = true;
      }

      return changed;
   }

   int CountValidZones()
   {
      int count = 0;
      for(int i = 0; i < m_days * 2; i++)
      {
         if(m_zones[i].valid)
            count++;
      }
      return count;
   }

public:
   CKittDaySwingsProducer()
   {
      m_symbol = "";
      m_calc_tf = PERIOD_CURRENT;
      m_calc_period_sec = 60;
      m_days = 5;
      m_initialized = false;
      m_is_tester = false;
      m_current_day_start = 0;
      m_last_calc_bar_time = 0;
      m_market_time = 0;
      m_update_count = 0;
      DSW_DefaultConfig(m_cfg);

      for(int i = 0; i < DSW_MAX_ZONES; i++)
         ZeroMemory(m_zones[i]);
   }

   bool Init(const string symbol,
             const ENUM_TIMEFRAMES calc_tf,
             const DSW_Config &cfg)
   {
      m_symbol = symbol;
      if(StringLen(m_symbol) <= 0)
         m_symbol = _Symbol;

      m_calc_tf = (calc_tf == PERIOD_CURRENT)
                ? (ENUM_TIMEFRAMES)_Period
                : calc_tf;

      m_calc_period_sec = MathMax(PeriodSeconds(m_calc_tf), 1);
      m_cfg = cfg;
      m_days = ClampDays(m_cfg.days_to_keep);
      m_cfg.days_to_keep = m_days;

      m_is_tester = (bool)MQLInfoInteger(MQL_TESTER);
      m_initialized = false;
      m_current_day_start = 0;
      m_last_calc_bar_time = 0;
      m_market_time = 0;
      m_update_count = 0;

      for(int i = 0; i < DSW_MAX_ZONES; i++)
         ZeroMemory(m_zones[i]);

      datetime market_time = 0;
      if(!ResolveMarketTime(market_time))
         return false;

      return Rebuild(market_time);
   }

   void Reset()
   {
      for(int i = 0; i < DSW_MAX_ZONES; i++)
         ZeroMemory(m_zones[i]);

      m_initialized = false;
      m_current_day_start = 0;
      m_last_calc_bar_time = 0;
      m_market_time = 0;
      m_update_count = 0;
   }

   bool Rebuild(const datetime market_time)
   {
      if(market_time <= 0)
         return false;

      for(int i = 0; i < DSW_MAX_ZONES; i++)
         ZeroMemory(m_zones[i]);

      for(int d = 0; d < m_days; d++)
      {
         DSW_Zone high_zone;
         DSW_Zone low_zone;

         if(!BuildDayPair(d, market_time, high_zone, low_zone))
            continue;

         m_zones[d * 2]     = high_zone;
         m_zones[d * 2 + 1] = low_zone;
      }

      m_current_day_start = DayStartByAge(0);
      if(m_current_day_start <= 0)
         return false;

      MqlRates pair[];
      int copied = CopyRates(m_symbol, m_calc_tf, 0, 2, pair);

      if(copied > 0)
      {
         ArraySetAsSeries(pair, true);
         m_last_calc_bar_time = pair[0].time;
      }
      else
      {
         m_last_calc_bar_time = 0;
      }

      ReplayHistoryOnce(market_time);
      m_market_time = market_time;
      m_initialized = true;
      m_update_count++;
      return true;
   }

   // Call from the owner EA/indicator OnTick, OnCalculate, or a coordinated
   // controller timer. This producer owns no MT5 lifecycle callbacks itself.
   bool Update()
   {
      datetime market_time = 0;
      if(!ResolveMarketTime(market_time))
         return false;

      datetime day0 = DayStartByAge(0);
      if(day0 <= 0)
         return false;

      if(!m_initialized || day0 != m_current_day_start)
         return Rebuild(market_time);

      MqlRates pair[];
      int copied = CopyRates(m_symbol, m_calc_tf, 0, 2, pair);
      if(copied <= 0)
         return false;

      ArraySetAsSeries(pair, true);

      MqlRates current_bar = pair[0];
      bool new_calc_bar = (m_last_calc_bar_time != 0 &&
                           current_bar.time != m_last_calc_bar_time);

      if(new_calc_bar && copied >= 2)
      {
         MqlRates closed_bar = pair[1];

         // Critical source-order invariant: if the just-closed bar established
         // a new D0 extreme, it becomes the D0 source before that same bar can
         // be evaluated as a later interaction against the zone.
         UpdateCurrentDayExtremeFromBar(closed_bar);
         ProcessClosedBarAcrossZones(closed_bar);

         m_last_calc_bar_time = current_bar.time;
      }
      else if(m_last_calc_bar_time == 0)
      {
         m_last_calc_bar_time = current_bar.time;
      }

      // Only D0 geometry develops intrabar.
      UpdateCurrentDayExtremeFromBar(current_bar);

      // Cheap O(10) interaction check. No five-day rescans here.
      ProcessIntrabarTouches(current_bar);

      m_market_time = market_time;
      m_update_count++;
      return true;
   }

   bool Initialized()
   {
      return m_initialized;
   }

   string SymbolName()
   {
      return m_symbol;
   }

   ENUM_TIMEFRAMES CalcTF()
   {
      return m_calc_tf;
   }

   int DaysToKeep()
   {
      return m_days;
   }

   datetime MarketTime()
   {
      return m_market_time;
   }

   int ZoneCount()
   {
      return CountValidZones();
   }

   bool GetZone(const int day_age,
                const DSW_ZONE_SIDE side,
                DSW_Zone &out_zone)
   {
      ZeroMemory(out_zone);

      if(day_age < 0 || day_age >= m_days)
         return false;

      int index = day_age * 2 + ((side == DSW_ZONE_HIGH) ? 0 : 1);
      if(index < 0 || index >= DSW_MAX_ZONES || !m_zones[index].valid)
         return false;

      out_zone = m_zones[index];
      return true;
   }

   void GetReading(DSW_Reading &reading)
   {
      ZeroMemory(reading);

      reading.valid = m_initialized;
      reading.symbol = m_symbol;
      reading.timeframe = m_calc_tf;
      reading.days_to_keep = m_days;
      reading.market_time = m_market_time;
      reading.current_day_start = m_current_day_start;
      reading.current_bar_time = m_last_calc_bar_time;
      reading.update_count = m_update_count;
      reading.valid_zone_count = CountValidZones();
   }

   // Export one band per valid day/side. Ordering is deterministic:
   // D0 HIGH, D0 LOW, D1 HIGH, D1 LOW ... D4 HIGH, D4 LOW.
   int ExportLevels(DSW_Level &out_levels[])
   {
      ArrayResize(out_levels, 0);

      int count = 0;
      for(int i = 0; i < m_days * 2; i++)
      {
         if(!m_zones[i].valid)
            continue;

         int new_size = count + 1;
         ArrayResize(out_levels, new_size);

         DSW_Level level;
         ZeroMemory(level);

         level.valid = true;
         level.local_id = DSW_MakeLocalId(m_zones[i].day_start, m_zones[i].side);
         level.family = DSW_FAMILY_DAILY_EXTREME;
         level.side = m_zones[i].side;
         level.state = m_zones[i].state;

         if(m_zones[i].side == DSW_ZONE_HIGH)
         {
            level.kind = DSW_KIND_DAILY_HIGH_BAND;
            level.role = DSW_ROLE_UPPER_BOUNDARY;
         }
         else
         {
            level.kind = DSW_KIND_DAILY_LOW_BAND;
            level.role = DSW_ROLE_LOWER_BOUNDARY;
         }

         level.lower = m_zones[i].zone_low;
         level.price = m_zones[i].extreme_price;
         level.center = (m_zones[i].zone_low + m_zones[i].zone_high) * 0.5;
         level.upper = m_zones[i].zone_high;
         level.width = MathMax(m_zones[i].zone_high - m_zones[i].zone_low, 0.0);

         level.timeframe = m_calc_tf;
         level.day_age = m_zones[i].day_age;
         level.day_start = m_zones[i].day_start;
         level.created_at = m_zones[i].source_time;
         level.updated_at = m_market_time;

         level.developing = (m_zones[i].day_age == 0);
         level.frozen_geometry = (m_zones[i].day_age > 0);

         level.touches = m_zones[i].touches;
         level.rejections = m_zones[i].rejections;
         level.first_touch_time = m_zones[i].first_touch_time;
         level.last_touch_time = m_zones[i].last_touch_time;
         level.break_time = m_zones[i].break_time;

         out_levels[count] = level;
         count++;
      }

      return count;
   }
};

#endif // __KITT_02DAYSWINGS_MQH__
