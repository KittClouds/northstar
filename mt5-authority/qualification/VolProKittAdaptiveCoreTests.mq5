//+------------------------------------------------------------------+
//| VolProKittAdaptiveCoreTests.mq5                                 |
//| Deterministic Strategy Tester checks for adaptive core seams.    |
//+------------------------------------------------------------------+
#property strict

#include "VolProKittAdaptiveCore.mqh"

bool TestMultimodalClusterSelection(int &selected, double &score)
{
   const int count = 360;
   double prices[];
   double volumes[];
   ArrayResize(prices, count);
   ArrayResize(volumes, count);
   for(int i = 0; i < count; i++)
   {
      int group = i % 3;
      int step = (i / 3) % 17;
      double center = 100.0 + group * 10.0;
      prices[i] = center + (step - 8) * 0.015;
      volumes[i] = 100.0 + group * 25.0 + (i % 7);
   }
   selected = KVA_SelectClusterCount(prices, volumes, count, 0, score);
   return selected == 3;
}

bool TestStableObjectMatching(int &first_id, int &second_id, int &new_id)
{
   KVA_CLUSTER previous[];
   KVA_CLUSTER current[];
   ArrayResize(previous, 2);
   ArrayResize(current, 3);
   for(int i = 0; i < 2; i++)
   {
      ZeroMemory(previous[i]);
      previous[i].valid = true;
      previous[i].object_id = i + 4;
      previous[i].centroid = 100.0 + i * 10.0;
      previous[i].low = previous[i].centroid - 1.0;
      previous[i].high = previous[i].centroid + 1.0;
      previous[i].mass_share = 0.45;
   }
   for(int i = 0; i < 3; i++)
   {
      ZeroMemory(current[i]);
      current[i].valid = true;
      current[i].object_id = -1;
      current[i].centroid = 100.1 + i * 5.0;
      current[i].low = current[i].centroid - 1.0;
      current[i].high = current[i].centroid + 1.0;
      current[i].mass_share = (i == 1) ? 0.10 : 0.45;
   }
   KVA_AssignStableObjects(previous, 2, current, 3);
   first_id = current[0].object_id;
   new_id = current[1].object_id;
   second_id = current[2].object_id;
   return first_id == 4 && second_id == 5
       && new_id >= 0 && new_id != first_id && new_id != second_id;
}

bool TestProfileHysteresis(ulong &generation, int &pending_count)
{
   KVA_PROFILE active;
   KVA_PROFILE pending;
   KVA_PROFILE candidate;
   ZeroMemory(active);
   ZeroMemory(pending);
   ZeroMemory(candidate);
   candidate.valid = true;
   candidate.lookback = 96;
   candidate.clusters = 3;
   candidate.velocity_bars = 8;
   candidate.confidence = 0.80;
   bool first = KVA_ConsiderProfile(candidate, 100, active, pending, pending_count);
   if(!first || active.generation != 1)
      return false;

   candidate.lookback = 128;
   bool commit_one = KVA_ConsiderProfile(candidate, 200, active, pending, pending_count);
   bool commit_two = KVA_ConsiderProfile(candidate, 300, active, pending, pending_count);
   bool commit_three = KVA_ConsiderProfile(candidate, 400, active, pending, pending_count);
   generation = active.generation;
   return !commit_one && !commit_two && commit_three
       && active.lookback == 128 && active.generation == 2;
}

int OnInit()
{
   int selected = 0;
   double score = 0.0;
   int first_id = -1, second_id = -1, new_id = -1;
   ulong generation = 0;
   int pending_count = 0;
   bool cluster_ok = TestMultimodalClusterSelection(selected, score);
   bool identity_ok = TestStableObjectMatching(first_id, second_id, new_id);
   bool hysteresis_ok = TestProfileHysteresis(generation, pending_count);
   PrintFormat("KVA_CORE_TEST cluster_ok=%d selected=%d score=%.6f identity_ok=%d first_id=%d second_id=%d new_id=%d hysteresis_ok=%d generation=%I64u pending=%d",
               cluster_ok, selected, score, identity_ok,
               first_id, second_id, new_id,
               hysteresis_ok, generation, pending_count);
   if(!cluster_ok || !identity_ok || !hysteresis_ok)
      return INIT_FAILED;
   return INIT_SUCCEEDED;
}

void OnTick()
{
}

//+------------------------------------------------------------------+
