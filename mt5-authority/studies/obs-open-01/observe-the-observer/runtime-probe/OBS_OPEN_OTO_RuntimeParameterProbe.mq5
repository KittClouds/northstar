//+------------------------------------------------------------------+
//| OBSERVE-THE-OBSERVER runtime parameter identity probe            |
//| Metadata only: reads no indicator buffers and no market values.  |
//+------------------------------------------------------------------+
#property strict
#property script_show_inputs
#property tester_indicator "OpeningRangeGrammar_v2_00_LegacyStateMachine.ex5"
#property tester_indicator "OpeningRangeGrammar_v2_10_ExtremeSentinels.ex5"

input group "Probe"
input string InpV200Name = "OpeningRangeGrammar_v2_00_LegacyStateMachine";
input string InpV210Name = "OpeningRangeGrammar_v2_10_ExtremeSentinels";
input string InpOutputFile = "OBS_OPEN_OTO_runtime_parameters.tsv";

input group "Legacy Breakout Clock"
input uint InpHourBegin = 9;
input uint InpMinBegin = 30;
input uint InpHourEnd = 9;
input uint InpMinEnd = 35;
input uint InpHourEndArea = 9;
input uint InpMinEndArea = 40;

const string OTO_SCHEMA = "OTO_EFFECTIVE_PARAMETER_VECTOR_V1";
const string OTO_BUNDLE_SCHEMA = "OTO_RUNTIME_INSTANTIATION_BUNDLE_V1";
const string V200_SOURCE_SHA256 =
   "9519323848f232ff4255f8d00896d2a78b91d16b7e1dc07ed7bac8451d0fd5c2";
const string V200_EX5_SHA256 =
   "6f221fdd9f536ffa64e23842a4d9f77bd218bafda1c350b45458c1232a1a23ed";
const string V210_SOURCE_SHA256 =
   "a133ef773531a1599d1cef1e5124fcacc24b037f84cca777205c67ee0a785c61";
const string V210_EX5_SHA256 =
   "a9e25979dc85a760b15499a13552216f19646e43274c93c4d032adc671f415dd";
const string INST01_ROOT =
   "5930c7d4f5b2de4549bfedacbd2c3b9df7da78f68c7d765b9c0669d08a17490f";

string SafeCell(string value)
{
   StringReplace(value, "\t", "\\t");
   StringReplace(value, "\r", "\\r");
   StringReplace(value, "\n", "\\n");
   return value;
}

void WriteCells(const int file,
                const string a,
                const string b,
                const string c,
                const string d,
                const string e,
                const string f,
                const string g,
                const string h)
{
   FileWriteString(file,
                   SafeCell(a) + "\t" + SafeCell(b) + "\t" +
                   SafeCell(c) + "\t" + SafeCell(d) + "\t" +
                   SafeCell(e) + "\t" + SafeCell(f) + "\t" +
                   SafeCell(g) + "\t" + SafeCell(h) + "\r\n");
}

bool WriteVector(const int file, const string role, const int handle)
{
   MqlParam parameters[];
   ENUM_INDICATOR indicator_type;
   ResetLastError();
   const int count = IndicatorParameters(handle, indicator_type, parameters);
   if(count < 0)
   {
      WriteCells(file, "ERROR", role, "IndicatorParameters", IntegerToString(GetLastError()),
                 "", "", "", "");
      return false;
   }

   WriteCells(file, "VECTOR", role, "parameter_count", IntegerToString(count),
              "indicator_type", IntegerToString((int)indicator_type),
              EnumToString(indicator_type), "");

   for(int i = 0; i < count; ++i)
   {
      WriteCells(file,
                 "PARAMETER",
                 role,
                 IntegerToString(i),
                 IntegerToString((int)parameters[i].type),
                 EnumToString((ENUM_DATATYPE)parameters[i].type),
                 IntegerToString(parameters[i].integer_value),
                 DoubleToString(parameters[i].double_value, 17),
                 parameters[i].string_value);
   }
   return true;
}

void OnStart()
{
   const int file = FileOpen(InpOutputFile,
                             FILE_COMMON | FILE_WRITE | FILE_ANSI | FILE_SHARE_READ);
   if(file == INVALID_HANDLE)
   {
      PrintFormat("OTO parameter probe FileOpen failed: %d", GetLastError());
      return;
   }

   WriteCells(file, "META", "schema", OTO_SCHEMA, "", "", "", "", "");
   WriteCells(file, "META", "runtime_bundle_schema", OTO_BUNDLE_SCHEMA, "", "", "", "", "");
   WriteCells(file, "META", "inst01_root", INST01_ROOT, "", "", "", "", "");
   WriteCells(file, "META", "v200_source_sha256", V200_SOURCE_SHA256, "", "", "", "", "");
   WriteCells(file, "META", "v200_ex5_sha256", V200_EX5_SHA256, "", "", "", "", "");
   WriteCells(file, "META", "v210_source_sha256", V210_SOURCE_SHA256, "", "", "", "", "");
   WriteCells(file, "META", "v210_ex5_sha256", V210_EX5_SHA256, "", "", "", "", "");
   WriteCells(file, "META", "symbol", _Symbol, "timeframe", EnumToString(_Period), "", "", "");
   WriteCells(file, "META", "period_seconds",
              IntegerToString(PeriodSeconds(_Period)),
              "digits", IntegerToString(_Digits), "", "", "");
   WriteCells(file, "META", "terminal_company", TerminalInfoString(TERMINAL_COMPANY),
              "terminal_name", TerminalInfoString(TERMINAL_NAME), "", "", "");
   WriteCells(file, "META", "terminal_build",
              IntegerToString((int)TerminalInfoInteger(TERMINAL_BUILD)),
              "program_build", IntegerToString(__MQLBUILD__), "", "", "");
   WriteCells(file, "META", "terminal_connected",
              (bool)TerminalInfoInteger(TERMINAL_CONNECTED) ? "true" : "false",
              "mql_tester", (bool)MQLInfoInteger(MQL_TESTER) ? "true" : "false",
              "mql_visual_mode", (bool)MQLInfoInteger(MQL_VISUAL_MODE) ? "true" : "false", "");
   WriteCells(file, "META", "capture_time", IntegerToString((long)TimeCurrent()), "", "", "", "", "");

   const int v200 = iCustom(_Symbol, _Period, InpV200Name,
                            "Legacy Breakout Clock",
                            InpHourBegin, InpMinBegin, InpHourEnd, InpMinEnd,
                            InpHourEndArea, InpMinEndArea,
                            "Causal State / Coverage",
                            20, true, true, true,
                            "Legacy Renderer",
                            true, clrRed, clrFuchsia, 2, 1, STYLE_SOLID, STYLE_DOT,
                            "Identity", "INST01_V200");
   const int v210 = iCustom(_Symbol, _Period, InpV210Name,
                            "Legacy Breakout Clock",
                            InpHourBegin, InpMinBegin, InpHourEnd, InpMinEnd,
                            InpHourEndArea, InpMinEndArea,
                            "Causal State / Coverage",
                            20, true, true, true,
                            "Legacy Renderer",
                            true, clrRed, clrFuchsia, 2, 1, STYLE_SOLID, STYLE_DOT,
                            "Causal Extreme Sentinels",
                            true, false, clrViolet, clrTeal, STYLE_SOLID, 2,
                            "Identity", "INST01_V210");

   bool ok = true;
   if(v200 == INVALID_HANDLE)
   {
      WriteCells(file, "ERROR", "V200", "iCustom", IntegerToString(GetLastError()),
                 "", "", "", "");
      ok = false;
   }
   else
      ok = WriteVector(file, "V200", v200) && ok;

   if(v210 == INVALID_HANDLE)
   {
      WriteCells(file, "ERROR", "V210", "iCustom", IntegerToString(GetLastError()),
                 "", "", "", "");
      ok = false;
   }
   else
      ok = WriteVector(file, "V210", v210) && ok;

   WriteCells(file, "RESULT", ok ? "PASS" : "FAIL", "", "", "", "", "", "");
   FileFlush(file);
   FileClose(file);
   if(v200 != INVALID_HANDLE) IndicatorRelease(v200);
   if(v210 != INVALID_HANDLE) IndicatorRelease(v210);
}
