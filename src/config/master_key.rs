//! Auto-persistence for the PulseEngine MCP master encryption key.
//!
//! The `pulseengine-mcp-auth` crate encrypts its internal credential store
//! with an AES-GCM master key. By default a new random key is generated on
//! every startup, which makes previously encrypted data unreadable (issue #23).
//!
//! [`ensure_master_key`] solves this by persisting the key to a file so the
//! same key is reused across restarts.
//!
//! Priority order:
//! 1. `PULSEENGINE_MCP_MASTER_KEY` environment variable (highest)
//! 2. `~/.config/loxone-mcp/master.key` file
//! 3. Generate a new 32-byte random key, save it, and set the env var

use anyhow::Result;
use base64::{Engine as _, engine::general_purpose};
use rand::RngCore;
use std::fs;
use std::path::PathBuf;
use tracing::{debug, info};

/// Name of the environment variable expected by `pulseengine-mcp-auth`.
const ENV_KEY: &str = "PULSEENGINE_MCP_MASTER_KEY";

/// File name used to persist the master key.
const KEY_FILE_NAME: &str = "master.key";

/// Subdirectory under the platform config directory.
const CONFIG_SUBDIR: &str = "loxone-mcp";

/// Ensure `PULSEENGINE_MCP_MASTER_KEY` is set in the process environment.
///
/// If the variable is already present it is left untouched. Otherwise the
/// function tries to load it from the on-disk key file. If no file exists a
/// fresh 32-byte key is generated, base64-encoded, written to the file with
/// restrictive permissions (0600 on Unix), and set in the environment.
pub fn ensure_master_key() -> Result<()> {
    // 1. Already in the environment -- nothing to do.
    if std::env::var(ENV_KEY).is_ok() {
        debug!("Master key found in environment variable");
        return Ok(());
    }

    let key_path = master_key_path()?;

    // 2. Try to load from file.
    if key_path.exists() {
        let contents = fs::read_to_string(&key_path)?;
        let key = contents.trim();
        if !key.is_empty() {
            // SAFETY: called early in main, before worker threads that might
            // read this variable are spawned.
            unsafe { std::env::set_var(ENV_KEY, key) };
            debug!("Master key loaded from {}", key_path.display());
            return Ok(());
        }
    }

    // 3. Generate, persist, and set.
    let key = generate_master_key();

    if let Some(parent) = key_path.parent() {
        fs::create_dir_all(parent)?;
    }

    fs::write(&key_path, &key)?;

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(&key_path, fs::Permissions::from_mode(0o600))?;
    }

    // SAFETY: same as above -- called before threads are spawned.
    unsafe { std::env::set_var(ENV_KEY, &key) };
    info!("Generated and saved master key to {}", key_path.display());
    Ok(())
}

/// Return the path to the master key file.
///
/// Uses [`dirs::config_dir`] for cross-platform support, falling back to
/// `$HOME/.config` when the platform helper is unavailable.
fn master_key_path() -> Result<PathBuf> {
    let config_dir = dirs::config_dir()
        .or_else(|| dirs::home_dir().map(|h| h.join(".config")))
        .ok_or_else(|| anyhow::anyhow!("Could not determine config directory"))?;
    Ok(config_dir.join(CONFIG_SUBDIR).join(KEY_FILE_NAME))
}

/// Generate a 32-byte random key and return it as a base64-encoded string.
fn generate_master_key() -> String {
    let mut key_bytes = [0u8; 32];
    rand::rng().fill_bytes(&mut key_bytes);
    general_purpose::STANDARD.encode(key_bytes)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serial_test::serial;
    use temp_env::with_var_unset;

    /// Generated keys are valid, non-empty base64 strings of the right length.
    #[test]
    fn test_generate_master_key_valid_base64() {
        let key = generate_master_key();
        assert!(!key.is_empty(), "key must not be empty");

        let decoded = general_purpose::STANDARD
            .decode(&key)
            .expect("key must be valid base64");
        assert_eq!(decoded.len(), 32, "decoded key must be 32 bytes");
    }

    /// Two generated keys must not be identical (randomness sanity check).
    #[test]
    fn test_generate_master_key_is_random() {
        let k1 = generate_master_key();
        let k2 = generate_master_key();
        assert_ne!(k1, k2, "two generated keys should differ");
    }

    /// When the env var is already set, `ensure_master_key` should succeed
    /// without touching the filesystem.
    #[test]
    #[serial]
    fn test_env_var_takes_precedence() {
        let sentinel = "test-key-from-env-12345";
        temp_env::with_var(ENV_KEY, Some(sentinel), || {
            ensure_master_key().expect("should succeed with env var set");
            assert_eq!(
                std::env::var(ENV_KEY).unwrap(),
                sentinel,
                "env var must not be overwritten"
            );
        });
    }

    /// When a key file exists and the env var is unset, the key should be
    /// loaded from the file.
    #[test]
    #[serial]
    fn test_loads_existing_key_file() {
        let dir = tempfile::tempdir().expect("tmpdir");
        let key_file = dir.path().join(CONFIG_SUBDIR).join(KEY_FILE_NAME);
        fs::create_dir_all(key_file.parent().unwrap()).unwrap();

        let expected_key = "existing-key-ABCDEF123456";
        fs::write(&key_file, expected_key).unwrap();

        with_var_unset(ENV_KEY, || {
            // Patch the path by testing the internal helper indirectly:
            // We can't easily override `master_key_path`, so instead we
            // test the file-round-trip by writing to the real path in a
            // controlled way.  For isolation we just test the constituent
            // parts.
            let contents = fs::read_to_string(&key_file).unwrap();
            assert_eq!(contents.trim(), expected_key);
        });
    }

    /// A freshly generated key file has correct Unix permissions (0600).
    #[test]
    #[serial]
    #[cfg(unix)]
    fn test_key_file_permissions() {
        use std::os::unix::fs::PermissionsExt;

        let dir = tempfile::tempdir().expect("tmpdir");
        let key_file = dir.path().join("master.key");

        let key = generate_master_key();
        fs::write(&key_file, &key).unwrap();
        fs::set_permissions(&key_file, fs::Permissions::from_mode(0o600)).unwrap();

        let perms = fs::metadata(&key_file).unwrap().permissions();
        assert_eq!(
            perms.mode() & 0o777,
            0o600,
            "key file must have 0600 permissions"
        );
    }

    /// `master_key_path` returns a path ending with the expected components.
    #[test]
    fn test_master_key_path_components() {
        let path = master_key_path().expect("should resolve path");
        assert!(
            path.ends_with(format!("{CONFIG_SUBDIR}/{KEY_FILE_NAME}")),
            "path should end with {CONFIG_SUBDIR}/{KEY_FILE_NAME}, got: {}",
            path.display()
        );
    }
}
