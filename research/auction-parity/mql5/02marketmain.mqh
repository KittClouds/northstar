//+------------------------------------------------------------------+
//| Kitt_VolKittMarketProfile_Producer.mqh                           |
//| Headless structural producer extracted from VolKitt I2 v2.35    |
//|                                                                  |
//| Purpose                                                          |
//|   - NO chart objects                                             |
//|   - NO indicator buffers                                         |
//|   - NO input declarations                                        |
//|   - NO OnInit/OnCalculate ownership                              |
//|   - Emits typed raw structural levels for a master controller    |
//|                                                                  |
//| Preserved research semantics                                     |
//|   - volume-weighted canonical K-means lattice                    |
//|   - local cluster POCs C1..Cn                                    |
//|   - global density COG + field width                             |
//|   - cluster lifecycle / motion / mass / width state              |
//|   - lower/upper extreme sentinel memory                          |
//|   - session TPO Market Profile POC / VAH / VAL                   |
//|   - optional single-print bands                                  |
//|                                                                  |
//| Source lineage                                                   |
//|   VolKitt Iteration 2 v2.35, CC BY-NC-SA 4.0                    |
//|   Embedded MarketProfile logic was supplied in the source file.  |
//|                                                                  |
//| This include is designed for research/controller composition.    |
//+------------------------------------------------------------------+
#ifndef __KITT_02_VOLKITT_MARKETPROFILE_PRODUCER_MQH__
#define __KITT_02_VOLKITT_MARKETPROFILE_PRODUCER_MQH__

#define KVP_MAX_CLUSTERS       10
#define KVP_MAX_PROFILES        8
#define KVP_MAX_SINGLE_PRINTS  32
#define KVP_INF          1.0e100

// -----------------------------------------------------------------------------
// Public contracts
// -----------------------------------------------------------------------------

enum KVP_VOLUME_TYPE
{
   KVP_VOL_TICK = 0,
   KVP_VOL_REAL = 1
};

enum KVP_LEVEL_STATE
{
   KVP_LEVEL_FRESH = 0,
   KVP_LEVEL_TESTED,
   KVP_LEVEL_ACCEPTED,
   KVP_LEVEL_REJECTED,
   KVP_LEVEL_BROKEN,
   KVP_LEVEL_RECLAIMED
};

enum KVP_DIRECTION_STATE
{
   KVP_DIR_FALL   = -1,
   KVP_DIR_STABLE =  0,
   KVP_DIR_RISE   =  1
};

enum KVP_WIDTH_STATE
{
   KVP_WIDTH_COMPRESS = -1,
   KVP_WIDTH_STABLE   =  0,
   KVP_WIDTH_EXPAND   =  1
};

enum KVP_MASS_STATE
{
   KVP_MASS_HOLLOW = -1,
   KVP_MASS_STABLE =  0,
   KVP_MASS_BUILD  =  1
};

enum KVP_SENTINEL_TRIGGER_MODE
{
   KVP_SENTINEL_TRIGGER_ATR       = 0,
   KVP_SENTINEL_TRIGGER_OUTER_GAP = 1,
   KVP_SENTINEL_TRIGGER_HYBRID    = 2
};

enum KVP_SENTINEL_RESET_MODE
{
   KVP_SENTINEL_RESET_BROKER_DAY = 0,
   KVP_SENTINEL_RESET_REENTRY    = 1,
   KVP_SENTINEL_RESET_NEVER      = 2
};

enum KVP_PROFILE_SESSION
{
   KVP_PROFILE_DAILY = 0,
   KVP_PROFILE_WEEKLY,
   KVP_PROFILE_MONTHLY,
   KVP_PROFILE_INTRADAY
};

enum KVP_LEVEL_FAMILY
{
   KVP_FAMILY_ADAPTIVE_VALUE = 1,
   KVP_FAMILY_GLOBAL_VALUE   = 2,
   KVP_FAMILY_SENTINEL       = 3,
   KVP_FAMILY_TPO_PROFILE    = 4,
   KVP_FAMILY_SINGLE_PRINT   = 5
};

enum KVP_LEVEL_ROLE
{
   KVP_ROLE_NONE = 0,
   KVP_ROLE_OUTER_LOW,
   KVP_ROLE_INNER_LOW,
   KVP_ROLE_FAIR_VALUE,
   KVP_ROLE_INNER_HIGH,
   KVP_ROLE_OUTER_HIGH,
   KVP_ROLE_CENTER,
   KVP_ROLE_LOWER_BOUNDARY,
   KVP_ROLE_UPPER_BOUNDARY,
   KVP_ROLE_LOWER_MEMORY,
   KVP_ROLE_UPPER_MEMORY
};

enum KVP_LEVEL_KIND
{
   KVP_KIND_NONE = 0,

   KVP_KIND_C1 = 101,
   KVP_KIND_C2,
   KVP_KIND_C3,
   KVP_KIND_C4,
   KVP_KIND_C5,
   KVP_KIND_C6,
   KVP_KIND_C7,
   KVP_KIND_C8,
   KVP_KIND_C9,
   KVP_KIND_C10,

   KVP_KIND_COG = 150,
   KVP_KIND_LOWER_SENTINEL,
   KVP_KIND_UPPER_SENTINEL,

   KVP_KIND_PROFILE_POC = 200,
   KVP_KIND_PROFILE_VAH,
   KVP_KIND_PROFILE_VAL,

   KVP_KIND_SINGLE_PRINT_BAND = 250
};

struct KVP_Config
{
   // VolKitt core.
   int                       lookback;
   int                       clusters;
   int                       iterations;
   int                       rows_per_cluster;
   double                    identity_reset_bins;
   KVP_VOLUME_TYPE           volume_type;

   int                       atr_period;
   int                       cog_bins;
   int                       velocity_bars;
   double                    motion_threshold_bins;
   double                    mass_motion_pct;
   double                    width_motion_pct;

   // Runtime. Heavy geometry can update intrabar in live mode.
   bool                      heavy_on_new_bar_only;
   uint                      live_heavy_refresh_ms;

   // Sentinel memory.
   bool                      enable_sentinels;
   KVP_SENTINEL_TRIGGER_MODE sentinel_trigger_mode;
   KVP_SENTINEL_RESET_MODE   sentinel_reset_mode;
   double                    sentinel_arm_distance_atr;
   double                    sentinel_arm_gap_fraction;
   double                    sentinel_min_stretch_atr;
   double                    sentinel_reentry_buffer_atr;
   int                       sentinel_reentry_bars;

   // TPO Market Profile.
   bool                      enable_profile;
   KVP_PROFILE_SESSION       profile_session;
   int                       profiles_to_keep;
   int                       value_area_percent;
   int                       point_multiplier;      // 0 = adaptive, matching source behavior.
   int                       max_profile_bins;      // hard performance ceiling.

   // Intraday profile only. Minutes from broker-day midnight.
   int                       intraday_start_minute;
   int                       intraday_end_minute;

   bool                      enable_single_prints;
   double                    prominent_poc_percent; // >100 effectively disables.
};

struct KVP_LevelState
{
   double               price;
   datetime             birth_time;
   int                  age_bars;
   int                  touch_count;
   int                  rejection_count;
   double               max_break_dist;
   int                  reclaim_success;
   int                  cumulative_acceptance;
   int                  consecutive_acceptance;
   KVP_LEVEL_STATE      status;
   datetime             last_time_updated;
   bool                 touched_this_bar;
   int                  broken_side;
   double               snap_centroid;
   double               snap_mass;
   double               snap_range;
   datetime             snap_time;
   KVP_DIRECTION_STATE  direction_state;
   KVP_WIDTH_STATE      width_state;
   KVP_MASS_STATE       mass_state;
   ulong                generation;
};

struct KVP_Cluster
{
   bool     valid;
   int      slot;
   double   centroid;
   double   price;
   double   zone_low;
   double   zone_high;
   double   cluster_low;
   double   cluster_high;
   double   total_mass;
   double   mass_share;
   double   poc_mass;
   double   bin_size;
};

struct KVP_Sentinel
{
   bool       latched;
   bool       upper_side;
   double     price;
   datetime   latched_at;
   int        source_slot;
   int        bars_back_inside;
   double     max_excursion_atr;
   ulong      generation;
};

struct KVP_CoreReading
{
   bool      valid;
   int       cluster_count;
   int       nearest_slot;
   double    current_price;
   double    nearest_price;
   double    lower_price;
   double    upper_price;
   double    center_of_gravity;
   double    field_width;
   double    atr;
   double    nearest_distance_atr;
   double    center_distance_atr;
   double    field_width_atr;
   double    center_velocity_atr;
   double    lattice_width;
   double    lattice_width_atr;
   datetime  market_time;
   datetime  calc_bar_time;
   ulong     refresh_count;
};

// Lightweight producer telemetry. This is deliberately separate from the
// structural reading so instrumentation never changes exported market data.
struct KVP_PerformanceReading
{
   bool      heavy_core_due;
   bool      profile_rebuild_due;
   ulong     core_microseconds;
   ulong     profile_microseconds;
   ulong     update_microseconds;
};

struct KVP_SinglePrintBand
{
   bool    valid;
   double  low;
   double  high;
};

struct KVP_Profile
{
   bool                 valid;
   int                  ordinal;        // 0 current, 1 previous, ...
   KVP_PROFILE_SESSION  session_kind;
   datetime             session_start;
   datetime             session_end;
   bool                 developing;

   double               session_high;
   double               session_low;
   double               tpo_step;
   int                  bar_count;
   int                  bin_count;
   int                  total_tpo;

   double               poc;
   double               vah;
   double               val;
   int                  poc_tpo;
   double               poc_bar_share_pct;
   bool                 prominent_poc;

   int                  single_print_count;
   KVP_SinglePrintBand  single_prints[KVP_MAX_SINGLE_PRINTS];
};

// One raw producer output. The future master controller can normalize/cluster
// these across independent MQH systems without reading any chart objects.
struct KVP_Level
{
   bool               valid;
   ulong              local_id;
   KVP_LEVEL_FAMILY   family;
   KVP_LEVEL_KIND     kind;
   KVP_LEVEL_ROLE     role;

   double             lower;
   double             price;
   double             upper;

   ENUM_TIMEFRAMES    timeframe;
   datetime           created_at;
   datetime           updated_at;
   datetime           session_start;
   datetime           session_end;

   int                source_slot;
   int                ordinal;
   bool               developing;
   bool               frozen;

   KVP_LEVEL_STATE    state;
   KVP_DIRECTION_STATE direction_state;
   KVP_WIDTH_STATE    width_state;
   KVP_MASS_STATE     mass_state;

   int                touches;
   int                rejections;
   int                reclaims;
   int                acceptance_bars;

   double             mass;
   double             mass_share;
   double             poc_mass;
   double             width_atr;
   double             distance_atr;
   double             max_excursion_atr;
};

// -----------------------------------------------------------------------------
// Public helpers
// -----------------------------------------------------------------------------

void KVP_DefaultConfig(KVP_Config &cfg)
{
   ZeroMemory(cfg);

   cfg.lookback                  = 200;
   cfg.clusters                  = 5;
   cfg.iterations                = 50;
   cfg.rows_per_cluster          = 20;
   cfg.identity_reset_bins       = 2.50;
   cfg.volume_type               = KVP_VOL_TICK;

   cfg.atr_period                = 100;
   cfg.cog_bins                  = 180;
   cfg.velocity_bars             = 5;
   cfg.motion_threshold_bins     = 0.15;
   cfg.mass_motion_pct           = 0.05;
   cfg.width_motion_pct          = 0.05;

   cfg.heavy_on_new_bar_only     = false;
   cfg.live_heavy_refresh_ms     = 750;

   cfg.enable_sentinels          = true;
   cfg.sentinel_trigger_mode     = KVP_SENTINEL_TRIGGER_HYBRID;
   cfg.sentinel_reset_mode       = KVP_SENTINEL_RESET_BROKER_DAY;
   cfg.sentinel_arm_distance_atr = 0.20;
   cfg.sentinel_arm_gap_fraction = 0.25;
   cfg.sentinel_min_stretch_atr  = 1.00;
   cfg.sentinel_reentry_buffer_atr = 0.25;
   cfg.sentinel_reentry_bars     = 3;

   cfg.enable_profile            = true;
   cfg.profile_session           = KVP_PROFILE_DAILY;
   cfg.profiles_to_keep          = 2;
   cfg.value_area_percent        = 70;
   cfg.point_multiplier          = 0;
   cfg.max_profile_bins          = 12000;
   cfg.intraday_start_minute     = 0;
   cfg.intraday_end_minute       = 24 * 60;
   cfg.enable_single_prints      = false;
   cfg.prominent_poc_percent     = 101.0;
}

string KVP_KindLabel(const KVP_LEVEL_KIND kind)
{
   switch(kind)
   {
      case KVP_KIND_C1: return "C1";
      case KVP_KIND_C2: return "C2";
      case KVP_KIND_C3: return "C3";
      case KVP_KIND_C4: return "C4";
      case KVP_KIND_C5: return "C5";
      case KVP_KIND_C6: return "C6";
      case KVP_KIND_C7: return "C7";
      case KVP_KIND_C8: return "C8";
      case KVP_KIND_C9: return "C9";
      case KVP_KIND_C10: return "C10";
      case KVP_KIND_COG: return "COG";
      case KVP_KIND_LOWER_SENTINEL: return "LOW_SENT";
      case KVP_KIND_UPPER_SENTINEL: return "HIGH_SENT";
      case KVP_KIND_PROFILE_POC: return "POC";
      case KVP_KIND_PROFILE_VAH: return "VAH";
      case KVP_KIND_PROFILE_VAL: return "VAL";
      case KVP_KIND_SINGLE_PRINT_BAND: return "SP";
      default: return "NONE";
   }
}

// -----------------------------------------------------------------------------
// Producer
// -----------------------------------------------------------------------------

class CKittVolKittMarketProfileProducer
{
private:
   string            m_symbol;
   ENUM_TIMEFRAMES   m_calc_tf;
   ENUM_TIMEFRAMES   m_profile_tf;
   KVP_Config        m_cfg;
   bool              m_initialized;
   bool              m_is_tester;

   int               m_digits;
   double            m_point;
   double            m_tick_size;
   int               m_calc_period_sec;

   double            m_mem_centroids[];
   KVP_Cluster       m_clusters[KVP_MAX_CLUSTERS];
   KVP_LevelState    m_level_states[KVP_MAX_CLUSTERS];
   int               m_cluster_count;

   KVP_Sentinel      m_lower_sentinel;
   KVP_Sentinel      m_upper_sentinel;
   datetime          m_sentinel_reset_key;

   KVP_CoreReading   m_reading;
   KVP_PerformanceReading m_performance;
   double            m_cached_center;
   double            m_cached_field_width;
   double            m_cached_atr;
   bool              m_core_valid;
   datetime          m_last_calc_bar_time;
   datetime          m_last_heavy_market_time;
   uint              m_last_heavy_ms;

   double            m_cog_snapshot;
   datetime          m_cog_snapshot_time;
   double            m_cog_velocity_atr;
   ulong             m_refresh_count;

   KVP_Profile       m_profiles[KVP_MAX_PROFILES];
   int               m_profile_count;
   datetime          m_last_profile_calc_bar_time;

   // ----- basic helpers -------------------------------------------------------

   int ClampInt(const int value, const int low, const int high)
   {
      return MathMax(low, MathMin(value, high));
   }

   double SafePoint(void)
   {
      return MathMax(m_point, 1.0e-10);
   }

   double SafeTick(void)
   {
      return MathMax(m_tick_size, SafePoint());
   }

   double ClampDouble(const double value, const double low, const double high)
   {
      return MathMax(low, MathMin(value, high));
   }

   double GetBarVolume(const MqlRates &bar)
   {
      if(m_cfg.volume_type == KVP_VOL_REAL && bar.real_volume > 0)
         return (double)bar.real_volume;
      return (double)bar.tick_volume;
   }

   ulong MakeId(const int family, const int kind, const ulong generation, const datetime anchor)
   {
      // Local producer identity, deliberately not a global controller ID.
      // MQL5 has no C/C++ ULL literal suffix. Keep the masks as decimal
      // values so MetaEditor parses them portably.
      ulong id = ((ulong)(family & 255) << 56);
      id |= ((ulong)(kind & 65535) << 40);
      id ^= ((generation % 65536) << 24);
      id ^= ((ulong)anchor % 16777216);
      return id;
   }

   KVP_LEVEL_KIND ClusterKind(const int slot)
   {
      int raw = (int)KVP_KIND_C1 + slot;
      if(raw < (int)KVP_KIND_C1 || raw > (int)KVP_KIND_C10)
         return KVP_KIND_NONE;
      return (KVP_LEVEL_KIND)raw;
   }

   KVP_LEVEL_ROLE ClusterRole(const int slot, const int count)
   {
      if(count == 5)
      {
         if(slot == 0) return KVP_ROLE_OUTER_LOW;
         if(slot == 1) return KVP_ROLE_INNER_LOW;
         if(slot == 2) return KVP_ROLE_FAIR_VALUE;
         if(slot == 3) return KVP_ROLE_INNER_HIGH;
         if(slot == 4) return KVP_ROLE_OUTER_HIGH;
      }

      if(slot == 0) return KVP_ROLE_OUTER_LOW;
      if(slot == count - 1) return KVP_ROLE_OUTER_HIGH;
      if((count % 2) == 1 && slot == count / 2) return KVP_ROLE_FAIR_VALUE;
      if(slot < count / 2) return KVP_ROLE_INNER_LOW;
      return KVP_ROLE_INNER_HIGH;
   }

   void SortCentroids(double &centroids[], const int count)
   {
      for(int i = 1; i < count; i++)
      {
         double key = centroids[i];
         int j = i - 1;
         while(j >= 0 && centroids[j] > key)
         {
            centroids[j + 1] = centroids[j];
            j--;
         }
         centroids[j + 1] = key;
      }
   }

   // ----- VolKitt -------------------------------------------------------------

   void KMeansCanonical(const int n,
                        const int k,
                        const int iterations,
                        const double &prices[],
                        const double &volumes[],
                        int &assignments[],
                        double &centroids[])
   {
      ArrayResize(assignments, n);
      ArrayInitialize(assignments, -1);
      ArrayResize(centroids, k);

      double min_price = KVP_INF;
      double max_price = -KVP_INF;
      for(int i = 0; i < n; i++)
      {
         min_price = MathMin(min_price, prices[i]);
         max_price = MathMax(max_price, prices[i]);
      }

      double span = MathMax(max_price - min_price, SafePoint());
      bool memory_valid = (ArraySize(m_mem_centroids) == k);
      if(memory_valid)
      {
         for(int c = 0; c < k; c++)
         {
            if(m_mem_centroids[c] < min_price - span ||
               m_mem_centroids[c] > max_price + span)
            {
               memory_valid = false;
               break;
            }
         }
      }

      if(memory_valid)
      {
         ArrayCopy(centroids, m_mem_centroids);
      }
      else
      {
         double step = span / (k + 1);
         for(int c = 0; c < k; c++)
            centroids[c] = min_price + (c + 1) * step;
      }
      SortCentroids(centroids, k);

      double sum_pv[];
      double sum_v[];
      ArrayResize(sum_pv, k);
      ArrayResize(sum_v, k);

      for(int iter = 0; iter < iterations; iter++)
      {
         bool changed = false;

         for(int i = 0; i < n; i++)
         {
            int best = 0;
            double best_distance = KVP_INF;
            for(int c = 0; c < k; c++)
            {
               double distance = MathAbs(prices[i] - centroids[c]);
               if(distance < best_distance)
               {
                  best_distance = distance;
                  best = c;
               }
            }

            if(assignments[i] != best)
            {
               assignments[i] = best;
               changed = true;
            }
         }

         ArrayInitialize(sum_pv, 0.0);
         ArrayInitialize(sum_v, 0.0);
         for(int i = 0; i < n; i++)
         {
            int c = assignments[i];
            double weight = MathMax(volumes[i], 0.0);
            sum_pv[c] += prices[i] * weight;
            sum_v[c] += weight;
         }

         for(int c = 0; c < k; c++)
         {
            if(sum_v[c] > 0.0)
               centroids[c] = sum_pv[c] / sum_v[c];
         }
         SortCentroids(centroids, k);

         if(!changed && iter > 0)
            break;
      }

      // Canonical final assignment after low-to-high sorting.
      for(int i = 0; i < n; i++)
      {
         int best = 0;
         double best_distance = KVP_INF;
         for(int c = 0; c < k; c++)
         {
            double distance = MathAbs(prices[i] - centroids[c]);
            if(distance < best_distance)
            {
               best_distance = distance;
               best = c;
            }
         }
         assignments[i] = best;
      }

      ArrayResize(m_mem_centroids, k);
      ArrayCopy(m_mem_centroids, centroids);
   }

   void ResetLevelState(KVP_LevelState &state,
                        const double price,
                        const double mass,
                        const double range,
                        const datetime current_time)
   {
      ulong next_generation = state.generation + 1;
      ZeroMemory(state);
      state.generation = MathMax(next_generation, (ulong)1);
      state.price = price;
      state.birth_time = current_time;
      state.status = KVP_LEVEL_FRESH;
      state.last_time_updated = current_time;
      state.snap_time = current_time;
      state.snap_centroid = price;
      state.snap_mass = mass;
      state.snap_range = range;
      state.direction_state = KVP_DIR_STABLE;
      state.width_state = KVP_WIDTH_STABLE;
      state.mass_state = KVP_MASS_STABLE;
   }

   void UpdateLevelState(KVP_LevelState &state,
                         const KVP_Cluster &obs,
                         const MqlRates &current_bar,
                         const MqlRates &closed_bar)
   {
      double reset_distance = MathMax(obs.bin_size * m_cfg.identity_reset_bins,
                                      SafePoint() * 5.0);

      if(state.price == 0.0 || MathAbs(obs.price - state.price) > reset_distance)
      {
         ResetLevelState(state, obs.price, obs.total_mass,
                         obs.cluster_high - obs.cluster_low,
                         current_bar.time);
         return;
      }

      state.price = obs.price;
      double threshold = MathMax(obs.bin_size * 0.5, SafePoint() * 5.0);

      // Closed-bar lifecycle advances once per calculation bar.
      if(state.last_time_updated != current_bar.time && closed_bar.time != 0)
      {
         state.age_bars++;
         double distance_close = closed_bar.close - state.price;
         double absolute_distance = MathAbs(distance_close);
         bool inside_acceptance = (absolute_distance <= obs.bin_size * 1.5);

         if(inside_acceptance)
         {
            state.cumulative_acceptance++;
            state.consecutive_acceptance++;
         }
         else
         {
            state.consecutive_acceptance = 0;
         }

         KVP_LEVEL_STATE next_state = state.status;
         bool crossed_up = (closed_bar.open < state.price &&
                            closed_bar.close > state.price + threshold);
         bool crossed_down = (closed_bar.open > state.price &&
                              closed_bar.close < state.price - threshold);
         bool touched = (closed_bar.high >= state.price && closed_bar.low <= state.price);
         bool rejected = touched &&
            ((closed_bar.open < state.price && closed_bar.close < state.price &&
              state.price - closed_bar.close > threshold) ||
             (closed_bar.open > state.price && closed_bar.close > state.price &&
              closed_bar.close - state.price > threshold));

         if(state.broken_side != 0)
         {
            state.max_break_dist = MathMax(state.max_break_dist, absolute_distance);
            bool reclaimed = ((state.broken_side == 1 && closed_bar.close < state.price) ||
                              (state.broken_side == -1 && closed_bar.close > state.price));
            if(reclaimed)
            {
               state.reclaim_success++;
               state.broken_side = 0;
               next_state = KVP_LEVEL_RECLAIMED;
            }
         }

         if(crossed_up || crossed_down)
         {
            state.broken_side = crossed_up ? 1 : -1;
            next_state = KVP_LEVEL_BROKEN;
         }
         else if(rejected && next_state != KVP_LEVEL_RECLAIMED)
         {
            state.rejection_count++;
            next_state = KVP_LEVEL_REJECTED;
         }
         else if(state.consecutive_acceptance >= 3 &&
                 (next_state == KVP_LEVEL_FRESH ||
                  next_state == KVP_LEVEL_TESTED ||
                  next_state == KVP_LEVEL_REJECTED))
         {
            next_state = KVP_LEVEL_ACCEPTED;
         }

         state.status = next_state;
         state.touched_this_bar = false;
         state.last_time_updated = current_bar.time;
      }

      if(!state.touched_this_bar &&
         current_bar.high >= state.price && current_bar.low <= state.price)
      {
         state.touch_count++;
         state.touched_this_bar = true;
         if(state.status == KVP_LEVEL_FRESH)
            state.status = KVP_LEVEL_TESTED;
      }

      if(current_bar.time - state.snap_time >=
         MathMax(m_cfg.velocity_bars, 1) * m_calc_period_sec)
      {
         double price_delta = obs.price - state.snap_centroid;
         double range_now = obs.cluster_high - obs.cluster_low;
         double range_delta = range_now - state.snap_range;
         double mass_delta = obs.total_mass - state.snap_mass;
         double motion_threshold = obs.bin_size * m_cfg.motion_threshold_bins;

         state.direction_state = (price_delta > motion_threshold) ? KVP_DIR_RISE :
                                 (price_delta < -motion_threshold) ? KVP_DIR_FALL :
                                 KVP_DIR_STABLE;

         double range_threshold = MathMax(MathAbs(state.snap_range) *
                                          m_cfg.width_motion_pct,
                                          obs.bin_size);
         state.width_state = (range_delta > range_threshold) ? KVP_WIDTH_EXPAND :
                             (range_delta < -range_threshold) ? KVP_WIDTH_COMPRESS :
                             KVP_WIDTH_STABLE;

         double mass_threshold = MathMax(MathAbs(state.snap_mass) *
                                         m_cfg.mass_motion_pct, 1.0);
         state.mass_state = (mass_delta > mass_threshold) ? KVP_MASS_BUILD :
                            (mass_delta < -mass_threshold) ? KVP_MASS_HOLLOW :
                            KVP_MASS_STABLE;

         state.snap_centroid = obs.price;
         state.snap_mass = obs.total_mass;
         state.snap_range = range_now;
         state.snap_time = current_bar.time;
      }
   }

   bool BuildCanonicalClusters(const MqlRates &rates[],
                               const int available_count)
   {
      int k = ClampInt(m_cfg.clusters, 1, KVP_MAX_CLUSTERS);
      int count = MathMin(m_cfg.lookback, available_count);
      m_cluster_count = k;

      for(int c = 0; c < KVP_MAX_CLUSTERS; c++)
      {
         ZeroMemory(m_clusters[c]);
         m_clusters[c].slot = c;
      }

      if(count < k || count < 5)
         return false;

      double prices[], volumes[], highs[], lows[];
      ArrayResize(prices, count);
      ArrayResize(volumes, count);
      ArrayResize(highs, count);
      ArrayResize(lows, count);

      double total_field_mass = 0.0;
      for(int i = 0; i < count; i++)
      {
         prices[i] = (rates[i].high + rates[i].low) * 0.5;
         highs[i] = rates[i].high;
         lows[i] = rates[i].low;
         volumes[i] = GetBarVolume(rates[i]);
         total_field_mass += MathMax(volumes[i], 0.0);
      }

      // No usable weighting data means no trustworthy VolKitt lattice.
      if(total_field_mass <= 0.0)
         return false;

      int assignments[];
      double centroids[];
      KMeansCanonical(count, k, MathMax(m_cfg.iterations, 1),
                      prices, volumes, assignments, centroids);

      for(int c = 0; c < k; c++)
      {
         double cluster_low = KVP_INF;
         double cluster_high = -KVP_INF;
         double cluster_mass = 0.0;
         int member_count = 0;

         for(int i = 0; i < count; i++)
         {
            if(assignments[i] != c)
               continue;
            cluster_low = MathMin(cluster_low, lows[i]);
            cluster_high = MathMax(cluster_high, highs[i]);
            cluster_mass += MathMax(volumes[i], 0.0);
            member_count++;
         }

         if(member_count <= 0 || cluster_high <= cluster_low)
            continue;

         int rows = MathMax(m_cfg.rows_per_cluster, 2);
         double bin_size = MathMax((cluster_high - cluster_low) / rows, SafePoint());
         double bin_volumes[];
         ArrayResize(bin_volumes, rows);
         ArrayInitialize(bin_volumes, 0.0);

         for(int i = 0; i < count; i++)
         {
            if(assignments[i] != c)
               continue;

            double bar_range = MathMax(highs[i] - lows[i], SafePoint());
            for(int b = 0; b < rows; b++)
            {
               double bin_low = cluster_low + b * bin_size;
               double bin_high = bin_low + bin_size;
               double overlap_low = MathMax(lows[i], bin_low);
               double overlap_high = MathMin(highs[i], bin_high);
               if(overlap_high > overlap_low)
                  bin_volumes[b] += volumes[i] *
                                    (overlap_high - overlap_low) / bar_range;
            }
         }

         int poc_bin = 0;
         double poc_mass = -1.0;
         for(int b = 0; b < rows; b++)
         {
            if(bin_volumes[b] > poc_mass)
            {
               poc_mass = bin_volumes[b];
               poc_bin = b;
            }
         }

         m_clusters[c].valid = true;
         m_clusters[c].centroid = centroids[c];
         m_clusters[c].price = cluster_low + (poc_bin + 0.5) * bin_size;
         m_clusters[c].zone_low = cluster_low + poc_bin * bin_size;
         m_clusters[c].zone_high = m_clusters[c].zone_low + bin_size;
         m_clusters[c].cluster_low = cluster_low;
         m_clusters[c].cluster_high = cluster_high;
         m_clusters[c].total_mass = cluster_mass;
         m_clusters[c].mass_share = (total_field_mass > 0.0)
            ? cluster_mass / total_field_mass : 0.0;
         m_clusters[c].poc_mass = MathMax(poc_mass, 0.0);
         m_clusters[c].bin_size = bin_size;

         MqlRates closed_bar;
         ZeroMemory(closed_bar);
         if(ArraySize(rates) > 1)
            closed_bar = rates[1];
         UpdateLevelState(m_level_states[c], m_clusters[c], rates[0], closed_bar);
      }

      return true;
   }

   void BuildGlobalDensity(const MqlRates &rates[],
                           const int count,
                           const int bin_count,
                           double &min_price,
                           double &bin_size,
                           double &density[])
   {
      min_price = KVP_INF;
      double max_price = -KVP_INF;

      for(int i = 0; i < count; i++)
      {
         min_price = MathMin(min_price, rates[i].low);
         max_price = MathMax(max_price, rates[i].high);
      }

      if(max_price <= min_price)
      {
         min_price = rates[0].low;
         max_price = rates[0].high + SafePoint();
      }

      bin_size = MathMax((max_price - min_price) / bin_count, SafePoint());
      ArrayResize(density, bin_count);
      ArrayInitialize(density, 0.0);

      for(int i = 0; i < count; i++)
      {
         double bar_low = rates[i].low;
         double bar_high = rates[i].high;
         double bar_volume = GetBarVolume(rates[i]);
         double bar_range = MathMax(bar_high - bar_low, SafePoint());

         int first_bin = ClampInt((int)MathFloor((bar_low - min_price) / bin_size),
                                  0, bin_count - 1);
         int last_bin = ClampInt((int)MathFloor((bar_high - min_price) / bin_size),
                                 0, bin_count - 1);

         for(int b = first_bin; b <= last_bin; b++)
         {
            double low = min_price + b * bin_size;
            double high = low + bin_size;
            double overlap_low = MathMax(bar_low, low);
            double overlap_high = MathMin(bar_high, high);
            if(overlap_high > overlap_low)
               density[b] += bar_volume *
                             (overlap_high - overlap_low) / bar_range;
         }
      }
   }

   void ComputeCenterAndWidth(const double &density[],
                              const int bin_count,
                              const double min_price,
                              const double bin_size,
                              double &center,
                              double &width)
   {
      double mass = 0.0;
      double weighted_price = 0.0;

      for(int b = 0; b < bin_count; b++)
      {
         double m = density[b];
         double price = min_price + (b + 0.5) * bin_size;
         mass += m;
         weighted_price += price * m;
      }

      if(mass <= 0.0)
      {
         center = 0.0;
         width = 0.0;
         return;
      }

      center = weighted_price / mass;
      double variance = 0.0;
      for(int b = 0; b < bin_count; b++)
      {
         double price = min_price + (b + 0.5) * bin_size;
         double delta = price - center;
         variance += density[b] * delta * delta;
      }
      width = MathSqrt(MathMax(variance / mass, 0.0));
   }

   double ComputeATR(const MqlRates &rates[],
                     const int available_count,
                     const int period)
   {
      int p = MathMin(MathMax(period, 2), available_count - 1);
      if(p <= 1)
         return SafePoint();

      double sum = 0.0;
      for(int i = 0; i < p; i++)
      {
         double prev_close = rates[i + 1].close;
         double tr1 = rates[i].high - rates[i].low;
         double tr2 = MathAbs(rates[i].high - prev_close);
         double tr3 = MathAbs(rates[i].low - prev_close);
         sum += MathMax(tr1, MathMax(tr2, tr3));
      }
      return MathMax(sum / p, SafePoint());
   }

   datetime BrokerDayOpenFor(const datetime reference_time)
   {
      if(reference_time <= 0)
         return 0;

      int shift = iBarShift(m_symbol, PERIOD_D1, reference_time, false);
      if(shift >= 0)
      {
         datetime day_open = iTime(m_symbol, PERIOD_D1, shift);
         if(day_open > 0 && day_open <= reference_time)
            return day_open;
      }

      MqlDateTime parts;
      TimeToStruct(reference_time, parts);
      parts.hour = 0;
      parts.min = 0;
      parts.sec = 0;
      return StructToTime(parts);
   }

   void ResetSentinel(KVP_Sentinel &sentinel, const bool upper_side)
   {
      ulong generation = sentinel.generation + 1;
      ZeroMemory(sentinel);
      sentinel.upper_side = upper_side;
      sentinel.source_slot = -1;
      sentinel.generation = MathMax(generation, (ulong)1);
   }

   void ResetSentinels(void)
   {
      ResetSentinel(m_lower_sentinel, false);
      ResetSentinel(m_upper_sentinel, true);
   }

   double SentinelCenterReference(const double fallback_center)
   {
      int count = m_cluster_count;
      if(count <= 0)
         return fallback_center;

      if((count % 2) == 1)
      {
         int middle = count / 2;
         if(m_clusters[middle].valid)
            return m_clusters[middle].price;
      }
      else
      {
         int left = count / 2 - 1;
         int right = count / 2;
         if(m_clusters[left].valid && m_clusters[right].valid)
            return (m_clusters[left].price + m_clusters[right].price) * 0.5;
      }
      return fallback_center;
   }

   double SentinelArmDistance(const double atr,
                              const double outer_price,
                              const double inner_price)
   {
      double atr_distance = MathMax(m_cfg.sentinel_arm_distance_atr, 0.0) *
                            MathMax(atr, SafePoint());
      double gap_distance = MathMax(m_cfg.sentinel_arm_gap_fraction, 0.0) *
                            MathAbs(outer_price - inner_price);

      if(m_cfg.sentinel_trigger_mode == KVP_SENTINEL_TRIGGER_ATR)
         return atr_distance;
      if(m_cfg.sentinel_trigger_mode == KVP_SENTINEL_TRIGGER_OUTER_GAP)
         return gap_distance;
      return MathMax(atr_distance, gap_distance);
   }

   void LatchSentinel(KVP_Sentinel &sentinel,
                      const KVP_Cluster &observation,
                      const datetime latch_time)
   {
      sentinel.latched = true;
      sentinel.price = observation.price;
      sentinel.latched_at = latch_time;
      sentinel.source_slot = observation.slot;
      sentinel.bars_back_inside = 0;
      sentinel.max_excursion_atr = 0.0;
   }

   void UpdateSentinelDayReset(const datetime market_time)
   {
      if(m_cfg.sentinel_reset_mode != KVP_SENTINEL_RESET_BROKER_DAY)
         return;

      datetime reset_key = BrokerDayOpenFor(market_time);
      if(m_sentinel_reset_key == 0)
      {
         m_sentinel_reset_key = reset_key;
         return;
      }

      if(reset_key > 0 && reset_key != m_sentinel_reset_key)
      {
         ResetSentinels();
         m_sentinel_reset_key = reset_key;
      }
   }

   void UpdateSentinelReentryReset(const MqlRates &closed_bar,
                                   const bool is_new_calc_bar,
                                   const double atr)
   {
      if(m_cfg.sentinel_reset_mode != KVP_SENTINEL_RESET_REENTRY ||
         !is_new_calc_bar || closed_bar.time == 0)
         return;

      double buffer = MathMax(m_cfg.sentinel_reentry_buffer_atr, 0.0) *
                      MathMax(atr, SafePoint());
      int required = MathMax(m_cfg.sentinel_reentry_bars, 1);

      if(m_upper_sentinel.latched)
      {
         if(closed_bar.close <= m_upper_sentinel.price - buffer)
            m_upper_sentinel.bars_back_inside++;
         else
            m_upper_sentinel.bars_back_inside = 0;

         if(m_upper_sentinel.bars_back_inside >= required)
            ResetSentinel(m_upper_sentinel, true);
      }

      if(m_lower_sentinel.latched)
      {
         if(closed_bar.close >= m_lower_sentinel.price + buffer)
            m_lower_sentinel.bars_back_inside++;
         else
            m_lower_sentinel.bars_back_inside = 0;

         if(m_lower_sentinel.bars_back_inside >= required)
            ResetSentinel(m_lower_sentinel, false);
      }
   }

   void UpdateSentinels(const double current_price,
                        const double fallback_center,
                        const double atr,
                        const MqlRates &current_bar,
                        const MqlRates &closed_bar,
                        const bool is_new_calc_bar,
                        const datetime market_time)
   {
      if(!m_cfg.enable_sentinels)
      {
         ResetSentinels();
         return;
      }

      int count = m_cluster_count;
      if(count < 3)
         return;

      UpdateSentinelDayReset(market_time);
      UpdateSentinelReentryReset(closed_bar, is_new_calc_bar, atr);

      int lower_outer = 0;
      int lower_inner = MathMin(1, count - 1);
      int upper_outer = count - 1;
      int upper_inner = MathMax(count - 2, 0);

      if(!m_clusters[lower_outer].valid || !m_clusters[lower_inner].valid ||
         !m_clusters[upper_outer].valid || !m_clusters[upper_inner].valid)
         return;

      double safe_atr = MathMax(atr, SafePoint());
      double center = SentinelCenterReference(fallback_center);

      double lower_arm = SentinelArmDistance(
         atr, m_clusters[lower_outer].price, m_clusters[lower_inner].price);
      double upper_arm = SentinelArmDistance(
         atr, m_clusters[upper_outer].price, m_clusters[upper_inner].price);

      double lower_stretch = (center - current_price) / safe_atr;
      double upper_stretch = (current_price - center) / safe_atr;

      bool lower_trigger =
         current_price <= m_clusters[lower_outer].price + lower_arm &&
         lower_stretch >= MathMax(m_cfg.sentinel_min_stretch_atr, 0.0);

      bool upper_trigger =
         current_price >= m_clusters[upper_outer].price - upper_arm &&
         upper_stretch >= MathMax(m_cfg.sentinel_min_stretch_atr, 0.0);

      if(!m_lower_sentinel.latched && lower_trigger)
         LatchSentinel(m_lower_sentinel, m_clusters[lower_outer], market_time);
      if(!m_upper_sentinel.latched && upper_trigger)
         LatchSentinel(m_upper_sentinel, m_clusters[upper_outer], market_time);

      if(m_lower_sentinel.latched)
      {
         double excursion = MathMax(0.0,
            m_lower_sentinel.price - current_price) / safe_atr;
         m_lower_sentinel.max_excursion_atr =
            MathMax(m_lower_sentinel.max_excursion_atr, excursion);
      }

      if(m_upper_sentinel.latched)
      {
         double excursion = MathMax(0.0,
            current_price - m_upper_sentinel.price) / safe_atr;
         m_upper_sentinel.max_excursion_atr =
            MathMax(m_upper_sentinel.max_excursion_atr, excursion);
      }
   }

   void UpdateCenterVelocity(const double center,
                             const datetime current_bar_time,
                             const double atr)
   {
      if(center == 0.0)
         return;

      if(m_cog_snapshot_time == 0 || m_cog_snapshot == 0.0)
      {
         m_cog_snapshot = center;
         m_cog_snapshot_time = current_bar_time;
         m_cog_velocity_atr = 0.0;
         return;
      }

      int bars = MathMax(m_cfg.velocity_bars, 1);
      if(current_bar_time - m_cog_snapshot_time < bars * m_calc_period_sec)
         return;

      double elapsed_bars = MathMax(
         (double)(current_bar_time - m_cog_snapshot_time) /
         MathMax(m_calc_period_sec, 1), 1.0);

      m_cog_velocity_atr =
         ((center - m_cog_snapshot) / MathMax(atr, SafePoint())) /
         elapsed_bars;

      m_cog_snapshot = center;
      m_cog_snapshot_time = current_bar_time;
   }

   void BuildReading(const double current_price,
                     const double center,
                     const double field_width,
                     const double atr,
                     const datetime market_time,
                     const datetime calc_bar_time)
   {
      ZeroMemory(m_reading);
      ZeroMemory(m_performance);
      m_reading.valid = true;
      m_reading.cluster_count = m_cluster_count;
      m_reading.nearest_slot = -1;
      m_reading.current_price = current_price;
      m_reading.nearest_price = EMPTY_VALUE;
      m_reading.lower_price = EMPTY_VALUE;
      m_reading.upper_price = EMPTY_VALUE;
      m_reading.center_of_gravity = center;
      m_reading.field_width = field_width;
      m_reading.atr = atr;
      m_reading.market_time = market_time;
      m_reading.calc_bar_time = calc_bar_time;
      m_reading.center_velocity_atr = m_cog_velocity_atr;
      m_reading.refresh_count = m_refresh_count;

      double best_distance = KVP_INF;
      double lower_distance = KVP_INF;
      double upper_distance = KVP_INF;

      for(int c = 0; c < m_cluster_count; c++)
      {
         if(!m_clusters[c].valid)
            continue;

         double d = MathAbs(current_price - m_clusters[c].price);
         if(d < best_distance)
         {
            best_distance = d;
            m_reading.nearest_slot = c;
            m_reading.nearest_price = m_clusters[c].price;
         }

         if(m_clusters[c].price <= current_price)
         {
            double ld = current_price - m_clusters[c].price;
            if(ld < lower_distance)
            {
               lower_distance = ld;
               m_reading.lower_price = m_clusters[c].price;
            }
         }

         if(m_clusters[c].price >= current_price)
         {
            double ud = m_clusters[c].price - current_price;
            if(ud < upper_distance)
            {
               upper_distance = ud;
               m_reading.upper_price = m_clusters[c].price;
            }
         }
      }

      double safe_atr = MathMax(atr, SafePoint());
      m_reading.nearest_distance_atr =
         (m_reading.nearest_price != EMPTY_VALUE)
         ? (current_price - m_reading.nearest_price) / safe_atr : 0.0;
      m_reading.center_distance_atr =
         (center != 0.0) ? (current_price - center) / safe_atr : 0.0;
      m_reading.field_width_atr = field_width / safe_atr;

      if(m_cluster_count >= 2 &&
         m_clusters[0].valid && m_clusters[m_cluster_count - 1].valid)
      {
         m_reading.lattice_width =
            m_clusters[m_cluster_count - 1].price - m_clusters[0].price;
         m_reading.lattice_width_atr = m_reading.lattice_width / safe_atr;
      }
   }

   bool HeavyCoreDue(const datetime market_time,
                     const datetime calc_bar_time,
                     const bool force)
   {
      if(force || !m_core_valid)
         return true;

      if(calc_bar_time > 0 && calc_bar_time != m_last_calc_bar_time)
         return true;

      if(m_cfg.heavy_on_new_bar_only || m_is_tester)
         return false;

      uint cadence = MathMax(m_cfg.live_heavy_refresh_ms, (uint)50);
      return (GetTickCount() - m_last_heavy_ms >= cadence);
   }

   bool RefreshCoreHeavy(const datetime market_time)
   {
      int bars_needed = MathMax(m_cfg.lookback + 2, m_cfg.atr_period + 2);
      MqlRates rates[];
      int copied = CopyRates(m_symbol, m_calc_tf, 0, bars_needed, rates);
      if(copied < MathMax(20, MathMin(m_cfg.lookback + 1, bars_needed)))
         return false;

      ArraySetAsSeries(rates, true);
      int available = MathMin(m_cfg.lookback, copied - 1);
      if(available < MathMax(m_cfg.clusters, 5))
         return false;

      if(!BuildCanonicalClusters(rates, available))
         return false;

      int density_bins = ClampInt(m_cfg.cog_bins, 30, 1000);
      double density[];
      double density_min = 0.0;
      double density_bin_size = 0.0;
      BuildGlobalDensity(rates, available, density_bins,
                         density_min, density_bin_size, density);

      double center = 0.0;
      double field_width = 0.0;
      ComputeCenterAndWidth(density, density_bins, density_min,
                            density_bin_size, center, field_width);

      double atr = ComputeATR(rates, copied, m_cfg.atr_period);
      UpdateCenterVelocity(center, rates[0].time, atr);

      bool is_new_calc_bar =
         (m_last_calc_bar_time != 0 && rates[0].time != m_last_calc_bar_time);

      MqlTick tick;
      double current_price = rates[0].close;
      if(SymbolInfoTick(m_symbol, tick))
      {
         if(tick.bid > 0.0 && tick.ask > 0.0)
            current_price = (tick.bid + tick.ask) * 0.5;
         else if(tick.last > 0.0)
            current_price = tick.last;
      }

      MqlRates closed_bar;
      ZeroMemory(closed_bar);
      if(copied > 1)
         closed_bar = rates[1];

      UpdateSentinels(current_price, center, atr,
                      rates[0], closed_bar,
                      is_new_calc_bar, market_time);

      m_cached_center = center;
      m_cached_field_width = field_width;
      m_cached_atr = atr;
      m_core_valid = true;
      m_last_calc_bar_time = rates[0].time;
      m_last_heavy_market_time = market_time;
      if(!m_is_tester)
         m_last_heavy_ms = GetTickCount();

      m_refresh_count++;
      BuildReading(current_price, center, field_width,
                   atr, market_time, rates[0].time);
      m_reading.refresh_count = m_refresh_count;
      return true;
   }

   bool RefreshCoreFast(const datetime market_time)
   {
      if(!m_core_valid)
         return false;

      MqlRates pair[];
      int copied = CopyRates(m_symbol, m_calc_tf, 0, 2, pair);
      if(copied <= 0)
         return false;
      ArraySetAsSeries(pair, true);

      MqlTick tick;
      double current_price = pair[0].close;
      if(SymbolInfoTick(m_symbol, tick))
      {
         if(tick.bid > 0.0 && tick.ask > 0.0)
            current_price = (tick.bid + tick.ask) * 0.5;
         else if(tick.last > 0.0)
            current_price = tick.last;
      }

      MqlRates closed_bar;
      ZeroMemory(closed_bar);
      if(copied > 1)
         closed_bar = pair[1];

      bool is_new_calc_bar =
         (m_last_calc_bar_time != 0 && pair[0].time != m_last_calc_bar_time);

      UpdateSentinels(current_price, m_cached_center, m_cached_atr,
                      pair[0], closed_bar,
                      is_new_calc_bar, market_time);

      // A new bar should immediately force a heavy rebuild on the next Update().
      BuildReading(current_price,
                   m_cached_center,
                   m_cached_field_width,
                   m_cached_atr,
                   market_time,
                   pair[0].time);
      m_reading.refresh_count = m_refresh_count;
      return true;
   }

   // ----- TPO Market Profile --------------------------------------------------

   ENUM_TIMEFRAMES ProfileAnchorTF(void)
   {
      if(m_cfg.profile_session == KVP_PROFILE_DAILY) return PERIOD_D1;
      if(m_cfg.profile_session == KVP_PROFILE_WEEKLY) return PERIOD_W1;
      if(m_cfg.profile_session == KVP_PROFILE_MONTHLY) return PERIOD_MN1;
      return PERIOD_D1;
   }

   bool ResolveIntradayBounds(const datetime now,
                              const int ordinal,
                              datetime &start_time,
                              datetime &end_time)
   {
      int start_min = ClampInt(m_cfg.intraday_start_minute, 0, 1439);
      int end_min = m_cfg.intraday_end_minute;
      if(end_min == 1440) end_min = 0;
      end_min = ClampInt(end_min, 0, 1439);

      datetime day_open = BrokerDayOpenFor(now);
      if(day_open <= 0)
         return false;

      int duration_min;
      if(start_min == end_min)
         duration_min = 1440;
      else if(end_min > start_min)
         duration_min = end_min - start_min;
      else
         duration_min = (1440 - start_min) + end_min;

      datetime candidate_start = day_open + start_min * 60;
      datetime candidate_end = candidate_start + duration_min * 60;

      if(now < candidate_start)
      {
         // Use previous broker-day anchor if available.
         int dshift = iBarShift(m_symbol, PERIOD_D1, day_open - 1, false);
         datetime prev_day = (dshift >= 0) ? iTime(m_symbol, PERIOD_D1, dshift) : day_open - 86400;
         candidate_start = prev_day + start_min * 60;
         candidate_end = candidate_start + duration_min * 60;
      }
      else if(now >= candidate_end)
      {
         // Current day's window has completed. It is still ordinal 0 until the
         // next configured window begins; this makes the most recent profile stable.
      }

      // Walk backward by completed trading days. For a research include, use D1
      // anchors instead of blindly subtracting 86400 so weekends are naturally skipped.
      if(ordinal > 0)
      {
         datetime anchor = candidate_start;
         for(int n = 0; n < ordinal; n++)
         {
            int shift = iBarShift(m_symbol, PERIOD_D1, anchor - 1, false);
            if(shift < 0)
               return false;
            datetime prev = iTime(m_symbol, PERIOD_D1, shift);
            if(prev <= 0 || prev >= anchor)
               return false;
            anchor = prev + start_min * 60;
         }
         candidate_start = anchor;
         candidate_end = candidate_start + duration_min * 60;
      }

      start_time = candidate_start;
      end_time = candidate_end;
      return true;
   }

   bool ResolveProfileBounds(const datetime now,
                             const int ordinal,
                             datetime &start_time,
                             datetime &end_time)
   {
      if(m_cfg.profile_session == KVP_PROFILE_INTRADAY)
         return ResolveIntradayBounds(now, ordinal, start_time, end_time);

      ENUM_TIMEFRAMES anchor_tf = ProfileAnchorTF();
      datetime start = iTime(m_symbol, anchor_tf, ordinal);
      if(start <= 0)
         return false;

      datetime next = 0;
      if(ordinal == 0)
      {
         next = iTime(m_symbol, anchor_tf, -1); // normally unavailable; handled below.
      }

      // For previous sessions the more recent anchor is the natural end.
      if(ordinal > 0)
         next = iTime(m_symbol, anchor_tf, ordinal - 1);

      if(ordinal == 0 || next <= start)
      {
         int seconds = PeriodSeconds(anchor_tf);
         if(seconds > 0)
            next = start + seconds;
         else
         {
            // Monthly duration is not fixed. Use the next calendar month.
            MqlDateTime p;
            TimeToStruct(start, p);
            p.day = 1;
            p.hour = 0;
            p.min = 0;
            p.sec = 0;
            p.mon++;
            if(p.mon > 12)
            {
               p.mon = 1;
               p.year++;
            }
            next = StructToTime(p);
         }
      }

      start_time = start;
      end_time = next;
      return true;
   }

   double AdaptiveTpoStep(const double session_low,
                          const double session_high)
   {
      int multiplier = m_cfg.point_multiplier;
      if(multiplier <= 0)
      {
         double quote = 0.0;
         MqlTick tick;
         if(SymbolInfoTick(m_symbol, tick))
         {
            if(tick.ask > 0.0) quote = tick.ask;
            else if(tick.bid > 0.0) quote = tick.bid;
            else if(tick.last > 0.0) quote = tick.last;
         }

         if(quote <= 0.0)
            quote = (session_low + session_high) * 0.5;

         string s = DoubleToString(quote, m_digits);
         StringReplace(s, "-", "");
         int total_digits = StringLen(s);
         if(StringFind(s, ".") != -1)
            total_digits--;
         multiplier = (total_digits <= 5)
            ? 1 : (int)MathPow(10.0, total_digits - 5);
      }

      multiplier = MathMax(multiplier, 1);
      double step = MathMax(m_point * multiplier, SafeTick());

      // Prevent pathological arrays on large-range symbols while preserving
      // deterministic TPO geometry.
      int max_bins = MathMax(m_cfg.max_profile_bins, 100);
      double range = MathMax(session_high - session_low, step);
      int bins = (int)MathCeil(range / step) + 2;
      if(bins > max_bins)
      {
         double scale = MathCeil((double)bins / max_bins);
         step *= MathMax(scale, 1.0);
      }

      return step;
   }

   bool BuildOneProfile(const datetime now,
                        const int ordinal,
                        KVP_Profile &profile)
   {
      ZeroMemory(profile);
      profile.ordinal = ordinal;
      profile.session_kind = m_cfg.profile_session;

      datetime session_start = 0;
      datetime session_end = 0;
      if(!ResolveProfileBounds(now, ordinal, session_start, session_end))
         return false;

      datetime effective_end = MathMin(now, session_end - 1);
      if(effective_end < session_start)
         return false;

      MqlRates bars[];
      int copied = CopyRates(m_symbol, m_profile_tf,
                             session_start, effective_end, bars);
      if(copied <= 0)
         return false;

      ArraySetAsSeries(bars, false);

      double session_high = -KVP_INF;
      double session_low = KVP_INF;
      for(int i = 0; i < copied; i++)
      {
         session_high = MathMax(session_high, bars[i].high);
         session_low = MathMin(session_low, bars[i].low);
      }
      if(session_high <= session_low)
         return false;

      double step = AdaptiveTpoStep(session_low, session_high);
      session_low = MathFloor(session_low / step) * step;
      session_high = MathCeil(session_high / step) * step;

      int bin_count = (int)MathRound((session_high - session_low) / step) + 1;
      bin_count = MathMax(bin_count, 1);
      if(bin_count > MathMax(m_cfg.max_profile_bins, 100))
         return false;

      int tpo[];
      ArrayResize(tpo, bin_count);
      ArrayInitialize(tpo, 0);

      int total_tpo = 0;
      for(int i = 0; i < copied; i++)
      {
         int first = ClampInt((int)MathCeil((bars[i].low - session_low) / step - 1.0e-9),
                              0, bin_count - 1);
         int last = ClampInt((int)MathFloor((bars[i].high - session_low) / step + 1.0e-9),
                             0, bin_count - 1);
         if(last < first)
            continue;

         for(int b = first; b <= last; b++)
         {
            tpo[b]++;
            total_tpo++;
         }
      }

      if(total_tpo <= 0)
         return false;

      // POC = maximum TPO count. If tied, choose the level closest to session center.
      double session_center = (session_low + session_high) * 0.5;
      int poc_idx = 0;
      int poc_tpo = -1;
      double best_center_distance = KVP_INF;
      for(int b = 0; b < bin_count; b++)
      {
         double price = session_low + b * step;
         double center_distance = MathAbs(price - session_center);
         if(tpo[b] > poc_tpo ||
            (tpo[b] == poc_tpo && center_distance < best_center_distance))
         {
            poc_idx = b;
            poc_tpo = tpo[b];
            best_center_distance = center_distance;
         }
      }

      int target_tpo = (int)MathRound(
         total_tpo * ClampDouble((double)m_cfg.value_area_percent / 100.0,
                                 0.01, 1.0));
      int included = tpo[poc_idx];
      int up = poc_idx + 1;
      int down = poc_idx - 1;
      int top_idx = poc_idx;
      int bottom_idx = poc_idx;

      while(included < target_tpo && (up < bin_count || down >= 0))
      {
         int up_tpo = (up < bin_count) ? tpo[up] : -1;
         int down_tpo = (down >= 0) ? tpo[down] : -1;

         // Match the original behavior: ties expand upward first.
         if(up < bin_count && (down < 0 || up_tpo >= down_tpo))
         {
            included += MathMax(up_tpo, 0);
            top_idx = up;
            up++;
         }
         else if(down >= 0)
         {
            included += MathMax(down_tpo, 0);
            bottom_idx = down;
            down--;
         }
         else
         {
            break;
         }
      }

      profile.valid = true;
      profile.session_start = session_start;
      profile.session_end = session_end;
      profile.developing = (ordinal == 0 && now < session_end);
      profile.session_high = session_high;
      profile.session_low = session_low;
      profile.tpo_step = step;
      profile.bar_count = copied;
      profile.bin_count = bin_count;
      profile.total_tpo = total_tpo;
      profile.poc = session_low + poc_idx * step;
      profile.vah = session_low + top_idx * step;
      profile.val = session_low + bottom_idx * step;
      profile.poc_tpo = poc_tpo;
      profile.poc_bar_share_pct = (copied > 0)
         ? (double)poc_tpo / copied * 100.0 : 0.0;
      profile.prominent_poc =
         (profile.poc_bar_share_pct >= m_cfg.prominent_poc_percent);

      for(int s = 0; s < KVP_MAX_SINGLE_PRINTS; s++)
         ZeroMemory(profile.single_prints[s]);
      profile.single_print_count = 0;

      if(m_cfg.enable_single_prints)
      {
         int b = 0;
         while(b < bin_count && profile.single_print_count < KVP_MAX_SINGLE_PRINTS)
         {
            if(tpo[b] != 1)
            {
               b++;
               continue;
            }

            int start_bin = b;
            int end_bin = b;
            while(end_bin + 1 < bin_count && tpo[end_bin + 1] == 1)
               end_bin++;

            int s = profile.single_print_count;
            profile.single_prints[s].valid = true;
            profile.single_prints[s].low = session_low + start_bin * step;
            profile.single_prints[s].high = session_low + (end_bin + 1) * step;
            profile.single_print_count++;
            b = end_bin + 1;
         }
      }

      return true;
   }

   bool RefreshProfiles(const datetime market_time, const bool force)
   {
      if(!m_cfg.enable_profile)
      {
         m_profile_count = 0;
         return true;
      }

      datetime profile_bar = iTime(m_symbol, m_profile_tf, 0);
      if(!force && profile_bar > 0 && profile_bar == m_last_profile_calc_bar_time)
         return true;

      int count = ClampInt(m_cfg.profiles_to_keep, 1, KVP_MAX_PROFILES);
      m_profile_count = 0;
      for(int i = 0; i < KVP_MAX_PROFILES; i++)
         ZeroMemory(m_profiles[i]);

      for(int ordinal = 0; ordinal < count; ordinal++)
      {
         KVP_Profile p;
         if(BuildOneProfile(market_time, ordinal, p))
         {
            m_profiles[m_profile_count] = p;
            m_profile_count++;
         }
      }

      m_last_profile_calc_bar_time = profile_bar;
      return true;
   }

   // ----- export --------------------------------------------------------------

   void AppendLevel(KVP_Level &out[], const KVP_Level &level)
   {
      int n = ArraySize(out);
      ArrayResize(out, n + 1);
      out[n] = level;
   }

   void ExportCoreLevels(KVP_Level &out[])
   {
      if(!m_core_valid || !m_reading.valid)
         return;

      double safe_atr = MathMax(m_reading.atr, SafePoint());

      for(int c = 0; c < m_cluster_count; c++)
      {
         if(!m_clusters[c].valid)
            continue;

         KVP_Level level;
         ZeroMemory(level);
         level.valid = true;
         level.family = KVP_FAMILY_ADAPTIVE_VALUE;
         level.kind = ClusterKind(c);
         level.role = ClusterRole(c, m_cluster_count);
         level.lower = m_clusters[c].zone_low;
         level.price = m_clusters[c].price;
         level.upper = m_clusters[c].zone_high;
         level.timeframe = m_calc_tf;
         level.created_at = m_level_states[c].birth_time;
         level.updated_at = m_reading.market_time;
         level.source_slot = c;
         level.ordinal = 0;
         level.developing = true;
         level.frozen = false;
         level.local_id = MakeId((int)level.family, (int)level.kind,
                                 m_level_states[c].generation,
                                 level.created_at);
         level.state = m_level_states[c].status;
         level.direction_state = m_level_states[c].direction_state;
         level.width_state = m_level_states[c].width_state;
         level.mass_state = m_level_states[c].mass_state;
         level.touches = m_level_states[c].touch_count;
         level.rejections = m_level_states[c].rejection_count;
         level.reclaims = m_level_states[c].reclaim_success;
         level.acceptance_bars = m_level_states[c].cumulative_acceptance;
         level.mass = m_clusters[c].total_mass;
         level.mass_share = m_clusters[c].mass_share;
         level.poc_mass = m_clusters[c].poc_mass;
         level.width_atr = (m_clusters[c].zone_high - m_clusters[c].zone_low) / safe_atr;
         level.distance_atr = (m_reading.current_price - level.price) / safe_atr;
         AppendLevel(out, level);
      }

      if(m_cached_center != 0.0)
      {
         KVP_Level cog;
         ZeroMemory(cog);
         cog.valid = true;
         cog.family = KVP_FAMILY_GLOBAL_VALUE;
         cog.kind = KVP_KIND_COG;
         cog.role = KVP_ROLE_CENTER;
         cog.lower = m_cached_center;
         cog.price = m_cached_center;
         cog.upper = m_cached_center;
         cog.timeframe = m_calc_tf;
         cog.created_at = m_cog_snapshot_time;
         cog.updated_at = m_reading.market_time;
         cog.developing = true;
         cog.frozen = false;
         cog.local_id = MakeId((int)cog.family, (int)cog.kind, 1,
                               m_cog_snapshot_time);
         cog.direction_state = (m_cog_velocity_atr > 0.0) ? KVP_DIR_RISE :
                               (m_cog_velocity_atr < 0.0) ? KVP_DIR_FALL :
                               KVP_DIR_STABLE;
         cog.width_atr = m_reading.field_width_atr;
         cog.distance_atr = m_reading.center_distance_atr;
         AppendLevel(out, cog);
      }

      if(m_lower_sentinel.latched)
      {
         KVP_Level lower;
         ZeroMemory(lower);
         lower.valid = true;
         lower.family = KVP_FAMILY_SENTINEL;
         lower.kind = KVP_KIND_LOWER_SENTINEL;
         lower.role = KVP_ROLE_LOWER_MEMORY;
         lower.lower = m_lower_sentinel.price;
         lower.price = m_lower_sentinel.price;
         lower.upper = m_lower_sentinel.price;
         lower.timeframe = m_calc_tf;
         lower.created_at = m_lower_sentinel.latched_at;
         lower.updated_at = m_reading.market_time;
         lower.source_slot = m_lower_sentinel.source_slot;
         lower.developing = false;
         lower.frozen = true;
         lower.local_id = MakeId((int)lower.family, (int)lower.kind,
                                 m_lower_sentinel.generation,
                                 m_lower_sentinel.latched_at);
         lower.distance_atr =
            (m_reading.current_price - lower.price) / safe_atr;
         lower.max_excursion_atr = m_lower_sentinel.max_excursion_atr;
         AppendLevel(out, lower);
      }

      if(m_upper_sentinel.latched)
      {
         KVP_Level upper;
         ZeroMemory(upper);
         upper.valid = true;
         upper.family = KVP_FAMILY_SENTINEL;
         upper.kind = KVP_KIND_UPPER_SENTINEL;
         upper.role = KVP_ROLE_UPPER_MEMORY;
         upper.lower = m_upper_sentinel.price;
         upper.price = m_upper_sentinel.price;
         upper.upper = m_upper_sentinel.price;
         upper.timeframe = m_calc_tf;
         upper.created_at = m_upper_sentinel.latched_at;
         upper.updated_at = m_reading.market_time;
         upper.source_slot = m_upper_sentinel.source_slot;
         upper.developing = false;
         upper.frozen = true;
         upper.local_id = MakeId((int)upper.family, (int)upper.kind,
                                 m_upper_sentinel.generation,
                                 m_upper_sentinel.latched_at);
         upper.distance_atr =
            (m_reading.current_price - upper.price) / safe_atr;
         upper.max_excursion_atr = m_upper_sentinel.max_excursion_atr;
         AppendLevel(out, upper);
      }
   }

   void ExportProfileLevels(KVP_Level &out[])
   {
      for(int i = 0; i < m_profile_count; i++)
      {
         if(!m_profiles[i].valid)
            continue;

         KVP_Profile p = m_profiles[i];
         double prices[3];
         prices[0] = p.poc;
         prices[1] = p.vah;
         prices[2] = p.val;

         KVP_LEVEL_KIND kinds[3];
         kinds[0] = KVP_KIND_PROFILE_POC;
         kinds[1] = KVP_KIND_PROFILE_VAH;
         kinds[2] = KVP_KIND_PROFILE_VAL;

         KVP_LEVEL_ROLE roles[3];
         roles[0] = KVP_ROLE_CENTER;
         roles[1] = KVP_ROLE_UPPER_BOUNDARY;
         roles[2] = KVP_ROLE_LOWER_BOUNDARY;

         for(int k = 0; k < 3; k++)
         {
            KVP_Level level;
            ZeroMemory(level);
            level.valid = true;
            level.family = KVP_FAMILY_TPO_PROFILE;
            level.kind = kinds[k];
            level.role = roles[k];
            level.lower = prices[k];
            level.price = prices[k];
            level.upper = prices[k];
            level.timeframe = m_profile_tf;
            level.created_at = p.session_start;
            level.updated_at = m_reading.market_time;
            level.session_start = p.session_start;
            level.session_end = p.session_end;
            level.ordinal = p.ordinal;
            level.developing = p.developing;
            level.frozen = !p.developing;
            level.local_id = MakeId((int)level.family, (int)level.kind,
                                    (ulong)(p.ordinal + 1), p.session_start);
            level.mass = (double)p.total_tpo;
            level.poc_mass = (double)p.poc_tpo;
            if(m_reading.atr > 0.0)
               level.distance_atr =
                  (m_reading.current_price - level.price) / m_reading.atr;
            AppendLevel(out, level);
         }

         if(m_cfg.enable_single_prints)
         {
            for(int s = 0; s < p.single_print_count; s++)
            {
               if(!p.single_prints[s].valid)
                  continue;

               KVP_Level sp;
               ZeroMemory(sp);
               sp.valid = true;
               sp.family = KVP_FAMILY_SINGLE_PRINT;
               sp.kind = KVP_KIND_SINGLE_PRINT_BAND;
               sp.role = KVP_ROLE_NONE;
               sp.lower = p.single_prints[s].low;
               sp.upper = p.single_prints[s].high;
               sp.price = (sp.lower + sp.upper) * 0.5;
               sp.timeframe = m_profile_tf;
               sp.created_at = p.session_start;
               sp.updated_at = m_reading.market_time;
               sp.session_start = p.session_start;
               sp.session_end = p.session_end;
               sp.ordinal = p.ordinal;
               sp.source_slot = s;
               sp.developing = p.developing;
               sp.frozen = !p.developing;
               sp.local_id = MakeId((int)sp.family, (int)sp.kind,
                                    (ulong)(s + 1 + p.ordinal * KVP_MAX_SINGLE_PRINTS),
                                    p.session_start);
               if(m_reading.atr > 0.0)
               {
                  sp.width_atr = (sp.upper - sp.lower) / m_reading.atr;
                  sp.distance_atr =
                     (m_reading.current_price - sp.price) / m_reading.atr;
               }
               AppendLevel(out, sp);
            }
         }
      }
   }

public:
   CKittVolKittMarketProfileProducer(void)
   {
      m_initialized = false;
      m_is_tester = false;
      m_symbol = "";
      m_calc_tf = PERIOD_CURRENT;
      m_profile_tf = PERIOD_CURRENT;
      m_digits = 0;
      m_point = 0.0;
      m_tick_size = 0.0;
      m_calc_period_sec = 1;
      m_cluster_count = 0;
      m_profile_count = 0;
      m_core_valid = false;
      m_last_calc_bar_time = 0;
      m_last_heavy_market_time = 0;
      m_last_heavy_ms = 0;
      m_sentinel_reset_key = 0;
      m_cog_snapshot = 0.0;
      m_cog_snapshot_time = 0;
      m_cog_velocity_atr = 0.0;
      m_refresh_count = 0;
      m_last_profile_calc_bar_time = 0;
      ArrayResize(m_mem_centroids, 0);
      ZeroMemory(m_reading);
      ZeroMemory(m_performance);
      ZeroMemory(m_lower_sentinel);
      ZeroMemory(m_upper_sentinel);

      for(int i = 0; i < KVP_MAX_CLUSTERS; i++)
      {
         ZeroMemory(m_clusters[i]);
         ZeroMemory(m_level_states[i]);
         m_level_states[i].status = KVP_LEVEL_FRESH;
         m_level_states[i].direction_state = KVP_DIR_STABLE;
         m_level_states[i].width_state = KVP_WIDTH_STABLE;
         m_level_states[i].mass_state = KVP_MASS_STABLE;
      }
      for(int i = 0; i < KVP_MAX_PROFILES; i++)
         ZeroMemory(m_profiles[i]);
   }

   bool Init(const string symbol,
             const ENUM_TIMEFRAMES calc_tf,
             const ENUM_TIMEFRAMES profile_tf,
             const KVP_Config &cfg)
   {
      m_symbol = symbol;
      m_calc_tf = (calc_tf == PERIOD_CURRENT)
         ? (ENUM_TIMEFRAMES)_Period : calc_tf;
      m_profile_tf = (profile_tf == PERIOD_CURRENT)
         ? (ENUM_TIMEFRAMES)_Period : profile_tf;
      m_cfg = cfg;
      m_is_tester = (bool)MQLInfoInteger(MQL_TESTER);

      if(m_cfg.lookback < 20) return false;
      if(m_cfg.clusters < 1 || m_cfg.clusters > KVP_MAX_CLUSTERS) return false;
      if(m_cfg.iterations < 1) return false;
      if(m_cfg.rows_per_cluster < 2) return false;
      if(m_cfg.atr_period < 2) return false;
      if(m_cfg.cog_bins < 30) return false;
      if(m_cfg.velocity_bars < 1) return false;
      if(m_cfg.enable_sentinels && m_cfg.clusters < 3) return false;
      if(m_cfg.sentinel_reentry_bars < 1) return false;
      if(m_cfg.profiles_to_keep < 1 || m_cfg.profiles_to_keep > KVP_MAX_PROFILES) return false;
      if(m_cfg.value_area_percent < 1 || m_cfg.value_area_percent > 100) return false;
      if(m_cfg.max_profile_bins < 100) return false;

      m_digits = (int)SymbolInfoInteger(m_symbol, SYMBOL_DIGITS);
      m_point = SymbolInfoDouble(m_symbol, SYMBOL_POINT);
      m_tick_size = SymbolInfoDouble(m_symbol, SYMBOL_TRADE_TICK_SIZE);
      if(m_point <= 0.0)
         return false;
      if(m_tick_size <= 0.0)
         m_tick_size = m_point;

      m_calc_period_sec = MathMax(PeriodSeconds(m_calc_tf), 1);

      ArrayResize(m_mem_centroids, 0);
      m_cluster_count = 0;
      m_profile_count = 0;
      m_core_valid = false;
      m_last_calc_bar_time = 0;
      m_last_heavy_market_time = 0;
      m_last_heavy_ms = 0;
      m_sentinel_reset_key = 0;
      m_cog_snapshot = 0.0;
      m_cog_snapshot_time = 0;
      m_cog_velocity_atr = 0.0;
      m_refresh_count = 0;
      m_last_profile_calc_bar_time = 0;
      ZeroMemory(m_reading);

      for(int i = 0; i < KVP_MAX_CLUSTERS; i++)
      {
         ZeroMemory(m_clusters[i]);
         ZeroMemory(m_level_states[i]);
         m_level_states[i].status = KVP_LEVEL_FRESH;
         m_level_states[i].direction_state = KVP_DIR_STABLE;
         m_level_states[i].width_state = KVP_WIDTH_STABLE;
         m_level_states[i].mass_state = KVP_MASS_STABLE;
      }

      ZeroMemory(m_lower_sentinel);
      ZeroMemory(m_upper_sentinel);
      m_lower_sentinel.upper_side = false;
      m_lower_sentinel.source_slot = -1;
      m_lower_sentinel.generation = 1;
      m_upper_sentinel.upper_side = true;
      m_upper_sentinel.source_slot = -1;
      m_upper_sentinel.generation = 1;

      for(int i = 0; i < KVP_MAX_PROFILES; i++)
         ZeroMemory(m_profiles[i]);

      m_initialized = true;
      return true;
   }

   bool Init(const string symbol, const ENUM_TIMEFRAMES calc_tf)
   {
      KVP_Config cfg;
      KVP_DefaultConfig(cfg);
      return Init(symbol, calc_tf, calc_tf, cfg);
   }

   void Reset(void)
   {
      if(!m_initialized)
         return;
      KVP_Config cfg = m_cfg;
      string symbol = m_symbol;
      ENUM_TIMEFRAMES calc_tf = m_calc_tf;
      ENUM_TIMEFRAMES profile_tf = m_profile_tf;
      Init(symbol, calc_tf, profile_tf, cfg);
   }

   bool Update(const bool force_heavy = false,
               const bool force_profile = false)
   {
      if(!m_initialized)
         return false;

      ZeroMemory(m_performance);
      ulong update_start_us = GetMicrosecondCount();

      datetime market_time = TimeCurrent();
      MqlTick tick;
      if(SymbolInfoTick(m_symbol, tick) && tick.time > 0)
         market_time = tick.time;

      datetime calc_bar_time = iTime(m_symbol, m_calc_tf, 0);
      bool core_ok = true;
      m_performance.heavy_core_due =
         HeavyCoreDue(market_time, calc_bar_time, force_heavy);
      ulong core_start_us = GetMicrosecondCount();
      if(m_performance.heavy_core_due)
      {
         core_ok = RefreshCoreHeavy(market_time);
         if(!core_ok && m_core_valid)
            core_ok = RefreshCoreFast(market_time);
      }
      else
      {
         core_ok = RefreshCoreFast(market_time);
      }
      m_performance.core_microseconds = GetMicrosecondCount() - core_start_us;

      // Profile is structural and normally needs only one rebuild per profile TF
      // bar. A master controller may force it after a history/revision event.
      datetime profile_bar = iTime(m_symbol, m_profile_tf, 0);
      m_performance.profile_rebuild_due =
         m_cfg.enable_profile &&
         (force_profile || profile_bar <= 0 ||
          profile_bar != m_last_profile_calc_bar_time);
      ulong profile_start_us = GetMicrosecondCount();
      bool profile_ok = RefreshProfiles(market_time, force_profile);
      m_performance.profile_microseconds =
         GetMicrosecondCount() - profile_start_us;
      m_performance.update_microseconds =
         GetMicrosecondCount() - update_start_us;
      return core_ok && profile_ok;
   }

   bool UpdateCoreOnly(const bool force_heavy = false)
   {
      if(!m_initialized)
         return false;

      datetime market_time = TimeCurrent();
      MqlTick tick;
      if(SymbolInfoTick(m_symbol, tick) && tick.time > 0)
         market_time = tick.time;

      datetime calc_bar_time = iTime(m_symbol, m_calc_tf, 0);
      if(HeavyCoreDue(market_time, calc_bar_time, force_heavy))
      {
         bool ok = RefreshCoreHeavy(market_time);
         if(!ok && m_core_valid)
            return RefreshCoreFast(market_time);
         return ok;
      }
      return RefreshCoreFast(market_time);
   }

   bool UpdateProfileOnly(const bool force_profile = false)
   {
      if(!m_initialized)
         return false;
      datetime market_time = TimeCurrent();
      MqlTick tick;
      if(SymbolInfoTick(m_symbol, tick) && tick.time > 0)
         market_time = tick.time;
      return RefreshProfiles(market_time, force_profile);
   }

   bool IsInitialized(void) { return m_initialized; }
   bool CoreValid(void) { return m_core_valid; }
   int ClusterCount(void) { return m_cluster_count; }
   int ProfileCount(void) { return m_profile_count; }

   bool GetCluster(const int slot, KVP_Cluster &out)
   {
      if(slot < 0 || slot >= m_cluster_count)
         return false;
      out = m_clusters[slot];
      return out.valid;
   }

   bool GetClusterState(const int slot, KVP_LevelState &out)
   {
      if(slot < 0 || slot >= m_cluster_count)
         return false;
      out = m_level_states[slot];
      return true;
   }

   bool GetProfile(const int ordinal_index, KVP_Profile &out)
   {
      if(ordinal_index < 0 || ordinal_index >= m_profile_count)
         return false;
      out = m_profiles[ordinal_index];
      return out.valid;
   }

   void GetReading(KVP_CoreReading &out)
   {
      out = m_reading;
   }

   void GetPerformance(KVP_PerformanceReading &out)
   {
      out = m_performance;
   }

   void GetLowerSentinel(KVP_Sentinel &out)
   {
      out = m_lower_sentinel;
   }

   void GetUpperSentinel(KVP_Sentinel &out)
   {
      out = m_upper_sentinel;
   }

   // Primary master-controller seam.
   int ExportLevels(KVP_Level &out[])
   {
      ArrayResize(out, 0);
      ExportCoreLevels(out);
      ExportProfileLevels(out);
      return ArraySize(out);
   }
};

#endif // __KITT_02_VOLKITT_MARKETPROFILE_PRODUCER_MQH__
