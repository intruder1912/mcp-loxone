# OpenCode + Loxone MCP Server Integration Guide

## Overview

This guide shows how to integrate the Loxone MCP Server with **OpenCode**, the open-source AI coding agent. This allows you to control your Loxone smart home devices directly from OpenCode.

## Configuration Files Created

### 1. **opencode.jsonc** - Main OpenCode Configuration
Located at: `/Users/intruder/devzone/mcp-loxone/opencode.jsonc`

This file configures the Loxone MCP server as a local MCP server for OpenCode. It includes:
- **Type**: `local` (runs the server locally via command)
- **Command**: Uses the release binary at `target/release/loxone-mcp-server`
- **Transport**: `stdio` (standard input/output for MCP protocol)
- **Environment Variables**: References your `.env.loxone.local.sh` credentials via `{env:VAR_NAME}`
- **Timeout**: 10 seconds (sufficient for device initialization)
- **Rules**: Embedded guidance for OpenCode on when/how to use Loxone tools

### 2. **test-opencode-integration.sh** - Integration Test Script
Located at: `/Users/intruder/devzone/mcp-loxone/test-opencode-integration.sh`

Validates the complete OpenCode integration:
- ✓ Loads environment credentials
- ✓ Verifies binary exists and is executable
- ✓ Validates JSON configuration
- ✓ Tests MCP server startup
- ✓ Confirms Loxone Miniserver connection

## Installation Instructions

### Step 1: Verify Prerequisites

```bash
# Check that environment variables are properly configured
source /Users/intruder/devzone/mcp-loxone/.env.loxone.local.sh
echo $LOXONE_HOST  # Should show your Miniserver IP
```

### Step 2: Copy OpenCode Configuration

Choose one of these options based on your OpenCode setup:

**Option A: Global OpenCode Configuration**
```bash
# For most users, copy to the default OpenCode config directory
cp /Users/intruder/devzone/mcp-loxone/opencode.jsonc ~/.config/opencode/opencode.jsonc
```

**Option B: Project-Specific Configuration**
```bash
# To use only in a specific project
cp /Users/intruder/devzone/mcp-loxone/opencode.jsonc /path/to/your/project/.opencode/opencode.jsonc
```

**Option C: Merge with Existing Configuration**
If you already have an `opencode.jsonc` file, merge the `mcp` section:
```json
{
  "mcp": {
    "loxone": {
      "type": "local",
      "command": [
        "/Users/intruder/devzone/mcp-loxone/target/release/loxone-mcp-server",
        "stdio"
      ],
      "enabled": true,
      "timeout": 10000,
      "environment": {
        "LOXONE_HOST": "{env:LOXONE_HOST}",
        "LOXONE_USER": "{env:LOXONE_USER}",
        "LOXONE_PASS": "{env:LOXONE_PASS}",
        "RUST_LOG": "info"
      }
    }
  }
}
```

### Step 3: Verify Installation

```bash
# Run the integration test
/Users/intruder/devzone/mcp-loxone/test-opencode-integration.sh

# Expected output: ✅ All tests passed!
```

## Using Loxone with OpenCode

### Activate Loxone MCP in OpenCode

Once configured, the Loxone MCP server will be available as `loxone` in OpenCode. You can:

1. **Reference it explicitly in prompts:**
   ```
   Turn on the living room light using the loxone tool
   ```

2. **Reference it in AGENTS.md rules:**
   ```markdown
   When controlling home automation devices, use the `loxone` MCP server.
   ```

3. **Add project-specific rules:**
   Create an `AGENTS.md` file in your project:
   ```markdown
   # Loxone Smart Home Controls

   When working with home automation, always use the `loxone` MCP server:
   - Controlling lights, blinds, and climate
   - Security system management
   - Audio zone control
   - Scene activation
   ```

### Available Tools

The Loxone MCP server exposes 17+ tools for home automation:

| Tool | Purpose |
|------|---------|
| `control_light` | Toggle, dim, or control lights (0-100%) |
| `control_blind` | Control blinds (up/down/stop, position 0-100%) |
| `set_temperature` | Set target temperature with safety validation |
| `set_security_mode` | Arm/disarm security (modes: arm, disarm, night, away) |
| `control_door_lock` | Lock/unlock doors |
| `control_intercom` | Answer, decline, or open door from intercom |
| `control_audio` | Play/pause, volume control per zone |
| `activate_scene` | Trigger predefined scenes |
| `control_device` | Direct device control by UUID |
| Various `get_*_status` | Query device states and status |

### Available Resources (Read-Only)

Access device information:

| Resource | Data |
|----------|------|
| `loxone://rooms` | Room listing with device counts |
| `loxone://rooms/{room}/devices` | Devices in a specific room |
| `loxone://devices/all` | Full device inventory |
| `loxone://devices/category/{cat}` | Devices by category |
| `loxone://sensors/*` | Door/window, temperature, motion sensors |
| `loxone://audio/zones` | Audio zone configuration |
| `loxone://system/status` | Miniserver status and capabilities |
| `loxone://energy/*` | Power monitoring and consumption data |

## Example Prompts

```
# Turn on a device
Turn on the kitchen lights using the loxone tool

# Check device status
What's the current temperature in the living room? Use the loxone resources to check

# Complex automation
Set the bedroom temperature to 21°C and close all blinds for privacy using the loxone tool

# Scene activation
Activate the "Movie Night" scene using the loxone MCP server
```

## Configuration Details

### Environment Variable Handling

The configuration uses `{env:VAR_NAME}` syntax to reference environment variables:

```jsonc
"environment": {
  "LOXONE_HOST": "{env:LOXONE_HOST}",    // 192.168.1.200
  "LOXONE_USER": "{env:LOXONE_USER}",    // tablet-og
  "LOXONE_PASS": "{env:LOXONE_PASS}",    // Your password
  "RUST_LOG": "info"                      // Logging level
}
```

These are sourced from `.env.loxone.local.sh` when you start OpenCode.

### MCP Server Settings

| Setting | Value | Description |
|---------|-------|-------------|
| `type` | `local` | Runs server locally (not remotely) |
| `command` | Array | Binary path and arguments |
| `enabled` | `true` | Server is active by default |
| `timeout` | `10000` | 10-second timeout for device discovery |
| `environment` | Object | Environment variables for the server |

## Troubleshooting

### Server fails to start

1. **Check environment variables:**
   ```bash
   source /Users/intruder/devzone/mcp-loxone/.env.loxone.local.sh
   env | grep LOXONE
   ```

2. **Verify binary permissions:**
   ```bash
   ls -la /Users/intruder/devzone/mcp-loxone/target/release/loxone-mcp-server
   # Should show: -rwxr-xr-x
   ```

3. **Test directly:**
   ```bash
   source /Users/intruder/devzone/mcp-loxone/.env.loxone.local.sh
   /Users/intruder/devzone/mcp-loxone/target/release/loxone-mcp-server stdio
   ```

### Cannot connect to Loxone Miniserver

1. **Check network connectivity:**
   ```bash
   ping 192.168.1.200  # Your LOXONE_HOST
   ```

2. **Verify credentials:**
   - Username: `tablet-og`
   - Password: Check the `.env.loxone.local.sh` file
   - Host: `192.168.1.200`

3. **Check Miniserver status:**
   - Visit `http://192.168.1.200/` in a browser
   - Ensure it's online and accessible

### Tools not appearing in OpenCode

1. **Verify MCP is enabled:**
   ```bash
   cat ~/.config/opencode/opencode.jsonc | grep '"enabled"'
   # Should show: "enabled": true
   ```

2. **Restart OpenCode:**
   - Close and reopen OpenCode
   - Tools are loaded on startup

3. **Check for errors:**
   - Run the test script: `test-opencode-integration.sh`

## Advanced Configuration

### Custom Logging

Enable debug logging to troubleshoot issues:

```jsonc
"environment": {
  "RUST_LOG": "debug",  // Change from "info" to "debug"
  // ... other variables
}
```

### Increasing Timeout

If devices take longer to initialize:

```jsonc
"timeout": 15000  // Increase to 15 seconds
```

### Multiple Loxone Servers

You can configure multiple Loxone servers by adding separate MCP entries:

```jsonc
"mcp": {
  "loxone-home": {
    "type": "local",
    "command": ["/path/to/binary", "stdio"],
    "environment": {
      "LOXONE_HOST": "{env:LOXONE_HOME_HOST}",
      "LOXONE_USER": "{env:LOXONE_HOME_USER}",
      "LOXONE_PASS": "{env:LOXONE_HOME_PASS}"
    }
  },
  "loxone-office": {
    "type": "local",
    "command": ["/path/to/binary", "stdio"],
    "environment": {
      "LOXONE_HOST": "{env:LOXONE_OFFICE_HOST}",
      "LOXONE_USER": "{env:LOXONE_OFFICE_USER}",
      "LOXONE_PASS": "{env:LOXONE_OFFICE_PASS}"
    }
  }
}
```

## Integration Architecture

```
OpenCode (AI Coding Agent)
    │
    ├─ MCP Protocol (stdio)
    │
    └─ loxone-mcp-server (release binary)
         │
         ├─ 17 MCP Tools (control_light, control_blind, etc.)
         ├─ 25+ MCP Resources (rooms, devices, sensors, etc.)
         │
         └─ Loxone HTTP API
              │
              └─ Loxone Miniserver @ 192.168.1.200
                   │
                   └─ Physical Devices (lights, blinds, sensors, etc.)
```

## Next Steps

1. ✅ Configuration created and tested
2. ✅ Environment variables configured
3. 📋 Install the configuration to your OpenCode directory
4. 🚀 Start OpenCode and test with home automation prompts
5. 📝 Consider adding project-specific rules to `AGENTS.md`

## Support

- **Project Repository**: https://github.com/avrabe/mcp-loxone
- **OpenCode Documentation**: https://opencode.ai/docs
- **MCP Server Docs**: `/docs/MCP_SERVERS.md` in the repository

## Version Information

- **Loxone MCP Server**: v0.7.0
- **MCP Protocol**: 2024-11-05
- **Release Binary Date**: March 9, 2026
- **Test Status**: ✅ All integration tests passed

---

Created: March 9, 2026
Configuration: `/Users/intruder/devzone/mcp-loxone/opencode.jsonc`
Test Script: `/Users/intruder/devzone/mcp-loxone/test-opencode-integration.sh`
