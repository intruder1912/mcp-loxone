//! Hello World MCP Server Example
//!
//! This demonstrates the minimal code needed to create an MCP server
//! using the mcp-framework macros. It implements a simple "say_hello" tool.

use pulseengine_mcp_macros::{mcp_server, mcp_tools};
use pulseengine_mcp_server::McpServerBuilder;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use tracing::info;
use tracing_subscriber::EnvFilter;

/// Hello World MCP Server using macros
#[mcp_server(
    name = "Hello World MCP Server",
    description = "A simple hello world example server"
)]
#[derive(Clone, Default)]
pub struct HelloWorldServer {
    greeting_count: Arc<AtomicU64>,
}

#[mcp_tools]
impl HelloWorldServer {
    /// Say hello to someone or something
    ///
    /// # Parameters
    /// - name: The name to greet (required)
    /// - greeting: Custom greeting (optional, defaults to "Hello")
    #[tool(description = "Say hello to someone or something")]
    pub async fn say_hello(
        &self,
        #[doc = "The name to greet"] name: String,
        #[doc = "Custom greeting (optional)"] greeting: Option<String>,
    ) -> Result<String, String> {
        let greeting = greeting.unwrap_or_else(|| "Hello".to_string());

        // Increment greeting counter
        self.greeting_count.fetch_add(1, Ordering::Relaxed);

        Ok(format!("{greeting}, {name}! 👋"))
    }

    /// Get the total number of greetings sent
    #[tool(description = "Get the total number of greetings sent")]
    pub async fn count_greetings(&self) -> Result<String, String> {
        let count = self.greeting_count.load(Ordering::Relaxed);
        Ok(format!("Total greetings sent: {count}"))
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::try_from_default_env().unwrap_or_else(|_| {
            EnvFilter::new("hello_world_mcp=debug,pulseengine_mcp_server=debug")
        }))
        .init();

    info!("🚀 Starting Hello World MCP Server");

    // Create server with defaults
    let server = HelloWorldServer::default();

    info!("✅ Hello World MCP Server initialized");
    info!("💡 Available tools: say_hello, count_greetings");
    info!("🔗 Connect using any MCP client via stdio");

    // Run server on stdio
    let mut mcp_server = server.serve_stdio().await?;
    mcp_server.run().await?;

    info!("👋 Hello World MCP Server stopped");
    Ok(())
}
