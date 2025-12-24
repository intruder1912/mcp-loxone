//! Sensor state logging service
//!
//! Provides logging and history tracking for sensor state changes.
//! This is a minimal implementation that stores state changes in memory
//! with optional disk persistence.

use crate::services::SensorType;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::RwLock;

/// Sensor state logger for tracking sensor value changes over time
#[derive(Debug)]
pub struct SensorStateLogger {
    log_file: PathBuf,
    history: Arc<RwLock<HashMap<String, Vec<SensorStateEntry>>>>,
    max_entries_per_sensor: usize,
}

/// Entry in the sensor state history
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct SensorStateEntry {
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub old_value: serde_json::Value,
    pub new_value: serde_json::Value,
    pub sensor_name: Option<String>,
    pub sensor_type: Option<SensorType>,
    pub room: Option<String>,
}

impl SensorStateLogger {
    /// Create new sensor state logger
    pub fn new(log_file: PathBuf) -> Self {
        Self {
            log_file,
            history: Arc::new(RwLock::new(HashMap::new())),
            max_entries_per_sensor: 1000,
        }
    }

    /// Load existing history from disk
    pub async fn load_from_disk(&self) -> crate::error::Result<()> {
        if self.log_file.exists() {
            match tokio::fs::read_to_string(&self.log_file).await {
                Ok(contents) => {
                    if let Ok(history) = serde_json::from_str(&contents) {
                        *self.history.write().await = history;
                        tracing::debug!("Loaded sensor history from {:?}", self.log_file);
                    }
                }
                Err(e) => {
                    tracing::warn!("Failed to read sensor log file: {}", e);
                }
            }
        }
        Ok(())
    }

    /// Start periodic sync task (background persistence)
    pub fn start_periodic_sync(&self) {
        let history = self.history.clone();
        let log_file = self.log_file.clone();

        tokio::spawn(async move {
            let mut interval = tokio::time::interval(std::time::Duration::from_secs(300));
            loop {
                interval.tick().await;
                let data = history.read().await;
                if let Ok(json) = serde_json::to_string_pretty(&*data)
                    && let Err(e) = tokio::fs::write(&log_file, json).await
                {
                    tracing::warn!("Failed to persist sensor history: {}", e);
                }
            }
        });
    }

    /// Log a sensor state change
    pub async fn log_state_change(
        &self,
        uuid: String,
        old_value: serde_json::Value,
        new_value: serde_json::Value,
        sensor_name: Option<String>,
        sensor_type: Option<SensorType>,
        room: Option<String>,
    ) {
        let entry = SensorStateEntry {
            timestamp: chrono::Utc::now(),
            old_value,
            new_value,
            sensor_name,
            sensor_type,
            room,
        };

        let mut history = self.history.write().await;
        let entries = history.entry(uuid).or_insert_with(Vec::new);
        entries.push(entry);

        // Limit history size per sensor
        if entries.len() > self.max_entries_per_sensor {
            entries.remove(0);
        }
    }

    /// Get history for a specific sensor
    pub async fn get_sensor_history(&self, uuid: &str) -> Vec<SensorStateEntry> {
        self.history
            .read()
            .await
            .get(uuid)
            .cloned()
            .unwrap_or_default()
    }

    /// Get all sensor histories
    pub async fn get_all_history(&self) -> HashMap<String, Vec<SensorStateEntry>> {
        self.history.read().await.clone()
    }
}
