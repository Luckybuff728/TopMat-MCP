# CalphaMesh 热力学 MCP 工具说明文档

> **版本**: v3.0（全量 7 工具版）  
> **对应源文件**: `calphaMesh.rs`、`tool_macros.rs`、`tools/mod.rs`  
> **后端服务**: `https://api.topmaterial-tech.com`（TopMat 热力学计算平台）  
> **已验证运行**: 2026-03-07，基于 Al-Si-Mg-Fe-Mn 体系实测

---

## 目录

- [概述](#概述)
- [调用流程](#调用流程)
- [TDB 数据库与激活相说明](#tdb-数据库与激活相说明)
- [提交类工具（7 类计算任务）](#提交类工具)
  - [1. calphamesh_submit_point_task — 单点平衡计算](#1-calphamesh_submit_point_task)
  - [2. calphamesh_submit_line_task — 线性温度扫描](#2-calphamesh_submit_line_task)
  - [3. calphamesh_submit_scheil_task — Scheil 凝固模拟](#3-calphamesh_submit_scheil_task)
  - [4. calphamesh_submit_binary_task — 二元平衡相图](#4-calphamesh_submit_binary_task)
  - [5. calphamesh_submit_ternary_task — 三元等温截面](#5-calphamesh_submit_ternary_task)
  - [6. calphamesh_submit_boiling_point_task — 沸点/熔点计算](#6-calphamesh_submit_boiling_point_task)
  - [7. calphamesh_submit_thermodynamic_properties_task — 热力学性质扫描](#7-calphamesh_submit_thermodynamic_properties_task)
- [结果查询工具](#结果查询工具)
  - [calphamesh_get_task_result — 等待并获取结果](#calphamesh_get_task_result)
  - [calphamesh_get_task_status — 非阻塞状态查询](#calphamesh_get_task_status)
  - [calphamesh_list_tasks — 历史任务列表](#calphamesh_list_tasks)
- [输出数据结构详解](#输出数据结构详解)
- [前置校验规则](#前置校验规则)
- [错误处理规范](#错误处理规范)
- [Al 合金设计场景使用指南](#al-合金设计场景使用指南)
- [扩展数据库说明](#扩展数据库说明)

---

## 概述

CalphaMesh 工具集基于 CALPHAD 方法，通过调用 TopMat 热力学计算平台对金属合金体系进行热力学计算。当前支持 **7 类核心计算任务**，完全覆盖 Al-Si-Mg 系压铸铝合金成分设计的全链路场景：

| # | 工具 | 计算类型 | 核心用途 | 典型耗时 |
|---|------|---------|---------|---------|
| 1 | `calphamesh_submit_point_task` | 单点平衡 | 查看固定温度/成分下的稳定相、相分数、热力学性质 | 10~20 s |
| 2 | `calphamesh_submit_line_task` | 温度扫描 | 绘制相分数-温度曲线，确定相变温度 | 15~30 s |
| 3 | `calphamesh_submit_scheil_task` | Scheil 凝固模拟 | 非平衡凝固路径，获得液/固相线、凝固范围 | 20~60 s |
| 4 | `calphamesh_submit_binary_task` | 二元相图 | 计算 Al-Si 等二元平衡相图 | 30~90 s |
| 5 | `calphamesh_submit_ternary_task` | 三元等温截面 | 计算 Al-Mg-Si 三元等温相图 | 60~180 s |
| 6 | `calphamesh_submit_boiling_point_task` | 沸点/熔点 | 计算纯元素或简单合金的熔/沸点 | 10~30 s |
| 7 | `calphamesh_submit_thermodynamic_properties_task` | 热力学性质扫描 | 输出 GM/HM/SM/CPM 随温度的变化曲线 | 15~40 s |

所有计算任务均为**异步执行**：提交后立即获得 `task_id`，再通过 `calphamesh_get_task_result` 等待并获取结构化结果。

### 认证机制

- **服务认证**：请求头 `Authorization: Bearer <TopMat-API-Key>`
- **API Key 自动注入**：服务端从请求头提取 Bearer Token，在调用工具时注入为 `api_key` 参数——LLM 无需感知此字段

---

## 调用流程

```
┌──────────────────────────────────────────────────────────────┐
│  标准 2 步工作流（推荐）                                         │
└──────────────────────────────────────────────────────────────┘

  Agent
    │
    ├─ Step 1 ─► calphamesh_submit_*_task(...)
    │               └─► 立即返回 { task_id, status: "pending", estimated_wait_seconds, next_action }
    │
    └─ Step 2 ─► calphamesh_get_task_result(task_id, timeout_seconds=80)
                    ├─ 内部每 8 秒轮询一次，直到 completed / failed / timeout
                    └─► 返回完整结构化结果


┌──────────────────────────────────────────────────────────────┐
│  可选：非阻塞查询（适合进度展示、并行执行其他操作）                  │
└──────────────────────────────────────────────────────────────┘

  Agent
    ├─► calphamesh_get_task_status(task_id)  ← 立即返回当前状态，不等待
    └─► calphamesh_list_tasks(page)           ← 查看历史任务列表
```

**工作流说明**：
1. 同一 `task_id` 结果可重复查询（结果会缓存）
2. 若 `get_task_result` 超时，返回 `"status": "still_running"`，直接再次调用即可继续等待
3. 每次 `submit_*` 都会创建新任务——若要避免重复计算，先用 `calphamesh_list_tasks` 确认是否已有结果

---

## TDB 数据库与激活相说明

### 可用数据库

| TDB 文件名 | 适用体系 | 包含元素 | 推荐用途 |
|-----------|---------|---------|---------|
| `Al-Si-Mg-Fe-Mn_by_wf.TDB` | **Al 基压铸铝合金** | AL, SI, MG, FE, MN | Al-Si-Mg 压铸铝合金（A380/ADC12 类） |
| `FE-C-SI-MN-CU-TI-O.TDB` | 铁基合金 | FE, C, SI, MN, CU, TI, O | 钢、铸铁计算 |
| `B-C-SI-ZR-HF-LA-Y-TI-O.TDB` | 难熔金属/硼化物 | B, C, SI, ZR, HF, LA, Y, TI, O | 硼化物/硅化物计算 |

> **维护说明**：TDB 映射表统一在 `calphaMesh.rs` 的 `TDB_ELEMENT_MAP` 常量中维护，新增数据库仅需在此处追加一条记录，schema 枚举和元素校验自动派生。

### Al 数据库推荐激活相

> 以下相名已通过 `http_api_payload参考.md` 实测验证，是后端 topthermo-next 接受的正式名称。

**5 元 Al-Si-Mg-Fe-Mn（point/line/scheil/thermo 任务）**：
```
LIQUID, FCC_A1, DIAMOND_A4, HCP_A3, BCC_A2, CBCC_A12,
BETA_ALMG, EPSILON_ALMG, GAMMA_ALMG, MG2SI,
AL5FE2, AL13FE4, ALPHA_ALFESI, BETA_ALFESI, ALPHA_ALFEMNSI, AL4_FEMN
```
共 16 个相，涵盖：
- 液相（LIQUID）和 FCC 铝基体（FCC_A1）
- Si 相（DIAMOND_A4）、Mg 相（HCP_A3、BCC_A2）
- Mg-Al 相（BETA/EPSILON/GAMMA_ALMG）、Mg-Si 相（MG2SI）
- Fe-Al 相（AL5FE2、AL13FE4、ALPHA/BETA_ALFESI）
- Fe-Mn-Si 相（ALPHA_ALFEMNSI）、Mn-Al 相（AL4_FEMN）

**3 元 Al-Mg-Si（ternary 任务）**：
```
LIQUID, FCC_A1, DIAMOND_A4, HCP_A3,
BETA_ALMG, EPSILON_ALMG, GAMMA_ALMG, MG2SI
```

**2 元 Al-Si（binary 任务）**：
```
LIQUID, FCC_A1, DIAMOND_A4
```

> ⚠️ **重要**：这些相名由 `calphaMesh.rs` 中的常量 `AL_5ELEMENT_PHASES`、`AL_TERNARY_PHASES`、`AL_BINARY_PHASES` 自动注入，LLM 调用工具时**无需手动指定**激活相。

---

## 提交类工具

### 1. `calphamesh_submit_point_task`

**功能**：提交单点热力学平衡计算。在固定温度和固定成分下，计算该状态下的稳定相、各相分数、摩尔 Gibbs 自由能（GM）、摩尔焓（HM）、摩尔熵（SM）、摩尔热容（CPM）以及各组元化学势（μ）。

**适用场景**：
- 查看铸造/固溶/时效某一温度点下的相组成
- 计算某成分下是否有液相残留（判断固溶完全性）
- 对比不同成分在同一温度的稳定相差异

#### 输入参数

| 参数 | 类型 | 必填 | 约束 | 说明 |
|------|------|------|------|------|
| `components` | `string[]` | ✅ | 至少 2 个，大写，须在 TDB 中 | 合金组元列表，如 `["AL","SI","MG","FE","MN"]` |
| `composition` | `object` | ✅ | 各值 0~1，**总和 = 1.0** | 各组元原子分数，键大写，如 `{"AL":0.93,"SI":0.04,...}` |
| `temperature` | `number` | ✅ | 200~6000 K | 计算温度（开尔文） |
| `tdb_file` | `string` | ✅ | 枚举值之一 | 热力学数据库文件名 |

#### Al-Si-Mg 示例（5 元体系）

```json
{
  "components": ["AL", "SI", "MG", "FE", "MN"],
  "composition": {
    "AL": 0.93,
    "SI": 0.04,
    "MG": 0.01,
    "FE": 0.015,
    "MN": 0.005
  },
  "temperature": 850.0,
  "tdb_file": "Al-Si-Mg-Fe-Mn_by_wf.TDB"
}
```

#### 提交成功响应

```json
{
  "task_id": 18896,
  "status": "pending",
  "task_type": "point_calculation",
  "summary": "Point 计算任务已提交：AL-SI-MG-FE-MN 体系，850 K",
  "estimated_wait_seconds": 15,
  "next_action": "调用 calphamesh_get_task_result(task_id=18896) 等待并获取结果"
}
```

#### 计算结果（get_task_result 返回）

```json
{
  "task_type": "point_calculation",
  "status": "completed",
  "result": {
    "temperature": 850.0,
    "pressure": 101325.0,
    "phases": "ALPHA_ALFEMNSI+BETA_ALFESI+FCC_A1+LIQUID",
    "phase_fractions": {
      "ALPHA_ALFEMNSI": 0.012,
      "BETA_ALFESI": 0.094,
      "FCC_A1": 0.758,
      "LIQUID": 0.136
    },
    "compositions": {
      "AL": 0.93,
      "FE": 0.015,
      "MG": 0.01,
      "MN": 0.005,
      "SI": 0.04
    },
    "chemical_potentials": {
      "AL": -33123.54,
      "FE": -143203.67,
      "MG": -66954.82,
      "MN": -117453.14,
      "SI": -26923.66
    },
    "thermodynamic_properties": {
      "GM": -35286.71,
      "HM": 15083.74,
      "SM": 59.26,
      "CPM": 31.47
    },
    "derived_metrics": {
      "dominant_phase": "ALPHA_ALFEMNSI+BETA_ALFESI+FCC_A1+LIQUID",
      "phase_count": 4
    }
  },
  "units": {
    "temperature": "K",
    "pressure": "Pa",
    "GM": "J/mol",
    "HM": "J/mol",
    "SM": "J/(mol·K)",
    "CPM": "J/(mol·K)",
    "chemical_potentials": "J/mol"
  },
  "files": {
    "results.json": "https://taskman.fs.skyzcstack.space/.../results.json?...",
    "table.csv":    "https://taskman.fs.skyzcstack.space/.../table.csv?...",
    "output.log":   "https://taskman.fs.skyzcstack.space/.../output.log?..."
  }
}
```

> **结果解读**：850 K（577°C）时合金处于半固态，FCC_A1（铝基体）占 75.8%，含 13.6% 残余液相，存在 Fe/Mn 相间化合物（ALPHA_ALFEMNSI、BETA_ALFESI），说明该温度下仍在固液两相区内。

---

### 2. `calphamesh_submit_line_task`

**功能**：提交线性扫描计算。在起止状态之间等分 `steps` 步，计算每个状态的平衡结果。最常用场景是固定成分、扫描温度区间，获得"相分数-温度"曲线（等同于平衡冷却路径）。

**适用场景**：
- 确定合金的平衡液相线和固相线温度（在曲线上找液相分数 = 0 / 1 的拐点）
- 分析各相在热处理温度范围内的析出/溶解行为
- 绘制化学势随温度的变化曲线

#### 输入参数

| 参数 | 类型 | 必填 | 约束 | 说明 |
|------|------|------|------|------|
| `components` | `string[]` | ✅ | 同 point | 合金组元列表 |
| `start_composition` | `object` | ✅ | 总和 = 1.0 | 扫描起始成分 |
| `end_composition` | `object` | ✅ | 总和 = 1.0 | 扫描终止成分（定成分扫描时与 start 相同） |
| `start_temperature` | `number` | ✅ | 200~6000 K | 起始温度 |
| `end_temperature` | `number` | ✅ | 200~6000 K，须 > start | 终止温度 |
| `steps` | `integer` | ✅ | 2~500，默认 50 | 扫描步数（实际输出 steps+1 个数据点） |
| `tdb_file` | `string` | ✅ | 枚举值之一 | 数据库文件 |

> **定成分温度扫描**（最常用）：`start_composition` 与 `end_composition` 完全相同，仅通过温度区间变化扫描。

#### Al-Si-Mg 示例（定成分温度扫描，500~900 K）

```json
{
  "components": ["AL", "SI", "MG", "FE", "MN"],
  "start_composition": {"AL": 0.93, "SI": 0.04, "MG": 0.01, "FE": 0.015, "MN": 0.005},
  "end_composition":   {"AL": 0.93, "SI": 0.04, "MG": 0.01, "FE": 0.015, "MN": 0.005},
  "start_temperature": 500.0,
  "end_temperature":   900.0,
  "steps": 8,
  "tdb_file": "Al-Si-Mg-Fe-Mn_by_wf.TDB"
}
```

#### 提交成功响应

```json
{
  "task_id": 18897,
  "status": "pending",
  "task_type": "line_calculation",
  "summary": "Line 计算任务已提交：AL-SI-MG-FE-MN 体系，500.0→900.0 K，8 步（9 个数据点）",
  "estimated_wait_seconds": 20,
  "next_action": "调用 calphamesh_get_task_result(task_id=18897) 等待并获取结果"
}
```

#### 计算结果（get_task_result 返回，summary 模式）

```json
{
  "task_id": 18897,
  "task_type": "line_calculation",
  "status": "completed",
  "result": {
    "data_summary": {
      "total_rows": 50,
      "shown_rows": 20,
      "temperature_range": {
        "start": 500.0,
        "end": 900.0
      },
      "columns": ["T/K", "P/Pa", "Phase", "f(FCC_A1)", "f(LIQUID)", "f(BETA_ALFESI)", "MU(AL)/J/mol", "MU(SI)/J/mol", "..."],
      "rows": [
        {"T/K": 500.0, "Phase": "FCC_A1", "f(FCC_A1)": 1.0, "MU(AL)/J/mol": -52000.0},
        {"T/K": 550.0, "Phase": "FCC_A1+MG2SI", "f(FCC_A1)": 0.98, "f(MG2SI)": 0.02},
        "...（共 50 行，前 20 行展示）"
      ],
      "representative_rows": {
        "first":  {"T/K": 500.0, "Phase": "FCC_A1"},
        "middle": {"T/K": 700.0, "Phase": "FCC_A1+BETA_ALFESI"},
        "last":   {"T/K": 900.0, "Phase": "FCC_A1+LIQUID"}
      }
    },
    "derived_metrics": {
      "phases_encountered": ["FCC_A1", "LIQUID", "MG2SI", "BETA_ALFESI", "ALPHA_ALFEMNSI"],
      "property_extrema": {
        "MU(AL)/J/mol": {
          "min": {"value": -62000.0, "temperature_K": 900.0},
          "max": {"value": -35000.0, "temperature_K": 500.0}
        }
      }
    }
  },
  "files": {
    "table_2.csv": "https://taskman.fs.skyzcstack.space/.../table_2.csv?...",
    "output.log":  "https://taskman.fs.skyzcstack.space/.../output.log?..."
  }
}
```

> **CSV 列名格式**：`T/K`（温度）、`P/Pa`（压力）、`Phase`（相名字符串）、`f(相名)`（各相分数）、`MU(元素)/J/mol`（化学势）。`steps=8` 但 `total_rows=50` 因为多相体系每个温度点有多行（每个相一行）。

---

### 3. `calphamesh_submit_scheil_task`

**功能**：提交 Scheil-Gulliver 非平衡凝固模拟。从指定起始温度逐步降温，假设固相中无扩散（固相不均匀化），模拟实际铸造凝固过程。输出液相分数随温度变化的凝固曲线、析出相序列及关键温度节点。

**适用场景**：
- 预测实际铸造（压铸）条件下的凝固路径
- 获取液相线温度（f_liquid=1 时）和实际固相线/共晶温度
- 分析各相的析出顺序（谁先析出、什么温度开始析出）
- 计算凝固范围（液相线 - 最终固化温度），评估铸造热裂倾向

#### 输入参数

| 参数 | 类型 | 必填 | 约束 | 说明 |
|------|------|------|------|------|
| `components` | `string[]` | ✅ | 至少 2 个 | 合金组元列表 |
| `composition` | `object` | ✅ | 总和 = 1.0 | 初始合金成分（原子分数） |
| `start_temperature` | `number` | ✅ | 500~6000 K | 起始温度，**必须高于实际液相线**（通常高 50~200 K） |
| `temperature_step` | `number` | 否 | 0.1~50 K，默认 1.0 | 每步降温幅度，越小精度越高但耗时越长 |
| `tdb_file` | `string` | ✅ | 枚举值之一 | 数据库文件 |

> **start_temperature 选取**：Al 合金液相线通常在 850~930 K，推荐设置 1050~1100 K 以确保从完全液态开始。

#### Al-Si-Mg 示例（5 元体系，压铸铝合金 A380 类）

```json
{
  "components": ["AL", "SI", "MG", "FE", "MN"],
  "composition": {
    "AL": 0.93,
    "SI": 0.04,
    "MG": 0.01,
    "FE": 0.015,
    "MN": 0.005
  },
  "start_temperature": 1100.0,
  "temperature_step": 5.0,
  "tdb_file": "Al-Si-Mg-Fe-Mn_by_wf.TDB"
}
```

#### 提交成功响应

```json
{
  "task_id": 18898,
  "status": "pending",
  "task_type": "scheil_solidification",
  "summary": "Scheil 凝固任务已提交：AL-SI-MG-FE-MN 体系，起始温度 1100.0 K，步长 5.0 K",
  "estimated_wait_seconds": 30,
  "next_action": "调用 calphamesh_get_task_result(task_id=18898) 等待并获取结果"
}
```

#### 计算结果（get_task_result 返回，summary 模式）

```json
{
  "task_id": 18898,
  "task_type": "scheil_solidification",
  "status": "completed",
  "result": {
    "data_summary": {
      "converged": true,
      "method": "scheil",
      "total_steps": 57,
      "temperature_range": {
        "liquidus_K": 1100.0,
        "solidus_K": 832.02
      },
      "columns": ["T/K", "f(LIQUID)", "f(FCC_A1)", "f(BETA_ALFESI)", "f(ALPHA_ALFEMNSI)", "..."],
      "shown_rows": [
        {"T/K": 1100.0, "f(LIQUID)": 1.0,  "f(FCC_A1)": 0.0},
        {"T/K": 1050.0, "f(LIQUID)": 0.92, "f(FCC_A1)": 0.08},
        {"T/K": 900.0,  "f(LIQUID)": 0.45, "f(FCC_A1)": 0.49, "f(BETA_ALFESI)": 0.06},
        {"T/K": 832.02, "f(LIQUID)": 0.0,  "f(FCC_A1)": 0.78, "f(MG2SI)": 0.05, "...": "..."}
      ],
      "key_points": [
        {"temperature_K": 1100.0, "liquid_fraction": 1.0,  "solid_fraction": 0.0},
        {"temperature_K": 966.01, "liquid_fraction": 0.5,  "solid_fraction": 0.5},
        {"temperature_K": 832.02, "liquid_fraction": 0.0,  "solid_fraction": 1.0}
      ]
    },
    "derived_metrics": {
      "freezing_range_K": 267.98,
      "t_at_liquid_fraction_0_9_K": 1065.0,
      "t_at_liquid_fraction_0_5_K": 966.01,
      "t_at_liquid_fraction_0_1_K": 870.0,
      "curve_monotonic_check": {
        "liquid_fraction_non_increasing": true
      }
    }
  },
  "files": {
    "scheil_solidification.csv":  "https://taskman.fs.skyzcstack.space/.../scheil_solidification.csv?...",
    "scheil_solidification.json": "https://taskman.fs.skyzcstack.space/.../scheil_solidification.json?...",
    "scheil_conditions.json":     "https://taskman.fs.skyzcstack.space/.../scheil_conditions.json?...",
    "output.log":                 "https://taskman.fs.skyzcstack.space/.../output.log?..."
  }
}
```

> **结果解读**：
> - 液相线 1100 K（start_temperature，确认从液态开始）
> - 固相线（最终凝固温度）832 K（559°C）
> - 凝固范围 268 K，相对较宽，说明存在明显的铸造偏析
> - 半凝固温度（f_liquid=50%）约为 966 K（693°C）
> - `total_steps=57` 表示实际模拟了 57 个温度步

#### full 模式（追加 raw_data）

当 `result_mode="full"` 时，在 `result` 下额外追加：

```json
{
  "raw_data": [
    {"T/K": 1100.0, "f(LIQUID)": 1.0, "f(FCC_A1)": 0.0},
    {"T/K": 1095.0, "f(LIQUID)": 0.98, "f(FCC_A1)": 0.02},
    "...（全部 57 行）"
  ]
}
```

---

### 4. `calphamesh_submit_binary_task`

**功能**：提交二元平衡相图计算。在给定温度区间和成分区间内，计算 Al-Si 等二元体系的平衡相图，输出相区边界、相区标注信息及 Plotly 可视化数据。

**适用场景**：
- 计算 Al-Si 二元相图，确定共晶成分和共晶温度
- 分析二元合金的固溶极限随温度的变化
- 为多元合金设计提供二元子系统基础参考

#### 输入参数

| 参数 | 类型 | 必填 | 约束 | 说明 |
|------|------|------|------|------|
| `components` | `string[]` | ✅ | **恰好 2 个**，大写 | 二元体系元素，如 `["AL","SI"]` |
| `start_composition` | `object` | ✅ | 总和 = 1.0 | Al 侧（富铝端）成分，如 `{"AL":1.0,"SI":0.0}` |
| `end_composition` | `object` | ✅ | 总和 = 1.0 | Si 侧（富 Si 端）成分，如 `{"AL":0.7,"SI":0.3}` |
| `start_temperature` | `number` | ✅ | 200~6000 K | 相图下限温度 |
| `end_temperature` | `number` | ✅ | 须 > start | 相图上限温度 |
| `tdb_file` | `string` | ✅ | 枚举值之一 | 数据库文件 |

> **注意**：两端点成分定义了相图的横坐标范围。对 Al-Si 压铸合金，`0~30 mol% Si` 涵盖所有常用牌号。

#### Al-Si 二元相图示例

```json
{
  "components": ["AL", "SI"],
  "start_composition": {"AL": 1.0, "SI": 0.0},
  "end_composition":   {"AL": 0.7, "SI": 0.3},
  "start_temperature": 500.0,
  "end_temperature":   1200.0,
  "tdb_file": "Al-Si-Mg-Fe-Mn_by_wf.TDB"
}
```

#### 提交成功响应

```json
{
  "task_id": 18899,
  "status": "pending",
  "task_type": "binary_equilibrium",
  "summary": "Binary 相图任务已提交：AL-SI 体系，温度范围 500.0~1200.0 K",
  "estimated_wait_seconds": 40,
  "next_action": "调用 calphamesh_get_task_result(task_id=18899) 等待并获取结果"
}
```

#### 计算结果（get_task_result 返回）

```json
{
  "task_id": 18899,
  "task_type": "binary_equilibrium",
  "status": "completed",
  "result": {
    "data_summary": {
      "system": "Al-Si",
      "phase_count": 3,
      "boundary_count": 11,
      "note": "二元相图已计算完成，完整图形数据见 files.binary_equilibrium.json"
    }
  },
  "files": {
    "binary_equilibrium.json": "https://taskman.fs.skyzcstack.space/.../binary_equilibrium.json?...",
    "output.log":              "https://taskman.fs.skyzcstack.space/.../output.log?..."
  }
}
```

> **结果字段说明**：
> - `phase_count`：相图中出现的相的数量（Al-Si 体系为 LIQUID/FCC_A1/DIAMOND_A4，共 3 相）
> - `boundary_count`：相界线总数（11 条相界线构成完整相图）
> - `binary_equilibrium.json`：包含完整 Plotly 格式图形数据，可直接用于前端可视化

---

### 5. `calphamesh_submit_ternary_task`

**功能**：提交三元等温截面计算。在给定温度下，计算三元体系的 Gibbs 三角相图，输出相区点（相区内所有网格点的相稳定区域标注）、共轭线（tie-line）和三相三角（tie-triangle）数据，直接用于 Plotly 可视化。

**适用场景**：
- 计算 Al-Mg-Si 时效热处理温度（约 500°C/773 K）下的等温截面
- 分析 β-Mg₂Si、β''-MgSi 等析出相的热力学稳定区域
- 为 6xxx 系合金（Al-Mg-Si）成分设计提供相图依据

#### 输入参数

| 参数 | 类型 | 必填 | 约束 | 说明 |
|------|------|------|------|------|
| `components` | `string[]` | ✅ | **恰好 3 个**，大写 | 三元体系元素，如 `["AL","MG","SI"]` |
| `temperature` | `number` | ✅ | 200~6000 K | 等温截面温度（K） |
| `composition_y` | `object` | ✅ | 总和 = 1.0 | 三角形顶点 Y（通常为第 1 元素的纯元素端） |
| `composition_x` | `object` | ✅ | 总和 = 1.0 | 三角形顶点 X（通常为第 2 元素的纯元素端） |
| `composition_o` | `object` | ✅ | 总和 = 1.0 | 三角形顶点 O（通常为第 3 元素的纯元素端） |
| `tdb_file` | `string` | ✅ | 枚举值之一 | 数据库文件 |

#### Al-Mg-Si 三元等温截面示例（773 K，时效温度）

```json
{
  "components": ["AL", "MG", "SI"],
  "temperature": 773.0,
  "composition_y": {"AL": 1.0, "MG": 0.0, "SI": 0.0},
  "composition_x": {"AL": 0.0, "MG": 1.0, "SI": 0.0},
  "composition_o": {"AL": 0.0, "MG": 0.0, "SI": 1.0},
  "tdb_file": "Al-Si-Mg-Fe-Mn_by_wf.TDB"
}
```

#### 提交成功响应

```json
{
  "task_id": 18900,
  "status": "pending",
  "task_type": "ternary_calculation",
  "summary": "Ternary 相图任务已提交：AL-MG-SI 体系，等温截面 773.0 K",
  "estimated_wait_seconds": 60,
  "next_action": "调用 calphamesh_get_task_result(task_id=18900) 等待并获取结果"
}
```

#### 计算结果（get_task_result 返回）

```json
{
  "task_id": 18900,
  "task_type": "ternary_calculation",
  "status": "completed",
  "result": {
    "data_summary": {
      "point_count": 2161,
      "tie_line_count": 218,
      "tie_triangle_count": 575,
      "phases_in_diagram": ["FCC_A1", "HCP_A3", "BETA_ALMG", "MG2SI", "DIAMOND_A4"]
    }
  },
  "files": {
    "ternary_plotly.json":    "https://taskman.fs.skyzcstack.space/.../ternary_plotly.json?...",
    "ternary_equilibrium.png": "https://taskman.fs.skyzcstack.space/.../ternary_equilibrium.png?...",
    "output.log":             "https://taskman.fs.skyzcstack.space/.../output.log?..."
  }
}
```

> **结果字段说明**：
> - `point_count`：三角形内计算网格点数（2161 点代表密集网格）
> - `tie_line_count`：两相区内的共轭线数量（218 条）
> - `tie_triangle_count`：三相区三角形数量（575 个三相三角）
> - `ternary_plotly.json`：Plotly 格式可视化数据，可直接渲染
> - `ternary_equilibrium.png`：预渲染的三元相图图片

---

### 6. `calphamesh_submit_boiling_point_task`

**功能**：在给定压力和温度搜索区间内，计算指定成分的固相线（solidus）、液相线（liquidus）、泡点（bubble point）和露点（dew point）。主要用于纯元素或简单合金的熔点/沸点计算。

**适用场景**：
- 计算纯 Al 的熔点（~933 K）和沸点（~2743 K）
- 验证合金液相线温度（作为 Scheil start_temperature 选取的依据）
- 研究蒸发行为（高温冶金、真空炉应用）

> ⚠️ **注意**：对于多元合金，液相线/固相线计算建议改用 `scheil_solidification`（更准确）或 `line_calculation`（平衡冷却路径）。`boiling_point` 更适合纯元素或 2 元简单合金。

#### 输入参数

| 参数 | 类型 | 必填 | 约束 | 说明 |
|------|------|------|------|------|
| `components` | `string[]` | ✅ | 至少 1 个，大写 | 组元列表，纯元素时只填 1 个 |
| `composition` | `object` | ✅ | 总和 = 1.0 | 各组元原子分数 |
| `pressure` | `number` | ✅ | > 0（单位 Pa） | **直接填 Pa，常压为 101325** |
| `temperature_range` | `[number, number]` | ✅ | 两元素数组 | 搜索温度区间 `[T_min, T_max]`（K） |
| `tdb_file` | `string` | ✅ | 枚举值之一 | 数据库文件 |

#### 纯 Al 熔点/沸点示例

```json
{
  "components": ["AL"],
  "composition": {"AL": 1.0},
  "pressure": 101325.0,
  "temperature_range": [800, 4000],
  "tdb_file": "Al-Si-Mg-Fe-Mn_by_wf.TDB"
}
```

#### 提交成功响应

```json
{
  "task_id": 18901,
  "status": "pending",
  "task_type": "boiling_point",
  "summary": "沸点/熔点任务已提交：AL 体系",
  "estimated_wait_seconds": 20,
  "next_action": "调用 calphamesh_get_task_result(task_id=18901) 等待并获取结果"
}
```

#### 计算结果（get_task_result 返回）

```json
{
  "task_id": 18901,
  "task_type": "boiling_point",
  "status": "completed",
  "result": {
    "data_summary": {
      "columns": ["Component", "Solidus/K", "Liquidus/K", "BubblePoint/K", "DewPoint/K"],
      "rows": [
        {
          "Component": "AL",
          "Solidus/K": 933.47,
          "Liquidus/K": 933.47,
          "BubblePoint/K": 2743.0,
          "DewPoint/K": 2743.0
        }
      ]
    },
    "derived_metrics": {
      "solidus_K": 933.47,
      "liquidus_K": 933.47,
      "bubble_point_K": 2743.0,
      "dew_point_K": 2743.0
    }
  },
  "units": {
    "temperature": "K",
    "pressure": "Pa"
  },
  "files": {
    "boiling_melting_point.csv": "https://taskman.fs.skyzcstack.space/.../boiling_melting_point.csv?...",
    "output.log":                "https://taskman.fs.skyzcstack.space/.../output.log?..."
  }
}
```

> **注意**：`temperature_range` 对搜索上界的约束并不完全严格，实际搜索可能超出所设上限（属于已知行为）。

---

### 7. `calphamesh_submit_thermodynamic_properties_task`

**功能**：在给定成分下，沿温度区间扫描计算摩尔 Gibbs 自由能（GM）、摩尔焓（HM）、摩尔熵（SM）、摩尔定压热容（CPM）随温度的变化曲线，支持同时扫描压力区间。

**适用场景**：
- 获取合金的热容-温度曲线（Cp-T 曲线），用于热模拟/铸造凝固仿真输入参数
- 分析 Gibbs 自由能变化，辅助理解相变驱动力
- 计算合金在凝固/热处理全程的焓变（总热量输入/输出估算）

#### 输入参数

| 参数 | 类型 | 必填 | 约束 | 说明 |
|------|------|------|------|------|
| `components` | `string[]` | ✅ | 至少 2 个 | 合金组元列表 |
| `composition` | `object` | ✅ | 总和 = 1.0 | 合金成分（原子分数）；内部自动以此作为 start=end 定成分扫描 |
| `temperature_start` | `number` | ✅ | 200~6000 K | 温度扫描起点 |
| `temperature_end` | `number` | ✅ | 须 > start | 温度扫描终点 |
| `increments` | `integer` | ✅ | 1~200 | 温度步长（K）；推荐 25 K（快速）或 5 K（精细） |
| `pressure_start` | `number` | ✅ | 0~15 | **log₁₀(P/Pa)**，常压 = 5（对应 10⁵ Pa = 100000 Pa） |
| `pressure_end` | `number` | ✅ | 0~15 | 常压扫描时与 pressure_start 相同，均设 5 |
| `pressure_increments` | `integer` | ✅ | 1~50 | 压力扫描步数；常压计算时设 2（最小值，相当于固定压力） |
| `properties` | `string[]` | ✅ | GM/HM/SM/CPM | 需要输出的热力学性质，建议全选 |
| `tdb_file` | `string` | ✅ | 枚举值之一 | 数据库文件 |

> ⚠️ **压力参数说明**：`pressure_start` / `pressure_end` 是 **log₁₀(P/Pa)**，不是直接的 Pa 值。
> - 常压（约 1 atm）：`pressure_start = pressure_end = 5`（即 10⁵ Pa = 100,000 Pa）
> - 高压 100 MPa：`pressure_start = pressure_end = 8`（即 10⁸ Pa）

#### Al-Si-Mg 示例（5 元体系，常压，500~950 K）

```json
{
  "components": ["AL", "SI", "MG", "FE", "MN"],
  "composition": {
    "AL": 0.93, "SI": 0.04, "MG": 0.01, "FE": 0.015, "MN": 0.005
  },
  "temperature_start": 500.0,
  "temperature_end":   950.0,
  "increments": 25,
  "pressure_start": 5.0,
  "pressure_end":   5.0,
  "pressure_increments": 2,
  "properties": ["GM", "HM", "SM", "CPM"],
  "tdb_file": "Al-Si-Mg-Fe-Mn_by_wf.TDB"
}
```

#### 提交成功响应

```json
{
  "task_id": 18902,
  "status": "pending",
  "task_type": "thermodynamic_properties",
  "summary": "热力学性质任务已提交：AL-SI-MG-FE-MN 体系，温度范围 500.0~950.0 K",
  "estimated_wait_seconds": 25,
  "next_action": "调用 calphamesh_get_task_result(task_id=18902) 等待并获取结果"
}
```

#### 计算结果（get_task_result 返回，summary 模式）

```json
{
  "task_id": 18902,
  "task_type": "thermodynamic_properties",
  "status": "completed",
  "result": {
    "data_summary": {
      "total_rows": 18,
      "shown_rows": 18,
      "temperature_range": {
        "start_K": 500.0,
        "end_K": 925.0
      },
      "columns": ["T/K", "P/Pa", "Phase", "GM/J/mol", "HM/J/mol", "SM/J/mol/K", "CPM/J/mol/K"],
      "rows": [
        {"T/K": 500.0, "P/Pa": 100000.0, "Phase": "FCC_A1", "GM/J/mol": -42500.0, "HM/J/mol": 5200.0, "SM/J/mol/K": 42.1, "CPM/J/mol/K": 28.6},
        {"T/K": 525.0, "P/Pa": 100000.0, "Phase": "FCC_A1", "GM/J/mol": -44200.0, "HM/J/mol": 5920.0, "SM/J/mol/K": 43.4, "CPM/J/mol/K": 28.9},
        "... (共 18 行)"
      ]
    },
    "derived_metrics": {
      "property_extrema": {
        "GM/J/mol": {
          "min": {"value": -65000.0},
          "max": {"value": -42500.0}
        },
        "CPM/J/mol/K": {
          "min": {"value": 28.6},
          "max": {"value": 35.2}
        }
      }
    }
  },
  "units": {
    "GM": "J/mol",
    "HM": "J/mol",
    "SM": "J/(mol·K)",
    "CPM": "J/(mol·K)"
  },
  "files": {
    "thermodynamic_properties.csv": "https://taskman.fs.skyzcstack.space/.../thermodynamic_properties.csv?...",
    "output.log":                   "https://taskman.fs.skyzcstack.space/.../output.log?..."
  }
}
```

> **结果说明**：`total_rows=18` 对应温度步长 25 K 下约 18 个温度点（500→950 K，步长 25 K 约 19 点，实际行数由相数量决定）。

---

## 结果查询工具

### `calphamesh_get_task_result`

**功能**：等待任务完成并返回结构化结果（**阻塞语义**）。内部每 8 秒轮询一次，直到任务进入终态（completed/failed/error）或超过 `timeout_seconds`。

#### 输入参数

| 参数 | 类型 | 必填 | 约束 | 说明 |
|------|------|------|------|------|
| `task_id` | `integer` | ✅ | > 0 | 由 `submit_*` 工具返回的任务 ID |
| `timeout_seconds` | `integer` | 否 | 10~90，默认 60 | 最大等待秒数 |
| `result_mode` | `string` | 否 | `"summary"` \| `"full"` | 结果详细程度，默认 `"summary"` |

#### result_mode 对比

| 模式 | 包含内容 | 适用场景 |
|------|---------|---------|
| `summary`（默认）| `data_summary` + `derived_metrics` + `files` | LLM 推理、前端展示 |
| `full` | summary 全部内容 + `raw_data`（完整原始数据） | 数据导出、二次分析 |

#### 任务类型检测逻辑（基于后端输出文件名）

| 优先级 | 文件名 | 对应任务类型 | 解析器 |
|--------|-------|------------|--------|
| 1 | `results.json` | `point_calculation` | `handle_point_result` |
| 2 | `scheil_solidification.csv` | `scheil_solidification` | `handle_scheil_csv_result` |
| 3 | `scheil_solidification.json` | `scheil_solidification`（legacy） | `handle_scheil_result` |
| 4 | `binary_equilibrium.json` | `binary_equilibrium` | `handle_binary_result` |
| 5 | `ternary_plotly.json` | `ternary_calculation` | `handle_ternary_result` |
| 6 | `thermodynamic_properties.csv` | `thermodynamic_properties` | `handle_thermo_csv_result` |
| 7 | `thermodynamic_properties.json` | `thermodynamic_properties`（legacy） | `handle_thermo_result` |
| 8 | `boiling_melting_point.csv` | `boiling_point` | `handle_boiling_result` |
| 9 | 其他 `*.csv`（非 scheil/boiling） | `line_calculation` | `handle_line_result` |

> **注意**：`scheil_conditions.json` 不代表计算结果，仅是输入条件回显文件，**不参与**类型检测。

#### 超时情况

```json
{
  "task_id": 18900,
  "status": "still_running",
  "elapsed_seconds": 63,
  "retry_after_seconds": 30,
  "message": "任务仍在计算中，请 30 秒后再次调用 calphamesh_get_task_result(task_id=18900)"
}
```

#### 计算失败情况

```json
{
  "error_code": "no_result_files",
  "task_id": 18900,
  "message": "任务已完成但未生成有效结果文件（实际文件：[\"output.log\", \"scheil_conditions.json\"]），计算过程中可能出现错误。日志文件: https://...",
  "retryable": true,
  "details": "请检查 components/composition/tdb_file 是否正确，或调整参数后重新提交"
}
```

---

### `calphamesh_get_task_status`

**功能**：非阻塞地查询任务状态，立即返回，不等待计算完成。

> 大多数场景直接使用 `calphamesh_get_task_result`（自动等待）。此工具适用于需要展示实时进度或在等待期间执行其他操作的场景。

#### 输入参数

| 参数 | 类型 | 必填 | 说明 |
|------|------|------|------|
| `task_id` | `integer` | ✅ | 任务 ID |

#### 输出示例

```json
{
  "task_id": 18900,
  "status": "running",
  "task_type": "topthermo_next",
  "title": "Ternary-Task-1741234567",
  "created_at": "2026-03-07T14:40:00Z",
  "updated_at": "2026-03-07T14:40:45Z",
  "result_ready": false,
  "next_action": "任务仍在运行中，调用 calphamesh_get_task_result(task_id=18900) 等待完成"
}
```

#### status 枚举值

| 值 | 含义 |
|----|------|
| `pending` | 已提交，等待调度 |
| `running` | 计算中 |
| `completed` | 完成，可获取结果 |
| `failed` | 计算失败 |
| `error` | 系统错误 |

---

### `calphamesh_list_tasks`

**功能**：分页查询当前用户的历史任务列表。

#### 输入参数

| 参数 | 类型 | 必填 | 约束 | 说明 |
|------|------|------|------|------|
| `page` | `integer` | 否 | ≥ 1，默认 1 | 页码（从 1 开始） |
| `items_per_page` | `integer` | 否 | 1~100，默认 20 | 每页任务数 |

#### 输出示例

```json
{
  "page": 1,
  "total_pages": 12,
  "items_per_page": 20,
  "tasks": [
    {
      "task_id": 18902,
      "status": "completed",
      "task_type": "topthermo_next",
      "title": "ThermoProp-Task-1741234890",
      "created_at": "2026-03-07T14:40:58Z"
    },
    {
      "task_id": 18900,
      "status": "completed",
      "task_type": "topthermo_next",
      "title": "Ternary-Task-1741234800",
      "created_at": "2026-03-07T14:40:15Z"
    }
  ]
}
```

---

## 输出数据结构详解

### 单位约定

| 物理量 | 字段 | 单位 |
|--------|------|------|
| 温度 | `temperature`, `T/K`, `liquidus_K` 等 | K（开尔文） |
| 压力 | `pressure`, `P/Pa` | Pa（帕斯卡）；thermo 任务的 pressure_start/end 用 log₁₀(Pa) |
| 摩尔 Gibbs 自由能 | `GM`, `GM/J/mol` | J/mol |
| 摩尔焓 | `HM`, `HM/J/mol` | J/mol |
| 摩尔熵 | `SM`, `SM/J/mol/K` | J/(mol·K) |
| 摩尔定压热容 | `CPM`, `CPM/J/mol/K` | J/(mol·K) |
| 化学势 | `chemical_potentials.*`, `MU(*)/J/mol` | J/mol |
| 相分数 | `phase_fractions.*`, `f(*)`  | 无量纲（0~1，总和 = 1） |
| 原子分数 | `composition.*`, `X(*)`  | 无量纲（0~1，总和 = 1） |

### 各任务类型的结果文件说明

| 任务类型 | 主结果文件 | 辅助文件 | 说明 |
|---------|----------|---------|------|
| `point_calculation` | `results.json` | `table.csv`, `output.log` | JSON 包含完整相结构数据 |
| `line_calculation` | `table_2.csv`（或其他 *.csv）| `output.log` | CSV 含多列热力学数据 |
| `scheil_solidification` | `scheil_solidification.csv` | `scheil_solidification.json`, `scheil_conditions.json`, `output.log` | CSV 为主要解析目标，JSON 为可选 |
| `binary_equilibrium` | `binary_equilibrium.json` | `output.log` | JSON 含 Plotly 图形数据 |
| `ternary_calculation` | `ternary_plotly.json` | `ternary_equilibrium.png`, `output.log` | JSON 含 Plotly 数据，PNG 为预渲染图 |
| `boiling_point` | `boiling_melting_point.csv` | `output.log` | CSV 含熔点/沸点数据 |
| `thermodynamic_properties` | `thermodynamic_properties.csv` | `output.log` | CSV 含 GM/HM/SM/CPM 数据 |

### 文件 URL 格式

所有文件 URL 均为**带时效的预签名 S3 URL**（有效期 3600 秒）：
```
https://taskman.fs.skyzcstack.space/<uuid>/<filename>?x-id=GetObject&X-Amz-Algorithm=...&X-Amz-Expires=3600&...
```
- 每次调用 `get_task_result` 都会返回刷新后的 URL
- 服务端下载文件时需携带 `Authorization: Bearer <API-Key>` 请求头

---

## 前置校验规则

所有 `submit_*` 工具在发送 API 请求**之前**，在服务端执行以下校验（校验失败直接返回 MCP 错误，不消耗后端配额）：

| 校验项 | 规则 | 涉及工具 |
|-------|------|---------|
| 组分原子分数之和 | `sum(values) == 1.0`（容差 1e-6） | Point / Line / Scheil / Binary / Ternary / Boiling |
| TDB 白名单校验 | `tdb_file` 必须是枚举值之一 | 全部 |
| TDB 元素覆盖校验 | `components` 中所有元素必须在 TDB 的元素集内 | 全部 |
| 温度范围 | Point/Line: 200~6000 K；Scheil: 500~6000 K；Ternary: 200~6000 K | 对应工具 |
| steps 范围 | 2~500 | Line |
| temperature_step 范围 | 0.1~50 K | Scheil |
| Binary 组元数 | 恰好 2 个 | Binary |
| Ternary 组元数 | 恰好 3 个 | Ternary |

**校验失败响应示例**：
```json
{
  "code": -32603,
  "message": "工具调用失败: 组分原子分数之和为 1.100000，必须等于 1.0（实际：AL=0.500000, SI=0.300000, MG=0.300000）"
}
```

---

## 错误处理规范

| 错误来源 | 错误码/字段 | 说明 | 处理建议 |
|---------|-----------|------|---------|
| 前置校验失败 | MCP `error.code=-32603` | 参数不合法 | 修正参数后重试 |
| API Key 缺失 | `MissingParameter("api_key")` | Bearer Token 未传 | 检查请求头 |
| HTTP 网络错误 | `HttpError(...)` | 网络不可达 | 稍后重试 |
| 后端 API 错误 | `ApiError { status, message }` | 后端拒绝请求 | 查看 message 详情 |
| 无结果文件 | `error_code: "no_result_files"` | 计算完成但未生成结果（通常是激活相配置问题） | 检查 output.log URL 中的错误日志 |
| 任务计算失败 | `status: "task_failed"` | 后端计算失败 | 检查参数，修正后重新提交 |
| 超时 | `status: "still_running"` | 计算仍在进行 | 直接再次调用 `get_task_result` |
| 文件格式未知 | `error_code: "unknown_result_format"` | 结果文件格式无法识别 | 通过 files URL 手动下载分析 |

---

## Al 合金设计场景使用指南

### 场景 1：Al-Si-Mg 压铸铝合金成分快速评估

**目标**：评估某成分在铸造温度下的相组成。

```
Step 1: calphamesh_submit_point_task
  components: ["AL","SI","MG","FE","MN"]
  composition: {AL: 0.93, SI: 0.04, MG: 0.01, FE: 0.015, MN: 0.005}
  temperature: 850.0  ← 压铸模具温度附近
  tdb_file: "Al-Si-Mg-Fe-Mn_by_wf.TDB"

Step 2: calphamesh_get_task_result(task_id, timeout_seconds=60)

预期结果解读：
- 若有 LIQUID 相，说明仍在半固态区，该温度偏高
- BETA_ALFESI、ALPHA_ALFEMNSI 等 Fe 相的相分数影响力学性能
- FCC_A1（铝基体）相分数代表已凝固的比例
```

### 场景 2：Scheil 凝固模拟获取凝固路径

**目标**：获取完整凝固路径，用于评估热裂倾向和缩孔风险。

```
Step 1: calphamesh_submit_scheil_task
  components: ["AL","SI","MG","FE","MN"]
  composition: {AL: 0.93, SI: 0.04, MG: 0.01, FE: 0.015, MN: 0.005}
  start_temperature: 1100.0  ← 高于液相线 ~170 K
  temperature_step: 5.0
  tdb_file: "Al-Si-Mg-Fe-Mn_by_wf.TDB"

Step 2: calphamesh_get_task_result(task_id, timeout_seconds=80)

关键指标解读：
- freezing_range_K：凝固范围，>100 K 表示高热裂倾向
- t_at_liquid_fraction_0_9_K：10% 固相出现温度（开始凝固）
- t_at_liquid_fraction_0_1_K：90% 固相温度（最终凝固段，共晶附近）
- solidus_K：最终凝固温度，越低说明含有低熔点共晶
```

### 场景 3：Al-Si 二元相图 + Al-Mg-Si 三元截面组合分析

**目标**：为合金成分设计建立相图基础。

```
Task A: calphamesh_submit_binary_task
  components: ["AL","SI"]
  start_composition: {AL: 1.0, SI: 0.0}
  end_composition:   {AL: 0.7, SI: 0.3}
  start_temperature: 500.0, end_temperature: 1200.0
  ↳ 目的：确定 Al-Si 共晶成分（12.6 mol% Si）和共晶温度（850 K）

Task B: calphamesh_submit_ternary_task
  components: ["AL","MG","SI"]
  temperature: 773.0  ← T6 时效温度（500°C）
  ↳ 目的：分析 MG₂SI 析出相的热力学稳定区，指导 Mg/Si 比优化
```

### 场景 4：热力学性质扫描（用于铸造仿真输入）

**目标**：获取凝固全温度区间内的热容（CPM）数据，供铸造仿真软件使用。

```
calphamesh_submit_thermodynamic_properties_task
  components: ["AL","SI","MG","FE","MN"]
  composition: {AL: 0.93, SI: 0.04, MG: 0.01, FE: 0.015, MN: 0.005}
  temperature_start: 500.0, temperature_end: 950.0
  increments: 25        ← 25 K 步长，18 个数据点
  pressure_start: 5.0, pressure_end: 5.0, pressure_increments: 2
  properties: ["GM","HM","SM","CPM"]
  ↳ 输出 CPM/J/mol/K：直接用作铸造仿真的等效比热容输入
```

---

## 扩展数据库说明

若需支持新的合金体系，同步更新以下位置：

```
calphaMesh.rs
  ├── TDB_ELEMENT_MAP  ← 追加 ("新文件名.TDB", &["元素A", "元素B", ...])
  └── tdb_default_phases()  ← 为新 TDB 添加对应的推荐相列表分支

tool_macros.rs（无需修改）
tools/mod.rs（无需修改）
```

schema 枚举值（`tdb_file` 的 `enum`）和 TDB 白名单校验均从 `TDB_ELEMENT_MAP` 自动派生。

### 当前已配置的完整列表

```rust
// TDB 数据库映射（位于 calphaMesh.rs）
const TDB_ELEMENT_MAP: &[(&str, &[&str])] = &[
    ("FE-C-SI-MN-CU-TI-O.TDB",     &["FE", "C", "SI", "MN", "CU", "TI", "O"]),
    ("B-C-SI-ZR-HF-LA-Y-TI-O.TDB", &["B", "C", "SI", "ZR", "HF", "LA", "Y", "TI", "O"]),
    // Al 基压铸铝合金（Al-Si-Mg-Fe-Mn 体系，含 Fe/Mn 杂质）
    ("Al-Si-Mg-Fe-Mn_by_wf.TDB",   &["AL", "SI", "MG", "FE", "MN"]),
];

// Al 数据库各任务类型推荐相（位于 calphaMesh.rs）
const AL_5ELEMENT_PHASES: &[&str] = &[
    "LIQUID", "FCC_A1", "DIAMOND_A4", "HCP_A3", "BCC_A2", "CBCC_A12",
    "BETA_ALMG", "EPSILON_ALMG", "GAMMA_ALMG", "MG2SI",
    "AL5FE2", "AL13FE4", "ALPHA_ALFESI", "BETA_ALFESI", "ALPHA_ALFEMNSI", "AL4_FEMN",
];  // 用于 point / line / scheil / thermo 任务

const AL_TERNARY_PHASES: &[&str] = &[
    "LIQUID", "FCC_A1", "DIAMOND_A4", "HCP_A3",
    "BETA_ALMG", "EPSILON_ALMG", "GAMMA_ALMG", "MG2SI",
];  // 用于 ternary 任务（Al-Mg-Si 3 元）

const AL_BINARY_PHASES: &[&str] = &[
    "LIQUID", "FCC_A1", "DIAMOND_A4",
];  // 用于 binary 任务（Al-Si 2 元）
```

### 扩展步骤示例（添加新 Al 合金数据库）

```rust
// 1. 在 TDB_ELEMENT_MAP 中追加新 TDB
const TDB_ELEMENT_MAP: &[(&str, &[&str])] = &[
    // ...已有条目...
    ("AL-CU-MG-SI-ZN.TDB", &["AL", "CU", "MG", "SI", "ZN"]),  // 新增：7xxx 系铝合金
];

// 2. 在 tdb_default_phases() 中为新 TDB 添加相列表
fn tdb_default_phases(tdb_file: &str) -> Vec<&'static str> {
    match tdb_file {
        "Al-Si-Mg-Fe-Mn_by_wf.TDB" => AL_5ELEMENT_PHASES.to_vec(),
        "AL-CU-MG-SI-ZN.TDB" => vec!["LIQUID", "FCC_A1", "S_PHASE", "MG2SI", "MGZN2", "..."],
        _ => vec!["*"],
    }
}
// schema 枚举和 TDB 元素校验自动更新，无需修改其他文件
```
