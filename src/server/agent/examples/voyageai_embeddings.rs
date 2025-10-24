use rig::Embed;
use rig::prelude::*;
use rig::providers::voyageai;

#[derive(Embed, Debug)]
struct Greetings {
    #[embed]
    message: String,
}

#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
    // 初始化 xAI 客户端
    let client = voyageai::Client::from_env();
    let embeddings = client
        .embeddings(voyageai::VOYAGE_3_LARGE)
        .document(Greetings {
            message: "Hello, world!".to_string(),
        })?
        .document(Greetings {
            message: "Goodbye, world!".to_string(),
        })?
        .build()
        .await
        .expect("Failed to embed documents");

    println!("{embeddings:?}");

    Ok(())
}
