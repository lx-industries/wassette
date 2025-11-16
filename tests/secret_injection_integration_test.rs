// Copyright (c) Microsoft Corporation.
// Licensed under the MIT license.

//! Integration tests for dynamic secret injection

use std::sync::Arc;

use anyhow::Result;
use tempfile::TempDir;
use wassette::SecretsManager;

#[tokio::test]
async fn test_inject_and_retrieve_secret() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let secrets_dir = temp_dir.path().join("secrets");

    let secrets_manager = Arc::new(SecretsManager::new(secrets_dir));
    secrets_manager.ensure_secrets_dir().await?;

    // Inject a secret into memory
    secrets_manager
        .inject_secret(
            "test-component",
            "API_KEY".to_string(),
            "secret123".to_string(),
        )
        .await?;

    // Retrieve all secrets (should include the injected one)
    let all_secrets = secrets_manager.get_all_secrets("test-component").await?;
    assert_eq!(all_secrets.get("API_KEY"), Some(&"secret123".to_string()));

    Ok(())
}

#[tokio::test]
async fn test_memory_secret_precedence_over_file() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let secrets_dir = temp_dir.path().join("secrets");

    let secrets_manager = Arc::new(SecretsManager::new(secrets_dir));
    secrets_manager.ensure_secrets_dir().await?;

    // Set a file-based secret
    secrets_manager
        .set_component_secrets(
            "test-component",
            &[("API_KEY".to_string(), "file_value".to_string())],
        )
        .await?;

    // Inject a memory secret with the same key
    secrets_manager
        .inject_secret(
            "test-component",
            "API_KEY".to_string(),
            "memory_value".to_string(),
        )
        .await?;

    // Retrieve all secrets - memory should override file
    let all_secrets = secrets_manager.get_all_secrets("test-component").await?;
    assert_eq!(
        all_secrets.get("API_KEY"),
        Some(&"memory_value".to_string())
    );

    Ok(())
}

#[tokio::test]
async fn test_remove_memory_secret() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let secrets_dir = temp_dir.path().join("secrets");

    let secrets_manager = Arc::new(SecretsManager::new(secrets_dir));
    secrets_manager.ensure_secrets_dir().await?;

    // Inject a secret
    secrets_manager
        .inject_secret(
            "test-component",
            "API_KEY".to_string(),
            "secret123".to_string(),
        )
        .await?;

    // Verify it's there
    let all_secrets = secrets_manager.get_all_secrets("test-component").await?;
    assert!(all_secrets.contains_key("API_KEY"));

    // Remove it
    secrets_manager
        .remove_memory_secret("test-component", "API_KEY")
        .await?;

    // Verify it's gone
    let all_secrets = secrets_manager.get_all_secrets("test-component").await?;
    assert!(!all_secrets.contains_key("API_KEY"));

    Ok(())
}

#[tokio::test]
async fn test_list_all_secrets_combines_file_and_memory() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let secrets_dir = temp_dir.path().join("secrets");

    let secrets_manager = Arc::new(SecretsManager::new(secrets_dir));
    secrets_manager.ensure_secrets_dir().await?;

    // Set a file-based secret
    secrets_manager
        .set_component_secrets(
            "test-component",
            &[("FILE_KEY".to_string(), "file_value".to_string())],
        )
        .await?;

    // Inject a memory secret
    secrets_manager
        .inject_secret(
            "test-component",
            "MEMORY_KEY".to_string(),
            "memory_value".to_string(),
        )
        .await?;

    // List all secrets
    let all_secrets = secrets_manager
        .list_all_secrets("test-component", false)
        .await?;

    assert_eq!(all_secrets.len(), 2);
    assert!(all_secrets.contains_key("FILE_KEY"));
    assert!(all_secrets.contains_key("MEMORY_KEY"));

    Ok(())
}

#[tokio::test]
async fn test_list_all_secrets_with_values() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let secrets_dir = temp_dir.path().join("secrets");

    let secrets_manager = Arc::new(SecretsManager::new(secrets_dir));
    secrets_manager.ensure_secrets_dir().await?;

    // Set a file-based secret
    secrets_manager
        .set_component_secrets(
            "test-component",
            &[("FILE_KEY".to_string(), "file_value".to_string())],
        )
        .await?;

    // Inject a memory secret
    secrets_manager
        .inject_secret(
            "test-component",
            "MEMORY_KEY".to_string(),
            "memory_value".to_string(),
        )
        .await?;

    // List all secrets with values
    let all_secrets = secrets_manager
        .list_all_secrets("test-component", true)
        .await?;

    assert_eq!(all_secrets.len(), 2);
    assert_eq!(
        all_secrets.get("FILE_KEY"),
        Some(&Some("file_value".to_string()))
    );
    assert_eq!(
        all_secrets.get("MEMORY_KEY"),
        Some(&Some("memory_value".to_string()))
    );

    Ok(())
}

#[tokio::test]
async fn test_multiple_components_isolated_secrets() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let secrets_dir = temp_dir.path().join("secrets");

    let secrets_manager = Arc::new(SecretsManager::new(secrets_dir));
    secrets_manager.ensure_secrets_dir().await?;

    // Inject secrets for component1
    secrets_manager
        .inject_secret("component1", "API_KEY".to_string(), "secret1".to_string())
        .await?;

    // Inject secrets for component2
    secrets_manager
        .inject_secret("component2", "API_KEY".to_string(), "secret2".to_string())
        .await?;

    // Verify component1 secrets
    let comp1_secrets = secrets_manager.get_all_secrets("component1").await?;
    assert_eq!(comp1_secrets.get("API_KEY"), Some(&"secret1".to_string()));

    // Verify component2 secrets
    let comp2_secrets = secrets_manager.get_all_secrets("component2").await?;
    assert_eq!(comp2_secrets.get("API_KEY"), Some(&"secret2".to_string()));

    Ok(())
}
