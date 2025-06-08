# Changelog

## [0.1.0] - 2024-06-08

### Initial Release

- **Features**
  - Room-based device control
  - Rolladen (blinds) control with up/down/stop commands
  - Light control with on/off/toggle/dim functionality
  - Device status querying
  - Secure credential storage using system keychain
  - MCP resources for structure data access

- **Technical**
  - Custom WebSocket implementation for Loxone communication
  - FastMCP 2.0 integration
  - Support for environment variables (CI/CD friendly)
  - Comprehensive logging and error handling
  - Type hints throughout the codebase

- **Security**
  - Credentials stored in system keychain
  - No plaintext passwords in configuration files
  - Support for basic HTTP authentication
  - Local network only (Gen 1 limitation)

### Known Limitations

- Loxone Generation 1 specific (no HTTPS support)
- Binary status updates not yet implemented
- State tracking is simplified
- No support for complex Loxone controls (e.g., mood controller)

### Future Improvements

- Full binary message parsing
- Real-time state tracking
- Support for more device types
- Token-based authentication for newer Loxone versions
