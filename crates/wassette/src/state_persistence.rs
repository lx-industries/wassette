// Copyright (c) Microsoft Corporation.
// Licensed under the MIT license.

//! State persistence for wassette
//!
//! This module provides functionality to export and import wassette state,
//! enabling state transfer between agents or backup/restore scenarios.
//!
//! # Use Cases
//!
//! 1. **Agent Handoff**: Transfer running state from one agent to another
//! 2. **Backup/Restore**: Save wassette state for disaster recovery
//! 3. **Environment Migration**: Move state between dev/staging/production
//! 4. **Collaboration**: Share working configurations between team members
//!
//! # Security Considerations
//!
//! - Secrets are NOT included in state snapshots by default
//! - Sensitive data must be explicitly included with proper encryption
//! - State files should be treated as security-sensitive artifacts
//! - Policy files are included to maintain permission configurations
//!
//! # State Components
//!
//! The following state is captured:
//!
//! - Component registry (loaded components and metadata)
//! - Policy configurations (permissions per component)
//! - Component storage (cached component files)
//! - Tool metadata (registered tools and schemas)
//! - (Optional) Secrets (encrypted, opt-in only)

use std::collections::HashMap;
use std::path::Path;

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

use crate::ComponentMetadata;

/// Complete snapshot of wassette state
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StateSnapshot {
    /// Schema version for compatibility checking
    pub version: u32,

    /// Timestamp when snapshot was created
    pub created_at: u64,

    /// Component registry state
    pub components: Vec<ComponentState>,

    /// Optional metadata about the snapshot
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<SnapshotMetadata>,
}

/// Metadata about the snapshot
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SnapshotMetadata {
    /// Human-readable description
    pub description: Option<String>,

    /// Wassette version that created this snapshot
    pub wassette_version: Option<String>,

    /// Hostname or source identifier
    pub source: Option<String>,

    /// Custom tags for organization
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tags: Option<HashMap<String, String>>,
}

/// State for a single component
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComponentState {
    /// Component identifier
    pub component_id: String,

    /// Source URI where component was loaded from
    pub source_uri: String,

    /// Component metadata (tools, schemas, etc.)
    pub metadata: ComponentMetadata,

    /// Policy configuration
    #[serde(skip_serializing_if = "Option::is_none")]
    pub policy: Option<PolicyState>,

    /// Binary data (base64 encoded)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub binary_data: Option<String>,

    /// Whether to include the component binary in the snapshot
    #[serde(skip_serializing_if = "Option::is_none")]
    pub include_binary: Option<bool>,
}

/// Policy state for a component
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PolicyState {
    /// Policy document content (YAML)
    pub content: String,

    /// Policy source URI
    pub source_uri: String,

    /// Policy creation timestamp
    pub created_at: u64,
}

/// Options for creating a state snapshot
#[derive(Debug, Clone, Default)]
pub struct SnapshotOptions {
    /// Include component binaries in snapshot (increases size significantly)
    pub include_binaries: bool,

    /// Include secrets (requires encryption key)
    pub include_secrets: bool,

    /// Encryption key for secrets (required if include_secrets is true)
    pub encryption_key: Option<String>,

    /// Filter components by ID (None = all components)
    pub component_filter: Option<Vec<String>>,

    /// Custom metadata to attach
    pub metadata: Option<SnapshotMetadata>,
}

/// Options for restoring from a state snapshot
#[derive(Debug, Clone, Default)]
pub struct RestoreOptions {
    /// Skip components that already exist
    pub skip_existing: bool,

    /// Decryption key for secrets
    pub decryption_key: Option<String>,

    /// Only restore specific components
    pub component_filter: Option<Vec<String>>,

    /// Verify component checksums before restoring
    pub verify_checksums: bool,
}

impl StateSnapshot {
    /// Create a new empty snapshot
    pub fn new() -> Self {
        Self {
            version: 1,
            created_at: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs(),
            components: Vec::new(),
            metadata: None,
        }
    }

    /// Validate the snapshot structure
    pub fn validate(&self) -> Result<()> {
        if self.version != 1 {
            anyhow::bail!("Unsupported snapshot version: {}", self.version);
        }

        // Check for duplicate component IDs
        let mut seen_ids = std::collections::HashSet::new();
        for component in &self.components {
            if !seen_ids.insert(&component.component_id) {
                anyhow::bail!(
                    "Duplicate component ID in snapshot: {}",
                    component.component_id
                );
            }
        }

        Ok(())
    }

    /// Serialize snapshot to JSON
    pub fn to_json(&self) -> Result<String> {
        serde_json::to_string_pretty(self).context("Failed to serialize snapshot to JSON")
    }

    /// Deserialize snapshot from JSON
    pub fn from_json(json: &str) -> Result<Self> {
        let snapshot: Self = serde_json::from_str(json).context("Failed to parse snapshot JSON")?;
        snapshot.validate()?;
        Ok(snapshot)
    }

    /// Save snapshot to a file
    pub async fn save_to_file(&self, path: impl AsRef<Path>) -> Result<()> {
        let json = self.to_json()?;
        tokio::fs::write(path.as_ref(), json)
            .await
            .with_context(|| format!("Failed to write snapshot to {}", path.as_ref().display()))?;
        Ok(())
    }

    /// Load snapshot from a file
    pub async fn load_from_file(path: impl AsRef<Path>) -> Result<Self> {
        let json = tokio::fs::read_to_string(path.as_ref())
            .await
            .with_context(|| format!("Failed to read snapshot from {}", path.as_ref().display()))?;
        Self::from_json(&json)
    }
}

impl Default for StateSnapshot {
    fn default() -> Self {
        Self::new()
    }
}

impl ComponentState {
    /// Create a new component state
    pub fn new(component_id: String, source_uri: String, metadata: ComponentMetadata) -> Self {
        Self {
            component_id,
            source_uri,
            metadata,
            policy: None,
            binary_data: None,
            include_binary: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_snapshot_creation() {
        let snapshot = StateSnapshot::new();
        assert_eq!(snapshot.version, 1);
        assert!(snapshot.components.is_empty());
    }

    #[test]
    fn test_snapshot_validation() {
        use crate::ValidationStamp;

        let mut snapshot = StateSnapshot::new();

        // Add a component
        let metadata = ComponentMetadata {
            component_id: "test-component".to_string(),
            tool_schemas: vec![],
            function_identifiers: vec![],
            tool_names: vec![],
            validation_stamp: ValidationStamp {
                file_size: 1024,
                mtime: 0,
                content_hash: None,
            },
            created_at: 0,
        };

        snapshot.components.push(ComponentState::new(
            "test-component".to_string(),
            "oci://example.com/test:latest".to_string(),
            metadata.clone(),
        ));

        // Should validate successfully
        snapshot.validate().unwrap();

        // Add duplicate component ID
        snapshot.components.push(ComponentState::new(
            "test-component".to_string(),
            "oci://example.com/test2:latest".to_string(),
            metadata,
        ));

        // Should fail validation
        assert!(snapshot.validate().is_err());
    }

    #[test]
    fn test_snapshot_serialization() {
        let snapshot = StateSnapshot::new();
        let json = snapshot.to_json().unwrap();

        // Should be valid JSON
        let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed["version"], 1);
    }

    #[test]
    fn test_snapshot_deserialization() {
        let json = r#"{
            "version": 1,
            "created_at": 1234567890,
            "components": []
        }"#;

        let snapshot = StateSnapshot::from_json(json).unwrap();
        assert_eq!(snapshot.version, 1);
        assert_eq!(snapshot.created_at, 1234567890);
    }

    #[tokio::test]
    async fn test_snapshot_file_operations() {
        let temp_dir = tempfile::tempdir().unwrap();
        let snapshot_path = temp_dir.path().join("snapshot.json");

        let snapshot = StateSnapshot::new();

        // Save to file
        snapshot.save_to_file(&snapshot_path).await.unwrap();

        // Load from file
        let loaded = StateSnapshot::load_from_file(&snapshot_path).await.unwrap();
        assert_eq!(loaded.version, snapshot.version);
    }
}
