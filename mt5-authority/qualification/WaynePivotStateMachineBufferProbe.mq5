//+------------------------------------------------------------------+
//| WaynePivotStateMachineBufferProbe.mq5                           |
//| Strategy Tester probe for all 17 machine-facing buffers.         |
//+------------------------------------------------------------------+
#property strict
#property tester_indicator "NorthstarWayneE2E\\WaynePivotStateMachine.ex5"

input ENUM_TIMEFRAMES InpTimePeriod = PERIOD_D1;
input string InpInstanceID = "PROBE";
input int InpInteractionBufferPoints = 2;
input int InpAcceptanceBars = 3;
input bool InpShowFuturePreview = true;
input datetime InpTesterFinalizeAt = D'2026.04.03 23:50:00';

int g_handle = INVALID_HANDLE;
datetime g_last_bar = 0;
ulong g_bars = 0;
ulong g_copy_failures = 0;
ulong g_invalid_values = 0;
ulong g_zero_active = 0;
int g_min_active = 99;
int g_max_active = 0;
bool g_mandatory_ok = true;
bool g_zone_ok = true;

void Emit(void)
{
   PrintFormat("WPE2E_PROBE_FINAL bars=%I64u copy_failures=%I64u invalid_values=%I64u zero_active=%I64u min_active=%d max_active=%d mandatory_ok=%d zone_ok=%d outcome=%s",
               g_bars, g_copy_failures, g_invalid_values, g_zero_active,
               g_min_active == 99 ? 0 : g_min_active, g_max_active,
               (int)g_mandatory_ok, (int)g_zone_ok,
               g_copy_failures == 0 && g_invalid_values == 0 &&
               g_zero_active == 0 && g_mandatory_ok && g_zone_ok ? "PASS" : "FAIL");
}

int OnInit()
{
   g_handle = iCustom(_Symbol, PERIOD_CURRENT,
                       "NorthstarWayneE2E\\WaynePivotStateMachine",
                       20, InpTimePeriod, InpInstanceID,
                       InpInteractionBufferPoints, InpAcceptanceBars,
                       InpShowFuturePreview, InpTesterFinalizeAt);
   if(g_handle == INVALID_HANDLE)
   {
      PrintFormat("WPE2E_PROBE_INIT_FAILED error=%d", GetLastError());
      return INIT_FAILED;
   }
   return INIT_SUCCEEDED;
}

void OnDeinit(const int reason)
{
   Emit();
   if(g_handle != INVALID_HANDLE) IndicatorRelease(g_handle);
}

void OnTick()
{
   datetime current_bar = iTime(_Symbol, PERIOD_CURRENT, 0);
   if(current_bar <= 0 || current_bar == g_last_bar) return;
   g_last_bar = current_bar;
   g_bars++;
   if(g_bars < 5) return;
   int active = 0;
   double values[17];
   for(int buffer = 0; buffer < 17; buffer++)
   {
      ResetLastError();
      int copied = CopyBuffer(g_handle, buffer, 0, 1, values);
      if(copied != 1)
      {
         g_copy_failures++;
         continue;
      }
      if(values[0] == EMPTY_VALUE) continue;
      if(!MathIsValidNumber(values[0]))
      {
         g_invalid_values++;
         continue;
      }
      active++;
   }
   g_min_active = MathMin(g_min_active, active);
   g_max_active = MathMax(g_max_active, active);
   if(active == 0) g_zero_active++;

   double mandatory[5];
   int mandatory_buffers[5] = {5, 6, 7, 9, 11};
   for(int i = 0; i < 5; i++)
   {
      if(CopyBuffer(g_handle, mandatory_buffers[i], 0, 1, mandatory) != 1 ||
         mandatory[0] == EMPTY_VALUE || !MathIsValidNumber(mandatory[0]) )
         g_mandatory_ok = false;
   }
   double zones[4];
   for(int i = 0; i < 4; i++)
   {
      if(CopyBuffer(g_handle, 13 + i, 0, 1, zones) != 1 ||
         zones[0] == EMPTY_VALUE || !MathIsValidNumber(zones[0]) )
         g_zone_ok = false;
   }
}

//+------------------------------------------------------------------+
