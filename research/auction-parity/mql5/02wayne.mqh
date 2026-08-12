//+------------------------------------------------------------------+
//|                                                     02wayne.mqh  |
//|  Headless Wayne/Wyatt pivot producer for master structure engine |
//|                                                                  |
//|  Emits only completed-source-period structure:                    |
//|    PP, R1-R3, S1-S3                                               |
//|    M0-M5 midpoint pivots                                          |
//|    optional original buy/sell structural zones                    |
//|                                                                  |
//|  Intentionally excluded:                                          |
//|    incomplete-source projections                                  |
//|    Fibot levels                                                   |
//|    chart objects / buffers / indicator lifecycle                  |
//+------------------------------------------------------------------+
#ifndef __KITT_02_WAYNE_MQH__
#define __KITT_02_WAYNE_MQH__

#define WYN_MAX_PERIODS 200

enum ENUM_WYN_FAMILY
{
   WYN_FAMILY_PIVOT = 0,
   WYN_FAMILY_MID_PIVOT = 1,
   WYN_FAMILY_ZONE = 2
};

enum ENUM_WYN_KIND
{
   WYN_KIND_NONE = 0,

   WYN_KIND_S3 = 1,
   WYN_KIND_M0 = 2,
   WYN_KIND_S2 = 3,
   WYN_KIND_M1 = 4,
   WYN_KIND_S1 = 5,
   WYN_KIND_M2 = 6,
   WYN_KIND_PP = 7,
   WYN_KIND_M3 = 8,
   WYN_KIND_R1 = 9,
   WYN_KIND_M4 = 10,
   WYN_KIND_R2 = 11,
   WYN_KIND_M5 = 12,
   WYN_KIND_R3 = 13,

   WYN_KIND_BUY_ZONE = 20,
   WYN_KIND_SELL_ZONE = 21
};

enum ENUM_WYN_ROLE
{
   WYN_ROLE_LOWER_EXTREME = 0,
   WYN_ROLE_LOWER_MAJOR = 1,
   WYN_ROLE_LOWER_INTERMEDIATE = 2,
   WYN_ROLE_CENTER = 3,
   WYN_ROLE_UPPER_INTERMEDIATE = 4,
   WYN_ROLE_UPPER_MAJOR = 5,
   WYN_ROLE_UPPER_EXTREME = 6,
   WYN_ROLE_LOWER_ZONE = 7,
   WYN_ROLE_UPPER_ZONE = 8
};

// Raw calculated block from one completed source period.
// period_age = 0 means the levels currently applicable now.
// Its source OHLC is always the immediately preceding COMPLETED period.
struct WYN_PivotBlock
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

// Normalized producer output consumed by the future master controller.
// Point levels use lower == price == center == upper.
// Zones retain real lower/upper geometry.
struct WYN_Level
{
   bool valid;
   ulong local_id;

   ENUM_WYN_FAMILY family;
   ENUM_WYN_KIND kind;
   ENUM_WYN_ROLE role;

   ENUM_TIMEFRAMES timeframe;
   int period_age;

   double lower;
   double price;
   double center;
   double upper;
   double width;

   datetime source_start;
   datetime source_end;
   datetime created_at;
   datetime effective_start;
   datetime effective_end;

   bool developing;
   bool frozen_geometry;

   // Provenance from the completed source period.
   double source_open;
   double source_high;
   double source_low;
   double source_close;
   double source_range;

   // Useful raw geometry for the master without forcing a second lookup.
   double distance_from_pp;
   double position_in_source_range;
};

struct WYN_Config
{
   int periods_to_keep;

   bool include_standard_pivots;
   bool include_m_pivots;
   bool include_zones;
};

struct WYN_Reading
{
   bool ready;
   string symbol;
   ENUM_TIMEFRAMES timeframe;

   int blocks;
   datetime current_period_start;
   datetime last_rebuild_time;

   double current_PP;
   double current_R1;
   double current_R2;
   double current_R3;
   double current_S1;
   double current_S2;
   double current_S3;

   double current_M0;
   double current_M1;
   double current_M2;
   double current_M3;
   double current_M4;
   double current_M5;
};

void WYN_DefaultConfig(WYN_Config &cfg)
{
   cfg.periods_to_keep = 20;
   cfg.include_standard_pivots = true;
   cfg.include_m_pivots = true;
   cfg.include_zones = true;
}

string WYN_KindName(const ENUM_WYN_KIND kind)
{
   if(kind == WYN_KIND_S3) return "S3";
   if(kind == WYN_KIND_M0) return "M0";
   if(kind == WYN_KIND_S2) return "S2";
   if(kind == WYN_KIND_M1) return "M1";
   if(kind == WYN_KIND_S1) return "S1";
   if(kind == WYN_KIND_M2) return "M2";
   if(kind == WYN_KIND_PP) return "PP";
   if(kind == WYN_KIND_M3) return "M3";
   if(kind == WYN_KIND_R1) return "R1";
   if(kind == WYN_KIND_M4) return "M4";
   if(kind == WYN_KIND_R2) return "R2";
   if(kind == WYN_KIND_M5) return "M5";
   if(kind == WYN_KIND_R3) return "R3";
   if(kind == WYN_KIND_BUY_ZONE) return "BUY_ZONE";
   if(kind == WYN_KIND_SELL_ZONE) return "SELL_ZONE";
   return "NONE";
}

string WYN_RoleName(const ENUM_WYN_ROLE role)
{
   if(role == WYN_ROLE_LOWER_EXTREME) return "LOWER_EXTREME";
   if(role == WYN_ROLE_LOWER_MAJOR) return "LOWER_MAJOR";
   if(role == WYN_ROLE_LOWER_INTERMEDIATE) return "LOWER_INTERMEDIATE";
   if(role == WYN_ROLE_CENTER) return "CENTER";
   if(role == WYN_ROLE_UPPER_INTERMEDIATE) return "UPPER_INTERMEDIATE";
   if(role == WYN_ROLE_UPPER_MAJOR) return "UPPER_MAJOR";
   if(role == WYN_ROLE_UPPER_EXTREME) return "UPPER_EXTREME";
   if(role == WYN_ROLE_LOWER_ZONE) return "LOWER_ZONE";
   if(role == WYN_ROLE_UPPER_ZONE) return "UPPER_ZONE";
   return "UNKNOWN";
}

class CWaynePivotProducer
{
private:
   string m_symbol;
   ENUM_TIMEFRAMES m_tf;
   int m_period_sec;
   WYN_Config m_cfg;

   bool m_initialized;
   bool m_ready;

   datetime m_last_tf_bar;
   datetime m_last_rebuild_time;

   MqlRates m_rates[];
   WYN_PivotBlock m_blocks[];

   int ClampPeriods(const int value) const
   {
      if(value < 1) return 1;
      if(value > WYN_MAX_PERIODS) return WYN_MAX_PERIODS;
      return value;
   }

   ulong BuildLocalId(const datetime effective_start,
                      const ENUM_WYN_KIND kind) const
   {
      // Intentionally plain decimal arithmetic for MQL5 compatibility.
      // local_id is namespaced by producer instance in the future master.
      ulong t = 0;
      if(effective_start > 0)
         t = (ulong)effective_start;

      ulong base = t % 4000000000;
      return base * 1000 + (ulong)((int)kind);
   }

   double SafeRange(const WYN_PivotBlock &b) const
   {
      double point = SymbolInfoDouble(m_symbol, SYMBOL_POINT);
      if(point <= 0.0)
         point = 0.00000001;

      return MathMax(b.source_range, point);
   }

   double PositionInSourceRange(const WYN_PivotBlock &b,
                                const double price) const
   {
      double range = SafeRange(b);
      return (price - b.source_low) / range;
   }

   bool CalculateBlock(const MqlRates &source,
                       const datetime effective_start,
                       const datetime effective_end,
                       const int period_age,
                       WYN_PivotBlock &out)
   {
      ZeroMemory(out);

      if(source.time <= 0)
         return false;

      if(source.high < source.low)
         return false;

      if(source.high == 0.0 &&
         source.low == 0.0 &&
         source.open == 0.0 &&
         source.close == 0.0)
         return false;

      out.valid = true;
      out.period_age = period_age;
      out.timeframe = m_tf;

      out.source_start = source.time;
      out.source_end = effective_start;

      out.effective_start = effective_start;
      out.effective_end = effective_end;

      out.source_open = source.open;
      out.source_high = source.high;
      out.source_low = source.low;
      out.source_close = source.close;
      out.source_range = source.high - source.low;

      // Classic Wayne/Wyatt pivot block from the completed source period.
      out.PP = (out.source_high + out.source_low + out.source_close) / 3.0;

      out.R1 = 2.0 * out.PP - out.source_low;
      out.R2 = out.PP + out.source_range;
      out.R3 = (2.0 * out.PP) +
               (out.source_high - (2.0 * out.source_low));

      out.S1 = 2.0 * out.PP - out.source_high;
      out.S2 = out.PP - out.source_range;
      out.S3 = (2.0 * out.PP) -
               ((2.0 * out.source_high) - out.source_low);

      // M pivots are the exact midpoint family from the source indicator.
      out.M0 = 0.5 * (out.S2 + out.S3);
      out.M1 = 0.5 * (out.S1 + out.S2);
      out.M2 = 0.5 * (out.PP + out.S1);

      out.M3 = 0.5 * (out.PP + out.R1);
      out.M4 = 0.5 * (out.R1 + out.R2);
      out.M5 = 0.5 * (out.R2 + out.R3);

      return true;
   }

   datetime ResolveEffectiveEnd(const int age,
                                const int copied) const
   {
      if(age > 0 && age - 1 < copied)
         return m_rates[age - 1].time;

      if(age == 0 && ArraySize(m_rates) > 0)
      {
         datetime start = m_rates[0].time;
         if(m_period_sec > 0)
            return start + m_period_sec;
      }

      return 0;
   }

   bool Rebuild()
   {
      int keep = ClampPeriods(m_cfg.periods_to_keep);
      int requested = keep + 1;

      ArrayResize(m_rates, 0);
      ArraySetAsSeries(m_rates, true);

      int copied = CopyRates(m_symbol, m_tf, 0, requested, m_rates);
      if(copied < 2)
      {
         m_ready = false;
         ArrayResize(m_blocks, 0);
         return false;
      }

      int available_blocks = MathMin(keep, copied - 1);
      ArrayResize(m_blocks, available_blocks);

      int built = 0;

      for(int age = 0; age < available_blocks; age++)
      {
         WYN_PivotBlock block;

         datetime effective_start = m_rates[age].time;
         datetime effective_end = ResolveEffectiveEnd(age, copied);

         if(CalculateBlock(m_rates[age + 1],
                           effective_start,
                           effective_end,
                           age,
                           block))
         {
            m_blocks[built] = block;
            built++;
         }
      }

      if(built != ArraySize(m_blocks))
         ArrayResize(m_blocks, built);

      m_last_tf_bar = m_rates[0].time;
      m_last_rebuild_time = TimeCurrent();
      m_ready = (built > 0);

      return m_ready;
   }

   void PushPointLevel(WYN_Level &out[],
                       int &index,
                       const WYN_PivotBlock &b,
                       const ENUM_WYN_FAMILY family,
                       const ENUM_WYN_KIND kind,
                       const ENUM_WYN_ROLE role,
                       const double value) const
   {
      WYN_Level level;
      ZeroMemory(level);

      level.valid = true;
      level.local_id = BuildLocalId(b.effective_start, kind);

      level.family = family;
      level.kind = kind;
      level.role = role;

      level.timeframe = b.timeframe;
      level.period_age = b.period_age;

      level.lower = value;
      level.price = value;
      level.center = value;
      level.upper = value;
      level.width = 0.0;

      level.source_start = b.source_start;
      level.source_end = b.source_end;
      level.created_at = b.source_end;
      level.effective_start = b.effective_start;
      level.effective_end = b.effective_end;

      // Every emitted Wayne level is computed only from a completed source
      // period. Geometry therefore stays fixed for its entire effective period.
      level.developing = false;
      level.frozen_geometry = true;

      level.source_open = b.source_open;
      level.source_high = b.source_high;
      level.source_low = b.source_low;
      level.source_close = b.source_close;
      level.source_range = b.source_range;

      level.distance_from_pp = value - b.PP;
      level.position_in_source_range = PositionInSourceRange(b, value);

      out[index] = level;
      index++;
   }

   void PushBandLevel(WYN_Level &out[],
                      int &index,
                      const WYN_PivotBlock &b,
                      const ENUM_WYN_KIND kind,
                      const ENUM_WYN_ROLE role,
                      const double a,
                      const double z) const
   {
      WYN_Level level;
      ZeroMemory(level);

      double lower = MathMin(a, z);
      double upper = MathMax(a, z);
      double center = 0.5 * (lower + upper);

      level.valid = true;
      level.local_id = BuildLocalId(b.effective_start, kind);

      level.family = WYN_FAMILY_ZONE;
      level.kind = kind;
      level.role = role;

      level.timeframe = b.timeframe;
      level.period_age = b.period_age;

      level.lower = lower;
      level.price = center;
      level.center = center;
      level.upper = upper;
      level.width = upper - lower;

      level.source_start = b.source_start;
      level.source_end = b.source_end;
      level.created_at = b.source_end;
      level.effective_start = b.effective_start;
      level.effective_end = b.effective_end;

      level.developing = false;
      level.frozen_geometry = true;

      level.source_open = b.source_open;
      level.source_high = b.source_high;
      level.source_low = b.source_low;
      level.source_close = b.source_close;
      level.source_range = b.source_range;

      level.distance_from_pp = center - b.PP;
      level.position_in_source_range = PositionInSourceRange(b, center);

      out[index] = level;
      index++;
   }

public:
   CWaynePivotProducer()
   {
      m_symbol = "";
      m_tf = PERIOD_D1;
      m_period_sec = 86400;

      WYN_DefaultConfig(m_cfg);

      m_initialized = false;
      m_ready = false;

      m_last_tf_bar = 0;
      m_last_rebuild_time = 0;

      ArrayResize(m_rates, 0);
      ArrayResize(m_blocks, 0);
   }

   bool Init(const string symbol,
             const ENUM_TIMEFRAMES timeframe,
             const WYN_Config &cfg)
   {
      m_symbol = symbol;
      m_tf = timeframe;
      m_cfg = cfg;
      m_cfg.periods_to_keep = ClampPeriods(m_cfg.periods_to_keep);

      m_period_sec = PeriodSeconds(m_tf);
      if(m_period_sec <= 0)
         m_period_sec = 86400;

      m_initialized = true;
      m_ready = false;
      m_last_tf_bar = 0;
      m_last_rebuild_time = 0;

      ArrayResize(m_rates, 0);
      ArrayResize(m_blocks, 0);

      return Rebuild();
   }

   bool Reset()
   {
      if(!m_initialized)
         return false;

      m_ready = false;
      m_last_tf_bar = 0;
      m_last_rebuild_time = 0;
      ArrayResize(m_rates, 0);
      ArrayResize(m_blocks, 0);

      return Rebuild();
   }

   bool Update()
   {
      if(!m_initialized)
         return false;

      datetime current_tf_bar = iTime(m_symbol, m_tf, 0);
      if(current_tf_bar <= 0)
         return false;

      // Because incomplete-source projections are intentionally excluded,
      // the entire Wayne structure is static intraperiod. Rebuild only when
      // a new source timeframe period opens.
      if(m_ready && current_tf_bar == m_last_tf_bar)
         return true;

      return Rebuild();
   }

   bool IsReady() const
   {
      return m_ready;
   }

   int BlockCount() const
   {
      return ArraySize(m_blocks);
   }

   bool GetBlock(const int period_age,
                 WYN_PivotBlock &out) const
   {
      ZeroMemory(out);

      if(period_age < 0 || period_age >= ArraySize(m_blocks))
         return false;

      out = m_blocks[period_age];
      return out.valid;
   }

   int ExportLevels(WYN_Level &out[]) const
   {
      ArrayResize(out, 0);

      if(!m_ready)
         return 0;

      int per_block = 0;

      if(m_cfg.include_standard_pivots)
         per_block += 7;

      if(m_cfg.include_m_pivots)
         per_block += 6;

      if(m_cfg.include_zones)
         per_block += 2;

      if(per_block <= 0)
         return 0;

      int blocks = ArraySize(m_blocks);
      ArrayResize(out, blocks * per_block);

      int index = 0;

      for(int i = 0; i < blocks; i++)
      {
         WYN_PivotBlock b = m_blocks[i];
         if(!b.valid)
            continue;

         // Export low-to-high. This ordering is deterministic and directly
         // useful to the master structure aggregator.
         if(m_cfg.include_standard_pivots)
            PushPointLevel(out, index, b,
                           WYN_FAMILY_PIVOT,
                           WYN_KIND_S3,
                           WYN_ROLE_LOWER_EXTREME,
                           b.S3);

         if(m_cfg.include_m_pivots)
            PushPointLevel(out, index, b,
                           WYN_FAMILY_MID_PIVOT,
                           WYN_KIND_M0,
                           WYN_ROLE_LOWER_INTERMEDIATE,
                           b.M0);

         if(m_cfg.include_standard_pivots)
            PushPointLevel(out, index, b,
                           WYN_FAMILY_PIVOT,
                           WYN_KIND_S2,
                           WYN_ROLE_LOWER_MAJOR,
                           b.S2);

         if(m_cfg.include_m_pivots)
            PushPointLevel(out, index, b,
                           WYN_FAMILY_MID_PIVOT,
                           WYN_KIND_M1,
                           WYN_ROLE_LOWER_INTERMEDIATE,
                           b.M1);

         if(m_cfg.include_standard_pivots)
            PushPointLevel(out, index, b,
                           WYN_FAMILY_PIVOT,
                           WYN_KIND_S1,
                           WYN_ROLE_LOWER_MAJOR,
                           b.S1);

         if(m_cfg.include_m_pivots)
            PushPointLevel(out, index, b,
                           WYN_FAMILY_MID_PIVOT,
                           WYN_KIND_M2,
                           WYN_ROLE_LOWER_INTERMEDIATE,
                           b.M2);

         if(m_cfg.include_standard_pivots)
            PushPointLevel(out, index, b,
                           WYN_FAMILY_PIVOT,
                           WYN_KIND_PP,
                           WYN_ROLE_CENTER,
                           b.PP);

         if(m_cfg.include_m_pivots)
            PushPointLevel(out, index, b,
                           WYN_FAMILY_MID_PIVOT,
                           WYN_KIND_M3,
                           WYN_ROLE_UPPER_INTERMEDIATE,
                           b.M3);

         if(m_cfg.include_standard_pivots)
            PushPointLevel(out, index, b,
                           WYN_FAMILY_PIVOT,
                           WYN_KIND_R1,
                           WYN_ROLE_UPPER_MAJOR,
                           b.R1);

         if(m_cfg.include_m_pivots)
            PushPointLevel(out, index, b,
                           WYN_FAMILY_MID_PIVOT,
                           WYN_KIND_M4,
                           WYN_ROLE_UPPER_INTERMEDIATE,
                           b.M4);

         if(m_cfg.include_standard_pivots)
            PushPointLevel(out, index, b,
                           WYN_FAMILY_PIVOT,
                           WYN_KIND_R2,
                           WYN_ROLE_UPPER_MAJOR,
                           b.R2);

         if(m_cfg.include_m_pivots)
            PushPointLevel(out, index, b,
                           WYN_FAMILY_MID_PIVOT,
                           WYN_KIND_M5,
                           WYN_ROLE_UPPER_INTERMEDIATE,
                           b.M5);

         if(m_cfg.include_standard_pivots)
            PushPointLevel(out, index, b,
                           WYN_FAMILY_PIVOT,
                           WYN_KIND_R3,
                           WYN_ROLE_UPPER_EXTREME,
                           b.R3);

         // Preserve the original Wayne/Wyatt structural zones as data:
         // lower zone = S2..M1, upper zone = M4..R2.
         if(m_cfg.include_zones)
         {
            PushBandLevel(out, index, b,
                          WYN_KIND_BUY_ZONE,
                          WYN_ROLE_LOWER_ZONE,
                          b.S2, b.M1);

            PushBandLevel(out, index, b,
                          WYN_KIND_SELL_ZONE,
                          WYN_ROLE_UPPER_ZONE,
                          b.M4, b.R2);
         }
      }

      if(index != ArraySize(out))
         ArrayResize(out, index);

      return index;
   }

   bool GetReading(WYN_Reading &out) const
   {
      ZeroMemory(out);

      out.ready = m_ready;
      out.symbol = m_symbol;
      out.timeframe = m_tf;
      out.blocks = ArraySize(m_blocks);
      out.last_rebuild_time = m_last_rebuild_time;

      if(ArraySize(m_rates) > 0)
         out.current_period_start = m_rates[0].time;

      if(!m_ready || ArraySize(m_blocks) <= 0)
         return false;

      WYN_PivotBlock b = m_blocks[0];

      out.current_PP = b.PP;
      out.current_R1 = b.R1;
      out.current_R2 = b.R2;
      out.current_R3 = b.R3;
      out.current_S1 = b.S1;
      out.current_S2 = b.S2;
      out.current_S3 = b.S3;

      out.current_M0 = b.M0;
      out.current_M1 = b.M1;
      out.current_M2 = b.M2;
      out.current_M3 = b.M3;
      out.current_M4 = b.M4;
      out.current_M5 = b.M5;

      return true;
   }
};

#endif // __KITT_02_WAYNE_MQH__
//+------------------------------------------------------------------+
