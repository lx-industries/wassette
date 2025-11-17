// Copyright (c) Microsoft Corporation.
// Licensed under the MIT license.

use anyhow::{Context, Result};
use serde_json::json;
use tempfile::TempDir;
use wassette::LifecycleManager;

mod common;
use common::build_fetch_component;

async fn setup_lifecycle_manager() -> Result<(LifecycleManager, TempDir)> {
    let tempdir = tempfile::tempdir().context("Failed to create temporary directory")?;
    let manager = LifecycleManager::new(&tempdir).await?;
    Ok((manager, tempdir))
}

/// Test that fetch-advanced properly enforces network permissions
#[tokio::test]
async fn test_fetch_advanced_permission_enforcement() -> Result<()> {
    let (manager, _tempdir) = setup_lifecycle_manager().await?;
    let component_path = build_fetch_component().await?;

    let component_id = manager
        .load_component(&format!("file://{}", component_path.to_str().unwrap()))
        .await?
        .component_id;

    let target_url = "https://example.com/";

    println!("Testing fetch-advanced without network permission...");

    let options = json!({
        "method": "get",
        "headers": null,
        "body": null,
        "timeout-secs": 30,
        "follow-redirects": true,
        "max-redirects": 10
    });

    let result = manager
        .execute_component_call(
            &component_id,
            "fetch-advanced",
            &json!({"url": target_url, "options": options}).to_string(),
        )
        .await;

    match result {
        Ok(response) => {
            println!("fetch-advanced response: {response}");

            // Check if the response contains an error indicating the request was blocked
            if response.contains("HttpRequestDenied") || response.contains("Err") {
                println!("✅ Network request properly blocked by policy!");
            } else {
                panic!(
                    "Expected network request to be blocked, but got successful response: {response}"
                );
            }
        }
        Err(e) => {
            let error_msg = e.to_string();
            println!("Error response: {error_msg}");

            // Network should be denied
            assert!(
                error_msg.contains("HttpRequestDenied") || error_msg.contains("permission") || error_msg.contains("denied"),
                "Expected permission-related error, got: {error_msg}"
            );
            println!("✅ Network request properly blocked!");
        }
    }

    Ok(())
}

/// Test that fetch-advanced handles POST method correctly
#[tokio::test]
async fn test_fetch_advanced_post_method() -> Result<()> {
    let (manager, _tempdir) = setup_lifecycle_manager().await?;
    let component_path = build_fetch_component().await?;

    let component_id = manager
        .load_component(&format!("file://{}", component_path.to_str().unwrap()))
        .await?
        .component_id;

    let target_url = "https://example.com/post";

    println!("Testing fetch-advanced POST method...");

    let test_data = json!({"test": "data", "value": 123});
    let options = json!({
        "method": "post",
        "headers": [
            {"name": "Content-Type", "value": "application/json"}
        ],
        "body": test_data.to_string(),
        "timeout-secs": 30,
        "follow-redirects": true,
        "max-redirects": 10
    });

    let result = manager
        .execute_component_call(
            &component_id,
            "fetch-advanced",
            &json!({"url": target_url, "options": options}).to_string(),
        )
        .await;

    // Without network permission, this should fail
    match result {
        Ok(response) => {
            println!("Response: {response}");
            assert!(
                response.contains("HttpRequestDenied") || response.contains("Err"),
                "Expected request to be denied without permission"
            );
            println!("✅ POST method test passed - request properly blocked!");
        }
        Err(e) => {
            let error_msg = e.to_string();
            println!("Error: {error_msg}");
            assert!(
                error_msg.contains("HttpRequestDenied") || error_msg.contains("permission") || error_msg.contains("denied"),
                "Expected permission error"
            );
            println!("✅ POST method test passed - request properly blocked!");
        }
    }

    Ok(())
}

/// Test that fetch-advanced handles HEAD method correctly
#[tokio::test]
async fn test_fetch_advanced_head_method() -> Result<()> {
    let (manager, _tempdir) = setup_lifecycle_manager().await?;
    let component_path = build_fetch_component().await?;

    let component_id = manager
        .load_component(&format!("file://{}", component_path.to_str().unwrap()))
        .await?
        .component_id;

    let target_url = "https://example.com/";

    println!("Testing fetch-advanced HEAD method...");

    let options = json!({
        "method": "head",
        "headers": null,
        "body": null,
        "timeout-secs": 30,
        "follow-redirects": true,
        "max-redirects": 10
    });

    let result = manager
        .execute_component_call(
            &component_id,
            "fetch-advanced",
            &json!({"url": target_url, "options": options}).to_string(),
        )
        .await;

    // Without network permission, this should fail
    match result {
        Ok(response) => {
            println!("Response: {response}");
            assert!(
                response.contains("HttpRequestDenied") || response.contains("Err"),
                "Expected request to be denied without permission"
            );
            println!("✅ HEAD method test passed - request properly blocked!");
        }
        Err(e) => {
            let error_msg = e.to_string();
            println!("Error: {error_msg}");
            assert!(
                error_msg.contains("HttpRequestDenied") || error_msg.contains("permission") || error_msg.contains("denied"),
                "Expected permission error"
            );
            println!("✅ HEAD method test passed - request properly blocked!");
        }
    }

    Ok(())
}

/// Test that fetch-advanced handles custom headers correctly
#[tokio::test]
async fn test_fetch_advanced_with_headers() -> Result<()> {
    let (manager, _tempdir) = setup_lifecycle_manager().await?;
    let component_path = build_fetch_component().await?;

    let component_id = manager
        .load_component(&format!("file://{}", component_path.to_str().unwrap()))
        .await?
        .component_id;

    let target_url = "https://example.com/headers";

    println!("Testing fetch-advanced with custom headers...");

    let options = json!({
        "method": "get",
        "headers": [
            {"name": "X-Custom-Header", "value": "test-value"},
            {"name": "User-Agent", "value": "wassette-test"}
        ],
        "body": null,
        "timeout-secs": 30,
        "follow-redirects": true,
        "max-redirects": 10
    });

    let result = manager
        .execute_component_call(
            &component_id,
            "fetch-advanced",
            &json!({"url": target_url, "options": options}).to_string(),
        )
        .await;

    // Without network permission, this should fail
    match result {
        Ok(response) => {
            println!("Response: {response}");
            assert!(
                response.contains("HttpRequestDenied") || response.contains("Err"),
                "Expected request to be denied without permission"
            );
            println!("✅ Custom headers test passed - request properly blocked!");
        }
        Err(e) => {
            let error_msg = e.to_string();
            println!("Error: {error_msg}");
            assert!(
                error_msg.contains("HttpRequestDenied") || error_msg.contains("permission") || error_msg.contains("denied"),
                "Expected permission error"
            );
            println!("✅ Custom headers test passed - request properly blocked!");
        }
    }

    Ok(())
}
