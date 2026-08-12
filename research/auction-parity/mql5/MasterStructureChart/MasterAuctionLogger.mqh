//+------------------------------------------------------------------+
//| MasterAuctionLogger.mqh                                          |
//| Append-only event, attempt, episode, and frozen-context receipts. |
//+------------------------------------------------------------------+
#ifndef __KITT_MASTER_AUCTION_LOGGER_MQH__
#define __KITT_MASTER_AUCTION_LOGGER_MQH__

#include "MasterAuctionTypes.mqh"

class CMstAuctionLogger
{
private:
   bool   m_enabled;
   int    m_events_handle;
   int    m_attempts_handle;
   int    m_episodes_handle;
   int    m_context_handle;
   int    m_features_handle;
   int    m_transits_handle;
   int    m_runs_handle;
   int    m_flush_interval;
   int    m_pending_rows;
   string m_session_id;
   long   m_event_rows;
   long   m_attempt_rows;
   long   m_episode_rows;
   long   m_context_rows;
   long   m_feature_rows;
   long   m_transit_rows;

   string SafeToken(string value)
   {
      StringReplace(value, "\\", "_");
      StringReplace(value, "/", "_");
      StringReplace(value, ":", "_");
      StringReplace(value, " ", "_");
      StringReplace(value, ".", "_");
      return value;
   }

   int OpenFile(const string filename, const int kind)
   {
      int handle = FileOpen(filename,
                            FILE_READ|FILE_WRITE|FILE_CSV|FILE_ANSI|FILE_COMMON,
                            '\t');
      if(handle == INVALID_HANDLE) return INVALID_HANDLE;
      if(FileSize(handle) == 0)
      {
         if(kind == 0)
            FileWrite(handle, "schema", "session", "event_id", "event_sequence",
                      "episode_id", "attempt_id", "node_id", "related_node_id",
                      "timestamp", "bar_time", "event_type", "direction", "price",
                      "atr", "distance_atr", "penetration_atr", "bar_sequence",
                      "attempt_event_sequence", "structure_snapshot_hash",
                      "regional_basis_hash", "price_region", "median_price",
                      "structural_sigma", "price_from_median_atr",
                      "price_from_median_sigma", "price_from_cog_atr",
                      "price_from_cog_sigma", "cog_median_gap_sigma");
         else if(kind == 1)
            FileWrite(handle, "schema", "session", "attempt_id", "episode_id", "node_id",
                      "evidence_id", "ordinal", "is_retest", "start", "contact", "break",
                      "accepted", "end", "direction", "resolution", "start_bar", "contact_bar",
                      "end_bar", "approach_start_price", "contact_price", "frozen_atr",
                      "frozen_lower", "frozen_price", "frozen_upper", "node_width_atr",
                      "initial_distance_atr", "approach_efficiency", "penetration_atr",
                      "penetration_node", "inside_updates", "inside_seconds", "far_closes",
                      "max_above_atr", "max_below_atr", "rejection_excursion_atr",
                      "nearest_above_id", "nearest_below_id", "corridor_up_atr",
                      "corridor_down_atr", "family_mask", "family_count", "member_count",
                      "developing_count", "frozen_count", "node_revision",
                      "completion_status", "censor_reason", "corridor_up_level_count",
                      "corridor_down_level_count", "corridor_up_noise_count",
                      "corridor_down_noise_count", "start_region", "end_region",
                      "node_region", "start_median_sigma", "end_median_sigma",
                      "min_median_sigma", "max_median_sigma",
                      "node_from_median_sigma", "node_from_cog_sigma",
                      "node_width_sigma");
         else if(kind == 2)
            FileWrite(handle, "schema", "session", "episode_id", "node_id", "start", "end",
                      "first_direction", "attempts", "breaks", "reclaims", "retests",
                      "next_node_id", "corridor_up_atr", "corridor_down_atr", "node_width_atr",
                      "family_mask", "family_count", "member_count", "resolution",
                      "max_up_excursion_atr", "max_down_excursion_atr", "duration_bars",
                      "duration_seconds", "completion_status", "censor_reason",
                      "initial_region", "terminal_region");
         else if(kind == 3)
            FileWrite(handle, "schema", "session", "attempt_id", "episode_id", "node_id",
                      "frozen_at", "source_key", "producer", "producer_instance", "local_id",
                      "family", "source_kind");
         else if(kind == 4)
            FileWrite(handle, "schema", "session", "attempt_id", "frozen_bar_time",
                      "has_cog", "cog_price", "cog_distance_atr", "cog_velocity_atr",
                      "has_c3", "c3_price", "c3_distance_atr", "c3_velocity_atr",
                      "has_lattice", "lattice_width_atr", "has_field", "field_width_atr",
                      "has_profile", "poc_price", "vah_price", "val_price",
                      "poc_distance_atr", "vah_distance_atr", "val_distance_atr",
                      "spread_atr", "raw_level_count", "noise_level_count", "active_node_count",
                      "regional_valid", "sigma_valid", "basis_changed",
                      "structure_snapshot_hash", "regional_basis_hash", "structure_generation",
                      "population_count", "population_count_delta", "velocity_elapsed_bars",
                      "median_price", "mean_price", "structural_sigma", "reference_price",
                      "price_region", "price_from_median_atr", "price_from_median_sigma",
                      "price_from_cog_atr", "price_from_cog_sigma", "cog_region",
                      "cog_median_gap_atr", "cog_median_gap_sigma",
                      "regional_cog_velocity_price", "regional_cog_velocity_atr",
                      "regional_cog_velocity_sigma", "regional_median_velocity_price",
                      "regional_median_velocity_atr", "regional_median_velocity_sigma",
                      "regional_sigma_log_change_per_bar");
         else if(kind == 5)
            FileWrite(handle, "schema", "session", "transit_id", "attempt_id", "episode_id",
                      "source_node_id", "destination_node_id", "direction", "start", "end",
                      "start_bar", "end_bar", "start_price", "end_price", "source_lower",
                      "source_upper", "destination_lower", "destination_upper", "frozen_atr",
                      "distance_atr", "path_length", "path_efficiency", "max_adverse_atr",
                      "completion_status", "censor_reason", "resolution", "duration_bars",
                      "duration_seconds", "regional_basis_hash", "structure_snapshot_hash",
                      "source_region", "destination_region", "start_price_region",
                      "end_price_region", "start_median_price", "start_structural_sigma",
                      "start_price_from_median_sigma", "end_price_from_median_sigma");
         else
            FileWrite(handle, "schema", "record_type", "session", "build_id", "symbol",
                      "timeframe", "digits", "point", "tick_size", "account_server",
                      "account_company", "tester", "visual_mode", "start", "end",
                      "terminal_reason", "config_hash", "config_text", "terminal_hash",
                      "event_sequence", "attempts_started", "attempts_resolved",
                      "attempts_censored", "episodes_started", "episodes_resolved",
                      "episodes_censored", "transits_started", "transits_resolved",
                      "transits_censored", "events_emitted", "active_attempts",
                      "active_episodes", "active_transits", "balanced", "receipt_hash",
                      "event_rows", "attempt_rows", "episode_rows", "context_rows",
                      "feature_rows", "transit_rows");
      }
      FileSeek(handle, 0, SEEK_END);
      return handle;
   }

   void MaybeFlush(void)
   {
      if(m_pending_rows < m_flush_interval) return;
      FileFlush(m_events_handle);
      FileFlush(m_attempts_handle);
      FileFlush(m_episodes_handle);
      FileFlush(m_context_handle);
      FileFlush(m_features_handle);
      FileFlush(m_transits_handle);
      FileFlush(m_runs_handle);
      m_pending_rows = 0;
   }

public:
   CMstAuctionLogger(void)
   {
      m_enabled = false;
      m_events_handle = INVALID_HANDLE;
      m_attempts_handle = INVALID_HANDLE;
      m_episodes_handle = INVALID_HANDLE;
      m_context_handle = INVALID_HANDLE;
      m_features_handle = INVALID_HANDLE;
      m_transits_handle = INVALID_HANDLE;
      m_runs_handle = INVALID_HANDLE;
      m_flush_interval = 32;
      m_pending_rows = 0;
      m_session_id = "";
      m_event_rows = 0;
      m_attempt_rows = 0;
      m_episode_rows = 0;
      m_context_rows = 0;
      m_feature_rows = 0;
      m_transit_rows = 0;
   }

   bool Init(const bool enabled,
             const string symbol,
             const ENUM_TIMEFRAMES timeframe,
             const string instance_tag,
             const int flush_interval = 32)
   {
      Close();
      m_enabled = enabled;
      m_flush_interval = MathMax(flush_interval, 1);
      if(!enabled) return true;
      string market_stamp = SafeToken(TimeToString(TimeCurrent(), TIME_DATE|TIME_MINUTES|TIME_SECONDS));
      m_session_id = SafeToken(symbol) + "_" + SafeToken(EnumToString(timeframe)) +
                     "_" + SafeToken(instance_tag) + "_" + market_stamp + "_R" +
                     StringFormat("%I64u", GetTickCount64());
      string stem = "MasterAuction_" + SafeToken(symbol) + "_" +
                    SafeToken(EnumToString(timeframe)) + "_" + SafeToken(instance_tag) + "_v5";
      m_events_handle = OpenFile(stem + "_events.tsv", 0);
      m_attempts_handle = OpenFile(stem + "_attempts.tsv", 1);
      m_episodes_handle = OpenFile(stem + "_episodes.tsv", 2);
      m_context_handle = OpenFile(stem + "_context.tsv", 3);
      m_features_handle = OpenFile(stem + "_features.tsv", 4);
      m_transits_handle = OpenFile(stem + "_transits.tsv", 5);
      m_runs_handle = OpenFile(stem + "_runs.tsv", 6);
      if(m_events_handle == INVALID_HANDLE || m_attempts_handle == INVALID_HANDLE ||
         m_episodes_handle == INVALID_HANDLE || m_context_handle == INVALID_HANDLE ||
         m_features_handle == INVALID_HANDLE || m_transits_handle == INVALID_HANDLE ||
         m_runs_handle == INVALID_HANDLE)
      {
         Close();
         m_enabled = false;
         return false;
      }
      return true;
   }

   bool WriteCycle(const MST_AuctionEvent &events[], const int event_count,
                   const MST_AuctionAttempt &attempts[], const int attempt_count,
                   const MST_AuctionEpisode &episodes[], const int episode_count,
                   const MST_AuctionContextReceipt &context[], const int context_count,
                   const MST_TransitTrack &transits[], const int transit_count)
   {
      if(!m_enabled) return true;
      if(m_events_handle == INVALID_HANDLE || m_attempts_handle == INVALID_HANDLE ||
         m_episodes_handle == INVALID_HANDLE || m_context_handle == INVALID_HANDLE ||
         m_features_handle == INVALID_HANDLE || m_transits_handle == INVALID_HANDLE)
         return false;

      int count = MathMin(event_count, ArraySize(events));
      for(int i = 0; i < count; i++)
      {
         FileWrite(m_events_handle, IntegerToString(MST_SCHEMA_VERSION), m_session_id,
                   StringFormat("%I64u", events[i].event_id),
                   StringFormat("%I64u", events[i].event_sequence),
                   StringFormat("%I64u", events[i].episode_id),
                   StringFormat("%I64u", events[i].attempt_id),
                   StringFormat("%I64u", events[i].node_id),
                   StringFormat("%I64u", events[i].related_node_id),
                   TimeToString(events[i].market_time, TIME_DATE|TIME_MINUTES|TIME_SECONDS),
                   TimeToString(events[i].bar_time, TIME_DATE|TIME_MINUTES|TIME_SECONDS),
                   MST_AuctionEventName(events[i].kind), IntegerToString((int)events[i].direction),
                   DoubleToString(events[i].price, _Digits), DoubleToString(events[i].atr, _Digits),
                   DoubleToString(events[i].distance_atr, 8),
                   DoubleToString(events[i].penetration_atr, 8),
                   IntegerToString(events[i].bar_sequence),
                   IntegerToString(events[i].attempt_event_sequence),
                   StringFormat("%I64u", events[i].regional.structure_snapshot_hash),
                   StringFormat("%I64u", events[i].regional.regional_basis_hash),
                   MST_RegionName(events[i].regional.price_region),
                   DoubleToString(events[i].regional.median_price, _Digits),
                   DoubleToString(events[i].regional.structural_sigma, _Digits),
                   DoubleToString(events[i].regional.price_from_median_atr, 8),
                   DoubleToString(events[i].regional.price_from_median_sigma, 8),
                   DoubleToString(events[i].regional.price_from_cog_atr, 8),
                   DoubleToString(events[i].regional.price_from_cog_sigma, 8),
                   DoubleToString(events[i].regional.cog_median_gap_sigma, 8));
      }

      count = MathMin(attempt_count, ArraySize(attempts));
      for(int i = 0; i < count; i++)
      {
         FileWrite(m_attempts_handle, IntegerToString(MST_SCHEMA_VERSION), m_session_id,
                   StringFormat("%I64u", attempts[i].attempt_id),
                   StringFormat("%I64u", attempts[i].episode_id),
                   StringFormat("%I64u", attempts[i].node_id),
                   StringFormat("%I64u", attempts[i].evidence_id),
                   IntegerToString(attempts[i].attempt_ordinal),
                   IntegerToString(attempts[i].is_retest ? 1 : 0),
                   TimeToString(attempts[i].started_at, TIME_DATE|TIME_MINUTES|TIME_SECONDS),
                   TimeToString(attempts[i].contact_at, TIME_DATE|TIME_MINUTES|TIME_SECONDS),
                   TimeToString(attempts[i].break_at, TIME_DATE|TIME_MINUTES|TIME_SECONDS),
                   TimeToString(attempts[i].accepted_at, TIME_DATE|TIME_MINUTES|TIME_SECONDS),
                   TimeToString(attempts[i].resolved_at, TIME_DATE|TIME_MINUTES|TIME_SECONDS),
                   IntegerToString((int)attempts[i].direction),
                   MST_AuctionResolutionName(attempts[i].resolution),
                   IntegerToString(attempts[i].start_bar_sequence),
                   IntegerToString(attempts[i].contact_bar_sequence),
                   IntegerToString(attempts[i].resolved_bar_sequence),
                   DoubleToString(attempts[i].start_price, _Digits),
                   DoubleToString(attempts[i].contact_price, _Digits),
                   DoubleToString(attempts[i].frozen_atr, _Digits),
                   DoubleToString(attempts[i].frozen_lower, _Digits),
                   DoubleToString(attempts[i].frozen_price, _Digits),
                   DoubleToString(attempts[i].frozen_upper, _Digits),
                   DoubleToString(attempts[i].node_width_atr, 8),
                   DoubleToString(attempts[i].initial_distance_atr, 8),
                   DoubleToString(attempts[i].approach_efficiency, 8),
                   DoubleToString(attempts[i].max_penetration_atr, 8),
                   DoubleToString(attempts[i].max_penetration_node, 8),
                   IntegerToString(attempts[i].inside_updates),
                   StringFormat("%I64d", attempts[i].inside_seconds),
                   IntegerToString(attempts[i].qualified_far_closes),
                   DoubleToString(attempts[i].max_above_node_atr, 8),
                   DoubleToString(attempts[i].max_below_node_atr, 8),
                   DoubleToString(attempts[i].rejection_excursion_atr, 8),
                   StringFormat("%I64u", attempts[i].nearest_above_id),
                   StringFormat("%I64u", attempts[i].nearest_below_id),
                   DoubleToString(attempts[i].corridor_up_atr, 8),
                   DoubleToString(attempts[i].corridor_down_atr, 8),
                   StringFormat("%I64u", attempts[i].family_mask),
                   IntegerToString(attempts[i].family_count),
                   IntegerToString(attempts[i].member_count),
                   IntegerToString(attempts[i].developing_count),
                   IntegerToString(attempts[i].frozen_count),
                   IntegerToString(attempts[i].node_revision),
                   MST_CompletionStatusName(attempts[i].completion_status),
                   MST_CensorReasonName(attempts[i].censor_reason),
                   IntegerToString(attempts[i].corridor_up_level_count),
                   IntegerToString(attempts[i].corridor_down_level_count),
                   IntegerToString(attempts[i].corridor_up_noise_count),
                   IntegerToString(attempts[i].corridor_down_noise_count),
                   MST_RegionName(attempts[i].start_region),
                   MST_RegionName(attempts[i].end_region),
                   MST_RegionName(attempts[i].node_region),
                   DoubleToString(attempts[i].start_median_sigma, 8),
                   DoubleToString(attempts[i].end_median_sigma, 8),
                   DoubleToString(attempts[i].min_median_sigma, 8),
                   DoubleToString(attempts[i].max_median_sigma, 8),
                   DoubleToString(attempts[i].node_from_median_sigma, 8),
                   DoubleToString(attempts[i].node_from_cog_sigma, 8),
                   DoubleToString(attempts[i].node_width_sigma, 8));
         const MST_AuctionFeatureSnapshot snapshot = attempts[i].context;
         FileWrite(m_features_handle, IntegerToString(MST_SCHEMA_VERSION), m_session_id,
                   StringFormat("%I64u", attempts[i].attempt_id),
                   TimeToString(snapshot.frozen_bar_time, TIME_DATE|TIME_MINUTES|TIME_SECONDS),
                   IntegerToString(snapshot.has_cog ? 1 : 0), DoubleToString(snapshot.cog_price, _Digits),
                   DoubleToString(snapshot.cog_distance_atr, 8), DoubleToString(snapshot.cog_velocity_atr_per_bar, 8),
                   IntegerToString(snapshot.has_c3 ? 1 : 0), DoubleToString(snapshot.c3_price, _Digits),
                   DoubleToString(snapshot.c3_distance_atr, 8), DoubleToString(snapshot.c3_velocity_atr_per_bar, 8),
                   IntegerToString(snapshot.has_lattice ? 1 : 0), DoubleToString(snapshot.lattice_width_atr, 8),
                   IntegerToString(snapshot.has_field ? 1 : 0), DoubleToString(snapshot.field_width_atr, 8),
                   IntegerToString(snapshot.has_profile ? 1 : 0), DoubleToString(snapshot.profile_poc, _Digits),
                   DoubleToString(snapshot.profile_vah, _Digits), DoubleToString(snapshot.profile_val, _Digits),
                   DoubleToString(snapshot.poc_distance_atr, 8), DoubleToString(snapshot.vah_distance_atr, 8),
                   DoubleToString(snapshot.val_distance_atr, 8), DoubleToString(snapshot.spread_atr, 8),
                   IntegerToString(snapshot.raw_level_count), IntegerToString(snapshot.noise_level_count),
                   IntegerToString(snapshot.active_node_count),
                   IntegerToString(snapshot.regional.valid ? 1 : 0),
                   IntegerToString(snapshot.regional.sigma_valid ? 1 : 0),
                   IntegerToString(snapshot.regional.basis_changed ? 1 : 0),
                   StringFormat("%I64u", snapshot.regional.structure_snapshot_hash),
                   StringFormat("%I64u", snapshot.regional.regional_basis_hash),
                   StringFormat("%I64u", snapshot.regional.structure_generation),
                   IntegerToString(snapshot.regional.population_count),
                   IntegerToString(snapshot.regional.population_count_delta),
                   IntegerToString(snapshot.regional.velocity_elapsed_bars),
                   DoubleToString(snapshot.regional.median_price, _Digits),
                   DoubleToString(snapshot.regional.mean_price, _Digits),
                   DoubleToString(snapshot.regional.structural_sigma, _Digits),
                   DoubleToString(snapshot.regional.reference_price, _Digits),
                   MST_RegionName(snapshot.regional.price_region),
                   DoubleToString(snapshot.regional.price_from_median_atr, 8),
                   DoubleToString(snapshot.regional.price_from_median_sigma, 8),
                   DoubleToString(snapshot.regional.price_from_cog_atr, 8),
                   DoubleToString(snapshot.regional.price_from_cog_sigma, 8),
                   MST_RegionName(snapshot.regional.cog_region),
                   DoubleToString(snapshot.regional.cog_median_gap_atr, 8),
                   DoubleToString(snapshot.regional.cog_median_gap_sigma, 8),
                   DoubleToString(snapshot.regional.cog_velocity_price_per_bar, _Digits),
                   DoubleToString(snapshot.regional.cog_velocity_atr_per_bar, 8),
                   DoubleToString(snapshot.regional.cog_velocity_sigma_per_bar, 8),
                   DoubleToString(snapshot.regional.median_velocity_price_per_bar, _Digits),
                   DoubleToString(snapshot.regional.median_velocity_atr_per_bar, 8),
                   DoubleToString(snapshot.regional.median_velocity_sigma_per_bar, 8),
                   DoubleToString(snapshot.regional.sigma_log_change_per_bar, 8));
      }

      count = MathMin(episode_count, ArraySize(episodes));
      for(int i = 0; i < count; i++)
      {
         FileWrite(m_episodes_handle, IntegerToString(MST_SCHEMA_VERSION), m_session_id,
                   StringFormat("%I64u", episodes[i].episode_id),
                   StringFormat("%I64u", episodes[i].node_id),
                   TimeToString(episodes[i].started_at, TIME_DATE|TIME_MINUTES|TIME_SECONDS),
                   TimeToString(episodes[i].ended_at, TIME_DATE|TIME_MINUTES|TIME_SECONDS),
                   IntegerToString((int)episodes[i].first_direction),
                   IntegerToString(episodes[i].attempts), IntegerToString(episodes[i].breaks),
                   IntegerToString(episodes[i].reclaims), IntegerToString(episodes[i].retests),
                   StringFormat("%I64u", episodes[i].next_node_id),
                   DoubleToString(episodes[i].corridor_up_atr, 8),
                   DoubleToString(episodes[i].corridor_down_atr, 8),
                   DoubleToString(episodes[i].node_width_atr, 8),
                   StringFormat("%I64u", episodes[i].family_mask),
                   IntegerToString(episodes[i].family_count),
                   IntegerToString(episodes[i].member_count),
                   MST_AuctionResolutionName(episodes[i].resolution),
                   DoubleToString(episodes[i].max_up_excursion_atr, 8),
                   DoubleToString(episodes[i].max_down_excursion_atr, 8),
                   IntegerToString(MathMax(episodes[i].last_attempt_end_bar -
                                           episodes[i].start_bar_sequence, 0)),
                   StringFormat("%I64d", (long)MathMax((long)(episodes[i].ended_at -
                                                               episodes[i].started_at), 0)),
                   MST_CompletionStatusName(episodes[i].completion_status),
                   MST_CensorReasonName(episodes[i].censor_reason),
                   MST_RegionName(episodes[i].initial_region),
                   MST_RegionName(episodes[i].terminal_region));
      }

      count = MathMin(context_count, ArraySize(context));
      for(int i = 0; i < count; i++)
      {
         FileWrite(m_context_handle, IntegerToString(MST_SCHEMA_VERSION), m_session_id,
                   StringFormat("%I64u", context[i].attempt_id),
                   StringFormat("%I64u", context[i].episode_id),
                   StringFormat("%I64u", context[i].node_id),
                   TimeToString(context[i].frozen_at, TIME_DATE|TIME_MINUTES|TIME_SECONDS),
                   StringFormat("%I64u", context[i].source.source_key),
                   MST_ProducerName(context[i].source.producer),
                   IntegerToString(context[i].source.producer_instance),
                   StringFormat("%I64u", context[i].source.local_id),
                   MST_FamilyName(context[i].source.family),
                   IntegerToString(context[i].source.source_kind));
      }

      count = MathMin(transit_count, ArraySize(transits));
      for(int i = 0; i < count; i++)
      {
         FileWrite(m_transits_handle, IntegerToString(MST_SCHEMA_VERSION), m_session_id,
                   StringFormat("%I64u", transits[i].transit_id),
                   StringFormat("%I64u", transits[i].attempt_id),
                   StringFormat("%I64u", transits[i].episode_id),
                   StringFormat("%I64u", transits[i].source_node_id),
                   StringFormat("%I64u", transits[i].destination_node_id),
                   IntegerToString(transits[i].direction),
                   TimeToString(transits[i].started_at, TIME_DATE|TIME_MINUTES|TIME_SECONDS),
                   TimeToString(transits[i].ended_at, TIME_DATE|TIME_MINUTES|TIME_SECONDS),
                   IntegerToString(transits[i].start_bar_sequence),
                   IntegerToString(transits[i].end_bar_sequence),
                   DoubleToString(transits[i].start_price, _Digits),
                   DoubleToString(transits[i].end_price, _Digits),
                   DoubleToString(transits[i].source_lower, _Digits),
                   DoubleToString(transits[i].source_upper, _Digits),
                   DoubleToString(transits[i].destination_lower, _Digits),
                   DoubleToString(transits[i].destination_upper, _Digits),
                   DoubleToString(transits[i].frozen_atr, _Digits),
                   DoubleToString(transits[i].distance_atr, 8),
                   DoubleToString(transits[i].path_length, _Digits),
                   DoubleToString(transits[i].path_efficiency, 8),
                   DoubleToString(transits[i].max_adverse_atr, 8),
                   MST_CompletionStatusName(transits[i].completion_status),
                   MST_CensorReasonName(transits[i].censor_reason),
                   MST_AuctionResolutionName(transits[i].resolution),
                   IntegerToString(MathMax(transits[i].end_bar_sequence -
                                           transits[i].start_bar_sequence, 0)),
                   StringFormat("%I64d", (long)MathMax((long)(transits[i].ended_at -
                                                               transits[i].started_at), 0)),
                   StringFormat("%I64u", transits[i].regional_basis_hash),
                   StringFormat("%I64u", transits[i].structure_snapshot_hash),
                   MST_RegionName(transits[i].source_region),
                   MST_RegionName(transits[i].destination_region),
                   MST_RegionName(transits[i].start_price_region),
                   MST_RegionName(transits[i].end_price_region),
                   DoubleToString(transits[i].start_median_price, _Digits),
                   DoubleToString(transits[i].start_structural_sigma, _Digits),
                   DoubleToString(transits[i].start_price_from_median_sigma, 8),
                   DoubleToString(transits[i].end_price_from_median_sigma, 8));
      }
      m_event_rows += MathMin(event_count, ArraySize(events));
      m_attempt_rows += MathMin(attempt_count, ArraySize(attempts));
      m_episode_rows += MathMin(episode_count, ArraySize(episodes));
      m_context_rows += MathMin(context_count, ArraySize(context));
      m_feature_rows += MathMin(attempt_count, ArraySize(attempts));
      m_transit_rows += MathMin(transit_count, ArraySize(transits));
      m_pending_rows += event_count + attempt_count * 2 + episode_count + context_count + transit_count;
      MaybeFlush();
      return true;
   }

   bool WriteRun(const string record_type, const MST_AuctionRunManifest &run)
   {
      if(!m_enabled) return true;
      if(m_runs_handle == INVALID_HANDLE) return false;
      const MST_AuctionInvariantReading inv = run.invariants;
      FileWrite(m_runs_handle, IntegerToString(MST_SCHEMA_VERSION), record_type,
                run.session_id, run.build_id, run.symbol, EnumToString(run.timeframe),
                IntegerToString(run.digits), DoubleToString(run.point, run.digits),
                DoubleToString(run.tick_size, run.digits), run.account_server,
                run.account_company, IntegerToString(run.tester ? 1 : 0),
                IntegerToString(run.visual_mode ? 1 : 0),
                TimeToString(run.started_at, TIME_DATE|TIME_MINUTES|TIME_SECONDS),
                TimeToString(run.ended_at, TIME_DATE|TIME_MINUTES|TIME_SECONDS),
                IntegerToString(run.terminal_reason), StringFormat("%I64u", run.config_hash),
                run.config_text, StringFormat("%I64u", run.terminal_hash),
                StringFormat("%I64u", run.event_sequence),
                StringFormat("%I64d", inv.attempts_started),
                StringFormat("%I64d", inv.attempts_resolved),
                StringFormat("%I64d", inv.attempts_censored),
                StringFormat("%I64d", inv.episodes_started),
                StringFormat("%I64d", inv.episodes_resolved),
                StringFormat("%I64d", inv.episodes_censored),
                StringFormat("%I64d", inv.transits_started),
                StringFormat("%I64d", inv.transits_resolved),
                StringFormat("%I64d", inv.transits_censored),
                StringFormat("%I64d", inv.events_emitted),
                IntegerToString(inv.active_attempts), IntegerToString(inv.active_episodes),
                IntegerToString(inv.active_transits), IntegerToString(inv.balanced ? 1 : 0),
                StringFormat("%I64u", inv.receipt_hash),
                StringFormat("%I64d", m_event_rows), StringFormat("%I64d", m_attempt_rows),
                StringFormat("%I64d", m_episode_rows), StringFormat("%I64d", m_context_rows),
                StringFormat("%I64d", m_feature_rows), StringFormat("%I64d", m_transit_rows));
      FileFlush(m_runs_handle);
      return true;
   }

   string SessionId(void) const { return m_session_id; }
   long EventRows(void) const { return m_event_rows; }
   long AttemptRows(void) const { return m_attempt_rows; }
   long EpisodeRows(void) const { return m_episode_rows; }
   long ContextRows(void) const { return m_context_rows; }
   long FeatureRows(void) const { return m_feature_rows; }
   long TransitRows(void) const { return m_transit_rows; }

   void Flush(void)
   {
      if(!m_enabled) return;
      FileFlush(m_events_handle);
      FileFlush(m_attempts_handle);
      FileFlush(m_episodes_handle);
      FileFlush(m_context_handle);
      FileFlush(m_features_handle);
      FileFlush(m_transits_handle);
      FileFlush(m_runs_handle);
      m_pending_rows = 0;
   }

   void Close(void)
   {
      int handles[7] = {m_events_handle, m_attempts_handle, m_episodes_handle,
                        m_context_handle, m_features_handle, m_transits_handle,
                        m_runs_handle};
      for(int i = 0; i < 7; i++)
      {
         if(handles[i] == INVALID_HANDLE) continue;
         FileFlush(handles[i]);
         FileClose(handles[i]);
      }
      m_events_handle = INVALID_HANDLE;
      m_attempts_handle = INVALID_HANDLE;
      m_episodes_handle = INVALID_HANDLE;
      m_context_handle = INVALID_HANDLE;
      m_features_handle = INVALID_HANDLE;
      m_transits_handle = INVALID_HANDLE;
      m_runs_handle = INVALID_HANDLE;
      m_pending_rows = 0;
   }
};

#endif // __KITT_MASTER_AUCTION_LOGGER_MQH__
