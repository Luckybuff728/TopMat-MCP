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
    // AL 基合金 TDB：Al-Si-Mg-Fe-Mn 压铸铝合金体系（含 Fe/Mn 杂质），已在 topthermo-next 后端注册
    (
        "Al-Si-Mg-Fe-Mn_by_wf.TDB",
        &["AL", "SI", "MG", "FE", "MN"],
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
// Al 数据库各任务类型的推荐激活相列表（以 http_api_payload参考.md 已验证运行为准）
// ═══════════════════════════════════════════════════════════════════

/// Al-Si-Mg-Fe-Mn 5 元体系推荐相集合（适用于 point/line/scheil/thermo 任务）
const AL_5ELEMENT_PHASES: &[&str] = &[
    "LIQUID", "FCC_A1", "DIAMOND_A4", "HCP_A3", "BCC_A2", "CBCC_A12",
    "BETA_ALMG", "EPSILON_ALMG", "GAMMA_ALMG", "MG2SI",
    "AL5FE2", "AL13FE4", "ALPHA_ALFESI", "BETA_ALFESI", "ALPHA_ALFEMNSI", "AL4_FEMN",
];

/// Al-Mg-Si 3 元体系推荐相集合（适用于 ternary 任务）
const AL_TERNARY_PHASES: &[&str] = &[
    "LIQUID", "FCC_A1", "DIAMOND_A4", "HCP_A3",
    "BETA_ALMG", "EPSILON_ALMG", "GAMMA_ALMG", "MG2SI",
];

/// Al-Si 2 元体系推荐相集合（适用于 binary 任务）
const AL_BINARY_PHASES: &[&str] = &[
    "LIQUID", "FCC_A1", "DIAMOND_A4",
];

/// 多元 Al 任务默认相列表（5 元配方）
fn tdb_default_phases(tdb_file: &str) -> Vec<&'static str> {
    match tdb_file {
        "Al-Si-Mg-Fe-Mn_by_wf.TDB" => AL_5ELEMENT_PHASES.to_vec(),
        // Fe 基和 B 基 TDB 使用全部可用相（["*"] 在这些库中有效）
        _ => vec!["*"],
    }
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

/// 归一化成分 HashMap：若误差在合理范围内（< 2%）则自动除以总和，
/// 使提交给 TopThermo 的数值严格等于 1.0。
/// 误差超过 2% 视为参数严重错误，返回 Err。
fn normalize_composition(composition: &HashMap<String, f64>) -> Result<HashMap<String, f64>, CalphaMeshError> {
    let sum: f64 = composition.values().sum();
    const HARD_LIMIT: f64 = 2e-2;
    if (sum - 1.0).abs() > HARD_LIMIT {
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
    Ok(composition
        .iter()
        .map(|(k, v)| (k.clone(), v / sum))
        .collect())
}

fn validate_composition_sum(composition: &HashMap<String, f64>) -> Result<(), CalphaMeshError> {
    normalize_composition(composition).map(|_| ())
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
    if params.start_temperature >= params.end_temperature {
        return Err(CalphaMeshError::ValidationError(format!(
            "start_temperature ({} K) 必须小于 end_temperature ({} K)，Line 扫描方向为升温（低温→高温）",
            params.start_temperature, params.end_temperature
        )));
    }
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
        let phases = tdb_default_phases(&params.tdb_file);
        let composition = normalize_composition(&params.composition)?;
        let inner = json!({
            "task_type": "point_calculation",
            "tdb_file": format!("/app/exe/topthermo-next/database/{}", params.tdb_file),
            "task_name": format!("point_{}", chrono::Utc::now().timestamp()),
            "task_path": Self::generate_task_path("point"),
            "condition": {
                "components": params.components,
                "activated_phases": phases,
                "temperature": params.temperature,
                "compositions": composition
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
        let phases = tdb_default_phases(&params.tdb_file);
        let start_composition = normalize_composition(&params.start_composition)?;
        let end_composition = normalize_composition(&params.end_composition)?;
        let inner = json!({
            "task_type": "line_calculation",
            "tdb_file": format!("/app/exe/topthermo-next/database/{}", params.tdb_file),
            "task_name": format!("line_{}", chrono::Utc::now().timestamp()),
            "task_path": Self::generate_task_path("line"),
            "condition": {
                "components": params.components,
                "compositions_start": start_composition,
                "compositions_end": end_composition,
                "temperature_start": params.start_temperature,
                "temperature_end": params.end_temperature,
                "increments": params.steps,
                "activated_phases": phases
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
        let phases = tdb_default_phases(&params.tdb_file);
        let composition = normalize_composition(&params.composition)?;
        let inner = json!({
            "task_type": "scheil_solidification",
            "tdb_file": format!("/app/exe/topthermo-next/database/{}", params.tdb_file),
            "task_name": format!("scheil_{}", chrono::Utc::now().timestamp()),
            "task_path": Self::generate_task_path("scheil"),
            "condition": {
                "components": params.components,
                "compositions": composition,
                "start_temperature": params.start_temperature,
                "temperature_step": params.temperature_step,
                "activated_phases": phases,
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
        // 二元相图：Binary 任务需要顶层 activated_elements 字段（API payload 参考要求）
        // Al-Si 二元选用 AL_BINARY_PHASES，其他 TDB 使用 ["*"]
        let phases: Vec<&str> = match params.tdb_file.as_str() {
            "Al-Si-Mg-Fe-Mn_by_wf.TDB" => AL_BINARY_PHASES.to_vec(),
            _ => vec!["*"],
        };
        let start_composition = normalize_composition(&params.start_composition)?;
        let end_composition = normalize_composition(&params.end_composition)?;
        let inner = json!({
            "task_type": "binary_equilibrium",
            "tdb_file": format!("/app/exe/topthermo-next/database/{}", params.tdb_file),
            "task_name": format!("binary_{}", chrono::Utc::now().timestamp()),
            "task_path": Self::generate_task_path("binary"),
            "activated_elements": params.components,
            "condition": {
                "components": params.components,
                "activated_phases": phases,
                "compositions_start": start_composition,
                "compositions_end": end_composition,
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
        // 三元等温截面：Al-Mg-Si 用 AL_TERNARY_PHASES（8相），其他 TDB 用 ["*"]
        let phases: Vec<&str> = match params.tdb_file.as_str() {
            "Al-Si-Mg-Fe-Mn_by_wf.TDB" => AL_TERNARY_PHASES.to_vec(),
            _ => vec!["*"],
        };
        let composition_y = normalize_composition(&params.composition_y)?;
        let composition_x = normalize_composition(&params.composition_x)?;
        let composition_o = normalize_composition(&params.composition_o)?;
        let inner = json!({
            "task_type": "ternary_calculation",
            "tdb_file": format!("/app/exe/topthermo-next/database/{}", params.tdb_file),
            "task_name": format!("ternary_{}", chrono::Utc::now().timestamp()),
            "task_path": Self::generate_task_path("ternary"),
            "condition": {
                "components": params.components,
                "activated_phases": phases,
                "temperature": params.temperature,
                "compositions_y": composition_y,
                "compositions_x": composition_x,
                "compositions_o": composition_o
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
        let composition = normalize_composition(&params.composition)?;
        let inner = json!({
            "task_type": "boiling_point",
            "tdb_file": format!("/app/exe/topthermo-next/database/{}", params.tdb_file),
            "task_name": format!("boiling_{}", chrono::Utc::now().timestamp()),
            "task_path": Self::generate_task_path("boiling"),
            "condition": {
                "components": params.components,
                "pressure": params.pressure,
                "compositions": composition,
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
        // 热力学性质任务使用与 point/line 相同的 5 元相集合
        let phases = tdb_default_phases(&params.tdb_file);
        let composition = normalize_composition(&params.composition)?;
        let inner = json!({
            "task_type": "thermodynamic_properties",
            "tdb_file": format!("/app/exe/topthermo-next/database/{}", params.tdb_file),
            "task_name": format!("properties_{}", chrono::Utc::now().timestamp()),
            "task_path": Self::generate_task_path("properties"),
            "condition": {
                "components": params.components,
                "activated_phases": phases,
                "compositions_start": composition,
                "compositions_end": composition,
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
                        "enum": ["FE-C-SI-MN-CU-TI-O.TDB", "B-C-SI-ZR-HF-LA-Y-TI-O.TDB", "Al-Si-Mg-Fe-Mn_by_wf.TDB"],
                        "description": "热力学数据库文件名。必须包含 components 中所有元素。Al-Si-Mg 压铸铝合金（AL/SI/MG/FE/MN）选 Al-Si-Mg-Fe-Mn_by_wf.TDB；FE 基合金选 FE-C-SI-MN-CU-TI-O.TDB；硼化物/硅化物选 B-C-SI-ZR-HF-LA-Y-TI-O.TDB。"
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
                        "enum": ["FE-C-SI-MN-CU-TI-O.TDB", "B-C-SI-ZR-HF-LA-Y-TI-O.TDB", "Al-Si-Mg-Fe-Mn_by_wf.TDB"],
                        "description": "热力学数据库文件名。必须包含 components 中所有元素。Al-Si-Mg 压铸铝合金（AL/SI/MG/FE/MN）选 Al-Si-Mg-Fe-Mn_by_wf.TDB；FE 基合金选 FE-C-SI-MN-CU-TI-O.TDB；硼化物/硅化物选 B-C-SI-ZR-HF-LA-Y-TI-O.TDB。"
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
                        "enum": ["FE-C-SI-MN-CU-TI-O.TDB", "B-C-SI-ZR-HF-LA-Y-TI-O.TDB", "Al-Si-Mg-Fe-Mn_by_wf.TDB"],
                        "description": "热力学数据库文件名。必须包含 components 中所有元素。Al-Si-Mg 压铸铝合金（AL/SI/MG/FE/MN）选 Al-Si-Mg-Fe-Mn_by_wf.TDB；FE 基合金选 FE-C-SI-MN-CU-TI-O.TDB；硼化物/硅化物选 B-C-SI-ZR-HF-LA-Y-TI-O.TDB。"
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

        // 偶发原因：后端将任务标为 completed 后，结果文件写入/索引可能存在延迟，
        // 首次 get_result_files 可能返回 []，短时等待后重试可恢复。最多重试 2 次，每次间隔 5 秒。
        const RESULT_FILES_RETRY_DELAY_SECS: u64 = 5;
        const RESULT_FILES_MAX_RETRIES: u32 = 2;

        let mut file_resp = client.get_result_files(task.id).await?;
        for attempt in 0..=RESULT_FILES_MAX_RETRIES {
            let file_names: Vec<String> = file_resp.files.iter().map(|u| extract_filename(u)).collect();
            let has_results_json = file_names.iter().any(|n| n == "results.json");
            let has_scheil_json = file_names.iter().any(|n| n == "scheil_solidification.json");
            let has_scheil_csv = file_names.iter().any(|n| n == "scheil_solidification.csv");
            let has_binary_json = file_names.iter().any(|n| n == "binary_equilibrium.json");
            let has_ternary_json = file_names.iter().any(|n| n == "ternary_plotly.json");
            let has_thermo_json = file_names.iter().any(|n| n == "thermodynamic_properties.json");
            let has_thermo_csv = file_names.iter().any(|n| n == "thermodynamic_properties.csv");
            let has_boiling_csv = file_names.iter().any(|n| n == "boiling_melting_point.csv");
            let has_line_csv = file_names.iter().any(|n| {
                n.ends_with(".csv")
                    && n != "scheil_solidification.csv"
                    && n != "boiling_melting_point.csv"
            });
            let has_actual_result = has_results_json || has_scheil_json || has_scheil_csv
                || has_binary_json || has_ternary_json
                || has_thermo_json || has_thermo_csv
                || has_boiling_csv || has_line_csv;
            if has_actual_result {
                break;
            }
            if attempt < RESULT_FILES_MAX_RETRIES {
                tokio::time::sleep(std::time::Duration::from_secs(RESULT_FILES_RETRY_DELAY_SECS)).await;
                file_resp = client.get_result_files(task.id).await?;
            }
        }

        let files_map = build_files_map(&file_resp.files);
        let file_names: Vec<String> = file_resp.files.iter().map(|u| extract_filename(u)).collect();

        // ── 结果文件检测 ──────────────────────────────────────────────
        // point_calculation → results.json
        let has_results_json = file_names.iter().any(|n| n == "results.json");
        // scheil（旧版 JSON 格式，legacy）
        let has_scheil_json = file_names.iter().any(|n| n == "scheil_solidification.json");
        // scheil（新版 CSV 格式，当前后端输出）
        let has_scheil_csv = file_names.iter().any(|n| n == "scheil_solidification.csv");
        // binary_equilibrium → binary_equilibrium.json
        let has_binary_json = file_names.iter().any(|n| n == "binary_equilibrium.json");
        // ternary_calculation → ternary_plotly.json
        let has_ternary_json = file_names.iter().any(|n| n == "ternary_plotly.json");
        // thermodynamic_properties → thermodynamic_properties.json OR thermodynamic_properties.csv
        let has_thermo_json = file_names.iter().any(|n| n == "thermodynamic_properties.json");
        let has_thermo_csv = file_names.iter().any(|n| n == "thermodynamic_properties.csv");
        // boiling_point → boiling_melting_point.csv
        let has_boiling_csv = file_names.iter().any(|n| n == "boiling_melting_point.csv");
        // line_calculation → 任意非 Scheil/Boiling 的 CSV
        let has_line_csv = file_names.iter().any(|n| {
            n.ends_with(".csv")
                && n != "scheil_solidification.csv"
                && n != "boiling_melting_point.csv"
        });
        // scheil_conditions.json = 仅是输入条件回显，不代表计算成功，不计入

        // 无实际结果文件：仅有 output.log 或仅有条件回显文件（含重试后仍为空）
        let has_actual_result = has_results_json || has_scheil_json || has_scheil_csv
            || has_binary_json || has_ternary_json
            || has_thermo_json || has_thermo_csv
            || has_boiling_csv || has_line_csv;
        if !has_actual_result {
            let log_url = file_resp
                .files
                .iter()
                .find(|u| extract_filename(u) == "output.log")
                .cloned()
                .unwrap_or_default();
            let log_hint = if !log_url.is_empty() {
                format!(" 日志文件: {}", log_url)
            } else {
                String::new()
            };
            let all_files: Vec<String> = file_resp.files.iter().map(|u| extract_filename(u)).collect();
            let output = serde_json::json!({
                "error_code": "no_result_files",
                "task_id": args.task_id,
                "message": format!(
                    "任务已完成但未生成有效结果文件（实际文件：{:?}），计算过程中可能出现错误。{}",
                    all_files, log_hint
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
        } else if has_scheil_csv {
            // Scheil CSV 优先（当前后端成功计算时输出的主要格式，字段更可靠）
            self.handle_scheil_csv_result(&client, &file_resp.files, &files_map, &args.result_mode, args.task_id)
                .await
        } else if has_scheil_json {
            // Scheil JSON（旧版 legacy 格式，仅当没有 CSV 时才使用）
            self.handle_scheil_result(&client, &file_resp.files, &files_map, &args.result_mode, args.task_id)
                .await
        } else if has_binary_json {
            self.handle_binary_result(&client, &file_resp.files, &files_map, args.task_id)
                .await
        } else if has_ternary_json {
            self.handle_ternary_result(&client, &file_resp.files, &files_map, args.task_id)
                .await
        } else if has_thermo_csv {
            // 热力学性质 CSV（当前后端主要输出格式）
            self.handle_thermo_csv_result(&client, &file_resp.files, &files_map, &args.result_mode, args.task_id)
                .await
        } else if has_thermo_json {
            self.handle_thermo_result(&client, &file_resp.files, &files_map, &args.result_mode, args.task_id)
                .await
        } else if has_boiling_csv {
            self.handle_boiling_result(&client, &file_resp.files, &files_map, args.task_id)
                .await
        } else if has_line_csv {
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

    /// 解析后端新格式 Scheil 结果：scheil_solidification.csv
    /// CSV 格式：第一列为温度（K），包含 f(LIQUID) 列，其余列为各固相分数
    async fn handle_scheil_csv_result(
        &self,
        client: &CalphaMeshClient,
        file_urls: &[String],
        files_map: &serde_json::Value,
        result_mode: &str,
        task_id: i32,
    ) -> Result<String, CalphaMeshError> {
        let csv_url = file_urls
            .iter()
            .find(|u| extract_filename(u) == "scheil_solidification.csv")
            .ok_or_else(|| {
                CalphaMeshError::HttpError("scheil_solidification.csv not found".to_string())
            })?;

        let content = client.download_file_content(csv_url).await?;
        let lines: Vec<&str> = content.lines().filter(|l| !l.trim().is_empty()).collect();

        if lines.len() < 2 {
            let output = json!({
                "task_id": task_id,
                "task_type": "scheil_solidification",
                "status": "completed",
                "result": {"data_summary": {"total_steps": 0}, "derived_metrics": {}},
                "files": files_map
            });
            return Ok(serde_json::to_string(&output).unwrap_or_default());
        }

        let headers: Vec<&str> = lines[0].split(',').map(|s| s.trim()).collect();

        // 找温度列和液相分数列的索引
        let temp_idx = headers.iter().position(|h| {
            let h_up = h.to_uppercase();
            h_up.contains("T/K") || h_up == "T" || h_up == "TEMP" || h_up == "TEMPERATURE"
        }).unwrap_or(0);

        let liquid_idx = headers.iter().position(|h| {
            let h_up = h.to_uppercase();
            h_up.contains("F(LIQUID)") || h_up.contains("LIQUID") || *h == "f(LIQUID)"
        });

        let mut temps: Vec<f64> = Vec::new();
        let mut liquid_fracs: Vec<f64> = Vec::new();
        let mut all_rows: Vec<serde_json::Value> = Vec::new();

        for line in &lines[1..] {
            let cols: Vec<&str> = line.split(',').map(|s| s.trim()).collect();
            if let Some(t) = cols.get(temp_idx).and_then(|s| s.parse::<f64>().ok()) {
                temps.push(t);
                if let Some(liq_idx) = liquid_idx {
                    let lf = cols.get(liq_idx).and_then(|s| s.parse::<f64>().ok()).unwrap_or(0.0);
                    liquid_fracs.push(lf);
                }
                // 构建行对象
                let mut row = serde_json::Map::new();
                for (i, h) in headers.iter().enumerate() {
                    if let Some(val) = cols.get(i).and_then(|s| s.parse::<f64>().ok()) {
                        row.insert(h.to_string(), json!(val));
                    }
                }
                all_rows.push(json!(row));
            }
        }

        let total_steps = temps.len();
        // 液相线 = 最高温（f(LIQUID)=1），固相线 = 最低温（f(LIQUID)=0）
        let liquidus = temps.first().copied().unwrap_or(0.0);
        let solidus = temps.last().copied().unwrap_or(0.0);

        let find_temp_at_fraction = |target: f64| -> f64 {
            if liquid_fracs.is_empty() { return 0.0; }
            for i in 0..liquid_fracs.len() {
                if liquid_fracs[i] <= target {
                    return *temps.get(i).unwrap_or(&solidus);
                }
            }
            solidus
        };

        let liquid_monotonic = liquid_fracs.windows(2).all(|w| w[0] >= w[1]);

        // 均匀采样：从全程57步中取最多20个均匀分布的数据点，覆盖完整凝固区间
        // （不能只取前20行，否则只覆盖高温液态区，看不到实际凝固曲线）
        let sample_size: usize = 20.min(total_steps);
        let shown_rows: Vec<_> = if total_steps <= sample_size {
            all_rows.clone()
        } else {
            (0..sample_size)
                .map(|i| {
                    // 线性插值索引，确保第一行（liquidus）和最后一行（solidus）都包含
                    i * (total_steps - 1) / (sample_size - 1)
                })
                .filter_map(|idx| all_rows.get(idx))
                .cloned()
                .collect()
        };

        let derived_metrics = json!({
            "freezing_range_K": liquidus - solidus,
            "t_at_liquid_fraction_0_9_K": find_temp_at_fraction(0.9),
            "t_at_liquid_fraction_0_5_K": find_temp_at_fraction(0.5),
            "t_at_liquid_fraction_0_1_K": find_temp_at_fraction(0.1),
            "curve_monotonic_check": {"liquid_fraction_non_increasing": liquid_monotonic}
        });

        let key_points = json!([
            {"temperature_K": liquidus, "liquid_fraction": liquid_fracs.first().copied().unwrap_or(1.0), "solid_fraction": 1.0 - liquid_fracs.first().copied().unwrap_or(1.0)},
            {"temperature_K": find_temp_at_fraction(0.5), "liquid_fraction": 0.5, "solid_fraction": 0.5},
            {"temperature_K": solidus, "liquid_fraction": liquid_fracs.last().copied().unwrap_or(0.0), "solid_fraction": 1.0 - liquid_fracs.last().copied().unwrap_or(0.0)}
        ]);

        let mut output = json!({
            "task_id": task_id,
            "task_type": "scheil_solidification",
            "status": "completed",
            "result": {
                "data_summary": {
                    "converged": true,
                    "method": "scheil",
                    "total_steps": total_steps,
                    "temperature_range": {"liquidus_K": liquidus, "solidus_K": solidus},
                    "key_points": key_points,
                    "columns": headers,
                    "shown_rows": shown_rows
                },
                "derived_metrics": derived_metrics
            },
            "files": files_map
        });

        if result_mode == "full" {
            if let Some(result_obj) = output.get_mut("result").and_then(|r| r.as_object_mut()) {
                result_obj.insert("raw_data".to_string(), json!(all_rows));
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
                name.ends_with(".csv") && name != "scheil_solidification.csv"
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

    // ── binary_equilibrium 结果处理 ──────────────────────────────────
    async fn handle_binary_result(
        &self,
        client: &CalphaMeshClient,
        file_urls: &[String],
        files_map: &serde_json::Value,
        task_id: i32,
    ) -> Result<String, CalphaMeshError> {
        let url = file_urls.iter()
            .find(|u| extract_filename(u) == "binary_equilibrium.json")
            .ok_or_else(|| CalphaMeshError::HttpError("binary_equilibrium.json not found".to_string()))?;
        let content = client.download_file_content(url).await?;
        let raw: serde_json::Value = serde_json::from_str(&content)
            .unwrap_or(serde_json::Value::String(content));

        // 后端实际返回结构：{ summary: { boundary_count, phase_count }, ... }
        // 用 summary 字段提取摘要，同时保留原始数据供高级用户
        let summary = raw.get("summary").cloned().unwrap_or(json!({}));
        let boundary_count = summary.get("boundary_count").and_then(|v| v.as_i64()).unwrap_or(0);
        let phase_count = summary.get("phase_count").and_then(|v| v.as_i64()).unwrap_or(0);
        let title = raw.get("title").and_then(|v| v.as_str()).unwrap_or("Al-Si").to_string();

        let output = json!({
            "task_id": task_id,
            "task_type": "binary_equilibrium",
            "status": "completed",
            "result": {
                "data_summary": {
                    "system": title,
                    "phase_count": phase_count,
                    "boundary_count": boundary_count,
                    "note": "二元相图已计算完成，完整图形数据见 files.binary_equilibrium.json"
                }
            },
            "files": files_map
        });
        Ok(serde_json::to_string(&output).unwrap_or_default())
    }

    // ── thermodynamic_properties CSV 结果处理 ────────────────────────
    async fn handle_thermo_csv_result(
        &self,
        client: &CalphaMeshClient,
        file_urls: &[String],
        files_map: &serde_json::Value,
        result_mode: &str,
        task_id: i32,
    ) -> Result<String, CalphaMeshError> {
        let url = file_urls.iter()
            .find(|u| extract_filename(u) == "thermodynamic_properties.csv")
            .ok_or_else(|| CalphaMeshError::HttpError("thermodynamic_properties.csv not found".to_string()))?;
        let content = client.download_file_content(url).await?;
        let lines: Vec<&str> = content.lines().filter(|l| !l.trim().is_empty()).collect();

        if lines.len() < 2 {
            let output = json!({
                "task_id": task_id,
                "task_type": "thermodynamic_properties",
                "status": "completed",
                "result": {"data_summary": {"total_rows": 0}},
                "files": files_map
            });
            return Ok(serde_json::to_string(&output).unwrap_or_default());
        }

        let headers: Vec<&str> = lines[0].split(',').map(|s| s.trim()).collect();
        let data_lines = &lines[1..];
        let total_rows = data_lines.len();

        let mut rows: Vec<serde_json::Value> = Vec::new();
        let mut all_nums: Vec<Vec<f64>> = Vec::new();

        for line in data_lines {
            let cols: Vec<&str> = line.split(',').map(|s| s.trim()).collect();
            let mut row = serde_json::Map::new();
            let mut nums: Vec<f64> = Vec::new();
            for (i, h) in headers.iter().enumerate() {
                if let Some(v) = cols.get(i) {
                    if let Ok(num) = v.parse::<f64>() {
                        row.insert(h.to_string(), json!(num));
                        nums.push(num);
                    } else {
                        row.insert(h.to_string(), json!(v));
                        nums.push(f64::NAN);
                    }
                }
            }
            rows.push(json!(row));
            all_nums.push(nums);
        }

        let t_col = headers.iter().position(|h| h.trim().to_uppercase().contains("T/K") || h.trim().to_uppercase() == "T");
        let t_start = t_col.and_then(|i| all_nums.first().and_then(|r| r.get(i)).copied()).unwrap_or(0.0);
        let t_end = t_col.and_then(|i| all_nums.last().and_then(|r| r.get(i)).copied()).unwrap_or(0.0);

        // 提取 GM/HM/SM/CPM 极值
        // 注意：实际 CSV 可能为 "GM(FCC_A1)/J/mol" 等包含相名的列，需用模糊匹配
        // 对每个性质前缀找到所有匹配列，合并后取全局 min/max
        let thermo_prefixes = [
            ("GM", "GM/J/mol"),
            ("HM", "HM/J/mol"),
            ("SM", "SM/J/mol/K"),
            ("CPM", "CPM/J/mol/K"),
        ];
        let mut extrema = serde_json::Map::new();
        for (prefix, canonical_key) in &thermo_prefixes {
            // 匹配所有包含该前缀的热力学列（如 GM/J/mol、GM(FCC_A1)/J/mol 均匹配）
            let matching_cols: Vec<usize> = headers.iter()
                .enumerate()
                .filter(|(_, h)| {
                    let ht = h.trim().to_uppercase();
                    // 以前缀开头，后跟 "/" 或 "(" —— 防止 CPM 匹配 CBCC、HM 匹配 HCP 等
                    ht.starts_with(prefix) && (ht.as_bytes().get(prefix.len()).map_or(true, |&b| b == b'/' || b == b'('))
                })
                .map(|(i, _)| i)
                .collect();

            if matching_cols.is_empty() { continue; }

            let mut global_min = f64::MAX;
            let mut global_max = f64::MIN;
            for &ci in &matching_cols {
                for row in &all_nums {
                    if let Some(&val) = row.get(ci) {
                        if !val.is_nan() {
                            if val < global_min { global_min = val; }
                            if val > global_max { global_max = val; }
                        }
                    }
                }
            }
            if global_min < f64::MAX {
                extrema.insert(canonical_key.to_string(), json!({"min": global_min, "max": global_max}));
            }
        }

        let shown_rows = if result_mode == "full" { total_rows } else { total_rows.min(20) };
        let display: Vec<_> = rows.iter().take(shown_rows).collect();

        let mut output = json!({
            "task_id": task_id,
            "task_type": "thermodynamic_properties",
            "status": "completed",
            "result": {
                "data_summary": {
                    "total_rows": total_rows,
                    "shown_rows": shown_rows,
                    "temperature_range": {"start_K": t_start, "end_K": t_end},
                    "columns": headers,
                    "rows": display
                },
                "derived_metrics": {"property_extrema": extrema}
            },
            "units": {"GM":"J/mol","HM":"J/mol","SM":"J/(mol·K)","CPM":"J/(mol·K)"},
            "files": files_map
        });
        if result_mode == "full" {
            if let Some(ro) = output.get_mut("result").and_then(|r| r.as_object_mut()) {
                ro.insert("raw_data".to_string(), json!(rows));
            }
        }
        Ok(serde_json::to_string(&output).unwrap_or_default())
    }

    // ── ternary_calculation 结果处理 ─────────────────────────────────
    async fn handle_ternary_result(
        &self,
        client: &CalphaMeshClient,
        file_urls: &[String],
        files_map: &serde_json::Value,
        task_id: i32,
    ) -> Result<String, CalphaMeshError> {
        let url = file_urls.iter()
            .find(|u| extract_filename(u) == "ternary_plotly.json")
            .ok_or_else(|| CalphaMeshError::HttpError("ternary_plotly.json not found".to_string()))?;
        let content = client.download_file_content(url).await?;
        let raw: serde_json::Value = serde_json::from_str(&content)
            .unwrap_or(serde_json::Value::String(content));

        // 从 Plotly JSON 统计关键数量
        // Plotly ternary JSON 结构可能是 {tie_triangles:[...], tie_lines:[...], data:[...], summary:{...}}
        // 兼容多种字段名
        // 统计三元相图计算网格点数
        // 优先查找顶层命名字段，兜底从 Plotly traces 中统计 marker 点数
        let point_count = ["points", "composition_points", "node_points", "grid_points", "phase_points"]
            .iter()
            .find_map(|&f| raw.get(f).and_then(|v| v.as_array()).map(|a| a.len()))
            .or_else(|| {
                // 从 Plotly data 数组汇总所有 marker/scatter trace 的坐标数量
                // marker-type trace → 计算格点；line-only trace → 相界/共轭线
                raw.get("data")
                    .and_then(|v| v.as_array())
                    .map(|arr| {
                        arr.iter()
                            .filter(|trace| {
                                let mode = trace.get("mode")
                                    .and_then(|m| m.as_str())
                                    .unwrap_or("");
                                let has_marker = trace.get("marker").is_some();
                                // 包含 markers 或有 marker 配置，且不是纯 line
                                (mode.contains("markers") || has_marker)
                                    && !mode.eq("lines")
                            })
                            .map(|trace| {
                                // 支持标准 {x,y} 和 Plotly ternary {a,b,c}
                                trace.get("x").or_else(|| trace.get("a"))
                                    .and_then(|v| v.as_array())
                                    .map(|a| a.len())
                                    .unwrap_or(0)
                            })
                            .sum::<usize>()
                    })
                    .filter(|&n| n > 0)
            })
            .unwrap_or(0);

        let tie_lines = ["tie_lines", "tielines", "two_phase_lines"]
            .iter()
            .find_map(|&f| raw.get(f).and_then(|v| v.as_array()).map(|a| a.len()))
            .unwrap_or(0);

        let tie_triangles = ["tie_triangles", "three_phase_triangles", "tiangles"]
            .iter()
            .find_map(|&f| raw.get(f).and_then(|v| v.as_array()).map(|a| a.len()))
            .unwrap_or(0);

        let phases = ["phases", "phase_list", "phase_names"]
            .iter()
            .find_map(|&f| {
                raw.get(f).and_then(|v| v.as_array())
                    .map(|a| a.iter().filter_map(|x| x.as_str()).collect::<Vec<_>>())
            })
            .unwrap_or_default();

        let output = json!({
            "task_id": task_id,
            "task_type": "ternary_calculation",
            "status": "completed",
            "result": {
                "data_summary": {
                    "point_count": point_count,
                    "tie_line_count": tie_lines,
                    "tie_triangle_count": tie_triangles,
                    "phases_in_diagram": phases
                }
            },
            "files": files_map
        });
        Ok(serde_json::to_string(&output).unwrap_or_default())
    }

    // ── thermodynamic_properties 结果处理 ────────────────────────────
    async fn handle_thermo_result(
        &self,
        client: &CalphaMeshClient,
        file_urls: &[String],
        files_map: &serde_json::Value,
        result_mode: &str,
        task_id: i32,
    ) -> Result<String, CalphaMeshError> {
        let url = file_urls.iter()
            .find(|u| extract_filename(u) == "thermodynamic_properties.json")
            .ok_or_else(|| CalphaMeshError::HttpError("thermodynamic_properties.json not found".to_string()))?;
        let content = client.download_file_content(url).await?;
        let raw: serde_json::Value = serde_json::from_str(&content)
            .unwrap_or(serde_json::Value::String(content));

        // 从 JSON 中提取摘要：温度点数量、包含的性质
        let data_points = raw.get("data").and_then(|v| v.as_array()).map(|a| a.len()).unwrap_or(0);
        let properties = raw.get("properties").cloned().unwrap_or(json!(["GM","HM","SM","CPM"]));
        let temperature_range = raw.get("temperature_range").cloned().unwrap_or(json!({}));

        let mut output = json!({
            "task_id": task_id,
            "task_type": "thermodynamic_properties",
            "status": "completed",
            "result": {
                "data_summary": {
                    "data_point_count": data_points,
                    "properties": properties,
                    "temperature_range": temperature_range
                }
            },
            "units": {
                "GM": "J/mol",
                "HM": "J/mol",
                "SM": "J/(mol·K)",
                "CPM": "J/(mol·K)"
            },
            "files": files_map
        });
        if result_mode == "full" {
            if let Some(result_obj) = output.get_mut("result").and_then(|r| r.as_object_mut()) {
                result_obj.insert("raw_data".to_string(), raw);
            }
        }
        Ok(serde_json::to_string(&output).unwrap_or_default())
    }

    // ── boiling_point 结果处理 ────────────────────────────────────────
    async fn handle_boiling_result(
        &self,
        client: &CalphaMeshClient,
        file_urls: &[String],
        files_map: &serde_json::Value,
        task_id: i32,
    ) -> Result<String, CalphaMeshError> {
        let url = file_urls.iter()
            .find(|u| extract_filename(u) == "boiling_melting_point.csv")
            .ok_or_else(|| CalphaMeshError::HttpError("boiling_melting_point.csv not found".to_string()))?;
        let content = client.download_file_content(url).await?;
        let lines: Vec<&str> = content.lines().filter(|l| !l.trim().is_empty()).collect();

        if lines.len() < 2 {
            let output = json!({
                "task_id": task_id,
                "task_type": "boiling_point",
                "status": "completed",
                "result": {"data_summary": {"note": "计算完成但无法解析结果，请查看原始文件"}},
                "files": files_map
            });
            return Ok(serde_json::to_string(&output).unwrap_or_default());
        }

        let headers: Vec<&str> = lines[0].split(',').map(|s| s.trim()).collect();
        let mut rows: Vec<serde_json::Value> = Vec::new();
        let mut derived = serde_json::Map::new();

        for line in &lines[1..] {
            let cols: Vec<&str> = line.split(',').map(|s| s.trim()).collect();
            let mut row = serde_json::Map::new();
            for (i, h) in headers.iter().enumerate() {
                if let Some(v) = cols.get(i) {
                    if let Ok(num) = v.parse::<f64>() {
                        row.insert(h.to_string(), json!(num));
                        // 提取关键熔点/沸点信息
                        let h_up = h.to_uppercase();
                        if h_up.contains("SOLIDUS") || h_up.contains("MELTING") || h_up.contains("SOLID") {
                            derived.insert("solidus_K".to_string(), json!(num));
                        }
                        if h_up.contains("LIQUIDUS") || h_up.contains("LIQUID") {
                            derived.insert("liquidus_K".to_string(), json!(num));
                        }
                        if h_up.contains("BUBBLE") || h_up.contains("BOIL") {
                            derived.insert("bubble_point_K".to_string(), json!(num));
                        }
                        if h_up.contains("DEW") {
                            derived.insert("dew_point_K".to_string(), json!(num));
                        }
                    } else {
                        row.insert(h.to_string(), json!(v));
                    }
                }
            }
            rows.push(json!(row));
        }

        let output = json!({
            "task_id": task_id,
            "task_type": "boiling_point",
            "status": "completed",
            "result": {
                "data_summary": {
                    "columns": headers,
                    "rows": rows
                },
                "derived_metrics": derived
            },
            "units": {"temperature": "K", "pressure": "Pa"},
            "files": files_map
        });
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
// 扩展工具：二元相图、三元相图、沸点、热力学性质（全部注册到 MCP）
// ═══════════════════════════════════════════════════════════════════

#[derive(Deserialize, Serialize, Default)]
pub struct SubmitBinaryTask {
    pub api_key: Option<String>,
}

impl SubmitBinaryTask {
    pub fn new(api_key: String) -> Self {
        Self { api_key: Some(api_key) }
    }
}

impl Tool for SubmitBinaryTask {
    const NAME: &'static str = "calphamesh_submit_binary_task";
    type Error = CalphaMeshError;
    type Args = BinaryTaskParams;
    type Output = String;

    async fn definition(&self, _prompt: String) -> ToolDefinition {
        ToolDefinition {
            name: Self::NAME.to_string(),
            description: "提交二元平衡相图计算任务。在两端点成分之间扫描温度区间，计算 Al-Si 等二元系的平衡相图数据。\n\n任务异步执行（典型耗时 20-60 秒）。提交后调用 calphamesh_get_task_result 获取结果。\n\n**Al-Si 二元相图（推荐用法）**：\n- components=[\"AL\",\"SI\"]，tdb_file=\"Al-Si-Mg-Fe-Mn_by_wf.TDB\"\n- start_composition Al=1/SI=0（纯铝端），end_composition Al=0.7/SI=0.3（30%Si端）\n- temperature_start=500K，temperature_end=1200K".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "components": {
                        "type": "array",
                        "items": {"type": "string"},
                        "description": "二元体系元素列表，必须恰好 2 个元素，大写，如 [\"AL\",\"SI\"]。",
                        "minItems": 2,
                        "maxItems": 2
                    },
                    "start_composition": {
                        "type": "object",
                        "additionalProperties": {"type": "number", "minimum": 0, "maximum": 1},
                        "description": "Al 端（富铝侧）成分，原子分数之和须等于 1.0。示例：{\"AL\":1.0,\"SI\":0.0}"
                    },
                    "end_composition": {
                        "type": "object",
                        "additionalProperties": {"type": "number", "minimum": 0, "maximum": 1},
                        "description": "Si 端（富 Si 侧）成分，原子分数之和须等于 1.0。示例：{\"AL\":0.7,\"SI\":0.3}"
                    },
                    "start_temperature": {
                        "type": "number",
                        "description": "相图计算温度下限，单位 K，范围 200~6000。",
                        "minimum": 200,
                        "maximum": 6000
                    },
                    "end_temperature": {
                        "type": "number",
                        "description": "相图计算温度上限，单位 K，须大于 start_temperature。",
                        "minimum": 200,
                        "maximum": 6000
                    },
                    "tdb_file": {
                        "type": "string",
                        "enum": ["FE-C-SI-MN-CU-TI-O.TDB", "B-C-SI-ZR-HF-LA-Y-TI-O.TDB", "Al-Si-Mg-Fe-Mn_by_wf.TDB"],
                        "description": "热力学数据库文件名。Al-Si 二元相图选 Al-Si-Mg-Fe-Mn_by_wf.TDB。"
                    }
                },
                "required": ["components", "start_composition", "end_composition", "start_temperature", "end_temperature", "tdb_file"],
                "additionalProperties": false
            }),
        }
    }

    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        // 基础校验
        validate_composition_sum(&args.start_composition)?;
        validate_composition_sum(&args.end_composition)?;
        validate_tdb_contains_elements(&args.tdb_file, &args.components)?;
        let api_key = get_api_key(&args.api_key, &self.api_key)?;
        let system = format_components_summary(&args.components);
        let t_start = args.start_temperature;
        let t_end = args.end_temperature;
        let client = CalphaMeshClient::new(api_key);
        let resp = client.submit_binary_task(args).await?;
        let output = json!({
            "task_id": resp.id,
            "status": resp.status,
            "task_type": "binary_equilibrium",
            "summary": format!("Binary 相图任务已提交：{} 体系，温度范围 {}~{} K", system, t_start, t_end),
            "estimated_wait_seconds": 40,
            "next_action": format!("调用 calphamesh_get_task_result(task_id={}) 等待并获取结果", resp.id)
        });
        Ok(serde_json::to_string(&output).unwrap_or_default())
    }
}

#[derive(Deserialize, Serialize, Default)]
pub struct SubmitTernaryTask {
    pub api_key: Option<String>,
}

impl SubmitTernaryTask {
    pub fn new(api_key: String) -> Self {
        Self { api_key: Some(api_key) }
    }
}

impl Tool for SubmitTernaryTask {
    const NAME: &'static str = "calphamesh_submit_ternary_task";
    type Error = CalphaMeshError;
    type Args = TernaryTaskParams;
    type Output = String;

    async fn definition(&self, _prompt: String) -> ToolDefinition {
        ToolDefinition {
            name: Self::NAME.to_string(),
            description: "提交三元等温截面计算任务。在给定温度下计算三元相图（成分三角形），输出相区边界、共轭线（tie-line）和三相三角（tie-triangle）数据，可直接用于 Plotly 可视化。\n\n任务异步执行（典型耗时 30-120 秒）。提交后调用 calphamesh_get_task_result 获取结果。\n\n**Al-Mg-Si 三元截面（推荐用法）**：\n- components=[\"AL\",\"MG\",\"SI\"]，tdb_file=\"Al-Si-Mg-Fe-Mn_by_wf.TDB\"\n- temperature=773K（时效温度附近）\n- 三顶点分别为纯 AL、纯 MG、纯 SI".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "components": {
                        "type": "array",
                        "items": {"type": "string"},
                        "description": "三元体系元素列表，必须恰好 3 个元素，大写，如 [\"AL\",\"MG\",\"SI\"]。",
                        "minItems": 3,
                        "maxItems": 3
                    },
                    "temperature": {
                        "type": "number",
                        "description": "等温截面温度，单位 K，范围 200~6000。典型 Al-Mg-Si 析出研究用 773K（500°C）。",
                        "minimum": 200,
                        "maximum": 6000
                    },
                    "composition_y": {
                        "type": "object",
                        "additionalProperties": {"type": "number", "minimum": 0, "maximum": 1},
                        "description": "三角形顶点 Y 的成分（通常为第一个组元的纯元素端），原子分数之和须等于 1.0。示例：{\"AL\":1.0,\"MG\":0.0,\"SI\":0.0}"
                    },
                    "composition_x": {
                        "type": "object",
                        "additionalProperties": {"type": "number", "minimum": 0, "maximum": 1},
                        "description": "三角形顶点 X 的成分（通常为第二个组元的纯元素端），原子分数之和须等于 1.0。示例：{\"AL\":0.0,\"MG\":1.0,\"SI\":0.0}"
                    },
                    "composition_o": {
                        "type": "object",
                        "additionalProperties": {"type": "number", "minimum": 0, "maximum": 1},
                        "description": "三角形顶点 O 的成分（通常为第三个组元的纯元素端），原子分数之和须等于 1.0。示例：{\"AL\":0.0,\"MG\":0.0,\"SI\":1.0}"
                    },
                    "tdb_file": {
                        "type": "string",
                        "enum": ["FE-C-SI-MN-CU-TI-O.TDB", "B-C-SI-ZR-HF-LA-Y-TI-O.TDB", "Al-Si-Mg-Fe-Mn_by_wf.TDB"],
                        "description": "热力学数据库文件名。Al-Mg-Si 三元相图选 Al-Si-Mg-Fe-Mn_by_wf.TDB。"
                    }
                },
                "required": ["components", "temperature", "composition_y", "composition_x", "composition_o", "tdb_file"],
                "additionalProperties": false
            }),
        }
    }

    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        validate_composition_sum(&args.composition_x)?;
        validate_composition_sum(&args.composition_y)?;
        validate_composition_sum(&args.composition_o)?;
        validate_tdb_contains_elements(&args.tdb_file, &args.components)?;
        let api_key = get_api_key(&args.api_key, &self.api_key)?;
        let system = format_components_summary(&args.components);
        let temp = args.temperature;
        let client = CalphaMeshClient::new(api_key);
        let resp = client.submit_ternary_task(args).await?;
        let output = json!({
            "task_id": resp.id,
            "status": resp.status,
            "task_type": "ternary_calculation",
            "summary": format!("Ternary 相图任务已提交：{} 体系，等温截面 {} K", system, temp),
            "estimated_wait_seconds": 60,
            "next_action": format!("调用 calphamesh_get_task_result(task_id={}) 等待并获取结果", resp.id)
        });
        Ok(serde_json::to_string(&output).unwrap_or_default())
    }
}

#[derive(Deserialize, Serialize, Default)]
pub struct SubmitBoilingPointTask {
    pub api_key: Option<String>,
}

impl SubmitBoilingPointTask {
    pub fn new(api_key: String) -> Self {
        Self { api_key: Some(api_key) }
    }
}

impl Tool for SubmitBoilingPointTask {
    const NAME: &'static str = "calphamesh_submit_boiling_point_task";
    type Error = CalphaMeshError;
    type Args = BoilingPointParams;
    type Output = String;

    async fn definition(&self, _prompt: String) -> ToolDefinition {
        ToolDefinition {
            name: Self::NAME.to_string(),
            description: "提交沸点/熔点搜索任务。在给定压力和温度搜索区间内，计算指定成分的固相线（solidus）、液相线（liquidus）、泡点（bubble point）和露点（dew point）。\n\n任务异步执行（典型耗时 15-30 秒）。提交后调用 calphamesh_get_task_result 获取结果。\n\n**适用场景**：纯元素或简单组分的熔点/沸点计算。多元合金的液相线/固相线推荐改用 Scheil 模拟。\n\n**注意**：pressure 单位为 Pa（不是 log10），常压为 101325 Pa。".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "components": {
                        "type": "array",
                        "items": {"type": "string"},
                        "description": "组元列表，大写，如 [\"AL\"]（单元素）。",
                        "minItems": 1
                    },
                    "composition": {
                        "type": "object",
                        "additionalProperties": {"type": "number", "minimum": 0, "maximum": 1},
                        "description": "各组元原子分数，所有值之和须等于 1.0。纯铝示例：{\"AL\":1.0}"
                    },
                    "pressure": {
                        "type": "number",
                        "description": "计算压力，单位 Pa（帕斯卡）。常压为 101325 Pa。",
                        "minimum": 1,
                        "maximum": 1e10
                    },
                    "temperature_range": {
                        "type": "array",
                        "items": {"type": "number"},
                        "minItems": 2,
                        "maxItems": 2,
                        "description": "搜索温度区间 [T_min, T_max]，单位 K。铝的熔点/沸点搜索推荐 [800, 4000]。"
                    },
                    "tdb_file": {
                        "type": "string",
                        "enum": ["FE-C-SI-MN-CU-TI-O.TDB", "B-C-SI-ZR-HF-LA-Y-TI-O.TDB", "Al-Si-Mg-Fe-Mn_by_wf.TDB"],
                        "description": "热力学数据库文件名。纯 Al 的沸点/熔点选 Al-Si-Mg-Fe-Mn_by_wf.TDB。"
                    }
                },
                "required": ["components", "composition", "pressure", "temperature_range", "tdb_file"],
                "additionalProperties": false
            }),
        }
    }

    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        validate_composition_sum(&args.composition)?;
        validate_tdb_contains_elements(&args.tdb_file, &args.components)?;
        let api_key = get_api_key(&args.api_key, &self.api_key)?;
        let system = format_components_summary(&args.components);
        let client = CalphaMeshClient::new(api_key);
        let resp = client.submit_boiling_point_task(args).await?;
        let output = json!({
            "task_id": resp.id,
            "status": resp.status,
            "task_type": "boiling_point",
            "summary": format!("沸点/熔点任务已提交：{} 体系", system),
            "estimated_wait_seconds": 20,
            "next_action": format!("调用 calphamesh_get_task_result(task_id={}) 等待并获取结果", resp.id)
        });
        Ok(serde_json::to_string(&output).unwrap_or_default())
    }
}

#[derive(Deserialize, Serialize, Default)]
pub struct SubmitThermoPropertiesTask {
    pub api_key: Option<String>,
}

impl SubmitThermoPropertiesTask {
    pub fn new(api_key: String) -> Self {
        Self { api_key: Some(api_key) }
    }
}

impl Tool for SubmitThermoPropertiesTask {
    const NAME: &'static str = "calphamesh_submit_thermodynamic_properties_task";
    type Error = CalphaMeshError;
    type Args = ThermoPropertiesParams;
    type Output = String;

    async fn definition(&self, _prompt: String) -> ToolDefinition {
        ToolDefinition {
            name: Self::NAME.to_string(),
            description: "提交热力学性质扫描任务。在给定成分下，沿温度（和压力）区间计算摩尔 Gibbs 自由能（GM）、摩尔焓（HM）、摩尔熵（SM）、摩尔定压热容（CPM）等热力学函数随温度的变化曲线。\n\n任务异步执行（典型耗时 15-40 秒）。提交后调用 calphamesh_get_task_result 获取结果。\n\n**压力参数说明**：pressure_start/pressure_end 是 log10(P/Pa) 而非直接压力值。常压（100000 Pa）对应 pressure_start=pressure_end=5。\n\n**推荐用法（Al 合金）**：temperature_start=500, temperature_end=950, increments=25, pressure_start=5, pressure_end=5, pressure_increments=2, properties=[\"GM\",\"HM\",\"SM\",\"CPM\"]".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "components": {
                        "type": "array",
                        "items": {"type": "string"},
                        "description": "合金元素列表，大写，如 [\"AL\",\"SI\",\"MG\",\"FE\",\"MN\"]。",
                        "minItems": 2
                    },
                    "composition": {
                        "type": "object",
                        "additionalProperties": {"type": "number", "minimum": 0, "maximum": 1},
                        "description": "合金成分（原子分数），所有值之和须等于 1.0。"
                    },
                    "temperature_start": {
                        "type": "number",
                        "description": "起始温度，单位 K，范围 200~6000。",
                        "minimum": 200,
                        "maximum": 6000
                    },
                    "temperature_end": {
                        "type": "number",
                        "description": "终止温度，单位 K，须大于 temperature_start。",
                        "minimum": 200,
                        "maximum": 6000
                    },
                    "increments": {
                        "type": "integer",
                        "description": "温度步长，单位 K。推荐 5~25 K，建议 25 K（快速扫描）或 5 K（精细分析）。",
                        "minimum": 1,
                        "maximum": 200
                    },
                    "pressure_start": {
                        "type": "number",
                        "description": "起始压力，以 log10(P/Pa) 表示。常压 = 5（即 10^5 Pa = 100000 Pa）。",
                        "minimum": 0,
                        "maximum": 15
                    },
                    "pressure_end": {
                        "type": "number",
                        "description": "终止压力，以 log10(P/Pa) 表示。常压扫描时与 pressure_start 相同，均设为 5。",
                        "minimum": 0,
                        "maximum": 15
                    },
                    "pressure_increments": {
                        "type": "integer",
                        "description": "压力扫描步数（对数压力步长）。常压计算时设为 2（最小值），相当于固定压力。",
                        "minimum": 1,
                        "maximum": 50
                    },
                    "properties": {
                        "type": "array",
                        "items": {
                            "type": "string",
                            "enum": ["GM", "HM", "SM", "CPM"]
                        },
                        "description": "需要输出的热力学性质列表。GM=摩尔吉布斯自由能(J/mol), HM=摩尔焓(J/mol), SM=摩尔熵(J/mol/K), CPM=摩尔定压热容(J/mol/K)。推荐全选：[\"GM\",\"HM\",\"SM\",\"CPM\"]",
                        "minItems": 1
                    },
                    "tdb_file": {
                        "type": "string",
                        "enum": ["FE-C-SI-MN-CU-TI-O.TDB", "B-C-SI-ZR-HF-LA-Y-TI-O.TDB", "Al-Si-Mg-Fe-Mn_by_wf.TDB"],
                        "description": "热力学数据库文件名。Al 合金热力学性质选 Al-Si-Mg-Fe-Mn_by_wf.TDB。"
                    }
                },
                "required": ["components", "composition", "temperature_start", "temperature_end", "increments", "pressure_start", "pressure_end", "pressure_increments", "properties", "tdb_file"],
                "additionalProperties": false
            }),
        }
    }

    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        validate_composition_sum(&args.composition)?;
        validate_tdb_contains_elements(&args.tdb_file, &args.components)?;
        let api_key = get_api_key(&args.api_key, &self.api_key)?;
        let system = format_components_summary(&args.components);
        let t_start = args.temperature_start;
        let t_end = args.temperature_end;
        let client = CalphaMeshClient::new(api_key);
        let resp = client.submit_thermo_properties_task(args).await?;
        let output = json!({
            "task_id": resp.id,
            "status": resp.status,
            "task_type": "thermodynamic_properties",
            "summary": format!("热力学性质任务已提交：{} 体系，温度范围 {}~{} K", system, t_start, t_end),
            "estimated_wait_seconds": 25,
            "next_action": format!("调用 calphamesh_get_task_result(task_id={}) 等待并获取结果", resp.id)
        });
        Ok(serde_json::to_string(&output).unwrap_or_default())
    }
}

