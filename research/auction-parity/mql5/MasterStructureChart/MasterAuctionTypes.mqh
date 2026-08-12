//+------------------------------------------------------------------+
//| MasterAuctionTypes.mqh                                           |
//| Deterministic interaction, attempt, episode, and transit records. |
//+------------------------------------------------------------------+
#ifndef __KITT_MASTER_AUCTION_TYPES_MQH__
#define __KITT_MASTER_AUCTION_TYPES_MQH__

#include "MasterTypes.mqh"

enum MST_APPROACH_DIRECTION
{
   MST_APPROACH_NONE = 0,
   MST_APPROACH_FROM_BELOW = -1,
   MST_APPROACH_FROM_ABOVE = 1
};

enum MST_AUCTION_STATE
{
   MST_AUCTION_IDLE = 0,
   MST_AUCTION_APPROACH,
   MST_AUCTION_CONTACT,
   MST_AUCTION_PENETRATION,
   MST_AUCTION_BROKEN,
   MST_AUCTION_PROVISIONAL_ACCEPTANCE,
   MST_AUCTION_ACCEPTED,
   MST_AUCTION_RETEST,
   MST_AUCTION_RESOLVED
};

enum MST_AUCTION_EVENT_KIND
{
   MST_AUCTION_EVENT_NONE = 0,
   MST_AUCTION_EVENT_APPROACH,
   MST_AUCTION_EVENT_CONTACT,
   MST_AUCTION_EVENT_PENETRATION,
   MST_AUCTION_EVENT_BREAK,
   MST_AUCTION_EVENT_PROVISIONAL_ACCEPTANCE,
   MST_AUCTION_EVENT_ACCEPTANCE,
   MST_AUCTION_EVENT_REJECTION,
   MST_AUCTION_EVENT_RECLAIM,
   MST_AUCTION_EVENT_RETEST,
   MST_AUCTION_EVENT_HOLD,
   MST_AUCTION_EVENT_RETEST_FAILURE,
   MST_AUCTION_EVENT_DEPARTURE,
   MST_AUCTION_EVENT_TRANSIT,
   MST_AUCTION_EVENT_RETURN_TO_SOURCE,
   MST_AUCTION_EVENT_CENSOR,
   MST_AUCTION_EVENT_EXPIRE
};

enum MST_COMPLETION_STATUS
{
   MST_COMPLETION_ACTIVE = 0,
   MST_COMPLETION_RESOLVED,
   MST_COMPLETION_RIGHT_CENSORED
};

enum MST_CENSOR_REASON
{
   MST_CENSOR_NONE = 0,
   MST_CENSOR_TEST_END,
   MST_CENSOR_SHUTDOWN,
   MST_CENSOR_DATA_GAP
};

enum MST_AUCTION_RESOLUTION
{
   MST_RESOLUTION_NONE = 0,
   MST_RESOLUTION_REJECT_TO_ORIGIN,
   MST_RESOLUTION_ACCEPT_THROUGH_NODE,
   MST_RESOLUTION_RECLAIM_AFTER_BREAK,
   MST_RESOLUTION_ACCEPT_AND_HOLD_RETEST,
   MST_RESOLUTION_ACCEPT_AND_FAIL_RETEST,
   MST_RESOLUTION_TRANSIT_TO_NEXT_NODE,
   MST_RESOLUTION_RETURN_TO_SOURCE_NODE,
   MST_RESOLUTION_TIMEOUT,
   MST_RESOLUTION_NODE_RETIRED
};

struct MST_AuctionGrammarConfig
{
   bool   enabled;
   double approach_radius_atr;
   double rejection_min_excursion_atr;
   double break_buffer_atr;
   int    acceptance_bars;
   double acceptance_min_distance_atr;
   double reclaim_tolerance_atr;
   double departure_distance_atr;
   int    max_attempt_bars;
   int    episode_gap_bars;
};

struct MST_AuctionEvent
{
   ulong                  event_id;
   ulong                  event_sequence;
   ulong                  episode_id;
   ulong                  attempt_id;
   ulong                  node_id;
   ulong                  related_node_id;
   datetime               market_time;
   datetime               bar_time;
   MST_AUCTION_EVENT_KIND kind;
   MST_APPROACH_DIRECTION direction;
   double                 price;
   double                 atr;
   double                 distance_atr;
   double                 penetration_atr;
   int                    bar_sequence;
   int                    attempt_event_sequence;
   MST_RegionalSnapshot   regional;
};

struct MST_AuctionFeatureSnapshot
{
   datetime frozen_bar_time;
   bool     has_cog;
   bool     has_c3;
   bool     has_lattice;
   bool     has_field;
   bool     has_profile;
   double   cog_price;
   double   c3_price;
   double   cog_distance_atr;
   double   c3_distance_atr;
   double   cog_velocity_atr_per_bar;
   double   c3_velocity_atr_per_bar;
   double   lattice_width_atr;
   double   field_width_atr;
   double   profile_poc;
   double   profile_vah;
   double   profile_val;
   double   poc_distance_atr;
   double   vah_distance_atr;
   double   val_distance_atr;
   double   spread_atr;
   int      raw_level_count;
   int      noise_level_count;
   int      active_node_count;
   MST_RegionalSnapshot regional;
};

struct MST_AuctionAttempt
{
   bool                   active;
   bool                   is_retest;
   MST_COMPLETION_STATUS  completion_status;
   MST_CENSOR_REASON      censor_reason;
   ulong                  attempt_id;
   ulong                  episode_id;
   ulong                  node_id;
   ulong                  evidence_id;
   int                    attempt_ordinal;
   MST_APPROACH_DIRECTION direction;
   MST_AUCTION_STATE      state;
   MST_AUCTION_RESOLUTION resolution;

   datetime               started_at;
   datetime               contact_at;
   datetime               break_at;
   datetime               accepted_at;
   datetime               resolved_at;
   datetime               last_observed_at;
   datetime               last_closed_bar_time;
   int                    start_bar_sequence;
   int                    contact_bar_sequence;
   int                    resolved_bar_sequence;
   int                    event_count;

   double                 frozen_lower;
   double                 frozen_price;
   double                 frozen_upper;
   double                 frozen_width;
   double                 frozen_atr;
   double                 start_price;
   double                 contact_price;
   double                 last_price;
   double                 approach_path;
   double                 approach_efficiency;
   double                 max_penetration;
   double                 max_penetration_atr;
   double                 max_penetration_node;
   double                 max_above_node_atr;
   double                 max_below_node_atr;
   double                 rejection_excursion_atr;
   int                    inside_updates;
   long                   inside_seconds;
   int                    qualified_far_closes;
   bool                   broke_far_boundary;
   bool                   provisional_acceptance;
   bool                   accepted;
   bool                   retest_contacted;

   ulong                  family_mask;
   int                    family_count;
   int                    member_count;
   int                    developing_count;
   int                    frozen_count;
   int                    node_revision;
   int                    provenance_offset;
   int                    provenance_count;
   double                 node_width_atr;
   double                 initial_distance_atr;
   ulong                  nearest_above_id;
   ulong                  nearest_below_id;
   double                 corridor_up_atr;
   double                 corridor_down_atr;
   int                    corridor_up_level_count;
   int                    corridor_down_level_count;
   int                    corridor_up_noise_count;
   int                    corridor_down_noise_count;
   MST_STRUCTURAL_REGION  start_region;
   MST_STRUCTURAL_REGION  end_region;
   MST_STRUCTURAL_REGION  node_region;
   double                 start_median_sigma;
   double                 end_median_sigma;
   double                 min_median_sigma;
   double                 max_median_sigma;
   double                 node_from_median_sigma;
   double                 node_from_cog_sigma;
   double                 node_width_sigma;
   MST_AuctionFeatureSnapshot context;
};

struct MST_AuctionEpisode
{
   bool                   active;
   MST_COMPLETION_STATUS  completion_status;
   MST_CENSOR_REASON      censor_reason;
   ulong                  episode_id;
   ulong                  node_id;
   datetime               started_at;
   datetime               ended_at;
   int                    start_bar_sequence;
   int                    last_attempt_end_bar;
   MST_APPROACH_DIRECTION first_direction;
   int                    attempts;
   int                    breaks;
   int                    reclaims;
   int                    retests;
   ulong                  next_node_id;
   MST_AUCTION_RESOLUTION resolution;
   double                 max_up_excursion_atr;
   double                 max_down_excursion_atr;
   double                 corridor_up_atr;
   double                 corridor_down_atr;
   double                 node_width_atr;
   ulong                  family_mask;
   int                    family_count;
   int                    member_count;
   MST_STRUCTURAL_REGION  initial_region;
   MST_STRUCTURAL_REGION  terminal_region;
};

struct MST_AuctionContextReceipt
{
   ulong                  attempt_id;
   ulong                  episode_id;
   ulong                  node_id;
   datetime               frozen_at;
   MST_SourceRef          source;
};

struct MST_TransitTrack
{
   bool                   active;
   ulong                  transit_id;
   ulong                  attempt_id;
   ulong                  episode_id;
   ulong                  source_node_id;
   ulong                  destination_node_id;
   int                    direction;
   datetime               started_at;
   int                    start_bar_sequence;
   double                 start_price;
   double                 source_lower;
   double                 source_upper;
   double                 destination_lower;
   double                 destination_upper;
   double                 frozen_atr;
   double                 distance_atr;
   double                 path_length;
   double                 last_price;
   double                 max_adverse_atr;
   datetime               ended_at;
   int                    end_bar_sequence;
   double                 end_price;
   double                 path_efficiency;
   ulong                  regional_basis_hash;
   ulong                  structure_snapshot_hash;
   MST_STRUCTURAL_REGION  source_region;
   MST_STRUCTURAL_REGION  destination_region;
   MST_STRUCTURAL_REGION  start_price_region;
   MST_STRUCTURAL_REGION  end_price_region;
   double                 start_median_price;
   double                 start_structural_sigma;
   double                 start_price_from_median_sigma;
   double                 end_price_from_median_sigma;
   MST_COMPLETION_STATUS  completion_status;
   MST_CENSOR_REASON      censor_reason;
   MST_AUCTION_RESOLUTION resolution;
};

struct MST_AuctionInvariantReading
{
   long  attempts_started;
   long  attempts_resolved;
   long  attempts_censored;
   long  episodes_started;
   long  episodes_resolved;
   long  episodes_censored;
   long  transits_started;
   long  transits_resolved;
   long  transits_censored;
   long  events_emitted;
   int   active_attempts;
   int   active_episodes;
   int   active_transits;
   bool  balanced;
   ulong receipt_hash;
};

struct MST_AuctionRunManifest
{
   string          build_id;
   string          session_id;
   string          symbol;
   string          account_server;
   string          account_company;
   string          config_text;
   ulong           config_hash;
   ENUM_TIMEFRAMES timeframe;
   int             digits;
   double          point;
   double          tick_size;
   bool            tester;
   bool            visual_mode;
   datetime        started_at;
   datetime        ended_at;
   int             terminal_reason;
   ulong           terminal_hash;
   ulong           event_sequence;
   MST_AuctionInvariantReading invariants;
};

struct MST_AuctionReading
{
   int                    active_attempts;
   int                    active_episodes;
   int                    active_transits;
   int                    event_count;
   int                    completed_attempts;
   int                    completed_episodes;
   ulong                  event_sequence;
   ulong                  terminal_hash;
};

void MST_DefaultAuctionGrammarConfig(MST_AuctionGrammarConfig &cfg)
{
   ZeroMemory(cfg);
   cfg.enabled = true;
   cfg.approach_radius_atr = 0.50;
   cfg.rejection_min_excursion_atr = 0.20;
   cfg.break_buffer_atr = 0.00;
   cfg.acceptance_bars = 2;
   cfg.acceptance_min_distance_atr = 0.00;
   cfg.reclaim_tolerance_atr = 0.05;
   cfg.departure_distance_atr = 0.25;
   cfg.max_attempt_bars = 24;
   cfg.episode_gap_bars = 12;
}

string MST_AuctionStateName(const MST_AUCTION_STATE state)
{
   if(state == MST_AUCTION_APPROACH) return "APPROACH";
   if(state == MST_AUCTION_CONTACT) return "CONTACT";
   if(state == MST_AUCTION_PENETRATION) return "PENETRATION";
   if(state == MST_AUCTION_BROKEN) return "BROKEN";
   if(state == MST_AUCTION_PROVISIONAL_ACCEPTANCE) return "PROVISIONAL_ACCEPTANCE";
   if(state == MST_AUCTION_ACCEPTED) return "ACCEPTED";
   if(state == MST_AUCTION_RETEST) return "RETEST";
   if(state == MST_AUCTION_RESOLVED) return "RESOLVED";
   return "IDLE";
}

string MST_AuctionEventName(const MST_AUCTION_EVENT_KIND kind)
{
   if(kind == MST_AUCTION_EVENT_APPROACH) return "APPROACH";
   if(kind == MST_AUCTION_EVENT_CONTACT) return "CONTACT";
   if(kind == MST_AUCTION_EVENT_PENETRATION) return "PENETRATION";
   if(kind == MST_AUCTION_EVENT_BREAK) return "BREAK";
   if(kind == MST_AUCTION_EVENT_PROVISIONAL_ACCEPTANCE) return "PROVISIONAL_ACCEPTANCE";
   if(kind == MST_AUCTION_EVENT_ACCEPTANCE) return "ACCEPTANCE";
   if(kind == MST_AUCTION_EVENT_REJECTION) return "REJECTION";
   if(kind == MST_AUCTION_EVENT_RECLAIM) return "RECLAIM";
   if(kind == MST_AUCTION_EVENT_RETEST) return "RETEST";
   if(kind == MST_AUCTION_EVENT_HOLD) return "HOLD";
   if(kind == MST_AUCTION_EVENT_RETEST_FAILURE) return "RETEST_FAILURE";
   if(kind == MST_AUCTION_EVENT_DEPARTURE) return "DEPARTURE";
   if(kind == MST_AUCTION_EVENT_TRANSIT) return "TRANSIT";
   if(kind == MST_AUCTION_EVENT_RETURN_TO_SOURCE) return "RETURN_TO_SOURCE";
   if(kind == MST_AUCTION_EVENT_CENSOR) return "CENSOR";
   if(kind == MST_AUCTION_EVENT_EXPIRE) return "EXPIRE";
   return "NONE";
}

string MST_CompletionStatusName(const MST_COMPLETION_STATUS status)
{
   if(status == MST_COMPLETION_RESOLVED) return "RESOLVED";
   if(status == MST_COMPLETION_RIGHT_CENSORED) return "RIGHT_CENSORED";
   return "ACTIVE";
}

string MST_CensorReasonName(const MST_CENSOR_REASON reason)
{
   if(reason == MST_CENSOR_TEST_END) return "TEST_END";
   if(reason == MST_CENSOR_SHUTDOWN) return "SHUTDOWN";
   if(reason == MST_CENSOR_DATA_GAP) return "DATA_GAP";
   return "NONE";
}

string MST_AuctionResolutionName(const MST_AUCTION_RESOLUTION resolution)
{
   if(resolution == MST_RESOLUTION_REJECT_TO_ORIGIN) return "REJECT_TO_ORIGIN";
   if(resolution == MST_RESOLUTION_ACCEPT_THROUGH_NODE) return "ACCEPT_THROUGH_NODE";
   if(resolution == MST_RESOLUTION_RECLAIM_AFTER_BREAK) return "RECLAIM_AFTER_BREAK";
   if(resolution == MST_RESOLUTION_ACCEPT_AND_HOLD_RETEST) return "ACCEPT_AND_HOLD_RETEST";
   if(resolution == MST_RESOLUTION_ACCEPT_AND_FAIL_RETEST) return "ACCEPT_AND_FAIL_RETEST";
   if(resolution == MST_RESOLUTION_TRANSIT_TO_NEXT_NODE) return "TRANSIT_TO_NEXT_NODE";
   if(resolution == MST_RESOLUTION_RETURN_TO_SOURCE_NODE) return "RETURN_TO_SOURCE_NODE";
   if(resolution == MST_RESOLUTION_TIMEOUT) return "TIMEOUT";
   if(resolution == MST_RESOLUTION_NODE_RETIRED) return "NODE_RETIRED";
   return "NONE";
}

#endif // __KITT_MASTER_AUCTION_TYPES_MQH__
