#ifndef NORTHSTAR_MARKET_OBJECT_TYPES_MQH
#define NORTHSTAR_MARKET_OBJECT_TYPES_MQH

#define MOR_RESEARCH_GENERATION 3
#define MOR_SOURCE_AUCTION_GENERATION 2
#define MOR_RAW_SCHEMA_VERSION 1
#define MOR_COMPRESSION_GRAMMAR_VERSION 2
#define MOR_EXPANSION_GRAMMAR_VERSION 1
#define MOR_CONTRACT_ID "NORTHSTAR_RG3_RAW_MARKET_OBJECTS_V1"
#define MOR_NULL "\\N"

enum MOR_TERMINAL_REASON
  {
   MOR_TERMINAL_NONE=0,
   MOR_COMPRESSION_CLOSED_UP=1,
   MOR_COMPRESSION_CLOSED_DOWN=2,
   MOR_COMPRESSION_EXPIRED=3,
   MOR_EXPANSION_HANDOFF=4,
   MOR_HORIZON_CENSORED=5,
   MOR_TEST_END_CENSORED=6,
   MOR_SHUTDOWN_CENSORED=7,
   MOR_DATA_GAP_CENSORED=8
  };

enum MOR_RELATION_KIND
  {
   MOR_RELATION_ORIGINATES_FROM=1,
   MOR_RELATION_HANDS_OFF_TO=2,
   MOR_RELATION_CONTACTS_NODE=3
  };

struct MOR_RunIdentity
  {
   string run_key;
   string invocation_id;
   string canonical_instrument;
   string broker_symbol;
   string data_source_id;
   string data_fingerprint;
   ENUM_TIMEFRAMES timeframe;
   datetime window_start;
   datetime window_end;
   ulong config_hash;
  };

struct MOR_CompressionObject
  {
   long id;
   datetime start_time;
   datetime confirm_time;
   datetime terminal_time;
   int terminal_reason;
   int direction;
   int escape_count;
   int sample_count;
   double seed_top;
   double seed_bottom;
   double seed_mid;
   double terminal_contain_top;
   double terminal_contain_bottom;
   double terminal_contain_mid;
  };

struct MOR_ExpansionObject
  {
   long id;
   long origin_compression_id;
   long destination_compression_id;
   datetime origin_time;
   datetime terminal_time;
   int terminal_reason;
   int direction;
   int sample_count;
   double origin_seed_frontier;
   double origin_containment_frontier;
   double origin_terminal_price;
   double origin_mid;
   double origin_atr;
   double terminal_price;
   double max_displacement;
   double opposite_displacement;
   double return_depth;
   double close_path_length;
  };

struct MOR_Balance
  {
   int compressions_started;
   int compressions_completed;
   int compressions_censored;
   int compressions_active;
   int expansions_started;
   int expansions_completed;
   int expansions_censored;
   int expansions_active;
   int compression_samples;
   int expansion_samples;
   int relations;
  };

string MOR_Bool(const bool value) { return(value ? "1" : "0"); }

string MOR_Number(const double value,const int digits=16)
  {
   if(value==EMPTY_VALUE || !MathIsValidNumber(value)) return(MOR_NULL);
   return(DoubleToString(value,digits));
  }

string MOR_Time(const datetime value)
  {
   if(value<=0) return(MOR_NULL);
   return(IntegerToString((long)value));
  }

string MOR_TerminalName(const int value)
  {
   if(value==MOR_COMPRESSION_CLOSED_UP) return("CLOSED_UP");
   if(value==MOR_COMPRESSION_CLOSED_DOWN) return("CLOSED_DOWN");
   if(value==MOR_COMPRESSION_EXPIRED) return("EXPIRED");
   if(value==MOR_EXPANSION_HANDOFF) return("HANDOFF_TO_COMPRESSION");
   if(value==MOR_HORIZON_CENSORED) return("HORIZON_CENSORED");
   if(value==MOR_TEST_END_CENSORED) return("TEST_END_CENSORED");
   if(value==MOR_SHUTDOWN_CENSORED) return("SHUTDOWN_CENSORED");
   if(value==MOR_DATA_GAP_CENSORED) return("DATA_GAP_CENSORED");
   return("NONE");
  }

bool MOR_IsCensor(const int value)
  {
   return(value==MOR_HORIZON_CENSORED || value==MOR_TEST_END_CENSORED ||
          value==MOR_SHUTDOWN_CENSORED || value==MOR_DATA_GAP_CENSORED);
  }

#endif
