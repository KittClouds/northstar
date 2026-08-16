//+------------------------------------------------------------------+
//| KittDailySwingStateMachine.mqh                                   |
//| Standalone daily-swing state machine and research receipt logger. |
//|                                                                  |
//| Geometry is intentionally compatible with the original           |
//| Kitt_DailySwingZones_5Day_ROYGB_Optimized indicator:              |
//|   HIGH = [max(open, close), high] of the day's high bar           |
//|   LOW  = [low, min(open, close)] of the day's low bar             |
//|                                                                  |
//| This header owns no MT5 lifecycle callbacks and no chart objects. |
//| The companion indicator is the lifecycle/rendering owner.         |
//+------------------------------------------------------------------+
#ifndef __KITT_DAILY_SWING_STATE_MACHINE_MQH__
#define __KITT_DAILY_SWING_STATE_MACHINE_MQH__

#define KDSW_MAX_DAYS  5
#define KDSW_MAX_ZONES 10

enum KDSW_ZONE_STATE
{
   KDSW_FRESH    = 0,
   KDSW_TOUCHED  = 1,
   KDSW_REJECTED = 2,
   KDSW_BROKEN   = 3
};

enum KDSW_ZONE_SIDE
{
   KDSW_LOW  = 0,
   KDSW_HIGH = 1
};

struct KDSW_CONFIG
{
   int  days_to_keep;
   bool confirm_reject_break_on_close;
};

struct KDSW_ZONE
{
   bool           valid;
   KDSW_ZONE_SIDE side;
   KDSW_ZONE_STATE state;
   int            day_age;

   datetime day_start;
   datetime source_time;

   double zone_low;
   double zone_high;
   double extreme_price;

   int touches;
   int rejections;

   datetime first_touch_time;
   datetime last_touch_time;
   datetime last_touch_bar_time;
   datetime last_rejection_bar_time;
   datetime break_time;
};

struct KDSW_READING
{
   bool            valid;
   string          symbol;
   ENUM_TIMEFRAMES timeframe;
   int             days_to_keep;
   datetime        market_time;
   datetime        current_day_start;
   datetime        current_bar_time;
   ulong           update_count;
   int             valid_zone_count;
};

void KDSW_DefaultConfig(KDSW_CONFIG &cfg)
{
   ZeroMemory(cfg);
   cfg.days_to_keep = KDSW_MAX_DAYS;
   cfg.confirm_reject_break_on_close = true;
}

string KDSW_StateName(const KDSW_ZONE_STATE state)
{
   if(state == KDSW_TOUCHED)  return "TOUCHED";
   if(state == KDSW_REJECTED) return "REJECTED";
   if(state == KDSW_BROKEN)   return "BROKEN";
   return "FRESH";
}

string KDSW_SideName(const KDSW_ZONE_SIDE side)
{
   return (side == KDSW_HIGH) ? "HIGH" : "LOW";
}

ulong KDSW_LocalId(const datetime day_start, const KDSW_ZONE_SIDE side)
{
   ulong base = (day_start > 0) ? (ulong)day_start : 0;
   return base * 4 + (ulong)((int)side + 1);
}

//+------------------------------------------------------------------+
//| Receipt logger                                                   |
//+------------------------------------------------------------------+
class CKittDailySwingLogger
{
private:
   bool   m_enabled;
   int    m_frames;
   int    m_zones;
   int    m_events;
   int    m_receipt;
   string m_run_key;
   string m_invocation_id;
   ulong  m_sequence;

   string D(const double value)
   {
      return StringFormat("%.17g", value);
   }

   bool OpenOne(int &handle, const string path, const string header)
   {
      handle = FileOpen(path,
                        FILE_COMMON | FILE_WRITE | FILE_CSV | FILE_ANSI |
                        FILE_SHARE_READ,
                        '\t');
      if(handle == INVALID_HANDLE)
         return false;

      FileWriteString(handle, header + "\r\n");
      return true;
   }

   void CloseOne(int &handle)
   {
      if(handle == INVALID_HANDLE)
         return;

      FileFlush(handle);
      FileClose(handle);
      handle = INVALID_HANDLE;
   }

public:
   CKittDailySwingLogger()
   {
      m_enabled = false;
      m_frames = INVALID_HANDLE;
      m_zones = INVALID_HANDLE;
      m_events = INVALID_HANDLE;
      m_receipt = INVALID_HANDLE;
      m_run_key = "";
      m_invocation_id = "";
      m_sequence = 0;
   }

   bool Init(const bool enabled,
             const string run_key,
             const string invocation_id)
   {
      Close();
      m_enabled = enabled;
      m_run_key = (StringLen(run_key) > 0) ? run_key : "KITT_DAILY_SWING";
      m_invocation_id = (StringLen(invocation_id) > 0) ? invocation_id : "AUTO";
      m_sequence = 0;

      if(!m_enabled)
         return true;

      string root = "KittDailySwingState\\" + m_run_key + "\\";
      FolderCreate("KittDailySwingState", FILE_COMMON);
      FolderCreate("KittDailySwingState\\" + m_run_key, FILE_COMMON);

      if(!OpenOne(m_frames, root + "frames.tsv",
         "contract\trun_key\tinvocation_id\tsequence\tmarket_time\tcurrent_day_start\tcurrent_bar_time\tupdate_count\tvalid_zone_count"))
      {
         Close();
         return false;
      }

      if(!OpenOne(m_zones, root + "zones.tsv",
         "contract\trun_key\tsequence\tordinal\tlocal_id\tsymbol\ttimeframe\tday_age\tside\tstate\tday_start\tsource_time\tzone_low\tzone_high\textreme_price\ttouches\trejections\tfirst_touch_time\tlast_touch_time\tlast_touch_bar_time\tlast_rejection_bar_time\tbreak_time"))
      {
         Close();
         return false;
      }

      if(!OpenOne(m_events, root + "events.tsv",
         "contract\trun_key\tinvocation_id\tsequence\tordinal\tevent\tlocal_id\tmarket_time\tbar_time\tprevious_state\tstate\tprevious_touches\ttouches\tprevious_rejections\trejections\tsource_time"))
      {
         Close();
         return false;
      }

      if(!OpenOne(m_receipt, root + "receipt.tsv",
         "contract\trun_key\tinvocation_id\tstatus\tterminal_reason"))
      {
         Close();
         return false;
      }

      FileWrite(m_receipt, "KITT_DAILY_SWING_RECEIPT_V1", m_run_key,
                m_invocation_id, "OPEN", "LOGGER_INITIALIZED");
      return true;
   }

   bool Enabled()
   {
      return m_enabled;
   }

   ulong NextSequence()
   {
      m_sequence++;
      return m_sequence;
   }

   ulong WriteFrame(const KDSW_READING &reading)
   {
      if(!m_enabled)
         return 0;

      ulong sequence = NextSequence();
      FileWrite(m_frames, "KITT_DAILY_SWING_FRAME_V1", m_run_key,
                m_invocation_id, sequence, (long)reading.market_time,
                (long)reading.current_day_start, (long)reading.current_bar_time,
                reading.update_count, reading.valid_zone_count);
      return sequence;
   }

   void WriteZone(const ulong sequence,
                  const int ordinal,
                  const KDSW_ZONE &zone,
                  const string symbol,
                  const ENUM_TIMEFRAMES timeframe)
   {
      if(!m_enabled || !zone.valid)
         return;

      FileWrite(m_zones, "KITT_DAILY_SWING_ZONE_V1", m_run_key, sequence,
                ordinal, KDSW_LocalId(zone.day_start, zone.side), symbol,
                (int)timeframe, zone.day_age, KDSW_SideName(zone.side),
                KDSW_StateName(zone.state), (long)zone.day_start,
                (long)zone.source_time, D(zone.zone_low), D(zone.zone_high),
                D(zone.extreme_price), zone.touches, zone.rejections,
                (long)zone.first_touch_time, (long)zone.last_touch_time,
                (long)zone.last_touch_bar_time,
                (long)zone.last_rejection_bar_time, (long)zone.break_time);
   }

   void WriteEvent(const ulong sequence,
                   const int ordinal,
                   const string event_name,
                   const KDSW_ZONE &previous,
                   const KDSW_ZONE &current,
                   const datetime market_time,
                   const datetime bar_time)
   {
      if(!m_enabled || !current.valid)
         return;

      FileWrite(m_events, "KITT_DAILY_SWING_EVENT_V1", m_run_key,
                m_invocation_id, sequence, ordinal, event_name,
                KDSW_LocalId(current.day_start, current.side),
                (long)market_time, (long)bar_time,
                previous.valid ? KDSW_StateName(previous.state) : "NONE",
                KDSW_StateName(current.state),
                previous.valid ? previous.touches : 0, current.touches,
                previous.valid ? previous.rejections : 0, current.rejections,
                (long)current.source_time);
   }

   void Close()
   {
      if(m_enabled && m_receipt != INVALID_HANDLE)
         FileWrite(m_receipt, "KITT_DAILY_SWING_RECEIPT_V1", m_run_key,
                   m_invocation_id, "CLOSED", "OWNER_DEINITIALIZED");

      CloseOne(m_frames);
      CloseOne(m_zones);
      CloseOne(m_events);
      CloseOne(m_receipt);
      m_enabled = false;
   }

   void Flush()
   {
      if(!m_enabled)
         return;

      FileFlush(m_frames);
      FileFlush(m_zones);
      FileFlush(m_events);
      FileFlush(m_receipt);
   }
};

//+------------------------------------------------------------------+
//| Deterministic state machine                                      |
//+------------------------------------------------------------------+
class CKittDailySwingStateMachine
{
private:
   string          m_symbol;
   ENUM_TIMEFRAMES m_calc_tf;
   int             m_calc_period_sec;
   KDSW_CONFIG     m_cfg;
   KDSW_ZONE       m_zones[KDSW_MAX_ZONES];
   KDSW_ZONE       m_logged[KDSW_MAX_ZONES];
   int             m_days;
   bool            m_initialized;
   bool            m_is_tester;
   bool            m_has_logged;
   bool            m_dirty;
   datetime        m_current_day_start;
   datetime        m_last_calc_bar_time;
   datetime        m_market_time;
   ulong           m_update_count;

   int ClampDays(const int value)
   {
      if(value < 1) return 1;
      if(value > KDSW_MAX_DAYS) return KDSW_MAX_DAYS;
      return value;
   }

   datetime DayStartByAge(const int day_age)
   {
      return iTime(m_symbol, PERIOD_D1, day_age);
   }

   bool ResolveMarketTime(datetime &market_time)
   {
      market_time = 0;
      MqlTick tick;
      bool got_tick = SymbolInfoTick(m_symbol, tick);

      if(got_tick && tick.time > 0)
         market_time = tick.time;

      if(m_is_tester)
      {
         if(market_time <= 0)
            market_time = TimeCurrent();
         return market_time > 0;
      }

      datetime server_now = TimeTradeServer();
      if(server_now > market_time)
         market_time = server_now;
      else if(market_time <= 0)
         market_time = TimeCurrent();

      return market_time > 0;
   }

   void ResetZone(KDSW_ZONE &zone,
                  const KDSW_ZONE_SIDE side,
                  const int day_age,
                  const datetime day_start,
                  const MqlRates &bar)
   {
      ZeroMemory(zone);
      zone.valid = true;
      zone.side = side;
      zone.state = KDSW_FRESH;
      zone.day_age = day_age;
      zone.day_start = day_start;
      zone.source_time = bar.time;

      if(side == KDSW_HIGH)
      {
         zone.zone_low = MathMax(bar.open, bar.close);
         zone.zone_high = bar.high;
         zone.extreme_price = bar.high;
      }
      else
      {
         zone.zone_low = bar.low;
         zone.zone_high = MathMin(bar.open, bar.close);
         zone.extreme_price = bar.low;
      }
   }

   bool BuildDayPair(const int day_age,
                     const datetime market_time,
                     KDSW_ZONE &high_zone,
                     KDSW_ZONE &low_zone)
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
      double best_low = DBL_MAX;
      int high_index = -1;
      int low_index = -1;

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
         ResetZone(high_zone, KDSW_HIGH, day_age, day_start, rates[high_index]);
      if(low_index >= 0)
         ResetZone(low_zone, KDSW_LOW, day_age, day_start, rates[low_index]);
      return high_zone.valid || low_zone.valid;
   }

   bool RegisterTouch(KDSW_ZONE &zone, const MqlRates &bar)
   {
      if(zone.last_touch_bar_time == bar.time)
         return false;

      zone.last_touch_bar_time = bar.time;
      zone.touches++;
      if(zone.first_touch_time == 0)
         zone.first_touch_time = bar.time;
      zone.last_touch_time = bar.time;
      if(zone.state == KDSW_FRESH)
         zone.state = KDSW_TOUCHED;
      return true;
   }

   bool ApplyClosedBar(KDSW_ZONE &zone, const MqlRates &bar)
   {
      if(!zone.valid || zone.state == KDSW_BROKEN || bar.time <= zone.source_time)
         return false;

      bool changed = false;
      bool overlaps = (bar.high >= zone.zone_low && bar.low <= zone.zone_high);

      if(zone.side == KDSW_HIGH)
      {
         if(bar.close > zone.zone_high)
         {
            zone.state = KDSW_BROKEN;
            zone.break_time = bar.time;
            return true;
         }

         if(overlaps)
         {
            if(RegisterTouch(zone, bar))
               changed = true;
            if(bar.close < zone.zone_low &&
               zone.last_rejection_bar_time != bar.time)
            {
               zone.last_rejection_bar_time = bar.time;
               zone.rejections++;
               zone.state = KDSW_REJECTED;
               changed = true;
            }
         }
      }
      else
      {
         if(bar.close < zone.zone_low)
         {
            zone.state = KDSW_BROKEN;
            zone.break_time = bar.time;
            return true;
         }

         if(overlaps)
         {
            if(RegisterTouch(zone, bar))
               changed = true;
            if(bar.close > zone.zone_high &&
               zone.last_rejection_bar_time != bar.time)
            {
               zone.last_rejection_bar_time = bar.time;
               zone.rejections++;
               zone.state = KDSW_REJECTED;
               changed = true;
            }
         }
      }
      return changed;
   }

   bool ApplyIntrabarTouch(KDSW_ZONE &zone, const MqlRates &bar)
   {
      if(!zone.valid || zone.state == KDSW_BROKEN || bar.time <= zone.source_time)
         return false;

      bool overlaps = (bar.high >= zone.zone_low && bar.low <= zone.zone_high);
      return overlaps && RegisterTouch(zone, bar);
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
                             market_time, history);
      if(copied <= 0)
         return;

      for(int b = 0; b < copied; b++)
      {
         for(int z = 0; z < m_days * 2; z++)
         {
            if(!m_cfg.confirm_reject_break_on_close)
               ApplyIntrabarTouch(m_zones[z], history[b]);
            ApplyClosedBar(m_zones[z], history[b]);
         }
      }
   }

   bool UpdateCurrentDayExtremeFromBar(const MqlRates &bar)
   {
      bool changed = false;
      if(!m_zones[0].valid ||
         bar.high > m_zones[0].extreme_price ||
         (bar.high == m_zones[0].extreme_price &&
          bar.time > m_zones[0].source_time))
      {
         ResetZone(m_zones[0], KDSW_HIGH, 0, m_current_day_start, bar);
         changed = true;
      }

      if(!m_zones[1].valid ||
         bar.low < m_zones[1].extreme_price ||
         (bar.low == m_zones[1].extreme_price &&
          bar.time > m_zones[1].source_time))
      {
         ResetZone(m_zones[1], KDSW_LOW, 0, m_current_day_start, bar);
         changed = true;
      }
      return changed;
   }

   bool ProcessClosedBar(const MqlRates &bar)
   {
      bool changed = false;
      for(int i = 0; i < m_days * 2; i++)
         if(ApplyClosedBar(m_zones[i], bar))
            changed = true;
      return changed;
   }

   bool ProcessIntrabar(const MqlRates &bar)
   {
      bool changed = false;
      for(int i = 0; i < m_days * 2; i++)
         if(ApplyIntrabarTouch(m_zones[i], bar))
            changed = true;
      return changed;
   }

   bool ZoneChanged(const KDSW_ZONE &a, const KDSW_ZONE &b)
   {
      if(a.valid != b.valid || a.side != b.side || a.state != b.state ||
         a.day_age != b.day_age || a.day_start != b.day_start ||
         a.source_time != b.source_time || a.touches != b.touches ||
         a.rejections != b.rejections || a.first_touch_time != b.first_touch_time ||
         a.last_touch_time != b.last_touch_time ||
         a.break_time != b.break_time)
         return true;

      return a.zone_low != b.zone_low ||
             a.zone_high != b.zone_high ||
             a.extreme_price != b.extreme_price;
   }

   string EventName(const KDSW_ZONE &previous,
                    const KDSW_ZONE &current)
   {
      if(!previous.valid)
         return "INIT";
      if(previous.day_start != current.day_start ||
         previous.source_time != current.source_time)
         return "EXTREME_REPLACED";
      if(previous.state != current.state)
         return "STATE_" + KDSW_StateName(current.state);
      if(current.touches > previous.touches)
         return "TOUCH";
      if(current.rejections > previous.rejections)
         return "REJECTION";
      if(previous.break_time != current.break_time)
         return "BREAK";
      return "OBSERVATION";
   }

public:
   CKittDailySwingStateMachine()
   {
      m_symbol = "";
      m_calc_tf = PERIOD_CURRENT;
      m_calc_period_sec = 60;
      m_days = KDSW_MAX_DAYS;
      m_initialized = false;
      m_is_tester = false;
      m_has_logged = false;
      m_dirty = false;
      m_current_day_start = 0;
      m_last_calc_bar_time = 0;
      m_market_time = 0;
      m_update_count = 0;
      KDSW_DefaultConfig(m_cfg);
      for(int i = 0; i < KDSW_MAX_ZONES; i++)
      {
         ZeroMemory(m_zones[i]);
         ZeroMemory(m_logged[i]);
      }
   }

   bool Init(const string symbol,
             const ENUM_TIMEFRAMES calc_tf,
             const KDSW_CONFIG &cfg)
   {
      m_symbol = (StringLen(symbol) > 0) ? symbol : _Symbol;
      m_calc_tf = (calc_tf == PERIOD_CURRENT)
                ? (ENUM_TIMEFRAMES)_Period
                : calc_tf;
      m_calc_period_sec = MathMax(PeriodSeconds(m_calc_tf), 1);
      m_cfg = cfg;
      m_days = ClampDays(m_cfg.days_to_keep);
      m_cfg.days_to_keep = m_days;
      m_is_tester = (bool)MQLInfoInteger(MQL_TESTER);
      Reset();

      datetime market_time = 0;
      if(!ResolveMarketTime(market_time))
         return false;
      return Rebuild(market_time);
   }

   void Reset()
   {
      for(int i = 0; i < KDSW_MAX_ZONES; i++)
      {
         ZeroMemory(m_zones[i]);
         ZeroMemory(m_logged[i]);
      }
      m_initialized = false;
      m_has_logged = false;
      m_dirty = false;
      m_current_day_start = 0;
      m_last_calc_bar_time = 0;
      m_market_time = 0;
      m_update_count = 0;
   }

   bool Rebuild(const datetime market_time)
   {
      if(market_time <= 0)
         return false;

      for(int i = 0; i < KDSW_MAX_ZONES; i++)
         ZeroMemory(m_zones[i]);

      for(int d = 0; d < m_days; d++)
      {
         KDSW_ZONE high_zone;
         KDSW_ZONE low_zone;
         if(!BuildDayPair(d, market_time, high_zone, low_zone))
            continue;
         m_zones[d * 2] = high_zone;
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
         m_last_calc_bar_time = 0;

      ReplayHistoryOnce(market_time);
      m_market_time = market_time;
      m_initialized = true;
      m_update_count++;
      m_dirty = true;
      return true;
   }

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
      bool changed = false;
      bool new_calc_bar = (m_last_calc_bar_time != 0 &&
                           current_bar.time != m_last_calc_bar_time);

      if(new_calc_bar && copied >= 2)
      {
         MqlRates closed_bar = pair[1];
         if(UpdateCurrentDayExtremeFromBar(closed_bar))
            changed = true;
         if(ProcessClosedBar(closed_bar))
            changed = true;
         m_last_calc_bar_time = current_bar.time;
         changed = true;
      }
      else if(m_last_calc_bar_time == 0)
         m_last_calc_bar_time = current_bar.time;

      if(UpdateCurrentDayExtremeFromBar(current_bar))
         changed = true;
      if(ProcessIntrabar(current_bar))
         changed = true;

      m_market_time = market_time;
      m_update_count++;
      m_dirty = m_dirty || changed;
      return true;
   }

   bool Initialized()
   {
      return m_initialized;
   }

   bool Dirty()
   {
      return m_dirty;
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
      int count = 0;
      for(int i = 0; i < m_days * 2; i++)
         if(m_zones[i].valid)
            count++;
      return count;
   }

   bool GetZone(const int day_age,
                const KDSW_ZONE_SIDE side,
                KDSW_ZONE &out_zone)
   {
      ZeroMemory(out_zone);
      if(day_age < 0 || day_age >= m_days)
         return false;

      int index = day_age * 2 + ((side == KDSW_HIGH) ? 0 : 1);
      if(index < 0 || index >= KDSW_MAX_ZONES || !m_zones[index].valid)
         return false;

      out_zone = m_zones[index];
      return true;
   }

   void GetReading(KDSW_READING &reading)
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
      reading.valid_zone_count = ZoneCount();
   }

   bool Log(CKittDailySwingLogger &logger, const bool log_every_update)
   {
      if(!m_initialized)
         return false;
      if(!logger.Enabled())
      {
         m_dirty = false;
         return false;
      }

      KDSW_READING reading;
      GetReading(reading);
      bool should_write = log_every_update || m_dirty || !m_has_logged;
      if(!should_write)
         return false;

      ulong sequence = logger.WriteFrame(reading);

      for(int i = 0; i < m_days * 2; i++)
      {
         if(!m_zones[i].valid)
            continue;

         bool changed = log_every_update || !m_has_logged ||
                        ZoneChanged(m_logged[i], m_zones[i]);
         if(changed)
         {
            logger.WriteZone(sequence, i, m_zones[i],
                             m_symbol, m_calc_tf);
            string event_name = log_every_update && m_has_logged
                              ? "OBSERVATION"
                              : EventName(m_logged[i], m_zones[i]);
            logger.WriteEvent(sequence, i, event_name, m_logged[i],
                              m_zones[i], m_market_time,
                              m_last_calc_bar_time);
         }
         m_logged[i] = m_zones[i];
      }

      m_has_logged = true;
      m_dirty = false;
      logger.Flush();
      return true;
   }
};

#endif // __KITT_DAILY_SWING_STATE_MACHINE_MQH__
