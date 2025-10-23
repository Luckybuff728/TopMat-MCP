use std::str::FromStr;
mod server;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 加载 .env 文件中的环境变量
    dotenvy::dotenv().ok();

    // 读取日志级别配置
    let log_level = std::env::var("RUST_LOG")
        .unwrap_or_else(|_| "info".to_string())
        .parse()
        .unwrap_or(tracing::Level::INFO);

    // 初始化日志
    tracing_subscriber::fmt()
        .with_max_level(log_level)
        .init();

    tracing::info!("启动TopMat LLM 服务...");

    // 读取服务器配置
    let host = std::env::var("SERVER_HOST").unwrap_or_else(|_| "127.0.0.1".to_string());
    let port = std::env::var("SERVER_PORT")
        .unwrap_or_else(|_| "3000".to_string())
        .parse()
        .unwrap_or(3000);

    // 创建路由
    let app = server::create_server();

    // 配置地址
    let addr_str = format!("{}:{}", host, port);
    let addr = std::net::SocketAddr::from_str(&addr_str)
        .map_err(|e| format!("无效的服务器地址 {}: {}", addr_str, e))?;
    tracing::info!("服务器监听地址: http://{}", addr);

    // 启动服务器
    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}