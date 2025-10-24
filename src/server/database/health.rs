use crate::server::database::DatabaseConnection;
use crate::server::models::ServiceStatus;
use std::time::Instant;
use tracing::{info, warn, error};

/// 检查数据库连接健康状态
pub async fn check_database_connection(db: Option<&DatabaseConnection>) -> ServiceStatus {
    let start_time = Instant::now();

    match db {
        Some(database_conn) => {
            // 执行实际的数据库健康检查
            match database_conn.health_check().await {
                Ok(()) => {
                    let duration = start_time.elapsed();
                    info!("数据库健康检查通过，耗时: {:?}", duration);

                    // 如果响应时间超过2秒，警告但仍然返回健康
                    if duration.as_secs() > 2 {
                        warn!("数据库响应较慢: {:?}ms", duration.as_millis());
                    }

                    ServiceStatus::Healthy
                }
                Err(e) => {
                    let duration = start_time.elapsed();
                    error!("数据库健康检查失败，耗时: {:?}, 错误: {}", duration, e);
                    ServiceStatus::Unhealthy
                }
            }
        }
        None => {
            warn!("数据库连接未初始化");
            ServiceStatus::Unhealthy
        }
    }
}

/// 检查数据库连接配置
pub async fn check_database_config() -> ServiceStatus {
    // 检查环境变量中的数据库配置
    match std::env::var("DATABASE_URL") {
        Ok(database_url) if !database_url.is_empty() => {
            info!("检测到数据库URL配置: {}", database_url);

            // 验证数据库URL格式
            if database_url.starts_with("sqlite:") {
                ServiceStatus::Healthy
            } else {
                error!("不支持的数据库类型: {}", database_url);
                ServiceStatus::Unhealthy
            }
        }
        Ok(_) | Err(_) => {
            // 使用默认的SQLite数据库路径
            let default_url = crate::server::database::connection::get_default_database_url();
            info!("使用默认数据库配置: {}", default_url);
            ServiceStatus::Healthy
        }
    }
}

/// 获取数据库状态详情
pub async fn get_database_status(db: Option<&DatabaseConnection>) -> DatabaseStatus {
    let start_time = Instant::now();

    let health = check_database_connection(db).await;
    let response_time_ms = start_time.elapsed().as_millis() as u64;

    DatabaseStatus {
        health,
        response_time_ms,
        url: std::env::var("DATABASE_URL")
            .unwrap_or_else(|_| crate::server::database::connection::get_default_database_url()),
        connection_pool_size: db.map(|conn| conn.pool().size()).unwrap_or(0),
        idle_connections: db.map(|conn| conn.pool().num_idle() as u32).unwrap_or(0),
    }
}

/// 数据库状态详情
#[derive(Debug, Clone)]
pub struct DatabaseStatus {
    /// 健康状态
    pub health: ServiceStatus,
    /// 响应时间（毫秒）
    pub response_time_ms: u64,
    /// 数据库连接URL
    pub url: String,
    /// 连接池大小
    pub connection_pool_size: u32,
    /// 空闲连接数
    pub idle_connections: u32,
}

impl DatabaseStatus {
    /// 是否健康
    pub fn is_healthy(&self) -> bool {
        matches!(self.health, ServiceStatus::Healthy)
    }

    /// 获取使用率
    pub fn get_usage_rate(&self) -> f64 {
        if self.connection_pool_size > 0 {
            (self.connection_pool_size - self.idle_connections) as f64 / self.connection_pool_size as f64
        } else {
            0.0
        }
    }
}