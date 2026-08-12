//+------------------------------------------------------------------+
//| ChartInteraction.mqh                                             |
//| Lightweight, non-persistent node interaction state for rendering.|
//+------------------------------------------------------------------+
#ifndef __KITT_MASTER_STRUCTURE_CHART_INTERACTION_MQH__
#define __KITT_MASTER_STRUCTURE_CHART_INTERACTION_MQH__

#include "MasterTypes.mqh"

enum MSC_INTERACTION_STATE
{
   MSC_INTERACTION_IDLE = 0,
   MSC_INTERACTION_APPROACH,
   MSC_INTERACTION_CONTACT,
   MSC_INTERACTION_PENETRATION,
   MSC_INTERACTION_BROKEN,
   MSC_INTERACTION_ACCEPTED,
   MSC_INTERACTION_RETEST
};

string MSC_InteractionStateName(const MSC_INTERACTION_STATE state)
{
   if(state == MSC_INTERACTION_APPROACH) return "APPROACH";
   if(state == MSC_INTERACTION_CONTACT) return "CONTACT";
   if(state == MSC_INTERACTION_PENETRATION) return "PENETRATION";
   if(state == MSC_INTERACTION_BROKEN) return "BROKEN";
   if(state == MSC_INTERACTION_ACCEPTED) return "ACCEPTED";
   if(state == MSC_INTERACTION_RETEST) return "RETEST";
   return "IDLE";
}

struct MSC_InteractionConfig
{
   bool   enabled;
   double approach_radius_atr;
   double break_buffer_atr;
   int    acceptance_closes;
   double acceptance_distance_atr;
   double departure_distance_atr;
};

void MSC_DefaultInteractionConfig(MSC_InteractionConfig &cfg)
{
   ZeroMemory(cfg);
   cfg.enabled = true;
   cfg.approach_radius_atr = 0.50;
   cfg.break_buffer_atr = 0.00;
   cfg.acceptance_closes = 2;
   cfg.acceptance_distance_atr = 0.00;
   cfg.departure_distance_atr = 0.25;
}

struct MSC_InteractionTrack
{
   ulong                 node_id;
   MSC_INTERACTION_STATE state;
   int                   approach_side;
   int                   acceptance_closes;
   int                   attempt_count;
   datetime              last_closed_bar;
   bool                  seen;
};

class CMscInteractionTracker
{
private:
   MSC_InteractionConfig m_config;
   MSC_InteractionTrack  m_tracks[];

   int Find(const ulong node_id) const
   {
      for(int i = 0; i < ArraySize(m_tracks); i++)
         if(m_tracks[i].node_id == node_id) return i;
      return -1;
   }

   int Ensure(const ulong node_id)
   {
      int slot = Find(node_id);
      if(slot >= 0) return slot;
      slot = ArraySize(m_tracks);
      ArrayResize(m_tracks, slot + 1, 64);
      ZeroMemory(m_tracks[slot]);
      m_tracks[slot].node_id = node_id;
      m_tracks[slot].state = MSC_INTERACTION_IDLE;
      return slot;
   }

   void Compact(void)
   {
      int write = 0;
      for(int i = 0; i < ArraySize(m_tracks); i++)
      {
         if(!m_tracks[i].seen) continue;
         if(write != i) m_tracks[write] = m_tracks[i];
         write++;
      }
      if(write != ArraySize(m_tracks)) ArrayResize(m_tracks, write);
   }

public:
   CMscInteractionTracker(void) { Reset(); }

   void Init(const MSC_InteractionConfig &config)
   {
      m_config = config;
      ArrayResize(m_tracks, 0);
   }

   void Reset(void)
   {
      MSC_DefaultInteractionConfig(m_config);
      ArrayResize(m_tracks, 0);
   }

   bool Observe(MST_Node &nodes[], const int node_count,
                const double price, const double atr,
                const datetime closed_bar_time, const double closed_price)
   {
      bool changed = false;
      for(int i = 0; i < ArraySize(m_tracks); i++) m_tracks[i].seen = false;

      for(int i = 0; i < node_count; i++)
      {
         if(!nodes[i].valid) continue;
         int slot = Ensure(nodes[i].node_id);
         MSC_InteractionTrack track = m_tracks[slot];
         track.seen = true;
         MSC_INTERACTION_STATE previous = track.state;

         if(!m_config.enabled || atr <= 0.0)
         {
            track.state = MSC_INTERACTION_IDLE;
         }
         else
         {
            double lower = MathMin(nodes[i].lower, nodes[i].upper);
            double upper = MathMax(nodes[i].lower, nodes[i].upper);
            double distance = price < lower ? lower - price :
                              price > upper ? price - upper : 0.0;
            bool inside = price >= lower && price <= upper;

            if(track.state == MSC_INTERACTION_IDLE &&
               distance <= m_config.approach_radius_atr * atr)
            {
               track.state = inside ? MSC_INTERACTION_CONTACT : MSC_INTERACTION_APPROACH;
               track.approach_side = price < lower ? -1 : (price > upper ? 1 : 0);
               track.acceptance_closes = 0;
               track.attempt_count++;
            }
            else if((track.state == MSC_INTERACTION_APPROACH ||
                     track.state == MSC_INTERACTION_CONTACT ||
                     track.state == MSC_INTERACTION_PENETRATION) && inside)
            {
               track.state = track.state == MSC_INTERACTION_APPROACH ?
                             MSC_INTERACTION_CONTACT : MSC_INTERACTION_PENETRATION;
            }

            double break_buffer = m_config.break_buffer_atr * atr;
            bool crossed_far = track.approach_side < 0 ? price > upper + break_buffer :
                               track.approach_side > 0 ? price < lower - break_buffer : false;
            if(crossed_far && track.state != MSC_INTERACTION_ACCEPTED &&
               track.state != MSC_INTERACTION_RETEST)
               track.state = MSC_INTERACTION_BROKEN;

            if(closed_bar_time > 0 && closed_bar_time != track.last_closed_bar)
            {
               track.last_closed_bar = closed_bar_time;
               double accept_buffer = m_config.acceptance_distance_atr * atr;
               bool close_far = track.approach_side < 0 ? closed_price > upper + accept_buffer :
                                track.approach_side > 0 ? closed_price < lower - accept_buffer : false;
               if(track.state == MSC_INTERACTION_BROKEN && close_far)
               {
                  track.acceptance_closes++;
                  if(track.acceptance_closes >= MathMax(m_config.acceptance_closes, 1))
                     track.state = MSC_INTERACTION_ACCEPTED;
               }
               else if(track.state == MSC_INTERACTION_BROKEN)
                  track.acceptance_closes = 0;
            }

            if(track.state == MSC_INTERACTION_ACCEPTED && inside)
               track.state = MSC_INTERACTION_RETEST;

            double departure = m_config.departure_distance_atr * atr;
            bool departed_origin = track.approach_side < 0 ? price < lower - departure :
                                   track.approach_side > 0 ? price > upper + departure : false;
            if(departed_origin && track.state != MSC_INTERACTION_ACCEPTED)
            {
               track.state = MSC_INTERACTION_IDLE;
               track.approach_side = 0;
               track.acceptance_closes = 0;
            }
         }

         m_tracks[slot] = track;
         nodes[i].interaction_state = (int)track.state;
         nodes[i].attempt_count = track.attempt_count;
         if(previous != track.state) changed = true;
      }
      Compact();
      return changed;
   }

   int ActiveCount(void) const
   {
      int count = 0;
      for(int i = 0; i < ArraySize(m_tracks); i++)
         if(m_tracks[i].state != MSC_INTERACTION_IDLE) count++;
      return count;
   }
};

#endif // __KITT_MASTER_STRUCTURE_CHART_INTERACTION_MQH__
