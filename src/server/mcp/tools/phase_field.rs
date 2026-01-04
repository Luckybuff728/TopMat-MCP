//! Phase Field MCP 工具
//!
//! 提供与  API 集成的相场模拟工具，支持 TiAlN 调幅分解模拟和 PVD 物理气相沉积模拟

use serde::{Deserialize, Serialize};
use serde_json::json;
use std::error::Error as StdError;
use rig::{
    completion::ToolDefinition,
    tool::Tool,
};

use reqwest;
use base64::{Engine as _, engine::general_purpose::STANDARD};

//  API 基础 URL
const MESOSPIRE_API_URL: &str = "http://192.168.7.103:4001";

// ==================== 错误类型 ====================

#[derive(Debug)]
pub enum PhaseFieldError {
    HttpError(String),
    ApiError { status: u16, message: String },
    JsonError(String),
    InvalidRequest(String),
    MissingParameter(String),
    Base64Error(String),
}

impl std::fmt::Display for PhaseFieldError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PhaseFieldError::HttpError(msg) => write!(f, "HTTP请求失败: {}", msg),
            PhaseFieldError::ApiError { status, message } => {
                write!(f, "API错误 (状态码 {}): {}", status, message)
            }
            PhaseFieldError::JsonError(msg) => write!(f, "JSON序列化/反序列化错误: {}", msg),
            PhaseFieldError::InvalidRequest(msg) => write!(f, "无效请求: {}", msg),
            PhaseFieldError::MissingParameter(param) => write!(f, "缺少必需参数: {}", param),
            PhaseFieldError::Base64Error(msg) => write!(f, "Base64编码错误: {}", msg),
        }
    }
}

impl StdError for PhaseFieldError {}

// ==================== 请求/响应结构体 ====================

/// 创建任务请求
#[derive(Debug, Serialize)]
struct CreateTaskRequest {
    pub omp_threads: i32,
    pub task_config: String,
    pub task_name: String,
    pub tool_name: String,
}

/// 任务ID请求
#[derive(Debug, Serialize)]
struct TaskIdRequest {
    pub task_id: String,
}

/// 文件检索请求
#[derive(Debug, Serialize)]
struct FileRetrieveRequest {
    pub file_path: String,
    pub task_id: String,
}

/// 任务信息
#[derive(Debug, Deserialize)]
struct Task {
    pub exe_tool: String,
    pub pid: i32,
    pub start_time: String,
    pub status: String,
    pub task_id: String,
    pub task_name: String,
}

/// 任务列表响应
#[derive(Debug, Deserialize)]
struct TaskListResponse {
    pub data: TaskListData,
    pub message: String,
    pub success: bool,
    pub timestamp: String,
}

/// 任务列表数据
#[derive(Debug, Deserialize)]
struct TaskListData {
    pub tasks: Vec<Task>,
    pub total: i32,
}

/// 创建任务响应
#[derive(Debug, Deserialize)]
struct CreateTaskResponse {
    pub data: CreateTaskData,
    pub message: String,
    pub success: bool,
    pub timestamp: String,
}

/// 创建任务数据
#[derive(Debug, Deserialize)]
struct CreateTaskData {
    pub command_line: String,
    pub pid: i32,
    pub task_id: String,
    pub task_name: String,
    pub working_directory: String,
}

/// 任务状态响应
#[derive(Debug, Deserialize)]
struct TaskStatusResponse {
    pub data: TaskStatusData,
    pub message: String,
    pub success: bool,
    pub timestamp: String,
}

/// 任务状态数据
#[derive(Debug, Deserialize)]
struct TaskStatusData {
    pub duration_seconds: Option<i32>,
    pub end_checked_time: Option<String>,
    pub exe_tool: Option<String>,
    pub pid: Option<i32>,
    pub start_time: Option<String>,
    pub status: Option<String>,
    pub task_id: Option<String>,
    pub task_name: Option<String>,
    pub working_directory: Option<String>,
}

/// 任务停止响应
#[derive(Debug, Deserialize)]
struct TaskStopResponse {
    pub data: TaskStopData,
    pub message: String,
    pub success: bool,
    pub timestamp: String,
}

/// 任务停止数据
#[derive(Debug, Deserialize)]
struct TaskStopData {
    pub message: String,
    pub task_id: String,
    pub task_name: String,
}

/// 文件列表响应
#[derive(Debug, Deserialize)]
struct FileListResponse {
    pub data: Vec<String>,
    pub message: String,
    pub success: bool,
    pub timestamp: String,
}

// ==================== 请求参数结构体 ====================

/// 调幅分解任务请求参数
#[derive(Debug, Deserialize)]
pub struct SpinodalDecompositionRequest {
    pub task_name: Option<String>,
    pub mean: Option<f64>,
}

/// PVD模拟任务请求参数
#[derive(Debug, Deserialize)]
pub struct PvdSimulationRequest {
    pub task_name: Option<String>,
    pub ay: Option<f64>,
    pub b: Option<f64>,
    pub c: Option<f64>,
    pub d: Option<f64>,
    pub a: Option<f64>,
}

/// 任务ID请求参数
#[derive(Debug, Deserialize)]
pub struct TaskIdParams {
    pub task_id: String,
}

/// 文件检索请求参数
#[derive(Debug, Deserialize)]
pub struct FileRetrieveParams {
    pub task_id: String,
    pub file_path: String,
}

/// 任务列表请求参数
#[derive(Debug, Deserialize)]
pub struct TaskListParams {
    pub status: Option<String>,
}

// ==================== 工具实现 ====================

/// 提交调幅分解模拟任务
#[derive(Deserialize, Serialize)]
pub struct SubmitSpinodalDecompositionTask;

impl Default for SubmitSpinodalDecompositionTask {
    fn default() -> Self {
        Self
    }
}

impl Tool for SubmitSpinodalDecompositionTask {
    const NAME: &'static str = "phase_field_submit_spinodal_decomposition_task";
    type Error = PhaseFieldError;
    type Args = SpinodalDecompositionRequest;
    type Output = String;

    async fn definition(&self, _prompt: String) -> ToolDefinition {
        ToolDefinition {
            name: "phase_field_submit_spinodal_decomposition_task".to_string(),
            description: "提交TiAlN调幅分解模拟任务".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "task_name": {
                        "type": "string",
                        "description": "任务名称（可选，默认为'调幅分解模拟'）"
                    },
                    "mean": {
                        "type": "number",
                        "description": "TiAlN中ALN含量，默认为0.0"
                    }
                },
                "required": []
            }),
        }
    }

    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        let client = reqwest::Client::new();
        let task_name = args.task_name.unwrap_or_else(|| "调幅分解模拟".to_string());
        let mean = args.mean.unwrap_or(0.0);

        // 构建任务配置
        let task_config = json!({
            "clear": true,
            "elastic": {
                "average_strain": [1e-04, 1e-04, 1e-04, 0, 0, 0],
                "eigen_strain": [0.048, 0.048, 0.048, 0, 0, 0],
                "lambda_matrix": [548500000000.0, 128500000000.0, 128500000000.0, 0.0, 0.0, 0.0, 128500000000.0, 548500000000.0, 128500000000.0, 0.0, 0.0, 0.0, 128500000000.0, 128500000000.0, 548500000000.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 169000000000.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 169000000000.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 169000000000.0],
                "lambda_precipitate": [413500000000.0, 157000000000.0, 157000000000.0, 0.0, 0.0, 0.0, 157000000000.0, 413500000000.0, 157000000000.0, 0.0, 0.0, 0.0, 157000000000.0, 157000000000.0, 413500000000.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 256000000000.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 256000000000.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 256000000000.0],
                "xe_matrix": 0.1,
                "xe_precipitate": 0.8
            },
            "enable_elastic_energy": false,
            "mesh": {
                "current_time": 0,
                "dt": 0.01,
                "dx": 5e-10,
                "nx": 48,
                "ny": 48,
                "nz": 48,
                "simulation_time": 1200
            },
            "omp_threads": 4,
            "output": {
                "every_number": 20,
                "factor": 1,
                "initial_interval": 1,
                "output_time": 0,
                "variable": ["M", "kappa", "dfgibbsdc"]
            },
            "parameter": {
                "Vm": 1e-05,
                "mean": mean,
                "variance": 0.01
            },
            "path": "UNb-demo",
            "phase": {
                "chemical_mobility": {
                    "type": "TiALN_zhang_FCC_ChemicalMobility_Functor"
                },
                "element": ["ALN"],
                "energy": {
                    "type": "TiALN_zhang_FCC_Energy_Functor"
                },
                "kappa": {
                    "type": "TiALN_zhang_FCC_Kappa_Functor"
                },
                "name": "Fcc",
                "type": "MultiComponentPhase_Vector"
            },
            "temperature": {
                "T": 1073,
                "dTdt": 0,
                "type": "LinearTemperature"
            },
            "type": "CahnHilliard_Vector"
        });

        let task_config_str = serde_json::to_string(&task_config)
            .map_err(|e| PhaseFieldError::JsonError(format!("Failed to serialize config: {}", e)))?;

        let task_config_base64 = STANDARD.encode(task_config_str.as_bytes());

        let request_body = CreateTaskRequest {
            omp_threads: 128,
            task_config: task_config_base64,
            task_name: task_name.clone(),
            tool_name: "phase_field".to_string(),
        };

        let url = format!("{}/tasks", MESOSPIRE_API_URL);
        let response = client
            .post(&url)
            .json(&request_body)
            .send()
            .await
            .map_err(|e| PhaseFieldError::HttpError(e.to_string()))?;

        if response.status() != 200 {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            return Err(PhaseFieldError::ApiError {
                status: status.as_u16(),
                message: error_text,
            });
        }

        let task_response: CreateTaskResponse = response
            .json()
            .await
            .map_err(|e| PhaseFieldError::JsonError(format!("Failed to parse response: {}", e)))?;

        Ok(format!(
            "调幅分解模拟任务已提交\nID: {}\n名称: {}\n进程ID: {}",
            task_response.data.task_id,
            task_response.data.task_name,
            task_response.data.pid
        ))
    }
}

/// 提交PVD物理气相沉积模拟任务
#[derive(Deserialize, Serialize)]
pub struct SubmitPvdSimulationTask;

impl Default for SubmitPvdSimulationTask {
    fn default() -> Self {
        Self
    }
}

impl Tool for SubmitPvdSimulationTask {
    const NAME: &'static str = "phase_field_submit_pvd_simulation_task";
    type Error = PhaseFieldError;
    type Args = PvdSimulationRequest;
    type Output = String;

    async fn definition(&self, _prompt: String) -> ToolDefinition {
        ToolDefinition {
            name: "phase_field_submit_pvd_simulation_task".to_string(),
            description: "提交PVD物理气相沉积模拟任务".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "task_name": {
                        "type": "string",
                        "description": "任务名称（可选，默认为'PVD物理气相沉积模拟'）"
                    },
                    "ay": {
                        "type": "number",
                        "description": "气相原子/分子入射到基底表面的速率 (nm/s)，默认为-0.29"
                    },
                    "b": {
                        "type": "number",
                        "description": "控制气相到固相转变速率的参数 (nm²/s)，默认为0.25"
                    },
                    "c": {
                        "type": "number",
                        "description": "模拟中引入的随机噪声幅度 (J/nm)，默认为2.5"
                    },
                    "d": {
                        "type": "number",
                        "description": "表面原子在基底上的扩散能力 (nm²/s)，默认为1.0"
                    },
                    "a": {
                        "type": "number",
                        "description": "描述气相-固相界面能量梯度的系数 (J/nm)，默认为0.3"
                    }
                },
                "required": []
            }),
        }
    }

    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        let client = reqwest::Client::new();
        let task_name = args.task_name.unwrap_or_else(|| "PVD物理气相沉积模拟".to_string());
        let ay = args.ay.unwrap_or(-0.29);
        let b = args.b.unwrap_or(0.25);
        let c = args.c.unwrap_or(2.5);
        let d = args.d.unwrap_or(1.0);
        let a_param = args.a.unwrap_or(0.3);

        // 构建任务配置
        let task_config = json!({
            "boundary_condition": {
                "BC0X": { "type": "ZEROFLUX" },
                "BC0Y": { "type": "ZEROFLUX" },
                "BC0Z": { "type": "ZEROFLUX" },
                "BCNX": { "type": "ZEROFLUX" },
                "BCNY": { "type": "ZEROFLUX" },
                "BCNZ": { "type": "ZEROFLUX" }
            },
            "clear": true,
            "flag": {
                "average": true,
                "roation": false
            },
            "mesh": {
                "current_time": 0,
                "dt": 0.01,
                "dx": 1,
                "nx": 48,
                "ny": 48,
                "nz": 16,
                "simulation_time": 3000
            },
            "omp_threads": 4,
            "output": {
                "every_number": 10,
                "factor": 1,
                "initial_interval": 1
            },
            "parameter": {
                "Ay": ay,
                "B": b,
                "C": c,
                "D": d,
                "a": a_param,
                "g0": 1,
                "mean": 0,
                "moving_frame_interval": 12,
                "stddev": 0.5
            },
            "path": "results",
            "statistic_path": "results/statistic.csv",
            "type": "PhysicalVapor"
        });

        let task_config_str = serde_json::to_string(&task_config)
            .map_err(|e| PhaseFieldError::JsonError(format!("Failed to serialize config: {}", e)))?;

        let task_config_base64 = STANDARD.encode(task_config_str.as_bytes());

        let request_body = CreateTaskRequest {
            omp_threads: 128,
            task_config: task_config_base64,
            task_name: task_name.clone(),
            tool_name: "phase_field".to_string(),
        };

        let url = format!("{}/tasks", MESOSPIRE_API_URL);
        let response = client
            .post(&url)
            .json(&request_body)
            .send()
            .await
            .map_err(|e| PhaseFieldError::HttpError(e.to_string()))?;

        if response.status() != 200 {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            return Err(PhaseFieldError::ApiError {
                status: status.as_u16(),
                message: error_text,
            });
        }

        let task_response: CreateTaskResponse = response
            .json()
            .await
            .map_err(|e| PhaseFieldError::JsonError(format!("Failed to parse response: {}", e)))?;

        Ok(format!(
            "PVD模拟任务已提交\nID: {}\n名称: {}\n进程ID: {}",
            task_response.data.task_id,
            task_response.data.task_name,
            task_response.data.pid
        ))
    }
}

/// 获取任务列表
#[derive(Deserialize, Serialize)]
pub struct GetTaskList;

impl Default for GetTaskList {
    fn default() -> Self {
        Self
    }
}

impl Tool for GetTaskList {
    const NAME: &'static str = "phase_field_get_task_list";
    type Error = PhaseFieldError;
    type Args = TaskListParams;
    type Output = String;

    async fn definition(&self, _prompt: String) -> ToolDefinition {
        ToolDefinition {
            name: "phase_field_get_task_list".to_string(),
            description: "获取任务列表，可按状态过滤".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "status": {
                        "type": "string",
                        "description": "过滤任务状态 (可选)"
                    }
                },
                "required": []
            }),
        }
    }

    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        let client = reqwest::Client::new();

        let url = if let Some(status) = args.status {
            format!("{}/tasks?status={}", MESOSPIRE_API_URL, status)
        } else {
            format!("{}/tasks", MESOSPIRE_API_URL)
        };

        let response = client
            .get(&url)
            .send()
            .await
            .map_err(|e| PhaseFieldError::HttpError(e.to_string()))?;

        if response.status() != 200 {
            let status = response.status();
            return Err(PhaseFieldError::ApiError {
                status: status.as_u16(),
                message: "任务列表查询失败".to_string(),
            });
        }

        let list_response: TaskListResponse = response
            .json()
            .await
            .map_err(|e| PhaseFieldError::JsonError(format!("Failed to parse response: {}", e)))?;

        let mut result = format!("任务列表 (共{}个)\n\n", list_response.data.total);

        if list_response.data.tasks.is_empty() {
            result.push_str("暂无任务");
        } else {
            for (idx, task) in list_response.data.tasks.iter().enumerate() {
                result.push_str(&format!(
                    "{}. [{}] {} | {} | {}\n",
                    idx + 1,
                    task.status,
                    task.task_name,
                    task.task_id,
                    task.start_time
                ));
            }
        }

        Ok(result)
    }
}

/// 获取任务状态
#[derive(Deserialize, Serialize)]
pub struct GetTaskStatus;

impl Default for GetTaskStatus {
    fn default() -> Self {
        Self
    }
}

impl Tool for GetTaskStatus {
    const NAME: &'static str = "phase_field_get_task_status";
    type Error = PhaseFieldError;
    type Args = TaskIdParams;
    type Output = String;

    async fn definition(&self, _prompt: String) -> ToolDefinition {
        ToolDefinition {
            name: "phase_field_get_task_status".to_string(),
            description: "根据任务ID查询任务状态和详细信息".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "task_id": {
                        "type": "string",
                        "description": "任务ID (UUID格式)"
                    }
                },
                "required": ["task_id"]
            }),
        }
    }

    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        let client = reqwest::Client::new();

        let request_body = TaskIdRequest {
            task_id: args.task_id.clone(),
        };

        let url = format!("{}/tasks/status", MESOSPIRE_API_URL);
        let response = client
            .post(&url)
            .json(&request_body)
            .send()
            .await
            .map_err(|e| PhaseFieldError::HttpError(e.to_string()))?;

        if response.status() != 200 {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            return Err(PhaseFieldError::ApiError {
                status: status.as_u16(),
                message: error_text,
            });
        }

        let status_response: TaskStatusResponse = response
            .json()
            .await
            .map_err(|e| PhaseFieldError::JsonError(format!("Failed to parse response: {}", e)))?;

        Ok(format!(
            "任务状态\nID: {}\n名称: {}\n状态: {}\n进程ID: {}\n开始: {}\n运行时长: {}秒",
            status_response.data.task_id.as_deref().unwrap_or(&args.task_id),
            status_response.data.task_name.as_deref().unwrap_or("未知"),
            status_response.data.status.as_deref().unwrap_or("未知"),
            status_response.data.pid.map(|p| p.to_string()).unwrap_or_else(|| "未知".to_string()),
            status_response.data.start_time.as_deref().unwrap_or("未知"),
            status_response.data.duration_seconds.map(|d| d.to_string()).unwrap_or_else(|| "未知".to_string())
        ))
    }
}

/// 停止任务
#[derive(Deserialize, Serialize)]
pub struct StopTask;

impl Default for StopTask {
    fn default() -> Self {
        Self
    }
}

impl Tool for StopTask {
    const NAME: &'static str = "phase_field_stop_task";
    type Error = PhaseFieldError;
    type Args = TaskIdParams;
    type Output = String;

    async fn definition(&self, _prompt: String) -> ToolDefinition {
        ToolDefinition {
            name: "phase_field_stop_task".to_string(),
            description: "停止正在运行的任务".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "task_id": {
                        "type": "string",
                        "description": "任务ID (UUID格式)"
                    }
                },
                "required": ["task_id"]
            }),
        }
    }

    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        let client = reqwest::Client::new();

        let request_body = TaskIdRequest {
            task_id: args.task_id.clone(),
        };

        let url = format!("{}/tasks/stop", MESOSPIRE_API_URL);
        let response = client
            .post(&url)
            .json(&request_body)
            .send()
            .await
            .map_err(|e| PhaseFieldError::HttpError(e.to_string()))?;

        if response.status() != 200 {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            return Err(PhaseFieldError::ApiError {
                status: status.as_u16(),
                message: error_text,
            });
        }

        let stop_response: TaskStopResponse = response
            .json()
            .await
            .map_err(|e| PhaseFieldError::JsonError(format!("Failed to parse response: {}", e)))?;

        Ok(format!(
            "任务已停止\nID: {}\n名称: {}",
            stop_response.data.task_id,
            stop_response.data.task_name
        ))
    }
}

/// 探测任务文件
#[derive(Deserialize, Serialize)]
pub struct ProbeTaskFiles;

impl Default for ProbeTaskFiles {
    fn default() -> Self {
        Self
    }
}

impl Tool for ProbeTaskFiles {
    const NAME: &'static str = "phase_field_probe_task_files";
    type Error = PhaseFieldError;
    type Args = TaskIdParams;
    type Output = String;

    async fn definition(&self, _prompt: String) -> ToolDefinition {
        ToolDefinition {
            name: "phase_field_probe_task_files".to_string(),
            description: "获取任务的输出文件列表".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "task_id": {
                        "type": "string",
                        "description": "任务ID (UUID格式)"
                    }
                },
                "required": ["task_id"]
            }),
        }
    }

    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        let client = reqwest::Client::new();

        let url = format!("{}/tasks/probe?task_id={}", MESOSPIRE_API_URL, args.task_id);
        let response = client
            .get(&url)
            .send()
            .await
            .map_err(|e| PhaseFieldError::HttpError(e.to_string()))?;

        if response.status() != 200 {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            return Err(PhaseFieldError::ApiError {
                status: status.as_u16(),
                message: error_text,
            });
        }

        let file_list_response: FileListResponse = response
            .json()
            .await
            .map_err(|e| PhaseFieldError::JsonError(format!("Failed to parse response: {}", e)))?;

        let mut result = format!("任务 {} 的文件列表\n\n", args.task_id);

        if file_list_response.data.is_empty() {
            result.push_str("暂无文件");
        } else {
            for (idx, file) in file_list_response.data.iter().enumerate() {
                result.push_str(&format!("{}. {}\n", idx + 1, file));
            }
        }

        Ok(result)
    }
}

/// 检索文件内容
#[derive(Deserialize, Serialize)]
pub struct RetrieveFile;

impl Default for RetrieveFile {
    fn default() -> Self {
        Self
    }
}

impl Tool for RetrieveFile {
    const NAME: &'static str = "phase_field_retrieve_file";
    type Error = PhaseFieldError;
    type Args = FileRetrieveParams;
    type Output = String;

    async fn definition(&self, _prompt: String) -> ToolDefinition {
        ToolDefinition {
            name: "phase_field_retrieve_file".to_string(),
            description: "下载任务的指定文件内容".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "task_id": {
                        "type": "string",
                        "description": "任务ID (UUID格式)"
                    },
                    "file_path": {
                        "type": "string",
                        "description": "文件路径"
                    }
                },
                "required": ["task_id", "file_path"]
            }),
        }
    }

    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        let client = reqwest::Client::new();

        let request_body = FileRetrieveRequest {
            file_path: args.file_path.clone(),
            task_id: args.task_id,
        };

        let url = format!("{}/tasks/retrieve", MESOSPIRE_API_URL);
        let response = client
            .post(&url)
            .json(&request_body)
            .send()
            .await
            .map_err(|e| PhaseFieldError::HttpError(e.to_string()))?;

        if response.status() != 200 {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            return Err(PhaseFieldError::ApiError {
                status: status.as_u16(),
                message: error_text,
            });
        }

        let file_content = response
            .text()
            .await
            .map_err(|e| PhaseFieldError::HttpError(e.to_string()))?;

        Ok(format!(
            "文件内容\n路径: {}\n\n{}",
            args.file_path,
            file_content
        ))
    }
}