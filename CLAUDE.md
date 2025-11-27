# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

**TopMat-LLM** is a unified LLM chat server built in Rust that provides standardized REST API interfaces for multiple AI model providers. It features conversation management, data persistence, real-time monitoring, and MCP (Model Context Protocol) support with specialized material science tools and domain-specific capabilities.

## Development Commands

### Building and Running
```bash
# Development mode
cargo run

# Production mode
cargo run --release

# Run specific binary
cargo run --bin TopMat-LLM

# Build for release
cargo build --release

# Development with auto-restart
cargo watch -x run
```

### Testing and Quality
```bash
# Run all tests
cargo test

# Run specific test
cargo test test_name

# Format code
cargo fmt

# Code linting
cargo clippy

# Security audit
cargo audit

# Generate documentation
cargo doc --open

# Check code coverage
cargo tarpaulin --out Html
```

### Docker Operations
```bash
# Build Docker image (multi-stage, optimized)
docker build -t 192.168.7.102:5000/topmat-llm:latest .

# Run with Docker Compose (port 10007:3000)
docker-compose up -d

# Check container logs
docker-compose logs -f topmat-llm
```

## Architecture Overview

### Core Technology Stack
- **Web Framework**: Axum v0.8 with Tokio async runtime
- **Database**: SQLite with SQLx v0.7 for async database access
- **LLM Framework**: rig-core v0.21.0 (vendored locally) with RMCP v0.8 support
- **MCP Transport**: StreamableHTTP, SSE, and client-server transports
- **Authentication**: External API service integration with MCP-specific auth
- **Serialization**: Serde for JSON handling
- **Streaming**: SSE (Server-Sent Events) for real-time responses

### Key Components

#### Server Structure (`src/server/`)
- **`server.rs`**: Main server configuration and routing setup
- **`handlers/`**: HTTP request handlers for different endpoints
- **`middleware/`**: Authentication, logging, and MCP-specific middleware
- **`models.rs`**: Core data structures for requests/responses
- **`database/`**: Database connection, models, and migrations
- **`agent/`**: AI model provider implementations and extensive examples
- **`mcp/`**: Model Context Protocol server with automatic tool registration
  - **`tool_registry.rs`**: Runtime tool registration and management
  - **`tool_macros.rs`**: Compile-time macro-based tool registration
  - **`tools/`**: Domain-specific material science and computational tools

#### Request Flow
1. HTTP Request → CORS Layer → Authentication Middleware
2. Route to appropriate handler (REST API or MCP)
3. Handler processes request → calls AI agent or MCP tool
4. Response (streaming or non-streaming) → Client

#### Database Schema
- **users**: User management with subscription levels
- **api_keys**: API key management with expiration tracking
- **conversations**: Conversation metadata and management
- **messages**: Chat message storage with role support
- **usage_statistics**: Token usage and cost tracking

### MCP Tool System

The project features a sophisticated MCP (Model Context Protocol) tool system with automatic registration and specialized domain tools:

#### Tool Registration System
- **Compile-time Macros**: Automatic tool registration using `register_mcp_tools!` macro
- **Runtime Registry**: Dynamic tool discovery and management via `ToolRegistry`
- **Type Safety**: Compile-time validation of tool signatures and parameters

#### Available Domain Tools
- **`think.rs`**: Internal reasoning and thinking capabilities
- **`simulation.rs`**: Material science simulations (TopPhiSimulator, ML Performance Predictor)
- **`calphaMesh.rs`**: CalphaMesh integration (point/line/Scheil tasks, task management)
- **`onnx_service.rs`**: ONNX model inference (model loading, inference, health checks)
- **`dify.rs`**: Dify platform integration (steel RAG, cemented carbide RAG, Al IDME workflows)
- **`phase_field.rs`**: Phase field simulations (spinodal decomposition, PVD simulation)

#### MCP Authentication Pattern
- **GET requests**: No authentication required (tool discovery)
- **POST requests**: Authentication required (tool execution)
- **Separate auth flow** from main REST API with dedicated middleware

### AI Provider Support

#### Supported Models
- **Qwen (Alibaba)**: qwen-plus, qwen-turbo, qwen-max, qwen-flash, qwq-plus
- **Ollama**: Local models like qwen3:4b, llama3
- **Specialized Agents**: Coating optimization and material science focused agents
- **Extensible**: Easy to add new providers via agent pattern

#### Agent Examples Collection
The project includes an extensive collection of 80+ agent examples in `src/server/agent/examples/` demonstrating:
- **Multiple Providers**: OpenAI, Anthropic, Gemini, Groq, Cohere, Together, Hyperbolic, etc.
- **Advanced Patterns**: Multi-agent systems, tool orchestration, RAG, autonomous agents
- **Streaming**: Real-time streaming responses with various providers
- **Domain-Specific**: PDF processing, image generation, audio transcription, embeddings
- **Tool Usage**: Complex tool chains and agent workflows
- **Enterprise Features**: OpenTelemetry integration, request hooks, custom evaluators

#### Agent Implementation Pattern
Each AI provider implements the same interface pattern in `src/server/agent/`:
```rust
pub async fn provider_model_with_response(
    request: ChatRequest,
) -> Result<(Response, ChatResponse), ErrorResponse>
```

## Configuration

### Environment Variables
```bash
# Server (optional)
SERVER_HOST=127.0.0.1
SERVER_PORT=3000
RUST_LOG=info

# Database (optional)
DATABASE_URL=sqlite:data.db

# Authentication (optional)
AUTH_API_URL=https://api.topmaterial-tech.com

# AI Provider API Keys
DASHSCOPE_API_KEY=your_dashscope_key  # Required for Qwen
OLLAMA_BASE_URL=http://localhost:11434  # Required for Ollama

# Docker Environment
TZ=Asia/Shanghai
SERVER_HOST=0.0.0.0  # For Docker containers
DATABASE_URL=sqlite:/app/data/data.db  # Docker data path
```

### Default Configuration
- Default model: `qwen3:4b`
- Default database: `sqlite:data.db`
- Default server: `http://127.0.0.1:3000`
- Docker container port mapping: `10007:3000`

## API Endpoints

### Public Endpoints
- `GET /health` - Health check
- `GET /v1/models` - List available models
- `POST /v1/auth` - API Key authentication

### Authenticated Endpoints
- `POST /v1/chat` - Main chat endpoint (supports streaming)
- `GET|POST /v1/conversations` - Conversation management
- `GET|PUT|DELETE /v1/conversations/:id` - Specific conversation operations
- `GET /v1/conversations/:id/messages` - Message history
- `GET /usage/stats` - Usage statistics

### MCP Endpoint
- `/mcp/*` - Model Context Protocol server with specialized authentication
  - **GET**: Tool discovery (no authentication required)
  - **POST**: Tool execution (authentication required)
  - Supports SSE transport and StreamableHTTP protocols

## Development Patterns

### Adding New AI Providers

1. Create provider module in `src/server/agent/`
2. Implement the standard handler function signature
3. Register in model router (`model_router.rs`)
4. Add model IDs to the models list handler
5. Reference `src/server/agent/examples/` for implementation patterns

### Adding New MCP Tools

#### Using the Registration Macro
```rust
// In your MCP tool module
use rig::tool::Tool;

register_mcp_tools!(registry,
    MyCustomTool {
        args_type: MyToolArgs,
        constructor: MyCustomTool::new()
    },
);
```

#### Manual Tool Registration
1. Implement `rig::tool::Tool` trait for your tool
2. Add tool to `src/server/mcp/tools/mod.rs`
3. Register in `mcp_server.rs` using `register_mcp_tools!` macro
4. Ensure proper error handling and JSON serialization

#### Tool Development Pattern
```rust
#[derive(Debug, Deserialize)]
pub struct MyToolArgs {
    pub input_param: String,
    pub optional_param: Option<i32>,
}

pub struct MyCustomTool;

impl MyCustomTool {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl Tool for MyCustomTool {
    type Error = anyhow::Error;
    type Args = MyToolArgs;
    type Output = serde_json::Value;

    async fn definition(&self, _scope: String) -> ToolDefinition {
        // Define tool schema
    }

    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        // Implement tool logic
    }
}
```

### Database Migrations
Database schema changes should be handled in `src/server/database/connection.rs`:
```rust
async fn run_migrations(pool: &SqlitePool) -> Result<(), sqlx::Error> {
    // Add migration SQL here
}
```

### Error Handling
All handlers return `Result<impl IntoResponse, ErrorResponse>` with standardized error format:
```json
{
  "error": "error_type",
  "message": "Description",
  "details": {},
  "timestamp": "2024-10-27T12:00:00Z"
}
```

### Streaming Responses
Use SSE format for streaming:
```rust
// Text chunk
data: {"type":"content","text":"Hello","finished":false}

// Final response
data: {"type":"final","response":{...}}
```

## Rig-Core Workspace

The project uses a vendored rig-core workspace in the `rig/` directory:
- **Location**: `rig/rig-core/`
- **Features**: RMCP (Model Context Protocol) support enabled
- **Version**: v0.21.0 (local development version)
- **Updates**: Workspace is committed alongside main project

## Testing

### Unit Tests
```rust
#[cfg(test)]
mod tests {
    #[tokio::test]
    async fn test_handler() {
        // Test implementation
    }
}
```

### MCP Tool Testing
Use the extensive agent examples as reference implementations:
```bash
# Test specific agent examples
cargo run --example qwen_basic
cargo run --example agent_with_tools
cargo run --example rmcp
```

### Integration Testing
The project includes comprehensive API testing scripts in the repository root for validating endpoints and functionality.

## MCP Integration

The server includes a sophisticated MCP (Model Context Protocol) implementation with:

### Core Features
- **StreamableHTTP Transport**: High-performance bidirectional communication
- **SSE Support**: Server-sent events for real-time updates
- **Automatic Tool Registration**: Compile-time macro-based registration system
- **Session Management**: Local session state with user context injection
- **Custom Authentication**: GET requests for discovery, POST for execution

### Architecture Flow
1. **Tool Discovery** (GET `/mcp/`): Returns available tools and their schemas
2. **Tool Execution** (POST `/mcp/`): Executes tools with proper authentication
3. **Session Context**: User information injected via MCP auth middleware
4. **Response Streaming**: Real-time responses via SSE protocol

### Material Science Integration
- **Domain-Specific Tools**: Specialized for materials science workflows
- **Simulation Integration**: Direct access to computational tools
- **Data Processing**: ONNX model inference for ML predictions
- **External Services**: Integration with CalphaMesh and Dify platforms

## Security Considerations

- API Key authentication via external service
- MCP-specific authentication patterns (GET open, POST secured)
- CORS configuration for cross-origin requests
- Input validation and sanitization
- Database connection pooling
- Error information sanitization in production responses
- Tool execution sandboxing and resource limits