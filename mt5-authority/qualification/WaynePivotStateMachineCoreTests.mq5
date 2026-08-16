//+------------------------------------------------------------------+
//| WaynePivotStateMachineCoreTests.mq5                             |
//| Hand-computable formula and grammar fixtures.                    |
//+------------------------------------------------------------------+
#property strict

#include "..\WaynePivotStateMachineCore.mqh"

int OnInit()
{
   WPE2E_CONFIG cfg;
   WPE2E_DefaultConfig(cfg);
   MqlRates source;
   ZeroMemory(source);
   source.time = D'2026.04.01 00:00:00';
   source.open = 100.0;
   source.high = 110.0;
   source.low = 90.0;
   source.close = 100.0;

   WPE2E_PIVOT_BLOCK block;
   WPE2E_CalculateBlock(source, 0, PERIOD_D1, 86400, block);
   WPE2E_LEVEL levels[];
   WPE2E_ExportLevels(block, cfg, levels);
   bool formula_ok = block.valid &&
                     MathAbs(block.PP - 100.0) < 1e-9 &&
                     MathAbs(block.M2 - 95.0) < 1e-9 &&
                     MathAbs(block.M4 - 115.0) < 1e-9 &&
                     MathAbs(block.M5 - 125.0) < 1e-9 &&
                     MathAbs(block.R2 - 120.0) < 1e-9 &&
                     MathAbs(block.S2 - 80.0) < 1e-9;
   bool zone_ok = ArraySize(levels) == WPE2E_LEVEL_COUNT &&
                  MathAbs(levels[13].lower - 80.0) < 1e-9 &&
                  MathAbs(levels[13].upper - 85.0) < 1e-9 &&
                  MathAbs(levels[14].lower - 115.0) < 1e-9 &&
                  MathAbs(levels[14].upper - 120.0) < 1e-9;
   bool mandatory_ok = levels[5].valid && levels[6].valid && levels[7].valid &&
                       levels[9].valid && levels[11].valid;

   WPE2E_EVENT_ROW event;
   bool touch = WPE2E_ObserveLevel(levels[6], D'2026.04.02 10:00:00',
                                   100.5, 99.5, 100.0, 0.01, cfg, event) &&
                event.event == WPE2E_EVENT_TOUCH && levels[6].state == WPE2E_STATE_TOUCHED;
   bool accepted = false;
   accepted = WPE2E_ObserveLevel(levels[6], D'2026.04.02 10:01:00',
                                 100.5, 99.5, 100.0, 0.01, cfg, event) || accepted;
   accepted = WPE2E_ObserveLevel(levels[6], D'2026.04.02 10:02:00',
                                 100.5, 99.5, 100.0, 0.01, cfg, event) || accepted;
   accepted = accepted && levels[6].state == WPE2E_STATE_ACCEPTED;
   bool broken = WPE2E_ObserveLevel(levels[6], D'2026.04.02 10:03:00',
                                    102.5, 101.5, 102.0, 0.01, cfg, event) &&
                 event.event == WPE2E_EVENT_BREAK && levels[6].state == WPE2E_STATE_BROKEN;
   bool reclaimed = WPE2E_ObserveLevel(levels[6], D'2026.04.02 10:04:00',
                                       100.5, 99.5, 100.0, 0.01, cfg, event) &&
                    event.event == WPE2E_EVENT_RECLAIM && levels[6].state == WPE2E_STATE_RECLAIMED;

   bool lower_zone = WPE2E_ObserveLevel(levels[13], D'2026.04.02 11:00:00',
                                        84.0, 82.0, 83.0, 0.01, cfg, event);
   WPE2E_ObserveLevel(levels[13], D'2026.04.02 11:01:00', 84.0, 82.0, 83.0, 0.01, cfg, event);
   WPE2E_ObserveLevel(levels[13], D'2026.04.02 11:02:00', 84.0, 82.0, 83.0, 0.01, cfg, event);
   bool lower_zone_accepted = levels[13].state == WPE2E_STATE_ACCEPTED;
   bool lower_zone_broken = WPE2E_ObserveLevel(levels[13], D'2026.04.02 11:03:00',
                                                88.0, 86.0, 87.0, 0.01, cfg, event) &&
                            levels[13].state == WPE2E_STATE_BROKEN;
   bool lower_zone_reclaimed = WPE2E_ObserveLevel(levels[13], D'2026.04.02 11:04:00',
                                                   84.0, 82.0, 83.0, 0.01, cfg, event) &&
                               levels[13].state == WPE2E_STATE_RECLAIMED;

   bool upper_zone = WPE2E_ObserveLevel(levels[14], D'2026.04.02 12:00:00',
                                        119.0, 116.0, 118.0, 0.01, cfg, event);
   WPE2E_ObserveLevel(levels[14], D'2026.04.02 12:01:00', 119.0, 116.0, 118.0, 0.01, cfg, event);
   WPE2E_ObserveLevel(levels[14], D'2026.04.02 12:02:00', 119.0, 116.0, 118.0, 0.01, cfg, event);
   bool upper_zone_accepted = levels[14].state == WPE2E_STATE_ACCEPTED;
   bool upper_zone_broken = WPE2E_ObserveLevel(levels[14], D'2026.04.02 12:03:00',
                                                114.0, 112.0, 113.0, 0.01, cfg, event) &&
                            levels[14].state == WPE2E_STATE_BROKEN;
   bool upper_zone_reclaimed = WPE2E_ObserveLevel(levels[14], D'2026.04.02 12:04:00',
                                                   119.0, 116.0, 118.0, 0.01, cfg, event) &&
                               levels[14].state == WPE2E_STATE_RECLAIMED;

   bool all_ok = formula_ok && zone_ok && mandatory_ok && touch && accepted && broken && reclaimed &&
                 lower_zone && lower_zone_accepted && lower_zone_broken && lower_zone_reclaimed &&
                 upper_zone && upper_zone_accepted && upper_zone_broken && upper_zone_reclaimed;
   PrintFormat("WPE2E_CORE_TEST formula_ok=%d zone_geometry_ok=%d mandatory_ok=%d touch=%d accepted=%d broken=%d reclaimed=%d lower_zone=%d/%d/%d/%d upper_zone=%d/%d/%d/%d outcome=%s",
               (int)formula_ok, (int)zone_ok, (int)mandatory_ok, (int)touch,
               (int)accepted, (int)broken, (int)reclaimed,
               (int)lower_zone, (int)lower_zone_accepted, (int)lower_zone_broken, (int)lower_zone_reclaimed,
               (int)upper_zone, (int)upper_zone_accepted, (int)upper_zone_broken, (int)upper_zone_reclaimed,
               all_ok ? "PASS" : "FAIL");
   return all_ok ? INIT_SUCCEEDED : INIT_FAILED;
}

void OnTick(void) {}

//+------------------------------------------------------------------+
