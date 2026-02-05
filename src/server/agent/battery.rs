//! 电池设计专家系统 (Battery Design A2A)
//!
//! 本系统由四个专门化的智能体协作完成电池设计逻辑：
//! 1. 首席架构师 (Lead Architect)：需求转化、任务规划、报告汇总。
//! 2. 材料科学家 (Materials Scientist)：数据库筛选、参数提供、机理推断。
//! 3. 仿真工程师 (Simulation Engineer)：模型选择、参数映射、物理求解。
//! 4. 优化分析师 (Optimization Analyst)：多方案对比、全局搜索与贝叶斯优化。

use super::history::HistoryManager;
use crate::server::database::DatabaseConnection;
use crate::server::mcp::tools::*;
use crate::server::middleware::auth::AuthUser;
use crate::server::models::*;
use crate::server::request::handle_chat_request;
use rig::agent::AgentBuilder;
use rig::completion::ToolDefinition;
use rig::prelude::*;
use rig::tool::Tool;
use serde::{Deserialize, Serialize};
use serde_json::json;

/// 电池设计协作流式处理器
pub async fn battery_agent(
    db: DatabaseConnection,
    request: ChatRequest,
    _auth_user: AuthUser,
) -> Result<(axum::response::Response, ChatResponse), ErrorResponse> {
    // API 配置
    let api_key = "sk-348d7ca647714c52aca12ea106cfa895";
    let client = rig::providers::qwen::Client::new_with_api_key(api_key);
    let model = client.completion_model("qwen-plus");

    // 1. 材料科学家 (Materials Scientist)
    let materials_scientist = AgentBuilder::new(model.clone())
        .name("materials_scientist")
        .description("电池材料科学家，精通晶体结构、热力学稳定性及电化学机理。")
        .preamble(
            "你是一个电池材料科学家。你的职责是根据架构师提出的指标需求，筛选候选材料并提供物理/化学参数。
            你可以解释材料微观结构对性能的影响。
            你可以调用以下工具：
            - query_materials_project: 查询无机材料性质。
            - query_pubchem: 查询电解液及添加剂化学性质。
            - run_ase_analysis: 使用 ASE 进行材料能带或稳定性分析。
            注意每个工具只能调用一次。"
        )
        .tool(QueryMaterialsProject)
        .tool(QueryPubChem)
        .tool(RunASEAnalysis)
        .build();

    // 2. 仿真工程师 (Simulation Engineer)
    let simulation_engineer = AgentBuilder::new(model.clone())
        .name("simulation_engineer")
        .description("电池仿真工程专家，精通计算物理、数值方法及 PyBaMM。")
        .preamble(
            "你是一个电池仿真工程师。你的职责是将设计参数映射到物理模型中，并执行数值求解。
            你需要选择合适的模型（如 SPM, DFN）并处理仿真中的数值波动。
            你可以调用以下工具：
            - list_battery_models: 获取可用模型列表。
            - list_battery_para_sets: 获取内置参数集。
            - get_battery_para_info: 查询参数详情。
            - run_battery_simulation: 执行物理仿真任务。
            - get_simulation_result: 获取仿真结果数据。",
        )
        .tool(ListBatteryModels)
        .tool(ListBatteryParaSets)
        .tool(GetBatteryParaInfo)
        .tool(GetBatteryOutParams)
        .tool(RunBatterySimulation)
        .tool(GetSimulationResult)
        // .tool(SimulateTiannengBattery)
        .build();

    // 3. 优化分析师 (Optimization Analyst)
    let optimization_analyst = AgentBuilder::new(model.clone())
        .name("optimization_analyst")
        .description("电池优化专家，擅长多方案评估、贝叶斯优化及参数敏感度分析。")
        .preamble(
            "你是一个电池优化分析师。你的职责是在多个设计方案中寻找全局最优解。
            你可以对比不同组仿真结果，并利用优化算法推荐最佳设计参数组合。
            你可以调用以下工具：
            - run_bayesian_optimization: 在参数空间中执行全局搜索。
            - analyze_battery_sox: 评估方案的健康状态、功率等指标。
            - predict_battery_rul: 预测设计方案下的循环寿命。
            - get_battery_task_status: 查询优化/预测任务进度。",
        )
        .tool(RunBayesianOptimization)
        .tool(AnalyzeBatterySox)
        .tool(PredictBatteryRul)
        .tool(GetBatteryTaskStatus)
        .build();

    // 4. 首席架构师 (Lead Architect) - 总控层
    let lead_architect = AgentBuilder::new(model.clone())
        .name("lead_architect")
        .preamble(
            "你是一个高级电池设计首席架构师。你负责将用户模糊的需求转化为具体的工程技术指标，并协调材料、仿真、优化专家完成设计闭环。

            **核心设计流程与执行纪律 (Granular HITL)**：
            你必须通过一个“提议并确认”的循环来驱动任务流。**严禁在前一个子智能体工作未确认前就连续安排多个环节。**

            每个环节的执行必须遵循以下三部曲：
            1. **规划**：规划好下一个要调度的子智能体及其具体参数。
            2. **提议确认**：先调用 `request_confirmation` 工具，向用户解释你接下来的具体意图。系统会在此处中断。
            3. **执行**：当且仅当收到用户的正面确认（如“开始”、“OK”、“继续”、“好”等）后，才真正执行该子智能体的工具调用。

            **步骤说明**：
            - **第一步 (找材料)**：解构需求后，提议调用 `materials_scientist`。得到确认后执行。
            - **第二步 (验证方案)**：阶段一完成后，提议调用 `simulation_engineer`。得到确认后执行。
            - **第三步 (寻找最优)**：阶段二完成后，提议调用 `optimization_analyst`。得到确认后执行。

            **执行准则**：
            - **禁止一次性越步**：不要在一个回复中同时调用或计划调用两个子智能体。
            - **状态感知**：检查对话历史。若发现你已输出过某个阶段的计划，且用户回复了肯定语义，请立即启动工具执行。
            - **中文沟通**：展现专业、严谨且具备工程远见的首席架构师风格。"
        )
        .max_tokens(4096)
        .tool(RequestConfirmation)
        .tool(materials_scientist)
        .tool(simulation_engineer)
        .tool(optimization_analyst)
        .build();

    // 获取历史并执行
    let history = if let Some(cvid) = &request.conversation_id {
        HistoryManager::new(db).get_context(cvid).await.ok()
    } else {
        None
    };

    handle_chat_request(lead_architect, request, history).await
}

// =============================================================================
// 以下为模拟工具 (Mock Tools)，后续可在此对接真实 API 或 Python 脚本
// =============================================================================

/// 1. Materials Project 模拟工具
#[derive(Deserialize, Serialize, Default)]
pub struct QueryMaterialsProject;

#[derive(Debug, Serialize, Deserialize)]
pub struct MockToolError(String);
impl std::fmt::Display for MockToolError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}
impl std::error::Error for MockToolError {}

impl Tool for QueryMaterialsProject {
    const NAME: &'static str = "query_materials_project";
    type Error = MockToolError;
    type Args = serde_json::Value;
    type Output = String;

    async fn definition(&self, _prompt: String) -> ToolDefinition {
        ToolDefinition {
            name: Self::NAME.to_string(),
            description:
                "查询 Materials Project 数据库获取无机材料的热力学、电化学及晶体结构性质。"
                    .to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "formula": { "type": "string", "description": "化学式，如 'LiFePO4'" },
                    "property": { "type": "string", "description": "所需性质，如 'band_gap', 'formation_energy_per_atom'" }
                }
            }),
        }
    }
    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        let formula = args
            .get("formula")
            .and_then(|v| v.as_str())
            .unwrap_or("Unknown");
        Ok(format!(
            "[MaterialsProject] 已检索到材料 {} 的核心性质：\n- 形成能: -2.15 eV/atom (高度稳定)\n- 带隙: 0.12 eV (表现出良好的电子导电性)\n- 理论比容量: 275 mAh/g (适用于高比能设计)\n- 空间群: R-3m (层状结构)",
            formula
        ))
    }
}

/// 2. PubChemPy 模拟工具
#[derive(Deserialize, Serialize, Default)]
pub struct QueryPubChem;
impl Tool for QueryPubChem {
    const NAME: &'static str = "query_pubchem";
    type Error = MockToolError;
    type Args = serde_json::Value;
    type Output = String;

    async fn definition(&self, _prompt: String) -> ToolDefinition {
        ToolDefinition {
            name: Self::NAME.to_string(),
            description: "查询 PubChem 获取有机溶剂、电解液及添加剂的化学性质。".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "name": { "type": "string", "description": "化学成分名称，如 'Ethylene carbonate'" }
                }
            }),
        }
    }
    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        let name = args
            .get("name")
            .and_then(|v| v.as_str())
            .unwrap_or("Unknown");
        Ok(format!(
            "[PubChem] 化学成分 {} 特性汇总：\n- 熔点: -43.5 ℃ (极佳的低温耐受性)\n- 极性: 高 (有助于提升电解液解离度)\n- 粘度 (@ -20℃): 2.8 mPa·s (低温下保持良好流动性)\n- 建议添加量: 2.5% - 5.0% wt.",
            name
        ))
    }
}

/// 3. ASE (Atomic Simulation Environment) 模拟工具
#[derive(Deserialize, Serialize, Default)]
pub struct RunASEAnalysis;
impl Tool for RunASEAnalysis {
    const NAME: &'static str = "run_ase_analysis";
    type Error = MockToolError;
    type Args = serde_json::Value;
    type Output = String;

    async fn definition(&self, _prompt: String) -> ToolDefinition {
        ToolDefinition {
            name: Self::NAME.to_string(),
            description: "调用原子尺度工具 ASE 进行材料稳定性或能带结构计算。".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "structure_data": { "type": "string", "description": "材料结构定义 (JSON格式)" },
                    "calc_type": { "type": "string", "description": "计算类型，如 'optimization', 'bandstructure'" }
                }
            }),
        }
    }
    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        Ok(format!(
            "[ASE] 原子级仿真结果：\n- 界面能: 0.45 J/m² (界面结合力良好)\n- 扩散能垒 (Li+): 0.28 eV (支持低温快速迁移)\n- 结论：该结构在极寒循环过程中具有出色的晶格稳定性，未发现严重的各向异性膨胀。"
        ))
    }
}

/// 4. 贝叶斯优化模拟工具
#[derive(Deserialize, Serialize, Default)]
pub struct RunBayesianOptimization;
impl Tool for RunBayesianOptimization {
    const NAME: &'static str = "run_bayesian_optimization";
    type Error = MockToolError;
    type Args = serde_json::Value;
    type Output = String;

    async fn definition(&self, _prompt: String) -> ToolDefinition {
        ToolDefinition {
            name: Self::NAME.to_string(),
            description: "在指定的参数空间内执行贝叶斯优化，寻找性能最优解。".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "search_space": { "type": "object", "description": "参数搜索范围，如 {'thickness': [10, 100], 'porosity': [0.1, 0.5]}" },
                    "target_metric": { "type": "string", "description": "优化目标，如 'discharge_capacity_Ah'" }
                },
                "required": ["search_space", "target_metric"]
            }),
        }
    }
    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        Ok(format!(
            "[BayesianOpt] 50轮贝叶斯优化迭代完成。针对 -20℃ 低温放电容量最优化的推荐配置：\n- 电极孔隙率 (Porosity): 0.38\n- 负极材料粒径 (Particle Size): 3.2μm\n- 电解液盐浓度 (Salt Conc): 1.15 M\n- 预测性能提升：低温能量保持率从 65% 提升至 82%。"
        ))
    }
}
