//+------------------------------------------------------------------+
//| MasterAdapters.mqh                                               |
//| Explicit mappings from each producer into the canonical contract.|
//+------------------------------------------------------------------+
#ifndef __KITT_MASTER_STRUCTURE_ADAPTERS_MQH__
#define __KITT_MASTER_STRUCTURE_ADAPTERS_MQH__

#include "MasterTypes.mqh"
#include <02marketmain.mqh>
#include <02dayswings.mqh>
#include <02wayne.mqh>

MST_FAMILY MST_MapKvpFamily(const KVP_LEVEL_FAMILY family)
{
   if(family == KVP_FAMILY_ADAPTIVE_VALUE) return MST_FAMILY_ADAPTIVE_VALUE;
   if(family == KVP_FAMILY_GLOBAL_VALUE) return MST_FAMILY_GLOBAL_VALUE;
   if(family == KVP_FAMILY_SENTINEL) return MST_FAMILY_SENTINEL;
   if(family == KVP_FAMILY_TPO_PROFILE) return MST_FAMILY_TPO_PROFILE;
   if(family == KVP_FAMILY_SINGLE_PRINT) return MST_FAMILY_SINGLE_PRINT;
   return MST_FAMILY_NONE;
}

MST_ROLE MST_MapKvpRole(const KVP_LEVEL_ROLE role)
{
   if(role == KVP_ROLE_OUTER_LOW) return MST_ROLE_LOWER_EXTREME;
   if(role == KVP_ROLE_INNER_LOW) return MST_ROLE_LOWER_INTERMEDIATE;
   if(role == KVP_ROLE_FAIR_VALUE) return MST_ROLE_FAIR_VALUE;
   if(role == KVP_ROLE_INNER_HIGH) return MST_ROLE_UPPER_INTERMEDIATE;
   if(role == KVP_ROLE_OUTER_HIGH) return MST_ROLE_UPPER_EXTREME;
   if(role == KVP_ROLE_CENTER) return MST_ROLE_CENTER;
   if(role == KVP_ROLE_LOWER_BOUNDARY) return MST_ROLE_LOWER_BOUNDARY;
   if(role == KVP_ROLE_UPPER_BOUNDARY) return MST_ROLE_UPPER_BOUNDARY;
   if(role == KVP_ROLE_LOWER_MEMORY) return MST_ROLE_LOWER_MEMORY;
   if(role == KVP_ROLE_UPPER_MEMORY) return MST_ROLE_UPPER_MEMORY;
   return MST_ROLE_NONE;
}

MST_LIFECYCLE_STATE MST_MapKvpState(const KVP_LEVEL_STATE state)
{
   if(state == KVP_LEVEL_FRESH) return MST_STATE_FRESH;
   if(state == KVP_LEVEL_TESTED) return MST_STATE_TESTED;
   if(state == KVP_LEVEL_ACCEPTED) return MST_STATE_ACCEPTED;
   if(state == KVP_LEVEL_REJECTED) return MST_STATE_REJECTED;
   if(state == KVP_LEVEL_BROKEN) return MST_STATE_BROKEN;
   if(state == KVP_LEVEL_RECLAIMED) return MST_STATE_RECLAIMED;
   return MST_STATE_UNKNOWN;
}

MST_ROLE MST_MapDayRole(const DSW_LEVEL_ROLE role)
{
   if(role == DSW_ROLE_LOWER_BOUNDARY) return MST_ROLE_LOWER_BOUNDARY;
   if(role == DSW_ROLE_UPPER_BOUNDARY) return MST_ROLE_UPPER_BOUNDARY;
   return MST_ROLE_NONE;
}

MST_LIFECYCLE_STATE MST_MapDayState(const DSW_ZONE_STATE state)
{
   if(state == DSW_ZONE_FRESH) return MST_STATE_FRESH;
   if(state == DSW_ZONE_TOUCHED) return MST_STATE_TOUCHED;
   if(state == DSW_ZONE_REJECTED) return MST_STATE_REJECTED;
   if(state == DSW_ZONE_BROKEN) return MST_STATE_BROKEN;
   return MST_STATE_UNKNOWN;
}

MST_FAMILY MST_MapWayneFamily(const ENUM_WYN_FAMILY family)
{
   if(family == WYN_FAMILY_PIVOT) return MST_FAMILY_PIVOT;
   if(family == WYN_FAMILY_MID_PIVOT) return MST_FAMILY_MID_PIVOT;
   if(family == WYN_FAMILY_ZONE) return MST_FAMILY_PIVOT_ZONE;
   return MST_FAMILY_NONE;
}

MST_ROLE MST_MapWayneRole(const ENUM_WYN_ROLE role)
{
   if(role == WYN_ROLE_LOWER_EXTREME) return MST_ROLE_LOWER_EXTREME;
   if(role == WYN_ROLE_LOWER_MAJOR) return MST_ROLE_LOWER_MAJOR;
   if(role == WYN_ROLE_LOWER_INTERMEDIATE) return MST_ROLE_LOWER_INTERMEDIATE;
   if(role == WYN_ROLE_CENTER) return MST_ROLE_CENTER;
   if(role == WYN_ROLE_UPPER_INTERMEDIATE) return MST_ROLE_UPPER_INTERMEDIATE;
   if(role == WYN_ROLE_UPPER_MAJOR) return MST_ROLE_UPPER_MAJOR;
   if(role == WYN_ROLE_UPPER_EXTREME) return MST_ROLE_UPPER_EXTREME;
   if(role == WYN_ROLE_LOWER_ZONE) return MST_ROLE_LOWER_BOUNDARY;
   if(role == WYN_ROLE_UPPER_ZONE) return MST_ROLE_UPPER_BOUNDARY;
   return MST_ROLE_NONE;
}

bool MST_NormalizeLevel(MST_Level &level,
                        const double reference_price,
                        const double atr)
{
   if(!level.valid || !MST_IsFinitePrice(level.price) ||
      !MST_IsFinitePrice(reference_price) || !MST_IsFinitePrice(atr) || atr <= 0.0)
      return false;

   if(!MST_IsFinitePrice(level.lower)) level.lower = level.price;
   if(!MST_IsFinitePrice(level.upper)) level.upper = level.price;
   if(level.lower > level.upper)
   {
      double swap = level.lower;
      level.lower = level.upper;
      level.upper = swap;
   }
   if(level.price < level.lower) level.lower = level.price;
   if(level.price > level.upper) level.upper = level.price;

   level.width = MathMax(level.upper - level.lower, 0.0);
   level.normalized_lower = (level.lower - reference_price) / atr;
   level.normalized_price = (level.price - reference_price) / atr;
   level.normalized_upper = (level.upper - reference_price) / atr;
   level.width_atr = level.width / atr;
   level.distance_atr = level.normalized_price;
   return MathIsValidNumber(level.normalized_price);
}

int MST_AdaptKvpLevels(const KVP_Level &source_levels[],
                       const int input_count,
                       const int producer_instance,
                       MST_Level &out[],
                       const int offset)
{
   int written = 0;
   int limit = MathMin(input_count, ArraySize(source_levels));
   for(int i = 0; i < limit; i++)
   {
      if(!source_levels[i].valid)
         continue;
      int dst = offset + written;
      if(dst < 0 || dst >= ArraySize(out))
         break;

      MST_Level level;
      ZeroMemory(level);
      level.valid = true;
      level.producer = MST_PRODUCER_VOLKITT;
      level.producer_instance = producer_instance;
      level.local_id = source_levels[i].local_id;
      level.source_key = MST_MakeSourceKey(level.producer, producer_instance, level.local_id);
      level.family = MST_MapKvpFamily(source_levels[i].family);
      level.source_kind = (int)source_levels[i].kind;
      level.role = MST_MapKvpRole(source_levels[i].role);
      level.lower = source_levels[i].lower;
      level.price = source_levels[i].price;
      level.upper = source_levels[i].upper;
      level.width = MathMax(source_levels[i].upper - source_levels[i].lower, 0.0);
      level.timeframe = source_levels[i].timeframe;
      level.created_at = source_levels[i].created_at;
      level.updated_at = source_levels[i].updated_at;
      level.effective_start = source_levels[i].session_start;
      level.effective_end = source_levels[i].session_end;
      level.developing = source_levels[i].developing;
      level.frozen_geometry = source_levels[i].frozen;
      level.state = MST_MapKvpState(source_levels[i].state);
      level.touches = source_levels[i].touches;
      level.rejections = source_levels[i].rejections;
      level.reclaims = source_levels[i].reclaims;
      level.acceptance_bars = source_levels[i].acceptance_bars;
      level.mass = source_levels[i].mass;
      level.mass_share = source_levels[i].mass_share;
      level.evidence_weight = 1.0;
      level.max_excursion_atr = source_levels[i].max_excursion_atr;
      out[dst] = level;
      written++;
   }
   return written;
}

int MST_AdaptDayLevels(const DSW_Level &source_levels[],
                       const int input_count,
                       const int producer_instance,
                       MST_Level &out[],
                       const int offset)
{
   int written = 0;
   int limit = MathMin(input_count, ArraySize(source_levels));
   for(int i = 0; i < limit; i++)
   {
      if(!source_levels[i].valid)
         continue;
      int dst = offset + written;
      if(dst < 0 || dst >= ArraySize(out))
         break;

      MST_Level level;
      ZeroMemory(level);
      level.valid = true;
      level.producer = MST_PRODUCER_DAY_SWINGS;
      level.producer_instance = producer_instance;
      level.local_id = source_levels[i].local_id;
      level.source_key = MST_MakeSourceKey(level.producer, producer_instance, level.local_id);
      level.family = MST_FAMILY_DAILY_EXTREME;
      level.source_kind = (int)source_levels[i].kind;
      level.role = MST_MapDayRole(source_levels[i].role);
      level.lower = source_levels[i].lower;
      level.price = source_levels[i].price;
      level.upper = source_levels[i].upper;
      level.width = source_levels[i].width;
      level.timeframe = source_levels[i].timeframe;
      level.created_at = source_levels[i].created_at;
      level.updated_at = source_levels[i].updated_at;
      level.effective_start = source_levels[i].day_start;
      level.effective_end = 0;
      level.developing = source_levels[i].developing;
      level.frozen_geometry = source_levels[i].frozen_geometry;
      level.state = MST_MapDayState(source_levels[i].state);
      level.touches = source_levels[i].touches;
      level.rejections = source_levels[i].rejections;
      level.evidence_weight = 1.0;
      out[dst] = level;
      written++;
   }
   return written;
}

int MST_AdaptWayneLevels(const WYN_Level &source_levels[],
                         const int input_count,
                         const int producer_instance,
                         MST_Level &out[],
                         const int offset)
{
   int written = 0;
   int limit = MathMin(input_count, ArraySize(source_levels));
   for(int i = 0; i < limit; i++)
   {
      if(!source_levels[i].valid)
         continue;
      int dst = offset + written;
      if(dst < 0 || dst >= ArraySize(out))
         break;

      MST_Level level;
      ZeroMemory(level);
      level.valid = true;
      level.producer = MST_PRODUCER_WAYNE;
      level.producer_instance = producer_instance;
      level.local_id = source_levels[i].local_id;
      level.source_key = MST_MakeSourceKey(level.producer, producer_instance, level.local_id);
      level.family = MST_MapWayneFamily(source_levels[i].family);
      level.source_kind = (int)source_levels[i].kind;
      level.role = MST_MapWayneRole(source_levels[i].role);
      level.lower = source_levels[i].lower;
      level.price = source_levels[i].price;
      level.upper = source_levels[i].upper;
      level.width = source_levels[i].width;
      level.timeframe = source_levels[i].timeframe;
      level.created_at = source_levels[i].created_at;
      level.updated_at = source_levels[i].effective_start;
      level.effective_start = source_levels[i].effective_start;
      level.effective_end = source_levels[i].effective_end;
      level.developing = source_levels[i].developing;
      level.frozen_geometry = source_levels[i].frozen_geometry;
      level.state = MST_STATE_FRESH;
      level.evidence_weight = 1.0;
      out[dst] = level;
      written++;
   }
   return written;
}

#endif // __KITT_MASTER_STRUCTURE_ADAPTERS_MQH__
