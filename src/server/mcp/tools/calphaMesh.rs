//! Calpha Mesh MCP 工具
//!
//! 提供与 Calpha Mesh API 交互的工具，用于提交材料计算任务和查询结果

use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use serde_json::json;
use std::error::Error as StdError;

use rig::{completion::ToolDefinition, tool::Tool};

// API 基础 URL
const API_BASE_URL: &str = "https://api.topmaterial-tech.com";

// 工具错误类型
#[derive(Debug)]
pub enum CalphaMeshError {
    HttpError(String),
    ApiError { status: u16, message: String },
    JsonError(serde_json::Error),
    InvalidTaskId(i32),
    MissingParameter(String),
}

impl std::fmt::Display for CalphaMeshError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CalphaMeshError::HttpError(msg) => write!(f, "HTTP request failed: {}", msg),
            CalphaMeshError::ApiError { status, message } => {
                write!(f, "API error (status {}): {}", status, message)
            }
            CalphaMeshError::JsonError(e) => {
                write!(f, "JSON serialization/deserialization error: {}", e)
            }
            CalphaMeshError::InvalidTaskId(id) => write!(f, "Invalid task ID: {}", id),
            CalphaMeshError::MissingParameter(param) => {
                write!(f, "Missing required parameter: {}", param)
            }
        }
    }
}

impl StdError for CalphaMeshError {}

impl From<serde_json::Error> for CalphaMeshError {
    fn from(err: serde_json::Error) -> Self {
        CalphaMeshError::JsonError(err)
    }
}

// 任务相关结构体 (更新后适配 topthermo_next)
#[derive(Debug, Serialize, Deserialize)]
pub struct CreateTaskRequest {
    pub title: String,
    pub description: String, // 序列化的内部配置
    pub task_type: String,   // 固定为 "topthermo_next"
    pub db_key: String,      // 默认为 "default"
}

#[derive(Debug, Serialize, Deserialize)]
pub struct GetTaskRequest {
    pub id: i32,
}

#[derive(Debug, Deserialize)]
pub struct TaskResponse {
    pub id: i32,
    pub status: String,
}

#[derive(Debug, Deserialize)]
pub struct TaskStatusResponse {
    pub id: i32,
    pub title: String,
    pub description: String,
    pub status: String,
    pub task_type: String,
    pub result: Option<serde_json::Value>,
    pub logs: Option<String>,
    pub user_id: i32,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Deserialize)]
pub struct TaskListResponse {
    pub data: Vec<TaskStatusResponse>,
    pub total_pages: i64,
    pub page: i32,
    pub items_per_page: i32,
}

#[derive(Debug, Deserialize)]
pub struct TaskResultFilesResponse {
    pub task_id: String,
    pub files: Vec<String>,
    pub total_count: usize,
}

// Point 计算参数
#[derive(Debug, Serialize, Deserialize)]
pub struct PointTaskParams {
    #[serde(default = "default_components")]
    pub components: Vec<String>,
    #[serde(default = "default_composition")]
    pub composition: HashMap<String, f64>,
    #[serde(default = "default_temperature")]
    pub temperature: f64,
    #[serde(default = "default_database")]
    pub tdb_file: String,
    /// Calpha Mesh API 密钥
    pub api_key: Option<String>,
}

// Line 计算参数
#[derive(Debug, Serialize, Deserialize)]
pub struct LineTaskParams {
    #[serde(default = "default_components")]
    pub components: Vec<String>,
    #[serde(default = "default_composition")]
    pub start_composition: HashMap<String, f64>,
    #[serde(default = "default_temperature")]
    pub start_temperature: f64,
    #[serde(default = "default_composition")]
    pub end_composition: HashMap<String, f64>,
    #[serde(default = "default_end_temperature")]
    pub end_temperature: f64,
    #[serde(default = "default_steps")]
    pub steps: i64,
    #[serde(default = "default_database")]
    pub tdb_file: String,
    /// Calpha Mesh API 密钥
    pub api_key: Option<String>,
}

// Scheil 计算参数
#[derive(Debug, Serialize, Deserialize)]
pub struct ScheilTaskParams {
    #[serde(default = "default_components")]
    pub components: Vec<String>,
    #[serde(default = "default_composition")]
    pub composition: HashMap<String, f64>,
    #[serde(default = "default_scheil_temperature")]
    pub start_temperature: f64,
    #[serde(default = "default_temperature_step")]
    pub temperature_step: f64,
    #[serde(default = "default_database")]
    pub tdb_file: String,
    /// Calpha Mesh API 密钥
    pub api_key: Option<String>,
}

// Binary 计算参数
#[derive(Debug, Serialize, Deserialize)]
pub struct BinaryTaskParams {
    pub components: Vec<String>,
    pub start_composition: HashMap<String, f64>,
    pub end_composition: HashMap<String, f64>,
    pub start_temperature: f64,
    pub end_temperature: f64,
    pub tdb_file: String,
    /// Calpha Mesh API 密钥
    pub api_key: Option<String>,
}

// Ternary 计算参数
#[derive(Debug, Serialize, Deserialize)]
pub struct TernaryTaskParams {
    pub components: Vec<String>,
    pub temperature: f64,
    pub composition_x: HashMap<String, f64>,
    pub composition_y: HashMap<String, f64>,
    pub composition_o: HashMap<String, f64>,
    pub tdb_file: String,
    /// Calpha Mesh API 密钥
    pub api_key: Option<String>,
}

// Boiling Point 计算参数
#[derive(Debug, Serialize, Deserialize)]
pub struct BoilingPointParams {
    pub components: Vec<String>,
    pub composition: HashMap<String, f64>,
    pub pressure: f64,
    pub temperature_range: (f64, f64),
    pub tdb_file: String,
    /// Calpha Mesh API 密钥
    pub api_key: Option<String>,
}

// Thermodynamic Properties 计算参数
#[derive(Debug, Serialize, Deserialize)]
pub struct ThermoPropertiesParams {
    pub components: Vec<String>,
    pub composition: HashMap<String, f64>,
    pub temperature_start: f64,
    pub temperature_end: f64,
    pub increments: i64,
    pub pressure_start: f64,
    pub pressure_end: f64,
    pub pressure_increments: i64,
    pub properties: Vec<String>,
    pub tdb_file: String,
    /// Calpha Mesh API 密钥
    pub api_key: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct TaskIdParams {
    pub task_id: i32,
    /// Calpha Mesh API 密钥
    pub api_key: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ListTasksParams {
    #[serde(default = "default_page")]
    pub page: i32,
    #[serde(default = "default_items_per_page")]
    pub items_per_page: i32,
    /// Calpha Mesh API 密钥
    pub api_key: Option<String>,
}

// 默认值函数
fn default_components() -> Vec<String> {
    vec!["AL".to_string(), "MG".to_string(), "SI".to_string()]
}

fn default_composition() -> HashMap<String, f64> {
    let mut comp = HashMap::new();
    comp.insert("AL".to_string(), 1.0);
    comp.insert("MG".to_string(), 0.0);
    comp.insert("SI".to_string(), 0.0);
    comp
}

fn default_temperature() -> f64 {
    298.15
}
fn default_end_temperature() -> f64 {
    1000.0
}
fn default_scheil_temperature() -> f64 {
    1073.15
}
fn default_pressure() -> f64 {
    1.0
}
fn default_scheil_pressure() -> f64 {
    1.01325
}
fn default_steps() -> i64 {
    50
}
fn default_database() -> String {
    "default".to_string()
}
fn default_temperature_step() -> f64 {
    1.0
}

fn default_page() -> i32 {
    1
}
fn default_items_per_page() -> i32 {
    50
}

/// 验证组分之和是否等于1（允许微小误差），返回错误信息
fn validate_composition_sum(composition: &HashMap<String, f64>) -> Option<String> {
    let sum: f64 = composition.values().sum();
    const TOLERANCE: f64 = 1e-6;
    if (sum - 1.0).abs() > TOLERANCE {
        return Some(format!(
            "组分之和必须为1，当前总和为 {:.6}，请调整组分值后重试",
            sum
        ));
    }
    None
}

// Calpha Mesh API 客户端
#[derive(Clone)]
pub struct CalphaMeshClient {
    api_key: String,
    client: reqwest::Client,
}

impl CalphaMeshClient {
    pub fn new(api_key: String) -> Self {
        Self {
            api_key,
            client: reqwest::Client::new(),
        }
    }

    async fn make_request(
        &self,
        method: reqwest::Method,
        url: &str,
        body: Option<String>,
    ) -> Result<String, CalphaMeshError> {
        let mut request = self
            .client
            .request(method, url)
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("Content-Type", "application/json");

        if let Some(b) = body {
            request = request.body(b);
        }

        let response = request
            .send()
            .await
            .map_err(|e| CalphaMeshError::HttpError(e.to_string()))?;

        let status = response.status().as_u16();
        let response_text = response
            .text()
            .await
            .map_err(|e| CalphaMeshError::HttpError(e.to_string()))?;

        if status >= 200 && status < 300 {
            Ok(response_text)
        } else {
            Err(CalphaMeshError::ApiError {
                status,
                message: response_text,
            })
        }
    }

    /// 通用的 topthermo_next 任务提交函数
    async fn submit_topthermo_task(
        &self,
        title: String,
        inner_description: serde_json::Value,
    ) -> Result<TaskResponse, CalphaMeshError> {
        let create_body = CreateTaskRequest {
            title,
            description: inner_description.to_string(),
            task_type: "topthermo_next".to_string(),
            db_key: "default".to_string(),
        };

        let url = format!("{}/api/v1/create_task", API_BASE_URL);
        let response_text = self
            .make_request(
                reqwest::Method::POST,
                &url,
                Some(serde_json::to_string(&create_body)?),
            )
            .await?;
        let task_response: TaskResponse = serde_json::from_str(&response_text)?;

        Ok(task_response)
    }

    pub async fn submit_point_task(
        &self,
        params: PointTaskParams,
    ) -> Result<TaskResponse, CalphaMeshError> {
        let inner = json!({
            "task_type": "point_calculation",
            "tdb_file": format!("/app/exe/topthermo-next/database/{}", params.tdb_file),
            "task_name": format!("point_{}", chrono::Utc::now().timestamp()),
            "task_path": "examples/framework_demo/result/point_calculation_1",
            "condition": {
                "components": params.components,
                "activated_phases": [],
                "temperature": params.temperature,
                "compositions": params.composition
            }
        });

        self.submit_topthermo_task(
            format!("Point-Task-{}", chrono::Utc::now().timestamp()),
            inner,
        )
        .await
    }

    pub async fn submit_line_task(
        &self,
        params: LineTaskParams,
    ) -> Result<TaskResponse, CalphaMeshError> {
        let inner = json!({
            "task_type": "line_calculation",
            "tdb_file": format!("/app/exe/topthermo-next/database/{}", params.tdb_file),
            "task_name": format!("line_{}", chrono::Utc::now().timestamp()),
            "task_path": "examples/framework_demo/result/point_calculation_1",
            "condition": {
                "components": params.components,
                "compositions_start": params.start_composition,
                "compositions_end": params.end_composition,
                "temperature_start": params.start_temperature,
                "temperature_end": params.end_temperature,
                "increments": params.steps,
                "activated_phases": []
            }
        });

        self.submit_topthermo_task(
            format!("Line-Task-{}", chrono::Utc::now().timestamp()),
            inner,
        )
        .await
    }

    pub async fn submit_scheil_task(
        &self,
        params: ScheilTaskParams,
    ) -> Result<TaskResponse, CalphaMeshError> {
        let inner = json!({
            "task_type": "scheil_solidification",
            "tdb_file": format!("/app/exe/topthermo-next/database/{}", params.tdb_file),
            "task_name": format!("scheil_{}", chrono::Utc::now().timestamp()),
            "task_path": "examples/framework_demo/result/point_calculation_1",
            "condition": {
                "components": params.components,
                "compositions": params.composition,
                "start_temperature": params.start_temperature,
                "temperature_step": params.temperature_step,
                "activated_phases": [],
                "inhibit_phases": []
            }
        });

        self.submit_topthermo_task(
            format!("Scheil-Task-{}", chrono::Utc::now().timestamp()),
            inner,
        )
        .await
    }

    pub async fn submit_binary_task(
        &self,
        params: BinaryTaskParams,
    ) -> Result<TaskResponse, CalphaMeshError> {
        let inner = json!({
            "task_type": "binary_equilibrium",
            "tdb_file": format!("/app/exe/topthermo-next/database/{}", params.tdb_file),
            "task_name": format!("binary_{}", chrono::Utc::now().timestamp()),
            "task_path": "examples/framework_demo/result/point_calculation_1",
            "condition": {
                "components": params.components,
                "activated_phases": ["*"],
                "compositions_start": params.start_composition,
                "compositions_end": params.end_composition,
                "temperature_start": params.start_temperature,
                "temperature_end": params.end_temperature
            }
        });

        self.submit_topthermo_task(
            format!("Binary-Task-{}", chrono::Utc::now().timestamp()),
            inner,
        )
        .await
    }

    pub async fn submit_ternary_task(
        &self,
        params: TernaryTaskParams,
    ) -> Result<TaskResponse, CalphaMeshError> {
        let inner = json!({
            "task_type": "ternary_calculation",
            "tdb_file": format!("/app/exe/topthermo-next/database/{}", params.tdb_file),
            "task_name": format!("ternary_{}", chrono::Utc::now().timestamp()),
            "task_path": "examples/framework_demo/result/point_calculation_1",
            "condition": {
                "components": params.components,
                "activated_phases": ["*"],
                "temperature": params.temperature,
                "compositions_y": params.composition_y,
                "compositions_x": params.composition_x,
                "compositions_o": params.composition_o
            }
        });

        self.submit_topthermo_task(
            format!("Ternary-Task-{}", chrono::Utc::now().timestamp()),
            inner,
        )
        .await
    }

    pub async fn submit_boiling_point_task(
        &self,
        params: BoilingPointParams,
    ) -> Result<TaskResponse, CalphaMeshError> {
        let inner = json!({
            "task_type": "boiling_point",
            "tdb_file": format!("/app/exe/topthermo-next/database/{}", params.tdb_file),
            "task_name": format!("boiling_{}", chrono::Utc::now().timestamp()),
            "task_path": "examples/framework_demo/result/point_calculation_1",
            "condition": {
                "components": params.components,
                "pressure": params.pressure,
                "compositions": params.composition,
                "temperature_range": [params.temperature_range.0, params.temperature_range.1]
            }
        });

        self.submit_topthermo_task(
            format!("Boiling-Task-{}", chrono::Utc::now().timestamp()),
            inner,
        )
        .await
    }

    pub async fn submit_thermo_properties_task(
        &self,
        params: ThermoPropertiesParams,
    ) -> Result<TaskResponse, CalphaMeshError> {
        let inner = json!({
            "task_type": "thermodynamic_properties",
            "tdb_file": format!("/app/exe/topthermo-next/database/{}", params.tdb_file),
            "task_name": format!("properties_{}", chrono::Utc::now().timestamp()),
            "task_path": "examples/framework_demo/result/point_calculation_1",
            "condition": {
                "components": params.components,
                "activated_phases": ["*"],
                "compositions_start": params.composition,
                "compositions_end": params.composition,
                "temperature_start": params.temperature_start,
                "temperature_end": params.temperature_end,
                "increments": params.increments,
                "pressure_start": params.pressure_start,
                "pressure_end": params.pressure_end,
                "pressure_increments": params.pressure_increments,
                "properties": params.properties
            }
        });

        self.submit_topthermo_task(
            format!("ThermoProp-Task-{}", chrono::Utc::now().timestamp()),
            inner,
        )
        .await
    }

    pub async fn get_task_status(
        &self,
        task_id: i32,
    ) -> Result<TaskStatusResponse, CalphaMeshError> {
        if task_id <= 0 {
            return Err(CalphaMeshError::InvalidTaskId(task_id));
        }

        let url = format!("{}/api/v1/get_task", API_BASE_URL);
        let body = json!({ "id": task_id });
        let response_text = self
            .make_request(reqwest::Method::POST, &url, Some(body.to_string()))
            .await?;
        let task: TaskStatusResponse = serde_json::from_str(&response_text)?;

        Ok(task)
    }

    pub async fn list_tasks(
        &self,
        page: i32,
        items_per_page: i32,
    ) -> Result<TaskListResponse, CalphaMeshError> {
        let url = format!("{}/api/v1/get_tasks", API_BASE_URL);
        let body = json!({
            "page": page,
            "items_per_page": items_per_page
        });
        let response_text = self
            .make_request(reqwest::Method::POST, &url, Some(body.to_string()))
            .await?;
        let list: TaskListResponse = serde_json::from_str(&response_text)?;

        Ok(list)
    }

    /// 获取任务结果文件列表
    pub async fn get_result_files(
        &self,
        task_id: i32,
    ) -> Result<TaskResultFilesResponse, CalphaMeshError> {
        let url = format!("{}/api/v1/get_result_files", API_BASE_URL);
        let body = json!({ "id": task_id });
        let response_text = self
            .make_request(reqwest::Method::POST, &url, Some(body.to_string()))
            .await?;
        let result: TaskResultFilesResponse = serde_json::from_str(&response_text)?;
        Ok(result)
    }

    /// 下载文件文本内容（通过预签名 URL）
    pub async fn download_file_content(&self, url: &str) -> Result<String, CalphaMeshError> {
        let response = self
            .client
            .get(url)
            .send()
            .await
            .map_err(|e| CalphaMeshError::HttpError(e.to_string()))?;

        let status = response.status().as_u16();
        let text = response
            .text()
            .await
            .map_err(|e| CalphaMeshError::HttpError(e.to_string()))?;

        if status >= 200 && status < 300 {
            Ok(text)
        } else {
            Err(CalphaMeshError::ApiError {
                status,
                message: text,
            })
        }
    }
}

// 工具实现

// 提交 Point 计算任务工具
#[derive(Deserialize, Serialize, Default)]
pub struct SubmitPointTask {
    pub api_key: Option<String>,
}

impl SubmitPointTask {
    pub fn new(api_key: String) -> Self {
        Self {
            api_key: Some(api_key),
        }
    }
}

impl Tool for SubmitPointTask {
    const NAME: &'static str = "calphamesh_submit_point_task";

    type Error = CalphaMeshError;
    type Args = PointTaskParams;
    type Output = String;

    async fn definition(&self, _prompt: String) -> ToolDefinition {
        ToolDefinition {
            name: "calphamesh_submit_point_task".to_string(),
            description: "提交 Point 平衡计算任务到 Calpha Mesh 服务器".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "components": {
                        "type": "array",
                        "items": {"type": "string"},
                        "description": "组分列表"
                    },
                    "composition": {
                        "type": "object",
                        "additionalProperties": {"type": "number"},
                        "description": "成分组成 (元素:原子分数)"
                    },
                    "temperature": {
                        "type": "number",
                        "description": "计算温度(K)"
                    },
                    "tdb_file": {
                        "type": "string",
                        "enum": ["FE-C-SI-MN-CU-TI-O.TDB", "B-C-SI-ZR-HF-LA-Y-TI-O.TDB"],
                        "description": "TDB 数据库文件名。请确保计算组分中所有的元素都在选择的数据库文件名中。"
                    }
                },
                "required": ["components", "composition", "temperature", "tdb_file"]
            }),
        }
    }

    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        // 验证组分之和
        if let Some(error_msg) = validate_composition_sum(&args.composition) {
            return Ok(error_msg);
        }

        // 获取 API key
        let api_key = args
            .api_key
            .clone()
            .or_else(|| self.api_key.clone())
            .ok_or_else(|| CalphaMeshError::MissingParameter("api_key".to_string()))?;
        let client = CalphaMeshClient::new(api_key);
        let task_response = client.submit_point_task(args).await?;

        Ok(format!(
            "Point计算任务已提交\nID: {}\n状态: {}",
            task_response.id, task_response.status
        ))
    }
}

// 提交 Line 计算任务工具
#[derive(Deserialize, Serialize, Default)]
pub struct SubmitLineTask {
    pub api_key: Option<String>,
}

impl SubmitLineTask {
    pub fn new(api_key: String) -> Self {
        Self {
            api_key: Some(api_key),
        }
    }
}

impl Tool for SubmitLineTask {
    const NAME: &'static str = "calphamesh_submit_line_task";

    type Error = CalphaMeshError;
    type Args = LineTaskParams;
    type Output = String;

    async fn definition(&self, _prompt: String) -> ToolDefinition {
        ToolDefinition {
            name: "calphamesh_submit_line_task".to_string(),
            description: "提交 Line 线性计算任务到 Calpha Mesh 服务器".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "components": {
                        "type": "array",
                        "items": {"type": "string"},
                        "description": "组分列表"
                    },
                    "start_composition": {
                        "type": "object",
                        "additionalProperties": {"type": "number"},
                        "description": "起始成分组成 (元素:原子分数)"
                    },
                    "start_temperature": {
                        "type": "number",
                        "description": "起始温度(K)"
                    },
                    "end_composition": {
                        "type": "object",
                        "additionalProperties": {"type": "number"},
                        "description": "结束成分组成 (元素:原子分数)"
                    },
                    "end_temperature": {
                        "type": "number",
                        "description": "结束温度(K)"
                    },
                    "steps": {
                        "type": "integer",
                        "description": "计算步数 (增量)"
                    },
                    "tdb_file": {
                        "type": "string",
                        "enum": ["FE-C-SI-MN-CU-TI-O.TDB", "B-C-SI-ZR-HF-LA-Y-TI-O.TDB"],
                        "description": "TDB 数据库文件名。请确保计算组分中所有的元素都在选择的数据库文件名中。"
                    }
                },
                "required": ["components", "start_composition", "end_composition", "start_temperature", "end_temperature", "steps", "tdb_file"]
            }),
        }
    }

    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        // 获取 API key
        let api_key = args
            .api_key
            .clone()
            .or_else(|| self.api_key.clone())
            .ok_or_else(|| CalphaMeshError::MissingParameter("api_key".to_string()))?;
        let client = CalphaMeshClient::new(api_key);
        let task_response = client.submit_line_task(args).await?;

        Ok(format!(
            "Line计算任务已提交\nID: {}\n状态: {}",
            task_response.id, task_response.status
        ))
    }
}

// 提交 Scheil 计算任务工具 (更新后)
#[derive(Deserialize, Serialize, Default)]
pub struct SubmitScheilTask {
    pub api_key: Option<String>,
}

impl SubmitScheilTask {
    pub fn new(api_key: String) -> Self {
        Self {
            api_key: Some(api_key),
        }
    }
}

impl Tool for SubmitScheilTask {
    const NAME: &'static str = "calphamesh_submit_scheil_task";

    type Error = CalphaMeshError;
    type Args = ScheilTaskParams;
    type Output = String;

    async fn definition(&self, _prompt: String) -> ToolDefinition {
        ToolDefinition {
            name: "calphamesh_submit_scheil_task".to_string(),
            description: "提交 Scheil 凝固计算任务到 Calpha Mesh 服务器".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "components": {
                        "type": "array",
                        "items": {"type": "string"},
                        "description": "组分列表"
                    },
                    "composition": {
                        "type": "object",
                        "additionalProperties": {"type": "number"},
                        "description": "成分组成 (元素:原子分数)"
                    },
                    "start_temperature": {
                        "type": "number",
                        "description": "起始温度(K)"
                    },
                    "temperature_step": {
                        "type": "number",
                        "description": "计算步长(K)，默认为 1.0"
                    },
                    "tdb_file": {
                        "type": "string",
                        "enum": ["FE-C-SI-MN-CU-TI-O.TDB", "B-C-SI-ZR-HF-LA-Y-TI-O.TDB"],
                        "description": "TDB 数据库文件名。请确保计算组分中所有的元素都在选择的数据库文件名中。"
                    }
                },
                "required": ["components", "composition", "start_temperature", "tdb_file"]
            }),
        }
    }

    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        // 获取 API key
        let api_key = args
            .api_key
            .clone()
            .or_else(|| self.api_key.clone())
            .ok_or_else(|| CalphaMeshError::MissingParameter("api_key".to_string()))?;
        let client = CalphaMeshClient::new(api_key);
        let task_response = client.submit_scheil_task(args).await?;

        Ok(format!(
            "Scheil计算任务已提交\nID: {}\n状态: {}",
            task_response.id, task_response.status
        ))
    }
}

// 提交 Binary 计算任务工具
#[derive(Deserialize, Serialize, Default)]
pub struct SubmitBinaryTask {
    pub api_key: Option<String>,
}

impl Tool for SubmitBinaryTask {
    const NAME: &'static str = "calphamesh_submit_binary_task";
    type Error = CalphaMeshError;
    type Args = BinaryTaskParams;
    type Output = String;
    async fn definition(&self, _prompt: String) -> ToolDefinition {
        ToolDefinition {
            name: Self::NAME.to_string(),
            description: "提交二元平衡相图计算任务".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                   "components": {"type": "array", "items": {"type": "string"}},
                   "start_composition": {"type": "object", "additionalProperties": {"type": "number"}},
                   "end_composition": {"type": "object", "additionalProperties": {"type": "number"}},
                   "start_temperature": {"type": "number"},
                   "end_temperature": {"type": "number"},
                    "tdb_file": {
                        "type": "string",
                        "enum": ["FE-C-SI-MN-CU-TI-O.TDB", "B-C-SI-ZR-HF-LA-Y-TI-O.TDB"],
                        "description": "TDB 数据库文件名。请确保计算组分中所有的元素都在选择的数据库文件名中。"
                    }
                },
                "required": ["components", "start_composition", "end_composition", "start_temperature", "end_temperature", "tdb_file"]
            }),
        }
    }
    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        let api_key = args
            .api_key
            .clone()
            .or_else(|| self.api_key.clone())
            .ok_or(CalphaMeshError::MissingParameter("api_key".to_string()))?;
        let client = CalphaMeshClient::new(api_key);
        let resp = client.submit_binary_task(args).await?;
        Ok(format!(
            "二元相图任务已提交\nID: {}\n状态: {}",
            resp.id, resp.status
        ))
    }
}

// 提交 Ternary 计算任务工具
#[derive(Deserialize, Serialize, Default)]
pub struct SubmitTernaryTask {
    pub api_key: Option<String>,
}

impl Tool for SubmitTernaryTask {
    const NAME: &'static str = "calphamesh_submit_ternary_task";
    type Error = CalphaMeshError;
    type Args = TernaryTaskParams;
    type Output = String;
    async fn definition(&self, _prompt: String) -> ToolDefinition {
        ToolDefinition {
            name: Self::NAME.to_string(),
            description: "提交三元组计算任务".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                   "components": {"type": "array", "items": {"type": "string"}},
                   "temperature": {"type": "number"},
                   "composition_x": {"type": "object", "additionalProperties": {"type": "number"}},
                   "composition_y": {"type": "object", "additionalProperties": {"type": "number"}},
                   "composition_o": {"type": "object", "additionalProperties": {"type": "number"}},
                   "tdb_file": {
                        "type": "string",
                        "enum": ["FE-C-SI-MN-CU-TI-O.TDB", "B-C-SI-ZR-HF-LA-Y-TI-O.TDB"],
                        "description": "TDB 数据库文件名。请确保计算组分中所有的元素都在选择的数据库文件名中。"
                   }
                },
                "required": ["components", "temperature", "composition_x", "composition_y", "composition_o", "tdb_file"]
            }),
        }
    }
    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        let api_key = args
            .api_key
            .clone()
            .or_else(|| self.api_key.clone())
            .ok_or(CalphaMeshError::MissingParameter("api_key".to_string()))?;
        let client = CalphaMeshClient::new(api_key);
        let resp = client.submit_ternary_task(args).await?;
        Ok(format!(
            "三元组计算任务已提交\nID: {}\n状态: {}",
            resp.id, resp.status
        ))
    }
}

// 提交 Boiling Point 计算任务工具
#[derive(Deserialize, Serialize, Default)]
pub struct SubmitBoilingPointTask {
    pub api_key: Option<String>,
}

impl Tool for SubmitBoilingPointTask {
    const NAME: &'static str = "calphamesh_submit_boiling_point_task";
    type Error = CalphaMeshError;
    type Args = BoilingPointParams;
    type Output = String;
    async fn definition(&self, _prompt: String) -> ToolDefinition {
        ToolDefinition {
            name: Self::NAME.to_string(),
            description: "提交沸点计算任务".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                   "components": {"type": "array", "items": {"type": "string"}},
                   "composition": {"type": "object", "additionalProperties": {"type": "number"}},
                   "pressure": {"type": "number"},
                   "temperature_range": {
                        "type": "array",
                        "items": {"type": "number"},
                        "minItems": 2,
                        "maxItems": 2
                   },
                   "tdb_file": {
                        "type": "string",
                        "enum": ["FE-C-SI-MN-CU-TI-O.TDB", "B-C-SI-ZR-HF-LA-Y-TI-O.TDB"],
                        "description": "TDB 数据库文件名。请确保计算组分中所有的元素都在选择的数据库文件名中。"
                   }
                },
                "required": ["components", "composition", "pressure", "temperature_range", "tdb_file"]
            }),
        }
    }
    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        let api_key = args
            .api_key
            .clone()
            .or_else(|| self.api_key.clone())
            .ok_or(CalphaMeshError::MissingParameter("api_key".to_string()))?;
        let client = CalphaMeshClient::new(api_key);
        let resp = client.submit_boiling_point_task(args).await?;
        Ok(format!(
            "沸点计算任务已提交\nID: {}\n状态: {}",
            resp.id, resp.status
        ))
    }
}

// 提交 Thermodynamic Properties 计算任务工具
#[derive(Deserialize, Serialize, Default)]
pub struct SubmitThermoPropertiesTask {
    pub api_key: Option<String>,
}

impl Tool for SubmitThermoPropertiesTask {
    const NAME: &'static str = "calphamesh_submit_thermodynamic_properties_task";
    type Error = CalphaMeshError;
    type Args = ThermoPropertiesParams;
    type Output = String;
    async fn definition(&self, _prompt: String) -> ToolDefinition {
        ToolDefinition {
            name: Self::NAME.to_string(),
            description: "提交热力学性质计算任务".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                   "components": {"type": "array", "items": {"type": "string"}},
                   "composition": {"type": "object", "additionalProperties": {"type": "number"}},
                   "temperature_start": {"type": "number"},
                   "temperature_end": {"type": "number"},
                   "increments": {"type": "integer"},
                   "pressure_start": {"type": "number"},
                   "pressure_end": {"type": "number"},
                   "pressure_increments": {"type": "integer"},
                   "properties": {"type": "array", "items": {"type": "string"}},
                   "tdb_file": {
                        "type": "string",
                        "enum": ["FE-C-SI-MN-CU-TI-O.TDB", "B-C-SI-ZR-HF-LA-Y-TI-O.TDB"],
                        "description": "TDB 数据库文件名。请确保计算组分中所有的元素都在选择的数据库文件名中。"
                   }
                },
                "required": ["components", "composition", "temperature_start", "temperature_end", "increments", "pressure_start", "pressure_end", "pressure_increments", "properties", "tdb_file"]
            }),
        }
    }
    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        let api_key = args
            .api_key
            .clone()
            .or_else(|| self.api_key.clone())
            .ok_or(CalphaMeshError::MissingParameter("api_key".to_string()))?;
        let client = CalphaMeshClient::new(api_key);
        let resp = client.submit_thermo_properties_task(args).await?;
        Ok(format!(
            "热力学性质计算任务已提交\nID: {}\n状态: {}",
            resp.id, resp.status
        ))
    }
}

// 查询任务状态工具
#[derive(Deserialize, Serialize, Default)]
pub struct GetTaskStatus {
    pub api_key: Option<String>,
}

impl GetTaskStatus {
    pub fn new(api_key: String) -> Self {
        Self {
            api_key: Some(api_key),
        }
    }
}

impl Tool for GetTaskStatus {
    const NAME: &'static str = "calphamesh_get_task_status";

    type Error = CalphaMeshError;
    type Args = TaskIdParams;
    type Output = String;

    async fn definition(&self, _prompt: String) -> ToolDefinition {
        ToolDefinition {
            name: "calphamesh_get_task_status".to_string(),
            description: "根据任务ID查询 Calpha Mesh 任务状态和结果".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "task_id": {
                        "type": "integer",
                        "description": "任务ID"
                    }
                },
                "required": ["task_id"]
            }),
        }
    }

    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        // 获取 API key
        let api_key = args
            .api_key
            .clone()
            .or_else(|| self.api_key.clone())
            .ok_or_else(|| CalphaMeshError::MissingParameter("api_key".to_string()))?;
        let client = CalphaMeshClient::new(api_key);

        // 轮询直到任务完成，每隔 5 秒查询一次
        let task = loop {
            let task = client.get_task_status(args.task_id).await?;
            if task.status == "completed" || task.status == "failed" || task.status == "error" {
                break task;
            }
            // 任务仍在进行中，等待 5 秒后重试
            tokio::time::sleep(std::time::Duration::from_secs(5)).await;
        };

        let mut result = String::new();
        /*
        let mut result = format!(
            "任务状态\nID: {}\n标题: {}\n类型: {}\n状态: {}\n用户ID: {}\n创建: {}\n更新: {}",
            task.id,
            task.title,
            task.task_type,
            task.status,
            task.user_id,
            task.created_at,
            task.updated_at
        );

        if let Some(result_data) = &task.result {
            result.push_str("\n\n结果:\n");
            result.push_str(&serde_json::to_string_pretty(result_data).unwrap_or_default());
        }

        if let Some(logs) = &task.logs {
            result.push_str(&format!("\n\n日志:\n{}", logs));
        }
        */

        // 任务完成时，自动获取结果文件并下载内容
        if task.status == "completed" {
            match client.get_result_files(task.id).await {
                Ok(file_resp) => {
                    // 展示所有文件链接
                    // result.push_str("\n\n结果文件:\n");
                    // for (idx, file_url) in file_resp.files.iter().enumerate() {
                    //     // 从 URL 中提取文件名
                    //     let file_name = file_url
                    //         .split('?')
                    //         .next()
                    //         .and_then(|p| p.rsplit('/').next())
                    //         .unwrap_or("unknown");
                    //     result.push_str(&format!("  {}. {} - {}\n", idx + 1, file_name, file_url));
                    // }

                    // 优先查找 .json 文件，其次 .log 文件
                    let target_url = file_resp
                        .files
                        .iter()
                        .find(|u| {
                            u.split('?')
                                .next()
                                .map(|p| p.ends_with(".json"))
                                .unwrap_or(false)
                        })
                        .or_else(|| {
                            file_resp.files.iter().find(|u| {
                                u.split('?')
                                    .next()
                                    .map(|p| p.ends_with(".log"))
                                    .unwrap_or(false)
                            })
                        });

                    if let Some(url) = target_url {
                        let file_name = url
                            .split('?')
                            .next()
                            .and_then(|p| p.rsplit('/').next())
                            .unwrap_or("file");
                        match client.download_file_content(url).await {
                            Ok(content) => {
                                // result.push_str(&format!("\n{} 内容:\n{}", file_name, content));
                                result.push_str(&content);
                            }
                            Err(e) => {
                                result.push_str(&format!("\n下载 {} 失败: {}", file_name, e));
                            }
                        }
                    }
                }
                Err(e) => {
                    result.push_str(&format!("\n\n获取结果文件列表失败: {}", e));
                }
            }
        }

        Ok(result)
    }
}

// 列出任务工具
#[derive(Deserialize, Serialize, Default)]
pub struct ListTasks {
    pub api_key: Option<String>,
}

impl ListTasks {
    pub fn new(api_key: String) -> Self {
        Self {
            api_key: Some(api_key),
        }
    }
}

impl Tool for ListTasks {
    const NAME: &'static str = "calphamesh_list_tasks";

    type Error = CalphaMeshError;
    type Args = ListTasksParams;
    type Output = String;

    async fn definition(&self, _prompt: String) -> ToolDefinition {
        ToolDefinition {
            name: "calphamesh_list_tasks".to_string(),
            description: "列出当前用户的 Calpha Mesh 任务列表".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "page": {
                        "type": "integer",
                        "description": "页码 (默认: 1)"
                    },
                    "items_per_page": {
                        "type": "integer",
                        "description": "每页项目数 (默认: 50)"
                    }
                },
                "required": []
            }),
        }
    }

    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        // 获取 API key
        let api_key = args
            .api_key
            .clone()
            .or_else(|| self.api_key.clone())
            .ok_or_else(|| CalphaMeshError::MissingParameter("api_key".to_string()))?;
        let client = CalphaMeshClient::new(api_key);
        let list = client.list_tasks(args.page, args.items_per_page).await?;

        let mut result = format!("任务列表 (第{}页/共{}页)\n\n", list.page, list.total_pages);

        if list.data.is_empty() {
            result.push_str("暂无任务");
        } else {
            for (idx, task) in list.data.iter().enumerate() {
                result.push_str(&format!(
                    "{}. [{}] {} | {} | {}\n",
                    idx + 1,
                    task.status,
                    task.id,
                    task.task_type,
                    task.title
                ));
            }
        }

        Ok(result)
    }
}
