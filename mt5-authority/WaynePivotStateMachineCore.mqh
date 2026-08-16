//+------------------------------------------------------------------+
//| WaynePivotStateMachineCore.mqh                                  |
//| Standalone, trading-agnostic Wayne pivot geometry and grammar.   |
//| Fibot geometry is intentionally absent.                         |
//+------------------------------------------------------------------+
#ifndef __NORTHSTAR_WAYNE_PIVOT_STATE_MACHINE_CORE_MQH__
#define __NORTHSTAR_WAYNE_PIVOT_STATE_MACHINE_CORE_MQH__

#define WPE2E_SCHEMA_VERSION 1
#define WPE2E_BUILD_ID "WAYNE_PIVOT_E2E_STATE_MACHINE_V1"
#define WPE2E_MAX_PERIODS 200
#define WPE2E_LEVEL_COUNT 15
#define WPE2E_MAX_EVENTS 64

enum WPE2E_FAMILY
{
   WPE2E_FAMILY_PIVOT = 0,
   WPE2E_FAMILY_MID_PIVOT = 1,
   WPE2E_FAMILY_ZONE = 2
};

enum WPE2E_KIND
{
   WPE2E_KIND_NONE = 0,
   WPE2E_KIND_S3 = 1,
   WPE2E_KIND_M0 = 2,
   WPE2E_KIND_S2 = 3,
   WPE2E_KIND_M1 = 4,
   WPE2E_KIND_S1 = 5,
   WPE2E_KIND_M2 = 6,
   WPE2E_KIND_PP = 7,
   WPE2E_KIND_M3 = 8,
   WPE2E_KIND_R1 = 9,
   WPE2E_KIND_M4 = 10,
   WPE2E_KIND_R2 = 11,
   WPE2E_KIND_M5 = 12,
   WPE2E_KIND_R3 = 13,
   WPE2E_KIND_LOWER_ZONE = 20,
   WPE2E_KIND_UPPER_ZONE = 21
};

enum WPE2E_ROLE
{
   WPE2E_ROLE_LOWER_EXTREME = 0,
   WPE2E_ROLE_LOWER_MAJOR = 1,
   WPE2E_ROLE_LOWER_INTERMEDIATE = 2,
   WPE2E_ROLE_CENTER = 3,
   WPE2E_ROLE_UPPER_INTERMEDIATE = 4,
   WPE2E_ROLE_UPPER_MAJOR = 5,
   WPE2E_ROLE_UPPER_EXTREME = 6,
   WPE2E_ROLE_LOWER_ZONE = 7,
   WPE2E_ROLE_UPPER_ZONE = 8
};

enum WPE2E_STATE
{
   WPE2E_STATE_UNKNOWN = 0,
   WPE2E_STATE_FRESH = 1,
   WPE2E_STATE_TOUCHED = 2,
   WPE2E_STATE_TESTED = 3,
   WPE2E_STATE_ACCEPTED = 4,
   WPE2E_STATE_REJECTED = 5,
   WPE2E_STATE_BROKEN = 6,
   WPE2E_STATE_RECLAIMED = 7
};

enum WPE2E_EVENT_KIND
{
   WPE2E_EVENT_NONE = 0,
   WPE2E_EVENT_CREATE = 1,
   WPE2E_EVENT_TOUCH = 2,
   WPE2E_EVENT_RETEST = 3,
   WPE2E_EVENT_ACCEPTANCE = 4,
   WPE2E_EVENT_REJECTION = 5,
   WPE2E_EVENT_BREAK = 6,
   WPE2E_EVENT_RECLAIM = 7,
   WPE2E_EVENT_ROLLOVER = 8,
   WPE2E_EVENT_SNAPSHOT = 9,
   WPE2E_EVENT_FINALIZE = 10
};

struct WPE2E_PIVOT_BLOCK
{
   bool valid;
   int period_age;
   ENUM_TIMEFRAMES timeframe;
   datetime source_start;
   datetime source_end;
   datetime effective_start;
   datetime effective_end;
   double source_open;
   double source_high;
   double source_low;
   double source_close;
   double source_range;
   double PP;
   double R1;
   double R2;
   double R3;
   double S1;
   double S2;
   double S3;
   double M0;
   double M1;
   double M2;
   double M3;
   double M4;
   double M5;
};

struct WPE2E_LEVEL
{
   bool valid;
   ulong local_id;
   WPE2E_FAMILY family;
   WPE2E_KIND kind;
   WPE2E_ROLE role;
   ENUM_TIMEFRAMES timeframe;
   int period_age;
   double lower;
   double price;
   double center;
   double upper;
   double width;
   datetime source_start;
   datetime source_end;
   datetime effective_start;
   datetime effective_end;
   datetime updated_at;
   bool developing;
   bool frozen_geometry;
   WPE2E_STATE state;
   WPE2E_STATE previous_state;
   bool in_contact;
   int touches;
   int attempts;
   int contact_bars;
   int rejections;
   int breaks;
   int reclaims;
   int acceptance_bars;
   double max_excursion;
   double source_open;
   double source_high;
   double source_low;
   double source_close;
   double source_range;
};

struct WPE2E_EVENT_ROW
{
   bool valid;
   WPE2E_EVENT_KIND event;
   datetime market_time;
   ulong local_id;
   WPE2E_KIND kind;
   WPE2E_FAMILY family;
   WPE2E_STATE previous_state;
   WPE2E_STATE current_state;
   double lower;
   double price;
   double upper;
   int touches;
   int attempts;
   int contact_bars;
   int rejections;
   int breaks;
   int reclaims;
   int acceptance_bars;
};

struct WPE2E_CONFIG
{
   int periods_to_keep;
   bool include_standard_pivots;
   bool include_mid_pivots;
   bool include_zones;
   int interaction_buffer_points;
   int acceptance_bars;
};

string WPE2E_KindName(const WPE2E_KIND kind)
{
   if(kind == WPE2E_KIND_S3) return "S3";
   if(kind == WPE2E_KIND_M0) return "M0";
   if(kind == WPE2E_KIND_S2) return "S2";
   if(kind == WPE2E_KIND_M1) return "M1";
   if(kind == WPE2E_KIND_S1) return "S1";
   if(kind == WPE2E_KIND_M2) return "M2";
   if(kind == WPE2E_KIND_PP) return "PP";
   if(kind == WPE2E_KIND_M3) return "M3";
   if(kind == WPE2E_KIND_R1) return "R1";
   if(kind == WPE2E_KIND_M4) return "M4";
   if(kind == WPE2E_KIND_R2) return "R2";
   if(kind == WPE2E_KIND_M5) return "M5";
   if(kind == WPE2E_KIND_R3) return "R3";
   if(kind == WPE2E_KIND_LOWER_ZONE) return "M1_S2_ZONE";
   if(kind == WPE2E_KIND_UPPER_ZONE) return "M4_R2_ZONE";
   return "NONE";
}

string WPE2E_FamilyName(const WPE2E_FAMILY family)
{
   if(family == WPE2E_FAMILY_PIVOT) return "PIVOT";
   if(family == WPE2E_FAMILY_MID_PIVOT) return "MID_PIVOT";
   if(family == WPE2E_FAMILY_ZONE) return "ZONE";
   return "NONE";
}

string WPE2E_RoleName(const WPE2E_ROLE role)
{
   if(role == WPE2E_ROLE_LOWER_EXTREME) return "LOWER_EXTREME";
   if(role == WPE2E_ROLE_LOWER_MAJOR) return "LOWER_MAJOR";
   if(role == WPE2E_ROLE_LOWER_INTERMEDIATE) return "LOWER_INTERMEDIATE";
   if(role == WPE2E_ROLE_CENTER) return "CENTER";
   if(role == WPE2E_ROLE_UPPER_INTERMEDIATE) return "UPPER_INTERMEDIATE";
   if(role == WPE2E_ROLE_UPPER_MAJOR) return "UPPER_MAJOR";
   if(role == WPE2E_ROLE_UPPER_EXTREME) return "UPPER_EXTREME";
   if(role == WPE2E_ROLE_LOWER_ZONE) return "LOWER_ZONE";
   if(role == WPE2E_ROLE_UPPER_ZONE) return "UPPER_ZONE";
   return "UNKNOWN";
}

string WPE2E_StateName(const WPE2E_STATE state)
{
   if(state == WPE2E_STATE_FRESH) return "FRESH";
   if(state == WPE2E_STATE_TOUCHED) return "TOUCHED";
   if(state == WPE2E_STATE_TESTED) return "TESTED";
   if(state == WPE2E_STATE_ACCEPTED) return "ACCEPTED";
   if(state == WPE2E_STATE_REJECTED) return "REJECTED";
   if(state == WPE2E_STATE_BROKEN) return "BROKEN";
   if(state == WPE2E_STATE_RECLAIMED) return "RECLAIMED";
   return "UNKNOWN";
}

string WPE2E_EventName(const WPE2E_EVENT_KIND event)
{
   if(event == WPE2E_EVENT_CREATE) return "CREATE";
   if(event == WPE2E_EVENT_TOUCH) return "TOUCH";
   if(event == WPE2E_EVENT_RETEST) return "RETEST";
   if(event == WPE2E_EVENT_ACCEPTANCE) return "ACCEPTANCE";
   if(event == WPE2E_EVENT_REJECTION) return "REJECTION";
   if(event == WPE2E_EVENT_BREAK) return "BREAK";
   if(event == WPE2E_EVENT_RECLAIM) return "RECLAIM";
   if(event == WPE2E_EVENT_ROLLOVER) return "ROLLOVER";
   if(event == WPE2E_EVENT_SNAPSHOT) return "SNAPSHOT";
   if(event == WPE2E_EVENT_FINALIZE) return "FINALIZE";
   return "NONE";
}

void WPE2E_DefaultConfig(WPE2E_CONFIG &cfg)
{
   cfg.periods_to_keep = 20;
   cfg.include_standard_pivots = true;
   cfg.include_mid_pivots = true;
   cfg.include_zones = true;
   cfg.interaction_buffer_points = 2;
   cfg.acceptance_bars = 3;
}

ulong WPE2E_HashMix(const ulong previous, const ulong value)
{
   ulong hash = previous == 0 ? 1469598103934665603 : previous;
   hash ^= value + 0x9E3779B97F4A7C15;
   hash *= 1099511628211;
   hash ^= hash >> 29;
   return hash;
}

ulong WPE2E_HashText(const string text)
{
   ulong hash = 1469598103934665603;
   int length = StringLen(text);
   for(int i = 0; i < length; i++)
      hash = WPE2E_HashMix(hash, (ulong)StringGetCharacter(text, i));
   return hash;
}

ulong WPE2E_SourceKey(const ENUM_TIMEFRAMES timeframe,
                     const datetime source_start,
                     const int period_age)
{
   ulong hash = WPE2E_HashText(WPE2E_BUILD_ID);
   hash = WPE2E_HashMix(hash, (ulong)timeframe);
   hash = WPE2E_HashMix(hash, (ulong)source_start);
   hash = WPE2E_HashMix(hash, (ulong)period_age);
   return hash;
}

double WPE2E_Clamp(const double value, const double lower, const double upper)
{
   return MathMax(lower, MathMin(value, upper));
}

void WPE2E_ResetLevel(WPE2E_LEVEL &level)
{
   ZeroMemory(level);
   level.state = WPE2E_STATE_UNKNOWN;
   level.previous_state = WPE2E_STATE_UNKNOWN;
}

void WPE2E_CalculateBlock(const MqlRates &source,
                         const int period_age,
                         const ENUM_TIMEFRAMES timeframe,
                         const int period_seconds,
                         WPE2E_PIVOT_BLOCK &block)
{
   ZeroMemory(block);
   block.valid = source.time > 0 && source.high >= source.low;
   if(!block.valid) return;
   block.period_age = period_age;
   block.timeframe = timeframe;
   block.source_start = source.time;
   block.source_end = source.time + period_seconds;
   block.effective_start = block.source_end;
   block.effective_end = block.effective_start + period_seconds;
   block.source_open = source.open;
   block.source_high = source.high;
   block.source_low = source.low;
   block.source_close = source.close;
   block.source_range = MathMax(source.high - source.low, 0.0);
   block.PP = (source.high + source.low + source.close) / 3.0;
   block.R1 = 2.0 * block.PP - source.low;
   block.R2 = block.PP + block.source_range;
   block.R3 = 2.0 * block.PP + source.high - 2.0 * source.low;
   block.S1 = 2.0 * block.PP - source.high;
   block.S2 = block.PP - block.source_range;
   block.S3 = 2.0 * block.PP - 2.0 * source.high + source.low;
   block.M0 = 0.5 * (block.S2 + block.S3);
   block.M1 = 0.5 * (block.S1 + block.S2);
   block.M2 = 0.5 * (block.PP + block.S1);
   block.M3 = 0.5 * (block.PP + block.R1);
   block.M4 = 0.5 * (block.R1 + block.R2);
   block.M5 = 0.5 * (block.R2 + block.R3);
}

void WPE2E_SetPoint(WPE2E_LEVEL &level,
                    const ulong local_id,
                    const WPE2E_FAMILY family,
                    const WPE2E_KIND kind,
                    const WPE2E_ROLE role,
                    const double price,
                    const WPE2E_PIVOT_BLOCK &block)
{
   WPE2E_ResetLevel(level);
   level.valid = block.valid;
   level.local_id = local_id;
   level.family = family;
   level.kind = kind;
   level.role = role;
   level.timeframe = block.timeframe;
   level.period_age = block.period_age;
   level.lower = price;
   level.price = price;
   level.center = price;
   level.upper = price;
   level.width = 0.0;
   level.source_start = block.source_start;
   level.source_end = block.source_end;
   level.effective_start = block.effective_start;
   level.effective_end = block.effective_end;
   level.source_open = block.source_open;
   level.source_high = block.source_high;
   level.source_low = block.source_low;
   level.source_close = block.source_close;
   level.source_range = block.source_range;
   level.state = WPE2E_STATE_FRESH;
   level.previous_state = WPE2E_STATE_UNKNOWN;
   level.frozen_geometry = true;
}

void WPE2E_SetZone(WPE2E_LEVEL &level,
                   const ulong local_id,
                   const WPE2E_KIND kind,
                   const WPE2E_ROLE role,
                   const double lower,
                   const double upper,
                   const WPE2E_PIVOT_BLOCK &block)
{
   WPE2E_ResetLevel(level);
   level.valid = block.valid;
   level.local_id = local_id;
   level.family = WPE2E_FAMILY_ZONE;
   level.kind = kind;
   level.role = role;
   level.timeframe = block.timeframe;
   level.period_age = block.period_age;
   level.lower = MathMin(lower, upper);
   level.upper = MathMax(lower, upper);
   level.center = 0.5 * (level.lower + level.upper);
   level.price = level.center;
   level.width = level.upper - level.lower;
   level.source_start = block.source_start;
   level.source_end = block.source_end;
   level.effective_start = block.effective_start;
   level.effective_end = block.effective_end;
   level.source_open = block.source_open;
   level.source_high = block.source_high;
   level.source_low = block.source_low;
   level.source_close = block.source_close;
   level.source_range = block.source_range;
   level.state = WPE2E_STATE_FRESH;
   level.previous_state = WPE2E_STATE_UNKNOWN;
   level.frozen_geometry = true;
}

int WPE2E_ExportLevels(const WPE2E_PIVOT_BLOCK &block,
                       const WPE2E_CONFIG &cfg,
                       WPE2E_LEVEL &levels[])
{
   ArrayResize(levels, WPE2E_LEVEL_COUNT);
   for(int i = 0; i < WPE2E_LEVEL_COUNT; i++)
      WPE2E_ResetLevel(levels[i]);
   if(!block.valid) return 0;

   WPE2E_SetPoint(levels[0], 0, WPE2E_FAMILY_PIVOT, WPE2E_KIND_S3, WPE2E_ROLE_LOWER_EXTREME, block.S3, block);
   WPE2E_SetPoint(levels[1], 1, WPE2E_FAMILY_MID_PIVOT, WPE2E_KIND_M0, WPE2E_ROLE_LOWER_INTERMEDIATE, block.M0, block);
   WPE2E_SetPoint(levels[2], 2, WPE2E_FAMILY_PIVOT, WPE2E_KIND_S2, WPE2E_ROLE_LOWER_MAJOR, block.S2, block);
   WPE2E_SetPoint(levels[3], 3, WPE2E_FAMILY_MID_PIVOT, WPE2E_KIND_M1, WPE2E_ROLE_LOWER_INTERMEDIATE, block.M1, block);
   WPE2E_SetPoint(levels[4], 4, WPE2E_FAMILY_PIVOT, WPE2E_KIND_S1, WPE2E_ROLE_LOWER_MAJOR, block.S1, block);
   WPE2E_SetPoint(levels[5], 5, WPE2E_FAMILY_MID_PIVOT, WPE2E_KIND_M2, WPE2E_ROLE_LOWER_INTERMEDIATE, block.M2, block);
   WPE2E_SetPoint(levels[6], 6, WPE2E_FAMILY_PIVOT, WPE2E_KIND_PP, WPE2E_ROLE_CENTER, block.PP, block);
   WPE2E_SetPoint(levels[7], 7, WPE2E_FAMILY_MID_PIVOT, WPE2E_KIND_M3, WPE2E_ROLE_UPPER_INTERMEDIATE, block.M3, block);
   WPE2E_SetPoint(levels[8], 8, WPE2E_FAMILY_PIVOT, WPE2E_KIND_R1, WPE2E_ROLE_UPPER_MAJOR, block.R1, block);
   WPE2E_SetPoint(levels[9], 9, WPE2E_FAMILY_MID_PIVOT, WPE2E_KIND_M4, WPE2E_ROLE_UPPER_INTERMEDIATE, block.M4, block);
   WPE2E_SetPoint(levels[10], 10, WPE2E_FAMILY_PIVOT, WPE2E_KIND_R2, WPE2E_ROLE_UPPER_MAJOR, block.R2, block);
   WPE2E_SetPoint(levels[11], 11, WPE2E_FAMILY_MID_PIVOT, WPE2E_KIND_M5, WPE2E_ROLE_UPPER_INTERMEDIATE, block.M5, block);
   WPE2E_SetPoint(levels[12], 12, WPE2E_FAMILY_PIVOT, WPE2E_KIND_R3, WPE2E_ROLE_UPPER_EXTREME, block.R3, block);
   WPE2E_SetZone(levels[13], 13, WPE2E_KIND_LOWER_ZONE, WPE2E_ROLE_LOWER_ZONE, block.S2, block.M1, block);
   WPE2E_SetZone(levels[14], 14, WPE2E_KIND_UPPER_ZONE, WPE2E_ROLE_UPPER_ZONE, block.M4, block.R2, block);

   if(!cfg.include_standard_pivots)
      for(int i = 0; i < WPE2E_LEVEL_COUNT; i++)
         if(levels[i].family == WPE2E_FAMILY_PIVOT) levels[i].valid = false;
   if(!cfg.include_mid_pivots)
      for(int i = 0; i < WPE2E_LEVEL_COUNT; i++)
         if(levels[i].family == WPE2E_FAMILY_MID_PIVOT) levels[i].valid = false;
   if(!cfg.include_zones)
      for(int i = 0; i < WPE2E_LEVEL_COUNT; i++)
         if(levels[i].family == WPE2E_FAMILY_ZONE) levels[i].valid = false;
   return WPE2E_LEVEL_COUNT;
}

void WPE2E_MakeEvent(const WPE2E_LEVEL &level,
                     const WPE2E_EVENT_KIND event,
                     const datetime market_time,
                     WPE2E_EVENT_ROW &row)
{
   ZeroMemory(row);
   row.valid = true;
   row.event = event;
   row.market_time = market_time;
   row.local_id = level.local_id;
   row.kind = level.kind;
   row.family = level.family;
   row.previous_state = level.previous_state;
   row.current_state = level.state;
   row.lower = level.lower;
   row.price = level.price;
   row.upper = level.upper;
   row.touches = level.touches;
   row.attempts = level.attempts;
   row.contact_bars = level.contact_bars;
   row.rejections = level.rejections;
   row.breaks = level.breaks;
   row.reclaims = level.reclaims;
   row.acceptance_bars = level.acceptance_bars;
}

bool WPE2E_ObserveLevel(WPE2E_LEVEL &level,
                        const datetime market_time,
                        const double bar_high,
                        const double bar_low,
                        const double bar_close,
                        const double point,
                        const WPE2E_CONFIG &cfg,
                        WPE2E_EVENT_ROW &event)
{
   if(!level.valid) return false;
   double tolerance = MathMax(point * (double)MathMax(cfg.interaction_buffer_points, 1), point);
   bool intersects = bar_high >= level.lower - tolerance && bar_low <= level.upper + tolerance;
   bool breached = bar_close > level.upper + tolerance || bar_close < level.lower - tolerance;
   bool had_contact = level.in_contact;
   level.updated_at = market_time;
   double excursion = 0.0;
   if(bar_close > level.upper) excursion = bar_close - level.upper;
   if(bar_close < level.lower) excursion = level.lower - bar_close;
   level.max_excursion = MathMax(level.max_excursion, excursion);

   if(intersects)
   {
      if(!had_contact)
      {
         level.in_contact = true;
         level.touches++;
         level.attempts++;
         level.contact_bars = 1;
         level.previous_state = level.state;
         if(level.state == WPE2E_STATE_BROKEN)
         {
            level.state = WPE2E_STATE_RECLAIMED;
            level.reclaims++;
            WPE2E_MakeEvent(level, WPE2E_EVENT_RECLAIM, market_time, event);
            return true;
         }
         if(level.state == WPE2E_STATE_REJECTED)
         {
            level.state = WPE2E_STATE_TESTED;
            WPE2E_MakeEvent(level, WPE2E_EVENT_RETEST, market_time, event);
            return true;
         }
         level.state = level.state == WPE2E_STATE_FRESH ? WPE2E_STATE_TOUCHED : WPE2E_STATE_TESTED;
         WPE2E_MakeEvent(level, WPE2E_EVENT_TOUCH, market_time, event);
         return true;
      }
      level.contact_bars++;
      if((level.state == WPE2E_STATE_TOUCHED || level.state == WPE2E_STATE_TESTED) &&
         level.contact_bars >= MathMax(cfg.acceptance_bars, 1))
      {
         level.previous_state = level.state;
         level.state = WPE2E_STATE_ACCEPTED;
         level.acceptance_bars = level.contact_bars;
         WPE2E_MakeEvent(level, WPE2E_EVENT_ACCEPTANCE, market_time, event);
         return true;
      }
      return false;
   }

   if(had_contact)
   {
      level.in_contact = false;
      level.contact_bars = 0;
      level.previous_state = level.state;
      if(breached)
      {
         level.state = WPE2E_STATE_BROKEN;
         level.breaks++;
         WPE2E_MakeEvent(level, WPE2E_EVENT_BREAK, market_time, event);
         return true;
      }
      if(level.state == WPE2E_STATE_TOUCHED || level.state == WPE2E_STATE_TESTED)
      {
         level.state = WPE2E_STATE_REJECTED;
         level.rejections++;
         WPE2E_MakeEvent(level, WPE2E_EVENT_REJECTION, market_time, event);
         return true;
      }
   }
   return false;
}

ulong WPE2E_HashLevel(const ulong previous, const WPE2E_LEVEL &level)
{
   ulong hash = previous;
   hash = WPE2E_HashMix(hash, level.local_id);
   hash = WPE2E_HashMix(hash, (ulong)level.kind);
   hash = WPE2E_HashMix(hash, (ulong)level.state);
   hash = WPE2E_HashMix(hash, (ulong)level.touches);
   hash = WPE2E_HashMix(hash, (ulong)level.rejections);
   hash = WPE2E_HashMix(hash, (ulong)level.breaks);
   hash = WPE2E_HashMix(hash, (ulong)level.reclaims);
   hash = WPE2E_HashMix(hash, (ulong)level.source_start);
   hash = WPE2E_HashMix(hash, (ulong)MathRound(level.lower / _Point));
   hash = WPE2E_HashMix(hash, (ulong)MathRound(level.upper / _Point));
   return hash;
}

ulong WPE2E_HashLevels(const WPE2E_LEVEL &levels[])
{
   ulong hash = WPE2E_HashText(WPE2E_BUILD_ID);
   int count = MathMin(ArraySize(levels), WPE2E_LEVEL_COUNT);
   for(int i = 0; i < count; i++)
      if(levels[i].valid) hash = WPE2E_HashLevel(hash, levels[i]);
   return hash;
}

class CWaynePivotE2ELogger
{
private:
   bool m_enabled;
   bool m_finalized;
   int m_frames_handle;
   int m_levels_handle;
   int m_zones_handle;
   int m_events_handle;
   int m_receipt_handle;
   int m_flush_interval;
   int m_pending;
   ulong m_root;
   ulong m_frames;
   ulong m_levels;
   ulong m_zones;
   ulong m_events;
   string m_symbol;
   string m_timeframe;
   string m_run_key;
   string m_invocation_id;

   string SafeToken(string value)
   {
      StringReplace(value, "\\", "_");
      StringReplace(value, "/", "_");
      StringReplace(value, ":", "_");
      StringReplace(value, " ", "_");
      StringReplace(value, ".", "_");
      return value;
   }

   int OpenAppend(const string filename, const string header)
   {
      int handle = FileOpen(filename, FILE_READ|FILE_WRITE|FILE_CSV|FILE_ANSI|FILE_COMMON, '\t');
      if(handle == INVALID_HANDLE) return INVALID_HANDLE;
      if(FileSize(handle) == 0) FileWriteString(handle, header + "\r\n");
      FileSeek(handle, 0, SEEK_END);
      return handle;
   }

   void MixText(const string text)
   {
      m_root = WPE2E_HashMix(m_root, WPE2E_HashText(text));
   }

public:
   CWaynePivotE2ELogger(void)
   {
      m_enabled = false;
      m_finalized = false;
      m_frames_handle = INVALID_HANDLE;
      m_levels_handle = INVALID_HANDLE;
      m_zones_handle = INVALID_HANDLE;
      m_events_handle = INVALID_HANDLE;
      m_receipt_handle = INVALID_HANDLE;
      m_flush_interval = 10;
      m_pending = 0;
      m_root = WPE2E_HashText(WPE2E_BUILD_ID);
      m_frames = 0;
      m_levels = 0;
      m_zones = 0;
      m_events = 0;
   }

   bool Init(const bool enabled,
             const string symbol,
             const ENUM_TIMEFRAMES timeframe,
             const string instance,
             const string run_key,
             const string invocation_id,
             const int flush_interval = 10)
   {
      Close();
      m_enabled = enabled;
      m_finalized = false;
      m_flush_interval = MathMax(flush_interval, 1);
      m_pending = 0;
      m_root = WPE2E_HashText(WPE2E_BUILD_ID);
      m_frames = 0;
      m_levels = 0;
      m_zones = 0;
      m_events = 0;
      m_symbol = symbol;
      m_timeframe = EnumToString(timeframe);
      m_run_key = SafeToken(run_key);
      m_invocation_id = SafeToken(invocation_id);
      if(!m_enabled) return true;

      string stem = "WaynePivotE2E_" + SafeToken(symbol) + "_" +
                    SafeToken(m_timeframe) + "_" + SafeToken(instance);
      m_frames_handle = OpenAppend(stem + "_frames.tsv",
         "schema\trun_key\tinvocation_id\tframe\tmarket_time\tperiod_start\tsource_time\treference_price\tactive_levels\tactive_zones\tevents\tframe_hash");
      m_levels_handle = OpenAppend(stem + "_levels.tsv",
         "schema\trun_key\tinvocation_id\tframe\tmarket_time\tlocal_id\tfamily\tkind\trole\tperiod_age\tlower\tprice\tupper\tstate\ttouches\tattempts\tcontact_bars\trejections\tbreaks\treclaims\tacceptance_bars\tmax_excursion\tsource_time\teffective_start\teffective_end");
      m_zones_handle = OpenAppend(stem + "_zones.tsv",
         "schema\trun_key\tinvocation_id\tframe\tmarket_time\tlocal_id\tkind\trole\tlower\tcenter\tupper\twidth\tstate\ttouches\tattempts\tcontact_bars\trejections\tbreaks\treclaims\tacceptance_bars\tmax_excursion\tsource_time");
      m_events_handle = OpenAppend(stem + "_events.tsv",
         "schema\trun_key\tinvocation_id\tmarket_time\tevent\tlocal_id\tfamily\tkind\tprevious_state\tcurrent_state\tlower\tprice\tupper\ttouches\tattempts\tcontact_bars\trejections\tbreaks\treclaims\tacceptance_bars");
      m_receipt_handle = OpenAppend(stem + "_receipt.tsv",
         "schema\trun_key\tinvocation_id\tstate\tsymbol\ttimeframe\tframes\tlevels\tzones\tevents\troot_hash\treason");
      if(m_frames_handle == INVALID_HANDLE || m_levels_handle == INVALID_HANDLE ||
         m_zones_handle == INVALID_HANDLE || m_events_handle == INVALID_HANDLE ||
         m_receipt_handle == INVALID_HANDLE)
      {
         Close();
         m_enabled = false;
         return false;
      }
      FileWrite(m_receipt_handle, WPE2E_SCHEMA_VERSION, m_run_key, m_invocation_id,
                "OPEN", m_symbol, m_timeframe, 0, 0, 0, 0,
                StringFormat("%I64u", m_root), "START");
      FileFlush(m_receipt_handle);
      return true;
   }

   bool WriteEvent(const WPE2E_EVENT_ROW &event)
   {
      if(!m_enabled || !event.valid || m_events_handle == INVALID_HANDLE) return true;
      FileWrite(m_events_handle, WPE2E_SCHEMA_VERSION, m_run_key, m_invocation_id,
                TimeToString(event.market_time, TIME_DATE|TIME_MINUTES|TIME_SECONDS),
                WPE2E_EventName(event.event), StringFormat("%I64u", event.local_id),
                WPE2E_FamilyName(event.family), WPE2E_KindName(event.kind),
                WPE2E_StateName(event.previous_state), WPE2E_StateName(event.current_state),
                DoubleToString(event.lower, _Digits), DoubleToString(event.price, _Digits),
                DoubleToString(event.upper, _Digits), event.touches, event.attempts,
                event.contact_bars, event.rejections, event.breaks, event.reclaims,
                event.acceptance_bars);
      m_events++;
      m_root = WPE2E_HashMix(m_root, (ulong)event.event);
      m_root = WPE2E_HashMix(m_root, event.local_id);
      m_root = WPE2E_HashMix(m_root, (ulong)event.market_time);
      m_root = WPE2E_HashMix(m_root, (ulong)event.current_state);
      m_pending++;
      FlushIfNeeded();
      return true;
   }

   bool WriteFrame(const ulong frame,
                   const datetime market_time,
                   const datetime period_start,
                   const datetime source_time,
                   const double reference_price,
                   const int active_levels,
                   const int active_zones,
                   const int frame_events,
                   const ulong frame_hash,
                   const WPE2E_LEVEL &levels[])
   {
      if(!m_enabled) return true;
      FileWrite(m_frames_handle, WPE2E_SCHEMA_VERSION, m_run_key, m_invocation_id,
                StringFormat("%I64u", frame),
                TimeToString(market_time, TIME_DATE|TIME_MINUTES|TIME_SECONDS),
                TimeToString(period_start, TIME_DATE|TIME_MINUTES|TIME_SECONDS),
                TimeToString(source_time, TIME_DATE|TIME_MINUTES|TIME_SECONDS),
                DoubleToString(reference_price, _Digits), active_levels, active_zones,
                frame_events, StringFormat("%I64u", frame_hash));
      m_frames++;
      for(int i = 0; i < MathMin(ArraySize(levels), WPE2E_LEVEL_COUNT); i++)
      {
         if(!levels[i].valid) continue;
         const WPE2E_LEVEL level = levels[i];
         FileWrite(m_levels_handle, WPE2E_SCHEMA_VERSION, m_run_key, m_invocation_id,
                   StringFormat("%I64u", frame),
                   TimeToString(market_time, TIME_DATE|TIME_MINUTES|TIME_SECONDS),
                   StringFormat("%I64u", level.local_id), WPE2E_FamilyName(level.family),
                   WPE2E_KindName(level.kind), WPE2E_RoleName(level.role), level.period_age,
                   DoubleToString(level.lower, _Digits), DoubleToString(level.price, _Digits),
                   DoubleToString(level.upper, _Digits), WPE2E_StateName(level.state),
                   level.touches, level.attempts, level.contact_bars, level.rejections,
                   level.breaks, level.reclaims, level.acceptance_bars,
                   DoubleToString(level.max_excursion, _Digits),
                   TimeToString(level.source_start, TIME_DATE|TIME_MINUTES|TIME_SECONDS),
                   TimeToString(level.effective_start, TIME_DATE|TIME_MINUTES|TIME_SECONDS),
                   TimeToString(level.effective_end, TIME_DATE|TIME_MINUTES|TIME_SECONDS));
         m_levels++;
         if(level.family == WPE2E_FAMILY_ZONE)
         {
            FileWrite(m_zones_handle, WPE2E_SCHEMA_VERSION, m_run_key, m_invocation_id,
                      StringFormat("%I64u", frame),
                      TimeToString(market_time, TIME_DATE|TIME_MINUTES|TIME_SECONDS),
                      StringFormat("%I64u", level.local_id), WPE2E_KindName(level.kind),
                      WPE2E_RoleName(level.role), DoubleToString(level.lower, _Digits),
                      DoubleToString(level.center, _Digits), DoubleToString(level.upper, _Digits),
                      DoubleToString(level.width, _Digits), WPE2E_StateName(level.state),
                      level.touches, level.attempts, level.contact_bars, level.rejections,
                      level.breaks, level.reclaims, level.acceptance_bars,
                      DoubleToString(level.max_excursion, _Digits),
                      TimeToString(level.source_start, TIME_DATE|TIME_MINUTES|TIME_SECONDS));
            m_zones++;
         }
      }
      m_root = WPE2E_HashMix(m_root, frame_hash);
      m_root = WPE2E_HashMix(m_root, frame);
      m_pending++;
      FlushIfNeeded();
      return true;
   }

   void FlushIfNeeded(void)
   {
      if(m_pending < m_flush_interval) return;
      FileFlush(m_frames_handle);
      FileFlush(m_levels_handle);
      FileFlush(m_zones_handle);
      FileFlush(m_events_handle);
      m_pending = 0;
   }

   void Finalize(const string reason)
   {
      if(!m_enabled || m_finalized) return;
      m_finalized = true;
      FileWrite(m_events_handle, WPE2E_SCHEMA_VERSION, m_run_key, m_invocation_id,
                TimeToString(TimeCurrent(), TIME_DATE|TIME_MINUTES|TIME_SECONDS),
                WPE2E_EventName(WPE2E_EVENT_FINALIZE), "-1", "NONE", "NONE",
                "UNKNOWN", "UNKNOWN", "0", "0", "0", 0, 0, 0, 0, 0, 0, 0);
      m_events++;
      m_root = WPE2E_HashMix(m_root, (ulong)WPE2E_EVENT_FINALIZE);
      FileWrite(m_receipt_handle, WPE2E_SCHEMA_VERSION, m_run_key, m_invocation_id,
                "CLOSED", m_symbol, m_timeframe, StringFormat("%I64u", m_frames),
                StringFormat("%I64u", m_levels), StringFormat("%I64u", m_zones),
                StringFormat("%I64u", m_events), StringFormat("%I64u", m_root), reason);
      FileFlush(m_frames_handle);
      FileFlush(m_levels_handle);
      FileFlush(m_zones_handle);
      FileFlush(m_events_handle);
      FileFlush(m_receipt_handle);
   }

   ulong RootHash(void) const { return m_root; }
   ulong FrameCount(void) const { return m_frames; }
   ulong LevelCount(void) const { return m_levels; }
   ulong ZoneCount(void) const { return m_zones; }
   ulong EventCount(void) const { return m_events; }

   void Close(void)
   {
      if(m_frames_handle != INVALID_HANDLE) { FileFlush(m_frames_handle); FileClose(m_frames_handle); m_frames_handle = INVALID_HANDLE; }
      if(m_levels_handle != INVALID_HANDLE) { FileFlush(m_levels_handle); FileClose(m_levels_handle); m_levels_handle = INVALID_HANDLE; }
      if(m_zones_handle != INVALID_HANDLE) { FileFlush(m_zones_handle); FileClose(m_zones_handle); m_zones_handle = INVALID_HANDLE; }
      if(m_events_handle != INVALID_HANDLE) { FileFlush(m_events_handle); FileClose(m_events_handle); m_events_handle = INVALID_HANDLE; }
      if(m_receipt_handle != INVALID_HANDLE) { FileFlush(m_receipt_handle); FileClose(m_receipt_handle); m_receipt_handle = INVALID_HANDLE; }
   }
};

#endif // __NORTHSTAR_WAYNE_PIVOT_STATE_MACHINE_CORE_MQH__
