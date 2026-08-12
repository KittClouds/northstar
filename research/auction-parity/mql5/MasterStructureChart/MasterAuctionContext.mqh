//+------------------------------------------------------------------+
//| MasterAuctionContext.mqh                                        |
//| Causal feature freezes and corridor-content measurements.       |
//+------------------------------------------------------------------+
#ifndef __KITT_MASTER_AUCTION_CONTEXT_MQH__
#define __KITT_MASTER_AUCTION_CONTEXT_MQH__

#include "MasterAuctionTypes.mqh"

class CMstAuctionContextBuilder
{
private:
   datetime m_previous_bar;
   double   m_previous_cog;
   double   m_previous_c3;
   double   m_cog_velocity;
   double   m_c3_velocity;

   bool NewestKind(const MST_Level &levels[], const int requested_count,
                   const int source_kind, double &price) const
   {
      int count = MathMin(requested_count, ArraySize(levels));
      datetime newest = 0;
      bool found = false;
      for(int i = 0; i < count; i++)
      {
         if(!levels[i].valid || levels[i].producer != MST_PRODUCER_VOLKITT ||
            levels[i].source_kind != source_kind) continue;
         if(!found || levels[i].updated_at >= newest)
         {
            price = levels[i].price;
            newest = levels[i].updated_at;
            found = true;
         }
      }
      return found;
   }

   int CountLevels(const MST_Level &levels[], const int requested_count,
                   const double lower, const double upper) const
   {
      int result = 0;
      int count = MathMin(requested_count, ArraySize(levels));
      for(int i = 0; i < count; i++)
         if(levels[i].valid && levels[i].price > lower && levels[i].price < upper) result++;
      return result;
   }

   int CountNoise(const MST_Level &levels[], const int &labels[], const int requested_count,
                  const double lower, const double upper) const
   {
      int result = 0;
      int count = MathMin(requested_count, MathMin(ArraySize(levels), ArraySize(labels)));
      for(int i = 0; i < count; i++)
         if(levels[i].valid && labels[i] == MST_CLUSTER_NOISE &&
            levels[i].price > lower && levels[i].price < upper) result++;
      return result;
   }

public:
   CMstAuctionContextBuilder(void) { Reset(); }

   void Reset(void)
   {
      m_previous_bar = 0;
      m_previous_cog = 0.0;
      m_previous_c3 = 0.0;
      m_cog_velocity = 0.0;
      m_c3_velocity = 0.0;
   }

   void EnrichCorridors(MST_Node &nodes[], const int requested_node_count,
                        const MST_Level &levels[], const int &labels[],
                        const int requested_level_count) const
   {
      int node_count = MathMin(requested_node_count, ArraySize(nodes));
      for(int i = 0; i < node_count; i++)
      {
         nodes[i].corridor_up_level_count = 0;
         nodes[i].corridor_down_level_count = 0;
         nodes[i].corridor_up_noise_count = 0;
         nodes[i].corridor_down_noise_count = 0;
         if(!nodes[i].valid || nodes[i].existence != MST_NODE_ACTIVE) continue;
         double upper_edge = DBL_MAX;
         double lower_edge = -DBL_MAX;
         for(int n = 0; n < node_count; n++)
         {
            if(n == i || !nodes[n].valid || nodes[n].existence != MST_NODE_ACTIVE) continue;
            if(nodes[n].lower >= nodes[i].upper && nodes[n].lower < upper_edge)
               upper_edge = nodes[n].lower;
            if(nodes[n].upper <= nodes[i].lower && nodes[n].upper > lower_edge)
               lower_edge = nodes[n].upper;
         }
         if(upper_edge != DBL_MAX)
         {
            nodes[i].corridor_up_level_count = CountLevels(levels, requested_level_count,
                                                            nodes[i].upper, upper_edge);
            nodes[i].corridor_up_noise_count = CountNoise(levels, labels, requested_level_count,
                                                          nodes[i].upper, upper_edge);
         }
         if(lower_edge != -DBL_MAX)
         {
            nodes[i].corridor_down_level_count = CountLevels(levels, requested_level_count,
                                                              lower_edge, nodes[i].lower);
            nodes[i].corridor_down_noise_count = CountNoise(levels, labels, requested_level_count,
                                                            lower_edge, nodes[i].lower);
         }
      }
   }

   void Build(const string symbol, const ENUM_TIMEFRAMES timeframe,
              const MST_Level &levels[], const int requested_level_count,
              const int noise_count, const MST_Node &nodes[], const int requested_node_count,
              const double reference_price, const double atr,
              const MST_RegionalSnapshot &regional,
              MST_AuctionFeatureSnapshot &out)
   {
      ZeroMemory(out);
      out.regional = regional;
      double safe_atr = MathMax(atr, 0.00000001);
      out.frozen_bar_time = iTime(symbol, timeframe, 0);
      double c1 = 0.0, c5 = 0.0, lower = 0.0, upper = 0.0;
      out.has_cog = NewestKind(levels, requested_level_count, 150, out.cog_price);
      out.has_c3 = NewestKind(levels, requested_level_count, 103, out.c3_price);
      bool has_c1 = NewestKind(levels, requested_level_count, 101, c1);
      bool has_c5 = NewestKind(levels, requested_level_count, 105, c5);
      bool has_lower = NewestKind(levels, requested_level_count, 151, lower);
      bool has_upper = NewestKind(levels, requested_level_count, 152, upper);
      out.has_lattice = has_c1 && has_c5;
      out.has_field = has_lower && has_upper;
      out.has_profile = NewestKind(levels, requested_level_count, 200, out.profile_poc) &&
                        NewestKind(levels, requested_level_count, 201, out.profile_vah) &&
                        NewestKind(levels, requested_level_count, 202, out.profile_val);
      if(out.has_cog) out.cog_distance_atr = (out.cog_price - reference_price) / safe_atr;
      if(out.has_c3) out.c3_distance_atr = (out.c3_price - reference_price) / safe_atr;
      if(out.has_lattice) out.lattice_width_atr = MathAbs(c5 - c1) / safe_atr;
      if(out.has_field) out.field_width_atr = MathAbs(upper - lower) / safe_atr;
      if(out.has_profile)
      {
         out.poc_distance_atr = (out.profile_poc - reference_price) / safe_atr;
         out.vah_distance_atr = (out.profile_vah - reference_price) / safe_atr;
         out.val_distance_atr = (out.profile_val - reference_price) / safe_atr;
      }
      MqlTick tick;
      if(SymbolInfoTick(symbol, tick) && tick.ask >= tick.bid)
         out.spread_atr = (tick.ask - tick.bid) / safe_atr;
      if(out.frozen_bar_time > 0 && out.frozen_bar_time != m_previous_bar)
      {
         if(m_previous_bar > 0 && out.has_cog && m_previous_cog > 0.0)
            m_cog_velocity = (out.cog_price - m_previous_cog) / safe_atr;
         if(m_previous_bar > 0 && out.has_c3 && m_previous_c3 > 0.0)
            m_c3_velocity = (out.c3_price - m_previous_c3) / safe_atr;
         if(out.has_cog) m_previous_cog = out.cog_price;
         if(out.has_c3) m_previous_c3 = out.c3_price;
         m_previous_bar = out.frozen_bar_time;
      }
      out.cog_velocity_atr_per_bar = m_cog_velocity;
      out.c3_velocity_atr_per_bar = m_c3_velocity;
      out.raw_level_count = MathMin(requested_level_count, ArraySize(levels));
      out.noise_level_count = noise_count;
      int node_count = MathMin(requested_node_count, ArraySize(nodes));
      for(int i = 0; i < node_count; i++)
         if(nodes[i].valid && nodes[i].existence == MST_NODE_ACTIVE) out.active_node_count++;
   }
};

#endif // __KITT_MASTER_AUCTION_CONTEXT_MQH__
