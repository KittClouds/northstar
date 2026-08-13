#property version "1.00"
#property indicator_chart_window
#property indicator_buffers 1
#property indicator_plots 1
#property indicator_type1 DRAW_NONE

#include <MarketObjectResearch\MarketObjectEngine.mqh>

double Hidden[];

int OnInit(void)
  {
   SetIndexBuffer(0,Hidden,INDICATOR_DATA);
   PlotIndexSetInteger(0,PLOT_DRAW_TYPE,DRAW_NONE);
   if(!RCMRunSelfTest() || !ETORunSelfTest() || !MORRunGoldenSelfTest())
      return(INIT_FAILED);
   Print("MOR_ALL_GOLDEN_TESTS_OK");
   return(INIT_SUCCEEDED);
  }

int OnCalculate(const int rates_total,const int prev_calculated,const datetime &time[],
                const double &open[],const double &high[],const double &low[],
                const double &close[],const long &tick_volume[],const long &volume[],
                const int &spread[])
  { return(rates_total); }
