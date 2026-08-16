from pathlib import Path
import json
import sys

sys.path.insert(0, str(Path(__file__).resolve().parents[1] / "implementation"))
from a2_transport import run

result = run()
out = Path(__file__).resolve().parent / "FC01A_A2_EXECUTION_RESULT_V1.json"
out.write_text(json.dumps(result, sort_keys=True, separators=(",", ":"), ensure_ascii=False), encoding="utf-8")
print(json.dumps({"result": result["result"], "output": str(out)}, sort_keys=True, separators=(",", ":")))
