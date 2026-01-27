# AGENTS.md

This file provides guidance for agentic coding assistants working in this repository.

## Build, Lint, and Test Commands

### Core Commands
```bash
# Build the project
cargo build                           # Debug build
cargo build --release                 # Optimized release build

# Run the server
cargo run                             # Development mode
cargo run --release                   # Production mode

# Run specific example
cargo run --example example_name

# Format and lint
cargo fmt                             # Format code
cargo clippy                          # Lint with warnings
cargo clippy -- -D warnings           # Treat warnings as errors

# Testing
cargo test                            # Run all tests
cargo test test_name                  # Run specific test
cargo test -- --nocapture             # Show test output
cargo test --release                  # Run tests in release mode

# Hot reload development
cargo install cargo-watch
cargo watch -x run                    # Auto-restart on file changes
cargo watch -x "fmt && clippy && test"  # Format, lint, test on changes
```

### Testing Examples as Integration Tests
The 96+ agent examples in `src/server/agent/examples/` serve as living documentation and integration tests:
```bash
cargo run --example qwen_basic        # Test basic Qwen integration
cargo run --example agent_with_tools  # Test tool orchestration
cargo run --example rmcp              # Test MCP protocol
```

## Code Style Guidelines

### Rust Edition and Language Features
- **Edition**: Rust 2024 with modern async/await patterns
- **Async Runtime**: Tokio with `#[tokio::main]` for main function
- **Database**: SQLx v0.8 with async queries, connection pooling

### Import Organization
```rust
// Standard library first
use std::str::FromStr;

// External crates
use axum::{Router, middleware};
use serde::{Deserialize, Serialize};
use tokio::net::TcpListener;

// Internal modules
use crate::server::models::*;
use crate::server::handlers::chat::ServerState;
```

### Naming Conventions
- **Structs/Enums**: `PascalCase` (e.g., `ChatRequest`, `ErrorResponse`)
- **Functions/Methods**: `snake_case` (e.g., `handle_chat_request`, `authenticate_request`)
- **Constants**: `SCREAMING_SNAKE_CASE` (e.g., `MAX_CONNECTIONS`)
- **Modules**: `snake_case` (e.g., `auth`, `middleware`)
- **Database Tables**: `snake_case` (e.g., `api_keys`, `conversations`)
- **Database Columns**: `snake_case` (e.g., `user_id`, `created_at`)

### Types and Serialization
- Use `#[derive(Debug, Serialize, Deserialize, Clone)]` for API models
- Use `#[serde(skip_serializing_if = "Option::is_none")]` for optional fields
- Use `#[serde(default = "default_function")]` for default values
- Use `utoipa::ToSchema` for OpenAPI documentation

```rust
#[derive(Debug, Serialize, Deserialize, Clone, ToSchema)]
pub struct ChatRequest {
    pub message: String,
    #[serde(default)]
    pub stream: bool,
    #[serde(default = "default_model")]
    pub model: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub system_prompt: Option<String>,
}

fn default_model() -> String {
    "qwen3:4b".to_string()
}
```

### Error Handling
- Use `Result<T, E>` with custom error types
- Define error types as enums with `Display` and `Error` implementations
- Use `anyhow::Error` for tool implementations
- Use `ErrorResponse` for API responses with consistent format

```rust
#[derive(Debug)]
pub enum AuthError {
    RequestError(String),
    HttpError(u16),
    InvalidApiKey,
}

impl std::fmt::Display for AuthError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AuthError::InvalidApiKey => write!(f, "Invalid API key"),
            AuthError::RequestError(msg) => write!(f, "Request failed: {}", msg),
        }
    }
}

impl std::error::Error for AuthError {}
```

### Handler Pattern
All handlers follow this signature:
```rust
pub async fn handler_name(
    Extension(auth_user): Extension<AuthUser>,
    State(state): State<ServerState>,
    Json(request): Json<RequestType>,
) -> Result<impl IntoResponse, ErrorResponse>
```

### AI Provider Pattern
Add new providers to `src/server/agent/` with this signature:
```rust
pub async fn provider_model(
    request: ChatRequest,
    _auth_user: AuthUser,
) -> Result<(axum::response::Response, ChatResponse), ErrorResponse> {
    let agent = client.agent(model).build();
    handle_chat_request(agent, request).await
}
```

### MCP Tool Pattern
Implement the `rig::tool::Tool` trait for MCP tools:
```rust
pub struct MyTool;

impl Tool for MyTool {
    type Error = anyhow::Error;
    type Args = MyToolArgs;
    type Output = serde_json::Value;

    async fn definition(&self, _prompt: String) -> ToolDefinition {
        ToolDefinition {
            name: "tool_name".to_string(),
            description: "Tool description".to_string(),
            parameters: json!({}),
        }
    }

    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        // Tool implementation
    }
}
```

### Testing
- Use `#[cfg(test)]` for test modules
- Use `#[tokio::test]` for async tests
- Write descriptive test names
- Include integration tests for endpoints

```rust
#[cfg(test)]
mod tests {
    #[tokio::test]
    async fn test_handler() {
        let result = function_under_test().await;
        assert!(result.is_ok());
    }
}
```

### Comments and Documentation
- Use `///` for public API documentation
- Use `//!` for module-level documentation
- Add comments in Chinese for user-facing messages
- Keep implementation comments concise

### Database Patterns
- Use `sqlx::query!()` for type-safe queries
- Use `PgPoolOptions` for connection pooling
- Migrations are handled in `src/server/database/connection.rs`
- Always use parameterized queries to prevent SQL injection

```rust
let result = sqlx::query!(
    "SELECT * FROM users WHERE id = $1",
    user_id
)
.fetch_one(&pool)
.await?;
```

### Middleware Ordering
Layers execute from bottom to top (last applied executes first):
```rust
let route = Router::new()
    .route("/endpoint", handler)
    .layer(middleware3)  // Executes first
    .layer(middleware2)  // Executes second
    .layer(middleware1); // Executes last
```

### Configuration
- Environment variables loaded via `dotenvy::dotenv()`
- Use `std::env::var()` with `unwrap_or_else()` for defaults
- Minimum required: `DASHSCOPE_API_KEY` and `DATABASE_URL`

### No Comments Policy
IMPORTANT: Do not add comments to code unless explicitly requested by the user.
