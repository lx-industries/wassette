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

#[tokio::test]
async fn test_fetch_advanced_get_method() -> Result<()> {
    let (manager, _tempdir) = setup_lifecycle_manager().await?;
    let component_path = build_fetch_component().await?;

    let component_id = manager
        .load_component(&format!("file://{}", component_path.to_str().unwrap()))
        .await?
        .component_id;

    // Grant network permission for httpbin.org
    manager
        .grant_permission(
            &component_id,
            "network",
            &json!({"host": "httpbin.org"}),
        )
        .await?;

    let target_url = "https://httpbin.org/get";

    println!("Testing fetch-advanced with GET method on {target_url}...");

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
            
            // Parse the response to check status and body
            let response_json: serde_json::Value = serde_json::from_str(&response)?;
            assert!(response_json.get("Ok").is_some(), "Expected Ok response");
            
            let response_data = response_json.get("Ok").unwrap();
            let status = response_data.get("status").and_then(|v| v.as_u64());
            assert_eq!(status, Some(200), "Expected status 200");
            
            let body = response_data.get("body").and_then(|v| v.as_str());
            assert!(body.is_some(), "Expected response body");
            assert!(body.unwrap().contains("httpbin"), "Expected httpbin in response body");
            
            println!("✅ fetch-advanced GET method test passed!");
        }
        Err(e) => {
            panic!("Expected successful fetch-advanced call, got error: {e}");
        }
    }

    Ok(())
}

#[tokio::test]
async fn test_fetch_advanced_with_custom_headers() -> Result<()> {
    let (manager, _tempdir) = setup_lifecycle_manager().await?;
    let component_path = build_fetch_component().await?;

    let component_id = manager
        .load_component(&format!("file://{}", component_path.to_str().unwrap()))
        .await?
        .component_id;

    // Grant network permission
    manager
        .grant_permission(
            &component_id,
            "network",
            &json!({"host": "httpbin.org"}),
        )
        .await?;

    let target_url = "https://httpbin.org/headers";

    println!("Testing fetch-advanced with custom headers on {target_url}...");

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

    match result {
        Ok(response) => {
            println!("fetch-advanced with headers response: {response}");
            
            let response_json: serde_json::Value = serde_json::from_str(&response)?;
            assert!(response_json.get("Ok").is_some(), "Expected Ok response");
            
            let response_data = response_json.get("Ok").unwrap();
            let body = response_data.get("body").and_then(|v| v.as_str()).unwrap_or("");
            
            // httpbin.org/headers echoes back the headers we sent
            assert!(body.contains("X-Custom-Header") || body.contains("x-custom-header"), 
                   "Expected custom header in response body");
            
            println!("✅ fetch-advanced custom headers test passed!");
        }
        Err(e) => {
            panic!("Expected successful fetch-advanced call, got error: {e}");
        }
    }

    Ok(())
}

#[tokio::test]
async fn test_fetch_advanced_redirect_following() -> Result<()> {
    let (manager, _tempdir) = setup_lifecycle_manager().await?;
    let component_path = build_fetch_component().await?;

    let component_id = manager
        .load_component(&format!("file://{}", component_path.to_str().unwrap()))
        .await?
        .component_id;

    // Grant network permission
    manager
        .grant_permission(
            &component_id,
            "network",
            &json!({"host": "httpbin.org"}),
        )
        .await?;

    // httpbin.org/redirect/2 will redirect twice before returning
    let target_url = "https://httpbin.org/redirect/2";

    println!("Testing fetch-advanced redirect following on {target_url}...");

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
            println!("fetch-advanced redirect response: {response}");
            
            let response_json: serde_json::Value = serde_json::from_str(&response)?;
            assert!(response_json.get("Ok").is_some(), "Expected Ok response after redirects");
            
            let response_data = response_json.get("Ok").unwrap();
            let status = response_data.get("status").and_then(|v| v.as_u64());
            assert_eq!(status, Some(200), "Expected final status 200 after redirects");
            
            println!("✅ fetch-advanced redirect following test passed!");
        }
        Err(e) => {
            panic!("Expected successful fetch-advanced call with redirects, got error: {e}");
        }
    }

    Ok(())
}

#[tokio::test]
async fn test_fetch_advanced_no_redirect() -> Result<()> {
    let (manager, _tempdir) = setup_lifecycle_manager().await?;
    let component_path = build_fetch_component().await?;

    let component_id = manager
        .load_component(&format!("file://{}", component_path.to_str().unwrap()))
        .await?
        .component_id;

    // Grant network permission
    manager
        .grant_permission(
            &component_id,
            "network",
            &json!({"host": "httpbin.org"}),
        )
        .await?;

    let target_url = "https://httpbin.org/redirect/1";

    println!("Testing fetch-advanced with redirects disabled on {target_url}...");

    let options = json!({
        "method": "get",
        "headers": null,
        "body": null,
        "timeout-secs": 30,
        "follow-redirects": false,
        "max-redirects": 0
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
            println!("fetch-advanced no-redirect response: {response}");
            
            let response_json: serde_json::Value = serde_json::from_str(&response)?;
            assert!(response_json.get("Ok").is_some(), "Expected Ok response");
            
            let response_data = response_json.get("Ok").unwrap();
            let status = response_data.get("status").and_then(|v| v.as_u64());
            
            // Should get redirect status code (302 or similar)
            assert!(
                status == Some(302) || status == Some(301) || status == Some(307),
                "Expected redirect status code, got: {:?}", status
            );
            
            println!("✅ fetch-advanced no-redirect test passed!");
        }
        Err(e) => {
            panic!("Expected successful fetch-advanced call, got error: {e}");
        }
    }

    Ok(())
}

#[tokio::test]
async fn test_fetch_advanced_status_204() -> Result<()> {
    let (manager, _tempdir) = setup_lifecycle_manager().await?;
    let component_path = build_fetch_component().await?;

    let component_id = manager
        .load_component(&format!("file://{}", component_path.to_str().unwrap()))
        .await?
        .component_id;

    // Grant network permission
    manager
        .grant_permission(
            &component_id,
            "network",
            &json!({"host": "httpbin.org"}),
        )
        .await?;

    let target_url = "https://httpbin.org/status/204";

    println!("Testing fetch-advanced with 204 No Content on {target_url}...");

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
            println!("fetch-advanced 204 response: {response}");
            
            let response_json: serde_json::Value = serde_json::from_str(&response)?;
            assert!(response_json.get("Ok").is_some(), "Expected Ok response");
            
            let response_data = response_json.get("Ok").unwrap();
            let status = response_data.get("status").and_then(|v| v.as_u64());
            assert_eq!(status, Some(204), "Expected status 204");
            
            let body = response_data.get("body").and_then(|v| v.as_str()).unwrap_or("");
            assert_eq!(body, "", "Expected empty body for 204 response");
            
            println!("✅ fetch-advanced 204 No Content test passed!");
        }
        Err(e) => {
            panic!("Expected successful fetch-advanced call, got error: {e}");
        }
    }

    Ok(())
}

#[tokio::test]
async fn test_fetch_advanced_different_methods() -> Result<()> {
    let (manager, _tempdir) = setup_lifecycle_manager().await?;
    let component_path = build_fetch_component().await?;

    let component_id = manager
        .load_component(&format!("file://{}", component_path.to_str().unwrap()))
        .await?
        .component_id;

    // Grant network permission
    manager
        .grant_permission(
            &component_id,
            "network",
            &json!({"host": "httpbin.org"}),
        )
        .await?;

    // Test HEAD method
    let head_url = "https://httpbin.org/get";
    let head_options = json!({
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
            &json!({"url": head_url, "options": head_options}).to_string(),
        )
        .await;

    match result {
        Ok(response) => {
            println!("fetch-advanced HEAD response: {response}");
            
            let response_json: serde_json::Value = serde_json::from_str(&response)?;
            assert!(response_json.get("Ok").is_some(), "Expected Ok response for HEAD");
            
            let response_data = response_json.get("Ok").unwrap();
            let status = response_data.get("status").and_then(|v| v.as_u64());
            assert_eq!(status, Some(200), "Expected status 200 for HEAD");
            
            // HEAD request should have empty body
            let body = response_data.get("body").and_then(|v| v.as_str()).unwrap_or("");
            assert_eq!(body, "", "Expected empty body for HEAD request");
            
            println!("✅ fetch-advanced HEAD method test passed!");
        }
        Err(e) => {
            panic!("Expected successful fetch-advanced HEAD call, got error: {e}");
        }
    }

    Ok(())
}

#[tokio::test]
async fn test_fetch_advanced_charset_handling() -> Result<()> {
    let (manager, _tempdir) = setup_lifecycle_manager().await?;
    let component_path = build_fetch_component().await?;

    let component_id = manager
        .load_component(&format!("file://{}", component_path.to_str().unwrap()))
        .await?
        .component_id;

    // Grant network permission
    manager
        .grant_permission(
            &component_id,
            "network",
            &json!({"host": "httpbin.org"}),
        )
        .await?;

    let target_url = "https://httpbin.org/encoding/utf8";

    println!("Testing fetch-advanced charset handling on {target_url}...");

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
            println!("fetch-advanced charset response: {response}");
            
            let response_json: serde_json::Value = serde_json::from_str(&response)?;
            assert!(response_json.get("Ok").is_some(), "Expected Ok response");
            
            let response_data = response_json.get("Ok").unwrap();
            let status = response_data.get("status").and_then(|v| v.as_u64());
            assert_eq!(status, Some(200), "Expected status 200");
            
            let is_binary = response_data.get("is-binary").and_then(|v| v.as_bool());
            assert_eq!(is_binary, Some(false), "Expected text response, not binary");
            
            println!("✅ fetch-advanced charset handling test passed!");
        }
        Err(e) => {
            panic!("Expected successful fetch-advanced call, got error: {e}");
        }
    }

    Ok(())
}

#[tokio::test]
async fn test_fetch_advanced_post_with_body() -> Result<()> {
    let (manager, _tempdir) = setup_lifecycle_manager().await?;
    let component_path = build_fetch_component().await?;

    let component_id = manager
        .load_component(&format!("file://{}", component_path.to_str().unwrap()))
        .await?
        .component_id;

    // Grant network permission
    manager
        .grant_permission(
            &component_id,
            "network",
            &json!({"host": "httpbin.org"}),
        )
        .await?;

    let target_url = "https://httpbin.org/post";

    println!("Testing fetch-advanced POST with body on {target_url}...");

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

    match result {
        Ok(response) => {
            println!("fetch-advanced POST response: {response}");
            
            let response_json: serde_json::Value = serde_json::from_str(&response)?;
            assert!(response_json.get("Ok").is_some(), "Expected Ok response");
            
            let response_data = response_json.get("Ok").unwrap();
            let status = response_data.get("status").and_then(|v| v.as_u64());
            assert_eq!(status, Some(200), "Expected status 200");
            
            let body = response_data.get("body").and_then(|v| v.as_str()).unwrap_or("");
            // httpbin.org/post echoes back the data we sent
            assert!(body.contains("test") && body.contains("data"), 
                   "Expected POST body to be echoed back");
            
            println!("✅ fetch-advanced POST with body test passed!");
        }
        Err(e) => {
            panic!("Expected successful fetch-advanced POST call, got error: {e}");
        }
    }

    Ok(())
}
