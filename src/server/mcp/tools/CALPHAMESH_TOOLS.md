# CalphaMesh 热力学 MCP 工具说明文档

> **版本**: v2.1  
> **对应源文件**: `calphaMesh.rs`  
> **设计依据**: `docs/CALPHA_MCP_CONTRACT_DESIGN.md`  
> **后端服务**: `https://api.topmaterial-tech.com`（TopMat 热力学计算平台）

---

## 目录

- [概述](#概述)
- [调用流程](#调用流程)
- [TDB 数据库说明](#tdb-数据库说明)
- [工具详情](#工具详情)
  - [calphamesh_submit_point_task](#1-calphamesh_submit_point_task)
  - [calphamesh_submit_line_task](#2-calphamesh_submit_line_task)
  - [calphamesh_submit_scheil_task](#3-calphamesh_submit_scheil_task)
  - [calphamesh_get_task_result](#4-calphamesh_get_task_result)
  - [calphamesh_get_task_status](#5-calphamesh_get_task_status)
  - [calphamesh_list_tasks](#6-calphamesh_list_tasks)
- [前置校验规则](#前置校验规则)
- [错误处理规范](#错误处理规范)
- [输出结构参考](#输出结构参考)
- [扩展工具（第二阶段）](#扩展工具第二阶段)
- [扩展数据库说明](#扩展数据库说明)

---

## 概述

CalphaMesh 工具集基于 CALPHAD 方法，对金属合金体系进行热力学计算，支持三类核心计算任务：

| 计算类型 | 工具 | 适用场景 |
|---------|------|---------|
| **点计算**（Point） | `calphamesh_submit_point_task` | 单一温度-成分状态下的平衡相分析 |
| **线计算**（Line） | `calphamesh_submit_line_task` | 温度或成分连续扫描，绘制性质曲线 |
| **Scheil 凝固** | `calphamesh_submit_scheil_task` | 非平衡凝固模拟，液/固相分数随温度变化 |

所有计算任务均为**异步执行**，提交后获得 `task_id`，再通过 `calphamesh_get_task_result` 等待并获取结构化结果。

### 认证机制

- **服务认证**：请求头 `Authorization: Bearer <TopMat-API-Key>`
- **CalphaMesh API Key 注入**：服务端从请求头自动提取 Bearer Token，在调用工具时注入为 `api_key` 参数——LLM 无需感知此字段

---

## 调用流程

```
┌─────────────────────────────────────────────────────────┐
│  标准 2 步工作流（推荐）                                    │
└─────────────────────────────────────────────────────────┘

  Agent
    │
    ├─1─► calphamesh_submit_*_task(components, composition, ...)
    │         │
    │         └─► 立即返回 { task_id, status: "pending", next_action }
    │
    └─2─► calphamesh_get_task_result(task_id, result_mode="summary")
              │
              ├─ 内部轮询（每 8 秒）直至 completed / failed / timeout
              │
              └─► 返回结构化结果（data_summary + derived_metrics + files）


┌─────────────────────────────────────────────────────────┐
│  可选：非阻塞查询                                          │
└─────────────────────────────────────────────────────────┘

  Agent
    ├─►  calphamesh_get_task_status(task_id)   ← 立即返回当前状态
    └─►  calphamesh_list_tasks(page)            ← 查看历史任务列表
```

---

## TDB 数据库说明

TDB（热力学数据库）文件决定了可计算的元素体系。**`components` 中的所有元素必须包含在所选 TDB 的元素集内**，否则校验失败。

| TDB 文件名 | 适用体系 | 包含元素 |
|-----------|---------|---------|
| `FE-C-SI-MN-CU-TI-O.TDB` | 铁基合金（钢、铸铁） | Fe, C, Si, Mn, Cu, Ti, O |
| `B-C-SI-ZR-HF-LA-Y-TI-O.TDB` | 硼化物/硅化物/难熔金属 | B, C, Si, Zr, Hf, La, Y, Ti, O |

> **维护说明**：TDB 映射表统一维护在 `calphaMesh.rs` 的 `TDB_ELEMENT_MAP` 常量中，新增数据库仅需在此处追加一条记录，校验逻辑和 schema 枚举自动派生。

---

## 工具详情

### 1. `calphamesh_submit_point_task`

**功能**：提交单点热力学平衡计算。给定合金组成（原子分数）和温度，计算该状态下的稳定相、相分数及热力学性质。

**输入参数**：

| 参数 | 类型 | 必填 | 说明 |
|------|------|------|------|
| `components` | `string[]` | ✅ | 元素列表，大写，如 `["FE", "C", "SI"]`。至少 2 个元素 |
| `composition` | `object` | ✅ | 各元素原子分数，键大写，值 0~1，**所有值之和必须等于 1.0** |
| `temperature` | `number` | ✅ | 计算温度，单位 K，范围 200~6000 |
| `tdb_file` | `string` | ✅ | TDB 数据库文件名，枚举值见上表 |

**示例输入**：
```json
{
  "components": ["FE", "C", "SI"],
  "composition": {"FE": 0.95, "C": 0.03, "SI": 0.02},
  "temperature": 1273.15,
  "tdb_file": "FE-C-SI-MN-CU-TI-O.TDB"
}
```

**输出**（提交成功）：
```json
{
  "task_id": 18901,
  "status": "pending",
  "task_type": "point_calculation",
  "summary": "Point 计算任务已提交：FE-C-SI 体系，1273.15 K",
  "estimated_wait_seconds": 15,
  "next_action": "调用 calphamesh_get_task_result(task_id=18901) 等待并获取结果"
}
```

---

### 2. `calphamesh_submit_line_task`

**功能**：提交线性扫描计算。在起止状态之间等分 `steps` 步，计算一系列平衡状态（可同时扫描温度和成分）。

**输入参数**：

| 参数 | 类型 | 必填 | 说明 |
|------|------|------|------|
| `components` | `string[]` | ✅ | 元素列表，大写 |
| `start_composition` | `object` | ✅ | 扫描起始成分，原子分数之和等于 1.0 |
| `end_composition` | `object` | ✅ | 扫描终止成分，原子分数之和等于 1.0 |
| `start_temperature` | `number` | ✅ | 起始温度，K，200~6000 |
| `end_temperature` | `number` | ✅ | 终止温度，K，200~6000 |
| `steps` | `integer` | ✅ | 扫描步数，范围 2~500，默认 50 |
| `tdb_file` | `string` | ✅ | TDB 数据库文件名 |

**示例输入**（固定成分，仅扫温度）：
```json
{
  "components": ["FE", "C", "SI"],
  "start_composition": {"FE": 0.95, "C": 0.03, "SI": 0.02},
  "end_composition":   {"FE": 0.95, "C": 0.03, "SI": 0.02},
  "start_temperature": 800.0,
  "end_temperature":   1500.0,
  "steps": 50,
  "tdb_file": "FE-C-SI-MN-CU-TI-O.TDB"
}
```

**输出**（提交成功）：
```json
{
  "task_id": 18902,
  "status": "pending",
  "task_type": "line_calculation",
  "summary": "Line 计算任务已提交：FE-C-SI 体系，800.0→1500.0 K，50 步（51 个数据点）",
  "estimated_wait_seconds": 20,
  "next_action": "调用 calphamesh_get_task_result(task_id=18902) 等待并获取结果"
}
```

---

### 3. `calphamesh_submit_scheil_task`

**功能**：提交 Scheil 凝固模拟任务。从指定起始温度逐步降温，模拟非平衡凝固过程（固相中无扩散），输出液相分数随温度变化的凝固曲线。

**输入参数**：

| 参数 | 类型 | 必填 | 说明 |
|------|------|------|------|
| `components` | `string[]` | ✅ | 元素列表，大写 |
| `composition` | `object` | ✅ | 初始合金成分，原子分数之和等于 1.0 |
| `start_temperature` | `number` | ✅ | 起始温度，K，范围 500~6000（通常高于液相线 50~200 K） |
| `temperature_step` | `number` | 否 | 每步降温幅度，K，范围 0.1~50，默认 1.0 |
| `tdb_file` | `string` | ✅ | TDB 数据库文件名 |

**示例输入**：
```json
{
  "components": ["FE", "C", "SI"],
  "composition": {"FE": 0.95, "C": 0.03, "SI": 0.02},
  "start_temperature": 1823.15,
  "temperature_step": 1.0,
  "tdb_file": "FE-C-SI-MN-CU-TI-O.TDB"
}
```

**输出**（提交成功）：
```json
{
  "task_id": 18903,
  "status": "pending",
  "task_type": "scheil_solidification",
  "summary": "Scheil 凝固任务已提交：FE-C-SI 体系，起始温度 1823.15 K，步长 1.0 K",
  "estimated_wait_seconds": 30,
  "next_action": "调用 calphamesh_get_task_result(task_id=18903) 等待并获取结果"
}
```

---

### 4. `calphamesh_get_task_result`

**功能**：等待计算任务完成并返回结构化结果（**阻塞语义**）。内部每 8 秒轮询一次，直到任务进入终态或超时。

**输入参数**：

| 参数 | 类型 | 必填 | 说明 |
|------|------|------|------|
| `task_id` | `integer` | ✅ | 任务 ID，由 `submit_*` 工具返回 |
| `timeout_seconds` | `integer` | 否 | 最大等待秒数，10~90，默认 60 |
| `result_mode` | `string` | 否 | `"summary"`（默认）或 `"full"` |

**`result_mode` 说明**：

| 模式 | 包含内容 | 适用场景 |
|------|---------|---------|
| `summary` | `data_summary` + `derived_metrics` + `files` | LLM 推理、前端展示（默认） |
| `full` | `summary` 全部内容 + `raw_data`（完整行列数据） | 导出、二次分析 |

**任务类型检测逻辑**（自动，基于结果文件名）：

```
results.json         存在 → point_calculation
scheil_solidification.json 存在 → scheil_solidification
*.csv                存在 → line_calculation
```

**Point 任务输出示例**（summary 模式）：
```json
{
  "task_type": "point_calculation",
  "status": "completed",
  "result": {
    "temperature": 1273.15,
    "phases": "FCC_A1",
    "phase_fractions": {"FCC_A1": 1.0},
    "GM": -45832.6,
    "derived_metrics": {
      "dominant_phase": "FCC_A1",
      "phase_count": 1
    }
  },
  "units": {
    "temperature": "K",
    "GM": "J/mol",
    "HM": "J/mol",
    "SM": "J/(mol·K)",
    "CPM": "J/(mol·K)",
    "chemical_potentials": "J/mol"
  },
  "files": {
    "results.json": "https://storage.example.com/tasks/18901/results.json?token=...",
    "output.log":   "https://storage.example.com/tasks/18901/output.log?token=..."
  }
}
```

**Line 任务输出示例**（summary 模式）：
```json
{
  "task_id": 18902,
  "task_type": "line_calculation",
  "status": "completed",
  "result": {
    "data_summary": {
      "total_rows": 51,
      "shown_rows": 20,
      "temperature_range": {"start": 800.0, "end": 1500.0},
      "columns": ["T/K", "P/Pa", "Phase", "f(BCC_A2)", "f(FCC_A1)", "GM/J/mol"],
      "rows": [
        {"T/K": 800.0, "Phase": "BCC_A2", "f(BCC_A2)": 1.0, "GM/J/mol": -35000.0},
        "...（前 20 行，共 51 行）"
      ],
      "representative_rows": {
        "first":  {"T/K": 800.0,  "Phase": "BCC_A2"},
        "middle": {"T/K": 1150.0, "Phase": "FCC_A1"},
        "last":   {"T/K": 1500.0, "Phase": "FCC_A1"}
      }
    },
    "derived_metrics": {
      "phases_encountered": ["BCC_A2", "FCC_A1"],
      "property_extrema": {
        "GM/J/mol": {
          "min": {"value": -65000.0, "temperature_K": 1273.15},
          "max": {"value": -35000.0, "temperature_K": 800.0}
        }
      }
    }
  },
  "files": {
    "table_2.csv": "https://storage.example.com/tasks/18902/table_2.csv?token=...",
    "output.log":  "https://storage.example.com/tasks/18902/output.log?token=..."
  }
}
```

**Scheil 任务输出示例**（summary 模式）：
```json
{
  "task_id": 18903,
  "task_type": "scheil_solidification",
  "status": "completed",
  "result": {
    "data_summary": {
      "converged": true,
      "method": "scheil",
      "total_steps": 823,
      "temperature_range": {"liquidus_K": 1803.4, "solidus_K": 1414.0},
      "key_points": [
        {"temperature_K": 1803.4, "liquid_fraction": 1.0,  "solid_fraction": 0.0},
        {"temperature_K": 1640.0, "liquid_fraction": 0.5,  "solid_fraction": 0.5},
        {"temperature_K": 1414.0, "liquid_fraction": 0.0,  "solid_fraction": 1.0}
      ]
    },
    "derived_metrics": {
      "freezing_range_K": 389.4,
      "t_at_liquid_fraction_0_9_K": 1785.0,
      "t_at_liquid_fraction_0_5_K": 1640.0,
      "t_at_liquid_fraction_0_1_K": 1450.0,
      "curve_monotonic_check": {"liquid_fraction_non_increasing": true}
    }
  },
  "files": {
    "scheil_solidification.json": "https://storage.example.com/tasks/18903/scheil_solidification.json?token=...",
    "output.log": "https://storage.example.com/tasks/18903/output.log?token=..."
  }
}
```

**超时情况输出**：
```json
{
  "task_id": 18903,
  "status": "still_running",
  "elapsed_seconds": 63,
  "retry_after_seconds": 30,
  "message": "任务仍在计算中，请 30 秒后再次调用 calphamesh_get_task_result(task_id=18903)"
}
```

---

### 5. `calphamesh_get_task_status`

**功能**：非阻塞地查询任务当前状态，立即返回，不等待计算完成。

> **使用建议**：大多数场景直接使用 `calphamesh_get_task_result`（自动等待）。此工具适用于需要展示实时进度、或在等待期间执行其他操作的场景。

**输入参数**：

| 参数 | 类型 | 必填 | 说明 |
|------|------|------|------|
| `task_id` | `integer` | ✅ | 任务 ID |

**输出示例**：
```json
{
  "task_id": 18902,
  "status": "running",
  "task_type": "topthermo_next",
  "title": "Line-Task-1741234567",
  "created_at": "2026-01-01T10:00:00Z",
  "updated_at": "2026-01-01T10:00:15Z",
  "result_ready": false,
  "next_action": "任务仍在运行中，调用 calphamesh_get_task_result(task_id=18902) 等待完成"
}
```

**`status` 枚举值说明**：

| 值 | 含义 |
|----|------|
| `pending` | 已提交，等待调度 |
| `running` | 计算中 |
| `completed` | 计算完成，可获取结果 |
| `failed` | 计算失败 |
| `error` | 系统错误 |

---

### 6. `calphamesh_list_tasks`

**功能**：分页查询当前用户的历史任务列表，用于查找历史 `task_id` 或确认是否已有相同计算。

**输入参数**：

| 参数 | 类型 | 必填 | 说明 |
|------|------|------|------|
| `page` | `integer` | 否 | 页码，从 1 开始，默认 1 |
| `items_per_page` | `integer` | 否 | 每页数量，1~100，默认 20 |

**输出示例**：
```json
{
  "page": 1,
  "total_pages": 5,
  "items_per_page": 20,
  "tasks": [
    {
      "task_id": 18903,
      "status": "completed",
      "task_type": "topthermo_next",
      "title": "Scheil-Task-1741234600",
      "created_at": "2026-01-01T10:05:00Z"
    },
    {
      "task_id": 18902,
      "status": "completed",
      "task_type": "topthermo_next",
      "title": "Line-Task-1741234500",
      "created_at": "2026-01-01T10:00:00Z"
    }
  ]
}
```

---

## 前置校验规则

所有 `submit_*` 工具在发送 API 请求**之前**，在服务端执行以下校验（校验失败直接返回错误，不消耗后端配额）：

| 校验项 | 规则 | 涉及工具 |
|-------|------|---------|
| **组分原子分数之和** | `sum(composition.values()) == 1.0`（容差 1e-6） | Point / Line（start+end）/ Scheil |
| **components 与 composition 键一致性** | 两者元素集完全相同 | Point / Line / Scheil |
| **TDB 白名单校验** | `tdb_file` 必须是枚举值之一 | 全部 |
| **TDB 元素覆盖校验** | `components` 中所有元素必须在 TDB 的元素集内 | 全部 |
| **温度范围** | Point/Line: 200~6000 K；Scheil: 500~6000 K | 对应工具 |
| **steps 范围** | 2~500 | Line |
| **temperature_step 范围** | 0.1~50 K | Scheil |

**校验失败返回示例**：
```json
{
  "code": -32603,
  "message": "工具调用失败: 组分原子分数之和为 1.100000，必须等于 1.0（实际：FE=0.500000, C=0.300000, SI=0.300000）"
}
```

---

## 错误处理规范

| 错误来源 | 错误码 / 字段 | 说明 | 是否可重试 |
|---------|-------------|------|-----------|
| **前置校验失败** | MCP `error.code=-32603` | 参数不合法，修正后重试 | ✅ 修正后 |
| **API Key 缺失** | `MissingParameter("api_key")` | Bearer Token 未传或注入失败 | ✅ 传入后 |
| **HTTP 网络错误** | `HttpError(...)` | 网络不可达，可稍后重试 | ✅ |
| **后端 API 错误** | `ApiError { status, message }` | 后端拒绝请求，检查参数 | 视情况 |
| **任务计算失败** | `status: "task_failed"` | 后端计算失败，检查体系 | ✅ 修正后 |
| **超时** | `status: "still_running"` | 计算仍在进行，再次调用 | ✅ 直接重试 |

---

## 输出结构参考

### 单位约定

| 字段 | 单位 |
|------|------|
| 温度 | K（开尔文） |
| 压力 | Pa（帕斯卡） |
| GM（摩尔吉布斯自由能） | J/mol |
| HM（摩尔焓） | J/mol |
| SM（摩尔熵） | J/(mol·K) |
| CPM（摩尔热容） | J/(mol·K) |
| 化学势 | J/mol |
| 相分数 | 无量纲（0~1） |
| 原子分数 | 无量纲（0~1） |

### result_mode 内容对比

```
summary 模式（默认）
├── data_summary       基础摘要：行列数、温度范围、前 N 行、代表行
├── derived_metrics    服务端预计算的精华指标（均值/峰值/相变温度等）
└── files              结果文件的预签名 URL（JSON / CSV / PNG / LOG）

full 模式（在 summary 基础上追加）
└── raw_data           完整行列数据（可能数百至数千行）
```

---

## 扩展工具（第二阶段）

以下工具已在 `calphaMesh.rs` 中实现，但**尚未注册到 MCP**，待第二阶段启用：

| 工具名 | 计算类型 | 状态 |
|-------|---------|------|
| `calphamesh_submit_binary_task` | 二元平衡相图 | 待注册 |
| `calphamesh_submit_ternary_task` | 三元相图 | 待注册 |
| `calphamesh_submit_boiling_point_task` | 沸点计算 | 待注册 |
| `calphamesh_submit_thermodynamic_properties_task` | 热力学性质扫描 | 待注册 |

启用方法：在 `tool_macros.rs` 的 `register_all_mcp_tools!` 宏中取消注释对应条目，并在 `tools/mod.rs` 中追加 `pub use` 导出即可。

---

## 扩展数据库说明

若需支持新的合金体系，需同步更新以下位置：

```
calphaMesh.rs
  └── TDB_ELEMENT_MAP  ← 追加 ("新文件名.TDB", &["元素A", "元素B", ...])

无需修改其他代码——schema 枚举值、TDB 白名单校验、元素覆盖校验均自动派生。
```

**示例**：新增 Al 基合金数据库

```rust
const TDB_ELEMENT_MAP: &[(&str, &[&str])] = &[
    ("FE-C-SI-MN-CU-TI-O.TDB", &["FE", "C", "SI", "MN", "CU", "TI", "O"]),
    ("B-C-SI-ZR-HF-LA-Y-TI-O.TDB", &["B", "C", "SI", "ZR", "HF", "LA", "Y", "TI", "O"]),
    // 新增：
    ("AL-MG-SI-CU-ZN.TDB", &["AL", "MG", "SI", "CU", "ZN"]),
];
```
