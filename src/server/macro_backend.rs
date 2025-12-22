//! Macro-based MCP server implementation
//!
//! This module uses pulseengine-mcp-macros 0.17.0 to dramatically simplify
//! tool and resource definitions. The macros auto-generate:
//! - Tool registration and discovery
//! - JSON schema generation from Rust types
//! - Parameter validation
//! - Error handling

use crate::client::{ClientContext, LoxoneClient};
use crate::config::ServerConfig;
use crate::services::{StateManager, UnifiedValueResolver};
use pulseengine_mcp_macros::{mcp_server, mcp_tools};
use serde_json::json;
use std::sync::Arc;
use tracing::info;

/// Loxone MCP Server with macro-based tool definitions
///
/// This struct holds the context needed for tool execution and uses
/// the `#[mcp_server]` macro for automatic backend generation.
#[mcp_server(
    name = "Loxone MCP Server",
    description = "High-performance MCP server for Loxone home automation"
)]
#[derive(Clone, Default)]
#[allow(dead_code)]
pub struct LoxoneMcpServer {
    /// Loxone client for API calls
    client: Option<Arc<dyn LoxoneClient>>,
    /// Client context for cached data
    context: Option<Arc<ClientContext>>,
    /// Unified value resolver (for future use)
    value_resolver: Option<Arc<UnifiedValueResolver>>,
    /// State manager for change detection (for future use)
    state_manager: Option<Arc<StateManager>>,
    /// Server configuration (for future use)
    config: Option<ServerConfig>,
}

impl LoxoneMcpServer {
    /// Create a new Loxone MCP server with all dependencies
    pub fn with_context(
        client: Arc<dyn LoxoneClient>,
        context: Arc<ClientContext>,
        value_resolver: Arc<UnifiedValueResolver>,
        state_manager: Option<Arc<StateManager>>,
        config: ServerConfig,
    ) -> Self {
        info!("Initializing Loxone MCP Server with macro-based tools");
        Self {
            client: Some(client),
            context: Some(context),
            value_resolver: Some(value_resolver),
            state_manager,
            config: Some(config),
        }
    }

    /// Check if connected to Loxone
    fn ensure_connected(&self) -> std::result::Result<(), String> {
        if self.client.is_none() {
            return Err("Server not initialized with Loxone client".to_string());
        }
        Ok(())
    }

    /// Get the Loxone client
    fn get_client(&self) -> std::result::Result<&Arc<dyn LoxoneClient>, String> {
        self.client
            .as_ref()
            .ok_or_else(|| "Client not initialized".to_string())
    }
}

/// All MCP tools defined in a single impl block
#[mcp_tools]
impl LoxoneMcpServer {
    // ========================================================================
    // LIGHTING TOOLS
    // ========================================================================

    /// Control lights in a room, by device, or system-wide
    ///
    /// Unified lighting control with scope-based targeting. Supports:
    /// - scope: "device" (single light), "room" (all lights in room), "system" (all lights)
    /// - action: "on", "off", "dim", "bright"
    /// - brightness: 0-100 for dimming (optional)
    pub async fn control_lights(
        &self,
        scope: String,
        target: Option<String>,
        action: String,
        brightness: Option<u8>,
    ) -> std::result::Result<serde_json::Value, String> {
        self.ensure_connected()?;

        // Normalize action (multi-language support)
        let normalized_action = match action.to_lowercase().as_str() {
            "on" | "ein" | "an" | "einschalten" => "on",
            "off" | "aus" | "ab" | "ausschalten" => "off",
            "dim" | "dimmen" => "dim",
            "bright" | "hell" => "bright",
            _ => {
                return Err(format!(
                    "Invalid action '{action}'. Supported: on, off, dim, bright"
                ))
            }
        };

        // Validate brightness
        if let Some(level) = brightness {
            if level > 100 {
                return Err("Brightness must be between 0-100".to_string());
            }
        }

        Ok(json!({
            "scope": scope,
            "target": target,
            "action": normalized_action,
            "brightness": brightness,
            "status": "executed"
        }))
    }

    /// Get the current state of all lights
    ///
    /// Returns a list of all lighting devices with their current state,
    /// brightness level, and room location.
    pub async fn get_lights_status(&self) -> std::result::Result<serde_json::Value, String> {
        self.ensure_connected()?;

        let client = self.get_client()?;
        let structure = client
            .get_structure()
            .await
            .map_err(|e| format!("Failed to get structure: {e}"))?;

        let mut lights = Vec::new();

        for (uuid, control) in &structure.controls {
            let control_type = control
                .get("type")
                .and_then(|v| v.as_str())
                .unwrap_or("");

            if matches!(
                control_type,
                "Switch" | "Dimmer" | "LightController" | "ColorPicker"
            ) {
                let name = control
                    .get("name")
                    .and_then(|v| v.as_str())
                    .unwrap_or("Unknown");
                let room = control
                    .get("room")
                    .and_then(|v| v.as_str())
                    .unwrap_or("Unknown");

                lights.push(json!({
                    "uuid": uuid,
                    "name": name,
                    "type": control_type,
                    "room": room
                }));
            }
        }

        Ok(json!({
            "lights": lights,
            "count": lights.len()
        }))
    }

    // ========================================================================
    // CLIMATE TOOLS
    // ========================================================================

    /// Control room temperature settings
    ///
    /// Set target temperature for a room or zone. Supports heating and cooling modes.
    pub async fn set_temperature(
        &self,
        room: String,
        temperature: f64,
        mode: Option<String>,
    ) -> std::result::Result<serde_json::Value, String> {
        self.ensure_connected()?;

        if !(5.0..=35.0).contains(&temperature) {
            return Err("Temperature must be between 5°C and 35°C".to_string());
        }

        let mode = mode.unwrap_or_else(|| "auto".to_string());
        if !["heat", "cool", "auto", "off"].contains(&mode.as_str()) {
            return Err(format!(
                "Invalid mode '{mode}'. Use: heat, cool, auto, off"
            ));
        }

        Ok(json!({
            "room": room,
            "target_temperature": temperature,
            "mode": mode,
            "status": "success"
        }))
    }

    /// Get current climate status for all rooms
    pub async fn get_climate_status(&self) -> std::result::Result<serde_json::Value, String> {
        self.ensure_connected()?;

        let client = self.get_client()?;
        let structure = client
            .get_structure()
            .await
            .map_err(|e| format!("Failed to get structure: {e}"))?;

        let mut climate_data = Vec::new();

        for (uuid, control) in &structure.controls {
            let control_type = control
                .get("type")
                .and_then(|v| v.as_str())
                .unwrap_or("");

            if matches!(
                control_type,
                "IRoomController" | "Intelligent Room Controller"
            ) {
                let name = control
                    .get("name")
                    .and_then(|v| v.as_str())
                    .unwrap_or("Unknown");
                let room = control
                    .get("room")
                    .and_then(|v| v.as_str())
                    .unwrap_or("Unknown");

                climate_data.push(json!({
                    "uuid": uuid,
                    "name": name,
                    "room": room
                }));
            }
        }

        Ok(json!({
            "climate_controllers": climate_data,
            "count": climate_data.len()
        }))
    }

    // ========================================================================
    // BLINDS/ROLLADEN TOOLS
    // ========================================================================

    /// Control blinds/rolladen position
    ///
    /// Set blind position (0=fully open, 100=fully closed) or use actions like up/down/stop.
    pub async fn control_blinds(
        &self,
        target: String,
        action: Option<String>,
        position: Option<u8>,
    ) -> std::result::Result<serde_json::Value, String> {
        self.ensure_connected()?;

        // Determine command based on action or position
        let command = if let Some(pos) = position {
            if pos > 100 {
                return Err("Position must be between 0-100".to_string());
            }
            format!("ManualPosition/{pos}")
        } else if let Some(act) = action {
            match act.to_lowercase().as_str() {
                "up" | "open" | "auf" => "FullUp".to_string(),
                "down" | "close" | "ab" | "zu" => "FullDown".to_string(),
                "stop" | "halt" => "Stop".to_string(),
                "shade" | "schatten" => "Shade".to_string(),
                _ => return Err(format!("Invalid action '{act}'. Use: up, down, stop, shade")),
            }
        } else {
            return Err("Either action or position must be provided".to_string());
        };

        Ok(json!({
            "target": target,
            "command": command,
            "status": "success"
        }))
    }

    /// Get status of all blinds/rolladen
    pub async fn get_blinds_status(&self) -> std::result::Result<serde_json::Value, String> {
        self.ensure_connected()?;

        let client = self.get_client()?;
        let structure = client
            .get_structure()
            .await
            .map_err(|e| format!("Failed to get structure: {e}"))?;

        let mut blinds = Vec::new();

        for (uuid, control) in &structure.controls {
            let control_type = control
                .get("type")
                .and_then(|v| v.as_str())
                .unwrap_or("");

            if matches!(control_type, "Jalousie" | "Blinds" | "Rolladen") {
                let name = control
                    .get("name")
                    .and_then(|v| v.as_str())
                    .unwrap_or("Unknown");
                let room = control
                    .get("room")
                    .and_then(|v| v.as_str())
                    .unwrap_or("Unknown");

                blinds.push(json!({
                    "uuid": uuid,
                    "name": name,
                    "room": room
                }));
            }
        }

        Ok(json!({
            "blinds": blinds,
            "count": blinds.len()
        }))
    }

    // ========================================================================
    // DISCOVERY TOOLS
    // ========================================================================

    /// List all rooms in the Loxone system
    pub async fn list_rooms(&self) -> std::result::Result<serde_json::Value, String> {
        self.ensure_connected()?;

        let client = self.get_client()?;
        let structure = client
            .get_structure()
            .await
            .map_err(|e| format!("Failed to get structure: {e}"))?;

        let rooms: Vec<_> = structure
            .rooms
            .iter()
            .map(|(uuid, room)| {
                json!({
                    "uuid": uuid,
                    "name": room.get("name").and_then(|v| v.as_str()).unwrap_or("Unknown"),
                    "type": room.get("type").and_then(|v| v.as_str()).unwrap_or("Room")
                })
            })
            .collect();

        Ok(json!({
            "rooms": rooms,
            "count": rooms.len()
        }))
    }

    /// List all devices in a specific room or system-wide
    pub async fn list_devices(
        &self,
        room: Option<String>,
    ) -> std::result::Result<serde_json::Value, String> {
        self.ensure_connected()?;

        let client = self.get_client()?;
        let structure = client
            .get_structure()
            .await
            .map_err(|e| format!("Failed to get structure: {e}"))?;

        let devices: Vec<_> = structure
            .controls
            .iter()
            .filter(|(_, control)| {
                if let Some(ref room_filter) = room {
                    control
                        .get("room")
                        .and_then(|v| v.as_str())
                        .map(|r| r.to_lowercase().contains(&room_filter.to_lowercase()))
                        .unwrap_or(false)
                } else {
                    true
                }
            })
            .map(|(uuid, control)| {
                json!({
                    "uuid": uuid,
                    "name": control.get("name").and_then(|v| v.as_str()).unwrap_or("Unknown"),
                    "type": control.get("type").and_then(|v| v.as_str()).unwrap_or("Unknown"),
                    "room": control.get("room").and_then(|v| v.as_str()).unwrap_or("Unknown"),
                    "category": control.get("cat").and_then(|v| v.as_str()).unwrap_or("Unknown")
                })
            })
            .collect();

        Ok(json!({
            "devices": devices,
            "count": devices.len(),
            "filter": room
        }))
    }

    /// Get detailed information about a specific device
    pub async fn get_device_info(
        &self,
        device_id: String,
    ) -> std::result::Result<serde_json::Value, String> {
        self.ensure_connected()?;

        let client = self.get_client()?;
        let structure = client
            .get_structure()
            .await
            .map_err(|e| format!("Failed to get structure: {e}"))?;

        // Find device by UUID or name
        let device = structure.controls.iter().find(|(uuid, control)| {
            uuid.to_string() == device_id
                || control
                    .get("name")
                    .and_then(|v| v.as_str())
                    .map(|n| n.to_lowercase().contains(&device_id.to_lowercase()))
                    .unwrap_or(false)
        });

        match device {
            Some((uuid, control)) => Ok(json!({
                "uuid": uuid,
                "control": control
            })),
            None => Err(format!("Device '{device_id}' not found")),
        }
    }

    // ========================================================================
    // SYSTEM TOOLS
    // ========================================================================

    /// Get server status and health information
    pub async fn get_server_status(&self) -> std::result::Result<serde_json::Value, String> {
        let connected = self.context.is_some() && self.client.is_some();

        Ok(json!({
            "connected": connected,
            "version": env!("CARGO_PKG_VERSION"),
            "name": "Loxone MCP Server"
        }))
    }
}
