#ifndef NORTHSTAR_MARKET_OBJECT_LOGGER_MQH
#define NORTHSTAR_MARKET_OBJECT_LOGGER_MQH

#include "MarketObjectTypes.mqh"

class CMorRawLogger
  {
private:
   bool m_enabled;
   int m_flush_every;
   int m_pending;
   MOR_RunIdentity m_identity;
   int m_runs,m_co,m_cs,m_ce,m_eo,m_es,m_ee,m_rel,m_ss;

   int Open(const string stem,const string dataset)
     {
      return(FileOpen(stem+"_"+dataset+".tsv",FILE_WRITE|FILE_CSV|FILE_ANSI|FILE_COMMON,'\t'));
     }

   void FlushIfNeeded(void)
     {
      m_pending++;
      if(m_pending<m_flush_every) return;
      int handles[]={m_runs,m_co,m_cs,m_ce,m_eo,m_es,m_ee,m_rel,m_ss};
      for(int i=0;i<ArraySize(handles);i++) if(handles[i]!=INVALID_HANDLE) FileFlush(handles[i]);
      m_pending=0;
     }

   void CloseOne(int &handle)
     {
      if(handle!=INVALID_HANDLE) { FileFlush(handle); FileClose(handle); handle=INVALID_HANDLE; }
     }

public:
   CMorRawLogger(void):m_enabled(false),m_flush_every(10),m_pending(0),
      m_runs(INVALID_HANDLE),m_co(INVALID_HANDLE),m_cs(INVALID_HANDLE),
      m_ce(INVALID_HANDLE),m_eo(INVALID_HANDLE),m_es(INVALID_HANDLE),
      m_ee(INVALID_HANDLE),m_rel(INVALID_HANDLE),m_ss(INVALID_HANDLE) {}

   bool Init(const bool enabled,const string stem,const MOR_RunIdentity &identity,const int flush_every)
     {
      m_enabled=enabled; m_identity=identity; m_flush_every=MathMax(1,flush_every); m_pending=0;
      if(!enabled) return(true);
      m_runs=Open(stem,"measurement_runs"); m_co=Open(stem,"compression_objects");
      m_cs=Open(stem,"compression_samples"); m_ce=Open(stem,"compression_events");
      m_eo=Open(stem,"expansion_objects"); m_es=Open(stem,"expansion_samples");
      m_ee=Open(stem,"expansion_events"); m_rel=Open(stem,"object_relations");
      m_ss=Open(stem,"structural_samples");
      if(m_runs==INVALID_HANDLE || m_co==INVALID_HANDLE || m_cs==INVALID_HANDLE ||
         m_ce==INVALID_HANDLE || m_eo==INVALID_HANDLE || m_es==INVALID_HANDLE ||
         m_ee==INVALID_HANDLE || m_rel==INVALID_HANDLE || m_ss==INVALID_HANDLE) { Close(); return(false); }
      FileWrite(m_runs,"contract","record_type","run_key","invocation_id","research_generation","source_auction_generation","raw_schema_version","compression_grammar_version","expansion_grammar_version","canonical_instrument","broker_symbol","timeframe","window_start","window_end","config_hash","data_source_id","data_fingerprint","terminal_reason","balanced","compression_started","compression_completed","compression_censored","compression_active","expansion_started","expansion_completed","expansion_censored","expansion_active","compression_samples","expansion_samples","relations");
      FileWrite(m_co,"run_key","compression_id","start_time","confirm_time","terminal_time","terminal_reason_code","terminal_reason","direction","escape_count","sample_count","seed_top","seed_bottom","seed_mid","terminal_contain_top","terminal_contain_bottom","terminal_contain_mid");
      FileWrite(m_cs,"run_key","compression_id","bar_time","age_bars","state_code","open","high","low","close","atr","seed_top","seed_bottom","seed_mid","contain_top","contain_bottom","contain_mid","escape_count","frontier_direction","bars_since_frontier_cross");
      FileWrite(m_ce,"run_key","sequence","compression_id","bar_time","event_code","event_direction","state_code","terminal_reason_code");
      FileWrite(m_eo,"run_key","expansion_id","origin_compression_id","destination_compression_id","origin_time","terminal_time","terminal_reason_code","terminal_reason","direction","sample_count","origin_seed_frontier","origin_containment_frontier","origin_terminal_price","origin_mid","origin_atr","terminal_price","max_displacement","opposite_displacement","return_depth","close_path_length");
      FileWrite(m_es,"run_key","expansion_id","bar_time","age_bars","open","high","low","close","state_code","origin_atr","origin_terminal_price","raw_displacement","raw_max_displacement","raw_opposite_displacement","raw_return_depth","raw_close_path_length","raw_velocity_per_bar","raw_acceleration_per_bar2");
      FileWrite(m_ee,"run_key","sequence","expansion_id","origin_compression_id","destination_compression_id","bar_time","event_code","state_code","terminal_reason_code");
      FileWrite(m_rel,"run_key","relation_id","bar_time","relation_code","source_kind","source_id","destination_kind","destination_id","structural_snapshot_hash");
      FileWrite(m_ss,"run_key","bar_time","structure_snapshot_hash","regional_basis_hash","structure_generation","reference_price","reference_atr","median_price","mean_price","structural_sigma","cog_price","price_region_code","nearest_node_id","nearest_node_lower","nearest_node_price","nearest_node_upper","nearest_node_region_code","node_contact");
      return(true);
     }

   void WriteRun(const string record_type,const int terminal_reason,const bool balanced,const MOR_Balance &b)
     {
      if(!m_enabled) return;
      FileWrite(m_runs,MOR_CONTRACT_ID,record_type,m_identity.run_key,m_identity.invocation_id,MOR_RESEARCH_GENERATION,MOR_SOURCE_AUCTION_GENERATION,MOR_RAW_SCHEMA_VERSION,MOR_COMPRESSION_GRAMMAR_VERSION,MOR_EXPANSION_GRAMMAR_VERSION,m_identity.canonical_instrument,m_identity.broker_symbol,(int)m_identity.timeframe,(long)m_identity.window_start,(long)m_identity.window_end,(string)m_identity.config_hash,m_identity.data_source_id,m_identity.data_fingerprint,terminal_reason,MOR_Bool(balanced),b.compressions_started,b.compressions_completed,b.compressions_censored,b.compressions_active,b.expansions_started,b.expansions_completed,b.expansions_censored,b.expansions_active,b.compression_samples,b.expansion_samples,b.relations);
      FlushIfNeeded();
     }

   void CompressionObject(const MOR_CompressionObject &o)
     {
      if(!m_enabled) return;
      FileWrite(m_co,m_identity.run_key,o.id,(long)o.start_time,(long)o.confirm_time,(long)o.terminal_time,o.terminal_reason,MOR_TerminalName(o.terminal_reason),o.direction,o.escape_count,o.sample_count,MOR_Number(o.seed_top),MOR_Number(o.seed_bottom),MOR_Number(o.seed_mid),MOR_Number(o.terminal_contain_top),MOR_Number(o.terminal_contain_bottom),MOR_Number(o.terminal_contain_mid)); FlushIfNeeded();
     }

   void CompressionSample(const long id,const datetime time,const int age,const int state,const double open,const double high,const double low,const double close,const double atr,const double top,const double bottom,const double mid,const double ctop,const double cbottom,const double cmid,const int escapes,const int frontier,const int since_cross)
     {
      if(!m_enabled) return;
      FileWrite(m_cs,m_identity.run_key,id,(long)time,age,state,MOR_Number(open),MOR_Number(high),MOR_Number(low),MOR_Number(close),MOR_Number(atr),MOR_Number(top),MOR_Number(bottom),MOR_Number(mid),MOR_Number(ctop),MOR_Number(cbottom),MOR_Number(cmid),escapes,frontier,since_cross); FlushIfNeeded();
     }

   void CompressionEvent(const ulong sequence,const long id,const datetime time,const int event_code,const int direction,const int state,const int terminal_reason)
     {
      if(!m_enabled) return;
      FileWrite(m_ce,m_identity.run_key,sequence,id,(long)time,event_code,direction,state,terminal_reason); FlushIfNeeded();
     }

   void ExpansionObject(const MOR_ExpansionObject &o)
     {
      if(!m_enabled) return;
      string destination=(o.destination_compression_id>0 ? IntegerToString(o.destination_compression_id) : MOR_NULL);
      FileWrite(m_eo,m_identity.run_key,o.id,o.origin_compression_id,destination,(long)o.origin_time,(long)o.terminal_time,o.terminal_reason,MOR_TerminalName(o.terminal_reason),o.direction,o.sample_count,MOR_Number(o.origin_seed_frontier),MOR_Number(o.origin_containment_frontier),MOR_Number(o.origin_terminal_price),MOR_Number(o.origin_mid),MOR_Number(o.origin_atr),MOR_Number(o.terminal_price),MOR_Number(o.max_displacement),MOR_Number(o.opposite_displacement),MOR_Number(o.return_depth),MOR_Number(o.close_path_length)); FlushIfNeeded();
     }

   void ExpansionSample(const long id,const datetime time,const double open,const double high,const double low,const double close,const int state,const double origin_atr,const double origin_price,const double displacement,const double max_displacement,const double opposite,const double return_depth,const double path,const double velocity,const double acceleration,const int age)
     {
      if(!m_enabled) return;
      FileWrite(m_es,m_identity.run_key,id,(long)time,age,MOR_Number(open),MOR_Number(high),MOR_Number(low),MOR_Number(close),state,MOR_Number(origin_atr),MOR_Number(origin_price),MOR_Number(displacement),MOR_Number(max_displacement),MOR_Number(opposite),MOR_Number(return_depth),MOR_Number(path),MOR_Number(velocity),MOR_Number(acceleration)); FlushIfNeeded();
     }

   void ExpansionEvent(const ulong sequence,const long id,const long origin,const long destination,const datetime time,const int event_code,const int state,const int terminal_reason)
     {
      if(!m_enabled) return;
      string destination_text=(destination>0 ? IntegerToString(destination) : MOR_NULL);
      FileWrite(m_ee,m_identity.run_key,sequence,id,origin,destination_text,(long)time,event_code,state,terminal_reason); FlushIfNeeded();
     }

   void Relation(const ulong relation_id,const datetime time,const int code,const string source_kind,const string source_id,const string destination_kind,const string destination_id,const ulong snapshot_hash)
     {
      if(!m_enabled) return;
      FileWrite(m_rel,m_identity.run_key,relation_id,(long)time,code,source_kind,source_id,destination_kind,destination_id,(string)snapshot_hash); FlushIfNeeded();
     }

   void StructuralSample(const datetime time,const ulong snapshot_hash,const ulong basis_hash,const ulong generation,const double reference,const double atr,const double median,const double mean,const double sigma,const double cog,const int region,const ulong node_id,const double lower,const double price,const double upper,const int node_region,const bool contact)
     {
      if(!m_enabled) return;
      string id=(node_id>0 ? (string)node_id : MOR_NULL);
      FileWrite(m_ss,m_identity.run_key,(long)time,(string)snapshot_hash,(string)basis_hash,(string)generation,MOR_Number(reference),MOR_Number(atr),MOR_Number(median),MOR_Number(mean),MOR_Number(sigma),MOR_Number(cog),region,id,MOR_Number(lower),MOR_Number(price),MOR_Number(upper),node_region,MOR_Bool(contact)); FlushIfNeeded();
     }

   void Close(void)
     {
      CloseOne(m_runs); CloseOne(m_co); CloseOne(m_cs); CloseOne(m_ce); CloseOne(m_eo);
      CloseOne(m_es); CloseOne(m_ee); CloseOne(m_rel); CloseOne(m_ss); m_enabled=false;
     }
  };

#endif
