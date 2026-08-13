//+------------------------------------------------------------------+
//| MarketObjectCollector.mq5                                       |
//| RG3 headless compression, expansion, and structure collector.    |
//+------------------------------------------------------------------+
#property version "1.00"
#property indicator_chart_window
#property indicator_buffers 1
#property indicator_plots 1
#property indicator_type1 DRAW_NONE

#include <MasterStructure\MasterController.mqh>
#include <MarketObjectResearch\MarketObjectEngine.mqh>

input group "RG3 Run Identity"
input string InpCanonicalInstrument="AUTO";
input string InpDataSourceId="BROKER_MT5";
input string InpDataFingerprint="REQUIRED";
input datetime InpWindowStart=0;
input datetime InpWindowEnd=0;
input bool InpEnableRawLogging=true;
input int InpFlushEvery=32;

input group "Private Compression Oracle V2"
input int InpRangeLen=20;
input double InpRangeMult=1.0;
input int InpAtrLen=500;
input int InpConfirmBars=1;
input int InpMaxActiveBars=240;
input double InpFrontierBufferAtr=0.0;
input bool InpUseFrontierBodyFilter=false;
input double InpFrontierBodyAtr=0.20;
input bool InpObserveReentry=true;
input int InpReentryWindow=5;
input bool InpReclaimNeedsMidline=false;
input int InpRearmResetBars=1;
input bool InpRequireFreshLineageWindow=true;

input group "Expansion Observation V1"
input int InpMaxObservationBars=500;

double HiddenBuffer[];
double g_source[],g_true_range[],g_atr[],g_sma[];
CMasterStructureController g_master;
CMarketObjectEngine g_objects;
int g_processed_closed=-1;
bool g_ready=false;
bool g_finalized=false;
string g_stem="";

int ClampInt(const int value,const int low,const int high)
  { return(MathMax(low,MathMin(high,value))); }
double ClampDouble(const double value,const double low,const double high)
  { return(MathMax(low,MathMin(high,value))); }

double RmaAt(const double &source[],const double &output[],const int bar,const int length)
  {
   const int len=MathMax(1,length);
   if(bar<0 || !RCMValidNumber(source[bar]))
      return(bar>0 && RCMValidNumber(output[bar-1]) ? output[bar-1] : EMPTY_VALUE);
   if(bar>0 && RCMValidNumber(output[bar-1]))
      return((output[bar-1]*(double)(len-1)+source[bar])/(double)len);
   int count=0; double sum=0.0;
   for(int i=bar;i>=0 && count<len;i--)
      if(RCMValidNumber(source[i])) { sum+=source[i]; count++; }
   return(count==len ? sum/(double)len : EMPTY_VALUE);
  }

void ResizeWorking(const int count,const bool clear)
  {
   ArrayResize(g_source,count); ArrayResize(g_true_range,count);
   ArrayResize(g_atr,count); ArrayResize(g_sma,count);
   if(clear)
     {
      ArrayInitialize(g_source,EMPTY_VALUE); ArrayInitialize(g_true_range,EMPTY_VALUE);
      ArrayInitialize(g_atr,EMPTY_VALUE); ArrayInitialize(g_sma,EMPTY_VALUE);
     }
  }

void CalculateSensorBar(const int bar,const double &high[],const double &low[],const double &close[])
  {
   g_source[bar]=close[bar];
   g_true_range[bar]=(bar==0 ? high[bar]-low[bar] :
                      MathMax(high[bar]-low[bar],MathMax(MathAbs(high[bar]-close[bar-1]),
                                                        MathAbs(low[bar]-close[bar-1]))));
   g_atr[bar]=RmaAt(g_true_range,g_atr,bar,ClampInt(InpAtrLen,1,3000));
   const int length=ClampInt(InpRangeLen,2,200);
   if(bar<length-1) { g_sma[bar]=EMPTY_VALUE; return; }
   double sum=0.0;
   for(int i=0;i<length;i++) sum+=g_source[bar-i];
   g_sma[bar]=sum/(double)length;
  }

string ObjectConfigText(void)
  {
   return StringFormat("contract=%s|range_len=%d|range_mult=%.12f|atr_len=%d|confirm=%d|max_active=%d|frontier_buffer=%.12f|body_filter=%d|body_atr=%.12f|observe_reentry=%d|reentry=%d|midline=%d|rearm=%d|fresh=%d|max_observation=%d|master_rg=%d|master_controller=%d|master_topology=%d|master_auction=%d|master_features=%d|master_dataset=%d|master_producers=%d|master_build=%s",
                       MOR_CONTRACT_ID,InpRangeLen,InpRangeMult,InpAtrLen,InpConfirmBars,
                       InpMaxActiveBars,InpFrontierBufferAtr,(int)InpUseFrontierBodyFilter,
                       InpFrontierBodyAtr,(int)InpObserveReentry,InpReentryWindow,
                       (int)InpReclaimNeedsMidline,InpRearmResetBars,
                       (int)InpRequireFreshLineageWindow,InpMaxObservationBars,
                       MST_RESEARCH_GENERATION,MST_CONTROLLER_VERSION,MST_TOPOLOGY_VERSION,
                       MST_AUCTION_GRAMMAR_VERSION,MST_FEATURE_SCHEMA_VERSION,
                       MST_DATASET_SCHEMA_VERSION,MST_PRODUCER_BUNDLE_VERSION,MST_CODE_BUILD_ID);
  }

bool BuildIdentity(MOR_RunIdentity &identity)
  {
   ZeroMemory(identity);
   string canonical=MST_ResolveCanonicalInstrument(InpCanonicalInstrument,_Symbol);
   if(canonical=="" || InpWindowStart<=0 || InpWindowEnd<=InpWindowStart ||
      InpDataFingerprint=="" || InpDataFingerprint=="REQUIRED") return(false);
   identity.canonical_instrument=canonical; identity.broker_symbol=_Symbol;
   identity.data_source_id=MST_IdentityToken(InpDataSourceId);
   identity.data_fingerprint=MST_IdentityToken(InpDataFingerprint);
   identity.timeframe=(ENUM_TIMEFRAMES)_Period;
   identity.window_start=InpWindowStart; identity.window_end=InpWindowEnd;
   identity.config_hash=MST_HashText(ObjectConfigText());
   ulong source_hash=MST_SourceFingerprintHash(identity.data_source_id,identity.data_fingerprint,_Symbol);
   string material=StringFormat("rg=%d|instrument=%s|symbol=%s|tf=%d|start=%I64d|end=%I64d|config=%I64u|source=%I64u",
                                MOR_RESEARCH_GENERATION,canonical,MST_IdentityToken(_Symbol),(int)_Period,
                                (long)InpWindowStart,(long)InpWindowEnd,identity.config_hash,source_hash);
   identity.run_key=StringFormat("RG3_%s_%s_%I64d_%I64d_%I64u",canonical,
                                 MST_IdentityToken(EnumToString((ENUM_TIMEFRAMES)_Period)),
                                 (long)InpWindowStart,(long)InpWindowEnd,MST_HashText(material));
   identity.invocation_id=MST_MakeInvocationId(identity.run_key,TimeLocal(),GetTickCount64());
   return(true);
  }

void ConfigureMaster(MST_ControllerConfig &config)
  {
   MST_DefaultControllerConfig(config);
   config.master_timeframe=(ENUM_TIMEFRAMES)_Period;
   config.instance_tag="RG3_OBJECT_SOURCE";
   config.enable_logging=false; config.enable_parity_oracle=false;
   config.deterministic_terminal_time=InpWindowEnd;
   config.canonical_instrument=InpCanonicalInstrument;
   config.data_source_id=InpDataSourceId;
   config.data_fingerprint=InpDataFingerprint;
   config.research_window_start=InpWindowStart;
   config.research_window_end=InpWindowEnd;
  }

RCMTransitionConfig CompressionConfig(void)
  {
   RCMTransitionConfig config;
   config.confirm_bars=ClampInt(InpConfirmBars,1,20);
   config.reentry_window=ClampInt(InpReentryWindow,1,50);
   config.rearm_reset_bars=ClampInt(InpRearmResetBars,1,20);
   config.max_active_bars=ClampInt(InpMaxActiveBars,20,1000);
   config.reentry_observation_enabled=InpObserveReentry;
   config.reclaim_needs_midline=InpReclaimNeedsMidline;
   config.use_frontier_body_filter=InpUseFrontierBodyFilter;
   config.frontier_buffer_atr=ClampDouble(InpFrontierBufferAtr,0.0,3.0);
   config.frontier_body_atr=ClampDouble(InpFrontierBodyAtr,0.0,3.0);
   return(config);
  }

void FinalizeCollector(const int reason,const datetime time,const double close)
  {
   if(g_finalized) return;
   g_objects.Finalize(reason,time,close);
   g_master.Finalize(reason);
   g_finalized=true;
  }

int OnInit(void)
  {
   if(!(bool)MQLInfoInteger(MQL_TESTER) || (ENUM_TIMEFRAMES)_Period!=PERIOD_M5)
     { Print("RG3 COLLECTOR: Strategy Tester M5 only"); return(INIT_PARAMETERS_INCORRECT); }
   if(!RCMRunSelfTest() || !ETORunSelfTest() || !MORRunGoldenSelfTest()) return(INIT_FAILED);
   SetIndexBuffer(0,HiddenBuffer,INDICATOR_DATA); ArraySetAsSeries(HiddenBuffer,false);
   PlotIndexSetInteger(0,PLOT_DRAW_TYPE,DRAW_NONE);
   MOR_RunIdentity identity;
   if(!BuildIdentity(identity)) { Print("RG3 COLLECTOR: explicit valid identity required"); return(INIT_PARAMETERS_INCORRECT); }
   MST_ControllerConfig master_config; ConfigureMaster(master_config);
   if(!g_master.Init(_Symbol,master_config)) return(INIT_FAILED);
   ETOConfig eto_config; eto_config.max_observation_bars=ClampInt(InpMaxObservationBars,1,10000);
   g_stem="MarketObjects_"+identity.canonical_instrument+"_M5_"+identity.run_key;
   if(!g_objects.Init(identity,g_stem,InpEnableRawLogging,InpFlushEvery,
                      CompressionConfig(),eto_config))
     { g_master.Deinit(); return(INIT_FAILED); }
   g_processed_closed=-1; g_finalized=false; g_ready=true;
   IndicatorSetString(INDICATOR_SHORTNAME,"RG3 Market Object Collector");
   return(INIT_SUCCEEDED);
  }

void OnDeinit(const int reason)
  {
   g_ready=false;
   if(!g_finalized)
     {
      datetime time=(g_processed_closed>=0 ? iTime(_Symbol,(ENUM_TIMEFRAMES)_Period,1) : InpWindowStart);
      double price=(g_processed_closed>=0 ? iClose(_Symbol,(ENUM_TIMEFRAMES)_Period,1) : EMPTY_VALUE);
      FinalizeCollector(MOR_SHUTDOWN_CENSORED,time,price);
     }
   g_objects.Deinit(); g_master.Deinit();
  }

int OnCalculate(const int rates_total,const int prev_calculated,const datetime &time[],
                const double &open[],const double &high[],const double &low[],
                const double &close[],const long &tick_volume[],const long &volume[],
                const int &spread[])
  {
   if(!g_ready || g_finalized || rates_total<3) return(rates_total);
   ArraySetAsSeries(time,false); ArraySetAsSeries(open,false); ArraySetAsSeries(high,false);
   ArraySetAsSeries(low,false); ArraySetAsSeries(close,false);
   const bool clear=(prev_calculated==0 || g_processed_closed>=rates_total);
   ResizeWorking(rates_total,clear);
   int calc_from=(clear ? 0 : MathMax(0,prev_calculated-2));
   for(int bar=calc_from;bar<rates_total;bar++) CalculateSensorBar(bar,high,low,close);
   const int last_closed=rates_total-2;
   int first=(g_processed_closed<0 ? 0 : g_processed_closed+1);
   const int length=ClampInt(InpRangeLen,2,200);
   for(int bar=first;bar<=last_closed;bar++)
     {
      if(time[bar]<InpWindowStart)
        {
         g_master.Update(bar==first,false,true);
         g_processed_closed=bar;
         continue;
        }
      if(time[bar]>=InpWindowEnd)
        {
         // The run contract ends at the authorized horizon, not at the first
         // broker bar observed after it.  Preserve the last admitted close as
         // sensor truth while recording the exact censor time independently.
         double cutoff_close=(bar>0 ? close[bar-1] : EMPTY_VALUE);
         FinalizeCollector(MOR_TEST_END_CENSORED,InpWindowEnd,cutoff_close);
         break;
        }
      if(!g_master.Update(bar==first,false,true))
        { g_processed_closed=bar; continue; }
      double center=g_sma[bar];
      double half=(RCMValidNumber(g_atr[bar]) ? g_atr[bar]*ClampDouble(InpRangeMult,0.1,10.0) : EMPTY_VALUE);
      bool enough=(bar>=length && RCMValidNumber(center) && RCMValidNumber(half));
      int outside=0;
      if(enough) for(int i=0;i<length;i++) if(MathAbs(g_source[bar-i]-center)>half) outside++;
      bool candidate=(enough && outside==0);
      int candidate_start=bar-length+1;
      MOR_ClosedBar object_bar;
      object_bar.bar=bar; object_bar.time=time[bar]; object_bar.open=open[bar];
      object_bar.high=high[bar]; object_bar.low=low[bar]; object_bar.close=close[bar];
      object_bar.atr=g_atr[bar]; object_bar.candidate=candidate;
      object_bar.candidate_start_bar=candidate_start;
      object_bar.candidate_start_time=(candidate_start>=0 ? time[candidate_start] : 0);
      object_bar.candidate_top=(enough ? center+half : EMPTY_VALUE);
      object_bar.candidate_bottom=(enough ? center-half : EMPTY_VALUE);
      object_bar.candidate_mid=center; object_bar.candidate_type=0;
      g_objects.ProcessClosedBar(object_bar);

      MST_ControllerReading reading; g_master.GetReading(reading);
      MST_Node nodes[]; int node_count=g_master.NodeCount(); ArrayResize(nodes,node_count);
      for(int i=0;i<node_count;i++) g_master.GetNode(i,nodes[i]);
      g_objects.ObserveStructure(object_bar,reading,nodes,node_count);
      g_processed_closed=bar;
     }
   return(rates_total);
  }
