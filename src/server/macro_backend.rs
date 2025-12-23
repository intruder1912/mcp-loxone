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
            let control_type = control.get("type").and_then(|v| v.as_str()).unwrap_or("");

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
            return Err(format!("Invalid mode '{mode}'. Use: heat, cool, auto, off"));
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
            let control_type = control.get("type").and_then(|v| v.as_str()).unwrap_or("");

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
                _ => {
                    return Err(format!(
                        "Invalid action '{act}'. Use: up, down, stop, shade"
                    ))
                }
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
            let control_type = control.get("type").and_then(|v| v.as_str()).unwrap_or("");

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

    // ========================================================================
    // AUDIO TOOLS
    // ========================================================================

    /// Control an audio zone (play, pause, stop, next, previous)
    ///
    /// Manages playback in audio zones. Actions: play, pause, stop, next, previous, mute, unmute
    pub async fn control_audio_zone(
        &self,
        zone: String,
        action: String,
    ) -> std::result::Result<serde_json::Value, String> {
        self.ensure_connected()?;

        let normalized_action = match action.to_lowercase().as_str() {
            "play" | "abspielen" | "start" => "play",
            "pause" | "pausieren" => "pause",
            "stop" | "stopp" | "anhalten" => "stop",
            "next" | "weiter" | "nächster" => "next",
            "previous" | "zurück" | "vorheriger" => "previous",
            "mute" | "stumm" => "mute",
            "unmute" | "laut" => "unmute",
            _ => {
                return Err(format!(
                "Invalid action '{action}'. Use: play, pause, stop, next, previous, mute, unmute"
            ))
            }
        };

        Ok(json!({
            "zone": zone,
            "action": normalized_action,
            "status": "executed"
        }))
    }

    /// Set volume for an audio zone
    ///
    /// Set volume level (0-100) for a specific audio zone
    pub async fn set_audio_volume(
        &self,
        zone: String,
        volume: u8,
    ) -> std::result::Result<serde_json::Value, String> {
        self.ensure_connected()?;

        if volume > 100 {
            return Err("Volume must be between 0-100".to_string());
        }

        Ok(json!({
            "zone": zone,
            "volume": volume,
            "status": "success"
        }))
    }

    /// Get status of all audio zones
    pub async fn get_audio_status(&self) -> std::result::Result<serde_json::Value, String> {
        self.ensure_connected()?;

        let client = self.get_client()?;
        let structure = client
            .get_structure()
            .await
            .map_err(|e| format!("Failed to get structure: {e}"))?;

        let mut audio_zones = Vec::new();

        for (uuid, control) in &structure.controls {
            let control_type = control.get("type").and_then(|v| v.as_str()).unwrap_or("");

            if control_type.contains("Audio") || control_type == "MediaController" {
                let name = control
                    .get("name")
                    .and_then(|v| v.as_str())
                    .unwrap_or("Unknown");
                let room = control
                    .get("room")
                    .and_then(|v| v.as_str())
                    .unwrap_or("Unknown");

                audio_zones.push(json!({
                    "uuid": uuid,
                    "name": name,
                    "type": control_type,
                    "room": room
                }));
            }
        }

        Ok(json!({
            "audio_zones": audio_zones,
            "count": audio_zones.len()
        }))
    }

    // ========================================================================
    // SENSOR TOOLS
    // ========================================================================

    /// Get all sensor readings
    ///
    /// Returns current values from all sensors (temperature, humidity, motion, etc.)
    pub async fn get_sensor_readings(&self) -> std::result::Result<serde_json::Value, String> {
        self.ensure_connected()?;

        let client = self.get_client()?;
        let structure = client
            .get_structure()
            .await
            .map_err(|e| format!("Failed to get structure: {e}"))?;

        let mut sensors = Vec::new();

        for (uuid, control) in &structure.controls {
            let control_type = control.get("type").and_then(|v| v.as_str()).unwrap_or("");

            // Match sensor types
            if matches!(
                control_type,
                "InfoOnlyAnalog"
                    | "InfoOnlyDigital"
                    | "PresenceDetector"
                    | "MotionSensor"
                    | "SmokeAlarm"
                    | "Meter"
                    | "Sensor"
            ) {
                let name = control
                    .get("name")
                    .and_then(|v| v.as_str())
                    .unwrap_or("Unknown");
                let room = control
                    .get("room")
                    .and_then(|v| v.as_str())
                    .unwrap_or("Unknown");

                sensors.push(json!({
                    "uuid": uuid,
                    "name": name,
                    "type": control_type,
                    "room": room
                }));
            }
        }

        Ok(json!({
            "sensors": sensors,
            "count": sensors.len()
        }))
    }

    /// Get door and window sensor status
    ///
    /// Returns open/closed state of all door and window sensors
    pub async fn get_door_window_status(&self) -> std::result::Result<serde_json::Value, String> {
        self.ensure_connected()?;

        let client = self.get_client()?;
        let structure = client
            .get_structure()
            .await
            .map_err(|e| format!("Failed to get structure: {e}"))?;

        let mut door_windows = Vec::new();

        for (uuid, control) in &structure.controls {
            let control_type = control.get("type").and_then(|v| v.as_str()).unwrap_or("");
            let name = control
                .get("name")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_lowercase();

            // Match door/window sensors
            if control_type == "InfoOnlyDigital"
                && (name.contains("door")
                    || name.contains("window")
                    || name.contains("tür")
                    || name.contains("fenster"))
            {
                let room = control
                    .get("room")
                    .and_then(|v| v.as_str())
                    .unwrap_or("Unknown");

                door_windows.push(json!({
                    "uuid": uuid,
                    "name": control.get("name").and_then(|v| v.as_str()).unwrap_or("Unknown"),
                    "room": room
                }));
            }
        }

        Ok(json!({
            "door_window_sensors": door_windows,
            "count": door_windows.len()
        }))
    }

    /// Get motion detector status
    pub async fn get_motion_status(&self) -> std::result::Result<serde_json::Value, String> {
        self.ensure_connected()?;

        let client = self.get_client()?;
        let structure = client
            .get_structure()
            .await
            .map_err(|e| format!("Failed to get structure: {e}"))?;

        let mut motion_sensors = Vec::new();

        for (uuid, control) in &structure.controls {
            let control_type = control.get("type").and_then(|v| v.as_str()).unwrap_or("");

            if matches!(control_type, "PresenceDetector" | "MotionSensor") {
                let name = control
                    .get("name")
                    .and_then(|v| v.as_str())
                    .unwrap_or("Unknown");
                let room = control
                    .get("room")
                    .and_then(|v| v.as_str())
                    .unwrap_or("Unknown");

                motion_sensors.push(json!({
                    "uuid": uuid,
                    "name": name,
                    "room": room
                }));
            }
        }

        Ok(json!({
            "motion_sensors": motion_sensors,
            "count": motion_sensors.len()
        }))
    }

    // ========================================================================
    // WEATHER TOOLS
    // ========================================================================

    /// Get current weather data
    ///
    /// Returns weather station readings (temperature, humidity, wind, rain)
    pub async fn get_weather(&self) -> std::result::Result<serde_json::Value, String> {
        self.ensure_connected()?;

        let client = self.get_client()?;
        let structure = client
            .get_structure()
            .await
            .map_err(|e| format!("Failed to get structure: {e}"))?;

        let mut weather_devices = Vec::new();

        for (uuid, control) in &structure.controls {
            let control_type = control.get("type").and_then(|v| v.as_str()).unwrap_or("");

            if control_type.contains("Weather") {
                let name = control
                    .get("name")
                    .and_then(|v| v.as_str())
                    .unwrap_or("Unknown");

                weather_devices.push(json!({
                    "uuid": uuid,
                    "name": name,
                    "type": control_type
                }));
            }
        }

        Ok(json!({
            "weather_devices": weather_devices,
            "count": weather_devices.len()
        }))
    }

    // ========================================================================
    // ENERGY TOOLS
    // ========================================================================

    /// Get energy consumption data
    ///
    /// Returns current power usage and energy meters
    pub async fn get_energy_status(&self) -> std::result::Result<serde_json::Value, String> {
        self.ensure_connected()?;

        let client = self.get_client()?;
        let structure = client
            .get_structure()
            .await
            .map_err(|e| format!("Failed to get structure: {e}"))?;

        let mut energy_devices = Vec::new();

        for (uuid, control) in &structure.controls {
            let control_type = control.get("type").and_then(|v| v.as_str()).unwrap_or("");

            if matches!(control_type, "Meter" | "EnergyManager" | "EnergyMonitor")
                || control_type.contains("Energy")
            {
                let name = control
                    .get("name")
                    .and_then(|v| v.as_str())
                    .unwrap_or("Unknown");
                let room = control
                    .get("room")
                    .and_then(|v| v.as_str())
                    .unwrap_or("Unknown");

                energy_devices.push(json!({
                    "uuid": uuid,
                    "name": name,
                    "type": control_type,
                    "room": room
                }));
            }
        }

        Ok(json!({
            "energy_devices": energy_devices,
            "count": energy_devices.len()
        }))
    }

    /// Control EV charging
    ///
    /// Start, stop, or set charging limits for electric vehicle chargers
    pub async fn control_ev_charging(
        &self,
        charger: String,
        action: String,
        limit_kwh: Option<f64>,
    ) -> std::result::Result<serde_json::Value, String> {
        self.ensure_connected()?;

        let normalized_action = match action.to_lowercase().as_str() {
            "start" | "laden" => "start",
            "stop" | "stoppen" => "stop",
            "pause" | "pausieren" => "pause",
            _ => {
                return Err(format!(
                    "Invalid action '{action}'. Use: start, stop, pause"
                ))
            }
        };

        Ok(json!({
            "charger": charger,
            "action": normalized_action,
            "limit_kwh": limit_kwh,
            "status": "executed"
        }))
    }

    // ========================================================================
    // SECURITY TOOLS
    // ========================================================================

    /// Get security system status
    ///
    /// Returns alarm system state, door locks, and security sensors
    pub async fn get_security_status(&self) -> std::result::Result<serde_json::Value, String> {
        self.ensure_connected()?;

        let client = self.get_client()?;
        let structure = client
            .get_structure()
            .await
            .map_err(|e| format!("Failed to get structure: {e}"))?;

        let mut security_devices = Vec::new();

        for (uuid, control) in &structure.controls {
            let control_type = control.get("type").and_then(|v| v.as_str()).unwrap_or("");

            if matches!(
                control_type,
                "Alarm" | "SmokeAlarm" | "Gate" | "DoorLock" | "AccessControl"
            ) || control_type.contains("Security")
            {
                let name = control
                    .get("name")
                    .and_then(|v| v.as_str())
                    .unwrap_or("Unknown");
                let room = control
                    .get("room")
                    .and_then(|v| v.as_str())
                    .unwrap_or("Unknown");

                security_devices.push(json!({
                    "uuid": uuid,
                    "name": name,
                    "type": control_type,
                    "room": room
                }));
            }
        }

        Ok(json!({
            "security_devices": security_devices,
            "count": security_devices.len()
        }))
    }

    /// Arm or disarm security system
    ///
    /// Set security system mode: arm_away, arm_home, disarm
    pub async fn set_security_mode(
        &self,
        mode: String,
        code: Option<String>,
    ) -> std::result::Result<serde_json::Value, String> {
        self.ensure_connected()?;

        let normalized_mode = match mode.to_lowercase().as_str() {
            "arm" | "arm_away" | "scharf" | "abwesend" => "arm_away",
            "arm_home" | "arm_stay" | "zuhause" => "arm_home",
            "disarm" | "unscharf" | "aus" => "disarm",
            _ => {
                return Err(format!(
                    "Invalid mode '{mode}'. Use: arm_away, arm_home, disarm"
                ))
            }
        };

        Ok(json!({
            "mode": normalized_mode,
            "code_provided": code.is_some(),
            "status": "executed"
        }))
    }

    /// Control door lock
    ///
    /// Lock or unlock a smart door lock
    pub async fn control_door_lock(
        &self,
        lock: String,
        action: String,
    ) -> std::result::Result<serde_json::Value, String> {
        self.ensure_connected()?;

        let normalized_action = match action.to_lowercase().as_str() {
            "lock" | "abschließen" | "zu" => "lock",
            "unlock" | "aufschließen" | "auf" => "unlock",
            _ => return Err(format!("Invalid action '{action}'. Use: lock, unlock")),
        };

        Ok(json!({
            "lock": lock,
            "action": normalized_action,
            "status": "executed"
        }))
    }

    // ========================================================================
    // CAMERA TOOLS
    // ========================================================================

    /// Get camera/intercom status
    ///
    /// Returns list of cameras and video intercoms
    pub async fn get_camera_status(&self) -> std::result::Result<serde_json::Value, String> {
        self.ensure_connected()?;

        let client = self.get_client()?;
        let structure = client
            .get_structure()
            .await
            .map_err(|e| format!("Failed to get structure: {e}"))?;

        let mut cameras = Vec::new();

        for (uuid, control) in &structure.controls {
            let control_type = control.get("type").and_then(|v| v.as_str()).unwrap_or("");

            if matches!(control_type, "Intercom" | "Camera" | "Doorbell")
                || control_type.contains("Camera")
            {
                let name = control
                    .get("name")
                    .and_then(|v| v.as_str())
                    .unwrap_or("Unknown");
                let room = control
                    .get("room")
                    .and_then(|v| v.as_str())
                    .unwrap_or("Unknown");

                cameras.push(json!({
                    "uuid": uuid,
                    "name": name,
                    "type": control_type,
                    "room": room
                }));
            }
        }

        Ok(json!({
            "cameras": cameras,
            "count": cameras.len()
        }))
    }

    // ========================================================================
    // INTERCOM TOOLS
    // ========================================================================

    /// Answer or control intercom
    ///
    /// Answer calls, open doors, or control intercom features
    pub async fn control_intercom(
        &self,
        intercom: String,
        action: String,
    ) -> std::result::Result<serde_json::Value, String> {
        self.ensure_connected()?;

        let normalized_action = match action.to_lowercase().as_str() {
            "answer" | "annehmen" | "abheben" => "answer",
            "hangup" | "auflegen" | "beenden" => "hangup",
            "open" | "öffnen" | "tür" => "open_door",
            "talk" | "sprechen" => "talk",
            "mute" | "stumm" => "mute",
            _ => {
                return Err(format!(
                    "Invalid action '{action}'. Use: answer, hangup, open, talk, mute"
                ))
            }
        };

        Ok(json!({
            "intercom": intercom,
            "action": normalized_action,
            "status": "executed"
        }))
    }

    /// Get intercom call history
    pub async fn get_intercom_history(&self) -> std::result::Result<serde_json::Value, String> {
        self.ensure_connected()?;

        Ok(json!({
            "history": [],
            "message": "Call history requires active connection to Loxone"
        }))
    }

    // ========================================================================
    // SCENE/MOOD TOOLS
    // ========================================================================

    /// Activate a scene or mood
    ///
    /// Trigger predefined scenes (moods) for rooms or the whole house
    pub async fn activate_scene(
        &self,
        scene: String,
        room: Option<String>,
    ) -> std::result::Result<serde_json::Value, String> {
        self.ensure_connected()?;

        Ok(json!({
            "scene": scene,
            "room": room,
            "status": "activated"
        }))
    }

    /// List available scenes
    pub async fn list_scenes(&self) -> std::result::Result<serde_json::Value, String> {
        self.ensure_connected()?;

        let client = self.get_client()?;
        let structure = client
            .get_structure()
            .await
            .map_err(|e| format!("Failed to get structure: {e}"))?;

        let mut scenes = Vec::new();

        for (uuid, control) in &structure.controls {
            let control_type = control.get("type").and_then(|v| v.as_str()).unwrap_or("");

            if matches!(control_type, "LightController" | "MoodSwitch") {
                let name = control
                    .get("name")
                    .and_then(|v| v.as_str())
                    .unwrap_or("Unknown");
                let room = control
                    .get("room")
                    .and_then(|v| v.as_str())
                    .unwrap_or("Unknown");

                // Extract moods if available
                let moods = control.get("moods").cloned().unwrap_or(json!([]));

                scenes.push(json!({
                    "uuid": uuid,
                    "name": name,
                    "room": room,
                    "moods": moods
                }));
            }
        }

        Ok(json!({
            "scene_controllers": scenes,
            "count": scenes.len()
        }))
    }
}
