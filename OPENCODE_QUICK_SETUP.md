# OpenCode + Loxone MCP - Quick Setup Checklist

## ✅ Completed Setup Steps

- [x] **Loxone MCP Server v0.7.0** - Release binary built and tested
- [x] **Environment Variables** - Loaded from `.env.loxone.local.sh`
  - LOXONE_HOST: `192.168.1.200`
  - LOXONE_USER: `tablet-og`
  - Miniserver: **Connected** ✓
- [x] **OpenCode Configuration** - Created at `opencode.jsonc`
- [x] **Integration Test** - All tests passed ✓

## 📋 Your Next Steps

### Step 1: Copy Configuration File

Choose your installation method:

**Option A: Global Installation (Recommended)**
```bash
# Copy to your OpenCode config directory
cp opencode.jsonc ~/.config/opencode/opencode.jsonc

# Or on macOS:
cp opencode.jsonc ~/Library/Application\ Support/opencode/opencode.jsonc
```

**Option B: Project Installation**
```bash
# Place in your project directory
cp opencode.jsonc /path/to/your/project/.opencode/opencode.jsonc
```

### Step 2: Start OpenCode

```bash
# Source your Loxone environment first
source .env.loxone.local.sh

# Then start OpenCode
opencode

# Or in a project directory
cd /path/to/your/project
opencode
```

### Step 3: Test a Loxone Command

Once in OpenCode, try one of these prompts:

```
Turn on the living room light using the loxone tool

or

Get the list of all devices using the loxone resources
```

## 🔧 Configuration Reference

**File**: `opencode.jsonc`
**Location**: `/Users/intruder/devzone/mcp-loxone/`

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

## 🎯 Available Tools

```
control_light         - Turn lights on/off, dim (0-100%)
control_blind         - Control blinds (up/down/stop, position)
set_temperature       - Set target temperature
set_security_mode     - Arm/disarm security (modes: arm, disarm, night, away)
control_door_lock     - Lock/unlock doors
control_intercom      - Answer/decline/open from intercom
control_audio         - Play/pause, volume per zone
activate_scene        - Trigger predefined scenes
control_device        - Direct device control by UUID
get_*_status          - Query device states
```

## 📚 Available Resources

```
loxone://rooms                    - Room listing
loxone://rooms/{room}/devices     - Devices in room
loxone://devices/all              - All devices
loxone://devices/category/{cat}   - Devices by category
loxone://sensors/*                - All sensors
loxone://audio/zones              - Audio zones
loxone://system/status            - System info
loxone://energy/*                 - Energy data
```

## 🧪 Verify Installation

After copying the config, verify it works:

```bash
# Run the integration test
./test-opencode-integration.sh

# Expected: ✅ All tests passed!
```

## 🆘 Troubleshooting

| Issue | Solution |
|-------|----------|
| **Tools not showing** | Restart OpenCode after copying config |
| **Connection fails** | Check `source .env.loxone.local.sh` loads credentials |
| **Timeout errors** | Increase `timeout` value in opencode.jsonc |
| **Cannot find binary** | Verify path in `command` array matches your system |

## 📖 Full Documentation

See `OPENCODE_INTEGRATION.md` for:
- Detailed installation instructions
- Advanced configuration options
- Troubleshooting guide
- Multiple server setup
- Architecture overview

## 💡 Example Workflows

### Control Lights
```
Turn on all lights in the living room and set them to 80% brightness
```

### Climate Control
```
Set the bedroom temperature to 21°C and check current status
```

### Security
```
Arm the security system in away mode
```

### Scene Activation
```
Activate the movie night scene
```

## 🚀 Ready to Go!

You're all set! OpenCode can now control your Loxone smart home.

- **Binary**: ✅ `/Users/intruder/devzone/mcp-loxone/target/release/loxone-mcp-server`
- **Config**: ✅ `opencode.jsonc`
- **Credentials**: ✅ Loaded from `.env.loxone.local.sh`
- **Miniserver**: ✅ Connected (192.168.1.200)

**Next**: Copy the configuration and start OpenCode!

---

Created: March 9, 2026
Integration Status: ✅ Ready for Use
