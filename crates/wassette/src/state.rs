// Copyright (c) Microsoft Corporation.
// Licensed under the MIT license.

//! State management for Wassette
//!
//! This module provides functionality to save and restore the complete state of a Wassette
//! instance, enabling transfer of state between agents or persistence across restarts.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use tracing::{debug, info};

use crate::ComponentMetadata;

/// Represents the complete state of a Wassette instance that can be persisted and transferred
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WassetteState {
    /// Version of the state format for compatibility checking
    pub version: String,
    /// Timestamp when the state was exported (Unix epoch seconds)
    pub exported_at: u64,
    /// Components that are currently loaded
    pub components: Vec<ComponentStateEntry>,
    /// Environment variables configuration
    pub environment: HashMap<String, String>,
    /// Directory paths used by this instance
    pub paths: PathConfiguration,
}

/// Configuration for directory paths
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PathConfiguration {
    /// Path to the component directory
    pub component_dir: PathBuf,
    /// Path to the secrets directory
    pub secrets_dir: PathBuf,
}

/// Represents a single component's state
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComponentStateEntry {
    /// Unique identifier for the component
    pub component_id: String,
    /// Component metadata
    pub metadata: ComponentMetadata,
    /// Policy attachment information if present
    pub policy: Option<PolicyAttachment>,
    /// Secrets configuration for this component
    pub secrets: Option<SecretsConfiguration>,
}

/// Information about a policy attached to a component
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PolicyAttachment {
    /// The original URI where the policy was loaded from
    pub source_uri: String,
    /// Policy content (YAML format)
    pub policy_content: String,
    /// Timestamp when the policy was attached
    pub attached_at: u64,
}

/// Configuration for component secrets
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecretsConfiguration {
    /// Environment variables provided as secrets
    /// Note: Values are intentionally excluded for security - only keys are exported
    pub env_keys: Vec<String>,
}

impl WassetteState {
    /// Create a new state snapshot with the current version
    pub fn new(component_dir: PathBuf, secrets_dir: PathBuf) -> Self {
        Self {
            version: env!("CARGO_PKG_VERSION").to_string(),
            exported_at: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs(),
            components: Vec::new(),
            environment: HashMap::new(),
            paths: PathConfiguration {
                component_dir,
                secrets_dir,
            },
        }
    }

    /// Add a component to the state
    pub fn add_component(&mut self, entry: ComponentStateEntry) {
        self.components.push(entry);
    }

    /// Add environment variable to the state
    pub fn add_environment_var(&mut self, key: String, value: String) {
        self.environment.insert(key, value);
    }

    /// Serialize the state to JSON
    pub fn to_json(&self) -> Result<String> {
        serde_json::to_string_pretty(self).context("Failed to serialize Wassette state to JSON")
    }

    /// Deserialize state from JSON
    pub fn from_json(json: &str) -> Result<Self> {
        serde_json::from_str(json).context("Failed to deserialize Wassette state from JSON")
    }

    /// Serialize the state to YAML
    pub fn to_yaml(&self) -> Result<String> {
        serde_yaml::to_string(self).context("Failed to serialize Wassette state to YAML")
    }

    /// Deserialize state from YAML
    pub fn from_yaml(yaml: &str) -> Result<Self> {
        serde_yaml::from_str(yaml).context("Failed to deserialize Wassette state from YAML")
    }

    /// Save the state to a file
    pub async fn save_to_file(&self, path: impl AsRef<Path>) -> Result<()> {
        let path = path.as_ref();
        let content = if path.extension().and_then(|s| s.to_str()) == Some("yaml")
            || path.extension().and_then(|s| s.to_str()) == Some("yml")
        {
            self.to_yaml()?
        } else {
            self.to_json()?
        };

        tokio::fs::write(path, content)
            .await
            .with_context(|| format!("Failed to write state to {}", path.display()))?;

        info!("Saved Wassette state to {}", path.display());
        Ok(())
    }

    /// Load state from a file
    pub async fn load_from_file(path: impl AsRef<Path>) -> Result<Self> {
        let path = path.as_ref();
        let content = tokio::fs::read_to_string(path)
            .await
            .with_context(|| format!("Failed to read state from {}", path.display()))?;

        let state = if path.extension().and_then(|s| s.to_str()) == Some("yaml")
            || path.extension().and_then(|s| s.to_str()) == Some("yml")
        {
            Self::from_yaml(&content)?
        } else {
            Self::from_json(&content)?
        };

        debug!(
            "Loaded Wassette state from {} (version: {}, {} components)",
            path.display(),
            state.version,
            state.components.len()
        );
        Ok(state)
    }

    /// Validate state compatibility
    pub fn validate_compatibility(&self) -> Result<()> {
        // For now, just check that version is present
        // In the future, we can add more sophisticated version checking
        if self.version.is_empty() {
            anyhow::bail!("State version is empty");
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_state_creation() {
        let state = WassetteState::new(
            PathBuf::from("/tmp/components"),
            PathBuf::from("/tmp/secrets"),
        );

        assert_eq!(state.version, env!("CARGO_PKG_VERSION"));
        assert_eq!(state.components.len(), 0);
        assert_eq!(state.environment.len(), 0);
    }

    #[test]
    fn test_state_json_serialization() {
        let mut state = WassetteState::new(
            PathBuf::from("/tmp/components"),
            PathBuf::from("/tmp/secrets"),
        );
        state.add_environment_var("TEST_VAR".to_string(), "test_value".to_string());

        let json = state.to_json().expect("Failed to serialize to JSON");
        let deserialized =
            WassetteState::from_json(&json).expect("Failed to deserialize from JSON");

        assert_eq!(state.version, deserialized.version);
        assert_eq!(state.environment.len(), deserialized.environment.len());
    }

    #[test]
    fn test_state_yaml_serialization() {
        let mut state = WassetteState::new(
            PathBuf::from("/tmp/components"),
            PathBuf::from("/tmp/secrets"),
        );
        state.add_environment_var("TEST_VAR".to_string(), "test_value".to_string());

        let yaml = state.to_yaml().expect("Failed to serialize to YAML");
        let deserialized =
            WassetteState::from_yaml(&yaml).expect("Failed to deserialize from YAML");

        assert_eq!(state.version, deserialized.version);
        assert_eq!(state.environment.len(), deserialized.environment.len());
    }

    #[tokio::test]
    async fn test_state_file_operations() {
        let temp_dir = tempfile::tempdir().expect("Failed to create temp dir");
        let json_path = temp_dir.path().join("state.json");
        let yaml_path = temp_dir.path().join("state.yaml");

        let mut state = WassetteState::new(
            PathBuf::from("/tmp/components"),
            PathBuf::from("/tmp/secrets"),
        );
        state.add_environment_var("KEY1".to_string(), "value1".to_string());

        // Test JSON
        state
            .save_to_file(&json_path)
            .await
            .expect("Failed to save JSON");
        let loaded_json = WassetteState::load_from_file(&json_path)
            .await
            .expect("Failed to load JSON");
        assert_eq!(state.version, loaded_json.version);

        // Test YAML
        state
            .save_to_file(&yaml_path)
            .await
            .expect("Failed to save YAML");
        let loaded_yaml = WassetteState::load_from_file(&yaml_path)
            .await
            .expect("Failed to load YAML");
        assert_eq!(state.version, loaded_yaml.version);
    }

    #[test]
    fn test_validate_compatibility() {
        let state = WassetteState::new(
            PathBuf::from("/tmp/components"),
            PathBuf::from("/tmp/secrets"),
        );
        assert!(state.validate_compatibility().is_ok());

        let mut invalid_state = state.clone();
        invalid_state.version = String::new();
        assert!(invalid_state.validate_compatibility().is_err());
    }
}
