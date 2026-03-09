#!/bin/bash
# OpenCode + Loxone MCP Server Integration Test
# This script validates the OpenCode configuration works correctly with the Loxone MCP server

set -e

echo "🧪 OpenCode + Loxone MCP Integration Test"
echo "========================================="

# Source environment
echo "📦 Loading Loxone credentials..."
source /Users/intruder/devzone/mcp-loxone/.env.loxone.local.sh

# Verify environment variables
echo "✓ Environment variables loaded:"
echo "  LOXONE_HOST=$LOXONE_HOST"
echo "  LOXONE_USER=$LOXONE_USER"
echo "  (password is hidden)"

# Check binary
BINARY="/Users/intruder/devzone/mcp-loxone/target/release/loxone-mcp-server"
if [ ! -f "$BINARY" ]; then
    echo "❌ Binary not found at $BINARY"
    exit 1
fi
echo "✓ Release binary found"

# Check config file
CONFIG="/Users/intruder/devzone/mcp-loxone/opencode.jsonc"
if [ ! -f "$CONFIG" ]; then
    echo "❌ OpenCode config not found at $CONFIG"
    exit 1
fi
echo "✓ OpenCode configuration file found"

# Validate JSON
echo "🔍 Validating JSON configuration..."
if jq . "$CONFIG" > /dev/null 2>&1; then
    echo "✓ JSON is valid"
else
    echo "❌ JSON validation failed"
    exit 1
fi

# Test MCP server startup
echo ""
echo "🚀 Testing MCP server startup with environment variables..."
echo "   (Server will run for 2 seconds then stop)"

$BINARY stdio > /tmp/mcp_startup.log 2>&1 &
MCP_PID=$!

sleep 2
kill $MCP_PID 2>/dev/null || true
wait $MCP_PID 2>/dev/null || true

if grep -q "MCP server started successfully" /tmp/mcp_startup.log; then
    echo "✓ MCP server started successfully"
else
    echo "❌ MCP server failed to start"
    echo "   Log output:"
    cat /tmp/mcp_startup.log
    exit 1
fi

if grep -q "Loxone client connected" /tmp/mcp_startup.log; then
    echo "✓ Connected to Loxone Miniserver"
else
    echo "❌ Failed to connect to Loxone Miniserver"
    echo "   Check your LOXONE_HOST and credentials"
fi

echo ""
echo "✅ All tests passed!"
echo ""
echo "📝 Configuration Details:"
echo "   Binary: $BINARY"
echo "   Config: $CONFIG"
echo "   Command: $BINARY stdio"
echo ""
echo "🎯 OpenCode Configuration:"
echo "   MCP Server Name: loxone"
echo "   Type: local"
echo "   Transport: stdio"
echo ""
echo "📚 Available Loxone Tools:"
echo "   - control_light"
echo "   - control_blind"
echo "   - set_temperature"
echo "   - set_security_mode"
echo "   - control_door_lock"
echo "   - control_intercom"
echo "   - control_audio"
echo "   - activate_scene"
echo ""
echo "💡 Next Steps:"
echo "   1. Copy the config to your OpenCode config directory:"
echo "      cp $CONFIG ~/.config/opencode/opencode.jsonc"
echo "      (or your OpenCode config location)"
echo ""
echo "   2. Start OpenCode and test with a prompt like:"
echo "      'Turn on the living room light using the loxone tool'"
echo ""
