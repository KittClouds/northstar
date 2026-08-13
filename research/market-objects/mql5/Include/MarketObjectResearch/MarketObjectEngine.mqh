#ifndef NORTHSTAR_MARKET_OBJECT_ENGINE_MQH
#define NORTHSTAR_MARKET_OBJECT_ENGINE_MQH

#include <RCMExpansion.mqh>
#include <ExpansionTrajectoryObserver\ETOStateMachineV1.mqh>
#include <MasterStructure\MasterTypes.mqh>
#include "MarketObjectLogger.mqh"

struct MOR_ClosedBar
  {
   int bar;
   datetime time;
   double open;
   double high;
   double low;
   double close;
   double atr;
   bool candidate;
   int candidate_start_bar;
   datetime candidate_start_time;
   double candidate_top;
   double candidate_bottom;
   double candidate_mid;
   int candidate_type;
  };

class CMarketObjectEngine
  {
private:
   RCMStateMachine m_rcm;
   ETOStateMachine m_eto;
   RCMTransitionConfig m_rcm_config;
   ETOConfig m_eto_config;
   CMorRawLogger m_logger;
   MOR_Balance m_balance;
   MOR_CompressionObject m_compression;
   MOR_ExpansionObject m_expansion;
   bool m_initialized;
   bool m_finalized;
   bool m_has_compression;
   bool m_has_expansion;
   ulong m_event_sequence;
   ulong m_relation_sequence;
   ulong m_last_contact_node;
   int m_lineage_epoch_bar;
   datetime m_last_time;
   double m_last_close;
   long m_recent_terminal_expansion_id;
   datetime m_recent_terminal_expansion_time;

   int CompressionReason(const ENUM_RCM_EVENT event) const
     {
      if(event==RCM_EVENT_RANGE_CLOSED_UP) return(MOR_COMPRESSION_CLOSED_UP);
      if(event==RCM_EVENT_RANGE_CLOSED_DOWN) return(MOR_COMPRESSION_CLOSED_DOWN);
      if(event==RCM_EVENT_RANGE_EXPIRED) return(MOR_COMPRESSION_EXPIRED);
      return(MOR_TERMINAL_NONE);
     }

   int ExpansionReason(const ENUM_ETO_EVENT event) const
     {
      if(event==ETO_EVENT_HANDOFF_TO_COMPRESSION) return(MOR_EXPANSION_HANDOFF);
      if(event==ETO_EVENT_HORIZON_CENSORED) return(MOR_HORIZON_CENSORED);
      return(MOR_TERMINAL_NONE);
     }

   void BeginCompression(const MOR_ClosedBar &bar)
     {
      ZeroMemory(m_compression);
      m_compression.id=m_rcm.range_id;
      m_compression.start_time=bar.candidate_start_time;
      m_compression.confirm_time=bar.time;
      m_compression.terminal_time=0;
      m_compression.terminal_reason=MOR_TERMINAL_NONE;
      m_compression.direction=0;
      m_compression.seed_top=m_rcm.top;
      m_compression.seed_bottom=m_rcm.bottom;
      m_compression.seed_mid=m_rcm.mid;
      m_compression.terminal_contain_top=m_rcm.contain_top;
      m_compression.terminal_contain_bottom=m_rcm.contain_bottom;
      m_compression.terminal_contain_mid=m_rcm.contain_mid;
      m_has_compression=true;
      m_balance.compressions_started++;
      m_balance.compressions_active=1;
     }

   void EndCompression(const MOR_ClosedBar &bar,const int reason,const int direction,const bool censored)
     {
      if(!m_has_compression) return;
      m_compression.terminal_time=bar.time;
      m_compression.terminal_reason=reason;
      m_compression.direction=direction;
      m_compression.escape_count=m_rcm.escape_count;
      m_compression.terminal_contain_top=m_rcm.contain_top;
      m_compression.terminal_contain_bottom=m_rcm.contain_bottom;
      m_compression.terminal_contain_mid=m_rcm.contain_mid;
      m_logger.CompressionObject(m_compression);
      if(censored) m_balance.compressions_censored++;
      else m_balance.compressions_completed++;
      m_balance.compressions_active=0;
      m_has_compression=false;
     }

   void BeginExpansion(const MOR_ClosedBar &bar)
     {
      ZeroMemory(m_expansion);
      m_expansion.id=m_eto.expansion_id;
      m_expansion.origin_compression_id=m_eto.origin_compression_id;
      m_expansion.origin_time=bar.time;
      m_expansion.direction=m_eto.direction;
      m_expansion.origin_seed_frontier=m_eto.origin_seed_frontier;
      m_expansion.origin_containment_frontier=m_eto.origin_containment_frontier;
      m_expansion.origin_terminal_price=m_eto.origin_terminal_price;
      m_expansion.origin_mid=m_eto.origin_mid;
      m_expansion.origin_atr=m_eto.origin_atr;
      m_expansion.terminal_price=m_eto.origin_terminal_price;
      m_has_expansion=true;
      m_balance.expansions_started++;
      m_balance.expansions_active=1;
      m_logger.Relation(++m_relation_sequence,bar.time,MOR_RELATION_ORIGINATES_FROM,
                        "EXPANSION",IntegerToString(m_expansion.id),
                        "COMPRESSION",IntegerToString(m_expansion.origin_compression_id),0);
      m_balance.relations++;
     }

   void UpdateExpansionObject(void)
     {
      if(!m_has_expansion) return;
      m_expansion.terminal_price=m_eto.previous_price;
      m_expansion.max_displacement=m_eto.max_displacement;
      m_expansion.opposite_displacement=m_eto.opposite_displacement;
      m_expansion.return_depth=m_eto.return_depth;
      m_expansion.close_path_length=m_eto.close_path_length;
     }

   void EndExpansion(const MOR_ClosedBar &bar,const int reason,const long destination,const bool censored)
     {
      if(!m_has_expansion) return;
      UpdateExpansionObject();
      m_expansion.destination_compression_id=destination;
      m_expansion.terminal_time=bar.time;
      m_expansion.terminal_reason=reason;
      m_logger.ExpansionObject(m_expansion);
      if(destination>0)
        {
         m_logger.Relation(++m_relation_sequence,bar.time,MOR_RELATION_HANDS_OFF_TO,
                           "EXPANSION",IntegerToString(m_expansion.id),
                           "COMPRESSION",IntegerToString(destination),0);
         m_balance.relations++;
        }
      if(censored) m_balance.expansions_censored++;
      else m_balance.expansions_completed++;
      m_balance.expansions_active=0;
      m_recent_terminal_expansion_id=m_expansion.id;
      m_recent_terminal_expansion_time=bar.time;
      m_has_expansion=false;
      m_last_contact_node=0;
     }

   void LogCompressionSample(const MOR_ClosedBar &bar,const RCMCommit &commit)
     {
      if(!m_has_compression) return;
      m_compression.sample_count++;
      m_balance.compression_samples++;
      m_logger.CompressionSample(m_compression.id,bar.time,commit.age_bars,(int)m_rcm.state,
                                 bar.open,bar.high,bar.low,bar.close,bar.atr,
                                 m_rcm.top,m_rcm.bottom,m_rcm.mid,m_rcm.contain_top,
                                 m_rcm.contain_bottom,m_rcm.contain_mid,m_rcm.escape_count,
                                 m_rcm.frontier_direction,commit.bars_since_frontier_cross);
     }

   void LogExpansionSample(const MOR_ClosedBar &bar)
     {
      if(!m_has_expansion) return;
      m_expansion.sample_count++;
      m_balance.expansion_samples++;
      m_logger.ExpansionSample(m_expansion.id,bar.time,bar.open,bar.high,bar.low,bar.close,
                               (int)m_eto.state,m_eto.origin_atr,m_eto.origin_terminal_price,
                               m_eto.displacement,m_eto.max_displacement,m_eto.opposite_displacement,
                               m_eto.return_depth,m_eto.close_path_length,m_eto.velocity,
                               m_eto.acceleration,m_eto.age_bars);
     }

public:
   CMarketObjectEngine(void):m_initialized(false),m_finalized(true),m_has_compression(false),
      m_has_expansion(false),m_event_sequence(0),m_relation_sequence(0),
      m_last_contact_node(0),m_last_time(0),m_last_close(EMPTY_VALUE) {}

   bool Init(const MOR_RunIdentity &identity,const string stem,const bool logging,
             const int flush_every,const RCMTransitionConfig &rcm_config,
             const ETOConfig &eto_config)
     {
      RCMReset(m_rcm); ETOReset(m_eto); ZeroMemory(m_balance);
      ZeroMemory(m_compression); ZeroMemory(m_expansion);
      m_rcm_config=rcm_config; m_eto_config=eto_config;
      m_event_sequence=0; m_relation_sequence=0; m_last_contact_node=0;
      m_lineage_epoch_bar=-1;
      m_recent_terminal_expansion_id=0; m_recent_terminal_expansion_time=0;
      m_last_time=0; m_last_close=EMPTY_VALUE; m_has_compression=false; m_has_expansion=false;
      if(!m_logger.Init(logging,stem,identity,flush_every)) return(false);
      m_initialized=true; m_finalized=false;
      m_logger.WriteRun("START",MOR_TERMINAL_NONE,true,m_balance);
      return(true);
     }

   void ProcessClosedBar(const MOR_ClosedBar &bar)
     {
      if(!m_initialized || m_finalized) return;
      m_last_time=bar.time; m_last_close=bar.close;

      RCMClosedBarInput rcm_input;
      rcm_input.bar=bar.bar; rcm_input.candidate=bar.candidate;
      rcm_input.candidate_start_bar=bar.candidate_start_bar;
      rcm_input.candidate_top=bar.candidate_top; rcm_input.candidate_bottom=bar.candidate_bottom;
      rcm_input.candidate_mid=bar.candidate_mid; rcm_input.candidate_type=bar.candidate_type;
      rcm_input.open=bar.open; rcm_input.high=bar.high; rcm_input.low=bar.low;
      rcm_input.close=bar.close; rcm_input.atr=bar.atr;
      if(m_lineage_epoch_bar>=0 &&
         (m_rcm.state==RCM_SEARCHING || m_rcm.state==RCM_FORMING) &&
         rcm_input.candidate_start_bar<m_lineage_epoch_bar)
         rcm_input.candidate=false;
      RCMCommit rcm_commit;
      RCMCommitClosedBar(m_rcm,m_rcm_config,rcm_input,rcm_commit);
      if(rcm_commit.event==RCM_EVENT_REARMED) m_lineage_epoch_bar=bar.bar+1;

      if(rcm_commit.range_created) BeginCompression(bar);
      if(m_has_compression) LogCompressionSample(bar,rcm_commit);
      if(rcm_commit.event!=RCM_EVENT_NONE && rcm_commit.event!=RCM_EVENT_REARMED)
         m_logger.CompressionEvent(++m_event_sequence,m_rcm.range_id,bar.time,
                                   (int)rcm_commit.event,rcm_commit.event_direction,
                                   (int)m_rcm.state,CompressionReason(rcm_commit.event));

      ETOClosedBarInput eto_input;
      eto_input.bar=bar.bar; eto_input.close=bar.close;
      eto_input.compression_created=rcm_commit.range_created;
      eto_input.expansion_opened=(rcm_commit.event==RCM_EVENT_RANGE_CLOSED_UP ||
                                  rcm_commit.event==RCM_EVENT_RANGE_CLOSED_DOWN);
      eto_input.compression_id=m_rcm.range_id;
      eto_input.direction=rcm_commit.event_direction;
      eto_input.seed_frontier=(rcm_commit.event_direction>0 ? m_rcm.top : m_rcm.bottom);
      eto_input.containment_frontier=(rcm_commit.event_direction>0 ? m_rcm.contain_top : m_rcm.contain_bottom);
      eto_input.terminal_price=bar.close; eto_input.origin_mid=m_rcm.mid; eto_input.origin_atr=bar.atr;
      ETOCommit eto_commit;
      ETOCommitClosedBar(m_eto,m_eto_config,eto_input,eto_commit);

      if(eto_commit.expansion_created) BeginExpansion(bar);
      if(m_has_expansion) LogExpansionSample(bar);
      if(eto_commit.event!=ETO_EVENT_NONE)
         m_logger.ExpansionEvent(++m_event_sequence,m_eto.expansion_id,m_eto.origin_compression_id,
                                 (eto_commit.event==ETO_EVENT_HANDOFF_TO_COMPRESSION ? m_rcm.range_id : 0),
                                 bar.time,(int)eto_commit.event,(int)m_eto.state,
                                 ExpansionReason(eto_commit.event));

      if(rcm_commit.range_terminal)
         EndCompression(bar,CompressionReason(rcm_commit.event),rcm_commit.event_direction,false);
      if(eto_commit.expansion_terminal)
        {
         long destination=(eto_commit.event==ETO_EVENT_HANDOFF_TO_COMPRESSION ? m_rcm.range_id : 0);
         EndExpansion(bar,ExpansionReason(eto_commit.event),destination,
                      eto_commit.event==ETO_EVENT_HORIZON_CENSORED);
        }
     }

   void ObserveStructure(const MOR_ClosedBar &bar,const MST_ControllerReading &reading,
                         const MST_Node &nodes[],const int node_count)
     {
      if(!m_initialized || m_finalized || !reading.valid) return;
      ulong nearest_id=0; double nearest_lower=EMPTY_VALUE,nearest_price=EMPTY_VALUE;
      double nearest_upper=EMPTY_VALUE,best=DBL_MAX; int nearest_region=MST_REGION_UNAVAILABLE;
      bool contact=false;
      for(int i=0;i<node_count;i++)
        {
         if(!nodes[i].valid) continue;
         double distance=0.0;
         if(bar.close<nodes[i].lower) distance=nodes[i].lower-bar.close;
         else if(bar.close>nodes[i].upper) distance=bar.close-nodes[i].upper;
         if(distance<best)
           {
            best=distance; nearest_id=nodes[i].node_id; nearest_lower=nodes[i].lower;
            nearest_price=nodes[i].price; nearest_upper=nodes[i].upper;
            nearest_region=(int)nodes[i].structural_region;
           }
        }
      if(nearest_id>0) contact=(bar.high>=nearest_lower && bar.low<=nearest_upper);
      const MST_RegionalSnapshot regional=reading.regional;
      m_logger.StructuralSample(bar.time,reading.snapshot_hash,regional.regional_basis_hash,
                                reading.generation,reading.reference_price,reading.atr,regional.median_price,
                                regional.mean_price,regional.structural_sigma,regional.cog_price,
                                (int)regional.price_region,nearest_id,nearest_lower,nearest_price,
                                nearest_upper,nearest_region,contact);
      long relation_expansion=(m_has_expansion ? m_expansion.id :
                               (m_recent_terminal_expansion_time==bar.time ?
                                m_recent_terminal_expansion_id : 0));
      if(relation_expansion>0 && contact && nearest_id!=m_last_contact_node)
        {
         m_logger.Relation(++m_relation_sequence,bar.time,MOR_RELATION_CONTACTS_NODE,
                           "EXPANSION",IntegerToString(relation_expansion),"NODE",(string)nearest_id,
                           reading.snapshot_hash);
         m_balance.relations++;
         m_last_contact_node=nearest_id;
        }
      else if(!contact) m_last_contact_node=0;
      if(m_recent_terminal_expansion_time==bar.time)
        { m_recent_terminal_expansion_id=0; m_recent_terminal_expansion_time=0; }
     }

   void Finalize(const int reason,const datetime cutoff_time,const double cutoff_price)
     {
      if(!m_initialized || m_finalized) return;
      MOR_ClosedBar bar; ZeroMemory(bar);
      bar.time=(cutoff_time>0 ? cutoff_time : m_last_time);
      bar.close=(cutoff_price!=EMPTY_VALUE && MathIsValidNumber(cutoff_price) ? cutoff_price : m_last_close);
      if(m_has_compression) EndCompression(bar,reason,0,true);
      if(m_has_expansion) EndExpansion(bar,reason,0,true);
      bool balanced=(m_balance.compressions_started==m_balance.compressions_completed+
                     m_balance.compressions_censored+m_balance.compressions_active &&
                     m_balance.expansions_started==m_balance.expansions_completed+
                     m_balance.expansions_censored+m_balance.expansions_active);
      m_logger.WriteRun("END",reason,balanced,m_balance);
      m_logger.Close(); m_finalized=true;
     }

   void Deinit(void)
     {
      if(m_initialized && !m_finalized) Finalize(MOR_SHUTDOWN_CENSORED,m_last_time,m_last_close);
      m_logger.Close(); m_initialized=false;
     }

   void GetBalance(MOR_Balance &out) const { out=m_balance; }
   long ActiveCompressionId(void) const { return(m_has_compression ? m_compression.id : 0); }
   long ActiveExpansionId(void) const { return(m_has_expansion ? m_expansion.id : 0); }
   ulong EventSequence(void) const { return(m_event_sequence); }
   ulong RelationSequence(void) const { return(m_relation_sequence); }
  };

bool MORGoldenCheck(const bool condition,const string message)
  {
   if(condition) return(true);
   Print("MOR_GOLDEN_FAIL "+message);
   return(false);
  }

void MORGoldenBar(MOR_ClosedBar &bar,const int index,const double close,
                  const bool candidate,const datetime start_time)
  {
   ZeroMemory(bar); bar.bar=index; bar.time=1000+index*300;
   bar.open=close; bar.high=close+0.25; bar.low=close-0.25; bar.close=close; bar.atr=1.0;
   bar.candidate=candidate; bar.candidate_start_bar=index; bar.candidate_start_time=start_time;
   bar.candidate_top=101.0; bar.candidate_bottom=99.0; bar.candidate_mid=100.0;
  }

bool MORRunGoldenSelfTest(void)
  {
   MOR_RunIdentity identity; ZeroMemory(identity);
   identity.run_key="RG3_GOLDEN"; identity.invocation_id="RG3_GOLDEN_I1";
   identity.canonical_instrument="US30"; identity.broker_symbol="US30";
   identity.data_source_id="FIXTURE"; identity.data_fingerprint="GOLDEN_V1";
   identity.timeframe=PERIOD_M5; identity.window_start=1000; identity.window_end=4000;
   RCMTransitionConfig rcm; rcm.confirm_bars=1; rcm.reentry_window=1;
   rcm.rearm_reset_bars=1; rcm.max_active_bars=20; rcm.reentry_observation_enabled=true;
   rcm.reclaim_needs_midline=false; rcm.use_frontier_body_filter=false;
   rcm.frontier_buffer_atr=0.0; rcm.frontier_body_atr=0.0;
   ETOConfig eto; eto.max_observation_bars=20;
   CMarketObjectEngine engine;
   if(!engine.Init(identity,"MOR_GOLDEN",false,1,rcm,eto)) return(false);
   MOR_ClosedBar bar;
   MORGoldenBar(bar,0,100.0,true,1000); engine.ProcessClosedBar(bar);
   if(!MORGoldenCheck(engine.ActiveCompressionId()==1,"compression-open")) return(false);
   MORGoldenBar(bar,1,102.0,false,0); engine.ProcessClosedBar(bar);
   MORGoldenBar(bar,2,103.0,false,0); engine.ProcessClosedBar(bar);
   MORGoldenBar(bar,3,104.0,false,0); engine.ProcessClosedBar(bar);
   if(!MORGoldenCheck(engine.ActiveExpansionId()==1 && engine.ActiveCompressionId()==0,
                      "close-opens-expansion")) return(false);
   MORGoldenBar(bar,4,103.0,false,0); engine.ProcessClosedBar(bar);
   MORGoldenBar(bar,5,102.0,true,2500); engine.ProcessClosedBar(bar);
   if(!MORGoldenCheck(engine.ActiveCompressionId()==2 && engine.ActiveExpansionId()==0,
                      "new-compression-handoff")) return(false);
   engine.Finalize(MOR_TEST_END_CENSORED,3000,102.0);
   MOR_Balance balance; engine.GetBalance(balance);
   if(!MORGoldenCheck(balance.compressions_started==2 && balance.compressions_completed==1 &&
                      balance.compressions_censored==1 && balance.compressions_active==0,
                      "compression-balance")) return(false);
   if(!MORGoldenCheck(balance.expansions_started==1 && balance.expansions_completed==1 &&
                      balance.expansions_censored==0 && balance.expansions_active==0,
                      "expansion-balance")) return(false);
   if(!MORGoldenCheck(engine.EventSequence()==6 && engine.RelationSequence()==2,
                      "sequence-ledger")) return(false);
   Print("MOR_GOLDEN_OK");
   return(true);
  }

#endif
