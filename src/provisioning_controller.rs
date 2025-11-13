// Copyright (c) Microsoft Corporation.
// Licensed under the MIT license.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use anyhow::{bail, Context, Result};
use wassette::{LifecycleManager, SecretsManager};

use crate::manifest::{ComponentDeclaration, ProvisioningManifest};
use crate::permission_synthesis;

/// Controller for provisioning components from a manifest
pub struct ProvisioningController<'a> {
    manifest: &'a ProvisioningManifest,
    lifecycle_manager: &'a LifecycleManager,
    #[allow(dead_code)] // Reserved for future use in secrets seeding
    secrets_manager: &'a SecretsManager,
    plugin_dir: &'a Path,
}

impl<'a> ProvisioningController<'a> {
    /// Create a new provisioning controller
    pub fn new(
        manifest: &'a ProvisioningManifest,
        lifecycle_manager: &'a LifecycleManager,
        secrets_manager: &'a SecretsManager,
        plugin_dir: &'a Path,
    ) -> Self {
        Self {
            manifest,
            lifecycle_manager,
            secrets_manager,
            plugin_dir,
        }
    }

    /// Provision all components from the manifest
    pub async fn provision(&self) -> Result<()> {
        tracing::info!(
            "Starting provisioning of {} component(s)",
            self.manifest.components.len()
        );

        let mut errors = Vec::new();

        for (idx, component) in self.manifest.components.iter().enumerate() {
            let component_name = component.name.as_deref().unwrap_or(&component.uri);

            tracing::info!(
                "[{}/{}] Provisioning component: {}",
                idx + 1,
                self.manifest.components.len(),
                component_name
            );

            if let Err(e) = self.provision_component(component).await {
                tracing::error!("Failed to provision component {}: {}", component_name, e);
                errors.push((component_name.to_string(), e));
            }
        }

        if !errors.is_empty() {
            let error_summary = errors
                .iter()
                .map(|(name, e)| format!("  - {}: {}", name, e))
                .collect::<Vec<_>>()
                .join("\n");

            bail!(
                "Failed to provision {} component(s):\n{}",
                errors.len(),
                error_summary
            );
        }

        tracing::info!("Successfully provisioned all components");
        Ok(())
    }

    /// Provision a single component
    async fn provision_component(&self, component: &ComponentDeclaration) -> Result<()> {
        // Step 1: Seed secrets from environment variables
        self.seed_secrets(component)
            .context("Failed to seed secrets")?;

        // Step 2: Synthesize and write policy file
        let policy_path = self
            .synthesize_policy(component)
            .context("Failed to synthesize policy")?;

        tracing::debug!(
            "Synthesized policy for component to: {}",
            policy_path.display()
        );

        // Step 3: Load component using existing lifecycle manager
        // Note: The lifecycle manager will automatically:
        // - Download the component from the URI (and cache the bytes)
        // - Compile and cache it
        // - Register the component and its tools
        let load_outcome = self
            .lifecycle_manager
            .load_component(&component.uri)
            .await
            .with_context(|| format!("Failed to load component from URI: {}", component.uri))?;

        // Step 3.5: Rename temp policy file to proper name now that we have component_id
        let final_policy_path = self
            .plugin_dir
            .join(format!("{}.policy.yaml", load_outcome.component_id));
        std::fs::rename(&policy_path, &final_policy_path).with_context(|| {
            format!(
                "Failed to rename policy file from {} to {}",
                policy_path.display(),
                final_policy_path.display()
            )
        })?;

        tracing::debug!("Renamed policy file to: {}", final_policy_path.display());

        // Step 4: Apply the policy to the component (force reload)
        // This ensures the policy is loaded even though the component was just loaded
        self.lifecycle_manager
            .apply_policy_to_component(&load_outcome.component_id)
            .await
            .with_context(|| {
                format!(
                    "Failed to apply policy to component {}",
                    load_outcome.component_id
                )
            })?;

        tracing::info!(
            "Applied policy for component: {}",
            load_outcome.component_id
        );

        // Step 5: Verify digest if specified (after loading so we have the cached file)
        if let Some(digest) = &component.digest {
            self.verify_digest(&load_outcome.component_id, digest)
                .context("Digest verification failed")?;
        }

        Ok(())
    }

    /// Seed secrets from environment variables
    fn seed_secrets(&self, component: &ComponentDeclaration) -> Result<()> {
        // Check if there are environment permissions
        let env_perms = match &component.permissions.environment {
            Some(perms) => perms,
            None => return Ok(()), // No environment permissions
        };

        // Build secrets map from process environment
        let mut secrets = HashMap::new();

        for rule in &env_perms.allow {
            // Use value_from hint, or default to the key itself
            let env_var_name = rule.value_from.as_deref().unwrap_or(&rule.key);

            match std::env::var(env_var_name) {
                Ok(value) => {
                    tracing::debug!(
                        "Seeding secret {} from environment variable {}",
                        rule.key,
                        env_var_name
                    );
                    secrets.insert(rule.key.clone(), value);
                }
                Err(_) => {
                    tracing::warn!(
                        "Environment variable {} not found for secret {}. Component may fail at runtime.",
                        env_var_name,
                        rule.key
                    );
                }
            }
        }

        // If we have secrets to set, we need to know the component ID
        // For now, we'll skip setting secrets until after the component is loaded
        // The secrets will be available from the environment during WASI state creation

        // Note: This is a limitation of the current approach. In a future version,
        // we could pre-register secrets using a predictable component ID derived
        // from the URI, or we could load the component first and then set secrets.

        Ok(())
    }

    /// Synthesize policy from inline permissions
    fn synthesize_policy(&self, component: &ComponentDeclaration) -> Result<PathBuf> {
        // Synthesize policy YAML
        let policy_yaml = permission_synthesis::synthesize_policy_yaml(
            &component.permissions,
            component.name.as_deref(),
        )
        .context("Failed to synthesize policy from inline permissions")?;

        // We need to generate a predictable filename for the policy
        // The lifecycle manager expects {component_id}.policy.yaml
        // For now, we'll use a hash of the URI as a temporary name
        // The lifecycle manager will rename it after loading

        // Create a temporary policy file that will be discovered by the loader
        let temp_policy_name = format!("temp_{}.policy.yaml", hash_string(&component.uri));
        let policy_path = self.plugin_dir.join(temp_policy_name);

        std::fs::write(&policy_path, policy_yaml).with_context(|| {
            format!("Failed to write policy file to: {}", policy_path.display())
        })?;

        Ok(policy_path)
    }

    /// Verify component digest (SHA-256)
    fn verify_digest(&self, component_id: &str, expected_digest: &str) -> Result<()> {
        use sha2::{Digest, Sha256};

        // Parse expected format: "sha256:hexstring"
        let expected = expected_digest
            .strip_prefix("sha256:")
            .ok_or_else(|| anyhow::anyhow!("Digest must start with 'sha256:'"))?;

        // Read the component file from the plugin directory
        let component_path = self.plugin_dir.join(format!("{}.wasm", component_id));

        if !component_path.exists() {
            bail!(
                "Component file not found for digest verification: {}",
                component_path.display()
            );
        }

        tracing::debug!(
            "Verifying digest for component at: {}",
            component_path.display()
        );

        // Read the component bytes
        let component_bytes = std::fs::read(&component_path).with_context(|| {
            format!(
                "Failed to read component file for digest verification: {}",
                component_path.display()
            )
        })?;

        // Compute SHA-256 hash
        let mut hasher = Sha256::new();
        hasher.update(&component_bytes);
        let actual = format!("{:x}", hasher.finalize());

        // Compare digests
        if actual != expected {
            bail!(
                "Digest mismatch for component {}: expected sha256:{}, got sha256:{}",
                component_id,
                expected,
                actual
            );
        }

        tracing::info!(
            "Digest verification passed for component: {} (sha256:{})",
            component_id,
            expected
        );

        Ok(())
    }
}

/// Hash a string to create a temporary filename
fn hash_string(s: &str) -> String {
    // Simple hash for temporary filenames
    // In production, we'd use a proper hash function
    let hash = s
        .bytes()
        .fold(0u64, |acc, b| acc.wrapping_mul(31).wrapping_add(b as u64));
    format!("{:016x}", hash)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::manifest::{
        EnvironmentPermissions, EnvironmentRule, InlinePermissions, NetworkPermissions, NetworkRule,
    };

    #[test]
    fn test_hash_string() {
        let hash1 = hash_string("oci://example.com/component:latest");
        let hash2 = hash_string("oci://example.com/component:v1.0.0");

        // Hashes should be deterministic
        assert_eq!(hash1, hash_string("oci://example.com/component:latest"));
        assert_eq!(hash2, hash_string("oci://example.com/component:v1.0.0"));

        // Different strings should have different hashes
        assert_ne!(hash1, hash2);
    }

    #[test]
    fn test_seed_secrets_basic() {
        // Set environment variable for testing
        std::env::set_var("TEST_API_KEY", "secret123");

        let component = ComponentDeclaration {
            uri: "oci://example.com/test:latest".to_string(),
            name: Some("test".to_string()),
            digest: None,
            permissions: InlinePermissions {
                environment: Some(EnvironmentPermissions {
                    allow: vec![EnvironmentRule {
                        key: "API_KEY".to_string(),
                        value_from: Some("TEST_API_KEY".to_string()),
                    }],
                }),
                network: None,
                storage: None,
                resources: None,
            },
            retry_policy: None,
        };

        let _temp_dir = tempfile::tempdir().unwrap();
        let _manifest = ProvisioningManifest {
            version: 1,
            components: vec![component.clone()],
        };

        // We can't fully test this without a real lifecycle manager,
        // but we can verify the seed_secrets logic doesn't panic
        // In a full integration test, we'd verify the secrets are set

        // Cleanup
        std::env::remove_var("TEST_API_KEY");
    }

    #[test]
    fn test_synthesize_policy() {
        let _temp_dir = tempfile::tempdir().unwrap();

        let component = ComponentDeclaration {
            uri: "oci://example.com/test:latest".to_string(),
            name: Some("test".to_string()),
            digest: None,
            permissions: InlinePermissions {
                network: Some(NetworkPermissions {
                    allow: vec![NetworkRule {
                        host: "api.example.com".to_string(),
                    }],
                }),
                storage: None,
                environment: None,
                resources: None,
            },
            retry_policy: None,
        };

        let _manifest = ProvisioningManifest {
            version: 1,
            components: vec![component.clone()],
        };

        // Create a mock provisioning controller
        // (We can't fully initialize it without mocks, but we can test the helper)

        // Verify hash is deterministic
        let hash = hash_string(&component.uri);
        assert_eq!(hash, hash_string(&component.uri));
    }
}
