//+------------------------------------------------------------------+
//| VolProKittAdaptiveBufferProbe.mq5                               |
//| Strategy Tester probe for the adaptive indicator's 10 buffers.  |
//+------------------------------------------------------------------+
#property strict
#property tester_indicator "NorthstarKittAdaptive\\VolProKittAdaptive.ex5"

input ENUM_TIMEFRAMES InpCalcTF = PERIOD_M5;
input datetime InpFinalizeAt = D'2026.04.03 23:55:00';

int g_handle = INVALID_HANDLE;
datetime g_last_bar = 0;
ulong g_bars = 0;
ulong g_copy_failures = 0;
ulong g_invalid_values = 0;
ulong g_zero_active_after_warmup = 0;
int g_min_active = 10;
int g_max_active = 0;
bool g_emitted = false;

void EmitProbeReceipt(const string reason)
{
   if(g_emitted)
      return;
   g_emitted = true;
   PrintFormat("KVA_PROBE_FINAL reason=%s bars=%I64u copy_failures=%I64u invalid_values=%I64u zero_active_after_warmup=%I64u min_active=%d max_active=%d",
               reason, g_bars, g_copy_failures, g_invalid_values,
               g_zero_active_after_warmup,
               g_min_active == 10 ? 0 : g_min_active,
               g_max_active);
}

int OnInit()
{
   g_handle = iCustom(_Symbol, PERIOD_CURRENT,
                      "NorthstarKittAdaptive\\VolProKittAdaptive",
                      InpCalcTF);
   if(g_handle == INVALID_HANDLE)
   {
      PrintFormat("KVA_PROBE_INIT_FAILED error=%d", GetLastError());
      return INIT_FAILED;
   }
   return INIT_SUCCEEDED;
}

void OnDeinit(const int reason)
{
   EmitProbeReceipt("DEINITIALIZATION");
   if(g_handle != INVALID_HANDLE)
      IndicatorRelease(g_handle);
}

void OnTick()
{
   datetime current_bar = iTime(_Symbol, PERIOD_CURRENT, 0);
   if(current_bar <= 0 || current_bar == g_last_bar)
      return;
   g_last_bar = current_bar;
   g_bars++;

   int active = 0;
   for(int buffer = 0; buffer < 10; buffer++)
   {
      double value[1];
      ResetLastError();
      int copied = CopyBuffer(g_handle, buffer, 0, 1, value);
      if(copied != 1)
      {
         g_copy_failures++;
         continue;
      }
      if(value[0] == EMPTY_VALUE)
         continue;
      if(!MathIsValidNumber(value[0]) || value[0] <= 0.0)
      {
         g_invalid_values++;
         continue;
      }
      active++;
   }

   if(g_bars > 5)
   {
      g_min_active = MathMin(g_min_active, active);
      g_max_active = MathMax(g_max_active, active);
      if(active == 0)
         g_zero_active_after_warmup++;
   }
   if(InpFinalizeAt > 0 && TimeCurrent() >= InpFinalizeAt)
      EmitProbeReceipt("DECLARED_BOUNDARY");
}

//+------------------------------------------------------------------+
