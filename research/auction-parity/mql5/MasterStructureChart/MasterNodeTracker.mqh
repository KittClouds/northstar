//+------------------------------------------------------------------+
//| MasterNodeTracker.mqh                                            |
//| Stable structural identity, lineage, and existence only.         |
//+------------------------------------------------------------------+
#ifndef __KITT_MASTER_STRUCTURE_NODE_TRACKER_MQH__
#define __KITT_MASTER_STRUCTURE_NODE_TRACKER_MQH__

#include "MasterTypes.mqh"

class CMstNodeTracker
{
private:
   MST_Node                m_tracks[];
   MST_NodeEvent           m_events[];
   MST_NodeLifecycleConfig m_config;
   ulong                   m_birth_sequence;

   bool GeometryCompatible(const MST_Node &track,
                           const MST_Node &candidate,
                           const double atr) const
   {
      if(!track.valid || track.existence == MST_NODE_RETIRED || !candidate.valid)
         return false;
      double safe_atr = MathMax(atr, 0.00000001);
      double gap = 0.0;
      if(candidate.lower > track.upper) gap = candidate.lower - track.upper;
      else if(track.lower > candidate.upper) gap = track.lower - candidate.upper;
      double center_gap = MathAbs(candidate.price - track.price);
      return (track.family_mask & candidate.family_mask) != 0 &&
             gap / safe_atr <= m_config.match_distance_atr &&
             center_gap / safe_atr <= m_config.match_distance_atr * 2.0;
   }

   bool LineageCompatible(const MST_Node &track,
                          const MST_Node &candidate,
                          const double atr) const
   {
      if(!GeometryCompatible(track, candidate, atr)) return false;
      double overlap = MathMin(track.upper, candidate.upper) -
                       MathMax(track.lower, candidate.lower);
      if(overlap >= 0.0) return true;
      return MathAbs(track.price - candidate.price) /
             MathMax(atr, 0.00000001) <= m_config.match_distance_atr * 0.5;
   }

   double MatchScore(const MST_Node &track,
                     const MST_Node &candidate,
                     const double atr) const
   {
      if(!GeometryCompatible(track, candidate, atr)) return -DBL_MAX;
      double score = track.evidence_id == candidate.evidence_id ? 1000.0 : 0.0;
      double safe_atr = MathMax(atr, 0.00000001);
      score += 10.0 - MathMin(MathAbs(track.price - candidate.price) / safe_atr, 10.0);
      double overlap = MathMax(0.0, MathMin(track.upper, candidate.upper) -
                                    MathMax(track.lower, candidate.lower));
      double span = MathMax(MathMax(track.upper, candidate.upper) -
                            MathMin(track.lower, candidate.lower), safe_atr * 0.000001);
      score += 10.0 * overlap / span;
      ulong common = track.family_mask & candidate.family_mask;
      for(int bit = 1; bit < MST_MAX_FAMILIES; bit++)
         if((common & ((ulong)1 << bit)) != 0) score += 1.0;
      if(track.role == candidate.role) score += 1.0;
      return score;
   }

   ulong NewStableId(const MST_Node &candidate, const datetime market_time)
   {
      ulong id = MST_HashMix(candidate.evidence_id, (ulong)market_time);
      id = MST_HashMix(id, ++m_birth_sequence);
      for(int retry = 0; retry < 8; retry++)
      {
         bool collision = false;
         for(int i = 0; i < ArraySize(m_tracks); i++)
            if(m_tracks[i].node_id == id) { collision = true; break; }
         if(!collision && id != 0) return id;
         id = MST_HashMix(id, (ulong)(retry + 1));
      }
      return id == 0 ? 1 : id;
   }

   void PushEvent(const MST_NODE_EVENT_KIND kind,
                  const MST_Node &node,
                  const ulong related_id,
                  const MST_NODE_EXISTENCE_STATE previous_state,
                  const datetime market_time,
                  const double reference_price)
   {
      int index = ArraySize(m_events);
      ArrayResize(m_events, index + 1, 32);
      m_events[index].kind = kind;
      m_events[index].node_id = node.node_id;
      m_events[index].related_node_id = related_id;
      m_events[index].evidence_id = node.evidence_id;
      m_events[index].market_time = market_time;
      m_events[index].reference_price = reference_price;
      m_events[index].lower = node.lower;
      m_events[index].price = node.price;
      m_events[index].upper = node.upper;
      m_events[index].previous_state = previous_state;
      m_events[index].current_state = node.existence;
      m_events[index].attempt_count = node.attempt_count;
      m_events[index].revision = node.revision;
   }

   int FindTrack(const ulong node_id) const
   {
      for(int i = 0; i < ArraySize(m_tracks); i++)
         if(m_tracks[i].node_id == node_id) return i;
      return -1;
   }

   void PreserveHistory(const MST_Node &old_node, MST_Node &fresh_node)
   {
      fresh_node.node_id = old_node.node_id;
      fresh_node.existence = old_node.existence;
      fresh_node.created_at = old_node.created_at;
      fresh_node.last_seen_at = old_node.last_seen_at;
      fresh_node.state_changed_at = old_node.state_changed_at;
      fresh_node.attempt_count = old_node.attempt_count;
      fresh_node.revision = old_node.revision + 1;
      fresh_node.primary_parent_id = old_node.primary_parent_id;
      fresh_node.secondary_parent_id = old_node.secondary_parent_id;
      fresh_node.missed_rebuilds = 0;
   }

public:
   CMstNodeTracker(void)
   {
      MST_DefaultNodeLifecycleConfig(m_config);
      m_birth_sequence = 0;
   }

   void Init(const MST_NodeLifecycleConfig &config)
   {
      m_config = config;
      Reset();
   }

   void Reset(void)
   {
      ArrayResize(m_tracks, 0);
      ArrayResize(m_events, 0);
      m_birth_sequence = 0;
   }

   void BeginCycle(void) { ArrayResize(m_events, 0); }
   int EventCount(void) const { return ArraySize(m_events); }
   int TrackCount(void) const { return ArraySize(m_tracks); }

   bool GetEvent(const int index, MST_NodeEvent &event) const
   {
      if(index < 0 || index >= ArraySize(m_events)) return false;
      event = m_events[index];
      return true;
   }

   int Reconcile(MST_Node &candidates[],
                 const int requested_count,
                 const double atr,
                 const datetime market_time,
                 const double reference_price)
   {
      BeginCycle();
      int count = MathMin(requested_count, ArraySize(candidates));
      int old_count = ArraySize(m_tracks);
      bool used[];
      ArrayResize(used, old_count);
      ArrayInitialize(used, false);

      for(int i = 0; i < count; i++)
      {
         if(!candidates[i].valid) continue;
         candidates[i].evidence_id = candidates[i].node_id;
         int best = -1;
         double best_score = -DBL_MAX;
         for(int t = 0; t < old_count; t++)
         {
            if(used[t]) continue;
            double score = MatchScore(m_tracks[t], candidates[i], atr);
            if(score > best_score || (score == best_score && best >= 0 &&
                                      m_tracks[t].node_id < m_tracks[best].node_id))
            {
               best = t;
               best_score = score;
            }
         }

         if(best >= 0 && best_score > -DBL_MAX / 2.0)
         {
            ulong previous_evidence = m_tracks[best].evidence_id;
            MST_Node fresh = candidates[i];
            PreserveHistory(m_tracks[best], fresh);
            fresh.last_seen_at = market_time;
            fresh.existence = MST_NODE_ACTIVE;
            candidates[i] = fresh;
            m_tracks[best] = fresh;
            used[best] = true;
            if(previous_evidence != fresh.evidence_id)
               PushEvent(MST_EVENT_EVIDENCE_CHANGED, fresh, 0,
                         fresh.existence, market_time, reference_price);
         }
         else
         {
            MST_Node fresh = candidates[i];
            fresh.node_id = NewStableId(fresh, market_time);
            fresh.existence = MST_NODE_ACTIVE;
            fresh.created_at = market_time;
            fresh.last_seen_at = market_time;
            fresh.state_changed_at = market_time;
            fresh.revision = 1;
            int slot = ArraySize(m_tracks);
            ArrayResize(m_tracks, slot + 1, 64);
            m_tracks[slot] = fresh;
            candidates[i] = fresh;
            PushEvent(MST_EVENT_CREATED, fresh, 0, MST_NODE_NEW,
                      market_time, reference_price);
            PushEvent(MST_EVENT_ACTIVATED, fresh, 0, MST_NODE_NEW,
                      market_time, reference_price);
         }
      }

      for(int i = 0; i < count; i++)
      {
         if(!candidates[i].valid) continue;
         ulong first_parent = 0;
         for(int t = 0; t < old_count; t++)
         {
            if(m_tracks[t].node_id == candidates[i].node_id) continue;
            if(!LineageCompatible(m_tracks[t], candidates[i], atr)) continue;
            if(first_parent == 0) first_parent = m_tracks[t].node_id;
            PushEvent(MST_EVENT_MERGE, candidates[i], m_tracks[t].node_id,
                      candidates[i].existence, market_time, reference_price);
         }
         if(first_parent != 0)
         {
            candidates[i].primary_parent_id = first_parent;
            int track = FindTrack(candidates[i].node_id);
            if(track >= 0) m_tracks[track].primary_parent_id = first_parent;
         }
      }

      for(int t = 0; t < old_count; t++)
      {
         int child_count = 0;
         for(int i = 0; i < count; i++)
            if(candidates[i].valid && candidates[i].node_id != m_tracks[t].node_id &&
               LineageCompatible(m_tracks[t], candidates[i], atr)) child_count++;
         if(child_count < 2) continue;
         for(int i = 0; i < count; i++)
            if(candidates[i].valid && candidates[i].node_id != m_tracks[t].node_id &&
               LineageCompatible(m_tracks[t], candidates[i], atr))
               PushEvent(MST_EVENT_SPLIT, candidates[i], m_tracks[t].node_id,
                         candidates[i].existence, market_time, reference_price);
      }

      for(int t = 0; t < old_count; t++)
      {
         if(used[t]) continue;
         m_tracks[t].missed_rebuilds++;
         if(m_tracks[t].missed_rebuilds < MathMax(m_config.retire_after_rebuilds, 1))
            continue;
         MST_NODE_EXISTENCE_STATE previous = m_tracks[t].existence;
         m_tracks[t].existence = MST_NODE_RETIRED;
         m_tracks[t].state_changed_at = market_time;
         PushEvent(MST_EVENT_RETIRE, m_tracks[t], 0, previous,
                   market_time, reference_price);
      }

      int write = 0;
      for(int t = 0; t < ArraySize(m_tracks); t++)
      {
         if(m_tracks[t].existence == MST_NODE_RETIRED) continue;
         if(write != t) m_tracks[write] = m_tracks[t];
         write++;
      }
      if(write != ArraySize(m_tracks)) ArrayResize(m_tracks, write);
      return count;
   }

   void SyncAttemptCounts(const MST_Node &nodes[], const int requested_count)
   {
      int count = MathMin(requested_count, ArraySize(nodes));
      for(int i = 0; i < count; i++)
      {
         int track = FindTrack(nodes[i].node_id);
         if(track >= 0) m_tracks[track].attempt_count = nodes[i].attempt_count;
      }
   }
};

#endif // __KITT_MASTER_STRUCTURE_NODE_TRACKER_MQH__
