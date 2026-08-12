//+------------------------------------------------------------------+
//| MasterLogger.mqh                                                 |
//| Append-only node and input receipts in the terminal common area. |
//+------------------------------------------------------------------+
#ifndef __KITT_MASTER_STRUCTURE_LOGGER_MQH__
#define __KITT_MASTER_STRUCTURE_LOGGER_MQH__

#include "MasterTypes.mqh"

class CMstResearchLogger
{
private:
   bool            m_enabled;
   int             m_nodes_handle;
   int             m_inputs_handle;
   int             m_events_handle;
   int             m_flush_interval;
   int             m_pending_snapshots;
   ulong           m_last_snapshot_hash;
   string          m_session_id;

   string SafeToken(string value)
   {
      StringReplace(value, "\\", "_");
      StringReplace(value, "/", "_");
      StringReplace(value, ":", "_");
      StringReplace(value, " ", "_");
      StringReplace(value, ".", "_");
      return value;
   }

   int OpenAppend(const string filename, const bool nodes_file)
   {
      int handle = FileOpen(filename,
                            FILE_READ|FILE_WRITE|FILE_CSV|FILE_ANSI|FILE_COMMON,
                            '\t');
      if(handle == INVALID_HANDLE)
         return INVALID_HANDLE;

      if(FileSize(handle) == 0)
      {
         if(nodes_file)
         {
            FileWrite(handle,
                      "schema", "session", "snapshot", "generation", "market_time",
                      "symbol", "timeframe", "reference_price", "atr", "node_id",
                      "evidence_id", "existence_state", "attempt_count", "interaction_state",
                      "revision",
                      "cluster_id", "node_role", "lower", "price", "upper", "width_atr",
                      "distance_atr", "member_count", "family_count", "family_mask",
                      "developing_count", "frozen_count", "oldest_source_time",
                      "newest_update_time", "update_us", "structural_region",
                      "median_distance_atr", "median_distance_sigma",
                      "cog_distance_sigma", "width_sigma", "contains_cog");
         }
         else
         {
            FileWrite(handle,
                      "schema", "session", "snapshot", "generation", "market_time",
                      "symbol", "timeframe", "cluster_id", "source_key", "producer",
                      "producer_instance", "local_id", "family", "source_kind", "role",
                      "lower", "price", "upper", "normalized_price", "width_atr",
                      "state", "developing", "frozen", "touches", "rejections",
                      "reclaims", "acceptance_bars", "mass", "mass_share");
         }
      }
      FileSeek(handle, 0, SEEK_END);
      return handle;
   }

   int OpenEvents(const string filename)
   {
      int handle = FileOpen(filename,
                            FILE_READ|FILE_WRITE|FILE_CSV|FILE_ANSI|FILE_COMMON,
                            '\t');
      if(handle == INVALID_HANDLE) return INVALID_HANDLE;
      if(FileSize(handle) == 0)
         FileWrite(handle, "schema", "session", "market_time", "symbol",
                   "timeframe", "event", "node_id", "related_node_id",
                   "evidence_id", "previous_state", "current_state",
                   "reference_price", "lower", "price", "upper",
                   "attempt_count", "revision");
      FileSeek(handle, 0, SEEK_END);
      return handle;
   }

public:
   CMstResearchLogger(void)
   {
      m_enabled = false;
      m_nodes_handle = INVALID_HANDLE;
      m_inputs_handle = INVALID_HANDLE;
      m_events_handle = INVALID_HANDLE;
      m_flush_interval = 10;
      m_pending_snapshots = 0;
      m_last_snapshot_hash = 0;
      m_session_id = "";
   }

   bool Init(const bool enabled,
             const string symbol,
             const ENUM_TIMEFRAMES timeframe,
             const string instance_tag,
             const int flush_interval = 10)
   {
      Close();
      m_enabled = enabled;
      m_flush_interval = MathMax(flush_interval, 1);
      m_pending_snapshots = 0;
      m_last_snapshot_hash = 0;
      if(!m_enabled)
         return true;

      string stamp = TimeToString(TimeCurrent(), TIME_DATE|TIME_MINUTES|TIME_SECONDS);
      m_session_id = SafeToken(symbol) + "_" + SafeToken(EnumToString(timeframe)) +
                     "_" + SafeToken(instance_tag) + "_" + SafeToken(stamp);
      string stem = "MasterStructure_" + SafeToken(symbol) + "_" +
                    SafeToken(EnumToString(timeframe)) + "_" + SafeToken(instance_tag) + "_v5";
      m_nodes_handle = OpenAppend(stem + "_nodes.tsv", true);
      m_inputs_handle = OpenAppend(stem + "_inputs.tsv", false);
      m_events_handle = OpenEvents(stem + "_events.tsv");
      if(m_nodes_handle == INVALID_HANDLE || m_inputs_handle == INVALID_HANDLE ||
         m_events_handle == INVALID_HANDLE)
      {
         Close();
         m_enabled = false;
         return false;
      }
      return true;
   }

   bool WriteEvents(const MST_ControllerReading &reading,
                    const MST_NodeEvent &events[],
                    const int requested_count)
   {
      if(!m_enabled) return true;
      if(m_events_handle == INVALID_HANDLE) return false;
      int count = MathMin(requested_count, ArraySize(events));
      for(int i = 0; i < count; i++)
      {
         FileWrite(m_events_handle, IntegerToString(MST_SCHEMA_VERSION), m_session_id,
                   TimeToString(events[i].market_time, TIME_DATE|TIME_MINUTES|TIME_SECONDS),
                   reading.symbol, EnumToString(reading.timeframe),
                   MST_NodeEventName(events[i].kind),
                   StringFormat("%I64u", events[i].node_id),
                   StringFormat("%I64u", events[i].related_node_id),
                   StringFormat("%I64u", events[i].evidence_id),
                   MST_NodeStateName(events[i].previous_state),
                   MST_NodeStateName(events[i].current_state),
                   DoubleToString(events[i].reference_price, _Digits),
                   DoubleToString(events[i].lower, _Digits),
                   DoubleToString(events[i].price, _Digits),
                   DoubleToString(events[i].upper, _Digits),
                   IntegerToString(events[i].attempt_count),
                   IntegerToString(events[i].revision));
      }
      return true;
   }

   bool WriteSnapshot(const MST_ControllerReading &reading,
                      const MST_Level &levels[],
                      const int requested_level_count,
                      const int &labels[],
                      const MST_Node &nodes[],
                      const int requested_node_count)
   {
      if(!m_enabled)
         return true;
      if(m_nodes_handle == INVALID_HANDLE || m_inputs_handle == INVALID_HANDLE)
         return false;
      if(reading.snapshot_hash == m_last_snapshot_hash)
         return true;

      string snapshot = StringFormat("%I64u", reading.snapshot_hash);
      string generation = StringFormat("%I64u", reading.generation);
      string market_time = TimeToString(reading.market_time, TIME_DATE|TIME_MINUTES|TIME_SECONDS);
      int node_count = MathMin(requested_node_count, ArraySize(nodes));
      for(int i = 0; i < node_count; i++)
      {
         if(!nodes[i].valid) continue;
         FileWrite(m_nodes_handle,
                   IntegerToString(MST_SCHEMA_VERSION), m_session_id, snapshot, generation,
                   market_time, reading.symbol, EnumToString(reading.timeframe),
                   DoubleToString(reading.reference_price, _Digits),
                   DoubleToString(reading.atr, _Digits),
                   StringFormat("%I64u", nodes[i].node_id),
                   StringFormat("%I64u", nodes[i].evidence_id),
                   MST_NodeStateName(nodes[i].existence),
                   IntegerToString(nodes[i].attempt_count),
                   IntegerToString(nodes[i].interaction_state),
                   IntegerToString(nodes[i].revision),
                   IntegerToString(nodes[i].cluster_id), IntegerToString((int)nodes[i].role),
                   DoubleToString(nodes[i].lower, _Digits),
                   DoubleToString(nodes[i].price, _Digits),
                   DoubleToString(nodes[i].upper, _Digits),
                   DoubleToString(nodes[i].width_atr, 8),
                   DoubleToString(nodes[i].distance_atr, 8),
                   IntegerToString(nodes[i].member_count),
                   IntegerToString(nodes[i].family_count),
                   StringFormat("%I64u", nodes[i].family_mask),
                   IntegerToString(nodes[i].developing_count),
                   IntegerToString(nodes[i].frozen_count),
                   TimeToString(nodes[i].oldest_source_time, TIME_DATE|TIME_MINUTES|TIME_SECONDS),
                   TimeToString(nodes[i].newest_update_time, TIME_DATE|TIME_MINUTES|TIME_SECONDS),
                   StringFormat("%I64u", reading.update_microseconds),
                   MST_RegionName(nodes[i].structural_region),
                   DoubleToString(nodes[i].median_distance_atr, 8),
                   DoubleToString(nodes[i].median_distance_sigma, 8),
                   DoubleToString(nodes[i].cog_distance_sigma, 8),
                   DoubleToString(nodes[i].width_sigma, 8),
                   IntegerToString(nodes[i].contains_cog ? 1 : 0));
      }

      int level_count = MathMin(requested_level_count, ArraySize(levels));
      level_count = MathMin(level_count, ArraySize(labels));
      for(int i = 0; i < level_count; i++)
      {
         FileWrite(m_inputs_handle,
                   IntegerToString(MST_SCHEMA_VERSION), m_session_id, snapshot, generation,
                   market_time, reading.symbol, EnumToString(reading.timeframe),
                   IntegerToString(labels[i]), StringFormat("%I64u", levels[i].source_key),
                   MST_ProducerName(levels[i].producer),
                   IntegerToString(levels[i].producer_instance),
                   StringFormat("%I64u", levels[i].local_id),
                   MST_FamilyName(levels[i].family), IntegerToString(levels[i].source_kind),
                   IntegerToString((int)levels[i].role),
                   DoubleToString(levels[i].lower, _Digits),
                   DoubleToString(levels[i].price, _Digits),
                   DoubleToString(levels[i].upper, _Digits),
                   DoubleToString(levels[i].normalized_price, 8),
                   DoubleToString(levels[i].width_atr, 8),
                   IntegerToString((int)levels[i].state),
                   IntegerToString(levels[i].developing ? 1 : 0),
                   IntegerToString(levels[i].frozen_geometry ? 1 : 0),
                   IntegerToString(levels[i].touches),
                   IntegerToString(levels[i].rejections),
                   IntegerToString(levels[i].reclaims),
                   IntegerToString(levels[i].acceptance_bars),
                   DoubleToString(levels[i].mass, 4),
                   DoubleToString(levels[i].mass_share, 8));
      }

      m_last_snapshot_hash = reading.snapshot_hash;
      m_pending_snapshots++;
      if(m_pending_snapshots >= m_flush_interval)
      {
         FileFlush(m_nodes_handle);
         FileFlush(m_inputs_handle);
         m_pending_snapshots = 0;
      }
      return true;
   }

   void Close(void)
   {
      if(m_nodes_handle != INVALID_HANDLE)
      {
         FileFlush(m_nodes_handle);
         FileClose(m_nodes_handle);
         m_nodes_handle = INVALID_HANDLE;
      }
      if(m_inputs_handle != INVALID_HANDLE)
      {
         FileFlush(m_inputs_handle);
         FileClose(m_inputs_handle);
         m_inputs_handle = INVALID_HANDLE;
      }
      if(m_events_handle != INVALID_HANDLE)
      {
         FileFlush(m_events_handle);
         FileClose(m_events_handle);
         m_events_handle = INVALID_HANDLE;
      }
   }
};

#endif // __KITT_MASTER_STRUCTURE_LOGGER_MQH__
