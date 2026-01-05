use reqwest;
use std::time::Duration;
use tracing::{debug, info, warn, error};
use sqlx::Row;

use crate::server::models::{ApiKeyInfo, AuthError, AuthResult, UserInfo};
use crate::server::database::DatabaseConnection;

/// 鉴权客户端
#[derive(Clone)]
pub struct AuthClient {
    /// HTTP客户端
    client: reqwest::Client,
    /// API服务地址
    api_url: String,
    /// 数据库连接（用于本地缓存）
    database: DatabaseConnection,
}

impl AuthClient {
    /// 创建新的鉴权客户端
    pub fn new(api_url: Option<String>, database: DatabaseConnection) -> Self {
        let api_url = api_url.unwrap_or_else(|| {
            std::env::var("AUTH_API_URL")
                .unwrap_or_else(|_| "https://api.topmaterial-tech.com".to_string())
        });

        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(10))
            .user_agent("TopMat-LLM/1.0")
            .build()
            .expect("Failed to create HTTP client");

        Self { client, api_url, database }
    }

    /// 验证API Key（带本地缓存）
    pub async fn verify_api_key(&self, api_key: &str) -> Result<AuthResult, AuthError> {
        debug!("开始验证API Key: {}", &api_key[..api_key.len().min(8)]);

        // 1. 首先检查本地缓存
        if let Ok(cached_result) = self.check_local_cache(api_key).await {
            info!("使用本地缓存验证成功: 用户={}", cached_result.user_info.username);
            return Ok(cached_result);
        }

        // 2. 本地缓存未命中或已过期，调用外部认证服务
        info!("本地缓存未命中，调用外部认证服务");
        let auth_result = self.verify_with_external_service(api_key).await?;

        // 3. 将认证结果存储到本地缓存
        if let Err(e) = self.save_to_cache(&auth_result, api_key).await {
            error!("保存认证缓存失败: {}", e);
            // 不影响认证结果，只记录错误
        } else {
            info!("认证信息已保存到本地缓存");
        }

        Ok(auth_result)
    }

    /// 检查本地缓存
    async fn check_local_cache(&self, api_key: &str) -> Result<AuthResult, AuthError> {
        // 从数据库查询API密钥信息
        let sql = "
            SELECT
                ak.id, ak.user_id, ak.api_key, ak.key_name, ak.is_active,
                ak.expires_at, ak.last_used_at, ak.created_at, ak.updated_at,
                u.username, u.email, u.subscription_level, u.subscription_expires_at
            FROM api_keys ak
            JOIN users u ON ak.user_id = u.id
            WHERE ak.api_key = $1 AND ak.is_active = TRUE
        ";

        let row = sqlx::query(sql)
            .bind(api_key)
            .fetch_optional(self.database.pool())
            .await
            .map_err(|e| {
                error!("查询本地缓存失败: {}", e);
                AuthError::DatabaseError(e.to_string())
            })?;

        let row = match row {
            Some(row) => row,
            None => {
                debug!("本地缓存中未找到API密钥");
                return Err(AuthError::InvalidApiKey);
            }
        };

        // 检查缓存是否过期（1小时）
        let last_updated: chrono::DateTime<chrono::Utc> = row.try_get("updated_at")
            .unwrap_or_else(|_| chrono::Utc::now());

        let one_hour_ago = chrono::Utc::now() - chrono::Duration::hours(1);

        if last_updated < one_hour_ago {
            info!("本地缓存已过期，需要重新验证");
            return Err(AuthError::CacheExpired);
        }

        // 检查API密钥是否过期
        if let Some(expires_at_str) = row.try_get::<Option<String>, _>("expires_at").ok().flatten() {
            if let Some(expires_at) = self.parse_datetime(&expires_at_str) {
                if expires_at < chrono::Utc::now() {
                    warn!("API密钥已过期: {}", expires_at);
                    return Err(AuthError::ExpiredApiKey);
                }
            }
        }

        // 检查用户订阅是否过期
        if let Some(subscription_expires_at_str) = row.try_get::<Option<String>, _>("subscription_expires_at").ok().flatten() {
            if let Some(subscription_expires_at) = self.parse_datetime(&subscription_expires_at_str) {
                if subscription_expires_at < chrono::Utc::now() {
                    warn!("用户订阅已过期: {}", subscription_expires_at);
                    return Err(AuthError::SubscriptionExpired);
                }
            }
        }

        // 构建认证结果
        let user_info = UserInfo {
            id: row.try_get::<i64, _>("user_id").unwrap_or(0) as u32,
            username: row.try_get("username").unwrap_or_default(),
            email: row.try_get("email").unwrap_or_default(),
            subscription_level: row.try_get("subscription_level").unwrap_or_default(),
            subscription_expires_at: row.try_get("subscription_expires_at").ok(),
        };

        let api_key_info = ApiKeyInfo {
            api_key: row.try_get("api_key").unwrap_or_default(),
            created_at: row.try_get("created_at")
                .unwrap_or_else(|_| chrono::Utc::now())
                .to_rfc3339(),
            expires_at: row.try_get("expires_at").ok(),
            id: row.try_get::<i64, _>("id").unwrap_or(0) as u32,
            is_active: row.try_get("is_active").unwrap_or(false),
            key_name: row.try_get("key_name").unwrap_or_default(),
            last_used_at: row.try_get::<Option<String>, _>("last_used_at").ok().flatten(),
            user: user_info.clone(),
        };

        // 更新最后使用时间
        let _ = self.update_last_used_time(api_key).await;

        Ok(AuthResult {
            user_info,
            api_key_info,
        })
    }

    /// 使用外部服务验证API Key
    async fn verify_with_external_service(&self, api_key: &str) -> Result<AuthResult, AuthError> {
        // 模拟响应：当API key为 "123" 时返回模拟数据
        if api_key == "123" {
            info!("使用模拟响应验证API Key: {}", api_key);

            let mock_user_info = UserInfo {
                id: 1,
                username: "test_user".to_string(),
                email: "test@example.com".to_string(),
                subscription_level: "pro".to_string(),
                subscription_expires_at: Some("2026-12-31T23:59:59Z".to_string()),
            };

            let mock_api_key_info = ApiKeyInfo {
                api_key: "tk_mock123456789".to_string(),
                created_at: "2024-01-01T00:00:00Z".to_string(),
                expires_at: Some("2026-12-31T23:59:59Z".to_string()),
                id: 1,
                is_active: true,
                key_name: "Test Mock API Key".to_string(),
                last_used_at: None,
                user: mock_user_info.clone(),
            };

            return Ok(AuthResult {
                user_info: mock_user_info,
                api_key_info: mock_api_key_info,
            });
        }

        let url = format!("{}/api/v1/apikey_info", self.api_url);
        debug!("验证API Key: {}", &api_key[..api_key.len().min(8)]);

        let response = self
            .client
            .get(&url)
            .header("Accept", "application/json")
            .header("Authorization", format!("Bearer {}", api_key))
            .send()
            .await
            .map_err(|e| AuthError::RequestError(e.to_string()))?;

        let status = response.status();
        debug!("鉴权API响应状态: {}", status);

        if !status.is_success() {
            match status.as_u16() {
                401 => return Err(AuthError::InvalidApiKey),
                403 => return Err(AuthError::InactiveApiKey),
                404 => return Err(AuthError::InvalidApiKey),
                _ => return Err(AuthError::HttpError(status.as_u16())),
            }
        }

        let api_key_info: ApiKeyInfo = response
            .json()
            .await
            .map_err(|e| AuthError::JsonError(e.to_string()))?;

        // 验证API Key状态
        if !api_key_info.is_active {
            warn!("API Key未激活: {}", api_key_info.key_name);
            return Err(AuthError::InactiveApiKey);
        }

        // 检查API Key是否过期
        if let Some(expires_at_str) = &api_key_info.expires_at {
            if let Some(expires_at) = self.parse_datetime(expires_at_str) {
                if expires_at < chrono::Utc::now() {
                    warn!("API Key已过期: {} (过期时间: {})",
                          api_key_info.key_name, expires_at_str);
                    return Err(AuthError::ExpiredApiKey);
                }
            }
        }

        // 检查用户订阅是否过期
        if let Some(sub_expires_at_str) = &api_key_info.user.subscription_expires_at {
            if let Some(subscription_expires_at) = self.parse_datetime(sub_expires_at_str) {
                if subscription_expires_at < chrono::Utc::now() {
                    warn!("用户订阅已过期: {} (用户: {}, 过期时间: {})",
                          api_key_info.key_name,
                          api_key_info.user.username,
                          sub_expires_at_str);
                    return Err(AuthError::SubscriptionExpired);
                }
            }
        }

        info!("API Key验证成功: {} (用户: {}, 订阅级别: {})",
              api_key_info.key_name,
              api_key_info.user.username,
              api_key_info.user.subscription_level);

        Ok(AuthResult {
            user_info: api_key_info.user.clone(),
            api_key_info,
        })
    }

    /// 保存认证结果到本地缓存
    async fn save_to_cache(&self, auth_result: &AuthResult, api_key: &str) -> Result<(), sqlx::Error> {
        let mut tx = self.database.pool().begin().await?;

        // 检查用户是否已存在
        let user_sql = "SELECT id FROM users WHERE username = $1";
        let existing_user = sqlx::query(user_sql)
            .bind(&auth_result.user_info.username)
            .fetch_optional(&mut *tx)
            .await?;

        let user_id = if let Some(user_row) = existing_user {
            // 用户已存在，更新用户信息
            // 解析 subscription_expires_at 字符串为 DateTime
            let subscription_expires: Option<chrono::DateTime<chrono::Utc>> = auth_result
                .user_info
                .subscription_expires_at
                .as_ref()
                .and_then(|s| self.parse_datetime(s));
            
            let update_sql = "
                UPDATE users SET
                    email = $1,
                    subscription_level = $2,
                    subscription_expires_at = $3,
                    updated_at = $4
                WHERE id = $5
            ";
            sqlx::query(update_sql)
                .bind(&auth_result.user_info.email)
                .bind(&auth_result.user_info.subscription_level)
                .bind(subscription_expires)
                .bind(chrono::Utc::now())
                .bind(user_row.try_get::<i64, _>("id").unwrap_or(0))
                .execute(&mut *tx)
                .await?;

            user_row.try_get::<i64, _>("id").unwrap_or(0)
        } else {
            // 创建新用户
            // 解析 subscription_expires_at 字符串为 DateTime
            let subscription_expires: Option<chrono::DateTime<chrono::Utc>> = auth_result
                .user_info
                .subscription_expires_at
                .as_ref()
                .and_then(|s| self.parse_datetime(s));
            
            let insert_sql = "
                INSERT INTO users (username, email, subscription_level, subscription_expires_at, created_at, updated_at)
                VALUES ($1, $2, $3, $4, $5, $6)
                RETURNING id
            ";
            let result = sqlx::query(insert_sql)
                .bind(&auth_result.user_info.username)
                .bind(&auth_result.user_info.email)
                .bind(&auth_result.user_info.subscription_level)
                .bind(subscription_expires)
                .bind(chrono::Utc::now())
                .bind(chrono::Utc::now())
                .fetch_one(&mut *tx)
                .await?;

            result.try_get::<i64, _>("id").unwrap_or(0)
        };

        // 检查API密钥是否已存在
        let key_sql = "SELECT id FROM api_keys WHERE api_key = $1";
        let existing_key = sqlx::query(key_sql)
            .bind(api_key)
            .fetch_optional(&mut *tx)
            .await?;

        if let Some(key_row) = existing_key {
            // API密钥已存在，更新信息
            // 解析 expires_at 字符串为 DateTime
            let expires_at: Option<chrono::DateTime<chrono::Utc>> = auth_result
                .api_key_info
                .expires_at
                .as_ref()
                .and_then(|s| self.parse_datetime(s));
            
            let update_key_sql = "
                UPDATE api_keys SET
                    user_id = $1,
                    key_name = $2,
                    is_active = $3,
                    expires_at = $4,
                    last_used_at = $5,
                    updated_at = $6
                WHERE id = $7
            ";
            sqlx::query(update_key_sql)
                .bind(user_id)
                .bind(&auth_result.api_key_info.key_name)
                .bind(auth_result.api_key_info.is_active)
                .bind(expires_at)
                .bind(Some(chrono::Utc::now()))
                .bind(chrono::Utc::now())
                .bind(key_row.try_get::<i64, _>("id").unwrap_or(0))
                .execute(&mut *tx)
                .await?;
        } else {
            // 创建新API密钥
            // 解析 expires_at 字符串为 DateTime
            let expires_at: Option<chrono::DateTime<chrono::Utc>> = auth_result
                .api_key_info
                .expires_at
                .as_ref()
                .and_then(|s| self.parse_datetime(s));
            
            let insert_key_sql = "
                INSERT INTO api_keys (user_id, api_key, key_name, is_active, expires_at, last_used_at, created_at, updated_at)
                VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
            ";
            sqlx::query(insert_key_sql)
                .bind(user_id)
                .bind(api_key)
                .bind(&auth_result.api_key_info.key_name)
                .bind(auth_result.api_key_info.is_active)
                .bind(expires_at)
                .bind(Some(chrono::Utc::now()))
                .bind(chrono::Utc::now())
                .bind(chrono::Utc::now())
                .execute(&mut *tx)
                .await?;
        }

        tx.commit().await?;
        Ok(())
    }

    /// 更新API密钥最后使用时间
    async fn update_last_used_time(&self, api_key: &str) -> Result<(), sqlx::Error> {
        let sql = "UPDATE api_keys SET last_used_at = $1 WHERE api_key = $2";

        sqlx::query(sql)
            .bind(chrono::Utc::now())
            .bind(api_key)
            .execute(self.database.pool())
            .await?;

        Ok(())
    }

    /// 解析时间字符串为DateTime
    fn parse_datetime(&self, datetime_str: &str) -> Option<chrono::DateTime<chrono::Utc>> {
        chrono::DateTime::parse_from_rfc3339(datetime_str)
            .ok()
            .map(|dt| dt.with_timezone(&chrono::Utc))
    }

    /// 获取客户端配置信息
    pub fn get_config(&self) -> AuthClientConfig {
        AuthClientConfig {
            api_url: self.api_url.clone(),
            timeout: Duration::from_secs(10),
        }
    }
}

/// 鉴权客户端配置
#[derive(Debug, Clone)]
pub struct AuthClientConfig {
    /// API服务地址
    pub api_url: String,
    /// 请求超时时间
    pub timeout: Duration,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_datetime_parsing() {
        // 创建一个虚拟的数据库连接用于测试
        let db = crate::server::database::DatabaseConnection::new();
        let client = AuthClient::new(None, db);

        let valid_datetime = "2024-12-31T23:59:59Z";
        let parsed = client.parse_datetime(valid_datetime);
        assert!(parsed.is_some());

        let invalid_datetime = "invalid-date";
        let parsed = client.parse_datetime(invalid_datetime);
        assert!(parsed.is_none());
    }
}