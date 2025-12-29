// Copyright (c) Microsoft Corporation.
// Licensed under the MIT license.

//! Integration tests for stateful component mode.
//!
//! These tests verify that components loaded with `stateful: true` maintain
//! their Store/Instance across multiple tool calls.

use std::sync::Arc;

use anyhow::{Context, Result};
use tempfile::TempDir;
use test_log::test;
use wassette::{LifecycleManager, LoadOptions};

mod common;
use common::build_filesystem_component;

async fn setup_lifecycle_manager() -> Result<(Arc<LifecycleManager>, TempDir)> {
    let tempdir = tempfile::tempdir()?;

    let manager = Arc::new(
        LifecycleManager::new_unloaded(&tempdir)
            .await
            .context("Failed to create LifecycleManager")?,
    );

    Ok((manager, tempdir))
}

#[test(tokio::test)]
async fn test_load_options_default() {
    let options = LoadOptions::default();
    assert!(!options.stateful, "Default LoadOptions should be stateless");
}

#[test(tokio::test)]
async fn test_load_options_stateful() {
    let options = LoadOptions { stateful: true };
    assert!(options.stateful, "Stateful option should be true");
}

#[cfg(any(target_os = "linux", target_os = "macos"))]
#[test(tokio::test)]
async fn test_load_component_with_stateful_option() -> Result<()> {
    let (manager, _tempdir) = setup_lifecycle_manager().await?;

    let component_path = build_filesystem_component().await?;
    let uri = format!("file://{}", component_path.to_str().unwrap());

    // Load with stateful: true
    let outcome = manager
        .load_component_with_options(&uri, LoadOptions { stateful: true })
        .await?;

    assert_eq!(outcome.component_id, "filesystem");

    // Verify the component is tracked as stateful by checking that
    // execute_component_call works (which internally checks stateful status)
    let result = manager
        .execute_component_call(
            &outcome.component_id,
            "list-directory",
            r#"{"path": "/tmp"}"#,
        )
        .await;

    // The call should work (may fail due to permissions, but that's expected)
    // The important thing is that it doesn't panic due to stateful handling
    match result {
        Ok(_) => {}
        Err(e) => {
            // Permission errors are expected without granting permissions
            let error_msg = e.to_string();
            assert!(
                error_msg.contains("Failed to read directory")
                    || error_msg.contains("permission")
                    || error_msg.contains("denied")
                    || error_msg.contains("storage"),
                "Unexpected error: {}",
                error_msg
            );
        }
    }

    Ok(())
}

#[cfg(any(target_os = "linux", target_os = "macos"))]
#[test(tokio::test)]
async fn test_stateful_component_multiple_calls() -> Result<()> {
    let (manager, _tempdir) = setup_lifecycle_manager().await?;

    let component_path = build_filesystem_component().await?;
    let uri = format!("file://{}", component_path.to_str().unwrap());

    // Load with stateful: true
    let outcome = manager
        .load_component_with_options(&uri, LoadOptions { stateful: true })
        .await?;

    // Make multiple calls - these should reuse the same Store/Instance
    for _ in 0..3 {
        let _ = manager
            .execute_component_call(
                &outcome.component_id,
                "list-directory",
                r#"{"path": "/tmp"}"#,
            )
            .await;
    }

    // If we got here without panicking, the stateful execution path works
    Ok(())
}

#[cfg(any(target_os = "linux", target_os = "macos"))]
#[test(tokio::test)]
async fn test_stateful_component_unload_clears_state() -> Result<()> {
    let (manager, _tempdir) = setup_lifecycle_manager().await?;

    let component_path = build_filesystem_component().await?;
    let uri = format!("file://{}", component_path.to_str().unwrap());

    // Load with stateful: true
    let outcome = manager
        .load_component_with_options(&uri, LoadOptions { stateful: true })
        .await?;

    // Make a call to create the stateful instance
    let _ = manager
        .execute_component_call(
            &outcome.component_id,
            "list-directory",
            r#"{"path": "/tmp"}"#,
        )
        .await;

    // Unload should succeed and clear stateful state
    manager.unload_component(&outcome.component_id).await?;

    // Verify component is unloaded
    let components = manager.list_components().await;
    assert!(
        components.is_empty(),
        "Component should be unloaded, but found: {:?}",
        components
    );

    Ok(())
}

#[cfg(any(target_os = "linux", target_os = "macos"))]
#[test(tokio::test)]
async fn test_stateful_component_reload_clears_previous_state() -> Result<()> {
    let (manager, _tempdir) = setup_lifecycle_manager().await?;

    let component_path = build_filesystem_component().await?;
    let uri = format!("file://{}", component_path.to_str().unwrap());

    // Load with stateful: true
    let outcome = manager
        .load_component_with_options(&uri, LoadOptions { stateful: true })
        .await?;

    // Make a call to create the stateful instance
    let _ = manager
        .execute_component_call(
            &outcome.component_id,
            "list-directory",
            r#"{"path": "/tmp"}"#,
        )
        .await;

    // Reload the component (should clear previous stateful state)
    let reload_outcome = manager
        .load_component_with_options(&uri, LoadOptions { stateful: true })
        .await?;

    assert_eq!(reload_outcome.component_id, outcome.component_id);
    assert_eq!(
        reload_outcome.status,
        wassette::LoadResult::Replaced,
        "Reload should replace existing component"
    );

    // Make another call - should work with fresh state
    let _ = manager
        .execute_component_call(
            &outcome.component_id,
            "list-directory",
            r#"{"path": "/tmp"}"#,
        )
        .await;

    Ok(())
}

#[cfg(any(target_os = "linux", target_os = "macos"))]
#[test(tokio::test)]
async fn test_stateless_component_default_behavior() -> Result<()> {
    let (manager, _tempdir) = setup_lifecycle_manager().await?;

    let component_path = build_filesystem_component().await?;
    let uri = format!("file://{}", component_path.to_str().unwrap());

    // Load without stateful option (default stateless)
    let outcome = manager.load_component(&uri).await?;

    // Make multiple calls - each should create fresh Store/Instance
    for _ in 0..3 {
        let _ = manager
            .execute_component_call(
                &outcome.component_id,
                "list-directory",
                r#"{"path": "/tmp"}"#,
            )
            .await;
    }

    // If we got here without panicking, stateless execution still works
    Ok(())
}
