//+------------------------------------------------------------------+
//| MasterAuctionEngine.mqh                                          |
//| Dense deterministic auction attempts, episodes, and transits.    |
//+------------------------------------------------------------------+
#ifndef __KITT_MASTER_AUCTION_ENGINE_MQH__
#define __KITT_MASTER_AUCTION_ENGINE_MQH__

#include "MasterAuctionTypes.mqh"
#include "MasterAuctionTransit.mqh"

struct MST_NodeAuctionMemory
{
   ulong node_id;
   int   attempt_ordinal;
   int   episode_ordinal;
   int   accepted_side;
};

class CMstAuctionEngine
{
private:
   MST_AuctionGrammarConfig m_config;
   MST_AuctionAttempt       m_attempts[];
   MST_AuctionAttempt       m_completed_attempts[];
   MST_AuctionEpisode       m_episodes[];
   MST_AuctionEpisode       m_completed_episodes[];
   MST_AuctionEvent         m_events[];
   MST_AuctionContextReceipt m_context[];
   CMstAuctionTransitBook   m_transits;
   MST_NodeAuctionMemory    m_memory[];
   MST_AuctionFeatureSnapshot m_feature_snapshot;
   datetime                 m_last_closed_bar_time;
   int                      m_bar_sequence;
   ulong                    m_event_sequence;
   ulong                    m_terminal_hash;
   long                     m_attempts_started;
   long                     m_attempts_resolved;
   long                     m_attempts_censored;
   long                     m_episodes_started;
   long                     m_episodes_resolved;
   long                     m_episodes_censored;

   int FindMemory(const ulong node_id) const
   {
      for(int i = 0; i < ArraySize(m_memory); i++)
         if(m_memory[i].node_id == node_id) return i;
      return -1;
   }

   int EnsureMemory(const ulong node_id)
   {
      int index = FindMemory(node_id);
      if(index >= 0) return index;
      index = ArraySize(m_memory);
      ArrayResize(m_memory, index + 1, 64);
      ZeroMemory(m_memory[index]);
      m_memory[index].node_id = node_id;
      return index;
   }

   int FindAttempt(const ulong node_id) const
   {
      for(int i = 0; i < ArraySize(m_attempts); i++)
         if(m_attempts[i].active && m_attempts[i].node_id == node_id) return i;
      return -1;
   }

   int FindEpisodeById(const ulong episode_id) const
   {
      for(int i = 0; i < ArraySize(m_episodes); i++)
         if(m_episodes[i].active && m_episodes[i].episode_id == episode_id) return i;
      return -1;
   }

   int FindEpisodeForNode(const ulong node_id) const
   {
      for(int i = 0; i < ArraySize(m_episodes); i++)
         if(m_episodes[i].active && m_episodes[i].node_id == node_id) return i;
      return -1;
   }

   double BoundaryGap(const double price, const double lower, const double upper) const
   {
      if(price < lower) return lower - price;
      if(price > upper) return price - upper;
      return 0.0;
   }

   int PriceSide(const double price, const double lower, const double upper) const
   {
      if(price < lower) return -1;
      if(price > upper) return 1;
      return 0;
   }

   ulong AttemptId(const ulong node_id,
                   const datetime started_at,
                   const int direction,
                   const int ordinal) const
   {
      ulong hash = MST_HashMix(1469598103934665603, node_id);
      hash = MST_HashMix(hash, (ulong)started_at);
      hash = MST_HashMix(hash, (ulong)(direction + 2));
      hash = MST_HashMix(hash, (ulong)ordinal);
      return hash == 0 ? 1 : hash;
   }

   ulong EpisodeId(const ulong node_id,
                   const datetime started_at,
                   const int ordinal) const
   {
      ulong hash = MST_HashMix(1099511628211, node_id);
      hash = MST_HashMix(hash, (ulong)started_at);
      hash = MST_HashMix(hash, (ulong)ordinal);
      return hash == 0 ? 1 : hash;
   }

   void PushEvent(MST_AuctionAttempt &attempt,
                  const MST_AUCTION_EVENT_KIND kind,
                  const datetime market_time,
                  const datetime bar_time,
                  const double price,
                  const double atr,
                  const double distance_atr,
                  const double penetration_atr,
                  const ulong related_node_id = 0)
   {
      int index = ArraySize(m_events);
      ArrayResize(m_events, index + 1, 32);
      attempt.event_count++;
      m_event_sequence++;
      ulong id = MST_HashMix(attempt.attempt_id, (ulong)attempt.event_count);
      id = MST_HashMix(id, (ulong)((int)kind));
      id = MST_HashMix(id, (ulong)market_time);
      MST_AuctionEvent event;
      ZeroMemory(event);
      event.event_id = id == 0 ? 1 : id;
      event.event_sequence = m_event_sequence;
      event.episode_id = attempt.episode_id;
      event.attempt_id = attempt.attempt_id;
      event.node_id = attempt.node_id;
      event.related_node_id = related_node_id;
      event.market_time = market_time;
      event.bar_time = bar_time;
      event.kind = kind;
      event.direction = attempt.direction;
      event.price = price;
      event.atr = atr;
      event.distance_atr = distance_atr;
      event.penetration_atr = penetration_atr;
      event.bar_sequence = m_bar_sequence;
      event.attempt_event_sequence = attempt.event_count;
      event.regional = m_feature_snapshot.regional;
      m_events[index] = event;
      m_terminal_hash = MST_HashMix(m_terminal_hash, event.event_id);
      m_terminal_hash = MST_HashMix(m_terminal_hash, (ulong)((int)kind));
      m_terminal_hash = MST_HashMix(m_terminal_hash, event.node_id);
      m_terminal_hash = MST_HashMix(m_terminal_hash,
                                    event.regional.regional_basis_hash);
      m_terminal_hash = MST_HashMix(m_terminal_hash,
         (ulong)((long)MathRound(event.regional.price_from_median_sigma * 100000000.0)));
      m_terminal_hash = MST_HashMix(m_terminal_hash,
                                    (ulong)((int)event.regional.price_region + 100));
   }

   void ApplyTransitCompletion(const MST_TransitTrack &transit,
                               const datetime bar_time)
   {
      MST_AuctionAttempt synthetic;
      ZeroMemory(synthetic);
      synthetic.attempt_id = transit.attempt_id;
      synthetic.episode_id = transit.episode_id;
      synthetic.node_id = transit.source_node_id;
      synthetic.direction = (MST_APPROACH_DIRECTION)transit.direction;
      MST_AUCTION_EVENT_KIND kind = MST_AUCTION_EVENT_EXPIRE;
      ulong related = 0;
      if(transit.completion_status == MST_COMPLETION_RIGHT_CENSORED)
         kind = MST_AUCTION_EVENT_CENSOR;
      else if(transit.resolution == MST_RESOLUTION_TRANSIT_TO_NEXT_NODE)
      {
         kind = MST_AUCTION_EVENT_TRANSIT;
         related = transit.destination_node_id;
      }
      else if(transit.resolution == MST_RESOLUTION_RETURN_TO_SOURCE_NODE)
      {
         kind = MST_AUCTION_EVENT_RETURN_TO_SOURCE;
         related = transit.source_node_id;
      }
      PushEvent(synthetic, kind, transit.ended_at, bar_time, transit.end_price,
                transit.frozen_atr, transit.distance_atr, 0.0, related);
      int episode = FindEpisodeById(transit.episode_id);
      if(episode >= 0 && transit.completion_status != MST_COMPLETION_RIGHT_CENSORED)
      {
         if(transit.resolution == MST_RESOLUTION_TRANSIT_TO_NEXT_NODE)
            m_episodes[episode].next_node_id = transit.destination_node_id;
         m_episodes[episode].resolution = transit.resolution;
         m_episodes[episode].ended_at = transit.ended_at;
         m_episodes[episode].terminal_region = transit.end_price_region;
      }
      m_terminal_hash = MST_HashMix(m_terminal_hash, transit.transit_id);
      m_terminal_hash = MST_HashMix(m_terminal_hash, (ulong)((int)transit.resolution));
      m_terminal_hash = MST_HashMix(m_terminal_hash, (ulong)((int)transit.completion_status));
      m_terminal_hash = MST_HashMix(m_terminal_hash, (ulong)((int)transit.censor_reason));
   }

   void CollectTransitCompletions(const datetime bar_time)
   {
      for(int i = 0; i < m_transits.CompletedCount(); i++)
      {
         MST_TransitTrack transit;
         if(m_transits.GetCompleted(i, transit)) ApplyTransitCompletion(transit, bar_time);
      }
   }

   void FreezeCorridors(MST_AuctionAttempt &attempt,
                        const MST_Node &nodes[],
                        const int node_count,
                        const double atr)
   {
      double up = DBL_MAX;
      double down = DBL_MAX;
      double safe_atr = MathMax(atr, 0.00000001);
      for(int i = 0; i < node_count; i++)
      {
         if(!nodes[i].valid || nodes[i].node_id == attempt.node_id ||
            nodes[i].existence != MST_NODE_ACTIVE) continue;
         if(nodes[i].lower >= attempt.frozen_upper)
         {
            double gap = nodes[i].lower - attempt.frozen_upper;
            if(gap < up) { up = gap; attempt.nearest_above_id = nodes[i].node_id; }
         }
         if(nodes[i].upper <= attempt.frozen_lower)
         {
            double gap = attempt.frozen_lower - nodes[i].upper;
            if(gap < down) { down = gap; attempt.nearest_below_id = nodes[i].node_id; }
         }
      }
      attempt.corridor_up_atr = up == DBL_MAX ? -1.0 : up / safe_atr;
      attempt.corridor_down_atr = down == DBL_MAX ? -1.0 : down / safe_atr;
   }

   int OpenEpisode(const MST_Node &node,
                   const datetime market_time,
                   const MST_APPROACH_DIRECTION direction,
                   const double atr)
   {
      int existing = FindEpisodeForNode(node.node_id);
      if(existing >= 0 && (m_episodes[existing].last_attempt_end_bar <= 0 ||
         m_bar_sequence - m_episodes[existing].last_attempt_end_bar <= m_config.episode_gap_bars))
         return existing;

      int memory = EnsureMemory(node.node_id);
      m_memory[memory].episode_ordinal++;
      MST_AuctionEpisode episode;
      ZeroMemory(episode);
      episode.active = true;
      episode.episode_id = EpisodeId(node.node_id, market_time,
                                     m_memory[memory].episode_ordinal);
      episode.node_id = node.node_id;
      episode.started_at = market_time;
      episode.start_bar_sequence = m_bar_sequence;
      episode.first_direction = direction;
      episode.resolution = MST_RESOLUTION_NONE;
      episode.completion_status = MST_COMPLETION_ACTIVE;
      episode.node_width_atr = node.width / MathMax(atr, 0.00000001);
      episode.family_mask = node.family_mask;
      episode.family_count = node.family_count;
      episode.member_count = node.member_count;
      episode.initial_region = m_feature_snapshot.regional.price_region;
      episode.terminal_region = episode.initial_region;
      int index = ArraySize(m_episodes);
      ArrayResize(m_episodes, index + 1, 32);
      m_episodes[index] = episode;
      m_episodes_started++;
      return index;
   }

   void EmitContext(const MST_AuctionAttempt &attempt,
                    const MST_SourceRef &provenance[],
                    const int provenance_count,
                    const datetime market_time)
   {
      int begin = MathMax(attempt.provenance_offset, 0);
      int end = MathMin(begin + MathMax(attempt.provenance_count, 0), provenance_count);
      end = MathMin(end, ArraySize(provenance));
      for(int i = begin; i < end; i++)
      {
         int slot = ArraySize(m_context);
         ArrayResize(m_context, slot + 1, 64);
         m_context[slot].attempt_id = attempt.attempt_id;
         m_context[slot].episode_id = attempt.episode_id;
         m_context[slot].node_id = attempt.node_id;
         m_context[slot].frozen_at = market_time;
         m_context[slot].source = provenance[i];
      }
   }

   int StartAttempt(MST_Node &node,
                    const MST_Node &nodes[],
                    const int node_count,
                    const MST_SourceRef &provenance[],
                    const int provenance_count,
                    const datetime market_time,
                    const datetime bar_time,
                    const double price,
                    const double atr,
                    const int side)
   {
      int memory = EnsureMemory(node.node_id);
      m_memory[memory].attempt_ordinal++;
      int episode_index = OpenEpisode(node, market_time,
                                      (MST_APPROACH_DIRECTION)side, atr);
      MST_AuctionAttempt attempt;
      ZeroMemory(attempt);
      attempt.active = true;
      attempt.completion_status = MST_COMPLETION_ACTIVE;
      attempt.is_retest = m_memory[memory].accepted_side == side;
      attempt.attempt_ordinal = m_memory[memory].attempt_ordinal;
      attempt.attempt_id = AttemptId(node.node_id, market_time, side,
                                     attempt.attempt_ordinal);
      attempt.episode_id = m_episodes[episode_index].episode_id;
      attempt.node_id = node.node_id;
      attempt.evidence_id = node.evidence_id;
      attempt.direction = (MST_APPROACH_DIRECTION)side;
      attempt.state = MST_AUCTION_APPROACH;
      attempt.started_at = market_time;
      attempt.last_observed_at = market_time;
      attempt.last_closed_bar_time = bar_time;
      attempt.start_bar_sequence = m_bar_sequence;
      attempt.frozen_lower = node.lower;
      attempt.frozen_price = node.price;
      attempt.frozen_upper = node.upper;
      attempt.frozen_width = MathMax(node.upper - node.lower, 0.0);
      attempt.frozen_atr = atr;
      attempt.start_price = price;
      attempt.last_price = price;
      attempt.family_mask = node.family_mask;
      attempt.family_count = node.family_count;
      attempt.member_count = node.member_count;
      attempt.developing_count = node.developing_count;
      attempt.frozen_count = node.frozen_count;
      attempt.node_revision = node.revision;
      attempt.provenance_offset = node.provenance_offset;
      attempt.provenance_count = node.provenance_count;
      attempt.corridor_up_level_count = node.corridor_up_level_count;
      attempt.corridor_down_level_count = node.corridor_down_level_count;
      attempt.corridor_up_noise_count = node.corridor_up_noise_count;
      attempt.corridor_down_noise_count = node.corridor_down_noise_count;
      attempt.context = m_feature_snapshot;
      attempt.start_region = m_feature_snapshot.regional.price_region;
      attempt.end_region = attempt.start_region;
      attempt.node_region = node.structural_region;
      attempt.start_median_sigma = m_feature_snapshot.regional.price_from_median_sigma;
      attempt.end_median_sigma = attempt.start_median_sigma;
      attempt.min_median_sigma = attempt.start_median_sigma;
      attempt.max_median_sigma = attempt.start_median_sigma;
      attempt.node_from_median_sigma = node.median_distance_sigma;
      attempt.node_from_cog_sigma = node.cog_distance_sigma;
      attempt.node_width_sigma = node.width_sigma;
      attempt.node_width_atr = attempt.frozen_width / MathMax(atr, 0.00000001);
      attempt.initial_distance_atr = BoundaryGap(price, node.lower, node.upper) /
                                     MathMax(atr, 0.00000001);
      FreezeCorridors(attempt, nodes, node_count, atr);
      m_episodes[episode_index].attempts++;
      m_episodes[episode_index].corridor_up_atr = attempt.corridor_up_atr;
      m_episodes[episode_index].corridor_down_atr = attempt.corridor_down_atr;
      node.attempt_count++;
      int slot = ArraySize(m_attempts);
      ArrayResize(m_attempts, slot + 1, 64);
      m_attempts[slot] = attempt;
      m_attempts_started++;
      EmitContext(m_attempts[slot], provenance, provenance_count, market_time);
      PushEvent(m_attempts[slot], MST_AUCTION_EVENT_APPROACH, market_time,
                bar_time, price, atr, attempt.initial_distance_atr, 0.0);
      return slot;
   }

   void UpdateEpisodeFromAttempt(const MST_AuctionAttempt &attempt)
   {
      int episode = FindEpisodeById(attempt.episode_id);
      if(episode < 0) return;
      m_episodes[episode].last_attempt_end_bar = m_bar_sequence;
      m_episodes[episode].ended_at = attempt.resolved_at;
      m_episodes[episode].resolution = attempt.resolution;
      if(attempt.broke_far_boundary) m_episodes[episode].breaks++;
      if(attempt.resolution == MST_RESOLUTION_RECLAIM_AFTER_BREAK) m_episodes[episode].reclaims++;
      if(attempt.is_retest) m_episodes[episode].retests++;
      m_episodes[episode].max_up_excursion_atr =
         MathMax(m_episodes[episode].max_up_excursion_atr, attempt.max_above_node_atr);
      m_episodes[episode].max_down_excursion_atr =
         MathMax(m_episodes[episode].max_down_excursion_atr, attempt.max_below_node_atr);
      m_episodes[episode].terminal_region = attempt.end_region;
   }

   void CompleteAttempt(MST_AuctionAttempt &attempt,
                        const MST_AUCTION_RESOLUTION resolution,
                        const datetime market_time)
   {
      attempt.active = false;
      attempt.state = MST_AUCTION_RESOLVED;
      attempt.resolution = resolution;
      attempt.completion_status = MST_COMPLETION_RESOLVED;
      attempt.censor_reason = MST_CENSOR_NONE;
      attempt.resolved_at = market_time;
      attempt.resolved_bar_sequence = m_bar_sequence;
      attempt.end_region = m_feature_snapshot.regional.price_region;
      attempt.end_median_sigma = m_feature_snapshot.regional.price_from_median_sigma;
      int slot = ArraySize(m_completed_attempts);
      ArrayResize(m_completed_attempts, slot + 1, 32);
      m_completed_attempts[slot] = attempt;
      m_attempts_resolved++;
      UpdateEpisodeFromAttempt(attempt);
      m_terminal_hash = MST_HashMix(m_terminal_hash, attempt.attempt_id);
      m_terminal_hash = MST_HashMix(m_terminal_hash, (ulong)((int)resolution));
   }

   void CensorAttempt(MST_AuctionAttempt &attempt,
                      const MST_CENSOR_REASON reason,
                      const datetime market_time)
   {
      attempt.active = false;
      attempt.state = MST_AUCTION_RESOLVED;
      attempt.completion_status = MST_COMPLETION_RIGHT_CENSORED;
      attempt.censor_reason = reason;
      attempt.resolution = MST_RESOLUTION_NONE;
      attempt.resolved_at = market_time;
      attempt.resolved_bar_sequence = m_bar_sequence;
      attempt.end_region = m_feature_snapshot.regional.price_region;
      attempt.end_median_sigma = m_feature_snapshot.regional.price_from_median_sigma;
      int slot = ArraySize(m_completed_attempts);
      ArrayResize(m_completed_attempts, slot + 1, 32);
      m_completed_attempts[slot] = attempt;
      m_attempts_censored++;
      m_terminal_hash = MST_HashMix(m_terminal_hash, attempt.attempt_id);
      m_terminal_hash = MST_HashMix(m_terminal_hash, (ulong)((int)reason));
   }

   bool EpisodeHasTransit(const ulong episode_id) const
   {
      return m_transits.HasActiveForEpisode(episode_id);
   }

   bool NodeHasAttempt(const ulong node_id) const
   {
      return FindAttempt(node_id) >= 0;
   }

   void CloseExpiredEpisodes(const datetime market_time)
   {
      for(int i = 0; i < ArraySize(m_episodes); i++)
      {
         if(!m_episodes[i].active || m_episodes[i].last_attempt_end_bar <= 0 ||
            NodeHasAttempt(m_episodes[i].node_id) || EpisodeHasTransit(m_episodes[i].episode_id))
            continue;
         if(m_bar_sequence - m_episodes[i].last_attempt_end_bar <= m_config.episode_gap_bars)
            continue;
         m_episodes[i].active = false;
         m_episodes[i].completion_status = MST_COMPLETION_RESOLVED;
         m_episodes[i].censor_reason = MST_CENSOR_NONE;
         if(m_episodes[i].ended_at == 0) m_episodes[i].ended_at = market_time;
         int slot = ArraySize(m_completed_episodes);
         ArrayResize(m_completed_episodes, slot + 1, 16);
         m_completed_episodes[slot] = m_episodes[i];
         m_episodes_resolved++;
         m_terminal_hash = MST_HashMix(m_terminal_hash, m_episodes[i].episode_id);
         m_terminal_hash = MST_HashMix(m_terminal_hash,
                                       (ulong)((int)m_episodes[i].resolution));
      }
   }

   void CompactInactive(void)
   {
      int write = 0;
      for(int i = 0; i < ArraySize(m_attempts); i++)
      {
         if(!m_attempts[i].active) continue;
         if(write != i) m_attempts[write] = m_attempts[i];
         write++;
      }
      if(write != ArraySize(m_attempts)) ArrayResize(m_attempts, write);
      write = 0;
      for(int i = 0; i < ArraySize(m_episodes); i++)
      {
         if(!m_episodes[i].active) continue;
         if(write != i) m_episodes[write] = m_episodes[i];
         write++;
      }
      if(write != ArraySize(m_episodes)) ArrayResize(m_episodes, write);
   }

public:
   CMstAuctionEngine(void)
   {
      MST_DefaultAuctionGrammarConfig(m_config);
      Reset();
   }

   void Init(const MST_AuctionGrammarConfig &config)
   {
      m_config = config;
      Reset();
   }

   void Reset(void)
   {
      ArrayResize(m_attempts, 0);
      ArrayResize(m_completed_attempts, 0);
      ArrayResize(m_episodes, 0);
      ArrayResize(m_completed_episodes, 0);
      ArrayResize(m_events, 0);
      ArrayResize(m_context, 0);
      m_transits.Reset();
      ArrayResize(m_memory, 0);
      ZeroMemory(m_feature_snapshot);
      m_last_closed_bar_time = 0;
      m_bar_sequence = 0;
      m_event_sequence = 0;
      m_terminal_hash = 1469598103934665603;
      m_attempts_started = 0;
      m_attempts_resolved = 0;
      m_attempts_censored = 0;
      m_episodes_started = 0;
      m_episodes_resolved = 0;
      m_episodes_censored = 0;
   }

   void BeginCycle(void)
   {
      ArrayResize(m_events, 0);
      ArrayResize(m_completed_attempts, 0);
      ArrayResize(m_completed_episodes, 0);
      ArrayResize(m_context, 0);
      m_transits.BeginCycle();
   }

   void SetFeatureSnapshot(const MST_AuctionFeatureSnapshot &snapshot)
   {
      m_feature_snapshot = snapshot;
   }

   void Observe(MST_Node &nodes[],
                const int requested_count,
                const MST_SourceRef &provenance[],
                const int requested_provenance_count,
                const double price,
                const double atr,
                const datetime market_time,
                const datetime closed_bar_time,
                const double closed_bar_price)
   {
      BeginCycle();
      if(!m_config.enabled) return;
      bool new_closed_bar = closed_bar_time > 0 && closed_bar_time > m_last_closed_bar_time;
      if(new_closed_bar)
      {
         m_last_closed_bar_time = closed_bar_time;
         m_bar_sequence++;
      }
      double safe_atr = MathMax(atr, 0.00000001);
      int node_count = MathMin(requested_count, ArraySize(nodes));
      int provenance_count = MathMin(requested_provenance_count, ArraySize(provenance));

      for(int i = 0; i < ArraySize(m_attempts); i++)
      {
         bool present = false;
         for(int n = 0; n < node_count; n++)
            if(nodes[n].valid && nodes[n].existence == MST_NODE_ACTIVE &&
               nodes[n].node_id == m_attempts[i].node_id) { present = true; break; }
         if(present) continue;
         MST_AuctionAttempt orphan = m_attempts[i];
         PushEvent(orphan, MST_AUCTION_EVENT_EXPIRE, market_time,
                   closed_bar_time, price, atr,
                   BoundaryGap(price, orphan.frozen_lower, orphan.frozen_upper) / safe_atr,
                   orphan.max_penetration_atr);
         CompleteAttempt(orphan, MST_RESOLUTION_NODE_RETIRED, market_time);
         m_attempts[i] = orphan;
      }
      m_transits.Observe(market_time, m_bar_sequence, price, m_config.max_attempt_bars,
                         m_feature_snapshot.regional);
      CollectTransitCompletions(closed_bar_time);

      for(int n = 0; n < node_count; n++)
      {
         if(!nodes[n].valid || nodes[n].existence != MST_NODE_ACTIVE) continue;
         int attempt_index = FindAttempt(nodes[n].node_id);
         int side = PriceSide(price, nodes[n].lower, nodes[n].upper);
         if(attempt_index < 0)
         {
            double gap_atr = BoundaryGap(price, nodes[n].lower, nodes[n].upper) / safe_atr;
            if(side != 0 && gap_atr <= m_config.approach_radius_atr)
               attempt_index = StartAttempt(nodes[n], nodes, node_count,
                                            provenance, provenance_count,
                                            market_time, closed_bar_time,
                                            price, atr, side);
            else
               continue;
         }

         MST_AuctionAttempt attempt = m_attempts[attempt_index];
         double delta = MathAbs(price - attempt.last_price);
         attempt.approach_path += delta;
         attempt.last_price = price;
         attempt.last_observed_at = market_time;
         if(m_feature_snapshot.regional.sigma_valid)
         {
            double regional_z = m_feature_snapshot.regional.price_from_median_sigma;
            attempt.end_region = m_feature_snapshot.regional.price_region;
            attempt.end_median_sigma = regional_z;
            attempt.min_median_sigma = MathMin(attempt.min_median_sigma, regional_z);
            attempt.max_median_sigma = MathMax(attempt.max_median_sigma, regional_z);
         }
         int frozen_side = PriceSide(price, attempt.frozen_lower, attempt.frozen_upper);
         bool inside = frozen_side == 0;
         if(inside)
         {
            attempt.inside_updates++;
            if(attempt.last_observed_at > 0)
               attempt.inside_seconds += (long)MathMax((long)(market_time -
                                        m_attempts[attempt_index].last_observed_at), 0);
         }
         double above = MathMax(price - attempt.frozen_upper, 0.0) / safe_atr;
         double below = MathMax(attempt.frozen_lower - price, 0.0) / safe_atr;
         attempt.max_above_node_atr = MathMax(attempt.max_above_node_atr, above);
         attempt.max_below_node_atr = MathMax(attempt.max_below_node_atr, below);

         bool crossed_near = attempt.direction == MST_APPROACH_FROM_BELOW ?
                             (m_attempts[attempt_index].last_price <= attempt.frozen_lower &&
                              price >= attempt.frozen_lower) :
                             (m_attempts[attempt_index].last_price >= attempt.frozen_upper &&
                              price <= attempt.frozen_upper);
         if(attempt.contact_at == 0 && (inside || crossed_near))
         {
            attempt.contact_at = market_time;
            attempt.contact_bar_sequence = m_bar_sequence;
            attempt.contact_price = price;
            attempt.state = attempt.is_retest ? MST_AUCTION_RETEST : MST_AUCTION_CONTACT;
            double net = MathAbs(price - attempt.start_price);
            attempt.approach_efficiency = attempt.approach_path > 0.0 ?
                                          net / attempt.approach_path : 1.0;
            PushEvent(attempt, MST_AUCTION_EVENT_CONTACT, market_time,
                      closed_bar_time, price, atr, 0.0, 0.0);
            if(attempt.is_retest)
               PushEvent(attempt, MST_AUCTION_EVENT_RETEST, market_time,
                         closed_bar_time, price, atr, 0.0, 0.0);
         }

         if(attempt.contact_at > 0)
         {
            double penetration = attempt.direction == MST_APPROACH_FROM_BELOW ?
                                 price - attempt.frozen_lower : attempt.frozen_upper - price;
            penetration = MathMax(penetration, 0.0);
            if(penetration > attempt.max_penetration)
            {
               bool first_penetration = attempt.max_penetration <= 0.0 && penetration > 0.0;
               attempt.max_penetration = penetration;
               attempt.max_penetration_atr = penetration / safe_atr;
               attempt.max_penetration_node = attempt.frozen_width > 0.0 ?
                                              penetration / attempt.frozen_width : 0.0;
               if(first_penetration)
                  PushEvent(attempt, MST_AUCTION_EVENT_PENETRATION, market_time,
                            closed_bar_time, price, atr, 0.0,
                            attempt.max_penetration_atr);
            }

            double far_buffer = m_config.break_buffer_atr * safe_atr;
            bool far_cross = attempt.direction == MST_APPROACH_FROM_BELOW ?
                             price > attempt.frozen_upper + far_buffer :
                             price < attempt.frozen_lower - far_buffer;
            if(attempt.is_retest && far_cross)
            {
               PushEvent(attempt, MST_AUCTION_EVENT_RETEST_FAILURE, market_time,
                         closed_bar_time, price, atr,
                         BoundaryGap(price, attempt.frozen_lower, attempt.frozen_upper) / safe_atr,
                         attempt.max_penetration_atr);
               PushEvent(attempt, MST_AUCTION_EVENT_RECLAIM, market_time,
                         closed_bar_time, price, atr, 0.0, attempt.max_penetration_atr);
               int memory = EnsureMemory(attempt.node_id);
               m_memory[memory].accepted_side = 0;
               CompleteAttempt(attempt, MST_RESOLUTION_ACCEPT_AND_FAIL_RETEST, market_time);
            }
            else if(!attempt.is_retest && !attempt.broke_far_boundary && far_cross)
            {
               attempt.broke_far_boundary = true;
               attempt.break_at = market_time;
               attempt.state = MST_AUCTION_BROKEN;
               PushEvent(attempt, MST_AUCTION_EVENT_BREAK, market_time,
                         closed_bar_time, price, atr,
                         BoundaryGap(price, attempt.frozen_lower, attempt.frozen_upper) / safe_atr,
                         attempt.max_penetration_atr);
            }

            double origin_gap = attempt.direction == MST_APPROACH_FROM_BELOW ?
                                attempt.frozen_lower - price : price - attempt.frozen_upper;
            double accepted_gap = attempt.direction == MST_APPROACH_FROM_BELOW ?
                                  price - attempt.frozen_upper : attempt.frozen_lower - price;
            if(attempt.is_retest && !far_cross && frozen_side == (int)attempt.direction &&
               origin_gap / safe_atr >= m_config.rejection_min_excursion_atr)
            {
               attempt.rejection_excursion_atr = origin_gap / safe_atr;
               PushEvent(attempt, MST_AUCTION_EVENT_HOLD, market_time,
                         closed_bar_time, price, atr,
                         attempt.rejection_excursion_atr, attempt.max_penetration_atr);
               CompleteAttempt(attempt, MST_RESOLUTION_ACCEPT_AND_HOLD_RETEST, market_time);
            }
            else if(!attempt.is_retest && !attempt.broke_far_boundary &&
                    frozen_side == (int)attempt.direction &&
                    origin_gap / safe_atr >= m_config.rejection_min_excursion_atr)
            {
               attempt.rejection_excursion_atr = origin_gap / safe_atr;
               PushEvent(attempt, MST_AUCTION_EVENT_REJECTION, market_time,
                         closed_bar_time, price, atr,
                         attempt.rejection_excursion_atr, attempt.max_penetration_atr);
               PushEvent(attempt, MST_AUCTION_EVENT_DEPARTURE, market_time,
                         closed_bar_time, price, atr,
                         attempt.rejection_excursion_atr, attempt.max_penetration_atr);
               CompleteAttempt(attempt, MST_RESOLUTION_REJECT_TO_ORIGIN, market_time);
            }
            else if(attempt.accepted && accepted_gap / safe_atr >=
                    m_config.departure_distance_atr)
            {
               PushEvent(attempt, MST_AUCTION_EVENT_DEPARTURE, market_time,
                         closed_bar_time, price, atr, accepted_gap / safe_atr,
                         attempt.max_penetration_atr);
               int memory = EnsureMemory(attempt.node_id);
               m_memory[memory].accepted_side = -(int)attempt.direction;
               m_transits.Start(attempt, nodes, node_count, market_time,
                                m_bar_sequence, price);
               CompleteAttempt(attempt, MST_RESOLUTION_ACCEPT_THROUGH_NODE, market_time);
            }
         }

         if(attempt.active && new_closed_bar && attempt.contact_at > 0)
         {
            int close_side = PriceSide(closed_bar_price,
                                       attempt.frozen_lower, attempt.frozen_upper);
            int far_side = -(int)attempt.direction;
            double far_distance = far_side > 0 ?
                                  closed_bar_price - attempt.frozen_upper :
                                  attempt.frozen_lower - closed_bar_price;
            if(attempt.broke_far_boundary && close_side == far_side &&
               far_distance / safe_atr >= m_config.acceptance_min_distance_atr)
            {
               attempt.qualified_far_closes++;
               if(!attempt.provisional_acceptance)
               {
                  attempt.provisional_acceptance = true;
                  attempt.state = MST_AUCTION_PROVISIONAL_ACCEPTANCE;
                  PushEvent(attempt, MST_AUCTION_EVENT_PROVISIONAL_ACCEPTANCE,
                            market_time, closed_bar_time, closed_bar_price, atr,
                            far_distance / safe_atr, attempt.max_penetration_atr);
               }
               if(!attempt.accepted && attempt.qualified_far_closes >=
                  MathMax(m_config.acceptance_bars, 1))
               {
                  attempt.accepted = true;
                  attempt.accepted_at = closed_bar_time;
                  attempt.state = MST_AUCTION_ACCEPTED;
                  PushEvent(attempt, MST_AUCTION_EVENT_ACCEPTANCE, market_time,
                            closed_bar_time, closed_bar_price, atr,
                            far_distance / safe_atr, attempt.max_penetration_atr);
               }
            }
            else if(attempt.broke_far_boundary && close_side == (int)attempt.direction)
            {
               double reclaim_distance = attempt.direction == MST_APPROACH_FROM_BELOW ?
                                         attempt.frozen_lower - closed_bar_price :
                                         closed_bar_price - attempt.frozen_upper;
               if(reclaim_distance / safe_atr >= m_config.reclaim_tolerance_atr)
               {
                  PushEvent(attempt, MST_AUCTION_EVENT_RECLAIM, market_time,
                            closed_bar_time, closed_bar_price, atr,
                            reclaim_distance / safe_atr, attempt.max_penetration_atr);
                  CompleteAttempt(attempt, MST_RESOLUTION_RECLAIM_AFTER_BREAK, market_time);
               }
            }
         }

         if(attempt.active && new_closed_bar &&
            m_bar_sequence - attempt.start_bar_sequence > m_config.max_attempt_bars)
         {
            PushEvent(attempt, MST_AUCTION_EVENT_EXPIRE, market_time,
                      closed_bar_time, price, atr,
                      BoundaryGap(price, attempt.frozen_lower, attempt.frozen_upper) / safe_atr,
                      attempt.max_penetration_atr);
            CompleteAttempt(attempt, MST_RESOLUTION_TIMEOUT, market_time);
         }
         m_attempts[attempt_index] = attempt;
      }

      if(new_closed_bar) CloseExpiredEpisodes(market_time);
      CompactInactive();
   }

   void Finalize(const datetime market_time,
                 const datetime bar_time,
                 const double price,
                 const double atr,
                 const MST_CENSOR_REASON reason)
   {
      BeginCycle();
      m_transits.CensorAll(market_time, m_bar_sequence, price, reason,
                           m_feature_snapshot.regional);
      CollectTransitCompletions(bar_time);
      double safe_atr = MathMax(atr, 0.00000001);
      for(int i = 0; i < ArraySize(m_attempts); i++)
      {
         if(!m_attempts[i].active) continue;
         MST_AuctionAttempt attempt = m_attempts[i];
         PushEvent(attempt, MST_AUCTION_EVENT_CENSOR, market_time, bar_time,
                   price, atr,
                   BoundaryGap(price, attempt.frozen_lower, attempt.frozen_upper) / safe_atr,
                   attempt.max_penetration_atr);
         CensorAttempt(attempt, reason, market_time);
         m_attempts[i] = attempt;
      }
      CompactInactive();
      for(int i = 0; i < ArraySize(m_episodes); i++)
      {
         if(!m_episodes[i].active) continue;
         m_episodes[i].active = false;
         m_episodes[i].completion_status = MST_COMPLETION_RIGHT_CENSORED;
         m_episodes[i].censor_reason = reason;
         m_episodes[i].ended_at = market_time;
         m_episodes[i].terminal_region = m_feature_snapshot.regional.price_region;
         int slot = ArraySize(m_completed_episodes);
         ArrayResize(m_completed_episodes, slot + 1, 16);
         m_completed_episodes[slot] = m_episodes[i];
         m_episodes_censored++;
         m_terminal_hash = MST_HashMix(m_terminal_hash, m_episodes[i].episode_id);
         m_terminal_hash = MST_HashMix(m_terminal_hash, (ulong)((int)reason));
      }
      CompactInactive();
   }

   int EventCount(void) const { return ArraySize(m_events); }
   int CompletedAttemptCount(void) const { return ArraySize(m_completed_attempts); }
   int CompletedEpisodeCount(void) const { return ArraySize(m_completed_episodes); }
   int CompletedTransitCount(void) const { return m_transits.CompletedCount(); }
   int ContextCount(void) const { return ArraySize(m_context); }
   int ActiveAttemptCount(void) const { return ArraySize(m_attempts); }
   int ActiveEpisodeCount(void) const { return ArraySize(m_episodes); }
   int ActiveTransitCount(void) const { return m_transits.ActiveCount(); }
   ulong TerminalHash(void) const { return m_terminal_hash; }
   ulong EventSequence(void) const { return m_event_sequence; }

   bool GetEvent(const int index, MST_AuctionEvent &out) const
   {
      if(index < 0 || index >= ArraySize(m_events)) return false;
      out = m_events[index]; return true;
   }
   bool GetCompletedAttempt(const int index, MST_AuctionAttempt &out) const
   {
      if(index < 0 || index >= ArraySize(m_completed_attempts)) return false;
      out = m_completed_attempts[index]; return true;
   }
   bool GetCompletedEpisode(const int index, MST_AuctionEpisode &out) const
   {
      if(index < 0 || index >= ArraySize(m_completed_episodes)) return false;
      out = m_completed_episodes[index]; return true;
   }
   bool GetContext(const int index, MST_AuctionContextReceipt &out) const
   {
      if(index < 0 || index >= ArraySize(m_context)) return false;
      out = m_context[index]; return true;
   }
   bool GetCompletedTransit(const int index, MST_TransitTrack &out) const
   {
      return m_transits.GetCompleted(index, out);
   }
   MST_AUCTION_STATE StateForNode(const ulong node_id) const
   {
      int attempt = FindAttempt(node_id);
      return attempt >= 0 ? m_attempts[attempt].state : MST_AUCTION_IDLE;
   }
   void GetReading(MST_AuctionReading &out) const
   {
      ZeroMemory(out);
      out.active_attempts = ArraySize(m_attempts);
      out.active_episodes = ArraySize(m_episodes);
      out.active_transits = m_transits.ActiveCount();
      out.event_count = ArraySize(m_events);
      out.completed_attempts = ArraySize(m_completed_attempts);
      out.completed_episodes = ArraySize(m_completed_episodes);
      out.event_sequence = m_event_sequence;
      out.terminal_hash = m_terminal_hash;
   }
   void GetInvariantReading(MST_AuctionInvariantReading &out) const
   {
      ZeroMemory(out);
      out.attempts_started = m_attempts_started;
      out.attempts_resolved = m_attempts_resolved;
      out.attempts_censored = m_attempts_censored;
      out.episodes_started = m_episodes_started;
      out.episodes_resolved = m_episodes_resolved;
      out.episodes_censored = m_episodes_censored;
      out.transits_started = m_transits.StartedTotal();
      out.transits_resolved = m_transits.ResolvedTotal();
      out.transits_censored = m_transits.CensoredTotal();
      out.events_emitted = (long)m_event_sequence;
      out.active_attempts = ArraySize(m_attempts);
      out.active_episodes = ArraySize(m_episodes);
      out.active_transits = m_transits.ActiveCount();
      bool attempts_ok = out.attempts_started == out.attempts_resolved +
                         out.attempts_censored + out.active_attempts;
      bool episodes_ok = out.episodes_started == out.episodes_resolved +
                         out.episodes_censored + out.active_episodes;
      bool transits_ok = out.transits_started == out.transits_resolved +
                         out.transits_censored + out.active_transits;
      out.balanced = attempts_ok && episodes_ok && transits_ok;
      ulong hash = m_terminal_hash;
      hash = MST_HashMix(hash, (ulong)out.attempts_started);
      hash = MST_HashMix(hash, (ulong)out.attempts_resolved);
      hash = MST_HashMix(hash, (ulong)out.attempts_censored);
      hash = MST_HashMix(hash, (ulong)out.episodes_started);
      hash = MST_HashMix(hash, (ulong)out.episodes_resolved);
      hash = MST_HashMix(hash, (ulong)out.episodes_censored);
      hash = MST_HashMix(hash, (ulong)out.transits_started);
      hash = MST_HashMix(hash, (ulong)out.transits_resolved);
      hash = MST_HashMix(hash, (ulong)out.transits_censored);
      out.receipt_hash = hash;
   }
};

#endif // __KITT_MASTER_AUCTION_ENGINE_MQH__
