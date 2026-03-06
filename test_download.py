"""快速验证：带认证头下载已完成任务的结果文件，解析内容结构"""
import asyncio
import httpx
import json

API_BASE = "https://api.topmaterial-tech.com"
API_KEY  = "tk_mAeBQyrp8MvPDBD4OxR4JbM9IyN8qvml"
HEADERS  = {"Authorization": f"Bearer {API_KEY}", "Content-Type": "application/json"}

# 已确认 completed 的任务（上次测试成功完成的）
TASKS = {"point": 18836, "line": 18840, "scheil": 18841}


async def main():
    async with httpx.AsyncClient(headers=HEADERS, timeout=30, follow_redirects=True) as client:
        for label, tid in TASKS.items():
            print(f"\n{'='*60}")
            print(f"  {label.upper()} task {tid}")
            print(f"{'='*60}")

            r = await client.post(f"{API_BASE}/api/v1/get_result_files", json={"id": tid})
            data = r.json()
            files = data.get("files", [])
            names = [u.split("?")[0].rsplit("/", 1)[-1] for u in files]
            print(f"  files: {names}")

            for url in files:
                name = url.split("?")[0].rsplit("/", 1)[-1]
                if name.endswith(".png"):
                    print(f"  [SKIP] {name} (binary)")
                    continue

                r2 = await client.get(url, timeout=30)
                ct = r2.headers.get("content-type", "")
                size = len(r2.text)
                status = r2.status_code

                if status != 200:
                    print(f"  [FAIL] {name}: HTTP {status}")
                    continue

                print(f"\n  [OK] {name} (status={status}, size={size}, ct={ct})")

                if name.endswith(".json"):
                    try:
                        d = r2.json()
                        print(f"    top-level keys: {list(d.keys())}")
                        print(f"    full content:\n{json.dumps(d, ensure_ascii=False, indent=2)[:1500]}")
                    except Exception as e:
                        print(f"    [JSON parse error] {e}")
                        print(f"    raw: {r2.text[:300]}")

                elif name.endswith(".csv"):
                    lines = r2.text.splitlines()
                    print(f"    total rows: {len(lines)}")
                    for ln in lines[:6]:
                        print(f"    | {ln[:120]}")
                    if len(lines) > 6:
                        print(f"    ... ({len(lines)-6} more rows)")

                elif name == "output.log":
                    log = r2.text
                    print(f"    log ({len(log)} bytes):")
                    for ln in log.splitlines():
                        print(f"    | {ln}")


asyncio.run(main())
