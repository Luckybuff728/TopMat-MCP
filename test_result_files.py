"""
诊断脚本：分析 calphamesh_get_task_result 失败根因

问题：
  Line 和 Scheil 任务调用 calphamesh_get_task_result 时全部返回
  "HTTP request failed: CSV file not found"

根因假说：
  get_result_files API 返回的文件列表中，文件名或 URL 格式不符合
  calphaMesh.rs 中的匹配逻辑，导致全部落入 handle_line_result 兜底分支，
  而该分支找不到 .csv 文件时抛出 HttpError("CSV file not found")

验证目标：
  1. 直接调用 /api/v1/get_result_files 获取各类已完成任务的实际文件列表
  2. 分别检查：Point 任务是否有 results.json
             Line  任务是否有 *.csv
             Scheil 任务是否有 scheil_solidification.json
  3. 尝试下载第一个文件，确认 URL 可访问
"""

import asyncio
import httpx
import json
import sys

# ── 配置 ────────────────────────────────────────────────────────────────────
API_BASE   = "https://api.topmaterial-tech.com"
API_KEY    = "tk_mAeBQyrp8MvPDBD4OxR4JbM9IyN8qvml"
HEADERS    = {
    "Authorization": f"Bearer {API_KEY}",
    "Content-Type": "application/json",
}

# 从日志中取出的已完成任务（按类型分组，从 get_task_status 确认 completed）
# Point 任务: 18829（成功）；18827/18828 也是同批次（应为 Point）
# Line  任务: 18830/18831/18832（失败）
# Scheil 任务: 18833/18834/18835（失败）
TASK_GROUPS = {
    "point":  [18827, 18828, 18829],
    "line":   [18830, 18831, 18832],
    "scheil": [18833, 18834, 18835],
}

SEP = "─" * 70


async def get_task_status(client: httpx.AsyncClient, task_id: int) -> dict:
    r = await client.post(f"{API_BASE}/api/v1/get_task", json={"id": task_id})
    r.raise_for_status()
    return r.json()


async def get_result_files(client: httpx.AsyncClient, task_id: int) -> dict:
    r = await client.post(f"{API_BASE}/api/v1/get_result_files", json={"id": task_id})
    r.raise_for_status()
    return r.json()


async def probe_file(client: httpx.AsyncClient, url: str) -> tuple[int, str]:
    """HEAD 请求探测文件是否可下载，返回 (status_code, content_type)"""
    try:
        r = await client.head(url, follow_redirects=True, timeout=10)
        ct = r.headers.get("content-type", "")
        return r.status_code, ct
    except Exception as e:
        return 0, str(e)


def classify_files(files: list[str]) -> dict:
    """按修复后的 calphaMesh.rs 匹配逻辑对文件分类"""
    def extract_filename(url: str) -> str:
        return url.split("?")[0].rsplit("/", 1)[-1]

    result = {
        "has_results_json": False,
        "has_scheil_json":  False,  # scheil_solidification.json OR scheil_conditions.json
        "has_csv":          False,
        "only_log":         False,
        "all_filenames":    [],
    }
    for url in files:
        name = extract_filename(url)
        result["all_filenames"].append(name)
        if name == "results.json":
            result["has_results_json"] = True
        if name in ("scheil_solidification.json", "scheil_conditions.json"):
            result["has_scheil_json"] = True
        if name.endswith(".csv"):
            result["has_csv"] = True

    # 仅有 output.log（后端计算失败但 status=completed）
    non_log = [n for n in result["all_filenames"] if n != "output.log"]
    result["only_log"] = len(non_log) == 0
    return result


async def diagnose_task(client: httpx.AsyncClient, task_id: int, expected_type: str):
    print(f"\n  Task {task_id} (expected={expected_type})")
    print(f"  {'─'*50}")

    # 1. 状态
    try:
        status_resp = await get_task_status(client, task_id)
        status = status_resp.get("status", "?")
        print(f"  status      : {status}")
    except Exception as e:
        print(f"  [ERROR] get_task_status 失败: {e}")
        return

    if status not in ("completed",):
        print(f"  [WARN] 任务未完成（status={status}），跳过文件检查")
        return

    # 2. 文件列表
    try:
        files_resp = await get_result_files(client, task_id)
        files = files_resp.get("files", [])
        total = files_resp.get("total_count", len(files))
        print(f"  文件总数    : {total}")
    except Exception as e:
        print(f"  [ERROR] get_result_files 失败: {e}")
        return

    if not files:
        print("  [WARN] files 列表为空，无结果文件！")
        return

    cls = classify_files(files)
    print(f"  文件名列表  : {cls['all_filenames']}")
    print(f"  has_results_json  : {cls['has_results_json']}")
    print(f"  has_scheil_json   : {cls['has_scheil_json']} (匹配 scheil_solidification.json / scheil_conditions.json)")
    print(f"  has_csv           : {cls['has_csv']}")
    print(f"  only_log          : {cls['only_log']}")

    # 3. 按修复后 calphaMesh.rs 的逻辑判断会走哪个分支
    if cls["only_log"]:
        branch = "[ERROR] no_result_files (仅有日志，后端计算内部失败)"
    elif cls["has_results_json"]:
        branch = "handle_point_result"
    elif cls["has_scheil_json"]:
        branch = "handle_scheil_result"
    elif cls["has_csv"]:
        branch = "handle_line_result"
    else:
        branch = "[ERROR] unknown_result_format"
    print(f"  → Rust 会走  : {branch}")

    # 4. 期望 vs 实际
    expected_branch = {
        "point":  "handle_point_result",
        "line":   "handle_line_result",
        "scheil": "handle_scheil_result",
    }.get(expected_type, "?")
    match = "[OK]" if branch == expected_branch else "[MISMATCH]"
    print(f"  期望分支    : {expected_branch}  {match}")

    # 5. 探测第一个文件是否可下载
    first_url = files[0]
    status_code, ct = await probe_file(client, first_url)
    print(f"  首文件 HEAD : status={status_code}, content-type={ct}")
    if status_code == 200:
        print("  [OK] 文件可访问")
    else:
        print(f"  [ERROR] 文件不可访问 (status={status_code})")


async def main():
    print(SEP)
    print(" CalphaMesh get_result_files 诊断测试")
    print(SEP)
    print(f" API_BASE = {API_BASE}")

    async with httpx.AsyncClient(headers=HEADERS, timeout=30) as client:
        # 先测试连通性
        try:
            pong = await client.get(f"{API_BASE}/health")
            print(f" 连通性检查: {pong.status_code}")
        except Exception:
            print(" 连通性检查: 跳过（无 /health 端点）")

        for group, task_ids in TASK_GROUPS.items():
            print(f"\n{SEP}")
            print(f" {group.upper()} 任务组")
            print(SEP)
            for tid in task_ids:
                await diagnose_task(client, tid, group)

    print(f"\n{SEP}")
    print(" 诊断完成")
    print(SEP)


if __name__ == "__main__":
    asyncio.run(main())
