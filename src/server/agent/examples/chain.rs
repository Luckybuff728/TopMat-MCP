use rig::prelude::*;

use rig::{
    embeddings::EmbeddingsBuilder,
    parallel,
    pipeline::{self, Op, agent_ops::lookup, passthrough},
    providers::openai::{Client, TEXT_EMBEDDING_ADA_002},
    vector_store::in_memory_store::InMemoryVectorStore,
};

#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
    tracing_subscriber::fmt().init();
    // 创建 OpenAI 客户端
    let openai_client = Client::from_env();
    let embedding_model = openai_client.embedding_model(TEXT_EMBEDDING_ADA_002);

    // 为我们的文档创建嵌入
    let embeddings = EmbeddingsBuilder::new(embedding_model.clone())
        .document("Definition of a *flurbo*: A flurbo is a green alien that lives on cold planets")?
        .document("Definition of a *glarb-glarb*: A glarb-glarb is a ancient tool used by the ancestors of the inhabitants of planet Jiro to farm the land.")?
        .document("Definition of a *linglingdong*: A term used by inhabitants of the far side of the moon to describe humans.")?
        .build()
        .await?;

    // 使用嵌入创建向量存储
    let vector_store = InMemoryVectorStore::from_documents(embeddings);

    // 创建向量存储索引
    let index = vector_store.index(embedding_model);
    let agent = openai_client.agent("gpt-4")
        .preamble("
            You are a dictionary assistant here to assist the user in understanding the meaning of words.
        ")
        .build();

    let chain = pipeline::new()
        // 将并行操作链接到当前链。并行操作将
        // 执行查找操作以从用户提示中检索附加上下文，
        // 同时应用直通操作。后者将允许
        // 我们将初始提示转发到链中的下一个操作。
        .chain(parallel!(
            passthrough::<&str>(),
            lookup::<_, _, String>(index, 1), // 需要指定文档类型
        ))
        // 将"map"操作链接到当前链，这将结合用户
        // 提示与检索到的上下文文档以创建最终提示。
        // 如果在查找操作期间发生错误，我们将记录错误并
        // 简单地返回初始提示。
        .map(|(prompt, maybe_docs)| match maybe_docs {
            Ok(docs) => format!(
                "Non standard word definitions:\n{}\n\n{}",
                docs.into_iter()
                    .map(|(_, _, doc)| doc)
                    .collect::<Vec<_>>()
                    .join("\n"),
                prompt,
            ),
            Err(err) => {
                println!("Error: {err}! Prompting without additional context");
                prompt.to_string()
            }
        })
        // 链接一个"prompt"操作，该操作将使用最终提示提示我们的代理
        .prompt(agent);

    // 提示代理并打印响应
    let response = chain.call("What does \"glarb-glarb\" mean?").await?;
    println!("{response}");

    Ok(())
}
