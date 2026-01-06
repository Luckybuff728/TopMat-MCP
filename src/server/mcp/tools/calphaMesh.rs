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

// 任务相关结构体
#[derive(Debug, Serialize, Deserialize)]
pub struct CreateTaskApiKeyRequest {
    pub db_key: String,
    pub title: String,
    pub description: String,
    pub task_type: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct GetTaskApiKeyRequest {
    pub id: i32,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct GetTasksApiKeyRequest {
    pub page: i32,
    pub items_per_page: i32,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct UpdateTaskStatusApiKeyRequest {
    pub id: i32,
    pub status: String,
    pub result: String,
}

#[derive(Debug, Deserialize)]
pub struct TaskResponse {
    pub id: i32,
    pub status: String,
    pub task_type: String,
}

#[derive(Debug, Deserialize)]
pub struct TaskStatusResponse {
    pub id: i32,
    pub title: String,
    pub description: String,
    pub status: String,
    pub task_type: String,
    pub result: Option<String>,
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

// Point 计算参数
#[derive(Debug, Serialize, Deserialize)]
pub struct PointTaskParams {
    #[serde(default = "default_components")]
    pub components: Vec<String>,
    #[serde(default = "default_composition")]
    pub composition: HashMap<String, f64>,
    #[serde(default = "default_temperature")]
    pub temperature: f64,
    #[serde(default = "default_pressure")]
    pub pressure: f64,
    #[serde(default = "default_database")]
    pub database: String,
    /// Calpha Mesh API 密钥
    pub api_key: String,
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
    #[serde(default = "default_pressure")]
    pub pressure: f64,
    #[serde(default = "default_steps")]
    pub steps: i64,
    #[serde(default = "default_database")]
    pub database: String,
    /// Calpha Mesh API 密钥
    pub api_key: String,
}

// Scheil 计算参数
#[derive(Debug, Serialize, Deserialize)]
pub struct ScheilTaskParams {
    #[serde(default = "default_components")]
    pub components: Vec<String>,
    #[serde(default = "default_composition")]
    pub composition: HashMap<String, f64>,
    #[serde(default = "default_scheil_temperature")]
    pub temperature: f64,
    #[serde(default = "default_scheil_pressure")]
    pub pressure: f64,
    #[serde(default = "default_database")]
    pub database: String,
    /// Calpha Mesh API 密钥
    pub api_key: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct TaskIdParams {
    pub task_id: i32,
    /// Calpha Mesh API 密钥
    pub api_key: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ListTasksParams {
    #[serde(default = "default_page")]
    pub page: i32,
    #[serde(default = "default_items_per_page")]
    pub items_per_page: i32,
    /// Calpha Mesh API 密钥
    pub api_key: String,
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

    async fn make_request(&self, url: &str, body: String) -> Result<String, CalphaMeshError> {
        let response = self
            .client
            .post(url)
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("Content-Type", "application/json")
            .body(body)
            .send()
            .await
            .map_err(|e| CalphaMeshError::HttpError(e.to_string()))?;

        let status = response.status().as_u16();
        let response_text = response
            .text()
            .await
            .map_err(|e| CalphaMeshError::HttpError(e.to_string()))?;

        if status == 200 || status == 201 {
            Ok(response_text)
        } else {
            Err(CalphaMeshError::ApiError {
                status,
                message: response_text,
            })
        }
    }

    pub async fn submit_point_task(
        &self,
        params: PointTaskParams,
    ) -> Result<TaskResponse, CalphaMeshError> {
        let task_description = json!({
            "task_type": "point",
            "components": params.components,
            "config": {
                "conditions": {
                    "equilibrium_type": {"@type": "global"},
                    "driving_force": {"@value": true}
                },
                "suspended_phases": ["*"],
                "entered_phases": ["Liquid", "Fcc"],
                "targets": ["T", "G(@*)", "phase_name", "mu(*@*)"],
                "n_unit": "x"
            },
            "ctp": {
                "composition": params.composition,
                "temperature": params.temperature,
                "pressure": params.pressure
            },
            "database": params.database
        });

        let title = format!("Task-Point-{}", chrono::Utc::now().timestamp());
        let create_body = CreateTaskApiKeyRequest {
            db_key: params.database.clone(),
            title: title.clone(),
            description: task_description.to_string(),
            task_type: "point".to_string(),
        };

        let url = format!("{}/api/v1/create_task", API_BASE_URL);
        let response_text = self
            .make_request(&url, serde_json::to_string(&create_body)?)
            .await?;
        let task_response: TaskResponse = serde_json::from_str(&response_text)?;

        Ok(task_response)
    }

    pub async fn submit_line_task(
        &self,
        params: LineTaskParams,
    ) -> Result<TaskResponse, CalphaMeshError> {
        let task_description = json!({
            "task_type": "line",
            "components": params.components,
            "ctp": {
                "composition": params.start_composition,
                "temperature": params.start_temperature,
                "pressure": params.pressure
            },
            "ctp_1": {
                "composition": params.end_composition,
                "temperature": params.end_temperature,
                "pressure": params.pressure
            },
            "ctp_steps": params.steps,
            "config": {
                "conditions": {
                    "equilibrium_type": {"@type": "global"},
                    "driving_force": {"@value": true}
                },
                "suspended_phases": ["*"],
                "entered_phases": ["Liquid", "Fcc"],
                "targets": ["T", "G(@*)", "phase_name", "mu(*@*)"],
                "n_unit": "x"
            },
            "database": params.database,
            "type": "line",
            "name": format!("Task-Line-{}", chrono::Utc::now().timestamp())
        });

        let create_body = CreateTaskApiKeyRequest {
            db_key: params.database.clone(),
            title: format!("Task-Line-{}", chrono::Utc::now().timestamp()),
            description: task_description.to_string(),
            task_type: "line".to_string(),
        };

        let url = format!("{}/api/v1/create_task", API_BASE_URL);
        let response_text = self
            .make_request(&url, serde_json::to_string(&create_body)?)
            .await?;
        let task_response: TaskResponse = serde_json::from_str(&response_text)?;

        Ok(task_response)
    }

    pub async fn submit_scheil_task(
        &self,
        params: ScheilTaskParams,
    ) -> Result<TaskResponse, CalphaMeshError> {
        let task_description = json!({
            "task_type": "scheil",
            "components": params.components,
            "ctp": {
                "composition": params.composition,
                "temperature": params.temperature,
                "pressure": params.pressure
            },
            "config": {
                "targets": ["fl", "fs", "phase_name", "Label", "f_tot(@*)", "f(@*)", "T//fs", "Q"],
                "entered_phases": ["*"],
                "suspended_phases": ["*"],
                "n_unit": "x",
                "conditions": {
                    "step_T_max": {"@value": "1"},
                    "model": {"@type": "Scheil"},
                    "start_from_liquidus_surface": {"@value": "yes"},
                    "end_when_no_more_liquid": {"@value": "yes"},
                    "T_end": {"@value": "300"},
                    "step_T_min": {"@value": "0.01"},
                    "liquid_amount_min": {"@value": "0.001"},
                    "x_min": {"@value": "1e-12"}
                }
            },
            "database": params.database,
            "name": format!("Task-Scheil-{}", chrono::Utc::now().timestamp())
        });

        let create_body = CreateTaskApiKeyRequest {
            db_key: params.database.clone(),
            title: format!("Task-Scheil-{}", chrono::Utc::now().timestamp()),
            description: task_description.to_string(),
            task_type: "scheil".to_string(),
        };

        let url = format!("{}/api/v1/create_task", API_BASE_URL);
        let response_text = self
            .make_request(&url, serde_json::to_string(&create_body)?)
            .await?;
        let task_response: TaskResponse = serde_json::from_str(&response_text)?;

        Ok(task_response)
    }

    pub async fn get_task_status(
        &self,
        task_id: i32,
    ) -> Result<TaskStatusResponse, CalphaMeshError> {
        if task_id <= 0 {
            return Err(CalphaMeshError::InvalidTaskId(task_id));
        }

        let get_task_body = GetTaskApiKeyRequest { id: task_id };
        let url = format!("{}/api/v1/get_task", API_BASE_URL);
        let response_text = self
            .make_request(&url, serde_json::to_string(&get_task_body)?)
            .await?;
        let task: TaskStatusResponse = serde_json::from_str(&response_text)?;

        Ok(task)
    }

    pub async fn list_tasks(
        &self,
        page: i32,
        items_per_page: i32,
    ) -> Result<TaskListResponse, CalphaMeshError> {
        let get_tasks_body = GetTasksApiKeyRequest {
            page,
            items_per_page,
        };
        let url = format!("{}/api/v1/get_tasks", API_BASE_URL);
        let response_text = self
            .make_request(&url, serde_json::to_string(&get_tasks_body)?)
            .await?;
        let list: TaskListResponse = serde_json::from_str(&response_text)?;

        Ok(list)
    }
}

// 工具实现

// 提交 Point 计算任务工具
#[derive(Deserialize, Serialize)]
pub struct SubmitPointTask;

impl Default for SubmitPointTask {
    fn default() -> Self {
        Self
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
                        "description": "成分组成 (元素:原子分数)，原子分数之和必须为1"
                    },
                    "temperature": {
                        "type": "number",
                        "description": "计算温度(K)"
                    },
                    "pressure": {
                        "type": "number",
                        "description": "计算压力(atm)"
                    },
                    "database": {
                        "type": "string",
                        "description": "数据库名称，默认为 default"
                    }
                },
                "required": []
            }),
        }
    }

    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        // 验证组分之和
        if let Some(error_msg) = validate_composition_sum(&args.composition) {
            return Ok(error_msg);
        }

        // 使用参数中的 API key
        let client = CalphaMeshClient::new(args.api_key.clone());
        let task_response = client.submit_point_task(args).await?;

        Ok(format!(
            "Point计算任务已提交\nID: {}\n状态: {}",
            task_response.id, task_response.status
        ))
    }
}

// 提交 Line 计算任务工具
#[derive(Deserialize, Serialize)]
pub struct SubmitLineTask;

impl Default for SubmitLineTask {
    fn default() -> Self {
        Self
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
                        "description": "起始成分组成 (元素:原子分数)，原子分数之和必须为1"
                    },
                    "start_temperature": {
                        "type": "number",
                        "description": "起始温度(K)"
                    },
                    "end_composition": {
                        "type": "object",
                        "additionalProperties": {"type": "number"},
                        "description": "结束成分组成 (元素:原子分数)，原子分数之和必须为1"
                    },
                    "end_temperature": {
                        "type": "number",
                        "description": "结束温度(K)"
                    },
                    "steps": {
                        "type": "integer",
                        "description": "计算步数"
                    },
                    "pressure": {
                        "type": "number",
                        "description": "计算压力(atm)"
                    },
                    "database": {
                        "type": "string",
                        "description": "数据库名称，默认为 default"
                    }
                },
                "required": []
            }),
        }
    }

    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        // 验证起始组分之和
        if let Some(error_msg) = validate_composition_sum(&args.start_composition) {
            return Ok(format!("起始组分错误: {}", error_msg));
        }
        // 验证结束组分之和
        if let Some(error_msg) = validate_composition_sum(&args.end_composition) {
            return Ok(format!("结束组分错误: {}", error_msg));
        }

        // 使用参数中的 API key
        let client = CalphaMeshClient::new(args.api_key.clone());
        let task_response = client.submit_line_task(args).await?;

        Ok(format!(
            "Line计算任务已提交\nID: {}\n状态: {}",
            task_response.id, task_response.status
        ))
    }
}

// 提交 Scheil 计算任务工具
#[derive(Deserialize, Serialize)]
pub struct SubmitScheilTask;

impl Default for SubmitScheilTask {
    fn default() -> Self {
        Self
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
                        "description": "成分组成 (元素:原子分数)，原子分数之和必须为1"
                    },
                    "temperature": {
                        "type": "number",
                        "description": "起始温度(K)"
                    },
                    "pressure": {
                        "type": "number",
                        "description": "计算压力(atm)"
                    },
                    "database": {
                        "type": "string",
                        "description": "数据库名称，默认为 default"
                    }
                },
                "required": []
            }),
        }
    }

    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        // 验证组分之和
        if let Some(error_msg) = validate_composition_sum(&args.composition) {
            return Ok(error_msg);
        }

        // 使用参数中的 API key
        let client = CalphaMeshClient::new(args.api_key.clone());
        let task_response = client.submit_scheil_task(args).await?;

        Ok(format!(
            "Scheil计算任务已提交\nID: {}\n状态: {}",
            task_response.id, task_response.status
        ))
    }
}

// 查询任务状态工具
#[derive(Deserialize, Serialize)]
pub struct GetTaskStatus;

impl Default for GetTaskStatus {
    fn default() -> Self {
        Self
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
        // 使用参数中的 API key
        let client = CalphaMeshClient::new(args.api_key.clone());
        let task = client.get_task_status(args.task_id).await?;

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
            result.push_str(result_data);
        }

        if let Some(logs) = &task.logs {
            result.push_str(&format!("\n\n日志:\n{}", logs));
        }

        Ok(result)
    }
}

// 列出任务工具
#[derive(Deserialize, Serialize)]
pub struct ListTasks;

impl Default for ListTasks {
    fn default() -> Self {
        Self
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
        // 使用参数中的 API key
        let client = CalphaMeshClient::new(args.api_key.clone());
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
