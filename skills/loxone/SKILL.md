---
name: loxone
description: Control Loxone smart home devices — lights, blinds, climate, security, audio, and more.
---

You have access to a Loxone Miniserver via the `loxone-mcp-server` MCP tool. Use it to control and monitor smart home devices.

## Available Commands

### Device Control
- **Lights**: Turn on/off, dim (0-100%), set color
- **Blinds**: Open, close, stop, set position (0-100%)
- **Climate/HVAC**: Set temperature (16-28°C safe range), change modes
- **Security**: Arm/disarm alarm, set night/away modes
- **Door Locks**: Lock, unlock, open
- **Intercom**: Answer, decline, open door
- **Audio**: Play, pause, set volume (0-100%), select zone
- **Scenes**: Activate named scenes

### Status Queries
- Get device status (lights, blinds, climate, sensors, energy)
- Get room listings and device inventory
- Get live temperature, energy, and sensor readings
- Get weather station data

## Safety Rules

1. **Temperature**: Only set values between 16-28°C unless the user explicitly requests otherwise
2. **Security**: Always confirm with the user before disarming alarms or unlocking doors
3. **Scenes**: Describe what a scene will do before activating it
4. **Blinds**: Warn if closing all blinds (could block emergency exits)
5. **Audio**: Don't set volume above 80% without confirmation

## Usage

The MCP server connects to the Loxone Miniserver automatically. All commands go through the MCP protocol — you don't need to construct HTTP requests manually.
