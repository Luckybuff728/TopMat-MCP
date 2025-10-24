use rig::prelude::*;
use rig::providers::openai;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, JsonSchema, Serialize)]
/// 表示文档情感的枚举
enum Sentiment {
    Positive,
    Negative,
    Neutral,
}

#[derive(Debug, Deserialize, JsonSchema, Serialize)]
struct DocumentSentiment {
    /// 文档的情感
    sentiment: Sentiment,
}

#[tokio::main]
async fn main() {
    // 创建 OpenAI 客户端
    let openai_client = openai::Client::from_env();

    // 创建提取器
    let data_extractor = openai_client
        .extractor::<DocumentSentiment>("gpt-4")
        .build();

    let sentiment = data_extractor
        .extract("I am happy")
        .await
        .expect("Failed to extract sentiment");

    println!("GPT-4: {sentiment:?}");
}
