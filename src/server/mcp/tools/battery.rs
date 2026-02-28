use rig::{completion::ToolDefinition, tool::Tool};
use serde::{Deserialize, Serialize};
use serde_json::json;
use serde_json::Value;
use std::error::Error as StdError;

// const BATTERY_API_BASE_URL: &str = "http://139.159.198.14:20002/v1"; //生产环境
const BATTERY_API_BASE_URL: &str = "http://192.168.7.102:8083/v1"; //测试环境
// const BATTERY_API_BASE_URL: &str = "http://127.0.0.1:8000/v1"; //开发环境
// ==================== 错误类型 ====================

#[derive(Debug)]
pub enum BatteryToolError {
    HttpError(String),
    ApiError { status: u16, message: String },
    JsonError(String),
}

impl std::fmt::Display for BatteryToolError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            BatteryToolError::HttpError(msg) => write!(f, "HTTP请求失败: {}", msg),
            BatteryToolError::ApiError { status, message } => {
                write!(f, "API错误 (状态码 {}): {}", status, message)
            }
            BatteryToolError::JsonError(msg) => write!(f, "JSON错误: {}", msg),
        }
    }
}

impl StdError for BatteryToolError {}

fn normalize_generate_task_response(raw: &str) -> Result<String, BatteryToolError> {
    let parsed: Value =
        serde_json::from_str(raw).map_err(|e| BatteryToolError::JsonError(e.to_string()))?;

    let task_id = parsed
        .get("task_id")
        .and_then(Value::as_str)
        .ok_or_else(|| BatteryToolError::JsonError("响应缺少 task_id".to_string()))?;
    let status = parsed
        .get("status")
        .and_then(Value::as_str)
        .unwrap_or("pending");
    let message = parsed.get("message").cloned().unwrap_or(Value::Null);

    let response = json!({
        "task_id": task_id,
        "status": status,
        "message": message,
        "workflow": "frontend_polling",
        "next_action": "将 task_id 返回前端，前端轮询 GET /v1/analysis/tasks/{task_id} 获取进度与最终结果",
        "status_endpoint": format!("/v1/analysis/tasks/{}", task_id),
        "cancel_endpoint": format!("/v1/analysis/tasks/{}", task_id)
    });

    Ok(response.to_string())
}

// ==================== 请求/响应结构体 ====================

#[derive(Deserialize, Serialize, Debug, Default)]
pub struct EmptyArgs {}

#[derive(Deserialize, Serialize, Debug, Default)]
pub struct VariablesArgs {
    pub battery_type: Option<String>,
    pub model_name: Option<String>,
    pub keyword: Option<String>,
}

#[derive(Deserialize, Serialize, Debug, Default)]
pub struct ParameterSetNameArgs {
    pub name: String,
    pub keyword: Option<String>,
}

#[derive(Deserialize, Serialize, Debug, Default)]
pub struct SimulateArgs {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub battery_type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub para_set: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub experiment: Option<Vec<Vec<String>>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub t_eval: Option<Vec<f64>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub initial_soc: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub solver_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub output_variables: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub options: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parameter_overrides: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parameter_updates: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub var_pts: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub calc_esoh: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub save_at_cycles: Option<serde_json::Value>,
}

#[derive(Deserialize, Serialize, Debug, Default)]
pub struct FileIdArgs {
    pub file_id: String,
    pub nominal_capacity: Option<f64>,
}

#[derive(Deserialize, Serialize, Debug, Default)]
pub struct TrainingArgs {
    pub file_id: String,
    pub epochs: Option<i32>,
    pub batch_size: Option<i32>,
    pub sequence_length: Option<i32>,
    pub prediction_horizon: Option<i32>,
    pub learning_rate: Option<f64>,
    pub lstm_units: Option<Vec<i32>>,
}

#[derive(Deserialize, Serialize, Debug, Default)]
pub struct PredictionArgs {
    pub task_id: Option<String>,
    pub file_id: Option<String>,
    pub eol_threshold: Option<f64>,
}

#[derive(Deserialize, Serialize, Debug, Default)]
pub struct TaskIdArgs {
    pub task_id: String,
}

#[derive(Deserialize, Serialize, Debug, Default)]
pub struct ElectrolyteFormulaArgs {
    pub conductivities: Vec<f64>,
    pub anion_ratios: Vec<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub temperature: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub concentration: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub num_batch: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub batch_size: Option<i32>,
}

#[derive(Deserialize, Serialize, Debug, Default)]
pub struct ElectrolytePredictArgs {
    pub data: Vec<serde_json::Value>,
}

#[derive(Deserialize, Serialize, Debug, Default)]
pub struct TiannengSimulateArgs {
    pub charge_file: Option<String>,
    pub discharge_file: Option<String>,
    pub ocv_file: Option<String>,
    pub config_file: Option<String>,
}

#[derive(Deserialize, Serialize, Debug, Default)]
pub struct SimulationResultArgs {
    pub result_id: String,
    pub fields: Option<String>,
}

// ==================== Simulation 工具 ====================

/// 获取可用电池模型列表
#[derive(Deserialize, Serialize, Default)]
pub struct ListBatteryModels;

impl Tool for ListBatteryModels {
    const NAME: &'static str = "list_battery_models";
    type Error = BatteryToolError;
    type Args = EmptyArgs;
    type Output = String;

    async fn definition(&self, _prompt: String) -> ToolDefinition {
        ToolDefinition {
            name: Self::NAME.to_string(),
            description: "获取按电池类型分组的所有可用电池模型列表".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {}
            }),
        }
    }

    async fn call(&self, _args: Self::Args) -> Result<Self::Output, Self::Error> {
        let url = format!("{}/simulation/models", BATTERY_API_BASE_URL);
        let client = reqwest::Client::new();
        let response = client
            .get(url)
            .send()
            .await
            .map_err(|e| BatteryToolError::HttpError(e.to_string()))?;

        if !response.status().is_success() {
            return Err(BatteryToolError::ApiError {
                status: response.status().as_u16(),
                message: response.text().await.unwrap_or_default(),
            });
        }

        Ok(response
            .text()
            .await
            .map_err(|e| BatteryToolError::HttpError(e.to_string()))?)
    }
}

/// 获取可用参数集列表 (化学体系)
#[derive(Deserialize, Serialize, Default)]
pub struct ListBatteryParaSets;

impl Tool for ListBatteryParaSets {
    const NAME: &'static str = "list_battery_para_sets";
    type Error = BatteryToolError;
    type Args = EmptyArgs;
    type Output = String;

    async fn definition(&self, _prompt: String) -> ToolDefinition {
        ToolDefinition {
            name: Self::NAME.to_string(),
            description: "获取每种电池类型可用的参数集（化学体系）列表".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {}
            }),
        }
    }

    async fn call(&self, _args: Self::Args) -> Result<Self::Output, Self::Error> {
        let url = format!("{}/simulation/parameter-sets", BATTERY_API_BASE_URL);
        let client = reqwest::Client::new();
        let response = client
            .get(url)
            .send()
            .await
            .map_err(|e| BatteryToolError::HttpError(e.to_string()))?;

        if !response.status().is_success() {
            return Err(BatteryToolError::ApiError {
                status: response.status().as_u16(),
                message: response.text().await.unwrap_or_default(),
            });
        }

        Ok(response
            .text()
            .await
            .map_err(|e| BatteryToolError::HttpError(e.to_string()))?)
    }
}

/// 获取模型选项列表
#[derive(Deserialize, Serialize, Default)]
pub struct GetBatteryOptions;

impl Tool for GetBatteryOptions {
    const NAME: &'static str = "get_battery_options";
    type Error = BatteryToolError;
    type Args = EmptyArgs;
    type Output = String;

    async fn definition(&self, _prompt: String) -> ToolDefinition {
        ToolDefinition {
            name: Self::NAME.to_string(),
            description: "获取所有可用的电池模型选项（如热模型、维度等）".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {}
            }),
        }
    }

    async fn call(&self, _args: Self::Args) -> Result<Self::Output, Self::Error> {
        let url = format!("{}/simulation/options", BATTERY_API_BASE_URL);
        let client = reqwest::Client::new();
        let response = client
            .get(url)
            .send()
            .await
            .map_err(|e| BatteryToolError::HttpError(e.to_string()))?;

        if !response.status().is_success() {
            return Err(BatteryToolError::ApiError {
                status: response.status().as_u16(),
                message: response.text().await.unwrap_or_default(),
            });
        }

        Ok(response
            .text()
            .await
            .map_err(|e| BatteryToolError::HttpError(e.to_string()))?)
    }
}

/// 获取输出变量列表 (Output Variables)
#[derive(Deserialize, Serialize, Default)]
pub struct GetBatteryOutParams;

impl Tool for GetBatteryOutParams {
    const NAME: &'static str = "get_battery_out_params";
    type Error = BatteryToolError;
    type Args = VariablesArgs;
    type Output = String;

    async fn definition(&self, _prompt: String) -> ToolDefinition {
        ToolDefinition {
            name: Self::NAME.to_string(),
            description: "获取指定电池模型和类型的可用输出变量列表。支持关键词搜索。".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "battery_type": { "type": "string", "description": "电池类型，如 'Lithium-ion', 'Lead-acid', 'Sodium-ion' (默认 'Lithium-ion')" },
                    "model_name": { "type": "string", "description": "模型名称，如 'DFN', 'SPM', 'SPMe' (默认 'DFN')" },
                    "keyword": { "type": "string", "description": "搜索关键词（不区分大小写），例如 'voltage'" }
                }
            }),
        }
    }

    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        let url = format!("{}/simulation/variables", BATTERY_API_BASE_URL);
        let mut query = Vec::new();
        if let Some(bt) = args.battery_type {
            query.push(("battery_type", bt));
        }
        if let Some(mn) = args.model_name {
            query.push(("model_name", mn));
        }
        if let Some(kw) = args.keyword {
            query.push(("keyword", kw));
        }

        let client = reqwest::Client::new();
        let response = client
            .get(url)
            .query(&query)
            .send()
            .await
            .map_err(|e| BatteryToolError::HttpError(e.to_string()))?;

        if !response.status().is_success() {
            return Err(BatteryToolError::ApiError {
                status: response.status().as_u16(),
                message: response.text().await.unwrap_or_default(),
            });
        }

        Ok(response
            .text()
            .await
            .map_err(|e| BatteryToolError::HttpError(e.to_string()))?)
    }
}

/// 获取输入参数详情 (Input Parameters)
#[derive(Deserialize, Serialize, Default)]
pub struct GetBatteryParaInfo;

impl Tool for GetBatteryParaInfo {
    const NAME: &'static str = "get_battery_para_info";
    type Error = BatteryToolError;
    type Args = ParameterSetNameArgs;
    type Output = String;

    async fn definition(&self, _prompt: String) -> ToolDefinition {
        ToolDefinition {
            name: Self::NAME.to_string(),
            description: "获取指定参数集的所有输入参数及其默认值。支持关键词搜索。".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "name": { "type": "string", "description": "参数集名称，例如 'Chen2020', 'Sulzer2019'" },
                    "keyword": { "type": "string", "description": "参数搜索关键词" }
                },
                "required": ["name"]
            }),
        }
    }

    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        let url = format!(
            "{}/simulation/parameter-sets/{}/parameters",
            BATTERY_API_BASE_URL, args.name
        );
        let mut query = Vec::new();
        if let Some(kw) = args.keyword {
            query.push(("keyword", kw));
        }

        let client = reqwest::Client::new();
        let response = client
            .get(url)
            .query(&query)
            .send()
            .await
            .map_err(|e| BatteryToolError::HttpError(e.to_string()))?;

        if !response.status().is_success() {
            return Err(BatteryToolError::ApiError {
                status: response.status().as_u16(),
                message: response.text().await.unwrap_or_default(),
            });
        }

        Ok(response
            .text()
            .await
            .map_err(|e| BatteryToolError::HttpError(e.to_string()))?)
    }
}

/// 运行电池仿真 (Run Simulation)
#[derive(Deserialize, Serialize, Default)]
pub struct RunBatterySimulation;

impl Tool for RunBatterySimulation {
    const NAME: &'static str = "run_battery_simulation";
    type Error = BatteryToolError;
    type Args = SimulateArgs;
    type Output = String;

    async fn definition(&self, _prompt: String) -> ToolDefinition {
        ToolDefinition {
            name: Self::NAME.to_string(),
            description:
                "根据配置运行电池仿真。返回 result_id，使用 get_simulation_result 获取详细结果。"
                    .to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "battery_type": { "type": "string", "description": "电池类型，默认为 Lithium-ion" },
                    "model_name": { "type": "string", "description": "模型名称，默认为 DFN" },
                    "para_set": { "type": "string", "description": "使用的参数集/化学体系，默认为 Chen2020" },
                    "experiment": { "type": "array", "items": { "type": "array", "items": { "type": "string" } }, "description": "实验工况列表（二维数组），例如 [['Discharge at 1C until 2.5V', 'Rest for 1 hour']]" },
                    "t_eval": { "type": "array", "items": { "type": "number" }, "description": "评估时间点列表 (秒)" },
                    "initial_soc": { "type": "number", "description": "初始 SOC (0.0 - 1.0)，默认为空" },
                    "solver_name": { "type": "string", "description": "求解器名称" },
                    "output_variables": { "type": "array", "items": { "type": "string" }, "description": "需要返回的变量列表，变量名称限制非常严格，如有必要请调用GetBatteryOutParams使用关键词进行查询" },
                    "options": { "type": "object", "description": "模型选项，例如 {'thermal': 'isothermal'}，默认为空" },
                    "parameter_overrides": { "type": "object", "description": "参数覆盖，参数名称限制非常严格，如有必要请使用GetBatteryParaInfo使用关键词进行查询" },
                    "calc_esoh": { "type": "boolean", "description": "是否计算电极状态，默认为空" },
                    "save_at_cycles": { "description": "仅在指定循环保存解 (整数或整数列表)，默认为空" },
                    "parameter_updates": { "type": "object", "description": "参数更新，当没有这个参数则加入这个参数，有这个参数则更新这个参数" },
                    "var_pts": {"type": "object", "description": "用于划分正负极和隔膜网格"}
                }
            }),
        }
    }

    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        let url = format!("{}/simulation/run", BATTERY_API_BASE_URL);
        let client = reqwest::Client::new();

        let response = client
            .post(url)
            .json(&args)
            .send()
            .await
            .map_err(|e| BatteryToolError::HttpError(e.to_string()))?;

        if !response.status().is_success() {
            return Err(BatteryToolError::ApiError {
                status: response.status().as_u16(),
                message: response.text().await.unwrap_or_default(),
            });
        }

        Ok(response
            .text()
            .await
            .map_err(|e| BatteryToolError::HttpError(e.to_string()))?)
    }
}

/// 列出所有仿真结果
#[derive(Deserialize, Serialize, Default)]
pub struct ListSimulationResults;

impl Tool for ListSimulationResults {
    const NAME: &'static str = "list_simulation_results";
    type Error = BatteryToolError;
    type Args = EmptyArgs;
    type Output = String;

    async fn definition(&self, _prompt: String) -> ToolDefinition {
        ToolDefinition {
            name: Self::NAME.to_string(),
            description:
                "列出所有保存的仿真结果（不包含 solution 数据），返回 result_id、创建时间和元数据。"
                    .to_string(),
            parameters: json!({
                "type": "object",
                "properties": {}
            }),
        }
    }

    async fn call(&self, _args: Self::Args) -> Result<Self::Output, Self::Error> {
        let url = format!("{}/simulation/results", BATTERY_API_BASE_URL);
        let client = reqwest::Client::new();

        let response = client
            .get(url)
            .send()
            .await
            .map_err(|e| BatteryToolError::HttpError(e.to_string()))?;

        if !response.status().is_success() {
            return Err(BatteryToolError::ApiError {
                status: response.status().as_u16(),
                message: response.text().await.unwrap_or_default(),
            });
        }

        Ok(response
            .text()
            .await
            .map_err(|e| BatteryToolError::HttpError(e.to_string()))?)
    }
}

/// 获取仿真结果详情
#[derive(Deserialize, Serialize, Default)]
pub struct GetSimulationResult;

impl Tool for GetSimulationResult {
    const NAME: &'static str = "get_simulation_result";
    type Error = BatteryToolError;
    type Args = SimulationResultArgs;
    type Output = String;

    async fn definition(&self, _prompt: String) -> ToolDefinition {
        ToolDefinition {
            name: Self::NAME.to_string(),
            description: "根据 result_id 获取仿真结果详情（返回压缩的统计摘要以减少token数量）。"
                .to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "result_id": { "type": "string", "description": "仿真结果 ID（由 run_battery_simulation 返回）" },
                    "fields": { "type": "string", "description": "逗号分隔的变量名列表，用于过滤返回的 solution 字段。若不提供则返回全部变量。" }
                },
                "required": ["result_id"]
            }),
        }
    }

    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        let url = format!(
            "{}/simulation/results/{}",
            BATTERY_API_BASE_URL, args.result_id
        );
        let mut query = Vec::new();
        if let Some(fields) = args.fields {
            query.push(("fields", fields));
        }
        // 始终使用压缩模式以减少返回数据量
        query.push(("compress", "true".to_string()));

        let client = reqwest::Client::new();
        let response = client
            .get(url)
            .query(&query)
            .send()
            .await
            .map_err(|e| BatteryToolError::HttpError(e.to_string()))?;

        if !response.status().is_success() {
            return Err(BatteryToolError::ApiError {
                status: response.status().as_u16(),
                message: response.text().await.unwrap_or_default(),
            });
        }

        Ok(response
            .text()
            .await
            .map_err(|e| BatteryToolError::HttpError(e.to_string()))?)
    }
}

// ==================== Analysis 工具 ====================

/// SOX 分析工具
#[derive(Deserialize, Serialize, Default)]
pub struct AnalyzeBatterySox;

impl Tool for AnalyzeBatterySox {
    const NAME: &'static str = "analyze_battery_sox";
    type Error = BatteryToolError;
    type Args = FileIdArgs;
    type Output = String;

    async fn definition(&self, _prompt: String) -> ToolDefinition {
        ToolDefinition {
            name: Self::NAME.to_string(),
            description:
                "分析电池健康状态 (SOH)、荷电状态 (SOC)、功率状态 (SOP)、能量状态 (SOE) 等指标"
                    .to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "file_id": { "type": "string", "description": "已上传数据的文件 ID" },
                    "nominal_capacity": { "type": "number", "description": "额定容量 (Ah)，若不提供将自动取数据中容量列的最大值" }
                },
                "required": ["file_id"]
            }),
        }
    }

    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        let url = format!("{}/analysis/sox", BATTERY_API_BASE_URL);
        let client = reqwest::Client::new();

        let response = client
            .post(url)
            .json(&args)
            .send()
            .await
            .map_err(|e| BatteryToolError::HttpError(e.to_string()))?;

        if !response.status().is_success() {
            return Err(BatteryToolError::ApiError {
                status: response.status().as_u16(),
                message: response.text().await.unwrap_or_default(),
            });
        }

        Ok(response
            .text()
            .await
            .map_err(|e| BatteryToolError::HttpError(e.to_string()))?)
    }
}

#[derive(Deserialize, Serialize, Default)]
pub struct GenerateElectrolyteFormula;

impl Tool for GenerateElectrolyteFormula {
    const NAME: &'static str = "generate_electrolyte_formula";
    type Error = BatteryToolError;
    type Args = ElectrolyteFormulaArgs;
    type Output = String;

    async fn definition(&self, _prompt: String) -> ToolDefinition {
        ToolDefinition {
            name: Self::NAME.to_string(),
            description: "提交电解液配方生成任务（异步）。该工具只负责创建任务并返回 task_id，不负责轮询；轮询应由前端调用任务查询接口完成。".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "conductivities": { "type": "array", "items": { "type": "number" }, "description": "电导率列表" },
                    "anion_ratios": { "type": "array", "items": { "type": "number" }, "description": "阴离子比例列表" },
                    "temperature": { "type": "number", "description": "温度 (°C)，默认 25" },
                    "concentration": { "type": "number", "description": "浓度，默认 0.1" },
                    "num_batch": { "type": "integer", "description": "批次数量，默认 2" },
                    "batch_size": { "type": "integer", "description": "每批数量，默认 16" }
                },
                "required": ["conductivities", "anion_ratios"]
            }),
        }
    }

    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        let url = format!("{}/analysis/bamboo-mixer/generate-task", BATTERY_API_BASE_URL);
        let client = reqwest::Client::new();

        let response = client
            .post(url)
            .json(&args)
            .send()
            .await
            .map_err(|e| BatteryToolError::HttpError(e.to_string()))?;

        if !response.status().is_success() {
            return Err(BatteryToolError::ApiError {
                status: response.status().as_u16(),
                message: response.text().await.unwrap_or_default(),
            });
        }

        let raw = response
            .text()
            .await
            .map_err(|e| BatteryToolError::HttpError(e.to_string()))?;
        normalize_generate_task_response(&raw)
    }
}

/// LSTM 训练任务
#[derive(Deserialize, Serialize, Default)]
pub struct TrainBatteryLstm;

impl Tool for TrainBatteryLstm {
    const NAME: &'static str = "train_battery_lstm";
    type Error = BatteryToolError;
    type Args = TrainingArgs;
    type Output = String;

    async fn definition(&self, _prompt: String) -> ToolDefinition {
        ToolDefinition {
            name: Self::NAME.to_string(),
            description: "在特定电池数据上训练 LSTM 模型（异步任务），用于后续寿命预测。返回 task_id 用于查询进度。".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "file_id": { "type": "string", "description": "已上传数据的文件 ID" },
                    "epochs": { "type": "integer", "description": "训练轮数，默认 100" },
                    "batch_size": { "type": "integer", "description": "批量大小，默认 64" },
                    "sequence_length": { "type": "integer", "description": "输入序列长度，默认 15" },
                    "prediction_horizon": { "type": "integer", "description": "预测时间步长，默认 8" },
                    "learning_rate": { "type": "number", "description": "学习率，默认 0.01" },
                    "lstm_units": { "type": "array", "items": { "type": "integer" }, "description": "LSTM 层单元数列表，默认 [128, 64, 32]" }
                },
                "required": ["file_id"]
            }),
        }
    }

    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        let url = format!("{}/analysis/training", BATTERY_API_BASE_URL);
        let client = reqwest::Client::new();

        let response = client
            .post(url)
            .json(&args)
            .send()
            .await
            .map_err(|e| BatteryToolError::HttpError(e.to_string()))?;

        if !response.status().is_success() {
            return Err(BatteryToolError::ApiError {
                status: response.status().as_u16(),
                message: response.text().await.unwrap_or_default(),
            });
        }

        Ok(response
            .text()
            .await
            .map_err(|e| BatteryToolError::HttpError(e.to_string()))?)
    }
}

/// 查询任务状态
#[derive(Deserialize, Serialize, Default)]
pub struct GetBatteryTaskStatus;

impl Tool for GetBatteryTaskStatus {
    const NAME: &'static str = "get_battery_task_status";
    type Error = BatteryToolError;
    type Args = TaskIdArgs;
    type Output = String;

    async fn definition(&self, _prompt: String) -> ToolDefinition {
        ToolDefinition {
            name: Self::NAME.to_string(),
            description: "查询后台任务状态（如训练进度）".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "task_id": { "type": "string", "description": "任务 ID" }
                },
                "required": ["task_id"]
            }),
        }
    }

    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        let url = format!("{}/analysis/tasks/{}", BATTERY_API_BASE_URL, args.task_id);
        let client = reqwest::Client::new();

        let response = client
            .get(url)
            .send()
            .await
            .map_err(|e| BatteryToolError::HttpError(e.to_string()))?;

        if !response.status().is_success() {
            return Err(BatteryToolError::ApiError {
                status: response.status().as_u16(),
                message: response.text().await.unwrap_or_default(),
            });
        }

        Ok(response
            .text()
            .await
            .map_err(|e| BatteryToolError::HttpError(e.to_string()))?)
    }
}

/// RUL 预测
#[derive(Deserialize, Serialize, Default)]
pub struct PredictBatteryRul;

impl Tool for PredictBatteryRul {
    const NAME: &'static str = "predict_battery_rul";
    type Error = BatteryToolError;
    type Args = PredictionArgs;
    type Output = String;

    async fn definition(&self, _prompt: String) -> ToolDefinition {
        ToolDefinition {
            name: Self::NAME.to_string(),
            description:
                "使用 LSTM 模型预测电池剩余寿命 (RUL)。可以使用默认模型，或指定训练任务生成的模型。"
                    .to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "task_id": { "type": "string", "description": "训练任务的 ID，若提供将使用该任务训练的模型" },
                    "file_id": { "type": "string", "description": "数据文件 ID，若提供了 task_id 则此项可选" },
                    "eol_threshold": { "type": "number", "description": "寿命终止容量阈值，默认 0.8 (80% SOH)" }
                }
            }),
        }
    }

    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        let url = format!("{}/analysis/prediction", BATTERY_API_BASE_URL);
        let client = reqwest::Client::new();

        let response = client
            .post(url)
            .json(&args)
            .send()
            .await
            .map_err(|e| BatteryToolError::HttpError(e.to_string()))?;

        if !response.status().is_success() {
            return Err(BatteryToolError::ApiError {
                status: response.status().as_u16(),
                message: response.text().await.unwrap_or_default(),
            });
        }

        Ok(response
            .text()
            .await
            .map_err(|e| BatteryToolError::HttpError(e.to_string()))?)
    }
}

#[derive(Deserialize, Serialize, Default)]
pub struct PredictElectrolyteProperties;

impl Tool for PredictElectrolyteProperties {
    const NAME: &'static str = "predict_electrolyte_properties";
    type Error = BatteryToolError;
    type Args = ElectrolytePredictArgs;
    type Output = String;

    async fn definition(&self, _prompt: String) -> ToolDefinition {
        ToolDefinition {
            name: Self::NAME.to_string(),
            description: "根据配方预测电解液理化性质。".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "data": {
                        "type": "array",
                        "items": { "type": "object" },
                        "description": "配方数据列表"
                    }
                },
                "required": ["data"]
            }),
        }
    }

    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        let url = format!("{}/analysis/bamboo-mixer/predict", BATTERY_API_BASE_URL);
        let client = reqwest::Client::new();

        let response = client
            .post(url)
            .json(&args)
            .send()
            .await
            .map_err(|e| BatteryToolError::HttpError(e.to_string()))?;

        if !response.status().is_success() {
            return Err(BatteryToolError::ApiError {
                status: response.status().as_u16(),
                message: response.text().await.unwrap_or_default(),
            });
        }

        Ok(response
            .text()
            .await
            .map_err(|e| BatteryToolError::HttpError(e.to_string()))?)
    }
}

#[derive(Deserialize, Serialize, Default)]
pub struct DeleteElectrolyteTask;

impl Tool for DeleteElectrolyteTask {
    const NAME: &'static str = "delete_task";
    type Error = BatteryToolError;
    type Args = TaskIdArgs;
    type Output = String;

    async fn definition(&self, _prompt: String) -> ToolDefinition {
        ToolDefinition {
            name: Self::NAME.to_string(),
            description: "取消电解液配方生成任务。".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "task_id": { "type": "string", "description": "任务 ID" }
                },
                "required": ["task_id"]
            }),
        }
    }

    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        let url = format!("{}/analysis/tasks/{}", BATTERY_API_BASE_URL, args.task_id);
        let client = reqwest::Client::new();

        let response = client
            .delete(url)
            .send()
            .await
            .map_err(|e| BatteryToolError::HttpError(e.to_string()))?;

        if !response.status().is_success() {
            return Err(BatteryToolError::ApiError {
                status: response.status().as_u16(),
                message: response.text().await.unwrap_or_default(),
            });
        }

        Ok(response
            .text()
            .await
            .map_err(|e| BatteryToolError::HttpError(e.to_string()))?)
    }
}

/// 异常检测
#[derive(Deserialize, Serialize, Default)]
pub struct DetectBatteryAnomalies;

impl Tool for DetectBatteryAnomalies {
    const NAME: &'static str = "detect_battery_anomalies";
    type Error = BatteryToolError;
    type Args = FileIdArgs;
    type Output = String;

    async fn definition(&self, _prompt: String) -> ToolDefinition {
        ToolDefinition {
            name: Self::NAME.to_string(),
            description: "检测电池数据中的异常（电压尖峰、温升过快等）".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "file_id": { "type": "string", "description": "已上传数据的文件 ID" }
                },
                "required": ["file_id"]
            }),
        }
    }

    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        let url = format!("{}/analysis/anomalies", BATTERY_API_BASE_URL);
        let client = reqwest::Client::new();

        let response = client
            .post(url)
            .json(&json!({ "file_id": args.file_id }))
            .send()
            .await
            .map_err(|e| BatteryToolError::HttpError(e.to_string()))?;

        if !response.status().is_success() {
            return Err(BatteryToolError::ApiError {
                status: response.status().as_u16(),
                message: response.text().await.unwrap_or_default(),
            });
        }

        Ok(response
            .text()
            .await
            .map_err(|e| BatteryToolError::HttpError(e.to_string()))?)
    }
}

// ==================== Tianneng 工具 ====================

/// 天能电池仿真
#[derive(Deserialize, Serialize, Default)]
pub struct SimulateTiannengBattery;

impl Tool for SimulateTiannengBattery {
    const NAME: &'static str = "simulate_tianneng_battery";
    type Error = BatteryToolError;
    type Args = TiannengSimulateArgs;
    type Output = String;

    async fn definition(&self, _prompt: String) -> ToolDefinition {
        ToolDefinition {
            name: Self::NAME.to_string(),
            description: "运行天能电池在不同倍率下的 DFN 模型仿真，返回与实验数据的电压-容量对比数据。不提供路径则使用默认280Ah测试数据。".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "charge_file": { "type": "string", "description": "充电实验 Excel 文件路径" },
                    "discharge_file": { "type": "string", "description": "放电实验 Excel 文件路径" },
                    "ocv_file": { "type": "string", "description": "OCV 数据文件路径" },
                    "config_file": { "type": "string", "description": "配置文件路径" }
                }
            }),
        }
    }

    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        let url = format!("{}/tianneng/simulate", BATTERY_API_BASE_URL);
        let client = reqwest::Client::new();

        let mut query = Vec::new();
        if let Some(cf) = args.charge_file {
            query.push(("charge_file", cf));
        }
        if let Some(df) = args.discharge_file {
            query.push(("discharge_file", df));
        }
        if let Some(of) = args.ocv_file {
            query.push(("ocv_file", of));
        }
        if let Some(cfg) = args.config_file {
            query.push(("config_file", cfg));
        }

        let response = client
            .post(url)
            .query(&query)
            .send()
            .await
            .map_err(|e| BatteryToolError::HttpError(e.to_string()))?;

        if !response.status().is_success() {
            return Err(BatteryToolError::ApiError {
                status: response.status().as_u16(),
                message: response.text().await.unwrap_or_default(),
            });
        }

        Ok(response
            .text()
            .await
            .map_err(|e| BatteryToolError::HttpError(e.to_string()))?)
    }
}
