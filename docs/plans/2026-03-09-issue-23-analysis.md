# Issue #23 Analysis: Encryption Key Error on Server Restart

**Date:** 2026-03-09
**Issue:** https://github.com/avrabe/mcp-loxone/issues/23

---

## Problem Statement

When a user stops the server and starts it again, they get:

```
WARN pulseengine_mcp_auth::crypto::keys: Generated new master key.
  Set PULSEENGINE_MCP_MASTER_KEY=... for persistence
Error: Config("Storage error: Encryption error: Decryption failed: aead::Error")
```

## Root Cause

The `pulseengine-mcp-auth` crate (v0.17.0, external dependency) generates a **random AES-GCM master encryption key** on each startup. It uses this key to encrypt its internal credential/API-key store on disk. On restart, a **new random key** is generated, and the previously encrypted data can no longer be decrypted, causing the `aead::Error`.

The key is only persisted if the `PULSEENGINE_MCP_MASTER_KEY` environment variable is set, which users don't know about on first run.

## Where It Triggers

1. **HTTP/StreamableHttp transports** -- `AuthenticationManager::new(AuthConfig { ..Default::default() })` at `src/main.rs` line 414. The `Default` config uses file-based encrypted storage.

2. **Stdio transport** -- The `#[mcp_server]` macro from `pulseengine-mcp-macros` generates `serve_stdio()` which creates its own internal `AuthenticationManager` with default (file-based) config.

3. The project's own tests already use `AuthConfig::memory()` to avoid this problem (see `tests/framework_auth_test.rs`), confirming awareness of the issue.

## Key Insight

The project's own credential system (`CredentialRegistry` at `~/.loxone-mcp/registry.json`, environment variables, Infisical) is **completely separate** from `pulseengine-mcp-auth`'s internal encrypted store. The issue is entirely within the framework dependency's internal state.

## Proposed Fix (Two-Part)

### Part 1: Auto-persist the master key

Create a utility function in `src/config/master_key.rs`:

```rust
use std::fs;
use std::path::PathBuf;
use anyhow::Result;
use tracing::{info, debug};

/// Ensure PULSEENGINE_MCP_MASTER_KEY is set, persisting to file if needed.
/// Priority: env var > file > generate new + save
pub fn ensure_master_key() -> Result<()> {
    // If already set in environment, nothing to do
    if std::env::var("PULSEENGINE_MCP_MASTER_KEY").is_ok() {
        debug!("Master key found in environment");
        return Ok(());
    }

    let key_path = master_key_path()?;

    // Try to read from file
    if key_path.exists() {
        let key = fs::read_to_string(&key_path)?;
        let key = key.trim();
        if !key.is_empty() {
            std::env::set_var("PULSEENGINE_MCP_MASTER_KEY", key);
            debug!("Master key loaded from {}", key_path.display());
            return Ok(());
        }
    }

    // Generate new key, save to file, set env var
    // We'll start the auth system once to get the generated key from the warning log,
    // or generate our own 32-byte key
    let key = generate_master_key();

    // Ensure parent directory exists
    if let Some(parent) = key_path.parent() {
        fs::create_dir_all(parent)?;
    }

    fs::write(&key_path, &key)?;

    // Set restrictive permissions (Unix only)
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(&key_path, fs::Permissions::from_mode(0o600))?;
    }

    std::env::set_var("PULSEENGINE_MCP_MASTER_KEY", &key);
    info!("Generated and saved master key to {}", key_path.display());
    Ok(())
}

fn master_key_path() -> Result<PathBuf> {
    let config_dir = dirs::config_dir()
        .or_else(|| dirs::home_dir().map(|h| h.join(".config")))
        .ok_or_else(|| anyhow::anyhow!("Could not determine config directory"))?;
    Ok(config_dir.join("loxone-mcp").join("master.key"))
}

fn generate_master_key() -> String {
    use rand::RngCore;
    let mut key_bytes = [0u8; 32];
    rand::rng().fill_bytes(&mut key_bytes);
    base64::engine::general_purpose::STANDARD.encode(key_bytes)
}
```

### Part 2: Use memory-based auth for stdio

For the stdio transport path, use `AuthConfig::memory()` instead of `AuthConfig::default()` since stdio mode doesn't need persistent API key storage. This eliminates the encryption issue entirely for the most common use case (Claude Desktop integration).

### Files to Modify

1. **New file:** `src/config/master_key.rs` -- The `ensure_master_key()` utility
2. **`src/config/mod.rs`** -- Re-export the new module
3. **`src/main.rs`** -- Call `ensure_master_key()` early in all three transport paths; use `AuthConfig::memory()` for stdio
4. **`src/bin/loxone-mcp-auth.rs`** -- Call `ensure_master_key()` at startup
5. **`src/bin/setup.rs`** -- Call `ensure_master_key()` at startup

### Testing Plan

1. Delete any existing `~/.config/loxone-mcp/master.key`
2. Start server -- should generate and save key
3. Stop server
4. Start server again -- should load key from file, no error
5. Verify `PULSEENGINE_MCP_MASTER_KEY` env var overrides file
6. Verify stdio mode works without file (memory auth)

## Alternative Considered

**Option B: Just use memory auth everywhere** -- Simpler but breaks HTTP mode's ability to persist API keys across restarts. Not recommended for production use.

**Option C: Clear encrypted state on key mismatch** -- Detect `aead::Error`, delete the encrypted store, and restart fresh. Simpler but loses any stored API keys. Acceptable as a fallback but not primary solution.
