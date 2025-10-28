# GitHub Copilot Instructions for mcp-loxone

This file provides guidance to GitHub Copilot when assisting with development in this repository.

## 🦀 Project Overview

This is a **Rust-based Model Context Protocol (MCP) server** for Loxone home automation systems. The project provides:

- **17 MCP tools** for device control and state modification
- **25+ MCP resources** for read-only data access with caching
- **Multiple transport modes**: stdio (Claude Desktop), HTTP/SSE (web clients)
- **Enterprise-grade security**: API key authentication, role-based access control
- **Production-ready**: Comprehensive monitoring, metrics, and dashboards
- **Modern architecture**: Async Rust with tokio, WebSocket support, intelligent caching

## 🏗️ Technology Stack

- **Language**: Rust 2021 edition (minimum 1.70)
- **Async Runtime**: Tokio
- **MCP Framework**: PulseEngine MCP (v0.5.0+)
- **Web Framework**: Axum (for HTTP transport)
- **Protocol**: HTTP, WebSocket, Server-Sent Events (SSE)
- **Serialization**: serde, serde_json
- **Target Platforms**: Native binaries, WebAssembly (WASM/WASIP2)

## 📁 Project Structure

```
src/
├── server/          # MCP protocol implementation (10+ files)
├── tools/           # 30+ Loxone device control tools
│   ├── adapters.rs  # Tool implementations
│   ├── lights/      # Lighting control tools
│   ├── climate/     # HVAC and temperature tools
│   ├── audio/       # Audio zone control tools
│   └── ...
├── client/          # HTTP/WebSocket clients (7 files)
├── config/          # Credential management (7 files)
├── security/        # Input validation, CORS, rate limiting
├── performance/     # Monitoring, profiling, metrics
├── monitoring/      # Dashboard, InfluxDB integration
├── history/         # Time-series data storage
├── validation/      # Request/response validation
├── discovery/       # Network device discovery
└── main.rs         # Binary entry points

tests/              # Integration tests
examples/           # Usage examples
docs/               # Additional documentation
```

## 🛠️ Common Development Commands

### Build and Run

```bash
# Build the project
cargo build

# Build optimized release
cargo build --release

# Run the MCP server (stdio mode for Claude Desktop)
cargo run --bin loxone-mcp-server stdio --credential-id <id>

# Run the MCP server (HTTP mode for web clients)
cargo run --bin loxone-mcp-server http --port 3001 --credential-id <id>

# Run with development hot-reload
make dev-run
```

### Credential Management

```bash
# Store Loxone credentials
cargo run --bin loxone-mcp-auth store --name "Home" --host 192.168.1.100 --username admin --password secret

# List stored credentials
cargo run --bin loxone-mcp-auth list

# Test a credential
cargo run --bin loxone-mcp-auth test <credential-id>
```

### Testing and Quality

```bash
# Run all tests
cargo test

# Run tests with output
cargo test -- --nocapture

# Run specific test
cargo test test_name

# Format code (required before commit)
cargo fmt

# Run linter (required before commit)
cargo clippy -- -D warnings

# Security audit
cargo audit

# Run all quality checks at once
make check
```

### WebAssembly (WASM)

```bash
# Build for WASM target
cargo build --target wasm32-wasip2 --release

# Or use make
make wasm

# Test WASM binary
wasmtime target/wasm32-wasip2/release/loxone-mcp-server.wasm
```

## 📝 Code Style Guidelines

### Rust Conventions

- Follow **Rust 2021 edition** idioms and conventions
- Use `cargo fmt` for consistent formatting (enforced in CI)
- Address all `cargo clippy` warnings before committing
- Write comprehensive rustdoc comments for public APIs
- Use meaningful variable and function names

### Error Handling

```rust
// Use the project's error types
use crate::error::{LoxoneError, Result};

fn process_command(cmd: &str) -> Result<()> {
    if cmd.is_empty() {
        return Err(LoxoneError::validation("Command cannot be empty"));
    }
    // Implementation
    Ok(())
}
```

### Async Code

```rust
// Use async/await with tokio
#[tokio::main]
async fn main() -> Result<()> {
    let client = LoxoneClient::new(config).await?;
    let result = client.send_command("uuid", "on").await?;
    Ok(())
}
```

### Testing

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validation() {
        assert!(validate_uuid("valid-uuid").is_ok());
        assert!(validate_uuid("invalid").is_err());
    }

    #[tokio::test]
    async fn test_async_operation() {
        let result = async_function().await;
        assert!(result.is_ok());
    }
}
```

## 🔧 Development Guidelines

### Adding New MCP Tools

1. **Create tool function** in `src/tools/adapters.rs` or relevant module
2. **Define parameters** using `serde` and `schemars` derives
3. **Implement validation** for all inputs (UUIDs, ranges, enums)
4. **Add comprehensive tests** including error cases
5. **Update documentation** in `docs/tools_reference.md`

Example:
```rust
#[derive(Debug, Deserialize, JsonSchema)]
pub struct MyToolParams {
    #[serde(description = "Device UUID")]
    pub uuid: String,
    #[serde(description = "Command value")]
    pub value: f64,
}

pub async fn my_new_tool(
    context: ToolContext,
    params: MyToolParams,
) -> Result<ToolResult> {
    // Validate inputs
    validate_uuid(&params.uuid)?;
    
    // Implementation
    let client = context.client();
    client.send_command(&params.uuid, &params.value.to_string()).await?;
    
    Ok(ToolResult::success("Command sent successfully"))
}
```

### Adding New MCP Resources

Resources are **read-only** data endpoints:
```rust
// Register in src/server/resource_manager.rs
manager.register_resource(
    "loxone://my/resource",
    "Description of the resource",
    Box::new(|context| {
        Box::pin(async move {
            // Fetch and return data
            Ok(ResourceData::new("application/json", json_data))
        })
    })
);
```

### Security Considerations

- **Always validate** user inputs (UUIDs, IP addresses, parameters)
- **Use parameterized queries** where applicable
- **Never log credentials** or sensitive data
- **Rate limit** expensive operations
- **Check permissions** before executing commands
- Follow the principle of **least privilege**

## 🚀 Integration Points

### Loxone Miniserver Communication

The project communicates with Loxone Miniserver using:
- **HTTP API**: Basic commands and authentication
- **WebSocket**: Real-time updates and binary protocol
- **Structure File**: JSON format for device discovery

Key files:
- `src/client/http_client.rs` - HTTP communication
- `src/client/websocket.rs` - WebSocket protocol
- `src/client/structure.rs` - Structure file parsing

### MCP Protocol Implementation

Built on PulseEngine MCP framework:
- Tool registration and execution
- Resource management with caching
- Multiple transport support (stdio, HTTP, SSE)
- Schema generation and validation

Key files:
- `src/server/tool_handler.rs` - Tool execution
- `src/server/resource_manager.rs` - Resource management
- `src/server/transport.rs` - Transport layer

## 📚 Important Files and Documentation

### Configuration Files
- `Cargo.toml` - Project dependencies and metadata
- `rust-toolchain.toml` - Rust version specification
- `Makefile` - Common development tasks
- `.github/workflows/ci.yml` - CI/CD pipeline

### Documentation
- `README.md` - Main project documentation
- `CLAUDE.md` - AI assistant guidance (comprehensive)
- `CONTRIBUTING.md` - Contribution guidelines
- `DEVELOPMENT.md` - Development setup and workflow
- `SECURITY.md` - Security policies and reporting
- `docs/ARCHITECTURE.md` - Architecture deep dive
- `docs/tools_reference.md` - Tool documentation
- `CREDENTIAL_MIGRATION_GUIDE.md` - Migration from env vars to credential IDs

### Key Source Files
- `src/main.rs` - Application entry points
- `src/server/mod.rs` - MCP server implementation
- `src/tools/adapters.rs` - Tool implementations
- `src/client/loxone_client.rs` - Main Loxone client
- `src/config/credential_manager.rs` - Credential management

## 🧪 Testing Strategy

### Test Organization
- **Unit tests**: In each module using `#[cfg(test)]`
- **Integration tests**: In `tests/` directory
- **Documentation tests**: In rustdoc comments (auto-tested)

### Running Tests
```bash
# All tests
cargo test

# Library tests only (fast)
cargo test --lib

# Integration tests
cargo test --test '*'

# Specific test file
cargo test --test integration_tests

# With verbose output
cargo test -- --nocapture --test-threads=1
```

### Test Coverage
Aim for:
- **Unit tests**: Core logic and validation functions
- **Integration tests**: End-to-end tool execution
- **Error cases**: Invalid inputs, network failures, auth errors
- **Edge cases**: Boundary conditions, unusual inputs

## 🔍 Common Patterns in the Codebase

### Credential Management (New Pattern)
```rust
// Store credentials with unique ID
let cred_id = CredentialManager::store(credentials)?;

// Retrieve and use
let credentials = CredentialManager::load(&cred_id)?;
let client = LoxoneClient::new(credentials).await?;
```

### Tool Execution Pattern
```rust
pub async fn tool_name(
    context: ToolContext,
    params: ToolParams,
) -> Result<ToolResult> {
    // 1. Validate inputs
    validate_params(&params)?;
    
    // 2. Get client from context
    let client = context.client();
    
    // 3. Execute operation
    let result = client.operation(&params).await?;
    
    // 4. Return structured result
    Ok(ToolResult::success_with_data(result))
}
```

### Resource Caching Pattern
```rust
// Check cache first
if let Some(cached) = cache.get(&key) {
    return Ok(cached);
}

// Fetch from Miniserver
let data = client.fetch_data().await?;

// Cache with TTL
cache.insert(key, data.clone(), Duration::from_secs(60));

Ok(data)
```

## ⚠️ Known Issues and Limitations

- **WASM Support**: Currently disabled due to tokio runtime limitations in WASIP2
- **WebSocket Encryption**: Not yet implemented (uses HTTP Basic Auth only)
- **Device Discovery**: Manual configuration required (no automatic Miniserver detection)
- **Cloud API**: Remote access through Loxone Cloud not implemented

## 🔒 Security Notes

### Credential Storage
- Credentials stored in system-specific secure location:
  - Linux/macOS: `~/.config/loxone-mcp/credentials/`
  - Windows: `%APPDATA%\loxone-mcp\credentials\`
- Files encrypted at rest (planned improvement)
- Never commit credentials to version control

### API Keys (Legacy System)
- JWT-based authentication for HTTP transport
- Role-based access control (admin, operator, viewer)
- Rate limiting per API key
- IP whitelisting support

## 📦 Release and Deployment

### Building Releases
```bash
# Build optimized binary
cargo build --release

# Strip debug symbols
strip target/release/loxone-mcp-server

# Create distributable package
tar czf loxone-mcp-server.tar.gz -C target/release loxone-mcp-server
```

### Docker Deployment
```bash
# Build Docker image
docker build -t loxone-mcp:latest .

# Run container
docker run -p 3001:3001 \
  --env-file .env \
  loxone-mcp:latest
```

## 💡 Tips for Contributors

1. **Read existing code** in similar modules before adding new features
2. **Test locally** with a real Loxone Miniserver when possible
3. **Run quality checks** frequently: `make check`
4. **Keep PRs focused** on a single feature or fix
5. **Update documentation** alongside code changes
6. **Ask questions** via GitHub issues if uncertain

## 🔗 External References

- **Loxone API Documentation**: See `CLAUDE.md` for official Loxone protocol references
- **MCP Specification**: https://spec.modelcontextprotocol.io/
- **PulseEngine MCP Framework**: https://github.com/EmilMarkov/pulseengine-mcp
- **Rust Documentation**: https://doc.rust-lang.org/book/

---

**Note**: This is a Rust project with async/await patterns and WebAssembly support. All development uses Cargo, not Python/npm tools. When in doubt, refer to `CLAUDE.md` for comprehensive project documentation.
