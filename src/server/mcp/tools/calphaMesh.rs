//! Calpha Mesh MCP 工具
//!
//! 提供与 CalphaMesh API 交互的工具，用于提交热力学计算任务和查询结果。
//! 基于 CALPHA_MCP_CONTRACT_DESIGN.md v2.1 设计稿实现。

use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use serde_json::json;
use std::error::Error as StdError;

use rig::{completion::ToolDefinition, tool::Tool};

const API_BASE_URL: &str = "https://api.topmaterial-tech.com";

// ═══════════════════════════════════════════════════════════════════
// TDB 静态映射表（单一维护点，enum 和校验逻辑均从此表派生）
// ═══════════════════════════════════════════════════════════════════

const TDB_ELEMENT_MAP: &[(&str, &[&str])] = &[
    (
        "FE-C-SI-MN-CU-TI-O.TDB",
        &["FE", "C", "SI", "MN", "CU", "TI", "O"],
    ),
    (
        "B-C-SI-ZR-HF-LA-Y-TI-O.TDB",
        &["B", "C", "SI", "ZR", "HF", "LA", "Y", "TI", "O"],
    ),
];

fn tdb_enum_values() -> Vec<&'static str> {
    TDB_ELEMENT_MAP.iter().map(|(name, _)| *name).collect()
}

fn tdb_elements(tdb_file: &str) -> Option<&'static [&'static str]> {
    TDB_ELEMENT_MAP
        .iter()
        .find(|(name, _)| *name == tdb_file)
        .map(|(_, elements)| *elements)
}

// ═══════════════════════════════════════════════════════════════════
// 错误类型
// ═══════════════════════════════════════════════════════════════════

#[derive(Debug)]
pub enum CalphaMeshError {
    HttpError(String),
    ApiError { status: u16, message: String },
    JsonError(serde_json::Error),
    InvalidTaskId(i32),
    MissingParameter(String),
    ValidationError(String),
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
            CalphaMeshError::ValidationError(msg) => {
                write!(f, "{}", msg)
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

// ═══════════════════════════════════════════════════════════════════
// 统一前置校验层
// ═══════════════════════════════════════════════════════════════════

fn validate_composition_sum(composition: &HashMap<String, f64>) -> Result<(), CalphaMeshError> {
    let sum: f64 = composition.values().sum();
    const TOLERANCE: f64 = 1e-6;
    if (sum - 1.0).abs() > TOLERANCE {
        let detail: Vec<String> = composition
            .iter()
            .map(|(k, v)| format!("{}={:.6}", k, v))
            .collect();
        return Err(CalphaMeshError::ValidationError(format!(
            "组分原子分数之和为 {:.6}，必须等于 1.0（实际：{}）",
            sum,
            detail.join(", ")
        )));
    }
    Ok(())
}

fn validate_components_match_composition(
    components: &[String],
    composition: &HashMap<String, f64>,
) -> Result<(), CalphaMeshError> {
    let mut comp_keys: Vec<&str> = composition.keys().map(|s| s.as_str()).collect();
    comp_keys.sort();
    let mut comp_list: Vec<&str> = components.iter().map(|s| s.as_str()).collect();
    comp_list.sort();
    if comp_keys != comp_list {
        return Err(CalphaMeshError::ValidationError(format!(
            "components {:?} 与 composition 的键 {:?} 不一致",
            components,
            composition.keys().collect::<Vec<_>>()
        )));
    }
    Ok(())
}

fn validate_tdb_contains_elements(
    tdb_file: &str,
    components: &[String],
) -> Result<(), CalphaMeshError> {
    let elements = tdb_elements(tdb_file).ok_or_else(|| {
        CalphaMeshError::ValidationError(format!(
            "不支持的 tdb_file: {}，可选值: {:?}",
            tdb_file,
            tdb_enum_values()
        ))
    })?;
    for comp in components {
        let upper = comp.to_uppercase();
        if !elements.contains(&upper.as_str()) {
            return Err(CalphaMeshError::ValidationError(format!(
                "tdb_file {} 不包含元素 {}，该数据库包含的元素: {:?}",
                tdb_file, comp, elements
            )));
        }
    }
    Ok(())
}

fn validate_temperature_range(temp: f64, min: f64, max: f64, field: &str) -> Result<(), CalphaMeshError> {
    if temp < min || temp > max {
        return Err(CalphaMeshError::ValidationError(format!(
            "{} = {} 超出有效范围 {}~{} K",
            field, temp, min, max
        )));
    }
    Ok(())
}

fn validate_point_params(params: &PointTaskParams) -> Result<(), CalphaMeshError> {
    validate_composition_sum(&params.composition)?;
    validate_components_match_composition(&params.components, &params.composition)?;
    validate_temperature_range(params.temperature, 200.0, 6000.0, "temperature")?;
    validate_tdb_contains_elements(&params.tdb_file, &params.components)?;
    Ok(())
}

fn validate_line_params(params: &LineTaskParams) -> Result<(), CalphaMeshError> {
    validate_composition_sum(&params.start_composition)?;
    validate_composition_sum(&params.end_composition)?;
    validate_components_match_composition(&params.components, &params.start_composition)?;
    validate_components_match_composition(&params.components, &params.end_composition)?;
    validate_temperature_range(params.start_temperature, 200.0, 6000.0, "start_temperature")?;
    validate_temperature_range(params.end_temperature, 200.0, 6000.0, "end_temperature")?;
    if params.steps < 2 || params.steps > 500 {
        return Err(CalphaMeshError::ValidationError(format!(
            "steps = {} 超出有效范围 2~500",
            params.steps
        )));
    }
    validate_tdb_contains_elements(&params.tdb_file, &params.components)?;
    Ok(())
}

fn validate_scheil_params(params: &ScheilTaskParams) -> Result<(), CalphaMeshError> {
    validate_composition_sum(&params.composition)?;
    validate_components_match_composition(&params.components, &params.composition)?;
    validate_temperature_range(params.start_temperature, 500.0, 6000.0, "start_temperature")?;
    if params.temperature_step < 0.1 || params.temperature_step > 50.0 {
        return Err(CalphaMeshError::ValidationError(format!(
            "temperature_step = {} 超出有效范围 0.1~50 K",
            params.temperature_step
        )));
    }
    validate_tdb_contains_elements(&params.tdb_file, &params.components)?;
    Ok(())
}

// ═══════════════════════════════════════════════════════════════════
// API 请求/响应结构体
// ═══════════════════════════════════════════════════════════════════

#[derive(Debug, Serialize, Deserialize)]
pub struct CreateTaskRequest {
    pub title: String,
    pub description: String,
    pub task_type: String,
    pub db_key: String,
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

// ═══════════════════════════════════════════════════════════════════
// 工具参数结构体
// ═══════════════════════════════════════════════════════════════════

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
    #[serde(default)]
    pub api_key: Option<String>,
}

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
    #[serde(default)]
    pub api_key: Option<String>,
}

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
    #[serde(default)]
    pub api_key: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct BinaryTaskParams {
    pub components: Vec<String>,
    pub start_composition: HashMap<String, f64>,
    pub end_composition: HashMap<String, f64>,
    pub start_temperature: f64,
    pub end_temperature: f64,
    pub tdb_file: String,
    #[serde(default)]
    pub api_key: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct TernaryTaskParams {
    pub components: Vec<String>,
    pub temperature: f64,
    pub composition_x: HashMap<String, f64>,
    pub composition_y: HashMap<String, f64>,
    pub composition_o: HashMap<String, f64>,
    pub tdb_file: String,
    #[serde(default)]
    pub api_key: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct BoilingPointParams {
    pub components: Vec<String>,
    pub composition: HashMap<String, f64>,
    pub pressure: f64,
    pub temperature_range: (f64, f64),
    pub tdb_file: String,
    #[serde(default)]
    pub api_key: Option<String>,
}

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
    #[serde(default)]
    pub api_key: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct TaskIdParams {
    pub task_id: i32,
    #[serde(default)]
    pub api_key: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct GetTaskResultParams {
    pub task_id: i32,
    #[serde(default = "default_timeout_seconds")]
    pub timeout_seconds: i64,
    #[serde(default = "default_result_mode")]
    pub result_mode: String,
    #[serde(default)]
    pub api_key: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ListTasksParams {
    #[serde(default = "default_page")]
    pub page: i32,
    #[serde(default = "default_items_per_page")]
    pub items_per_page: i32,
    #[serde(default)]
    pub api_key: Option<String>,
}

// ═══════════════════════════════════════════════════════════════════
// 默认值函数
// ═══════════════════════════════════════════════════════════════════

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
fn default_steps() -> i64 {
    50
}
fn default_database() -> String {
    "FE-C-SI-MN-CU-TI-O.TDB".to_string()
}
fn default_temperature_step() -> f64 {
    1.0
}
fn default_page() -> i32 {
    1
}
fn default_items_per_page() -> i32 {
    20
}
fn default_timeout_seconds() -> i64 {
    60
}
fn default_result_mode() -> String {
    "summary".to_string()
}

// ═══════════════════════════════════════════════════════════════════
// CalphaMesh API 客户端
// ═══════════════════════════════════════════════════════════════════

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

    fn generate_task_path(task_type_slug: &str) -> String {
        format!(
            "mcp_results/{}/{}",
            task_type_slug,
            chrono::Utc::now().timestamp_millis()
        )
    }

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
            "task_path": Self::generate_task_path("point"),
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
            "task_path": Self::generate_task_path("line"),
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
            "task_path": Self::generate_task_path("scheil"),
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
            "task_path": Self::generate_task_path("binary"),
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
            "task_path": Self::generate_task_path("ternary"),
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
            "task_path": Self::generate_task_path("boiling"),
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
            "task_path": Self::generate_task_path("properties"),
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

    pub async fn download_file_content(&self, url: &str) -> Result<String, CalphaMeshError> {
        // presigned URL 指向内网对象存储（taskman.fs.skyzcstack.space），
        // 该存储要求 Authorization 头才可访问，不带头会返回 403。
        let response = self
            .client
            .get(url)
            .header("Authorization", format!("Bearer {}", self.api_key))
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
                message: format!("下载文件失败 (status={}): {}", status, &text[..text.len().min(200)]),
            })
        }
    }
}

// ═══════════════════════════════════════════════════════════════════
// 辅助：从文件 URL 提取文件名
// ═══════════════════════════════════════════════════════════════════

fn extract_filename(url: &str) -> String {
    url.split('?')
        .next()
        .and_then(|p| p.rsplit('/').next())
        .unwrap_or("unknown")
        .to_string()
}

fn build_files_map(file_urls: &[String]) -> serde_json::Value {
    let mut map = serde_json::Map::new();
    for url in file_urls {
        let name = extract_filename(url);
        map.insert(name, serde_json::Value::String(url.clone()));
    }
    serde_json::Value::Object(map)
}

fn get_api_key(args_key: &Option<String>, self_key: &Option<String>) -> Result<String, CalphaMeshError> {
    args_key
        .clone()
        .or_else(|| self_key.clone())
        .ok_or_else(|| CalphaMeshError::MissingParameter("api_key".to_string()))
}

fn format_components_summary(components: &[String]) -> String {
    components.join("-")
}

// ═══════════════════════════════════════════════════════════════════
// 工具实现：SubmitPointTask
// ═══════════════════════════════════════════════════════════════════

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
            description: "提交一个单点热力学平衡计算任务。给定合金组成（原子分数）和温度，计算该状态下的稳定相、相分数、热力学性质（GM/HM/SM/CPM，单位 J/mol）和各组分化学势（J/mol）。\n\n任务异步执行（典型耗时 10-20 秒）。提交后立即获得 task_id，然后调用 calphamesh_get_task_result 等待结果。每次调用都会创建新任务——若需避免重复计算，请先用 calphamesh_list_tasks 确认是否已有相同任务。".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "components": {
                        "type": "array",
                        "items": {"type": "string"},
                        "description": "合金组分元素列表，元素名称必须大写（如 FE、C、SI、MN）。必须与 composition 的键完全一致，至少包含 2 个元素。示例：[\"FE\", \"C\", \"SI\"]",
                        "minItems": 2
                    },
                    "composition": {
                        "type": "object",
                        "additionalProperties": {"type": "number", "minimum": 0, "maximum": 1},
                        "description": "各组分的原子分数（摩尔分数），键为元素名称（大写），值为 0~1 的数值。所有值之和必须严格等于 1.0（容差 1e-6）。示例：{\"FE\": 0.95, \"C\": 0.03, \"SI\": 0.02}"
                    },
                    "temperature": {
                        "type": "number",
                        "description": "计算温度，单位：K（开尔文）。有效范围 200~6000 K。示例：1273.15（即 1000°C）",
                        "minimum": 200,
                        "maximum": 6000
                    },
                    "tdb_file": {
                        "type": "string",
                        "enum": ["FE-C-SI-MN-CU-TI-O.TDB", "B-C-SI-ZR-HF-LA-Y-TI-O.TDB"],
                        "description": "热力学数据库文件名。必须包含 components 中所有元素。FE 基合金选 FE-C-SI-MN-CU-TI-O.TDB，硼化物/硅化物选 B-C-SI-ZR-HF-LA-Y-TI-O.TDB。"
                    }
                },
                "required": ["components", "composition", "temperature", "tdb_file"],
                "additionalProperties": false
            }),
        }
    }

    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        validate_point_params(&args)?;
        let api_key = get_api_key(&args.api_key, &self.api_key)?;
        let system = format_components_summary(&args.components);
        let temp = args.temperature;
        let client = CalphaMeshClient::new(api_key);
        let task_response = client.submit_point_task(args).await?;

        let output = json!({
            "task_id": task_response.id,
            "status": task_response.status,
            "task_type": "point_calculation",
            "summary": format!("Point 计算任务已提交：{} 体系，{} K", system, temp),
            "estimated_wait_seconds": 15,
            "next_action": format!("调用 calphamesh_get_task_result(task_id={}) 等待并获取结果", task_response.id)
        });
        Ok(serde_json::to_string(&output).unwrap_or_default())
    }
}

// ═══════════════════════════════════════════════════════════════════
// 工具实现：SubmitLineTask
// ═══════════════════════════════════════════════════════════════════

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
            description: "提交一条从起始状态到终止状态的线性扫描计算任务。可以同时变化温度和组成，在指定步数内计算一系列平衡状态，用于绘制温度-性质曲线或伪二元截面。\n\n任务异步执行（典型耗时 15-30 秒）。提交后调用 calphamesh_get_task_result 获取结果。".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "components": {
                        "type": "array",
                        "items": {"type": "string"},
                        "description": "合金组分元素列表，元素名称必须大写。必须与 start_composition 和 end_composition 的键完全一致。",
                        "minItems": 2
                    },
                    "start_composition": {
                        "type": "object",
                        "additionalProperties": {"type": "number", "minimum": 0, "maximum": 1},
                        "description": "扫描起始点各组分的原子分数，所有值之和必须等于 1.0。"
                    },
                    "end_composition": {
                        "type": "object",
                        "additionalProperties": {"type": "number", "minimum": 0, "maximum": 1},
                        "description": "扫描终止点各组分的原子分数，所有值之和必须等于 1.0。"
                    },
                    "start_temperature": {
                        "type": "number",
                        "description": "起始温度，单位 K，范围 200~6000。",
                        "minimum": 200,
                        "maximum": 6000
                    },
                    "end_temperature": {
                        "type": "number",
                        "description": "终止温度，单位 K，范围 200~6000。",
                        "minimum": 200,
                        "maximum": 6000
                    },
                    "steps": {
                        "type": "integer",
                        "description": "扫描步数（将起止区间等分为 steps 段，产生 steps+1 个数据点）。范围 2~500，默认 50。",
                        "minimum": 2,
                        "maximum": 500
                    },
                    "tdb_file": {
                        "type": "string",
                        "enum": ["FE-C-SI-MN-CU-TI-O.TDB", "B-C-SI-ZR-HF-LA-Y-TI-O.TDB"],
                        "description": "热力学数据库文件名。必须包含 components 中所有元素。"
                    }
                },
                "required": ["components", "start_composition", "end_composition", "start_temperature", "end_temperature", "steps", "tdb_file"],
                "additionalProperties": false
            }),
        }
    }

    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        validate_line_params(&args)?;
        let api_key = get_api_key(&args.api_key, &self.api_key)?;
        let system = format_components_summary(&args.components);
        let t_start = args.start_temperature;
        let t_end = args.end_temperature;
        let steps = args.steps;
        let client = CalphaMeshClient::new(api_key);
        let task_response = client.submit_line_task(args).await?;

        let output = json!({
            "task_id": task_response.id,
            "status": task_response.status,
            "task_type": "line_calculation",
            "summary": format!("Line 计算任务已提交：{} 体系，{}→{} K，{} 步（{} 个数据点）", system, t_start, t_end, steps, steps + 1),
            "estimated_wait_seconds": 20,
            "next_action": format!("调用 calphamesh_get_task_result(task_id={}) 等待并获取结果", task_response.id)
        });
        Ok(serde_json::to_string(&output).unwrap_or_default())
    }
}

// ═══════════════════════════════════════════════════════════════════
// 工具实现：SubmitScheilTask
// ═══════════════════════════════════════════════════════════════════

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
            description: "提交一个 Scheil 凝固模拟任务。从指定起始温度逐步降温，模拟非平衡凝固过程（不允许固相中的扩散），计算液相分数、固相分数随温度的变化曲线。\n\n任务异步执行（典型耗时 20-40 秒）。提交后调用 calphamesh_get_task_result 获取凝固曲线等结构化结果。".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "components": {
                        "type": "array",
                        "items": {"type": "string"},
                        "description": "合金组分元素列表，元素名称必须大写。必须与 composition 的键完全一致。",
                        "minItems": 2
                    },
                    "composition": {
                        "type": "object",
                        "additionalProperties": {"type": "number", "minimum": 0, "maximum": 1},
                        "description": "合金的初始原子分数组成。所有值之和必须等于 1.0。"
                    },
                    "start_temperature": {
                        "type": "number",
                        "description": "凝固模拟的起始温度，单位 K。应设置在预期液相线温度以上（通常高于实际液相线 50~200 K）。范围 500~6000 K。",
                        "minimum": 500,
                        "maximum": 6000
                    },
                    "temperature_step": {
                        "type": "number",
                        "description": "每步降温幅度，单位 K。值越小精度越高但耗时越长。范围 0.1~50 K，默认 1.0 K。",
                        "minimum": 0.1,
                        "maximum": 50
                    },
                    "tdb_file": {
                        "type": "string",
                        "enum": ["FE-C-SI-MN-CU-TI-O.TDB", "B-C-SI-ZR-HF-LA-Y-TI-O.TDB"],
                        "description": "热力学数据库文件名。必须包含 components 中所有元素。"
                    }
                },
                "required": ["components", "composition", "start_temperature", "tdb_file"],
                "additionalProperties": false
            }),
        }
    }

    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        validate_scheil_params(&args)?;
        let api_key = get_api_key(&args.api_key, &self.api_key)?;
        let system = format_components_summary(&args.components);
        let t_start = args.start_temperature;
        let t_step = args.temperature_step;
        let client = CalphaMeshClient::new(api_key);
        let task_response = client.submit_scheil_task(args).await?;

        let output = json!({
            "task_id": task_response.id,
            "status": task_response.status,
            "task_type": "scheil_solidification",
            "summary": format!("Scheil 凝固任务已提交：{} 体系，起始温度 {} K，步长 {} K", system, t_start, t_step),
            "estimated_wait_seconds": 30,
            "next_action": format!("调用 calphamesh_get_task_result(task_id={}) 等待并获取结果", task_response.id)
        });
        Ok(serde_json::to_string(&output).unwrap_or_default())
    }
}

// ═══════════════════════════════════════════════════════════════════
// 工具实现：GetTaskStatus（非阻塞）
// ═══════════════════════════════════════════════════════════════════

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
            description: "快速查询指定任务的当前状态，不等待，立即返回。适用于需要了解后台任务进度而不想阻塞的场景。大多数情况下，直接使用 calphamesh_get_task_result 即可（它会自动等待完成）。".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "task_id": {
                        "type": "integer",
                        "description": "任务 ID。",
                        "minimum": 1
                    }
                },
                "required": ["task_id"],
                "additionalProperties": false
            }),
        }
    }

    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        let api_key = get_api_key(&args.api_key, &self.api_key)?;
        let client = CalphaMeshClient::new(api_key);
        let task = client.get_task_status(args.task_id).await?;

        let result_ready = task.status == "completed";
        let next_action = if result_ready {
            format!(
                "调用 calphamesh_get_task_result(task_id={}) 获取结果",
                task.id
            )
        } else if task.status == "failed" || task.status == "error" {
            format!("任务 {} 已失败，请检查参数后重新提交", task.id)
        } else {
            format!(
                "任务仍在运行中，调用 calphamesh_get_task_result(task_id={}) 等待完成",
                task.id
            )
        };

        let output = json!({
            "task_id": task.id,
            "status": task.status,
            "task_type": task.task_type,
            "title": task.title,
            "created_at": task.created_at,
            "updated_at": task.updated_at,
            "result_ready": result_ready,
            "next_action": next_action
        });
        Ok(serde_json::to_string(&output).unwrap_or_default())
    }
}

// ═══════════════════════════════════════════════════════════════════
// 工具实现：GetTaskResult（阻塞语义）
// ═══════════════════════════════════════════════════════════════════

#[derive(Deserialize, Serialize, Default)]
pub struct GetTaskResult {
    pub api_key: Option<String>,
}

impl GetTaskResult {
    pub fn new(api_key: String) -> Self {
        Self {
            api_key: Some(api_key),
        }
    }
}

impl Tool for GetTaskResult {
    const NAME: &'static str = "calphamesh_get_task_result";
    type Error = CalphaMeshError;
    type Args = GetTaskResultParams;
    type Output = String;

    async fn definition(&self, _prompt: String) -> ToolDefinition {
        ToolDefinition {
            name: "calphamesh_get_task_result".to_string(),
            description: "等待指定 CalphaMesh 任务完成并返回结构化计算结果。此工具会阻塞等待直到任务进入终态（completed / failed / error）或超过 timeout_seconds。\n\n- 任务完成：返回结构化结果（默认 summary 模式，适合 LLM 和前端消费；full 模式返回完整数据）\n- 任务失败：返回 isError 及失败原因\n- 超时：返回 still_running 状态，可再次调用继续等待\n\n对 Point 类任务，result 是单条记录；对 Line/Scheil 类任务，result 采用 data_summary + derived_metrics 结构。".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "task_id": {
                        "type": "integer",
                        "description": "任务 ID，由 calphamesh_submit_* 工具返回。",
                        "minimum": 1
                    },
                    "timeout_seconds": {
                        "type": "integer",
                        "description": "等待任务完成的最大秒数。超时后返回 still_running 状态，可再次调用。默认 60，最大 90。",
                        "minimum": 10,
                        "maximum": 90
                    },
                    "result_mode": {
                        "type": "string",
                        "enum": ["summary", "full"],
                        "description": "结果详细程度。summary（默认）：返回 data_summary + derived_metrics。full：在 summary 基础上追加 raw_data。"
                    }
                },
                "required": ["task_id"],
                "additionalProperties": false
            }),
        }
    }

    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        let api_key = get_api_key(&args.api_key, &self.api_key)?;
        let client = CalphaMeshClient::new(api_key);

        let timeout = args.timeout_seconds.min(90).max(10);
        let start = std::time::Instant::now();
        let poll_interval = std::time::Duration::from_secs(8);

        let task = loop {
            let task = client.get_task_status(args.task_id).await?;
            match task.status.as_str() {
                "completed" | "failed" | "error" => break task,
                _ => {
                    if start.elapsed().as_secs() as i64 >= timeout {
                        let output = json!({
                            "task_id": args.task_id,
                            "status": "still_running",
                            "elapsed_seconds": start.elapsed().as_secs(),
                            "retry_after_seconds": 30,
                            "message": format!("任务仍在计算中，请 30 秒后再次调用 calphamesh_get_task_result(task_id={})", args.task_id)
                        });
                        return Ok(serde_json::to_string(&output).unwrap_or_default());
                    }
                    tokio::time::sleep(poll_interval).await;
                }
            }
        };

        if task.status == "failed" || task.status == "error" {
            let output = json!({
                "error_code": "task_failed",
                "task_id": args.task_id,
                "message": format!("任务计算失败，后端返回 status: {}", task.status),
                "retryable": false,
                "details": "请检查 tdb_file 是否包含所有 components 元素，或调整参数后重新提交"
            });
            return Err(CalphaMeshError::ValidationError(
                serde_json::to_string(&output).unwrap_or_default(),
            ));
        }

        let file_resp = client.get_result_files(task.id).await?;
        let files_map = build_files_map(&file_resp.files);

        let has_results_json = file_resp
            .files
            .iter()
            .any(|u| extract_filename(u) == "results.json");
        // 实际后端输出为 scheil_conditions.json，保留旧名兼容
        let has_scheil_json = file_resp
            .files
            .iter()
            .any(|u| {
                let name = extract_filename(u);
                name == "scheil_solidification.json" || name == "scheil_conditions.json"
            });
        let has_csv = file_resp
            .files
            .iter()
            .any(|u| extract_filename(u).ends_with(".csv"));

        // 仅有 output.log 时说明后端计算失败但 status 仍为 completed
        let only_log = file_resp.files.len() <= 1
            && file_resp.files.iter().all(|u| extract_filename(u) == "output.log");
        if only_log {
            let log_hint = if !file_resp.files.is_empty() {
                format!(" 日志文件: {}", file_resp.files[0])
            } else {
                String::new()
            };
            let output = serde_json::json!({
                "error_code": "no_result_files",
                "task_id": args.task_id,
                "message": format!(
                    "任务已完成但未生成结果文件，可能是计算过程中出现错误。{}",
                    log_hint
                ),
                "retryable": true,
                "details": "请检查 components/composition/tdb_file 是否正确，或调整参数后重新提交"
            });
            return Err(CalphaMeshError::ValidationError(
                serde_json::to_string(&output).unwrap_or_default(),
            ));
        }

        if has_results_json {
            self.handle_point_result(&client, &file_resp.files, &files_map)
                .await
        } else if has_scheil_json {
            self.handle_scheil_result(&client, &file_resp.files, &files_map, &args.result_mode, args.task_id)
                .await
        } else if has_csv {
            self.handle_line_result(&client, &file_resp.files, &files_map, &args.result_mode, args.task_id)
                .await
        } else {
            // 有文件但类型未知，返回文件列表让上层感知
            let output = serde_json::json!({
                "error_code": "unknown_result_format",
                "task_id": args.task_id,
                "files": files_map,
                "message": format!(
                    "结果文件格式未识别，实际文件列表: {:?}",
                    file_resp.files.iter().map(|u| extract_filename(u)).collect::<Vec<_>>()
                ),
                "retryable": false
            });
            Err(CalphaMeshError::ValidationError(
                serde_json::to_string(&output).unwrap_or_default(),
            ))
        }
    }
}

impl GetTaskResult {
    async fn handle_point_result(
        &self,
        client: &CalphaMeshClient,
        file_urls: &[String],
        files_map: &serde_json::Value,
    ) -> Result<String, CalphaMeshError> {
        let results_url = file_urls
            .iter()
            .find(|u| extract_filename(u) == "results.json")
            .ok_or_else(|| {
                let names: Vec<String> = file_urls.iter().map(|u| extract_filename(u)).collect();
                CalphaMeshError::HttpError(format!(
                    "results.json not found in result files: {:?}",
                    names
                ))
            })?;

        let content = client.download_file_content(results_url).await?;
        let result_data: serde_json::Value =
            serde_json::from_str(&content).unwrap_or(serde_json::Value::String(content));

        let dominant_phase = result_data
            .get("phases")
            .and_then(|p| p.as_str())
            .unwrap_or("unknown");
        let phase_count = result_data
            .get("phase_fractions")
            .and_then(|pf| pf.as_object())
            .map(|obj| obj.len())
            .unwrap_or(0);

        let mut result_with_metrics = result_data.clone();
        if let Some(obj) = result_with_metrics.as_object_mut() {
            obj.insert(
                "derived_metrics".to_string(),
                json!({
                    "dominant_phase": dominant_phase,
                    "phase_count": phase_count
                }),
            );
        }

        let output = json!({
            "task_type": "point_calculation",
            "status": "completed",
            "result": result_with_metrics,
            "units": {
                "temperature": "K",
                "pressure": "Pa",
                "GM": "J/mol",
                "HM": "J/mol",
                "SM": "J/(mol·K)",
                "CPM": "J/(mol·K)",
                "chemical_potentials": "J/mol"
            },
            "files": files_map
        });
        Ok(serde_json::to_string(&output).unwrap_or_default())
    }

    async fn handle_scheil_result(
        &self,
        client: &CalphaMeshClient,
        file_urls: &[String],
        files_map: &serde_json::Value,
        result_mode: &str,
        task_id: i32,
    ) -> Result<String, CalphaMeshError> {
        // 后端实际输出 scheil_conditions.json，保留旧名兼容
        let scheil_url = file_urls
            .iter()
            .find(|u| {
                let name = extract_filename(u);
                name == "scheil_solidification.json" || name == "scheil_conditions.json"
            })
            .ok_or_else(|| {
                CalphaMeshError::HttpError(
                    "scheil result file (scheil_solidification.json / scheil_conditions.json) not found".to_string()
                )
            })?;

        let content = client.download_file_content(scheil_url).await?;
        let raw: serde_json::Value =
            serde_json::from_str(&content).unwrap_or(serde_json::Value::String(content));

        let metadata = raw.get("metadata").cloned().unwrap_or(json!({}));
        let curve = raw.get("solidification_curve").cloned().unwrap_or(json!({}));

        let converged = metadata
            .get("converged")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);
        let temp_range = metadata.get("temperature_range").cloned().unwrap_or(json!({}));
        let liquidus = temp_range
            .get("max")
            .and_then(|v| v.as_f64())
            .unwrap_or(0.0);
        let solidus = temp_range
            .get("min")
            .and_then(|v| v.as_f64())
            .unwrap_or(0.0);

        let temps: Vec<f64> = curve
            .get("temperatures")
            .and_then(|v| v.as_array())
            .map(|arr| arr.iter().filter_map(|x| x.as_f64()).collect())
            .unwrap_or_default();
        let liquid_fracs: Vec<f64> = curve
            .get("liquid_fractions")
            .and_then(|v| v.as_array())
            .map(|arr| arr.iter().filter_map(|x| x.as_f64()).collect())
            .unwrap_or_default();

        let total_steps = temps.len();

        let find_temp_at_fraction =
            |target: f64| -> f64 {
                for i in 0..liquid_fracs.len() {
                    if liquid_fracs[i] <= target {
                        return if i < temps.len() { temps[i] } else { 0.0 };
                    }
                }
                solidus
            };

        let key_points = json!([
            {"temperature_K": liquidus, "liquid_fraction": 1.0, "solid_fraction": 0.0},
            {"temperature_K": find_temp_at_fraction(0.5), "liquid_fraction": 0.5, "solid_fraction": 0.5},
            {"temperature_K": solidus, "liquid_fraction": 0.0, "solid_fraction": 1.0}
        ]);

        let liquid_monotonic = liquid_fracs.windows(2).all(|w| w[0] >= w[1]);

        let derived_metrics = json!({
            "freezing_range_K": liquidus - solidus,
            "t_at_liquid_fraction_0_9_K": find_temp_at_fraction(0.9),
            "t_at_liquid_fraction_0_5_K": find_temp_at_fraction(0.5),
            "t_at_liquid_fraction_0_1_K": find_temp_at_fraction(0.1),
            "curve_monotonic_check": {
                "liquid_fraction_non_increasing": liquid_monotonic
            }
        });

        let mut output = json!({
            "task_id": task_id,
            "task_type": "scheil_solidification",
            "status": "completed",
            "result": {
                "data_summary": {
                    "converged": converged,
                    "method": "scheil",
                    "total_steps": total_steps,
                    "temperature_range": {"liquidus_K": liquidus, "solidus_K": solidus},
                    "key_points": key_points
                },
                "derived_metrics": derived_metrics
            },
            "files": files_map
        });

        if result_mode == "full" {
            if let Some(result_obj) = output.get_mut("result").and_then(|r| r.as_object_mut()) {
                result_obj.insert("raw_data".to_string(), curve);
            }
        }

        Ok(serde_json::to_string(&output).unwrap_or_default())
    }

    async fn handle_line_result(
        &self,
        client: &CalphaMeshClient,
        file_urls: &[String],
        files_map: &serde_json::Value,
        result_mode: &str,
        task_id: i32,
    ) -> Result<String, CalphaMeshError> {
        let csv_url = file_urls
            .iter()
            .find(|u| {
                let name = extract_filename(u);
                name.ends_with(".csv")
            })
            .ok_or_else(|| {
                let names: Vec<String> = file_urls.iter().map(|u| extract_filename(u)).collect();
                CalphaMeshError::HttpError(format!(
                    "CSV file not found in result files: {:?}",
                    names
                ))
            })?;

        let content = client.download_file_content(csv_url).await?;
        let lines: Vec<&str> = content.lines().collect();

        if lines.is_empty() {
            let output = json!({
                "task_id": task_id,
                "task_type": "line_calculation",
                "status": "completed",
                "result": {"data_summary": {"total_rows": 0, "columns": []}},
                "files": files_map
            });
            return Ok(serde_json::to_string(&output).unwrap_or_default());
        }

        let headers: Vec<&str> = lines[0].split(',').collect();
        let data_lines = &lines[1..];
        let total_rows = data_lines.len();

        let mut rows: Vec<serde_json::Value> = Vec::new();
        let mut all_parsed: Vec<Vec<f64>> = Vec::new();
        let mut phases_set: Vec<String> = Vec::new();

        for line in data_lines {
            let cols: Vec<&str> = line.split(',').collect();
            let mut row_obj = serde_json::Map::new();
            let mut row_nums: Vec<f64> = Vec::new();

            for (i, col) in cols.iter().enumerate() {
                if i < headers.len() {
                    let header = headers[i].trim();
                    let trimmed = col.trim();
                    if let Ok(num) = trimmed.parse::<f64>() {
                        row_obj.insert(header.to_string(), json!(num));
                        row_nums.push(num);
                    } else {
                        row_obj.insert(header.to_string(), json!(trimmed));
                        row_nums.push(f64::NAN);
                        if header == "Phase" && !phases_set.contains(&trimmed.to_string()) {
                            phases_set.push(trimmed.to_string());
                        }
                    }
                }
            }
            rows.push(serde_json::Value::Object(row_obj));
            all_parsed.push(row_nums);
        }

        let t_col_idx = headers.iter().position(|h| h.trim() == "T/K");
        let temperatures: Vec<f64> = all_parsed
            .iter()
            .filter_map(|row| t_col_idx.and_then(|i| row.get(i)).copied())
            .filter(|v| !v.is_nan())
            .collect();

        let t_start = temperatures.first().copied().unwrap_or(0.0);
        let t_end = temperatures.last().copied().unwrap_or(0.0);

        let thermo_cols = ["GM/J/mol", "HM/J/mol", "SM/J/mol/K", "CPM/J/mol/K"];
        let mut property_extrema = serde_json::Map::new();

        for col_name in &thermo_cols {
            if let Some(col_idx) = headers.iter().position(|h| h.trim() == *col_name) {
                let mut min_val = f64::MAX;
                let mut max_val = f64::MIN;
                let mut min_t = 0.0_f64;
                let mut max_t = 0.0_f64;

                for (row_i, row) in all_parsed.iter().enumerate() {
                    if let Some(&val) = row.get(col_idx) {
                        if !val.is_nan() {
                            let t = t_col_idx
                                .and_then(|ti| row.get(ti))
                                .copied()
                                .unwrap_or(0.0);
                            if val < min_val {
                                min_val = val;
                                min_t = t;
                            }
                            if val > max_val {
                                max_val = val;
                                max_t = t;
                            }
                        }
                    }
                }

                if min_val < f64::MAX {
                    property_extrema.insert(
                        col_name.to_string(),
                        json!({
                            "min": {"value": min_val, "temperature_K": min_t},
                            "max": {"value": max_val, "temperature_K": max_t}
                        }),
                    );
                }
            }
        }

        let shown_rows: usize = if result_mode == "full" {
            total_rows
        } else {
            total_rows.min(20)
        };

        let display_rows: Vec<&serde_json::Value> = rows.iter().take(shown_rows).collect();

        let representative = json!({
            "first": rows.first(),
            "middle": rows.get(total_rows / 2),
            "last": rows.last()
        });

        let mut output = json!({
            "task_id": task_id,
            "task_type": "line_calculation",
            "status": "completed",
            "result": {
                "data_summary": {
                    "total_rows": total_rows,
                    "shown_rows": shown_rows,
                    "temperature_range": {"start": t_start, "end": t_end},
                    "columns": headers.iter().map(|h| h.trim()).collect::<Vec<&str>>(),
                    "rows": display_rows,
                    "representative_rows": representative
                },
                "derived_metrics": {
                    "phases_encountered": phases_set,
                    "property_extrema": property_extrema
                }
            },
            "files": files_map
        });

        if result_mode == "full" {
            if let Some(result_obj) = output.get_mut("result").and_then(|r| r.as_object_mut()) {
                result_obj.insert("raw_data".to_string(), json!(rows));
            }
        }

        Ok(serde_json::to_string(&output).unwrap_or_default())
    }
}

// ═══════════════════════════════════════════════════════════════════
// 工具实现：ListTasks
// ═══════════════════════════════════════════════════════════════════

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
            description: "分页查询当前用户的 CalphaMesh 历史任务列表。用于查找任务 ID、检查是否已有相同计算任务、了解历史计算记录。".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "page": {
                        "type": "integer",
                        "description": "页码，从 1 开始，默认 1。",
                        "minimum": 1
                    },
                    "items_per_page": {
                        "type": "integer",
                        "description": "每页任务数量，范围 1~100，默认 20。",
                        "minimum": 1,
                        "maximum": 100
                    }
                },
                "additionalProperties": false
            }),
        }
    }

    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        let api_key = get_api_key(&args.api_key, &self.api_key)?;
        let client = CalphaMeshClient::new(api_key);
        let list = client.list_tasks(args.page, args.items_per_page).await?;

        let tasks: Vec<serde_json::Value> = list
            .data
            .iter()
            .map(|t| {
                json!({
                    "task_id": t.id,
                    "status": t.status,
                    "task_type": t.task_type,
                    "title": t.title,
                    "created_at": t.created_at
                })
            })
            .collect();

        let output = json!({
            "page": list.page,
            "total_pages": list.total_pages,
            "items_per_page": list.items_per_page,
            "tasks": tasks
        });
        Ok(serde_json::to_string(&output).unwrap_or_default())
    }
}

// ═══════════════════════════════════════════════════════════════════
// 第二阶段工具（已有代码但尚未注册到 MCP）保持原样
// ═══════════════════════════════════════════════════════════════════

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
                        "description": "TDB 数据库文件名。"
                    }
                },
                "required": ["components", "start_composition", "end_composition", "start_temperature", "end_temperature", "tdb_file"]
            }),
        }
    }
    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        let api_key = get_api_key(&args.api_key, &self.api_key)?;
        let client = CalphaMeshClient::new(api_key);
        let resp = client.submit_binary_task(args).await?;
        Ok(serde_json::to_string(&json!({"task_id": resp.id, "status": resp.status}))
            .unwrap_or_default())
    }
}

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
                        "description": "TDB 数据库文件名。"
                   }
                },
                "required": ["components", "temperature", "composition_x", "composition_y", "composition_o", "tdb_file"]
            }),
        }
    }
    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        let api_key = get_api_key(&args.api_key, &self.api_key)?;
        let client = CalphaMeshClient::new(api_key);
        let resp = client.submit_ternary_task(args).await?;
        Ok(serde_json::to_string(&json!({"task_id": resp.id, "status": resp.status}))
            .unwrap_or_default())
    }
}

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
                   "temperature_range": {"type": "array", "items": {"type": "number"}, "minItems": 2, "maxItems": 2},
                   "tdb_file": {
                        "type": "string",
                        "enum": ["FE-C-SI-MN-CU-TI-O.TDB", "B-C-SI-ZR-HF-LA-Y-TI-O.TDB"],
                        "description": "TDB 数据库文件名。"
                   }
                },
                "required": ["components", "composition", "pressure", "temperature_range", "tdb_file"]
            }),
        }
    }
    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        let api_key = get_api_key(&args.api_key, &self.api_key)?;
        let client = CalphaMeshClient::new(api_key);
        let resp = client.submit_boiling_point_task(args).await?;
        Ok(serde_json::to_string(&json!({"task_id": resp.id, "status": resp.status}))
            .unwrap_or_default())
    }
}

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
                        "description": "TDB 数据库文件名。"
                   }
                },
                "required": ["components", "composition", "temperature_start", "temperature_end", "increments", "pressure_start", "pressure_end", "pressure_increments", "properties", "tdb_file"]
            }),
        }
    }
    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        let api_key = get_api_key(&args.api_key, &self.api_key)?;
        let client = CalphaMeshClient::new(api_key);
        let resp = client.submit_thermo_properties_task(args).await?;
        Ok(serde_json::to_string(&json!({"task_id": resp.id, "status": resp.status}))
            .unwrap_or_default())
    }
}
