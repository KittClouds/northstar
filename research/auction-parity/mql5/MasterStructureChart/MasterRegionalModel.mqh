//+------------------------------------------------------------------+
//| MasterRegionalModel.mqh                                         |
//| Deterministic structural-median coordinate system.              |
//+------------------------------------------------------------------+
#ifndef __KITT_MASTER_REGIONAL_MODEL_MQH__
#define __KITT_MASTER_REGIONAL_MODEL_MQH__

#include "MasterTypes.mqh"

class CMstRegionalModel
{
private:
   MST_RegionalSnapshot m_state;
   datetime m_previous_bar_time;
   double   m_previous_median;
   double   m_previous_sigma;
   double   m_previous_cog;
   double   m_previous_atr;
   bool     m_previous_valid;
   bool     m_previous_has_cog;
   double   m_cog_velocity_price;
   double   m_cog_velocity_atr;
   double   m_cog_velocity_sigma;
   double   m_median_velocity_price;
   double   m_median_velocity_atr;
   double   m_median_velocity_sigma;
   double   m_sigma_log_change;
   ulong    m_last_basis_hash;
   int      m_last_population_count;

   void SortPopulation(double &prices[], ulong &ids[]) const
   {
      int count = ArraySize(prices);
      for(int i = 1; i < count; i++)
      {
         double price = prices[i];
         ulong id = ids[i];
         int j = i - 1;
         while(j >= 0 && (prices[j] > price ||
               (prices[j] == price && ids[j] > id)))
         {
            prices[j + 1] = prices[j];
            ids[j + 1] = ids[j];
            j--;
         }
         prices[j + 1] = price;
         ids[j + 1] = id;
      }
   }

   bool FindCog(const MST_Level &levels[], const int requested_count,
                double &price) const
   {
      int count = MathMin(requested_count, ArraySize(levels));
      datetime newest = 0;
      bool found = false;
      for(int i = 0; i < count; i++)
      {
         if(!levels[i].valid || levels[i].producer != MST_PRODUCER_VOLKITT ||
            levels[i].source_kind != 150) continue;
         if(!found || levels[i].updated_at >= newest)
         {
            newest = levels[i].updated_at;
            price = levels[i].price;
            found = true;
         }
      }
      return found;
   }

   bool NodeContainsCog(const MST_Node &node,
                        const MST_SourceRef &provenance[],
                        const int requested_count) const
   {
      int count = MathMin(requested_count, ArraySize(provenance));
      int begin = MathMax(node.provenance_offset, 0);
      int end = MathMin(begin + MathMax(node.provenance_count, 0), count);
      for(int i = begin; i < end; i++)
         if(provenance[i].producer == MST_PRODUCER_VOLKITT &&
            provenance[i].source_kind == 150) return true;
      return false;
   }

   int ElapsedBars(const string symbol, const ENUM_TIMEFRAMES timeframe,
                   const datetime previous, const datetime current) const
   {
      if(previous <= 0 || current <= previous) return 1;
      int old_shift = iBarShift(symbol, timeframe, previous, false);
      int new_shift = iBarShift(symbol, timeframe, current, false);
      if(old_shift >= 0 && new_shift >= 0 && old_shift != new_shift)
         return MathMax(MathAbs(old_shift - new_shift), 1);
      return 1;
   }

public:
   CMstRegionalModel(void) { Reset(); }

   void Reset(void)
   {
      ZeroMemory(m_state);
      m_state.price_region = MST_REGION_UNAVAILABLE;
      m_state.cog_region = MST_REGION_UNAVAILABLE;
      m_previous_bar_time = 0;
      m_previous_median = 0.0;
      m_previous_sigma = 0.0;
      m_previous_cog = 0.0;
      m_previous_atr = 0.0;
      m_previous_valid = false;
      m_previous_has_cog = false;
      m_cog_velocity_price = 0.0;
      m_cog_velocity_atr = 0.0;
      m_cog_velocity_sigma = 0.0;
      m_median_velocity_price = 0.0;
      m_median_velocity_atr = 0.0;
      m_median_velocity_sigma = 0.0;
      m_sigma_log_change = 0.0;
      m_last_basis_hash = 0;
      m_last_population_count = 0;
   }

   MST_STRUCTURAL_REGION Classify(const double z) const
   {
      if(!MathIsValidNumber(z)) return MST_REGION_UNAVAILABLE;
      int region = (int)MathRound(z);
      region = MathMax(-3, MathMin(region, 3));
      return (MST_STRUCTURAL_REGION)region;
   }

   void RebuildBasis(const MST_Node &nodes[], const int requested_node_count,
                     const MST_Level &levels[], const int requested_level_count,
                     const ulong structure_snapshot_hash,
                     const ulong structure_generation,
                     const double point)
   {
      double prices[];
      ulong ids[];
      int node_count = MathMin(requested_node_count, ArraySize(nodes));
      ArrayResize(prices, node_count);
      ArrayResize(ids, node_count);
      int count = 0;
      for(int i = 0; i < node_count; i++)
      {
         if(!nodes[i].valid || nodes[i].existence != MST_NODE_ACTIVE) continue;
         prices[count] = nodes[i].price;
         ids[count] = nodes[i].node_id;
         count++;
      }
      ArrayResize(prices, count);
      ArrayResize(ids, count);
      SortPopulation(prices, ids);

      ulong hash = MST_HashMix(1469598103934665603, (ulong)count);
      double safe_point = MathMax(point, 0.00000001);
      double mean = 0.0;
      for(int i = 0; i < count; i++)
      {
         mean += prices[i];
         hash = MST_HashMix(hash, ids[i]);
         hash = MST_HashMix(hash, (ulong)((long)MathRound(prices[i] / safe_point)));
      }
      if(count > 0) mean /= (double)count;
      double median = 0.0;
      if(count > 0)
         median = (count % 2 == 1) ? prices[count / 2] :
                  (prices[count / 2 - 1] + prices[count / 2]) * 0.5;
      double variance = 0.0;
      for(int i = 0; i < count; i++)
      {
         double delta = prices[i] - mean;
         variance += delta * delta;
      }
      double sigma = count > 0 ? MathSqrt(variance / (double)count) : 0.0;

      int population_delta = count - m_last_population_count;
      bool basis_changed = hash != m_last_basis_hash;
      m_state.valid = count > 0;
      m_state.sigma_valid = count >= 2 && sigma > safe_point * 0.5;
      m_state.structure_snapshot_hash = structure_snapshot_hash;
      m_state.regional_basis_hash = hash;
      m_state.structure_generation = structure_generation;
      m_state.population_count = count;
      m_state.population_count_delta = population_delta;
      m_state.basis_changed = basis_changed;
      m_state.median_price = median;
      m_state.mean_price = mean;
      m_state.structural_sigma = sigma;
      m_state.cog_price = 0.0;
      m_state.has_cog = FindCog(levels, requested_level_count, m_state.cog_price);
      m_last_basis_hash = hash;
      m_last_population_count = count;
   }

   void Observe(const string symbol, const ENUM_TIMEFRAMES timeframe,
                const double reference_price, const double atr,
                const datetime closed_bar_time, const bool basis_rebuilt)
   {
      double safe_atr = MathMax(atr, 0.00000001);
      if(!basis_rebuilt)
      {
         m_state.basis_changed = false;
         m_state.population_count_delta = 0;
      }
      if(closed_bar_time > 0 && closed_bar_time != m_previous_bar_time)
      {
         int elapsed = ElapsedBars(symbol, timeframe, m_previous_bar_time,
                                   closed_bar_time);
         m_state.velocity_elapsed_bars = elapsed;
         if(m_previous_valid)
         {
            double previous_atr = MathMax(m_previous_atr, 0.00000001);
            m_median_velocity_price = (m_state.median_price - m_previous_median) /
                                      (double)elapsed;
            m_median_velocity_atr = m_median_velocity_price / previous_atr;
            if(m_previous_sigma > 0.0)
            {
               m_median_velocity_sigma = m_median_velocity_price / m_previous_sigma;
               if(m_state.structural_sigma > 0.0)
                  m_sigma_log_change = MathLog(m_state.structural_sigma /
                                               m_previous_sigma) / (double)elapsed;
            }
            if(m_state.has_cog && m_previous_has_cog)
            {
               m_cog_velocity_price = (m_state.cog_price - m_previous_cog) /
                                      (double)elapsed;
               m_cog_velocity_atr = m_cog_velocity_price / previous_atr;
               if(m_previous_sigma > 0.0)
                  m_cog_velocity_sigma = m_cog_velocity_price / m_previous_sigma;
            }
         }
         m_previous_bar_time = closed_bar_time;
         m_previous_median = m_state.median_price;
         m_previous_sigma = m_state.structural_sigma;
         m_previous_cog = m_state.cog_price;
         m_previous_atr = atr;
         m_previous_valid = m_state.valid && m_state.sigma_valid;
         m_previous_has_cog = m_state.has_cog;
      }

      m_state.frozen_bar_time = closed_bar_time;
      m_state.reference_price = reference_price;
      m_state.cog_velocity_price_per_bar = m_cog_velocity_price;
      m_state.cog_velocity_atr_per_bar = m_cog_velocity_atr;
      m_state.cog_velocity_sigma_per_bar = m_cog_velocity_sigma;
      m_state.median_velocity_price_per_bar = m_median_velocity_price;
      m_state.median_velocity_atr_per_bar = m_median_velocity_atr;
      m_state.median_velocity_sigma_per_bar = m_median_velocity_sigma;
      m_state.sigma_log_change_per_bar = m_sigma_log_change;
      m_state.price_from_median_atr = m_state.valid ?
         (reference_price - m_state.median_price) / safe_atr : 0.0;
      m_state.price_from_cog_atr = m_state.has_cog ?
         (reference_price - m_state.cog_price) / safe_atr : 0.0;
      m_state.cog_median_gap_atr = m_state.valid && m_state.has_cog ?
         (m_state.cog_price - m_state.median_price) / safe_atr : 0.0;
      if(m_state.sigma_valid)
      {
         m_state.price_from_median_sigma =
            (reference_price - m_state.median_price) / m_state.structural_sigma;
         m_state.price_region = Classify(m_state.price_from_median_sigma);
         if(m_state.has_cog)
         {
            m_state.price_from_cog_sigma =
               (reference_price - m_state.cog_price) / m_state.structural_sigma;
            m_state.cog_median_gap_sigma =
               (m_state.cog_price - m_state.median_price) / m_state.structural_sigma;
            m_state.cog_region = Classify(m_state.cog_median_gap_sigma);
         }
         else
            m_state.cog_region = MST_REGION_UNAVAILABLE;
      }
      else
      {
         m_state.price_from_median_sigma = 0.0;
         m_state.price_from_cog_sigma = 0.0;
         m_state.cog_median_gap_sigma = 0.0;
         m_state.price_region = MST_REGION_UNAVAILABLE;
         m_state.cog_region = MST_REGION_UNAVAILABLE;
      }
   }

   void ApplyToNodes(MST_Node &nodes[], const int requested_node_count,
                     const MST_SourceRef &provenance[],
                     const int requested_provenance_count,
                     const double atr) const
   {
      int count = MathMin(requested_node_count, ArraySize(nodes));
      double safe_atr = MathMax(atr, 0.00000001);
      for(int i = 0; i < count; i++)
      {
         nodes[i].structural_region = MST_REGION_UNAVAILABLE;
         nodes[i].median_distance_atr = 0.0;
         nodes[i].median_distance_sigma = 0.0;
         nodes[i].cog_distance_sigma = 0.0;
         nodes[i].width_sigma = 0.0;
         nodes[i].contains_cog = NodeContainsCog(nodes[i], provenance,
                                                 requested_provenance_count);
         if(!nodes[i].valid || !m_state.valid) continue;
         nodes[i].median_distance_atr =
            (nodes[i].price - m_state.median_price) / safe_atr;
         if(!m_state.sigma_valid) continue;
         nodes[i].median_distance_sigma =
            (nodes[i].price - m_state.median_price) / m_state.structural_sigma;
         nodes[i].structural_region = Classify(nodes[i].median_distance_sigma);
         nodes[i].width_sigma = MathMax(nodes[i].upper - nodes[i].lower, 0.0) /
                                m_state.structural_sigma;
         if(m_state.has_cog)
            nodes[i].cog_distance_sigma =
               (nodes[i].price - m_state.cog_price) / m_state.structural_sigma;
      }
   }

   void GetSnapshot(MST_RegionalSnapshot &out) const { out = m_state; }
};

#endif // __KITT_MASTER_REGIONAL_MODEL_MQH__
