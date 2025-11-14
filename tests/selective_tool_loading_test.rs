// Copyright (c) Microsoft Corporation.
// Licensed under the MIT license.

//! Integration tests for selective tool loading

use anyhow::{Context, Result};
use wassette::LifecycleManager;

#[tokio::test]
async fn test_selective_tool_loading() -> Result<()> {
    // Create lifecycle manager
    let tempdir = tempfile::tempdir()?;
    let lifecycle_manager = LifecycleManager::new(&tempdir).await?;

    // Build the fetch-rs component
    let project_root = std::env::current_dir().context("Failed to get current directory")?;
    let fetch_component_path = project_root
        .join("examples")
        .join("fetch-rs")
        .join("target")
        .join("wasm32-wasip2")
        .join("release")
        .join("fetch_rs.wasm");

    // Load component with only specific tools
    let tools_to_load = vec!["fetch".to_string()];
    let outcome = lifecycle_manager
        .load_component_with_tools(
            &format!("file://{}", fetch_component_path.display()),
            Some(&tools_to_load),
        )
        .await
        .context("Failed to load component with selective tools")?;

    // Verify only the specified tool was loaded
    assert_eq!(
        outcome.tool_names.len(),
        1,
        "Expected only 1 tool to be loaded"
    );
    assert_eq!(
        outcome.tool_names[0], "fetch",
        "Expected 'fetch' tool to be loaded"
    );

    // Verify we can list the component and it shows the filtered tools
    let components = lifecycle_manager.list_components().await;
    assert!(
        components.contains(&outcome.component_id),
        "Component should be in the list"
    );

    // Get the component schema and verify it only has the filtered tools
    let schema = lifecycle_manager
        .get_component_schema(&outcome.component_id)
        .await
        .context("Failed to get component schema")?;

    let tools = schema
        .get("tools")
        .and_then(|v| v.as_array())
        .context("Schema should have tools array")?;

    assert_eq!(
        tools.len(),
        1,
        "Schema should only contain 1 tool after filtering"
    );

    // Verify the tool in the schema is 'fetch'
    let tool_name = tools[0]
        .get("name")
        .and_then(|v| v.as_str())
        .context("Tool should have name")?;
    assert_eq!(tool_name, "fetch", "Tool name should be 'fetch'");

    Ok(())
}

#[tokio::test]
async fn test_load_all_tools_when_no_filter() -> Result<()> {
    // Create lifecycle manager
    let tempdir = tempfile::tempdir()?;
    let lifecycle_manager = LifecycleManager::new(&tempdir).await?;

    // Build the fetch-rs component
    let project_root = std::env::current_dir().context("Failed to get current directory")?;
    let fetch_component_path = project_root
        .join("examples")
        .join("fetch-rs")
        .join("target")
        .join("wasm32-wasip2")
        .join("release")
        .join("fetch_rs.wasm");

    // Load component without filter (should load all tools)
    let outcome = lifecycle_manager
        .load_component(&format!("file://{}", fetch_component_path.display()))
        .await
        .context("Failed to load component")?;

    // The fetch-rs component should have at least one tool
    assert!(
        !outcome.tool_names.is_empty(),
        "Component should have at least one tool"
    );

    Ok(())
}

#[tokio::test]
async fn test_empty_tools_filter() -> Result<()> {
    // Create lifecycle manager
    let tempdir = tempfile::tempdir()?;
    let lifecycle_manager = LifecycleManager::new(&tempdir).await?;

    // Build the fetch-rs component
    let project_root = std::env::current_dir().context("Failed to get current directory")?;
    let fetch_component_path = project_root
        .join("examples")
        .join("fetch-rs")
        .join("target")
        .join("wasm32-wasip2")
        .join("release")
        .join("fetch_rs.wasm");

    // Load component with empty filter (should load no tools)
    let tools_to_load: Vec<String> = vec![];
    let outcome = lifecycle_manager
        .load_component_with_tools(
            &format!("file://{}", fetch_component_path.display()),
            Some(&tools_to_load),
        )
        .await
        .context("Failed to load component with empty tools filter")?;

    // Verify no tools were loaded
    assert_eq!(
        outcome.tool_names.len(),
        0,
        "Expected 0 tools to be loaded with empty filter"
    );

    Ok(())
}

#[tokio::test]
async fn test_nonexistent_tool_filter() -> Result<()> {
    // Create lifecycle manager
    let tempdir = tempfile::tempdir()?;
    let lifecycle_manager = LifecycleManager::new(&tempdir).await?;

    // Build the fetch-rs component
    let project_root = std::env::current_dir().context("Failed to get current directory")?;
    let fetch_component_path = project_root
        .join("examples")
        .join("fetch-rs")
        .join("target")
        .join("wasm32-wasip2")
        .join("release")
        .join("fetch_rs.wasm");

    // Load component with filter for non-existent tool
    let tools_to_load = vec!["nonexistent_tool".to_string()];
    let outcome = lifecycle_manager
        .load_component_with_tools(
            &format!("file://{}", fetch_component_path.display()),
            Some(&tools_to_load),
        )
        .await
        .context("Failed to load component")?;

    // Verify no tools were loaded (filter matched nothing)
    assert_eq!(
        outcome.tool_names.len(),
        0,
        "Expected 0 tools when filter doesn't match any tools"
    );

    Ok(())
}
