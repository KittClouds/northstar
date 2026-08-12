//+------------------------------------------------------------------+
//| MasterAuctionTransit.mqh                                        |
//| Dense deterministic source/destination competing-risk ledger.   |
//+------------------------------------------------------------------+
#ifndef __KITT_MASTER_AUCTION_TRANSIT_MQH__
#define __KITT_MASTER_AUCTION_TRANSIT_MQH__

#include "MasterAuctionTypes.mqh"

class CMstAuctionTransitBook
{
private:
   MST_TransitTrack m_active[];
   MST_TransitTrack m_completed[];
   long             m_started;
   long             m_resolved;
   long             m_censored;

   bool TouchesBand(const double previous,
                    const double current,
                    const double lower,
                    const double upper) const
   {
      if(current >= lower && current <= upper) return true;
      return MathMin(previous, current) <= upper && MathMax(previous, current) >= lower;
   }

   void Complete(MST_TransitTrack &transit,
                 const MST_AUCTION_RESOLUTION resolution,
                 const MST_COMPLETION_STATUS status,
                 const MST_CENSOR_REASON censor_reason,
                 const datetime market_time,
                 const int bar_sequence,
                 const double price,
                 const MST_RegionalSnapshot &regional)
   {
      transit.active = false;
      transit.resolution = resolution;
      transit.completion_status = status;
      transit.censor_reason = censor_reason;
      transit.ended_at = market_time;
      transit.end_bar_sequence = bar_sequence;
      transit.end_price = price;
      transit.end_price_region = regional.price_region;
      transit.end_price_from_median_sigma = regional.price_from_median_sigma;
      double net = MathAbs(price - transit.start_price);
      transit.path_efficiency = transit.path_length > 0.0 ? net / transit.path_length : 1.0;
      int slot = ArraySize(m_completed);
      ArrayResize(m_completed, slot + 1, 16);
      m_completed[slot] = transit;
      if(status == MST_COMPLETION_RIGHT_CENSORED) m_censored++;
      else m_resolved++;
   }

   void Compact(void)
   {
      int write = 0;
      for(int i = 0; i < ArraySize(m_active); i++)
      {
         if(!m_active[i].active) continue;
         if(write != i) m_active[write] = m_active[i];
         write++;
      }
      if(write != ArraySize(m_active)) ArrayResize(m_active, write);
   }

public:
   CMstAuctionTransitBook(void) { Reset(); }

   void Reset(void)
   {
      ArrayResize(m_active, 0);
      ArrayResize(m_completed, 0);
      m_started = 0;
      m_resolved = 0;
      m_censored = 0;
   }

   void BeginCycle(void) { ArrayResize(m_completed, 0); }

   bool Start(const MST_AuctionAttempt &attempt,
              const MST_Node &nodes[],
              const int requested_count,
              const datetime market_time,
              const int bar_sequence,
              const double price)
   {
      ulong destination = attempt.direction == MST_APPROACH_FROM_BELOW ?
                          attempt.nearest_above_id : attempt.nearest_below_id;
      int count = MathMin(requested_count, ArraySize(nodes));
      for(int i = 0; i < count; i++)
      {
         if(destination == 0 || !nodes[i].valid || nodes[i].node_id != destination) continue;
         MST_TransitTrack transit;
         ZeroMemory(transit);
         transit.active = true;
         transit.attempt_id = attempt.attempt_id;
         transit.episode_id = attempt.episode_id;
         transit.source_node_id = attempt.node_id;
         transit.destination_node_id = destination;
         transit.transit_id = MST_HashMix(attempt.attempt_id, destination);
         if(transit.transit_id == 0) transit.transit_id = 1;
         transit.direction = (int)attempt.direction;
         transit.started_at = market_time;
         transit.start_bar_sequence = bar_sequence;
         transit.start_price = price;
         transit.last_price = price;
         transit.source_lower = attempt.frozen_lower;
         transit.source_upper = attempt.frozen_upper;
         transit.destination_lower = nodes[i].lower;
         transit.destination_upper = nodes[i].upper;
         transit.frozen_atr = attempt.frozen_atr;
         transit.distance_atr = MathAbs(nodes[i].price - attempt.frozen_price) /
                                MathMax(attempt.frozen_atr, 0.00000001);
         transit.regional_basis_hash = attempt.context.regional.regional_basis_hash;
         transit.structure_snapshot_hash = attempt.context.regional.structure_snapshot_hash;
         transit.source_region = attempt.node_region;
         transit.destination_region = nodes[i].structural_region;
         transit.start_price_region = attempt.start_region;
         transit.end_price_region = attempt.start_region;
         transit.start_median_price = attempt.context.regional.median_price;
         transit.start_structural_sigma = attempt.context.regional.structural_sigma;
         transit.start_price_from_median_sigma = attempt.start_median_sigma;
         transit.end_price_from_median_sigma = attempt.start_median_sigma;
         transit.completion_status = MST_COMPLETION_ACTIVE;
         int slot = ArraySize(m_active);
         ArrayResize(m_active, slot + 1, 16);
         m_active[slot] = transit;
         m_started++;
         return true;
      }
      return false;
   }

   void Observe(const datetime market_time,
                const int bar_sequence,
                const double price,
                const int max_bars,
                const MST_RegionalSnapshot &regional)
   {
      for(int i = 0; i < ArraySize(m_active); i++)
      {
         MST_TransitTrack transit = m_active[i];
         double previous = transit.last_price;
         transit.path_length += MathAbs(price - previous);
         transit.last_price = price;
         bool moving_up = transit.direction == (int)MST_APPROACH_FROM_BELOW;
         double adverse = moving_up ? transit.start_price - price :
                                      price - transit.start_price;
         transit.max_adverse_atr = MathMax(transit.max_adverse_atr,
            MathMax(adverse / MathMax(transit.frozen_atr, 0.00000001), 0.0));
         if(TouchesBand(previous, price, transit.destination_lower, transit.destination_upper))
            Complete(transit, MST_RESOLUTION_TRANSIT_TO_NEXT_NODE, MST_COMPLETION_RESOLVED,
                     MST_CENSOR_NONE, market_time, bar_sequence, price, regional);
         else if(TouchesBand(previous, price, transit.source_lower, transit.source_upper))
            Complete(transit, MST_RESOLUTION_RETURN_TO_SOURCE_NODE, MST_COMPLETION_RESOLVED,
                     MST_CENSOR_NONE, market_time, bar_sequence, price, regional);
         else if(bar_sequence - transit.start_bar_sequence > MathMax(max_bars, 1) * 2)
            Complete(transit, MST_RESOLUTION_TIMEOUT, MST_COMPLETION_RESOLVED,
                     MST_CENSOR_NONE, market_time, bar_sequence, price, regional);
         m_active[i] = transit;
      }
      Compact();
   }

   void CensorAll(const datetime market_time,
                  const int bar_sequence,
                  const double price,
                  const MST_CENSOR_REASON reason,
                  const MST_RegionalSnapshot &regional)
   {
      for(int i = 0; i < ArraySize(m_active); i++)
      {
         MST_TransitTrack transit = m_active[i];
         Complete(transit, MST_RESOLUTION_NONE, MST_COMPLETION_RIGHT_CENSORED,
                  reason, market_time, bar_sequence, price, regional);
         m_active[i] = transit;
      }
      Compact();
   }

   int ActiveCount(void) const { return ArraySize(m_active); }
   int CompletedCount(void) const { return ArraySize(m_completed); }
   bool HasActiveForEpisode(const ulong episode_id) const
   {
      for(int i = 0; i < ArraySize(m_active); i++)
         if(m_active[i].active && m_active[i].episode_id == episode_id) return true;
      return false;
   }
   long StartedTotal(void) const { return m_started; }
   long ResolvedTotal(void) const { return m_resolved; }
   long CensoredTotal(void) const { return m_censored; }

   bool GetCompleted(const int index, MST_TransitTrack &out) const
   {
      if(index < 0 || index >= ArraySize(m_completed)) return false;
      out = m_completed[index];
      return true;
   }
};

#endif // __KITT_MASTER_AUCTION_TRANSIT_MQH__
