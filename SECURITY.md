# Security Policy

## Supported Versions

| Version | Supported          |
| ------- | ------------------ |
| 0.1.x   | :white_check_mark: |

## Reporting a Vulnerability

We take security seriously. If you discover a security vulnerability, please follow these steps:

1. **DO NOT** create a public issue
2. Email security concerns to: security@example.com
3. Include:
   - Description of the vulnerability
   - Steps to reproduce
   - Potential impact
   - Suggested fix (if any)

## Security Best Practices

When using this MCP server:

1. **Credentials Storage**
   - Always use the built-in keychain storage
   - Never commit credentials to version control
   - Use environment variables only in secure CI/CD environments

2. **Network Security**
   - Use this only on trusted local networks
   - Consider VPN for remote access
   - The Loxone Gen1 protocol uses unencrypted HTTP

3. **Access Control**
   - Limit MCP server access to trusted applications
   - Review permissions regularly
   - Use minimal required privileges

## Known Limitations

- Loxone Gen1 uses HTTP (not HTTPS) for communication
- WebSocket connections are unencrypted
- Consider network-level security measures

## Updates

Security updates will be released as soon as possible after verification.
Monitor the repository for security advisories.
