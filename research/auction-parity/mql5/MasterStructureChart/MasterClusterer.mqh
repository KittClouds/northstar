//+------------------------------------------------------------------+
//| MasterClusterer.mqh                                              |
//| Allocation-amortized compatibility-aware 1D DBSCAN and nodes.    |
//+------------------------------------------------------------------+
#ifndef __KITT_MASTER_STRUCTURE_CLUSTERER_MQH__
#define __KITT_MASTER_STRUCTURE_CLUSTERER_MQH__

#include "MasterTypes.mqh"

bool MST_LevelLess(const MST_Level &left, const MST_Level &right)
{
   if(left.normalized_price < right.normalized_price) return true;
   if(left.normalized_price > right.normalized_price) return false;
   if((int)left.family < (int)right.family) return true;
   if((int)left.family > (int)right.family) return false;
   if((int)left.producer < (int)right.producer) return true;
   if((int)left.producer > (int)right.producer) return false;
   if(left.producer_instance < right.producer_instance) return true;
   if(left.producer_instance > right.producer_instance) return false;
   return left.local_id < right.local_id;
}

void MST_SortLevels(MST_Level &levels[], const int count)
{
   int limit = MathMin(count, ArraySize(levels));
   for(int i = 1; i < limit; i++)
   {
      MST_Level value = levels[i];
      int j = i - 1;
      while(j >= 0 && MST_LevelLess(value, levels[j]))
      {
         levels[j + 1] = levels[j];
         j--;
      }
      levels[j + 1] = value;
   }
}

double MST_LevelDistance(const MST_Level &left,
                         const MST_Level &right,
                         const bool use_interval_distance)
{
   if(!use_interval_distance)
      return MathAbs(left.normalized_price - right.normalized_price);

   if(left.normalized_upper < right.normalized_lower)
      return right.normalized_lower - left.normalized_upper;
   if(right.normalized_upper < left.normalized_lower)
      return left.normalized_lower - right.normalized_upper;
   return 0.0;
}

class CMstDensityClusterer
{
private:
   int   m_capacity;
   int   m_neighbors[];
   int   m_queue[];
   bool  m_queued[];
   int   m_noise_count;

   bool EnsureCapacity(const int count)
   {
      if(count <= m_capacity)
         return true;
      if(ArrayResize(m_neighbors, count) != count) return false;
      if(ArrayResize(m_queue, count) != count) return false;
      if(ArrayResize(m_queued, count) != count) return false;
      m_capacity = count;
      return true;
   }

   int CollectNeighbors(const MST_Level &levels[],
                        const int count,
                        const int point_index,
                        const MST_CompatibilityConfig &compatibility,
                        const MST_ClusterConfig &config)
   {
      int found = 0;
      for(int i = 0; i < count; i++)
      {
         if(!MST_LevelsCompatible(levels[point_index], levels[i], compatibility))
            continue;
         double distance = MST_LevelDistance(levels[point_index], levels[i],
                                             config.use_interval_distance);
         if(distance <= config.epsilon_atr)
         {
            m_neighbors[found] = i;
            found++;
         }
      }
      return found;
   }

   void ExpandCluster(const MST_Level &levels[],
                      const int count,
                      const int seed_index,
                      const int seed_neighbor_count,
                      const int cluster_id,
                      const MST_CompatibilityConfig &compatibility,
                      const MST_ClusterConfig &config,
                      int &labels[])
   {
      ArrayInitialize(m_queued, false);
      int queue_head = 0;
      int queue_count = 0;

      for(int i = 0; i < seed_neighbor_count; i++)
      {
         int neighbor = m_neighbors[i];
         if(neighbor < 0 || neighbor >= count || m_queued[neighbor])
            continue;
         m_queue[queue_count] = neighbor;
         queue_count++;
         m_queued[neighbor] = true;
         if(labels[neighbor] == MST_CLUSTER_UNCLASSIFIED ||
            labels[neighbor] == MST_CLUSTER_NOISE)
            labels[neighbor] = cluster_id;
      }
      labels[seed_index] = cluster_id;

      while(queue_head < queue_count)
      {
         int current = m_queue[queue_head];
         queue_head++;

         int neighbor_count = CollectNeighbors(levels, count, current,
                                               compatibility, config);
         if(neighbor_count < config.min_samples)
            continue;

         for(int i = 0; i < neighbor_count; i++)
         {
            int neighbor = m_neighbors[i];
            if(neighbor < 0 || neighbor >= count)
               continue;

            if(labels[neighbor] == MST_CLUSTER_NOISE)
               labels[neighbor] = cluster_id;

            if(labels[neighbor] != MST_CLUSTER_UNCLASSIFIED)
               continue;

            labels[neighbor] = cluster_id;
            if(!m_queued[neighbor] && queue_count < count)
            {
               m_queue[queue_count] = neighbor;
               queue_count++;
               m_queued[neighbor] = true;
            }
         }
      }
   }

public:
   CMstDensityClusterer(void)
   {
      m_capacity = 0;
      m_noise_count = 0;
   }

   int Fit(const MST_Level &levels[],
           const int requested_count,
           const MST_CompatibilityConfig &compatibility,
           const MST_ClusterConfig &config,
           int &labels[])
   {
      int count = MathMin(requested_count, ArraySize(levels));
      count = MathMin(count, MathMax(config.max_levels, 0));
      m_noise_count = 0;
      if(count <= 0 || config.epsilon_atr < 0.0 || config.min_samples < 1)
      {
         ArrayResize(labels, 0);
         return 0;
      }
      if(!EnsureCapacity(count) || ArrayResize(labels, count) != count)
         return -1;

      ArrayInitialize(labels, MST_CLUSTER_UNCLASSIFIED);
      int cluster_count = 0;

      for(int i = 0; i < count; i++)
      {
         if(labels[i] != MST_CLUSTER_UNCLASSIFIED)
            continue;

         int neighbor_count = CollectNeighbors(levels, count, i,
                                               compatibility, config);
         if(neighbor_count < config.min_samples)
         {
            labels[i] = MST_CLUSTER_NOISE;
            continue;
         }

         ExpandCluster(levels, count, i, neighbor_count, cluster_count,
                       compatibility, config, labels);
         cluster_count++;
      }

      for(int i = 0; i < count; i++)
         if(labels[i] == MST_CLUSTER_NOISE)
            m_noise_count++;
      return cluster_count;
   }

   int NoiseCount(void) const { return m_noise_count; }
};

int MST_CountBits(ulong value)
{
   int count = 0;
   while(value != 0)
   {
      value &= (value - 1);
      count++;
   }
   return count;
}

bool MST_SourceRefLess(const MST_SourceRef &left, const MST_SourceRef &right)
{
   if(left.source_key < right.source_key) return true;
   if(left.source_key > right.source_key) return false;
   if((int)left.producer < (int)right.producer) return true;
   if((int)left.producer > (int)right.producer) return false;
   if(left.producer_instance < right.producer_instance) return true;
   if(left.producer_instance > right.producer_instance) return false;
   return left.local_id < right.local_id;
}

void MST_SortSourceRange(MST_SourceRef &sources[], const int offset, const int count)
{
   int end = MathMin(offset + count, ArraySize(sources));
   for(int i = offset + 1; i < end; i++)
   {
      MST_SourceRef value = sources[i];
      int j = i - 1;
      while(j >= offset && MST_SourceRefLess(value, sources[j]))
      {
         sources[j + 1] = sources[j];
         j--;
      }
      sources[j + 1] = value;
   }
}

ulong MST_BuildNodeId(const MST_SourceRef &sources[], const int offset, const int count)
{
   ulong hash = 1469598103934665603;
   hash = MST_HashMix(hash, (ulong)count);
   int end = MathMin(offset + count, ArraySize(sources));
   for(int i = MathMax(offset, 0); i < end; i++)
   {
      hash = MST_HashMix(hash, sources[i].source_key);
      hash = MST_HashMix(hash, (ulong)((int)sources[i].family));
      hash = MST_HashMix(hash, (ulong)MathMax(sources[i].source_kind, 0));
   }
   return hash;
}

bool MST_NodeLess(const MST_Node &left, const MST_Node &right)
{
   if(left.normalized_price < right.normalized_price) return true;
   if(left.normalized_price > right.normalized_price) return false;
   return left.node_id < right.node_id;
}

void MST_SortNodes(MST_Node &nodes[], const int count)
{
   int limit = MathMin(count, ArraySize(nodes));
   for(int i = 1; i < limit; i++)
   {
      MST_Node value = nodes[i];
      int j = i - 1;
      while(j >= 0 && MST_NodeLess(value, nodes[j]))
      {
         nodes[j + 1] = nodes[j];
         j--;
      }
      nodes[j + 1] = value;
   }
}

int MST_BuildNodes(const MST_Level &levels[],
                   const int requested_count,
                   const int &labels[],
                   const int requested_cluster_count,
                   const double reference_price,
                   const double atr,
                   const MST_ClusterConfig &config,
                   MST_Node &nodes[],
                   MST_SourceRef &provenance[])
{
   int count = MathMin(requested_count, ArraySize(levels));
   count = MathMin(count, ArraySize(labels));
   int cluster_count = MathMin(requested_cluster_count, MathMax(config.max_nodes, 0));
   if(count <= 0 || cluster_count <= 0 || atr <= 0.0)
   {
      ArrayResize(nodes, 0);
      ArrayResize(provenance, 0);
      return 0;
   }

   ArrayResize(nodes, cluster_count);
   double weight_sum[];
   double price_sum[];
   int cursor[];
   ArrayResize(weight_sum, cluster_count);
   ArrayResize(price_sum, cluster_count);
   ArrayResize(cursor, cluster_count);
   ArrayInitialize(weight_sum, 0.0);
   ArrayInitialize(price_sum, 0.0);
   ArrayInitialize(cursor, 0);

   for(int c = 0; c < cluster_count; c++)
   {
      ZeroMemory(nodes[c]);
      nodes[c].valid = true;
      nodes[c].structural_region = MST_REGION_UNAVAILABLE;
      nodes[c].cluster_id = c;
      nodes[c].lower = DBL_MAX;
      nodes[c].upper = -DBL_MAX;
      nodes[c].normalized_lower = DBL_MAX;
      nodes[c].normalized_upper = -DBL_MAX;
   }

   int provenance_count = 0;
   for(int i = 0; i < count; i++)
   {
      int cluster = labels[i];
      if(cluster < 0 || cluster >= cluster_count)
         continue;
      double weight = MathMax(levels[i].evidence_weight, 0.000001);
      nodes[cluster].member_count++;
      provenance_count++;
      weight_sum[cluster] += weight;
      price_sum[cluster] += levels[i].price * weight;
      nodes[cluster].lower = MathMin(nodes[cluster].lower, levels[i].lower);
      nodes[cluster].upper = MathMax(nodes[cluster].upper, levels[i].upper);
      nodes[cluster].normalized_lower = MathMin(nodes[cluster].normalized_lower, levels[i].normalized_lower);
      nodes[cluster].normalized_upper = MathMax(nodes[cluster].normalized_upper, levels[i].normalized_upper);
      nodes[cluster].family_mask |= MST_FamilyBit(levels[i].family);
      nodes[cluster].role_mask |= MST_RoleBit(levels[i].role);
      if(levels[i].developing) nodes[cluster].developing_count++;
      if(levels[i].frozen_geometry) nodes[cluster].frozen_count++;
      if(levels[i].state == MST_STATE_BROKEN) nodes[cluster].broken_count++;
      if(levels[i].created_at > 0 &&
         (nodes[cluster].oldest_source_time == 0 || levels[i].created_at < nodes[cluster].oldest_source_time))
         nodes[cluster].oldest_source_time = levels[i].created_at;
      if(levels[i].updated_at > nodes[cluster].newest_update_time)
         nodes[cluster].newest_update_time = levels[i].updated_at;
   }

   ArrayResize(provenance, provenance_count);
   int running = 0;
   for(int c = 0; c < cluster_count; c++)
   {
      nodes[c].provenance_offset = running;
      nodes[c].provenance_count = nodes[c].member_count;
      cursor[c] = running;
      running += nodes[c].member_count;
   }

   for(int i = 0; i < count; i++)
   {
      int cluster = labels[i];
      if(cluster < 0 || cluster >= cluster_count)
         continue;
      int dst = cursor[cluster];
      cursor[cluster]++;
      provenance[dst].source_key = levels[i].source_key;
      provenance[dst].producer = levels[i].producer;
      provenance[dst].producer_instance = levels[i].producer_instance;
      provenance[dst].local_id = levels[i].local_id;
      provenance[dst].family = levels[i].family;
      provenance[dst].source_kind = levels[i].source_kind;
   }

   for(int c = 0; c < cluster_count; c++)
   {
      if(nodes[c].member_count <= 0 || weight_sum[c] <= 0.0)
      {
         nodes[c].valid = false;
         continue;
      }
      nodes[c].price = price_sum[c] / weight_sum[c];
      nodes[c].width = MathMax(nodes[c].upper - nodes[c].lower, 0.0);
      nodes[c].normalized_price = (nodes[c].price - reference_price) / atr;
      nodes[c].width_atr = nodes[c].width / atr;
      nodes[c].distance_atr = nodes[c].normalized_price;
      nodes[c].family_count = MST_CountBits(nodes[c].family_mask);
      if(nodes[c].normalized_price < -config.center_role_tolerance_atr)
         nodes[c].role = MST_NODE_LOWER;
      else if(nodes[c].normalized_price > config.center_role_tolerance_atr)
         nodes[c].role = MST_NODE_UPPER;
      else
         nodes[c].role = MST_NODE_CENTER;

      MST_SortSourceRange(provenance, nodes[c].provenance_offset,
                          nodes[c].provenance_count);
      nodes[c].node_id = MST_BuildNodeId(provenance, nodes[c].provenance_offset,
                                        nodes[c].provenance_count);
   }

   MST_SortNodes(nodes, cluster_count);
   return cluster_count;
}

ulong MST_SnapshotHash(const MST_Node &nodes[],
                       const int requested_count,
                       const double point)
{
   int count = MathMin(requested_count, ArraySize(nodes));
   double safe_point = MathMax(point, 0.00000001);
   ulong hash = 1469598103934665603;
   hash = MST_HashMix(hash, (ulong)count);
   for(int i = 0; i < count; i++)
   {
      if(!nodes[i].valid) continue;
      long price_ticks = (long)MathRound(nodes[i].price / safe_point);
      long lower_ticks = (long)MathRound(nodes[i].lower / safe_point);
      long upper_ticks = (long)MathRound(nodes[i].upper / safe_point);
      hash = MST_HashMix(hash, nodes[i].node_id);
      hash = MST_HashMix(hash, (ulong)price_ticks);
      hash = MST_HashMix(hash, (ulong)lower_ticks);
      hash = MST_HashMix(hash, (ulong)upper_ticks);
   }
   return hash;
}

#endif // __KITT_MASTER_STRUCTURE_CLUSTERER_MQH__
