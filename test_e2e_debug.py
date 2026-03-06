"""
CalphaMesh 上下游接口完整端到端调试脚本
=========================================
覆盖范围：
  STAGE 1  REST API 连通性与认证
  STAGE 2  三类任务提交接口（Point / Line / Scheil）
  STAGE 3  任务轮询 + 状态机验证
  STAGE 4  get_result_files 文件列表精确分析
  STAGE 5  Presigned URL 可访问性 + 文件内容下载
  STAGE 6  文件内容解析正确性（对照契约文档）
  STAGE 7  Scheil scheil_conditions.json 格式分析
  STAGE 8  output.log 内容分析（失败根因）
  STAGE 9  MCP 层文件分类逻辑完整性验证
"""

import asyncio
import json
import time
import httpx

# ── 配置 ──────────────────────────────────────────────────────────────────────
API_BASE = "https://api.topmaterial-tech.com"
API_KEY  = "tk_mAeBQyrp8MvPDBD4OxR4JbM9IyN8qvml"
HEADERS  = {"Authorization": f"Bearer {API_KEY}", "Content-Type": "application/json"}

# 测试用成分（FE-SI 简单二元，总和=1.0，使用 FE-C-SI-MN-CU-TI-O.TDB）
POINT_PARAMS = {
    "task_type": "point_calculation",
    "tdb_file": "/app/exe/topthermo-next/database/FE-C-SI-MN-CU-TI-O.TDB",
    "task_name": f"debug_point_{int(time.time())}",
    "task_path": f"mcp_results/point/{int(time.time()*1000)}",
    "condition": {
        "components": ["FE", "SI"],
        "activated_phases": [],
        "temperature": 1273.15,
        "compositions": {"FE": 0.95, "SI": 0.05}
    }
}
LINE_PARAMS = {
    "task_type": "line_calculation",
    "tdb_file": "/app/exe/topthermo-next/database/FE-C-SI-MN-CU-TI-O.TDB",
    "task_name": f"debug_line_{int(time.time())}",
    "task_path": f"mcp_results/line/{int(time.time()*1000)}",
    "condition": {
        "components": ["FE", "SI"],
        "compositions_start": {"FE": 0.95, "SI": 0.05},
        "compositions_end":   {"FE": 0.95, "SI": 0.05},
        "temperature_start": 800.0,
        "temperature_end":   1400.0,
        "increments": 10,
        "activated_phases": []
    }
}
SCHEIL_PARAMS = {
    "task_type": "scheil_solidification",
    "tdb_file": "/app/exe/topthermo-next/database/FE-C-SI-MN-CU-TI-O.TDB",
    "task_name": f"debug_scheil_{int(time.time())}",
    "task_path": f"mcp_results/scheil/{int(time.time()*1000)}",
    "condition": {
        "components": ["FE", "SI"],
        "compositions": {"FE": 0.95, "SI": 0.05},
        "start_temperature": 1600.0,
        "temperature_step": 1.0,
        "activated_phases": [],
        "inhibit_phases": []
    }
}

SEP  = "=" * 72
SEP2 = "-" * 72

# ── 工具函数 ──────────────────────────────────────────────────────────────────

def section(title: str):
    print(f"\n{SEP}")
    print(f"  {title}")
    print(SEP)

def subsection(title: str):
    print(f"\n  {SEP2}")
    print(f"  {title}")
    print(f"  {SEP2}")

def ok(msg):  print(f"  [OK]    {msg}")
def err(msg): print(f"  [FAIL]  {msg}")
def info(msg):print(f"  [INFO]  {msg}")
def warn(msg):print(f"  [WARN]  {msg}")


def extract_filename(url: str) -> str:
    return url.split("?")[0].rsplit("/", 1)[-1]


def classify_files_mcp(files: list[str]) -> dict:
    """复现修复后的 Rust MCP 分类逻辑"""
    names = [extract_filename(u) for u in files]
    only_log = all(n == "output.log" for n in names) and len(names) <= 1
    return {
        "names": names,
        "has_results_json": "results.json" in names,
        "has_scheil_json":  any(n in ("scheil_solidification.json","scheil_conditions.json") for n in names),
        "has_csv":          any(n.endswith(".csv") for n in names),
        "only_log":         only_log,
    }


async def create_task(client, inner_params: dict) -> dict:
    body = {
        "title": inner_params["task_name"],
        "description": json.dumps(inner_params),
        "task_type": "topthermo_next",
        "db_key": "default"
    }
    r = await client.post(f"{API_BASE}/api/v1/create_task", json=body)
    r.raise_for_status()
    return r.json()


async def get_task(client, task_id: int) -> dict:
    r = await client.post(f"{API_BASE}/api/v1/get_task", json={"id": task_id})
    r.raise_for_status()
    return r.json()


async def get_result_files(client, task_id: int) -> dict:
    r = await client.post(f"{API_BASE}/api/v1/get_result_files", json={"id": task_id})
    r.raise_for_status()
    return r.json()


async def wait_for_completion(client, task_id: int, timeout: int = 120) -> dict:
    """轮询直到 completed/failed/error 或超时，返回最终 task 状态"""
    start = time.time()
    while True:
        task = await get_task(client, task_id)
        status = task.get("status", "")
        elapsed = int(time.time() - start)
        info(f"  task {task_id}: status={status}, elapsed={elapsed}s")
        if status in ("completed", "failed", "error"):
            return task
        if elapsed >= timeout:
            warn(f"  task {task_id}: 超时 ({timeout}s)，最后状态: {status}")
            return task
        await asyncio.sleep(8)


async def probe_url(client, url: str, download: bool = False) -> dict:
    """探测 presigned URL：GET 检查可达性（presigned URL 需要带 Authorization 才能访问），可选下载内容"""
    result = {"url": url, "head_status": 0, "head_ct": "", "content": None, "error": None}
    try:
        # 用 GET 而非 HEAD：内网对象存储的 presigned URL 需要携带认证头，
        # HEAD 不一定和 GET 行为一致（有些实现忽略 HEAD）
        r = await client.get(url, follow_redirects=True, timeout=30)
        result["head_status"] = r.status_code
        result["head_ct"] = r.headers.get("content-type", "")
        if download and r.status_code == 200:
            result["content"] = r.text
    except Exception as e:
        result["error"] = str(e)
    return result


# ── STAGE 1: REST 连通性 ──────────────────────────────────────────────────────

async def stage1_connectivity(client):
    section("STAGE 1: REST API 连通性与认证")

    # 1a. 健康检查（可选端点）
    try:
        r = await client.get(f"{API_BASE}/health", timeout=5)
        ok(f"/health => {r.status_code}")
    except Exception:
        info("/health 端点不存在（正常）")

    # 1b. 无认证应返回 401
    subsection("1b. 无认证头 -> 期望 401")
    try:
        r = await client.post(
            f"{API_BASE}/api/v1/get_task",
            json={"id": 1},
            headers={"Content-Type": "application/json"}
        )
        if r.status_code == 401:
            ok(f"401 Unauthorized (正确)")
        else:
            warn(f"期望 401，实际: {r.status_code}  body: {r.text[:100]}")
    except Exception as e:
        err(f"请求失败: {e}")

    # 1c. 不存在的 task_id 应返回 404
    subsection("1c. 不存在的 task_id -> 期望 404")
    try:
        r = await client.post(f"{API_BASE}/api/v1/get_task", json={"id": 999999999})
        if r.status_code == 404:
            ok(f"404 Not Found (正确)")
        else:
            warn(f"期望 404，实际: {r.status_code}  body: {r.text[:100]}")
    except Exception as e:
        err(f"请求失败: {e}")

    # 1d. 正常认证
    subsection("1d. 有效 Bearer Token -> get_tasks")
    try:
        r = await client.post(f"{API_BASE}/api/v1/get_tasks", json={"page":1,"items_per_page":1})
        if r.status_code == 200:
            d = r.json()
            ok(f"get_tasks OK: total_pages={d.get('total_pages')}, items={len(d.get('data',[]))}")
        else:
            err(f"status={r.status_code}  body={r.text[:200]}")
    except Exception as e:
        err(f"请求失败: {e}")


# ── STAGE 2: 三类任务提交 ─────────────────────────────────────────────────────

async def stage2_submit(client) -> dict:
    section("STAGE 2: 三类任务提交")
    task_ids = {}

    for label, params in [("Point", POINT_PARAMS), ("Line", LINE_PARAMS), ("Scheil", SCHEIL_PARAMS)]:
        subsection(f"2x. 提交 {label} 任务")
        info(f"  condition: {json.dumps(params['condition'], ensure_ascii=False)[:120]}")
        try:
            resp = await create_task(client, params)
            tid = resp.get("id")
            status = resp.get("status")
            if tid:
                ok(f"task_id={tid}, status={status}")
                task_ids[label.lower()] = tid
            else:
                err(f"未返回 task_id: {resp}")
        except Exception as e:
            err(f"提交失败: {e}")

    return task_ids


# ── STAGE 3: 任务轮询 ────────────────────────────────────────────────────────

async def stage3_poll(client, task_ids: dict) -> dict:
    section("STAGE 3: 任务轮询 + 状态机验证")
    completed = {}

    # 并发等待所有任务
    async def wait_one(label, tid):
        info(f"等待 {label} 任务 {tid}...")
        task = await wait_for_completion(client, tid, timeout=180)
        status = task.get("status", "")
        if status == "completed":
            ok(f"{label} task {tid} -> completed")
            completed[label] = tid
        else:
            err(f"{label} task {tid} -> {status}  (失败或超时)")

    await asyncio.gather(*[wait_one(k, v) for k, v in task_ids.items()])
    return completed


# ── STAGE 4: 文件列表分析 ──────────────────────────────────────────────────────

async def stage4_file_list(client, task_ids: dict) -> dict:
    section("STAGE 4: get_result_files 文件列表精确分析")
    all_files = {}   # label -> files list

    for label, tid in task_ids.items():
        subsection(f"4x. {label} task {tid}")
        try:
            resp = await get_result_files(client, tid)
            files = resp.get("files", [])
            total = resp.get("total_count", len(files))
            all_files[label] = files

            info(f"total_count={total}")
            for f in files:
                name = extract_filename(f)
                print(f"    {name}")
                print(f"      URL: {f[:80]}...")

            cls = classify_files_mcp(files)

            # 期望文件集（来自契约文档 §1.4）
            expected = {
                "point":  {"results.json", "table.csv", "output.log"},
                "line":   {"table_2.csv", "output.log"},
                "scheil": {"scheil_solidification.json", "scheil_conditions.json",
                           "scheil_solidification.csv", "scheil_solidification.png", "output.log"},
            }.get(label, set())

            actual_names = set(cls["names"])
            missing = expected - actual_names
            extra   = actual_names - expected

            if missing:
                err(f"缺少文件: {missing}")
            if extra:
                warn(f"多余文件: {extra}")
            if not missing and not extra:
                ok("文件集合完全符合契约")

            # MCP 分支判断
            if cls["only_log"]:
                branch = "no_result_files (仅有日志)"
            elif cls["has_results_json"]:
                branch = "handle_point_result"
            elif cls["has_scheil_json"]:
                branch = "handle_scheil_result"
            elif cls["has_csv"]:
                branch = "handle_line_result"
            else:
                branch = "unknown_result_format"
            info(f"MCP 分支判断: {branch}")

        except Exception as e:
            err(f"get_result_files 失败: {e}")
            all_files[label] = []

    return all_files


# ── STAGE 5: Presigned URL 可达性 ─────────────────────────────────────────────

async def stage5_url_probe(client, all_files: dict):
    section("STAGE 5: Presigned URL 可访问性测试")

    for label, files in all_files.items():
        if not files:
            continue
        subsection(f"5x. {label} 任务文件 URL")
        for url in files:
            name = extract_filename(url)
            # 只对非图片文件尝试下载
            download = not name.endswith(".png")
            result = await probe_url(client, url, download=download)
            code = result["head_status"]
            ct = result["head_ct"]
            has_content = result["content"] is not None

            if code == 200:
                ok(f"{name}: HEAD={code}, content-type={ct}, downloaded={has_content}")
            elif code == 403:
                err(f"{name}: HEAD={code} (Forbidden) -> presigned URL 403: {result['error'] or ct}")
            elif code == 0:
                err(f"{name}: 连接失败 -> {result['error']}")
            else:
                warn(f"{name}: HEAD={code}, ct={ct}")


# ── STAGE 6: 文件内容解析 ─────────────────────────────────────────────────────

async def stage6_parse_content(client, all_files: dict):
    section("STAGE 6: 文件内容解析正确性")

    for label, files in all_files.items():
        if not files:
            continue
        subsection(f"6x. {label} 任务 - 关键文件解析")
        for url in files:
            name = extract_filename(url)
            if name == "output.log" or name.endswith(".png"):
                continue

            result = await probe_url(client, url, download=True)
            if result["head_status"] != 200:
                warn(f"{name}: 无法下载 (status={result['head_status']}), 跳过解析")
                continue

            content = result["content"] or ""
            info(f"{name}: 大小={len(content)} 字节")

            if name.endswith(".json"):
                try:
                    data = json.loads(content)
                    ok(f"{name}: JSON 解析成功, 顶层字段: {list(data.keys())[:10]}")

                    # Point: results.json
                    if name == "results.json":
                        _validate_point_json(data)
                    # Scheil: scheil_solidification.json
                    elif name == "scheil_solidification.json":
                        _validate_scheil_json(data)
                    # Scheil: scheil_conditions.json
                    elif name == "scheil_conditions.json":
                        _validate_scheil_conditions_json(data)
                except json.JSONDecodeError as e:
                    err(f"{name}: JSON 解析失败: {e}")
                    print(f"      原始内容(前 200 字符): {content[:200]}")

            elif name.endswith(".csv"):
                lines = content.splitlines()
                ok(f"{name}: CSV 行数={len(lines)}, 首行(header): {lines[0][:100] if lines else 'empty'}")
                if len(lines) > 1:
                    info(f"      第2行: {lines[1][:100]}")


def _validate_point_json(data: dict):
    expected_fields = ["temperature", "pressure", "compositions", "phases",
                       "phase_fractions", "thermodynamic_properties", "chemical_potentials"]
    for f in expected_fields:
        if f in data:
            ok(f"  results.json.{f}: {str(data[f])[:60]}")
        else:
            err(f"  results.json 缺少字段: {f}")


def _validate_scheil_json(data: dict):
    # 契约文档结构: {"metadata": {...}, "solidification_curve": {...}}
    for section_key in ("metadata", "solidification_curve"):
        if section_key in data:
            ok(f"  scheil_solidification.json.{section_key}: {list(data[section_key].keys())}")
        else:
            err(f"  scheil_solidification.json 缺少顶层字段: {section_key}")
    if "metadata" in data:
        meta = data["metadata"]
        for f in ("converged", "method", "temperature_range"):
            if f not in meta:
                warn(f"  metadata 缺少字段: {f}")
    if "solidification_curve" in data:
        curve = data["solidification_curve"]
        for f in ("temperatures", "liquid_fractions", "solid_fractions"):
            if f in curve:
                arr = curve[f]
                ok(f"  solidification_curve.{f}: length={len(arr)}, first={arr[0] if arr else 'empty'}")
            else:
                err(f"  solidification_curve 缺少字段: {f}")


def _validate_scheil_conditions_json(data: dict):
    # scheil_conditions.json 是条件回显，格式待确认
    ok(f"  scheil_conditions.json 顶层结构: {json.dumps(data, ensure_ascii=False)[:300]}")


# ── STAGE 7: output.log 失败根因分析 ──────────────────────────────────────────

async def stage7_output_log(client, all_files: dict):
    section("STAGE 7: output.log 内容分析（失败根因）")

    for label, files in all_files.items():
        if not files:
            continue
        log_urls = [u for u in files if extract_filename(u) == "output.log"]
        if not log_urls:
            continue
        subsection(f"7x. {label} 任务 output.log")
        result = await probe_url(client, log_urls[0], download=True)
        if result["head_status"] != 200:
            warn(f"output.log 无法访问: status={result['head_status']}")
            continue
        log_content = result["content"] or ""
        lines = log_content.splitlines()
        info(f"output.log 总行数: {len(lines)}")
        # 打印全部日志（日志通常不长）
        for line in lines:
            print(f"    | {line}")


# ── STAGE 8: 历史失败任务的 output.log（用已知失败任务 ID） ─────────────────────

async def stage8_old_task_log(client):
    section("STAGE 8: 历史失败任务的 output.log（18830/18831/18832 为 Line 仅有日志）")
    # 这些 Line 任务只有 output.log，从设计看是后端计算失败
    for tid in [18830, 18831]:
        subsection(f"8x. 历史 task {tid} output.log")
        try:
            resp = await get_result_files(client, tid)
            files = resp.get("files", [])
            log_urls = [u for u in files if extract_filename(u) == "output.log"]
            if not log_urls:
                info(f"无 output.log 文件")
                continue
            result = await probe_url(client, log_urls[0], download=True)
            if result["head_status"] == 200:
                log = result["content"] or ""
                info(f"output.log ({len(log)} bytes):")
                for line in log.splitlines():
                    print(f"    | {line}")
            else:
                warn(f"output.log 无法访问: status={result['head_status']}")
        except Exception as e:
            err(f"获取失败: {e}")


# ── STAGE 9: MCP 逻辑完整性验证 ───────────────────────────────────────────────

async def stage9_mcp_logic(all_files: dict):
    section("STAGE 9: MCP 层文件分类逻辑完整性验证")

    cases = [
        # (描述, files_list, 期望分支)
        ("Point 完整", ["results.json", "table.csv", "output.log"], "handle_point_result"),
        ("Point 仅log", ["output.log"], "no_result_files"),
        ("Line 完整",   ["table_2.csv", "output.log"], "handle_line_result"),
        ("Line 仅log",  ["output.log"], "no_result_files"),
        ("Scheil 完整(新)", ["scheil_solidification.json", "scheil_conditions.json",
                           "scheil_solidification.csv", "output.log"], "handle_scheil_result"),
        ("Scheil 仅conditions", ["scheil_conditions.json", "output.log"], "handle_scheil_result"),
        ("未知格式", ["custom_output.xml", "output.log"], "unknown_result_format"),
        ("空文件列表", [], "no_result_files"),
    ]

    for desc, files_list, expected_branch in cases:
        cls = classify_files_mcp(files_list)
        if not files_list or (len(files_list) <= 1 and all(n == "output.log" for n in files_list)):
            branch = "no_result_files"
        elif cls["has_results_json"]:
            branch = "handle_point_result"
        elif cls["has_scheil_json"]:
            branch = "handle_scheil_result"
        elif cls["has_csv"]:
            branch = "handle_line_result"
        else:
            branch = "unknown_result_format"

        status = "[OK]  " if branch == expected_branch else "[FAIL]"
        print(f"  {status} {desc:<35} -> {branch}")
        if branch != expected_branch:
            print(f"         期望: {expected_branch}")


# ── 主程序 ────────────────────────────────────────────────────────────────────

async def main():
    print(f"\n{SEP}")
    print("  CalphaMesh 上下游接口完整端到端调试")
    print(f"  API_BASE = {API_BASE}")
    print(SEP)

    async with httpx.AsyncClient(headers=HEADERS, timeout=60, follow_redirects=True) as client:

        # STAGE 1: 连通性
        await stage1_connectivity(client)

        # STAGE 2: 提交三类任务
        task_ids = await stage2_submit(client)
        if not task_ids:
            err("无可用任务，终止测试")
            return

        # STAGE 3: 等待完成
        completed = await stage3_poll(client, task_ids)
        # 使用全部 task_ids 继续后续（不管成功失败都要分析文件列表）

        # STAGE 4: 文件列表分析
        all_files = await stage4_file_list(client, task_ids)

        # STAGE 5: URL 可达性
        await stage5_url_probe(client, all_files)

        # STAGE 6: 内容解析
        await stage6_parse_content(client, all_files)

        # STAGE 7: output.log 新任务
        await stage7_output_log(client, all_files)

        # STAGE 8: 历史失败任务 log
        await stage8_old_task_log(client)

        # STAGE 9: MCP 逻辑验证
        await stage9_mcp_logic(all_files)

    section("测试完成")
    print(f"  新提交任务 ID: {task_ids}")


if __name__ == "__main__":
    asyncio.run(main())
