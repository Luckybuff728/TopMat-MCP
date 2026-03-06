"""MCP CalphaMesh E2E 完整链路测试 — Point / Scheil / Line + 参数校验"""
import requests
import json
import sys

BASE = "http://localhost:3000"
API_KEY = "tk_mAeBQyrp8MvPDBD4OxR4JbM9IyN8qvml"


def sep(t):
    print(f"\n{'='*64}\n  {t}\n{'='*64}")


def parse_sse(r):
    """SSE 响应解析 — 必须用 r.content.decode 而非 r.text（避免 ISO-8859-1 损坏中文）"""
    body = r.content.decode("utf-8")
    for line in body.splitlines():
        if line.startswith("data: "):
            try:
                return json.loads(line[6:])
            except json.JSONDecodeError:
                pass
    return {}


class MCP:
    def __init__(self):
        self.sid = None
        self.rid = 0
        self.h = {
            "Content-Type": "application/json",
            "Accept": "application/json, text/event-stream",
            "Authorization": f"Bearer {API_KEY}",
        }
        self._connect()

    def _post(self, body, timeout=120):
        h = dict(self.h)
        if self.sid:
            h["Mcp-Session-Id"] = self.sid
        r = requests.post(f"{BASE}/mcp", json=body, headers=h, timeout=timeout)
        s = r.headers.get("Mcp-Session-Id")
        if s:
            self.sid = s
        return r

    def _connect(self):
        self.rid += 1
        r = self._post({"jsonrpc": "2.0", "method": "initialize", "id": self.rid, "params": {
            "protocolVersion": "2024-11-05", "capabilities": {},
            "clientInfo": {"name": "e2e-test", "version": "1.0"},
        }}, timeout=10)
        data = parse_sse(r)
        info = data.get("result", {}).get("serverInfo", {})
        print(f"  session={self.sid}")
        self._post({"jsonrpc": "2.0", "method": "notifications/initialized"}, timeout=5)

    def call(self, tool, args, timeout=120):
        self.rid += 1
        r = self._post({"jsonrpc": "2.0", "method": "tools/call", "id": self.rid,
                         "params": {"name": tool, "arguments": args}}, timeout=timeout)
        data = parse_sse(r)
        err = data.get("error")
        if err:
            return None, err.get("message", str(err))
        content = data.get("result", {}).get("content", [])
        if not content:
            return None, f"empty response (status={r.status_code})"
        text = content[0].get("text", "")
        try:
            return json.loads(text), None
        except json.JSONDecodeError:
            return None, f"JSON parse fail: {text[:300]}"


def submit_and_result(label, tool, args):
    s = MCP()
    parsed, err = s.call(tool, args)
    if err:
        print(f"  submit ERROR: {err[:400]}")
        return False
    tid = parsed.get("task_id")
    print(f"  submit OK: task_id={tid}, type={parsed.get('task_type')}")
    print(f"  summary: {parsed.get('summary', '')[:120]}")

    print(f"  等待 get_task_result(task_id={tid})...")
    parsed2, err2 = s.call("calphamesh_get_task_result", {
        "task_id": tid, "result_mode": "summary", "timeout_seconds": 90,
    })
    if err2:
        print(f"  get_result ERROR: {err2[:500]}")
        return False

    status = parsed2.get("status")
    ttype = parsed2.get("task_type")
    print(f"  result: status={status}, type={ttype}")

    if status != "completed":
        print(f"  非 completed: {json.dumps(parsed2, ensure_ascii=False)[:400]}")
        return False

    result = parsed2.get("result", {})

    if "phases" in result:
        print(f"  phases={result['phases']}")
        print(f"  fractions={json.dumps(result.get('phase_fractions', {}), ensure_ascii=False)[:250]}")
        dm = result.get("derived_metrics", {})
        if dm:
            print(f"  derived_metrics={json.dumps(dm, ensure_ascii=False)[:250]}")

    ds = result.get("data_summary")
    if ds:
        print(f"  data_summary.total_rows={ds.get('total_rows')}")
        if "liquidus_K" in ds:
            print(f"  liquidus_K={ds.get('liquidus_K')}, solidus_K={ds.get('solidus_K')}")
        cols = ds.get("columns", [])
        if cols:
            print(f"  columns({len(cols)})={cols[:5]}...")
        kp = ds.get("key_points")
        if kp:
            print(f"  key_points: {json.dumps(kp[:2], ensure_ascii=False)[:200]}...")

    dm2 = result.get("derived_metrics")
    if dm2 and ds:
        print(f"  derived_metrics={json.dumps(dm2, ensure_ascii=False)[:300]}")

    files = parsed2.get("files", {})
    if files:
        print(f"  files={list(files.keys())}")

    print(f"  >>> {label} PASSED")
    return True


if __name__ == "__main__":
    print("TopMat-LLM MCP CalphaMesh E2E Test")
    print(f"Target: {BASE}")
    try:
        requests.get(f"{BASE}/health", timeout=5)
    except Exception as e:
        print(f"Server unreachable: {e}")
        sys.exit(1)

    results = {}

    sep("1. Point (FE-C-SI @ 1273K)")
    results["point"] = submit_and_result("POINT", "calphamesh_submit_point_task", {
        "components": ["FE", "C", "SI"],
        "composition": {"FE": 0.95, "C": 0.03, "SI": 0.02},
        "temperature": 1273.15,
        "tdb_file": "FE-C-SI-MN-CU-TI-O.TDB",
    })

    sep("2. Scheil (FE-C-SI @ 1800K)")
    results["scheil"] = submit_and_result("SCHEIL", "calphamesh_submit_scheil_task", {
        "components": ["FE", "C", "SI"],
        "composition": {"FE": 0.95, "C": 0.03, "SI": 0.02},
        "start_temperature": 1800.0,
        "temperature_step": 2.0,
        "tdb_file": "FE-C-SI-MN-CU-TI-O.TDB",
    })

    sep("3. Line (FE-C-SI 800~1800K)")
    results["line"] = submit_and_result("LINE", "calphamesh_submit_line_task", {
        "components": ["FE", "C", "SI"],
        "start_composition": {"FE": 0.95, "C": 0.03, "SI": 0.02},
        "end_composition": {"FE": 0.95, "C": 0.03, "SI": 0.02},
        "start_temperature": 800.0,
        "end_temperature": 1800.0,
        "steps": 50,
        "tdb_file": "FE-C-SI-MN-CU-TI-O.TDB",
    })

    sep("4. 参数校验")
    cases = [
        ("sum!=1",     {"components": ["FE", "C"], "composition": {"FE": 0.5, "C": 0.3}, "temperature": 1273, "tdb_file": "FE-C-SI-MN-CU-TI-O.TDB"}),
        ("keys不匹配", {"components": ["FE", "C"], "composition": {"FE": 0.97, "C": 0.02, "SI": 0.01}, "temperature": 1273, "tdb_file": "FE-C-SI-MN-CU-TI-O.TDB"}),
        ("TDB缺NI",   {"components": ["FE", "C", "NI"], "composition": {"FE": 0.95, "C": 0.03, "NI": 0.02}, "temperature": 1273, "tdb_file": "FE-C-SI-MN-CU-TI-O.TDB"}),
        ("TDB缺SR",   {"components": ["FE", "SI", "SR"], "composition": {"FE": 0.95, "SI": 0.04, "SR": 0.01}, "temperature": 973, "tdb_file": "FE-C-SI-MN-CU-TI-O.TDB"}),
    ]
    all_pass = True
    for label, args in cases:
        s = MCP()
        _, err = s.call("calphamesh_submit_point_task", args)
        if err and ("工具调用失败" in err or "组分" in err or "不包含" in err or "不一致" in err):
            print(f"  [PASS] {label}: {err[:150]}")
        else:
            print(f"  [FAIL] {label}: err={err}")
            all_pass = False
    results["validation"] = all_pass

    sep("SUMMARY")
    for k, v in results.items():
        print(f"  {k}: {'PASS' if v else 'FAIL'}")
