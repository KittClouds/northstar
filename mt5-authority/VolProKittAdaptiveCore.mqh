//+------------------------------------------------------------------+
//| VolProKittAdaptiveCore.mqh                                      |
//| Deterministic adaptive geometry for VolProKittAdaptive.mq5.      |
//| No chart objects, indicator buffers, files, or MT5 callbacks.    |
//+------------------------------------------------------------------+
#ifndef __VOL_PRO_KITT_ADAPTIVE_CORE_MQH__
#define __VOL_PRO_KITT_ADAPTIVE_CORE_MQH__

#define KVA_MAX_CLUSTERS 10
#define KVA_MIN_CLUSTERS 2
#define KVA_MAX_ACTIVE_CLUSTERS 8
#define KVA_MIN_LOOKBACK 64
#define KVA_MAX_LOOKBACK 512
#define KVA_MAX_ITERATIONS 100
#define KVA_MIN_ROWS 8
#define KVA_MAX_ROWS 80
#define KVA_HISTOGRAM_BINS 16

enum KVA_LEVEL_STATE
{
   KVA_LEVEL_FRESH = 0,
   KVA_LEVEL_TESTED = 1,
   KVA_LEVEL_ACCEPTED = 2,
   KVA_LEVEL_REJECTED = 3,
   KVA_LEVEL_BROKEN = 4,
   KVA_LEVEL_RECLAIMED = 5
};

struct KVA_PROFILE
{
   bool     valid;
   int      lookback;
   int      clusters;
   int      velocity_bars;
   double   median_range;
   double   realized_volatility;
   double   auction_entropy;
   double   structural_similarity;
   double   cluster_score;
   double   confidence;
   ulong    generation;
   datetime committed_at;
};

struct KVA_CLUSTER
{
   bool     valid;
   int      fit_index;
   int      object_id;
   double   centroid;
   double   low;
   double   high;
   double   mass;
   double   mass_share;
   double   poc;
   double   bin_size;
   int      rows;
   int      observations;
};

struct KVA_LEVEL_MEMORY
{
   bool            valid;
   int             object_id;
   double          price;
   datetime        birth_time;
   int             age_bars;
   int             touch_count;
   int             rejection_count;
   double          max_break_dist;
   int             reclaim_success;
   int             bars_accepted;
   KVA_LEVEL_STATE status;
   datetime        last_time_updated;
   bool            touched_this_bar;
   int             broken_side;
   double          snap_centroid;
   double          snap_mass;
   double          snap_range;
   datetime        snap_time;
   string          regime_dir;
   string          regime_auc;
   string          regime_vol;
};

int KVA_ClampInt(const int value, const int lower, const int upper)
{
   if(value < lower) return lower;
   if(value > upper) return upper;
   return value;
}

double KVA_Clamp(const double value, const double lower, const double upper)
{
   if(value < lower) return lower;
   if(value > upper) return upper;
   return value;
}

double KVA_BarVolume(const MqlRates &bar)
{
   if(bar.real_volume > 0)
      return (double)bar.real_volume;
   return MathMax((double)bar.tick_volume, 1.0);
}

double KVA_MedianRange(const MqlRates &rates[], const int start, const int count)
{
   if(count <= 0)
      return 0.0;
   double values[];
   ArrayResize(values, count);
   for(int i = 0; i < count; i++)
      values[i] = MathMax(rates[start + i].high - rates[start + i].low, 0.0);
   ArraySort(values);
   if((count & 1) == 1)
      return values[count / 2];
   return (values[count / 2 - 1] + values[count / 2]) * 0.5;
}

double KVA_RealizedVolatility(const MqlRates &rates[], const int start, const int count)
{
   if(count < 2)
      return 0.0;
   double sum = 0.0;
   double sum_sq = 0.0;
   int used = 0;
   for(int i = start; i < start + count - 1; i++)
   {
      double newer = rates[i].close;
      double older = rates[i + 1].close;
      if(newer <= 0.0 || older <= 0.0)
         continue;
      double value = MathLog(newer / older);
      sum += value;
      sum_sq += value * value;
      used++;
   }
   if(used < 2)
      return 0.0;
   double mean = sum / used;
   return MathSqrt(MathMax(sum_sq / used - mean * mean, 0.0));
}

void KVA_Distribution(const MqlRates &rates[],
                      const int start,
                      const int count,
                      const double minimum,
                      const double maximum,
                      double &bins[])
{
   ArrayResize(bins, KVA_HISTOGRAM_BINS);
   ArrayInitialize(bins, 0.0);
   double span = MathMax(maximum - minimum, _Point);
   double total = 0.0;
   for(int i = 0; i < count; i++)
   {
      const MqlRates bar = rates[start + i];
      double price = (bar.high + bar.low + bar.close) / 3.0;
      int index = (int)MathFloor((price - minimum) / span * KVA_HISTOGRAM_BINS);
      index = KVA_ClampInt(index, 0, KVA_HISTOGRAM_BINS - 1);
      double weight = KVA_BarVolume(bar);
      bins[index] += weight;
      total += weight;
   }
   if(total > 0.0)
   {
      for(int i = 0; i < KVA_HISTOGRAM_BINS; i++)
         bins[i] /= total;
   }
}

double KVA_DistributionSimilarity(const MqlRates &rates[],
                                  const int first_start,
                                  const int second_start,
                                  const int block)
{
   double minimum = DBL_MAX;
   double maximum = -DBL_MAX;
   for(int i = 0; i < block; i++)
   {
      minimum = MathMin(minimum, MathMin(rates[first_start + i].low,
                                        rates[second_start + i].low));
      maximum = MathMax(maximum, MathMax(rates[first_start + i].high,
                                        rates[second_start + i].high));
   }
   if(maximum <= minimum)
      return 1.0;

   double first[];
   double second[];
   KVA_Distribution(rates, first_start, block, minimum, maximum, first);
   KVA_Distribution(rates, second_start, block, minimum, maximum, second);
   double distance = 0.0;
   for(int i = 0; i < KVA_HISTOGRAM_BINS; i++)
      distance += MathAbs(first[i] - second[i]);
   return KVA_Clamp(1.0 - 0.5 * distance, 0.0, 1.0);
}

double KVA_AuctionEntropy(const MqlRates &rates[], const int start, const int count)
{
   if(count <= 0)
      return 0.0;
   double minimum = DBL_MAX;
   double maximum = -DBL_MAX;
   for(int i = 0; i < count; i++)
   {
      minimum = MathMin(minimum, rates[start + i].low);
      maximum = MathMax(maximum, rates[start + i].high);
   }
   double bins[];
   KVA_Distribution(rates, start, count, minimum, maximum, bins);
   double entropy = 0.0;
   for(int i = 0; i < KVA_HISTOGRAM_BINS; i++)
   {
      if(bins[i] > 0.0)
         entropy -= bins[i] * MathLog(bins[i]);
   }
   return entropy / MathLog((double)KVA_HISTOGRAM_BINS);
}

int KVA_SelectLookback(const MqlRates &rates[],
                       const int copied,
                       double &similarity)
{
   const int candidates[7] = {64, 96, 128, 192, 256, 384, 512};
   int selected = MathMin(KVA_MIN_LOOKBACK, copied - 2);
   similarity = 1.0;
   double accumulated = 0.0;
   int comparisons = 0;

   for(int c = 0; c < 7; c++)
   {
      int horizon = candidates[c];
      if(horizon + 1 > copied)
         break;
      int block = MathMin(64, horizon / 2);
      int older_start = 1 + horizon - block;
      double value = KVA_DistributionSimilarity(rates, 1, older_start, block);
      accumulated += value;
      comparisons++;

      double threshold = (horizon <= 128) ? 0.60 : 0.68;
      if(value < threshold && horizon > KVA_MIN_LOOKBACK)
         break;
      selected = horizon;
   }

   if(comparisons > 0)
      similarity = accumulated / comparisons;
   return KVA_ClampInt(selected, 32, MathMin(KVA_MAX_LOOKBACK, copied - 2));
}

void KVA_PrepareSamples(const MqlRates &rates[],
                        const int lookback,
                        double &prices[],
                        double &volumes[],
                        double &highs[],
                        double &lows[])
{
   ArrayResize(prices, lookback);
   ArrayResize(volumes, lookback);
   ArrayResize(highs, lookback);
   ArrayResize(lows, lookback);
   for(int i = 0; i < lookback; i++)
   {
      const MqlRates bar = rates[i + 1];
      prices[i] = (bar.high + bar.low + bar.close) / 3.0;
      volumes[i] = KVA_BarVolume(bar);
      highs[i] = bar.high;
      lows[i] = bar.low;
   }
}

double KVA_KMeans(const double &prices[],
                  const double &volumes[],
                  const int count,
                  const int clusters,
                  int &assignments[],
                  double &centroids[],
                  double &masses[])
{
   ArrayResize(assignments, count);
   ArrayInitialize(assignments, -1);
   ArrayResize(centroids, clusters);
   ArrayResize(masses, clusters);

   double minimum = DBL_MAX;
   double maximum = -DBL_MAX;
   for(int i = 0; i < count; i++)
   {
      minimum = MathMin(minimum, prices[i]);
      maximum = MathMax(maximum, prices[i]);
   }
   double span = MathMax(maximum - minimum, _Point);
   for(int cluster = 0; cluster < clusters; cluster++)
      centroids[cluster] = minimum + span * (cluster + 1.0) / (clusters + 1.0);

   double sum_pv[];
   ArrayResize(sum_pv, clusters);
   for(int iteration = 0; iteration < KVA_MAX_ITERATIONS; iteration++)
   {
      bool changed = false;
      for(int i = 0; i < count; i++)
      {
         int best = 0;
         double best_distance = DBL_MAX;
         for(int cluster = 0; cluster < clusters; cluster++)
         {
            double distance = MathAbs(prices[i] - centroids[cluster]);
            if(distance < best_distance)
            {
               best_distance = distance;
               best = cluster;
            }
         }
         if(assignments[i] != best)
         {
            assignments[i] = best;
            changed = true;
         }
      }
      if(!changed && iteration > 0)
         break;

      ArrayInitialize(sum_pv, 0.0);
      ArrayInitialize(masses, 0.0);
      for(int i = 0; i < count; i++)
      {
         int cluster = assignments[i];
         sum_pv[cluster] += prices[i] * volumes[i];
         masses[cluster] += volumes[i];
      }
      for(int cluster = 0; cluster < clusters; cluster++)
      {
         if(masses[cluster] > 0.0)
            centroids[cluster] = sum_pv[cluster] / masses[cluster];
      }
   }

   ArrayInitialize(masses, 0.0);
   double total_mass = 0.0;
   double distortion = 0.0;
   for(int i = 0; i < count; i++)
   {
      int cluster = assignments[i];
      double weight = volumes[i];
      masses[cluster] += weight;
      total_mass += weight;
      distortion += weight * MathAbs(prices[i] - centroids[cluster]) / span;
   }
   if(total_mass <= 0.0)
      return -DBL_MAX;

   double separation = 1.0;
   for(int left = 0; left < clusters; left++)
   {
      for(int right = left + 1; right < clusters; right++)
         separation = MathMin(separation,
                              MathAbs(centroids[left] - centroids[right]) / span);
   }
   double mass_entropy = 0.0;
   int tiny = 0;
   for(int cluster = 0; cluster < clusters; cluster++)
   {
      double share = masses[cluster] / total_mass;
      if(share > 0.0)
         mass_entropy -= share * MathLog(share);
      if(share < 0.04)
         tiny++;
   }
   mass_entropy /= MathLog((double)clusters);
   double normalized_distortion = distortion / total_mass;
   double fragmentation = (clusters - KVA_MIN_CLUSTERS) * 0.025;
   return 0.48 * (1.0 - KVA_Clamp(normalized_distortion, 0.0, 1.0))
        + 0.24 * KVA_Clamp(separation * clusters, 0.0, 1.0)
        + 0.20 * KVA_Clamp(mass_entropy, 0.0, 1.0)
        - 0.08 * ((double)tiny / clusters)
        - fragmentation;
}

int KVA_SelectClusterCount(const double &prices[],
                           const double &volumes[],
                           const int count,
                           const int current_clusters,
                           double &selected_score)
{
   int best = KVA_MIN_CLUSTERS;
   double best_score = -DBL_MAX;
   double current_score = -DBL_MAX;
   for(int clusters = KVA_MIN_CLUSTERS;
       clusters <= KVA_MAX_ACTIVE_CLUSTERS;
       clusters++)
   {
      int assignments[];
      double centroids[];
      double masses[];
      double score = KVA_KMeans(prices, volumes, count, clusters,
                                assignments, centroids, masses);
      if(score > best_score)
      {
         best_score = score;
         best = clusters;
      }
      if(clusters == current_clusters)
         current_score = score;
   }

   if(current_clusters >= KVA_MIN_CLUSTERS
      && current_clusters <= KVA_MAX_ACTIVE_CLUSTERS
      && current_score >= best_score - 0.055)
   {
      selected_score = current_score;
      return current_clusters;
   }
   selected_score = best_score;
   return best;
}

int KVA_SelectVelocityBars(const MqlRates &rates[],
                           const int lookback,
                           const double median_range)
{
   int short_count = MathMin(16, lookback / 2);
   int long_count = MathMin(96, lookback);
   double short_range = KVA_MedianRange(rates, 1, short_count);
   double long_range = KVA_MedianRange(rates, 1, long_count);
   double ratio = (long_range > 0.0) ? short_range / long_range : 1.0;
   double base = MathSqrt((double)MathMax(lookback, 1));
   double scale = KVA_Clamp(ratio, 0.55, 2.0);
   if(median_range <= _Point)
      scale = 1.0;
   return KVA_ClampInt((int)MathRound(base / scale), 2, 24);
}

void KVA_BuildCandidateProfile(const MqlRates &rates[],
                               const int copied,
                               const KVA_PROFILE &current,
                               KVA_PROFILE &candidate)
{
   ZeroMemory(candidate);
   double similarity = 0.0;
   int lookback = KVA_SelectLookback(rates, copied, similarity);
   double prices[], volumes[], highs[], lows[];
   KVA_PrepareSamples(rates, lookback, prices, volumes, highs, lows);
   double cluster_score = 0.0;
   int clusters = KVA_SelectClusterCount(prices, volumes, lookback,
                                         current.valid ? current.clusters : 0,
                                         cluster_score);
   double median_range = KVA_MedianRange(rates, 1, lookback);

   candidate.valid = true;
   candidate.lookback = lookback;
   candidate.clusters = clusters;
   candidate.velocity_bars = KVA_SelectVelocityBars(rates, lookback, median_range);
   candidate.median_range = median_range;
   candidate.realized_volatility = KVA_RealizedVolatility(rates, 1, lookback);
   candidate.auction_entropy = KVA_AuctionEntropy(rates, 1, lookback);
   candidate.structural_similarity = similarity;
   candidate.cluster_score = cluster_score;
   candidate.confidence = KVA_Clamp(0.55 * similarity
                                  + 0.30 * cluster_score
                                  + 0.15 * (1.0 - candidate.auction_entropy * 0.25),
                                    0.0, 1.0);
}

bool KVA_SameAdaptiveShape(const KVA_PROFILE &left, const KVA_PROFILE &right)
{
   return left.lookback == right.lookback
       && left.clusters == right.clusters
       && MathAbs(left.velocity_bars - right.velocity_bars) <= 1;
}

bool KVA_ConsiderProfile(const KVA_PROFILE &candidate,
                         const datetime closed_bar_time,
                         KVA_PROFILE &active,
                         KVA_PROFILE &pending,
                         int &pending_observations)
{
   if(!active.valid)
   {
      active = candidate;
      active.generation = 1;
      active.committed_at = closed_bar_time;
      pending_observations = 0;
      return true;
   }

   if(KVA_SameAdaptiveShape(candidate, active))
   {
      active.median_range = candidate.median_range;
      active.realized_volatility = candidate.realized_volatility;
      active.auction_entropy = candidate.auction_entropy;
      active.structural_similarity = candidate.structural_similarity;
      active.cluster_score = candidate.cluster_score;
      active.confidence = candidate.confidence;
      pending_observations = 0;
      pending.valid = false;
      return false;
   }

   if(!pending.valid || !KVA_SameAdaptiveShape(candidate, pending))
   {
      pending = candidate;
      pending_observations = 1;
      return false;
   }

   pending_observations++;
   pending = candidate;
   if(pending_observations < 3 || candidate.confidence < 0.52)
      return false;

   ulong next_generation = active.generation + 1;
   active = candidate;
   active.generation = next_generation;
   active.committed_at = closed_bar_time;
   pending.valid = false;
   pending_observations = 0;
   return true;
}

void KVA_SortClusters(KVA_CLUSTER &clusters[], const int count)
{
   for(int i = 1; i < count; i++)
   {
      KVA_CLUSTER value = clusters[i];
      int position = i - 1;
      while(position >= 0 && clusters[position].centroid > value.centroid)
      {
         clusters[position + 1] = clusters[position];
         position--;
      }
      clusters[position + 1] = value;
   }
}

bool KVA_FitClusters(const MqlRates &rates[],
                     const KVA_PROFILE &profile,
                     KVA_CLUSTER &clusters[])
{
   double prices[], volumes[], highs[], lows[];
   KVA_PrepareSamples(rates, profile.lookback, prices, volumes, highs, lows);
   int assignments[];
   double centroids[];
   double masses[];
   KVA_KMeans(prices, volumes, profile.lookback, profile.clusters,
              assignments, centroids, masses);

   ArrayResize(clusters, profile.clusters);
   double total_mass = 0.0;
   for(int cluster = 0; cluster < profile.clusters; cluster++)
   {
      ZeroMemory(clusters[cluster]);
      clusters[cluster].valid = true;
      clusters[cluster].fit_index = cluster;
      clusters[cluster].object_id = -1;
      clusters[cluster].centroid = centroids[cluster];
      clusters[cluster].low = DBL_MAX;
      clusters[cluster].high = -DBL_MAX;
      clusters[cluster].mass = masses[cluster];
      total_mass += masses[cluster];
   }

   for(int i = 0; i < profile.lookback; i++)
   {
      int cluster = assignments[i];
      clusters[cluster].low = MathMin(clusters[cluster].low, lows[i]);
      clusters[cluster].high = MathMax(clusters[cluster].high, highs[i]);
      clusters[cluster].observations++;
   }

   double point = MathMax(SymbolInfoDouble(_Symbol, SYMBOL_TRADE_TICK_SIZE), _Point);
   for(int cluster = 0; cluster < profile.clusters; cluster++)
   {
      if(clusters[cluster].observations <= 0)
      {
         clusters[cluster].valid = false;
         continue;
      }
      clusters[cluster].mass_share = (total_mass > 0.0)
                                     ? clusters[cluster].mass / total_mass : 0.0;
      double span = MathMax(clusters[cluster].high - clusters[cluster].low, point);
      double target_bin = MathMax(point, profile.median_range * 0.25);
      clusters[cluster].rows = KVA_ClampInt((int)MathRound(span / target_bin),
                                           KVA_MIN_ROWS, KVA_MAX_ROWS);
      clusters[cluster].bin_size = span / clusters[cluster].rows;

      double bins[];
      ArrayResize(bins, clusters[cluster].rows);
      ArrayInitialize(bins, 0.0);
      for(int i = 0; i < profile.lookback; i++)
      {
         if(assignments[i] != cluster)
            continue;
         double bar_span = MathMax(highs[i] - lows[i], point);
         for(int row = 0; row < clusters[cluster].rows; row++)
         {
            double bottom = clusters[cluster].low + row * clusters[cluster].bin_size;
            double top = bottom + clusters[cluster].bin_size;
            double overlap = MathMin(highs[i], top) - MathMax(lows[i], bottom);
            if(overlap > 0.0)
               bins[row] += volumes[i] * overlap / bar_span;
         }
      }
      int best_row = 0;
      double best_volume = -1.0;
      for(int row = 0; row < clusters[cluster].rows; row++)
      {
         if(bins[row] > best_volume)
         {
            best_volume = bins[row];
            best_row = row;
         }
      }
      clusters[cluster].poc = clusters[cluster].low
                            + (best_row + 0.5) * clusters[cluster].bin_size;
   }
   KVA_SortClusters(clusters, profile.clusters);
   return true;
}

double KVA_OverlapRatio(const KVA_CLUSTER &left, const KVA_CLUSTER &right)
{
   double overlap = MathMin(left.high, right.high) - MathMax(left.low, right.low);
   if(overlap <= 0.0)
      return 0.0;
   double union_span = MathMax(left.high, right.high) - MathMin(left.low, right.low);
   return (union_span > 0.0) ? overlap / union_span : 1.0;
}

double KVA_MatchCost(const KVA_CLUSTER &previous, const KVA_CLUSTER &current)
{
   double scale = MathMax(MathMax(previous.high - previous.low,
                                  current.high - current.low), _Point);
   double price_cost = MathAbs(previous.centroid - current.centroid) / scale;
   double overlap_cost = 1.0 - KVA_OverlapRatio(previous, current);
   double mass_cost = MathAbs(previous.mass_share - current.mass_share);
   return 0.55 * price_cost + 0.30 * overlap_cost + 0.15 * mass_cost;
}

void KVA_AssignStableObjects(const KVA_CLUSTER &previous[],
                             const int previous_count,
                             KVA_CLUSTER &current[],
                             const int current_count)
{
   bool previous_used[KVA_MAX_CLUSTERS];
   bool current_used[KVA_MAX_CLUSTERS];
   ArrayInitialize(previous_used, false);
   ArrayInitialize(current_used, false);

   int match_limit = MathMin(previous_count, current_count);
   for(int match = 0; match < match_limit; match++)
   {
      int best_previous = -1;
      int best_current = -1;
      double best_cost = DBL_MAX;
      for(int left = 0; left < previous_count; left++)
      {
         if(previous_used[left] || !previous[left].valid)
            continue;
         for(int right = 0; right < current_count; right++)
         {
            if(current_used[right] || !current[right].valid)
               continue;
            double cost = KVA_MatchCost(previous[left], current[right]);
            if(cost < best_cost)
            {
               best_cost = cost;
               best_previous = left;
               best_current = right;
            }
         }
      }
      if(best_previous < 0 || best_current < 0 || best_cost > 1.35)
         break;
      current[best_current].object_id = previous[best_previous].object_id;
      previous_used[best_previous] = true;
      current_used[best_current] = true;
   }

   bool object_used[KVA_MAX_CLUSTERS];
   ArrayInitialize(object_used, false);
   for(int current_index = 0; current_index < current_count; current_index++)
   {
      int object_id = current[current_index].object_id;
      if(object_id >= 0 && object_id < KVA_MAX_CLUSTERS)
         object_used[object_id] = true;
   }
   for(int current_index = 0; current_index < current_count; current_index++)
   {
      if(current[current_index].object_id >= 0)
         continue;
      for(int object_id = 0; object_id < KVA_MAX_CLUSTERS; object_id++)
      {
         if(!object_used[object_id])
         {
            current[current_index].object_id = object_id;
            object_used[object_id] = true;
            break;
         }
      }
   }
}

void KVA_ResetLevel(KVA_LEVEL_MEMORY &level, const int object_id)
{
   ZeroMemory(level);
   level.valid = true;
   level.object_id = object_id;
   level.status = KVA_LEVEL_FRESH;
   level.regime_dir = "Init";
   level.regime_auc = "Init";
   level.regime_vol = "Init";
}

void KVA_UpdateLevel(KVA_LEVEL_MEMORY &level,
                     const KVA_CLUSTER &cluster,
                     const MqlRates &current_bar,
                     const MqlRates &closed_bar,
                     const int velocity_bars,
                     const int period_seconds)
{
   double reset_distance = MathMax(cluster.bin_size * 3.0,
                                   (cluster.high - cluster.low) * 0.75);
   if(!level.valid || level.object_id != cluster.object_id
      || level.price <= 0.0 || MathAbs(cluster.poc - level.price) > reset_distance)
   {
      KVA_ResetLevel(level, cluster.object_id);
      level.price = cluster.poc;
      level.birth_time = closed_bar.time;
      level.last_time_updated = closed_bar.time;
      level.snap_time = closed_bar.time;
      level.snap_centroid = cluster.centroid;
      level.snap_mass = cluster.mass;
      level.snap_range = cluster.high - cluster.low;
      return;
   }

   level.price = cluster.poc;
   double threshold = MathMax(cluster.bin_size * 0.5, _Point * 5.0);
   if(level.last_time_updated != closed_bar.time)
   {
      level.age_bars++;
      double distance = closed_bar.close - level.price;
      double absolute_distance = MathAbs(distance);
      if(absolute_distance <= cluster.bin_size * 1.5)
      {
         level.bars_accepted++;
         if((level.status == KVA_LEVEL_TESTED || level.status == KVA_LEVEL_FRESH)
            && level.bars_accepted >= 3)
            level.status = KVA_LEVEL_ACCEPTED;
      }
      if(closed_bar.high >= level.price && closed_bar.low <= level.price)
      {
         bool rejected_below = closed_bar.open < level.price
                            && closed_bar.close < level.price
                            && level.price - closed_bar.close > threshold;
         bool rejected_above = closed_bar.open > level.price
                            && closed_bar.close > level.price
                            && closed_bar.close - level.price > threshold;
         if(rejected_below || rejected_above)
         {
            level.rejection_count++;
            if(level.status != KVA_LEVEL_BROKEN)
               level.status = KVA_LEVEL_REJECTED;
         }
      }
      if(level.broken_side == 0)
      {
         if(closed_bar.close > level.price + threshold
            && closed_bar.open < level.price)
         {
            level.broken_side = 1;
            level.status = KVA_LEVEL_BROKEN;
         }
         else if(closed_bar.close < level.price - threshold
                 && closed_bar.open > level.price)
         {
            level.broken_side = -1;
            level.status = KVA_LEVEL_BROKEN;
         }
      }
      else
      {
         level.max_break_dist = MathMax(level.max_break_dist, absolute_distance);
         if((level.broken_side == 1 && closed_bar.close < level.price)
            || (level.broken_side == -1 && closed_bar.close > level.price))
         {
            level.reclaim_success++;
            level.broken_side = 0;
            level.status = KVA_LEVEL_RECLAIMED;
         }
      }
      level.touched_this_bar = false;
      level.last_time_updated = closed_bar.time;
   }

   if(!level.touched_this_bar
      && current_bar.high >= level.price && current_bar.low <= level.price)
   {
      level.touch_count++;
      level.touched_this_bar = true;
      if(level.status == KVA_LEVEL_FRESH)
         level.status = KVA_LEVEL_TESTED;
   }

   int horizon_seconds = MathMax(velocity_bars, 1) * MathMax(period_seconds, 1);
   if(closed_bar.time - level.snap_time >= horizon_seconds)
   {
      if(level.snap_mass > 0.0)
      {
         double centroid_delta = cluster.centroid - level.snap_centroid;
         double mass_delta = cluster.mass - level.snap_mass;
         double range_delta = (cluster.high - cluster.low) - level.snap_range;
         double micro = cluster.bin_size * 0.15;
         level.regime_dir = centroid_delta > micro ? "Rise"
                          : centroid_delta < -micro ? "Fall" : "Stat";
         level.regime_auc = range_delta > micro ? "Expand"
                          : range_delta < -micro ? "Compress" : "Stat";
         level.regime_vol = mass_delta > 0.0 ? "Supply" : "Hollow";
      }
      level.snap_centroid = cluster.centroid;
      level.snap_mass = cluster.mass;
      level.snap_range = cluster.high - cluster.low;
      level.snap_time = closed_bar.time;
   }
}

string KVA_LevelStateName(const KVA_LEVEL_STATE state)
{
   if(state == KVA_LEVEL_TESTED) return "TESTED";
   if(state == KVA_LEVEL_ACCEPTED) return "ACCEPTED";
   if(state == KVA_LEVEL_REJECTED) return "REJECTED";
   if(state == KVA_LEVEL_BROKEN) return "BROKEN";
   if(state == KVA_LEVEL_RECLAIMED) return "RECLAIMED";
   return "FRESH";
}

#endif
