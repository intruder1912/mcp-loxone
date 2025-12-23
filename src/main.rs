//! Loxone MCP Server - Main Entry Point
//!
//! Uses pulseengine-mcp 0.17.0 framework with macros support.
//!
//! This server supports two modes:
//! - Macro-based: Uses #[mcp_server] and #[mcp_tools] for simplified tool definitions
//! - Legacy: Uses manual McpBackend implementation for complex HTTP setups

use pulseengine_mcp_auth::AuthenticationManager;
use pulseengine_mcp_protocol::{
    ElicitationCapability, FormElicitationCapability, Implementation, PromptsCapability,
    ProtocolVersion, ResourcesCapability, SamplingCapability, SamplingContextCapability,
    SamplingToolsCapability, ServerCapabilities, ServerInfo, ToolsCapability,
    UrlElicitationCapability,
};
use pulseengine_mcp_security::SecurityMiddleware;
use pulseengine_mcp_server::{middleware::MiddlewareStack, GenericServerHandler, McpServerBuilder};
use pulseengine_mcp_transport::{create_transport, Transport};

use loxone_mcp_rust::{
    config::{
        credential_registry::CredentialRegistry, credentials::create_best_credential_manager,
    },
    server::{framework_backend::create_loxone_backend, macro_backend::LoxoneMcpServer},
    Result, ServerConfig as LoxoneServerConfig,
};

use clap::{Parser, Subcommand};
use std::sync::Arc;
use tracing::info;
use tracing_subscriber::{fmt, layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};

/// Loxone MCP Server Configuration
#[derive(Parser, Debug)]
#[command(name = "loxone-mcp-server")]
#[command(about = "High-performance Loxone MCP Server with multi-transport support")]
#[command(version = env!("CARGO_PKG_VERSION"))]
struct Config {
    /// Transport configuration
    #[command(subcommand)]
    transport: TransportCommand,

    /// Enable debug logging
    #[arg(long, global = true)]
    debug: bool,

    /// Loxone Miniserver host
    #[arg(long, global = true, env = "LOXONE_HOST")]
    loxone_host: Option<String>,

    /// Loxone username
    #[arg(long, global = true, env = "LOXONE_USER")]
    loxone_user: Option<String>,

    /// Loxone password
    #[arg(long, global = true, env = "LOXONE_PASS")]
    loxone_password: Option<String>,

    /// Credential ID (from loxone-mcp-auth)
    #[arg(long, global = true)]
    credential_id: Option<String>,
}

#[derive(Subcommand, Debug)]
enum TransportCommand {
    /// Run with stdio transport (Claude Desktop)
    Stdio {
        /// Enable offline mode (no Loxone connection required)
        #[arg(long)]
        offline: bool,
    },
    /// Run with HTTP transport (MCP Inspector, n8n)
    Http {
        /// Port to listen on
        #[arg(short, long, default_value = "3001")]
        port: u16,

        /// Enable SSE support for legacy clients
        #[arg(long)]
        enable_sse: bool,

        /// API key for authentication
        #[arg(long, env = "LOXONE_API_KEY")]
        api_key: Option<String>,

        /// Enable development mode (no auth)
        #[arg(long)]
        dev_mode: bool,

        /// Enable CORS (permissive mode)
        #[arg(long)]
        enable_cors: bool,
    },
    /// Run with streamable HTTP transport (new MCP Inspector)
    StreamableHttp {
        /// Port to listen on
        #[arg(short, long, default_value = "3001")]
        port: u16,

        /// Enable CORS
        #[arg(long)]
        enable_cors: bool,
    },
}

impl Config {
    /// Initialize logging based on debug flag
    fn initialize_logging(&self) {
        let filter = if self.debug {
            EnvFilter::new("debug")
        } else {
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"))
        };

        tracing_subscriber::registry()
            .with(filter)
            .with(fmt::layer().compact())
            .init();
    }

    /// Validate configuration
    fn validate(&self) -> Result<()> {
        let has_credential_id = self.credential_id.is_some();
        let has_direct_credentials = self.loxone_host.is_some()
            && self.loxone_user.is_some()
            && self.loxone_password.is_some();

        match &self.transport {
            TransportCommand::Stdio { offline } => {
                if !offline && !has_credential_id && !has_direct_credentials {
                    return Err(loxone_mcp_rust::LoxoneError::config(
                        "Loxone credentials required. Use --credential-id <id>, set LOXONE_HOST/LOXONE_USER/LOXONE_PASS, or use --offline mode"
                    ));
                }
            }
            TransportCommand::Http { dev_mode, .. } => {
                if !dev_mode && !has_credential_id && !has_direct_credentials {
                    return Err(loxone_mcp_rust::LoxoneError::config(
                        "Loxone credentials required. Use --credential-id <id>, set LOXONE_HOST/LOXONE_USER/LOXONE_PASS, or use --dev-mode"
                    ));
                }
            }
            TransportCommand::StreamableHttp { .. } => {
                if !has_credential_id && !has_direct_credentials {
                    return Err(loxone_mcp_rust::LoxoneError::config(
                        "Loxone credentials required. Use --credential-id <id> or set LOXONE_HOST/LOXONE_USER/LOXONE_PASS"
                    ));
                }
            }
        }
        Ok(())
    }
}

/// Get default server info with 0.17.0 protocol types
#[allow(dead_code)]
fn get_default_server_info() -> ServerInfo {
    ServerInfo {
        protocol_version: ProtocolVersion::default(),
        capabilities: ServerCapabilities {
            tools: Some(ToolsCapability { list_changed: None }),
            resources: Some(ResourcesCapability {
                subscribe: Some(true),
                list_changed: None,
            }),
            prompts: Some(PromptsCapability { list_changed: None }),
            logging: None,
            sampling: Some(SamplingCapability {
                context: Some(SamplingContextCapability {}),
                tools: Some(SamplingToolsCapability {}),
            }),
            elicitation: Some(ElicitationCapability {
                form: Some(FormElicitationCapability {}),
                url: Some(UrlElicitationCapability {}),
            }),
            tasks: None,
        },
        server_info: Implementation {
            name: env!("CARGO_PKG_NAME").to_string(),
            version: env!("CARGO_PKG_VERSION").to_string(),
            description: Some("Loxone home automation MCP server".to_string()),
        },
        instructions: None,
    }
}

/// Load credentials from credential ID
async fn load_credentials_by_id(credential_id: &str) -> Result<(String, String, String)> {
    let registry = CredentialRegistry::load()?;

    let stored = registry.get_credential(credential_id).ok_or_else(|| {
        loxone_mcp_rust::LoxoneError::config(format!(
            "Credential ID '{credential_id}' not found. Use 'loxone-mcp-auth list' to see available credentials"
        ))
    })?;

    let manager = create_best_credential_manager().await?;
    std::env::set_var("LOXONE_HOST", format!("{}:{}", stored.host, stored.port));

    let credentials = manager.get_credentials().await.map_err(|e| {
        loxone_mcp_rust::LoxoneError::config(format!(
            "Failed to load credentials for ID '{credential_id}': {e}"
        ))
    })?;

    let host = format!("{}:{}", stored.host, stored.port);
    Ok((host, credentials.username, credentials.password))
}

/// Try to auto-detect credentials from available credential managers
async fn try_auto_detect_credentials() -> Result<(String, String, String)> {
    let manager = create_best_credential_manager().await?;
    let credentials = manager.get_credentials().await?;

    let host = std::env::var("LOXONE_HOST").map_err(|_| {
        loxone_mcp_rust::LoxoneError::config("LOXONE_HOST environment variable not set".to_string())
    })?;

    Ok((host, credentials.username, credentials.password))
}

#[tokio::main]
async fn main() -> Result<()> {
    let config = Config::parse();

    // Initialize logging
    config.initialize_logging();

    // Validate configuration
    config.validate()?;

    info!(
        "🚀 Starting Loxone MCP Server v{}",
        env!("CARGO_PKG_VERSION")
    );

    // Load credentials with precedence: credential_id > direct args > auto-detect
    let (loxone_host, loxone_user, _loxone_password) = if let Some(credential_id) =
        &config.credential_id
    {
        info!("🔑 Loading credentials from ID: {}", credential_id);
        load_credentials_by_id(credential_id).await?
    } else if config.loxone_host.is_some()
        && config.loxone_user.is_some()
        && config.loxone_password.is_some()
    {
        info!("🔑 Using direct CLI credentials");
        (
            config.loxone_host.clone().unwrap(),
            config.loxone_user.clone().unwrap(),
            config.loxone_password.clone().unwrap(),
        )
    } else {
        info!("🔍 Auto-detecting credentials from available backends...");
        match try_auto_detect_credentials().await {
            Ok((host, user, pass)) => {
                info!("✅ Auto-detected credentials from credential manager");
                (host, user, pass)
            }
            Err(e) => {
                return Err(loxone_mcp_rust::LoxoneError::config(format!(
                        "No credentials available. Please either:\n\
                         1. Use --credential-id <id> (run 'loxone-mcp-auth list' to see available IDs)\n\
                         2. Set --loxone-host, --loxone-user, --loxone-password\n\
                         3. Set environment variables LOXONE_HOST, LOXONE_USER, LOXONE_PASS\n\
                         4. Run 'loxone-mcp-setup' to configure credentials\n\
                         \n\
                         Error details: {e}"
                    )));
            }
        }
    };

    // For ALL stdio modes, use the macro-based server
    if let TransportCommand::Stdio { offline } = &config.transport {
        LoxoneMcpServer::configure_stdio_logging();

        let server = if *offline {
            info!("🚀 Starting macro-based MCP server in offline mode");
            LoxoneMcpServer::with_defaults()
        } else {
            info!("🚀 Starting macro-based MCP server with Loxone connection");

            // Create Loxone client for online mode
            use loxone_mcp_rust::client::{ClientContext, LoxoneHttpClient};
            use loxone_mcp_rust::config::credentials::LoxoneCredentials;
            use loxone_mcp_rust::services::SensorTypeRegistry;

            let loxone_url: url::Url = format!("http://{loxone_host}")
                .parse()
                .map_err(|e| loxone_mcp_rust::LoxoneError::config(format!("Invalid URL: {e}")))?;

            let loxone_cfg = loxone_mcp_rust::config::LoxoneConfig {
                url: loxone_url,
                timeout: std::time::Duration::from_secs(30),
                verify_ssl: false,
                ..Default::default()
            };

            let credentials = LoxoneCredentials {
                username: loxone_user.clone(),
                password: _loxone_password.clone(),
                api_key: None,
                #[cfg(feature = "crypto-openssl")]
                public_key: None,
            };

            let client = LoxoneHttpClient::new(loxone_cfg, credentials)
                .await
                .map_err(|e| {
                    loxone_mcp_rust::LoxoneError::connection(format!(
                        "Failed to create client: {e}"
                    ))
                })?;

            // Get context from the client before wrapping in Arc
            let context = Arc::new(ClientContext::new());
            let client_arc: Arc<dyn loxone_mcp_rust::client::LoxoneClient> = Arc::new(client);

            // Create value resolver with required dependencies
            let sensor_registry = Arc::new(SensorTypeRegistry::new());
            let value_resolver = Arc::new(loxone_mcp_rust::services::UnifiedValueResolver::new(
                client_arc.clone(),
                sensor_registry,
            ));

            info!("✅ Loxone client connected");

            LoxoneMcpServer::with_context(
                client_arc,
                context,
                value_resolver,
                None, // state_manager
                LoxoneServerConfig::default(),
            )
        };

        let mut mcp_server = server.serve_stdio().await.map_err(|e| {
            loxone_mcp_rust::LoxoneError::connection(format!("Failed to start server: {e}"))
        })?;

        info!("✅ Macro-based server started successfully");
        mcp_server
            .run()
            .await
            .map_err(|e| loxone_mcp_rust::LoxoneError::connection(format!("Server error: {e}")))?;

        return Ok(());
    }

    // Create Loxone configuration
    let loxone_config = match &config.transport {
        TransportCommand::Stdio { offline } => {
            if *offline {
                // This branch is now handled above, but kept for completeness
                info!("Running in offline mode - no Loxone connection");
                LoxoneServerConfig::offline_mode()
            } else {
                let mut server_config = LoxoneServerConfig::default();
                server_config.loxone.url =
                    format!("http://{loxone_host}").parse().map_err(|e| {
                        loxone_mcp_rust::LoxoneError::config(format!("Invalid URL: {e}"))
                    })?;
                server_config.loxone.username = loxone_user.clone();
                server_config.loxone.timeout = std::time::Duration::from_secs(30);
                server_config.loxone.verify_ssl = false;
                server_config
            }
        }
        TransportCommand::Http { dev_mode, .. } => {
            if *dev_mode {
                info!("Running in development mode - minimal configuration");
                LoxoneServerConfig::dev_mode()
            } else {
                let mut server_config = LoxoneServerConfig::default();
                server_config.loxone.url =
                    format!("http://{loxone_host}").parse().map_err(|e| {
                        loxone_mcp_rust::LoxoneError::config(format!("Invalid URL: {e}"))
                    })?;
                server_config.loxone.username = loxone_user.clone();
                server_config.loxone.timeout = std::time::Duration::from_secs(30);
                server_config.loxone.verify_ssl = false;
                server_config
            }
        }
        TransportCommand::StreamableHttp { .. } => {
            let mut server_config = LoxoneServerConfig::default();
            server_config.loxone.url = format!("https://{loxone_host}")
                .parse()
                .map_err(|e| loxone_mcp_rust::LoxoneError::config(format!("Invalid URL: {e}")))?;
            server_config.loxone.username = loxone_user.clone();
            server_config.loxone.timeout = std::time::Duration::from_secs(30);
            server_config.loxone.verify_ssl = false;
            server_config
        }
    };

    // Create framework authentication manager
    let framework_auth_manager = {
        let auth_config = match &config.transport {
            TransportCommand::Http { dev_mode, .. } if !dev_mode => {
                pulseengine_mcp_auth::AuthConfig {
                    enabled: true,
                    ..Default::default()
                }
            }
            _ => pulseengine_mcp_auth::AuthConfig {
                enabled: false,
                ..Default::default()
            },
        };

        let auth_manager = AuthenticationManager::new(auth_config)
            .await
            .map_err(|e| loxone_mcp_rust::LoxoneError::config(e.to_string()))?;

        info!("✅ Framework authentication initialized");
        Arc::new(auth_manager)
    };

    // Create Loxone framework backend
    let backend = create_loxone_backend(loxone_config).await?;
    info!("✅ Loxone framework backend initialized");

    // Create middleware stack based on transport (without monitoring - dropped in 0.17.0 upgrade)
    let middleware = match &config.transport {
        TransportCommand::Stdio { .. } => MiddlewareStack::new(),
        TransportCommand::Http { api_key, .. } => {
            let mut stack = MiddlewareStack::new();

            // Add security middleware
            let security_config = pulseengine_mcp_security::SecurityConfig::default();
            stack = stack.with_security(SecurityMiddleware::new(security_config));

            // Add auth middleware
            if let Some(key) = api_key {
                if key.len() < 32 {
                    return Err(loxone_mcp_rust::LoxoneError::config(
                        "API key must be at least 32 characters for security",
                    ));
                }
                info!(
                    "API key provided for HTTP authentication (key length: {} chars)",
                    key.len()
                );
            }
            stack = stack.with_auth(framework_auth_manager.clone());
            info!("Framework authentication middleware added");

            stack
        }
        TransportCommand::StreamableHttp { .. } => {
            let mut stack = MiddlewareStack::new();
            let security_config = pulseengine_mcp_security::SecurityConfig::default();
            stack = stack.with_security(SecurityMiddleware::new(security_config));
            stack
        }
    };

    // Create generic handler with middleware
    let handler = GenericServerHandler::new(backend, framework_auth_manager.clone(), middleware);

    // Create and configure transport
    let mut transport: Box<dyn Transport> = match &config.transport {
        TransportCommand::Stdio { .. } => {
            info!("Starting stdio transport for Claude Desktop");
            create_transport(pulseengine_mcp_transport::TransportConfig::Stdio)
                .map_err(|e| loxone_mcp_rust::LoxoneError::connection(e.to_string()))?
        }
        TransportCommand::Http {
            port,
            enable_sse,
            enable_cors,
            ..
        } => {
            if *enable_sse {
                info!("Starting HTTP transport with SSE support on port {}", port);
            } else {
                info!("Starting HTTP transport on port {}", port);
            }

            let http_transport = pulseengine_mcp_transport::http::HttpTransport::new(*port);

            if *enable_cors {
                info!("CORS enabled for HTTP transport");
            }

            Box::new(http_transport)
        }
        TransportCommand::StreamableHttp { port, enable_cors } => {
            info!("Starting Streamable HTTP transport on port {}", port);

            if *enable_cors {
                info!("CORS enabled for Streamable HTTP transport");
            }

            create_transport(pulseengine_mcp_transport::TransportConfig::StreamableHttp {
                port: *port,
                host: None,
            })
            .map_err(|e| loxone_mcp_rust::LoxoneError::connection(e.to_string()))?
        }
    };

    // Start the transport with the handler
    transport
        .start(Box::new(move |req| {
            let handler = handler.clone();
            Box::pin(async move {
                handler.handle_request(req).await.unwrap_or_else(|e| {
                    tracing::error!("Request handling error: {}", e);
                    pulseengine_mcp_protocol::Response {
                        jsonrpc: "2.0".to_string(),
                        id: None,
                        result: None,
                        error: Some(pulseengine_mcp_protocol::Error::internal_error(
                            e.to_string(),
                        )),
                    }
                })
            })
        }))
        .await
        .map_err(|e| loxone_mcp_rust::LoxoneError::connection(e.to_string()))?;

    info!("✅ Server started successfully");

    // Handle shutdown based on transport type
    match config.transport {
        TransportCommand::Stdio { .. } => {
            info!("Server running. Will exit when stdin closes.");
        }
        _ => {
            info!("Server running. Press Ctrl+C to stop.");
            tokio::signal::ctrl_c()
                .await
                .map_err(|e| loxone_mcp_rust::LoxoneError::connection(e.to_string()))?;
            info!("Shutting down...");
        }
    }

    Ok(())
}
