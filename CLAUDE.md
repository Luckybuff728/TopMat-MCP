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

# Health check endpoint
curl http://localhost:10007/health
```

**Note**: Dockerfile uses Debian Bookworm slim base image for optimal size with multi-stage builds.

## Project Structure

### Core Architecture
```
TopMat-LLM/
├── src/
│   ├── main.rs                    # Application entry point with server initialization
│   ├── docs.rs                    # OpenAPI/Swagger documentation configuration
│   └── server/                    # Main server module
│       ├── server.rs              # Router setup and middleware configuration
│       ├── models.rs              # Core data structures and API models
│       ├── handlers/              # HTTP request handlers for different endpoints
│       ├── middleware/            # Authentication, logging, and MCP-specific middleware
│       ├── database/              # SQLite database connection and models
│       ├── agent/                 # AI model provider implementations
│       │   └── examples/          # 96+ comprehensive agent examples
│       └── mcp/                   # Model Context Protocol server
│           ├── tool_registry.rs   # Runtime tool registration and management
│           ├── tool_macros.rs     # Compile-time macro-based tool registration
│           └── tools/             # Domain-specific material science tools
├── rig/                           # Vendored rig-core workspace with RMCP support
├── docs/                          # Additional documentation and design specs
└── docker-compose.yml             # Production deployment configuration
```

### Configuration Files
- **`.env`**: Environment variables for API keys and server settings (create locally)
- **`Cargo.toml`**: Rust dependencies including RMCP v0.8, Axum v0.8, SQLx v0.7
- **`docker-compose.yml`**: Production deployment with port mapping 10007:3000

## Development Workflow

### Hot Reload Development
```bash
# Development with auto-restart (recommended for development)
cargo install cargo-watch
cargo watch -x run
```

### Quality Assurance
```bash
# Full quality check chain
cargo fmt && cargo clippy && cargo test

# Development with quality checks
cargo watch -x "fmt && clippy && test"
```

## Architecture Overview

### Core Technology Stack
- **Web Framework**: Axum v0.8 with Tokio async runtime
- **Database**: SQLite with SQLx v0.7 for async database access
- **LLM Framework**: rig-core v0.23.1 (vendored locally) with RMCP v0.8 support
- **MCP Transport**: StreamableHTTP, SSE, and client-server transports
- **Authentication**: External API service integration with MCP-specific auth
- **Serialization**: Serde for JSON handling
- **Streaming**: SSE (Server-Sent Events) for real-time responses
- **Rust Edition**: 2024

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

##### Database Schema
Database migrations are handled in `src/server/database/connection.rs`. Core tables:
- **users**: User management with subscription levels
- **api_keys**: API key management with expiration tracking
- **conversations**: Conversation metadata and management
- **messages**: Chat message storage with role support
- **usage_statistics**: Token usage and cost tracking
- **mcp_sessions**: MCP session tracking with transport types and client info
- **mcp_tool_calls**: Detailed MCP tool execution records with performance metrics

#### Key Configuration
- **Rust Edition**: 2024 with modern async/await patterns
- **Default Model**: `qwen3:4b` (configurable via environment)
- **Database**: SQLite with async SQLx v0.7
- **Documentation**: Auto-generated OpenAPI with Swagger UI at `/swagger-ui`

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

#### Tool Architecture
- **Static Registration**: Compile-time tool registration using `register_mcp_tools!` macro
- **Runtime Discovery**: Dynamic tool discovery and management via `ToolRegistry` singleton
- **Type Safety**: Compile-time validation of tool signatures and parameters
- **Error Handling**: Comprehensive error tracking and logging with execution metrics

### AI Provider Support

#### Supported Models
- **Qwen (Alibaba)**: qwen-plus, qwen-turbo, qwen-max, qwen-flash, qwq-plus
- **Ollama**: Local models like qwen3:4b, llama3
- **Specialized Agents**: Coating optimization and material science focused agents
- **Extensible**: Easy to add new providers via agent pattern

#### Agent Examples Collection
The project includes an extensive collection of 96 agent examples in `src/server/agent/examples/` demonstrating:
- **Multiple Providers**: OpenAI, Anthropic, Gemini, Groq, Cohere, Together, Hyperbolic, etc.
- **Advanced Patterns**: Multi-agent systems, tool orchestration, RAG, autonomous agents
- **Streaming**: Real-time streaming responses with various providers
- **Domain-Specific**: PDF processing, image generation, audio transcription, embeddings
- **Tool Usage**: Complex tool chains and agent workflows
- **Enterprise Features**: OpenTelemetry integration, request hooks, custom evaluators

#### Agent Implementation Pattern
Each AI provider implements the standard handler function signature in `src/server/agent/`:
```rust
pub async fn provider_model_with_response(
    request: ChatRequest,
) -> Result<(Response, ChatResponse), ErrorResponse>
```

#### Agent Examples Collection
The project includes 96+ comprehensive examples in `src/server/agent/examples/`:
- **Basic examples**: `qwen_basic.rs`, `openai_streaming.rs`, `anthropic_streaming.rs`
- **Tool integration**: `agent_with_tools.rs`, `qwen_tools.rs`, `rmcp.rs`
- **Advanced patterns**: `multi_agent.rs`, `rag.rs`, `vector_search.rs`
- **Domain-specific**: `pdf_agent.rs`, `image_generation.rs`, `transcription.rs`

Run examples with: `cargo run --example example_name`

## Configuration

### Environment Variables
Create a `.env` file in the project root:

```bash
# Server (optional)
SERVER_HOST=127.0.0.1
SERVER_PORT=3000
RUST_LOG=info

# Database (optional)
DATABASE_URL=sqlite:data.db

# Authentication (optional)
AUTH_API_URL=https://api.topmaterial-tech.com

# AI Provider API Keys (at least one required)
DASHSCOPE_API_KEY=your_dashscope_key     # Required for Qwen models
OLLAMA_BASE_URL=http://localhost:11434   # Required for Ollama models
OPENAI_API_KEY=your_openai_key          # Optional: OpenAI models
ANTHROPIC_API_KEY=your_anthropic_key    # Optional: Claude models

# MCP Server Configuration (optional)
MCP_SERVER_URL=http://127.0.0.1:10001/mcp  # External MCP server URL
MCP_API_KEY=your_mcp_api_key               # MCP server authentication

# Docker Environment (for containerized deployment)
TZ=Asia/Shanghai
SERVER_HOST=0.0.0.0                        # For Docker containers
DATABASE_URL=sqlite:/app/data/data.db      # Docker data path
```

### Minimum Working Configuration
Only these are required to get started:
```bash
DASHSCOPE_API_KEY=your_qwen_api_key
DATABASE_URL=sqlite:data.db
```

### Default Configuration
- **Default model**: `qwen3:4b`
- **Default database**: `sqlite:data.db` (creates automatically)
- **Default server**: `http://127.0.0.1:3000`
- **Docker port mapping**: `10007:3000`
- **API Documentation**: Available at `/swagger-ui` after server starts

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
- `GET /usage/mcp/stats` - MCP usage statistics and analytics
- `GET /usage/mcp/sessions` - MCP session history with pagination
- `GET /usage/mcp/tool-calls` - MCP tool execution records
- `GET /usage/comprehensive` - Combined chat and MCP statistics

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
- **Version**: v0.23.1 (local development version)
- **Updates**: Workspace is committed alongside main project

## Testing

### Running Tests
```bash
# Run all tests
cargo test

# Run with output
cargo test -- --nocapture

# Run specific test
cargo test test_name

# Run tests in parallel
cargo test --release

# Run with coverage
cargo tarpaulin --out Html
```

### Testing Strategy
- **Unit Tests**: Located in individual modules with `#[cfg(test)]` sections
- **Integration Tests**: Use the server's REST API endpoints and MCP protocols
- **Example Validation**: 96+ agent examples serve as comprehensive integration tests
- **Manual Testing**: Use Swagger UI at `/swagger-ui` or provided client examples

### Unit Test Pattern
```rust
#[cfg(test)]
mod tests {
    #[tokio::test]
    async fn test_handler() {
        // Test implementation with async support
    }
}
```

### Agent Examples as Tests
The extensive example collection serves as living documentation and tests:
```bash
# Test basic functionality
cargo run --example qwen_basic
cargo run --example agent_with_tools
cargo run --example rmcp

# Test streaming and advanced features
cargo run --example openai_streaming_with_tools
cargo run --example multi_agent
cargo run --example rag
```

### API Testing with Swagger UI
After starting the server, visit `http://localhost:3000/swagger-ui` for:
- Interactive API documentation
- Live API testing with authentication
- Request/response validation
- Schema exploration

## MCP Integration

The server includes a sophisticated MCP (Model Context Protocol) implementation with:

### Core Features
- **StreamableHTTP Transport**: High-performance bidirectional communication
- **SSE Support**: Server-sent events for real-time updates at `/sse/` endpoint
- **Automatic Tool Registration**: Compile-time macro-based registration system
- **Session Management**: Local session state with user context injection
- **Custom Authentication**: GET requests for discovery, POST for execution
- **Data Storage**: Comprehensive MCP session and tool call tracking with analytics
- **Performance Monitoring**: Execution time tracking and success rate analytics

### Architecture Flow
1. **Tool Discovery** (GET `/mcp/`): Returns available tools and their schemas
2. **Tool Execution** (POST `/mcp/`): Executes tools with proper authentication
3. **Session Context**: User information injected via MCP auth middleware
4. **Response Streaming**: Real-time responses via SSE protocol
5. **Data Persistence**: All MCP interactions stored to database for analytics
6. **Performance Tracking**: Execution metrics and success rates recorded

### Material Science Integration
- **Domain-Specific Tools**: Specialized for materials science workflows
- **Simulation Integration**: Direct access to computational tools
- **Data Processing**: ONNX model inference for ML predictions
- **External Services**: Integration with CalphaMesh and Dify platforms

## Security Considerations

### Authentication & Authorization
- **API Key Authentication**: External service integration via `AUTH_API_URL`
- **MCP Authentication**: Dual-mode (GET open for discovery, POST secured for execution)
- **User Context**: Injection of user information into MCP sessions for multi-tenancy

### Development vs Production
- **CORS Policy**: Currently `very_permissive()` for development - should be restricted in production
- **Error Sanitization**: Detailed errors in development, sanitized responses in production
- **Logging**: Configurable log levels via `RUST_LOG` environment variable

### Data Protection
- **Input Validation**: Comprehensive parameter validation and sanitization
- **Database Security**: Connection pooling and prepared statements
- **Tool Sandboxing**: Resource limits and execution time tracking for MCP tools
- **Rate Limiting**: API key-based usage tracking and potential rate limiting

### Production Deployment Notes
- Use reverse proxy (nginx/Apache) for additional security headers
- Configure proper CORS policies for your domain
- Enable HTTPS/TLS in production environments
- Monitor and restrict tool execution resources
- Implement proper backup strategies for SQLite database