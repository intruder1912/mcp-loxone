# Loxone MCP Examples

This directory contains example scripts showing how to use the Loxone MCP client directly in Python.

## Prerequisites

Before running these examples, make sure you have:

1. Configured your Loxone credentials:
   ```bash
   uvx --from .. loxone-mcp setup
   ```

2. Installed the dependencies:
   ```bash
   cd ..
   uv venv
   source .venv/bin/activate  # On Windows: .venv\Scripts\activate
   uv pip install -r requirements.txt
   ```

## Available Examples

### basic_control.py
Basic connection and structure exploration:
- Connect to Loxone Miniserver
- List all rooms
- List sample controls
- Shows how to send commands

```bash
python basic_control.py
```

### room_scenarios.py
Common room control scenarios:
- Movie mode (dim lights, close blinds)
- Room-specific light control
- Room-specific blind control
- Goodnight routine

```bash
python room_scenarios.py
```

## Using in Your Own Scripts

To use the Loxone client in your own Python scripts:

```python
from loxone_mcp.loxone_http_client import LoxoneHTTPClient
from loxone_mcp.secrets import LoxoneSecrets

# Get credentials from keychain
host = LoxoneSecrets.get(LoxoneSecrets.HOST_KEY)
username = LoxoneSecrets.get(LoxoneSecrets.USER_KEY)
password = LoxoneSecrets.get(LoxoneSecrets.PASS_KEY)

# Create and use client
async with LoxoneHTTPClient(host, username, password) as client:
    await client.connect()
    
    # Send commands
    await client.send_command("jdev/sps/io/{uuid}/On")
```

## Security Note

These examples use the system keychain for credential storage. Never hardcode credentials in your scripts!
