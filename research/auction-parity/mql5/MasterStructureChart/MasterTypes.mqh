//+------------------------------------------------------------------+
//| MasterTypes.mqh                                                  |
//| Canonical contracts for the master structural controller.        |
//+------------------------------------------------------------------+
#ifndef __KITT_MASTER_STRUCTURE_TYPES_MQH__
#define __KITT_MASTER_STRUCTURE_TYPES_MQH__

#define MST_SCHEMA_VERSION       5
#define MST_BUILD_ID             "MASTER_STRUCTURE_REGIONAL_V5"
#define MST_MAX_FAMILIES        16
#define MST_CLUSTER_UNCLASSIFIED -2
#define MST_CLUSTER_NOISE        -1

enum MST_PRODUCER_KIND
{
   MST_PRODUCER_NONE       = 0,
   MST_PRODUCER_VOLKITT    = 1,
   MST_PRODUCER_DAY_SWINGS = 2,
   MST_PRODUCER_WAYNE      = 3,
   MST_PRODUCER_MARKET_STRUCTURE = 4
};

enum MST_FAMILY
{
   MST_FAMILY_NONE = 0,
   MST_FAMILY_ADAPTIVE_VALUE,
   MST_FAMILY_GLOBAL_VALUE,
   MST_FAMILY_SENTINEL,
   MST_FAMILY_TPO_PROFILE,
   MST_FAMILY_SINGLE_PRINT,
   MST_FAMILY_DAILY_EXTREME,
   MST_FAMILY_PIVOT,
   MST_FAMILY_MID_PIVOT,
   MST_FAMILY_PIVOT_ZONE,
   MST_FAMILY_MARKET_STRUCTURE,
   MST_FAMILY_COUNT
};

enum MST_ROLE
{
   MST_ROLE_NONE = 0,
   MST_ROLE_LOWER_EXTREME,
   MST_ROLE_LOWER_MAJOR,
   MST_ROLE_LOWER_INTERMEDIATE,
   MST_ROLE_LOWER_BOUNDARY,
   MST_ROLE_LOWER_MEMORY,
   MST_ROLE_CENTER,
   MST_ROLE_FAIR_VALUE,
   MST_ROLE_UPPER_INTERMEDIATE,
   MST_ROLE_UPPER_MAJOR,
   MST_ROLE_UPPER_EXTREME,
   MST_ROLE_UPPER_BOUNDARY,
   MST_ROLE_UPPER_MEMORY,
   MST_ROLE_COUNT
};

enum MST_LIFECYCLE_STATE
{
   MST_STATE_UNKNOWN = 0,
   MST_STATE_FRESH,
   MST_STATE_TOUCHED,
   MST_STATE_TESTED,
   MST_STATE_ACCEPTED,
   MST_STATE_REJECTED,
   MST_STATE_BROKEN,
   MST_STATE_RECLAIMED
};

enum MST_NODE_ROLE
{
   MST_NODE_LOWER = -1,
   MST_NODE_CENTER = 0,
   MST_NODE_UPPER = 1
};

enum MST_STRUCTURAL_REGION
{
   MST_REGION_EXTREME_BELOW = -3,
   MST_REGION_FAR_BELOW     = -2,
   MST_REGION_BELOW         = -1,
   MST_REGION_MEDIAN_CORE   =  0,
   MST_REGION_ABOVE         =  1,
   MST_REGION_FAR_ABOVE     =  2,
   MST_REGION_EXTREME_ABOVE =  3,
   MST_REGION_UNAVAILABLE   = 99
};

struct MST_RegionalSnapshot
{
   bool                  valid;
   bool                  sigma_valid;
   bool                  has_cog;
   bool                  basis_changed;
   datetime              frozen_bar_time;
   ulong                 structure_snapshot_hash;
   ulong                 regional_basis_hash;
   ulong                 structure_generation;
   int                   population_count;
   int                   population_count_delta;
   int                   velocity_elapsed_bars;
   double                median_price;
   double                mean_price;
   double                structural_sigma;
   double                reference_price;
   double                cog_price;
   double                price_from_median_atr;
   double                price_from_median_sigma;
   double                price_from_cog_atr;
   double                price_from_cog_sigma;
   double                cog_median_gap_atr;
   double                cog_median_gap_sigma;
   double                cog_velocity_price_per_bar;
   double                cog_velocity_atr_per_bar;
   double                cog_velocity_sigma_per_bar;
   double                median_velocity_price_per_bar;
   double                median_velocity_atr_per_bar;
   double                median_velocity_sigma_per_bar;
   double                sigma_log_change_per_bar;
   MST_STRUCTURAL_REGION price_region;
   MST_STRUCTURAL_REGION cog_region;
};

enum MST_NODE_EXISTENCE_STATE
{
   MST_NODE_NEW = 0,
   MST_NODE_ACTIVE,
   MST_NODE_SUPERSEDED,
   MST_NODE_RETIRED
};

enum MST_NODE_EVENT_KIND
{
   MST_EVENT_NONE = 0,
   MST_EVENT_CREATED,
   MST_EVENT_ACTIVATED,
   MST_EVENT_RETIRE,
   MST_EVENT_MERGE,
   MST_EVENT_SPLIT,
   MST_EVENT_EVIDENCE_CHANGED
};

struct MST_Level
{
   bool                valid;
   ulong               source_key;
   MST_PRODUCER_KIND   producer;
   int                 producer_instance;
   ulong               local_id;
   MST_FAMILY          family;
   int                 source_kind;
   MST_ROLE            role;

   double              lower;
   double              price;
   double              upper;
   double              width;

   double              normalized_lower;
   double              normalized_price;
   double              normalized_upper;
   double              width_atr;
   double              distance_atr;

   ENUM_TIMEFRAMES     timeframe;
   datetime            created_at;
   datetime            updated_at;
   datetime            effective_start;
   datetime            effective_end;

   bool                developing;
   bool                frozen_geometry;
   MST_LIFECYCLE_STATE state;

   int                 touches;
   int                 rejections;
   int                 reclaims;
   int                 acceptance_bars;

   double              mass;
   double              mass_share;
   double              evidence_weight;
   double              max_excursion_atr;
};

struct MST_SourceRef
{
   ulong               source_key;
   MST_PRODUCER_KIND   producer;
   int                 producer_instance;
   ulong               local_id;
   MST_FAMILY          family;
   int                 source_kind;
};

struct MST_Node
{
   bool                valid;
   ulong               node_id;
   ulong               evidence_id;
   int                 cluster_id;

   double              lower;
   double              price;
   double              upper;
   double              width;
   double              normalized_lower;
   double              normalized_price;
   double              normalized_upper;
   double              width_atr;
   double              distance_atr;

   MST_STRUCTURAL_REGION structural_region;
   double              median_distance_atr;
   double              median_distance_sigma;
   double              cog_distance_sigma;
   double              width_sigma;
   bool                contains_cog;

   MST_NODE_ROLE       role;
   ulong               family_mask;
   ulong               role_mask;
   int                 family_count;
   int                 member_count;
   int                 developing_count;
   int                 frozen_count;
   int                 broken_count;
   int                 corridor_up_level_count;
   int                 corridor_down_level_count;
   int                 corridor_up_noise_count;
   int                 corridor_down_noise_count;

   datetime            oldest_source_time;
   datetime            newest_update_time;
   int                 provenance_offset;
   int                 provenance_count;

   MST_NODE_EXISTENCE_STATE existence;
   datetime            created_at;
   datetime            last_seen_at;
   datetime            state_changed_at;
   int                 attempt_count;
   int                 interaction_state;
   int                 missed_rebuilds;
   int                 revision;
   ulong               primary_parent_id;
   ulong               secondary_parent_id;
};

struct MST_NodeLifecycleConfig
{
   double match_distance_atr;
   int    retire_after_rebuilds;
};

struct MST_NodeEvent
{
   MST_NODE_EVENT_KIND kind;
   ulong               node_id;
   ulong               related_node_id;
   ulong               evidence_id;
   datetime            market_time;
   double              reference_price;
   double              lower;
   double              price;
   double              upper;
   MST_NODE_EXISTENCE_STATE previous_state;
   MST_NODE_EXISTENCE_STATE current_state;
   int                 attempt_count;
   int                 revision;
};

struct MST_CompatibilityConfig
{
   ulong family_masks[MST_MAX_FAMILIES];
   bool  require_role_compatibility;
   bool  include_broken;
   bool  include_developing;
};

struct MST_ClusterConfig
{
   double epsilon_atr;
   int    min_samples;
   int    max_levels;
   int    max_nodes;
   bool   use_interval_distance;
   double center_role_tolerance_atr;
};

struct MST_ControllerReading
{
   bool      valid;
   string    symbol;
   ENUM_TIMEFRAMES timeframe;
   datetime  market_time;
   datetime  calculation_bar_time;
   double    reference_price;
   double    atr;
   ulong     generation;
   ulong     render_generation;
   ulong     snapshot_hash;
   int       raw_level_count;
   int       cluster_count;
   int       node_count;
   int       noise_count;
   int       dropped_level_count;
   MST_RegionalSnapshot regional;
   bool      structure_rebuilt;
   int       lifecycle_event_count;
   int       tracked_node_count;
   int       auction_event_count;
   int       active_attempt_count;
   int       active_episode_count;
   int       active_transit_count;
   int       completed_attempt_count;
   int       completed_episode_count;
   ulong     auction_event_sequence;
   ulong     auction_terminal_hash;
   bool      volkitt_heavy_core;
   bool      volkitt_profile_rebuilt;
   ulong     volkitt_microseconds;
   ulong     volkitt_core_microseconds;
   ulong     volkitt_profile_microseconds;
   ulong     day_swings_microseconds;
   ulong     wayne_microseconds;
   ulong     reference_atr_microseconds;
   ulong     collect_microseconds;
   ulong     normalize_microseconds;
   ulong     dbscan_microseconds;
   ulong     node_build_microseconds;
   ulong     lifecycle_microseconds;
   ulong     hash_microseconds;
   ulong     logger_microseconds;
   ulong     update_microseconds;
};

void MST_DefaultNodeLifecycleConfig(MST_NodeLifecycleConfig &cfg)
{
   ZeroMemory(cfg);
   cfg.match_distance_atr = 0.20;
   cfg.retire_after_rebuilds = 3;
}

string MST_NodeStateName(const MST_NODE_EXISTENCE_STATE state)
{
   if(state == MST_NODE_NEW) return "NEW";
   if(state == MST_NODE_ACTIVE) return "ACTIVE";
   if(state == MST_NODE_SUPERSEDED) return "SUPERSEDED";
   if(state == MST_NODE_RETIRED) return "RETIRED";
   return "UNKNOWN";
}

string MST_RegionName(const MST_STRUCTURAL_REGION region)
{
   if(region == MST_REGION_EXTREME_BELOW) return "EXTREME_BELOW";
   if(region == MST_REGION_FAR_BELOW) return "FAR_BELOW";
   if(region == MST_REGION_BELOW) return "BELOW";
   if(region == MST_REGION_MEDIAN_CORE) return "MEDIAN_CORE";
   if(region == MST_REGION_ABOVE) return "ABOVE";
   if(region == MST_REGION_FAR_ABOVE) return "FAR_ABOVE";
   if(region == MST_REGION_EXTREME_ABOVE) return "EXTREME_ABOVE";
   return "UNAVAILABLE";
}

string MST_NodeEventName(const MST_NODE_EVENT_KIND kind)
{
   if(kind == MST_EVENT_CREATED) return "CREATED";
   if(kind == MST_EVENT_ACTIVATED) return "ACTIVATED";
   if(kind == MST_EVENT_RETIRE) return "RETIRE";
   if(kind == MST_EVENT_MERGE) return "MERGE";
   if(kind == MST_EVENT_SPLIT) return "SPLIT";
   if(kind == MST_EVENT_EVIDENCE_CHANGED) return "EVIDENCE_CHANGED";
   return "NONE";
}

bool MST_IsFinitePrice(const double value)
{
   return value != EMPTY_VALUE && MathIsValidNumber(value);
}

ulong MST_FamilyBit(const MST_FAMILY family)
{
   int index = (int)family;
   if(index <= 0 || index >= 63)
      return 0;
   return ((ulong)1 << index);
}

ulong MST_RoleBit(const MST_ROLE role)
{
   int index = (int)role;
   if(index <= 0 || index >= 63)
      return 0;
   return ((ulong)1 << index);
}

int MST_RoleSide(const MST_ROLE role)
{
   if(role == MST_ROLE_LOWER_EXTREME ||
      role == MST_ROLE_LOWER_MAJOR ||
      role == MST_ROLE_LOWER_INTERMEDIATE ||
      role == MST_ROLE_LOWER_BOUNDARY ||
      role == MST_ROLE_LOWER_MEMORY)
      return -1;

   if(role == MST_ROLE_UPPER_INTERMEDIATE ||
      role == MST_ROLE_UPPER_MAJOR ||
      role == MST_ROLE_UPPER_EXTREME ||
      role == MST_ROLE_UPPER_BOUNDARY ||
      role == MST_ROLE_UPPER_MEMORY)
      return 1;

   return 0;
}

bool MST_RolesCompatible(const MST_ROLE left, const MST_ROLE right)
{
   if(left == MST_ROLE_NONE || right == MST_ROLE_NONE)
      return true;

   int left_side = MST_RoleSide(left);
   int right_side = MST_RoleSide(right);
   if(left_side == 0 || right_side == 0)
      return true;
   return left_side == right_side;
}

void MST_DefaultCompatibility(MST_CompatibilityConfig &cfg)
{
   ZeroMemory(cfg);
   ulong all_families = 0;
   for(int i = 1; i < (int)MST_FAMILY_COUNT; i++)
      all_families |= ((ulong)1 << i);
   for(int i = 0; i < MST_MAX_FAMILIES; i++)
      cfg.family_masks[i] = all_families;
   cfg.require_role_compatibility = true;
   cfg.include_broken = false;
   cfg.include_developing = true;
}

void MST_SetFamilyCompatibility(MST_CompatibilityConfig &cfg,
                                const MST_FAMILY left,
                                const MST_FAMILY right,
                                const bool enabled)
{
   int li = (int)left;
   int ri = (int)right;
   if(li <= 0 || ri <= 0 || li >= MST_MAX_FAMILIES || ri >= MST_MAX_FAMILIES)
      return;

   ulong right_bit = ((ulong)1 << ri);
   ulong left_bit = ((ulong)1 << li);
   if(enabled)
   {
      cfg.family_masks[li] |= right_bit;
      cfg.family_masks[ri] |= left_bit;
   }
   else
   {
      cfg.family_masks[li] &= ~right_bit;
      cfg.family_masks[ri] &= ~left_bit;
   }
}

void MST_DefaultClusterConfig(MST_ClusterConfig &cfg)
{
   ZeroMemory(cfg);
   cfg.epsilon_atr = 0.12;
   cfg.min_samples = 2;
   cfg.max_levels = 1024;
   cfg.max_nodes = 256;
   cfg.use_interval_distance = true;
   cfg.center_role_tolerance_atr = 0.15;
}

bool MST_LevelsCompatible(const MST_Level &left,
                          const MST_Level &right,
                          const MST_CompatibilityConfig &cfg)
{
   if(!left.valid || !right.valid)
      return false;
   if(!cfg.include_broken &&
      (left.state == MST_STATE_BROKEN || right.state == MST_STATE_BROKEN))
      return false;
   if(!cfg.include_developing && (left.developing || right.developing))
      return false;

   int li = (int)left.family;
   int ri = (int)right.family;
   if(li <= 0 || ri <= 0 || li >= MST_MAX_FAMILIES || ri >= MST_MAX_FAMILIES)
      return false;
   if((cfg.family_masks[li] & ((ulong)1 << ri)) == 0)
      return false;

   if(cfg.require_role_compatibility && !MST_RolesCompatible(left.role, right.role))
      return false;
   return true;
}

ulong MST_HashMix(ulong hash, const ulong value)
{
   hash ^= value;
   hash *= 1099511628211;
   return hash;
}

ulong MST_MakeSourceKey(const MST_PRODUCER_KIND producer,
                        const int producer_instance,
                        const ulong local_id)
{
   ulong hash = 1469598103934665603;
   hash = MST_HashMix(hash, (ulong)((int)producer));
   hash = MST_HashMix(hash, (ulong)MathMax(producer_instance, 0));
   hash = MST_HashMix(hash, local_id);
   return hash;
}

string MST_ProducerName(const MST_PRODUCER_KIND producer)
{
   if(producer == MST_PRODUCER_VOLKITT) return "VOLKITT";
   if(producer == MST_PRODUCER_DAY_SWINGS) return "DAY_SWINGS";
   if(producer == MST_PRODUCER_WAYNE) return "WAYNE";
   if(producer == MST_PRODUCER_MARKET_STRUCTURE) return "MARKET_STRUCTURE";
   return "NONE";
}

string MST_FamilyName(const MST_FAMILY family)
{
   if(family == MST_FAMILY_ADAPTIVE_VALUE) return "ADAPTIVE_VALUE";
   if(family == MST_FAMILY_GLOBAL_VALUE) return "GLOBAL_VALUE";
   if(family == MST_FAMILY_SENTINEL) return "SENTINEL";
   if(family == MST_FAMILY_TPO_PROFILE) return "TPO_PROFILE";
   if(family == MST_FAMILY_SINGLE_PRINT) return "SINGLE_PRINT";
   if(family == MST_FAMILY_DAILY_EXTREME) return "DAILY_EXTREME";
   if(family == MST_FAMILY_PIVOT) return "PIVOT";
   if(family == MST_FAMILY_MID_PIVOT) return "MID_PIVOT";
   if(family == MST_FAMILY_PIVOT_ZONE) return "PIVOT_ZONE";
   if(family == MST_FAMILY_MARKET_STRUCTURE) return "MARKET_STRUCTURE";
   return "NONE";
}

#endif // __KITT_MASTER_STRUCTURE_TYPES_MQH__
