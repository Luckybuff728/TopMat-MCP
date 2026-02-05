use crate::server::database::DatabaseConnection;
use crate::server::models::Message as DbMessage;
use rig::message::{AssistantContent, Message as RigMessage, Text, UserContent};

pub struct HistoryConfig {
    pub max_messages: i64,
    pub enable_summarization: bool,
}

impl Default for HistoryConfig {
    fn default() -> Self {
        Self {
            max_messages: 20,
            enable_summarization: false,
        }
    }
}

pub struct HistoryManager {
    db: DatabaseConnection,
    config: HistoryConfig,
}

impl HistoryManager {
    pub fn new(db: DatabaseConnection) -> Self {
        Self {
            db,
            config: HistoryConfig::default(),
        }
    }

    pub fn with_config(db: DatabaseConnection, config: HistoryConfig) -> Self {
        Self { db, config }
    }

    /// 获取并处理对话历史，转换为 Rig 消息格式
    pub async fn get_context(
        &self,
        conversation_id: &str,
    ) -> Result<Vec<RigMessage>, Box<dyn std::error::Error + Send + Sync>> {
        let db_messages = self
            .db
            .get_conversation_history(conversation_id, self.config.max_messages)
            .await?;

        // 这里可以插入总结或压缩逻辑
        let processed_messages = self.process_history(db_messages).await?;

        Ok(self.to_rig_messages(processed_messages))
    }

    /// 对历史消息进行处理（如总结、压缩等）
    async fn process_history(
        &self,
        messages: Vec<DbMessage>,
    ) -> Result<Vec<DbMessage>, Box<dyn std::error::Error + Send + Sync>> {
        // 目前只是简单的透传，未来可以在这里实现总结逻辑
        if self.config.enable_summarization && messages.len() > 10 {
            // TODO: 实现总结逻辑
        }
        Ok(messages)
    }

    /// 将数据库模型转换为 Rig 消息模型
    fn to_rig_messages(&self, db_messages: Vec<DbMessage>) -> Vec<RigMessage> {
        db_messages
            .into_iter()
            .filter_map(|msg| {
                let content_text = msg.content.unwrap_or_default();
                match msg.role.as_str() {
                    "user" => Some(RigMessage::User {
                        content: rig::OneOrMany::one(UserContent::Text(Text {
                            text: content_text,
                        })),
                    }),
                    "assistant" => Some(RigMessage::Assistant {
                        id: None,
                        content: rig::OneOrMany::one(AssistantContent::Text(Text {
                            text: content_text,
                        })),
                    }),
                    "tool" => {
                        let is_agent = msg
                            .metadata
                            .as_ref()
                            .and_then(|m| m.get("is_agent"))
                            .and_then(|v| v.as_bool())
                            .unwrap_or(false);

                        if is_agent {
                            Some(RigMessage::Assistant {
                                id: None,
                                content: rig::OneOrMany::one(AssistantContent::Text(Text {
                                    text: content_text,
                                })),
                            })
                        } else {
                            None
                        }
                    }
                    // "system" 角色通常由 preamble 处理，这里暂不包含在历史中以避免冲突
                    // 或者可以根据需要将其合并到 preamble
                    _ => None,
                }
            })
            .collect()
    }
}
