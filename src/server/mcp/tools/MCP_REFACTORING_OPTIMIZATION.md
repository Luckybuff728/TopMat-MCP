# MCP 服务重构与优化记录

> **版本**: v1.0  
> **日期**: 2026-03-12  
> **涉及文件**: `calphaMesh.rs`、`auth/client.rs`  
> **参考文档**: [CALPHAMESH_TOOLS.md](./CALPHAMESH_TOOLS.md)

---

## 目录

- [背景与问题](#背景与问题)
- [优化项一览](#优化项一览)
- [规范化工作](#规范化工作)
- [详细变更](#详细变更)
- [部署与验证](#部署与验证)
- [周报总结](#周报总结)

---

## 背景与问题

在 Al 合金热力学分析场景中，MCP 服务与 Agent 协作时暴露出以下问题：

| 问题 | 现象 | 根因 |
|------|------|------|
| thermodynamic_properties ZeroDivisionError | 4 元素（AL/SI/MG/FE）提交后任务完成但无结果文件，output.log 显示 `ZeroDivisionError: float division` | 5 元 TDB（Al-Si-Mg-Fe-Mn）含 MN 相参数，components 缺 MN 时后端 solver 数值崩溃，但返回 `completed` 状态 |
| no_result_files 无效重试 | LLM 反复重试（如 increments 25→10），仍失败 | `retryable: true` 导致 Agent 认为可重试，ZeroDivisionError 实为后端 bug，重试无效 |
| ternary S3 延迟 | ternary 任务 files=[]，但实际计算成功 | 2161 点计算量大，S3 上传延迟超过 23s 重试窗口 |
| MCP 认证失败 | 容器重建后 MCP 初始化 0 工具，`API密钥验证失败: 无效的API Key` | 本地缓存清空后调用外部认证服务，token 失效或外部服务不可达 |

---

## 优化项一览

| # | 优化项 | 文件 | 效果 |
|---|--------|------|------|
| 1 | thermodynamic_properties 自动补 MN | `calphaMesh.rs` | 使用 5 元 TDB 且 components 无 MN 时，自动补入 MN=1e-4 并归一化，规避 ZeroDivisionError |
| 2 | ZeroDivisionError 改标 retryable:false | `calphaMesh.rs` | 检测 log_excerpt 含 ZeroDivisionError 时，`retryable: false` + 明确备选方案提示 |
| 3 | 结果文件重试窗口 23s→33s | `calphaMesh.rs` | `RESULT_FILES_MAX_RETRIES` 4→6，覆盖 ternary 等大型任务 S3 延迟 |
| 4 | 服务令牌本地快速路径 | `auth/client.rs` | API Key 与 MCP_TOKEN/MCP_API_KEY 一致时，直接本地通过，不依赖外部认证 |

---

## 规范化工作

在优化前已完成的基础规范化，确保 MCP 服务在 LLM 输入不完美时仍能正确运行：

| # | 规范化项 | 位置 | 说明 |
|---|----------|------|------|
| 1 | 成分与组元清洗 | `sanitize_composition_and_components` | 移除 composition=0 的元素，同步更新 components 列表，重新归一化；避免 LLM 提交 MN=0 等导致后端异常 |
| 2 | 相列表动态过滤 | `filter_phases_for_components` | 根据实际 components 过滤 activated_phases，移除依赖已剔除元素的相（如 MN 移除后剔除 ALPHA_ALFEMNSI） |
| 3 | 空结果兜底检测 | `handle_point_result` | phases/phase_fractions 为空时返回 `empty_result` + log_excerpt，避免静默失败 |
| 4 | 错误日志提取 | `fetch_log_excerpt` | 下载 output.log，提取含错误关键词或最后 20 行，注入 no_result_files/task_failed 的 log_excerpt 字段 |
| 5 | S3 presigned URL 处理 | `download_file_content` | 检测 URL 含 X-Amz-Signature 时不再追加 Authorization 头，避免 403 |
| 6 | 结果文件重试策略 | `get_task_result` | 初始等待 3s + 多次重试（现为 6 次×5s），应对 S3 异步上传延迟 |

**应用范围**：`sanitize` 与 `filter_phases` 已接入 point、line、scheil、binary、ternary、thermodynamic_properties 等所有提交类工具。

---

## 详细变更

### 1. thermodynamic_properties 自动补 MN

**位置**: `calphaMesh.rs` → `submit_thermo_properties_task`

**逻辑**:
```
sanitize_composition_and_components 之后
  ↓
若 tdb_file == "TOPDB-Al-Si-Mg-Fe-Mn_by_wf.TOPDB" 且 components 不含 "MN"
  ↓
补入 MN=1e-4，其余成分按 (1 - 1e-4) 比例缩放，components 追加 MN 并排序
  ↓
filter_phases_for_components 照常执行
```

**依据**: 直连 API 测试证实 MN=1e-4 时 thermodynamic_properties 可成功；1e-4 量级对铝合金热力学性质影响可忽略。

---

### 2. no_result_files 中 ZeroDivisionError 特殊处理

**位置**: `calphaMesh.rs` → `get_task_result` 内 `!has_actual_result` 分支

**逻辑**:
```rust
let is_zero_division = log_excerpt.contains("ZeroDivisionError");
let retryable = !is_zero_division;
let details = if is_zero_division {
    "后端 CALPHAD solver 数值错误（ZeroDivisionError），调整步数或组成不能解决此问题。\
     建议：跳过 thermodynamic_properties 改用 point_calculation 在关键温度点单独计算，\
     或继续其他任务（binary/ternary/Scheil）。"
} else {
    "请根据 log_excerpt 中的错误信息调整参数后重新提交"
};
```

**效果**: Agent 不再无效重试，收到明确备选方案指引。

---

### 3. 结果文件重试窗口扩展

**位置**: `calphaMesh.rs` → `get_task_result` 常量

| 常量 | 原值 | 新值 |
|------|------|------|
| `RESULT_FILES_MAX_RETRIES` | 4 | 6 |
| 总等待时间 | 3 + 4×5 = 23 s | 3 + 6×5 = 33 s |

**说明**: 后端将任务标为 completed 后，结果文件写入对象存储存在异步延迟（实测 5~25 s），ternary 等大型任务可能更长。

---

### 4. 服务令牌本地快速路径（认证）

**位置**: `auth/client.rs` → `verify_api_key`

**逻辑**:
```
verify_api_key 入口
  ↓
若 MCP_TOKEN 或 MCP_API_KEY 环境变量非空，且 api_key 与之完全一致
  ↓
直接返回 AuthResult（用户=service），不查缓存、不调外部认证
  ↓
否则走原有 本地缓存 → 外部认证 流程
```

**用途**: 内部服务调用（alalloy-backend → topmat-mcp）时，当外部认证服务不可用或 token 失效，仍可通过配置的 MCP_TOKEN 正常连接。

---

## 任务完成判定机制（补充说明）

`get_task_result` 采用**两步验证**：

1. **轮询状态码**：每 8s 轮询，直到 `completed` / `failed` / `error`
2. **读取结果文件**：状态为 completed 后，等待 S3 上传（3s 初始 + 最多 6 次×5s 重试），检查是否存在 `results.json`、`binary_equilibrium.json`、`thermodynamic_properties.json` 等

**关键点**: 后端在 ZeroDivisionError 等 crash 时仍可能返回 `completed`，因此**不能仅凭状态码判定成功**，必须依赖结果文件存在性。

---

## 部署与验证

### 部署流程（参考 SERVER_DEPLOYMENT.md）

1. **MCP 服务（TopMat-LLM-Server）**
   - 本地 Docker 交叉编译 → 提取二进制 → SCP 上传 `/opt/topmat/TopMat-LLM`
   - 服务器: `docker build -t topmat-mcp:latest .` → `docker compose up -d topmat-mcp`

2. **Token 配置**
   - `/opt/topmat/.env` 与 `/opt/alalloy/.env` 中 `MCP_TOKEN`、`MCP_API_KEY` 须与 alalloy-backend 使用的 token 一致
   - 更新 token 后需重启 topmat-mcp 与 alalloy-backend

### 验证指标

- `docker logs alalloy-backend` 中应出现 `MCP 初始化成功: 12/43 个工具`、`工具分配: analysisExpert=13个 (本地1 + onnx3 + calphad9)`
- thermodynamic_properties 任务（4 元素 AL/SI/MG/FE）应能正常返回结果（自动补 MN 生效）

---

## 周报总结

- **成分与组元规范化**：自动移除 composition=0 的元素并归一化，动态过滤相列表，避免 LLM 输入 MN=0 等导致后端 ZeroDivisionError
- **thermodynamic_properties 自动补 MN**：5 元 TDB 缺 MN 时自动补入 1e-4，规避 solver 数值崩溃
- **错误诊断增强**：no_result_files/task_failed 时注入 output.log 摘要，ZeroDivisionError 时 retryable=false 并给出明确备选方案
- **结果文件获取**：S3 presigned URL 不追加 Authorization、重试窗口 23s→33s，覆盖 ternary 等大型任务延迟
- **认证容错**：MCP_TOKEN 本地快速路径，外部认证不可用时仍可正常连接
