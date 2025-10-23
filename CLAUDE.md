# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

**TopMat-LLM** is a Rust-based unified Large Language Model (LLM) chat server that provides a standardized REST API interface supporting multiple AI providers with both streaming and non-streaming response capabilities.

## Key Architecture

### Layered Architecture Design
The project follows a layered architecture pattern:
- **API Layer**: Axum-based HTTP server with unified `/chat` endpoint
- **Agent Layer**: Abstraction over different AI providers (Qwen, Ollama)
- **Core Layer**: Built on top of `rig-core` LLM framework
- **Configuration Layer**: Environment-based configuration management

### Core Components

- **`src/main.rs`**: Application entry point with server initialization
- **`src/server/chat.rs`**: Main chat handler and request routing logic
- **`src/server/models.rs`**: Data structures for requests/responses
- **`src/server/agent/`**: AI provider implementations (Qwen, Ollama)
- **`rig/rig-core/`**: Local dependency for LLM framework (version 0.21.0)

## Development Commands

### Building and Running
```bash
# Development build
cargo build

# Production build
cargo build --release

# Run the unified server
cargo run

# Run with specific binary
cargo run --bin unified_server
```

### Testing and Quality
```bash
# Run tests
cargo test

# Format code
cargo fmt

# Lint code
cargo clippy

# Generate documentation
cargo doc --open
```

### Testing the API
```bash
# Test the chat API (requires server running)
chmod +x test_unified_chat.sh
./test_unified_chat.sh

# Test individual endpoints
chmod +x test_chat_api.sh
./test_chat_api.sh
```

## Configuration Requirements

### Environment Variables
Create a `.env` file based on `.env.example`:

```bash
# Server configuration (optional)
SERVER_HOST=127.0.0.1
SERVER_PORT=3000
RUST_LOG=info

# Required for Qwen models
DASHSCOPE_API_KEY=your_dashscope_api_key_here

# Optional: Other provider keys
# OPENAI_API_KEY=your_openai_api_key_here
# ANTHROPIC_API_KEY=your_anthropic_api_key_here
```

### Model Support
- **Qwen Models**: `qwen-plus`, `qwen-turbo`, `qwen-max` (requires DASHSCOPE_API_KEY)
- **Ollama Models**: `ollama-qwen3-4b`, `ollama-llama3` (requires Ollama service running)

## Key Implementation Details

### Request Flow
1. HTTP request hits `/chat` endpoint
2. `chat_handler` routes to appropriate AI provider based on model name
3. Provider-specific handlers process streaming vs non-streaming requests
4. Unified response format returned via SSE (streaming) or JSON (non-streaming)

### Streaming Architecture
- Uses Server-Sent Events (SSE) for streaming responses
- `StreamChunk` enum with tagged variants for different content types
- Async streaming with `axum::response::sse::Event` handling

### Error Handling
- Unified `ErrorResponse` structure with HTTP status code mapping
- Provider-specific error types: `qwen_not_configured`, `model_not_supported`, etc.
- Comprehensive logging via `tracing` crate

### Model Abstraction
The codebase uses the `rig-core` framework to provide a unified interface over different LLM providers:
- `rig::providers::qwen::Client` for Qwen models
- `rig::providers::ollama::Client` for Ollama models
- Shared `handle_normal_request` and `handle_streaming_request` functions

## Adding New AI Providers

1. Create new provider file in `src/server/agent/`
2. Implement provider-specific handler functions
3. Add module declaration in `src/server/agent/mod.rs`
4. Update routing logic in `src/server/chat.rs`
5. Follow existing patterns for streaming and non-streaming responses

## Architecture Patterns

### State Management
- `ServerState` struct maintains availability flags for different providers
- Clone-based state sharing across request handlers
- Environment-based provider detection at startup

### Request/Response Models
- Serde-based serialization with conditional field inclusion
- Generic `ChatRequest` with optional parameters
- Tagged `StreamChunk` enum for type-safe streaming responses
- Token usage tracking with `TokenUsage` struct

### Provider Integration
- Each provider implements both streaming and non-streaming handlers
- Shared request processing utilities in `src/server/request.rs`
- Consistent error handling across all providers
- Temperature and system prompt configuration support

## Development Notes

- The project uses Rust 2024 Edition
- Async runtime powered by Tokio
- Web framework: Axum v0.7
- Core LLM functionality: custom `rig-core` dependency
- Configuration management via `dotenvy`
- Comprehensive logging with `tracing`