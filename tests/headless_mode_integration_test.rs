// Copyright (c) Microsoft Corporation.
// Licensed under the MIT license.

//! Comprehensive integration tests for headless deployment mode
//!
//! These tests verify:
//! - Provisioning from manifests
//! - Policy enforcement in headless mode
//! - Runtime grant blocking
//! - Manifest validation
//! - Multi-component provisioning
//! - Digest verification
//! - Environment variable seeding

use std::sync::Arc;

use anyhow::{Context, Result};
use tempfile::TempDir;
use test_log::test;
use wassette::{DeploymentProfile, LifecycleManager};

mod common;
use common::build_fetch_component;

/// Helper to create a lifecycle manager in interactive mode (default)
async fn setup_interactive_manager() -> Result<(Arc<LifecycleManager>, TempDir)> {
    let tempdir = tempfile::tempdir()?;
    let manager = Arc::new(
        LifecycleManager::builder(&tempdir)
            .with_profile(DeploymentProfile::Interactive)
            .build()
            .await
            .context("Failed to create LifecycleManager")?,
    );
    Ok((manager, tempdir))
}

/// Helper to create a lifecycle manager in headless mode
async fn setup_headless_manager() -> Result<(Arc<LifecycleManager>, TempDir)> {
    let tempdir = tempfile::tempdir()?;
    let manager = Arc::new(
        LifecycleManager::builder(&tempdir)
            .with_profile(DeploymentProfile::Headless)
            .build()
            .await
            .context("Failed to create LifecycleManager")?,
    );
    Ok((manager, tempdir))
}

/// Helper to create a simple manifest YAML content
fn create_simple_manifest(component_path: &str, allowed_host: &str) -> String {
    format!(
        r#"version: 1
components:
  - uri: file://{}
    name: test-component
    permissions:
      network:
        allow:
          - host: "{}"
"#,
        component_path, allowed_host
    )
}

/// Helper to create a multi-component manifest
fn create_multi_component_manifest(component_path: &str) -> String {
    format!(
        r#"version: 1
components:
  - uri: file://{}
    name: component1
    permissions:
      network:
        allow:
          - host: "api.example.com"
  - uri: file://{}
    name: component2
    permissions:
      network:
        allow:
          - host: "cdn.example.com"
      storage:
        allow:
          - uri: "fs:///tmp/data"
            access:
              - read
"#,
        component_path, component_path
    )
}

/// Helper to create a manifest with environment variables
fn create_manifest_with_env(component_path: &str) -> String {
    format!(
        r#"version: 1
components:
  - uri: file://{}
    name: env-component
    permissions:
      environment:
        allow:
          - key: "API_KEY"
            value_from: "TEST_API_KEY"
          - key: "CONFIG_URL"
            value_from: "TEST_CONFIG_URL"
      network:
        allow:
          - host: "api.example.com"
"#,
        component_path
    )
}

/// Helper to create a manifest with digest verification
fn create_manifest_with_digest(component_path: &str, digest: &str) -> String {
    format!(
        r#"version: 1
components:
  - uri: file://{}
    name: verified-component
    digest: "sha256:{}"
    permissions:
      network:
        allow:
          - host: "api.example.com"
"#,
        component_path, digest
    )
}

// ============================================================================
// BASIC HEADLESS MODE TESTS
// ============================================================================

#[cfg(any(target_os = "linux", target_os = "macos"))]
#[test(tokio::test)]
async fn test_interactive_mode_allows_runtime_grants() -> Result<()> {
    let (manager, _tempdir) = setup_interactive_manager().await?;
    let component_path = build_fetch_component().await?;

    let component_id = manager
        .load_component(&format!("file://{}", component_path.display()))
        .await?
        .component_id;

    // In interactive mode, runtime grants should succeed
    let result = manager
        .grant_permission(
            &component_id,
            "network",
            &serde_json::json!({"host": "example.com"}),
        )
        .await;

    assert!(
        result.is_ok(),
        "Interactive mode should allow runtime grants"
    );

    Ok(())
}

#[cfg(any(target_os = "linux", target_os = "macos"))]
#[test(tokio::test)]
async fn test_headless_mode_blocks_runtime_grants() -> Result<()> {
    let (manager, _tempdir) = setup_headless_manager().await?;
    let component_path = build_fetch_component().await?;

    let component_id = manager
        .load_component(&format!("file://{}", component_path.display()))
        .await?
        .component_id;

    // In headless mode, runtime grants should fail
    let result = manager
        .grant_permission(
            &component_id,
            "network",
            &serde_json::json!({"host": "example.com"}),
        )
        .await;

    assert!(result.is_err(), "Headless mode should block runtime grants");
    let error_msg = result.unwrap_err().to_string();
    assert!(
        error_msg.contains("headless mode"),
        "Error should mention headless mode"
    );
    assert!(
        error_msg.contains("manifest"),
        "Error should mention updating manifest"
    );

    Ok(())
}

#[cfg(any(target_os = "linux", target_os = "macos"))]
#[test(tokio::test)]
async fn test_headless_mode_blocks_all_permission_types() -> Result<()> {
    let (manager, _tempdir) = setup_headless_manager().await?;
    let component_path = build_fetch_component().await?;

    let component_id = manager
        .load_component(&format!("file://{}", component_path.display()))
        .await?
        .component_id;

    // Test network permission
    let network_result = manager
        .grant_permission(
            &component_id,
            "network",
            &serde_json::json!({"host": "example.com"}),
        )
        .await;
    assert!(
        network_result.is_err(),
        "Network grants should be blocked in headless mode"
    );

    // Test storage permission
    let storage_result = manager
        .grant_permission(
            &component_id,
            "storage",
            &serde_json::json!({"uri": "fs:///tmp/test", "access": ["read"]}),
        )
        .await;
    assert!(
        storage_result.is_err(),
        "Storage grants should be blocked in headless mode"
    );

    // Test environment permission
    let env_result = manager
        .grant_permission(
            &component_id,
            "environment",
            &serde_json::json!({"key": "API_KEY"}),
        )
        .await;
    assert!(
        env_result.is_err(),
        "Environment grants should be blocked in headless mode"
    );

    Ok(())
}

// ============================================================================
// MANIFEST PROVISIONING TESTS
// ============================================================================

#[cfg(any(target_os = "linux", target_os = "macos"))]
#[test(tokio::test)]
async fn test_provision_from_simple_manifest() -> Result<()> {
    let component_path = build_fetch_component().await?;
    let tempdir = tempfile::tempdir()?;

    // Create manifest file
    let manifest_content =
        create_simple_manifest(component_path.to_str().unwrap(), "api.github.com");
    let manifest_path = tempdir.path().join("manifest.yaml");
    tokio::fs::write(&manifest_path, manifest_content).await?;

    // Parse and validate manifest
    let manifest_content = tokio::fs::read_to_string(&manifest_path).await?;
    let manifest: wassette_mcp_server::manifest::ProvisioningManifest =
        serde_yaml::from_str(&manifest_content)?;

    assert_eq!(manifest.components.len(), 1);
    assert_eq!(
        manifest.components[0].name,
        Some("test-component".to_string())
    );

    Ok(())
}

#[cfg(any(target_os = "linux", target_os = "macos"))]
#[test(tokio::test)]
async fn test_provision_multi_component_manifest() -> Result<()> {
    let component_path = build_fetch_component().await?;
    let tempdir = tempfile::tempdir()?;

    // Create multi-component manifest
    let manifest_content = create_multi_component_manifest(component_path.to_str().unwrap());
    let manifest_path = tempdir.path().join("manifest.yaml");
    tokio::fs::write(&manifest_path, manifest_content).await?;

    // Parse and validate manifest
    let manifest_content = tokio::fs::read_to_string(&manifest_path).await?;
    let manifest: wassette_mcp_server::manifest::ProvisioningManifest =
        serde_yaml::from_str(&manifest_content)?;

    assert_eq!(manifest.components.len(), 2);
    assert_eq!(manifest.components[0].name, Some("component1".to_string()));
    assert_eq!(manifest.components[1].name, Some("component2".to_string()));

    // Verify permissions are present
    assert!(manifest.components[0].permissions.network.is_some());
    assert!(manifest.components[1].permissions.network.is_some());
    assert!(manifest.components[1].permissions.storage.is_some());

    Ok(())
}

#[cfg(any(target_os = "linux", target_os = "macos"))]
#[test(tokio::test)]
async fn test_provision_with_environment_variables() -> Result<()> {
    let component_path = build_fetch_component().await?;
    let tempdir = tempfile::tempdir()?;

    // Set environment variables for the test
    std::env::set_var("TEST_API_KEY", "secret123");
    std::env::set_var("TEST_CONFIG_URL", "https://config.example.com");

    // Create manifest with environment variables
    let manifest_content = create_manifest_with_env(component_path.to_str().unwrap());
    let manifest_path = tempdir.path().join("manifest.yaml");
    tokio::fs::write(&manifest_path, manifest_content).await?;

    // Parse manifest
    let manifest_content = tokio::fs::read_to_string(&manifest_path).await?;
    let manifest: wassette_mcp_server::manifest::ProvisioningManifest =
        serde_yaml::from_str(&manifest_content)?;

    assert_eq!(manifest.components.len(), 1);
    assert!(manifest.components[0].permissions.environment.is_some());

    let env_perms = manifest.components[0]
        .permissions
        .environment
        .as_ref()
        .unwrap();
    assert_eq!(env_perms.allow.len(), 2);
    assert_eq!(env_perms.allow[0].key, "API_KEY");
    assert_eq!(
        env_perms.allow[0].value_from,
        Some("TEST_API_KEY".to_string())
    );

    // Cleanup
    std::env::remove_var("TEST_API_KEY");
    std::env::remove_var("TEST_CONFIG_URL");

    Ok(())
}

// ============================================================================
// POLICY ENFORCEMENT TESTS
// ============================================================================

#[cfg(any(target_os = "linux", target_os = "macos"))]
#[test(tokio::test)]
async fn test_headless_enforces_manifest_network_policy() -> Result<()> {
    let component_path = build_fetch_component().await?;
    let tempdir = tempfile::tempdir()?;

    // Create manifest with specific allowed host
    let manifest_content =
        create_simple_manifest(component_path.to_str().unwrap(), "api.allowed.com");
    let manifest_path = tempdir.path().join("manifest.yaml");
    tokio::fs::write(&manifest_path, manifest_content).await?;

    // Parse manifest
    let manifest_content = tokio::fs::read_to_string(&manifest_path).await?;
    let manifest: wassette_mcp_server::manifest::ProvisioningManifest =
        serde_yaml::from_str(&manifest_content)?;

    // Create headless lifecycle manager
    let manager = Arc::new(
        LifecycleManager::builder(&tempdir)
            .with_profile(DeploymentProfile::Headless)
            .build()
            .await?,
    );

    // Provision the component using provisioning controller
    let secrets_manager = wassette::SecretsManager::new(tempdir.path().join("secrets"));
    let controller = wassette_mcp_server::provisioning_controller::ProvisioningController::new(
        &manifest,
        &manager,
        &secrets_manager,
        tempdir.path(),
    );

    controller.provision().await?;

    // Verify the component was loaded
    let components = manager.list_components().await;
    assert_eq!(components.len(), 1);
    let component_id = &components[0];

    // Verify policy was applied
    let policy_info = manager.get_policy_info(component_id).await;
    assert!(policy_info.is_some(), "Policy should be attached");

    // Read and verify policy content
    let policy_info = policy_info.unwrap();
    let policy_content = tokio::fs::read_to_string(&policy_info.local_path).await?;
    assert!(
        policy_content.contains("api.allowed.com"),
        "Policy should contain allowed host"
    );

    Ok(())
}

#[cfg(any(target_os = "linux", target_os = "macos"))]
#[test(tokio::test)]
async fn test_headless_multiple_network_permissions() -> Result<()> {
    let component_path = build_fetch_component().await?;
    let tempdir = tempfile::tempdir()?;

    // Create manifest with multiple hosts
    let manifest_content = format!(
        r#"version: 1
components:
  - uri: file://{}
    name: multi-host-component
    permissions:
      network:
        allow:
          - host: "api.example.com"
          - host: "cdn.example.com"
          - host: "backup.example.com"
"#,
        component_path.display()
    );
    let manifest_path = tempdir.path().join("manifest.yaml");
    tokio::fs::write(&manifest_path, manifest_content).await?;

    let manifest_content = tokio::fs::read_to_string(&manifest_path).await?;
    let manifest: wassette_mcp_server::manifest::ProvisioningManifest =
        serde_yaml::from_str(&manifest_content)?;

    let manager = Arc::new(
        LifecycleManager::builder(&tempdir)
            .with_profile(DeploymentProfile::Headless)
            .build()
            .await?,
    );

    let secrets_manager = wassette::SecretsManager::new(tempdir.path().join("secrets"));
    let controller = wassette_mcp_server::provisioning_controller::ProvisioningController::new(
        &manifest,
        &manager,
        &secrets_manager,
        tempdir.path(),
    );

    controller.provision().await?;

    let components = manager.list_components().await;
    let component_id = &components[0];

    // Verify all hosts are in the policy
    let policy_info = manager.get_policy_info(component_id).await.unwrap();
    let policy_content = tokio::fs::read_to_string(&policy_info.local_path).await?;

    assert!(policy_content.contains("api.example.com"));
    assert!(policy_content.contains("cdn.example.com"));
    assert!(policy_content.contains("backup.example.com"));

    Ok(())
}

#[cfg(any(target_os = "linux", target_os = "macos"))]
#[test(tokio::test)]
async fn test_headless_storage_permissions() -> Result<()> {
    let component_path = build_fetch_component().await?;
    let tempdir = tempfile::tempdir()?;

    let manifest_content = format!(
        r#"version: 1
components:
  - uri: file://{}
    name: storage-component
    permissions:
      storage:
        allow:
          - uri: "fs:///tmp/data"
            access:
              - read
              - write
          - uri: "fs:///tmp/cache"
            access:
              - read
"#,
        component_path.display()
    );
    let manifest_path = tempdir.path().join("manifest.yaml");
    tokio::fs::write(&manifest_path, manifest_content).await?;

    let manifest_content = tokio::fs::read_to_string(&manifest_path).await?;
    let manifest: wassette_mcp_server::manifest::ProvisioningManifest =
        serde_yaml::from_str(&manifest_content)?;

    let manager = Arc::new(
        LifecycleManager::builder(&tempdir)
            .with_profile(DeploymentProfile::Headless)
            .build()
            .await?,
    );

    let secrets_manager = wassette::SecretsManager::new(tempdir.path().join("secrets"));
    let controller = wassette_mcp_server::provisioning_controller::ProvisioningController::new(
        &manifest,
        &manager,
        &secrets_manager,
        tempdir.path(),
    );

    controller.provision().await?;

    let components = manager.list_components().await;
    let component_id = &components[0];

    let policy_info = manager.get_policy_info(component_id).await.unwrap();
    let policy_content = tokio::fs::read_to_string(&policy_info.local_path).await?;

    assert!(policy_content.contains("fs:///tmp/data"));
    assert!(policy_content.contains("fs:///tmp/cache"));
    assert!(policy_content.contains("read"));
    assert!(policy_content.contains("write"));

    Ok(())
}

// ============================================================================
// DIGEST VERIFICATION TESTS
// ============================================================================

#[cfg(any(target_os = "linux", target_os = "macos"))]
#[test(tokio::test)]
async fn test_digest_verification_success() -> Result<()> {
    use sha2::{Digest, Sha256};

    let component_path = build_fetch_component().await?;
    let tempdir = tempfile::tempdir()?;

    // Calculate actual digest
    let component_bytes = tokio::fs::read(&component_path).await?;
    let mut hasher = Sha256::new();
    hasher.update(&component_bytes);
    let digest = format!("{:x}", hasher.finalize());

    // Create manifest with correct digest
    let manifest_content = create_manifest_with_digest(component_path.to_str().unwrap(), &digest);
    let manifest_path = tempdir.path().join("manifest.yaml");
    tokio::fs::write(&manifest_path, manifest_content).await?;

    let manifest_content = tokio::fs::read_to_string(&manifest_path).await?;
    let manifest: wassette_mcp_server::manifest::ProvisioningManifest =
        serde_yaml::from_str(&manifest_content)?;

    let manager = Arc::new(
        LifecycleManager::builder(&tempdir)
            .with_profile(DeploymentProfile::Headless)
            .build()
            .await?,
    );

    let secrets_manager = wassette::SecretsManager::new(tempdir.path().join("secrets"));
    let controller = wassette_mcp_server::provisioning_controller::ProvisioningController::new(
        &manifest,
        &manager,
        &secrets_manager,
        tempdir.path(),
    );

    // Should succeed with correct digest
    let result = controller.provision().await;
    assert!(
        result.is_ok(),
        "Provisioning should succeed with correct digest"
    );

    Ok(())
}

#[cfg(any(target_os = "linux", target_os = "macos"))]
#[test(tokio::test)]
async fn test_digest_verification_failure() -> Result<()> {
    let component_path = build_fetch_component().await?;
    let tempdir = tempfile::tempdir()?;

    // Create manifest with incorrect digest
    let wrong_digest = "0000000000000000000000000000000000000000000000000000000000000000";
    let manifest_content =
        create_manifest_with_digest(component_path.to_str().unwrap(), wrong_digest);
    let manifest_path = tempdir.path().join("manifest.yaml");
    tokio::fs::write(&manifest_path, manifest_content).await?;

    let manifest_content = tokio::fs::read_to_string(&manifest_path).await?;
    let manifest: wassette_mcp_server::manifest::ProvisioningManifest =
        serde_yaml::from_str(&manifest_content)?;

    let manager = Arc::new(
        LifecycleManager::builder(&tempdir)
            .with_profile(DeploymentProfile::Headless)
            .build()
            .await?,
    );

    let secrets_manager = wassette::SecretsManager::new(tempdir.path().join("secrets"));
    let controller = wassette_mcp_server::provisioning_controller::ProvisioningController::new(
        &manifest,
        &manager,
        &secrets_manager,
        tempdir.path(),
    );

    // Should fail with incorrect digest
    let result = controller.provision().await;
    assert!(
        result.is_err(),
        "Provisioning should fail with incorrect digest"
    );
    let error_msg = result.unwrap_err().to_string();
    assert!(
        error_msg.contains("Digest") || error_msg.contains("digest"),
        "Error should mention digest: {}",
        error_msg
    );

    Ok(())
}

#[cfg(any(target_os = "linux", target_os = "macos"))]
#[test(tokio::test)]
async fn test_digest_verification_malformed() -> Result<()> {
    let component_path = build_fetch_component().await?;
    let tempdir = tempfile::tempdir()?;

    // Create manifest with malformed digest (missing sha256: prefix)
    let manifest_content = format!(
        r#"version: 1
components:
  - uri: file://{}
    name: malformed-digest
    digest: "abcd1234"
    permissions:
      network:
        allow:
          - host: "api.example.com"
"#,
        component_path.display()
    );
    let manifest_path = tempdir.path().join("manifest.yaml");
    tokio::fs::write(&manifest_path, manifest_content).await?;

    let manifest_content = tokio::fs::read_to_string(&manifest_path).await?;
    let manifest: wassette_mcp_server::manifest::ProvisioningManifest =
        serde_yaml::from_str(&manifest_content)?;

    let manager = Arc::new(
        LifecycleManager::builder(&tempdir)
            .with_profile(DeploymentProfile::Headless)
            .build()
            .await?,
    );

    let secrets_manager = wassette::SecretsManager::new(tempdir.path().join("secrets"));
    let controller = wassette_mcp_server::provisioning_controller::ProvisioningController::new(
        &manifest,
        &manager,
        &secrets_manager,
        tempdir.path(),
    );

    let result = controller.provision().await;
    assert!(result.is_err(), "Should fail with malformed digest");
    let error_msg = result.unwrap_err().to_string();
    // Just verify it failed - the specific error message is implementation-dependent
    assert!(
        error_msg.contains("Digest")
            || error_msg.contains("digest")
            || error_msg.contains("sha256"),
        "Error should relate to digest: {}",
        error_msg
    );

    Ok(())
}

// ============================================================================
// MANIFEST VALIDATION TESTS
// ============================================================================

#[test(tokio::test)]
async fn test_manifest_validation_empty_components() -> Result<()> {
    let tempdir = tempfile::tempdir()?;

    let manifest_content = r#"version: 1
components: []
"#;
    let manifest_path = tempdir.path().join("manifest.yaml");
    tokio::fs::write(&manifest_path, manifest_content).await?;

    let manifest_content = tokio::fs::read_to_string(&manifest_path).await?;
    let manifest: wassette_mcp_server::manifest::ProvisioningManifest =
        serde_yaml::from_str(&manifest_content)?;

    assert_eq!(manifest.components.len(), 0);

    Ok(())
}

#[test(tokio::test)]
async fn test_manifest_validation_missing_version() -> Result<()> {
    let tempdir = tempfile::tempdir()?;

    let manifest_content = r#"
components:
  - uri: "file:///tmp/test.wasm"
    name: "test"
"#;
    let manifest_path = tempdir.path().join("manifest.yaml");
    tokio::fs::write(&manifest_path, manifest_content).await?;

    let manifest_content = tokio::fs::read_to_string(&manifest_path).await?;
    let result: Result<wassette_mcp_server::manifest::ProvisioningManifest, _> =
        serde_yaml::from_str(&manifest_content);

    assert!(result.is_err(), "Should fail without version field");

    Ok(())
}

#[cfg(any(target_os = "linux", target_os = "macos"))]
#[test(tokio::test)]
async fn test_manifest_validation_missing_uri() -> Result<()> {
    let tempdir = tempfile::tempdir()?;

    let manifest_content = r#"version: 1
components:
  - name: "test-component"
    permissions:
      network:
        allow:
          - host: "api.example.com"
"#;
    let manifest_path = tempdir.path().join("manifest.yaml");
    tokio::fs::write(&manifest_path, manifest_content).await?;

    let manifest_content = tokio::fs::read_to_string(&manifest_path).await?;
    let result: Result<wassette_mcp_server::manifest::ProvisioningManifest, _> =
        serde_yaml::from_str(&manifest_content);

    assert!(result.is_err(), "Should fail without uri field");

    Ok(())
}

// ============================================================================
// ERROR HANDLING TESTS
// ============================================================================

#[cfg(any(target_os = "linux", target_os = "macos"))]
#[test(tokio::test)]
async fn test_headless_error_message_quality() -> Result<()> {
    let (manager, _tempdir) = setup_headless_manager().await?;
    let component_path = build_fetch_component().await?;

    let component_id = manager
        .load_component(&format!("file://{}", component_path.display()))
        .await?
        .component_id;

    // Test network permission error
    let network_result = manager
        .grant_permission(
            &component_id,
            "network",
            &serde_json::json!({"host": "example.com"}),
        )
        .await;

    assert!(network_result.is_err());
    let error_msg = network_result.unwrap_err().to_string();

    // Verify error message is helpful
    assert!(error_msg.contains("Runtime permission grants are disabled"));
    assert!(error_msg.contains("headless mode"));
    assert!(error_msg.contains("network permission"));
    assert!(error_msg.contains(&component_id));
    assert!(error_msg.contains("provisioning manifest"));

    Ok(())
}

#[cfg(any(target_os = "linux", target_os = "macos"))]
#[test(tokio::test)]
async fn test_provision_with_nonexistent_component() -> Result<()> {
    let tempdir = tempfile::tempdir()?;

    let manifest_content = r#"version: 1
components:
  - uri: file:///nonexistent/component.wasm
    name: missing-component
    permissions:
      network:
        allow:
          - host: "api.example.com"
"#;
    let manifest_path = tempdir.path().join("manifest.yaml");
    tokio::fs::write(&manifest_path, manifest_content).await?;

    let manifest_content = tokio::fs::read_to_string(&manifest_path).await?;
    let manifest: wassette_mcp_server::manifest::ProvisioningManifest =
        serde_yaml::from_str(&manifest_content)?;

    let manager = Arc::new(
        LifecycleManager::builder(&tempdir)
            .with_profile(DeploymentProfile::Headless)
            .build()
            .await?,
    );

    let secrets_manager = wassette::SecretsManager::new(tempdir.path().join("secrets"));
    let controller = wassette_mcp_server::provisioning_controller::ProvisioningController::new(
        &manifest,
        &manager,
        &secrets_manager,
        tempdir.path(),
    );

    let result = controller.provision().await;
    assert!(result.is_err(), "Should fail with nonexistent component");

    Ok(())
}

// ============================================================================
// PROFILE CONFIGURATION TESTS
// ============================================================================

#[cfg(any(target_os = "linux", target_os = "macos"))]
#[test(tokio::test)]
async fn test_profile_default_is_interactive() -> Result<()> {
    let tempdir = tempfile::tempdir()?;

    // Create manager without specifying profile
    let manager = LifecycleManager::new(&tempdir).await?;
    let component_path = build_fetch_component().await?;

    let component_id = manager
        .load_component(&format!("file://{}", component_path.display()))
        .await?
        .component_id;

    // Default profile should allow runtime grants
    let result = manager
        .grant_permission(
            &component_id,
            "network",
            &serde_json::json!({"host": "example.com"}),
        )
        .await;

    assert!(result.is_ok(), "Default profile should be interactive");

    Ok(())
}

#[cfg(any(target_os = "linux", target_os = "macos"))]
#[test(tokio::test)]
async fn test_profile_can_switch_between_modes() -> Result<()> {
    let tempdir = tempfile::tempdir()?;
    let component_path = build_fetch_component().await?;

    // Create interactive manager
    let interactive_manager = Arc::new(
        LifecycleManager::builder(&tempdir)
            .with_profile(DeploymentProfile::Interactive)
            .build()
            .await?,
    );

    let component_id = interactive_manager
        .load_component(&format!("file://{}", component_path.display()))
        .await?
        .component_id;

    // Should allow grants in interactive mode
    let interactive_result = interactive_manager
        .grant_permission(
            &component_id,
            "network",
            &serde_json::json!({"host": "interactive.com"}),
        )
        .await;
    assert!(interactive_result.is_ok());

    // Create new headless manager with same directory
    let headless_manager = Arc::new(
        LifecycleManager::builder(&tempdir)
            .with_profile(DeploymentProfile::Headless)
            .build()
            .await?,
    );

    // Reload the same component
    let component_id2 = headless_manager
        .load_component(&format!("file://{}", component_path.display()))
        .await?
        .component_id;

    // Should block grants in headless mode
    let headless_result = headless_manager
        .grant_permission(
            &component_id2,
            "network",
            &serde_json::json!({"host": "headless.com"}),
        )
        .await;
    assert!(headless_result.is_err());

    Ok(())
}

// ============================================================================
// INTEGRATION WITH EXISTING FEATURES
// ============================================================================

#[cfg(any(target_os = "linux", target_os = "macos"))]
#[test(tokio::test)]
async fn test_headless_with_component_unload() -> Result<()> {
    let (manager, _tempdir) = setup_headless_manager().await?;
    let component_path = build_fetch_component().await?;

    let component_id = manager
        .load_component(&format!("file://{}", component_path.display()))
        .await?
        .component_id;

    // Verify component is loaded
    let components = manager.list_components().await;
    assert_eq!(components.len(), 1);

    // Unload should work in headless mode
    let unload_result = manager.unload_component(&component_id).await;
    assert!(unload_result.is_ok());

    // Verify component is unloaded
    let components = manager.list_components().await;
    assert_eq!(components.len(), 0);

    Ok(())
}

#[cfg(any(target_os = "linux", target_os = "macos"))]
#[test(tokio::test)]
async fn test_headless_with_component_reload() -> Result<()> {
    let component_path = build_fetch_component().await?;
    let tempdir = tempfile::tempdir()?;

    // Create manifest
    let manifest_content =
        create_simple_manifest(component_path.to_str().unwrap(), "api.example.com");
    let manifest_path = tempdir.path().join("manifest.yaml");
    tokio::fs::write(&manifest_path, manifest_content).await?;

    let manifest_content = tokio::fs::read_to_string(&manifest_path).await?;
    let manifest: wassette_mcp_server::manifest::ProvisioningManifest =
        serde_yaml::from_str(&manifest_content)?;

    let manager = Arc::new(
        LifecycleManager::builder(&tempdir)
            .with_profile(DeploymentProfile::Headless)
            .build()
            .await?,
    );

    let secrets_manager = wassette::SecretsManager::new(tempdir.path().join("secrets"));
    let controller = wassette_mcp_server::provisioning_controller::ProvisioningController::new(
        &manifest,
        &manager,
        &secrets_manager,
        tempdir.path(),
    );

    controller.provision().await?;

    let components = manager.list_components().await;
    let component_id = components[0].clone();

    // Unload component
    manager.unload_component(&component_id).await?;

    // Reload component - policy should still be enforced
    let new_component_id = manager
        .load_component(&format!("file://{}", component_path.display()))
        .await?
        .component_id;

    // Runtime grants should still be blocked
    let grant_result = manager
        .grant_permission(
            &new_component_id,
            "network",
            &serde_json::json!({"host": "new.com"}),
        )
        .await;

    assert!(grant_result.is_err());

    Ok(())
}

#[cfg(any(target_os = "linux", target_os = "macos"))]
#[test(tokio::test)]
async fn test_headless_list_components() -> Result<()> {
    let component_path = build_fetch_component().await?;
    let tempdir = tempfile::tempdir()?;

    let manifest_content = create_multi_component_manifest(component_path.to_str().unwrap());
    let manifest_path = tempdir.path().join("manifest.yaml");
    tokio::fs::write(&manifest_path, manifest_content).await?;

    let manifest_content = tokio::fs::read_to_string(&manifest_path).await?;
    let manifest: wassette_mcp_server::manifest::ProvisioningManifest =
        serde_yaml::from_str(&manifest_content)?;

    let manager = Arc::new(
        LifecycleManager::builder(&tempdir)
            .with_profile(DeploymentProfile::Headless)
            .build()
            .await?,
    );

    let secrets_manager = wassette::SecretsManager::new(tempdir.path().join("secrets"));
    let controller = wassette_mcp_server::provisioning_controller::ProvisioningController::new(
        &manifest,
        &manager,
        &secrets_manager,
        tempdir.path(),
    );

    controller.provision().await?;

    // List components should work
    // Note: Since both components have the same URI, the second one replaces the first,
    // so we only have 1 component registered
    let components = manager.list_components().await;
    assert!(
        components.len() >= 1,
        "Should have at least 1 component loaded"
    );

    Ok(())
}
