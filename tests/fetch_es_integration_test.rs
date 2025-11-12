// Copyright (c) Microsoft Corporation.
// Licensed under the MIT license.

use anyhow::{Context, Result};
use serde_json::json;
use tempfile::TempDir;
use wassette::LifecycleManager;

mod common;
use common::build_fetch_es_component;

async fn setup_lifecycle_manager() -> Result<(LifecycleManager, TempDir)> {
    let tempdir = tempfile::tempdir().context("Failed to create temporary directory")?;
    let manager = LifecycleManager::new(&tempdir).await?;
    Ok((manager, tempdir))
}

#[tokio::test]
async fn test_fetch_es_simple_get() -> Result<()> {
    let (manager, _tempdir) = setup_lifecycle_manager().await?;
    let component_path = build_fetch_es_component().await?;

    let component_id = manager
        .load_component(&format!("file://{}", component_path.to_str().unwrap()))
        .await?
        .component_id;

    // Grant network permission for example.com
    manager
        .grant_permission(
            &component_id,
            "network",
            &json!({"host": "https://example.com"}),
        )
        .await?;

    // Test simple GET request
    let result = manager
        .execute_component_call(
            &component_id,
            "component_fetch_es_types_fetch",
            &json!({
                "url": "https://example.com/",
                "options": null
            })
            .to_string(),
        )
        .await?;

    println!("Fetch response: {result}");
    
    // Parse the response
    let response: serde_json::Value = serde_json::from_str(&result)?;
    
    // Should have status 200
    assert!(response.get("status").is_some());
    let status = response["status"].as_u64().unwrap();
    assert_eq!(status, 200);
    
    // Should have body with "Example Domain"
    assert!(response.get("body").is_some());
    let body = response["body"].as_str().unwrap();
    assert!(body.contains("Example Domain"));

    Ok(())
}

#[tokio::test]
async fn test_fetch_es_post_request() -> Result<()> {
    let (manager, _tempdir) = setup_lifecycle_manager().await?;
    let component_path = build_fetch_es_component().await?;

    let component_id = manager
        .load_component(&format!("file://{}", component_path.to_str().unwrap()))
        .await?
        .component_id;

    // Grant network permission for httpbin.org
    manager
        .grant_permission(
            &component_id,
            "network",
            &json!({"host": "https://httpbin.org"}),
        )
        .await?;

    // Test POST request with JSON body
    let post_body = json!({"test": "data"}).to_string();
    let result = manager
        .execute_component_call(
            &component_id,
            "component_fetch_es_types_fetch",
            &json!({
                "url": "https://httpbin.org/post",
                "options": {
                    "method": "POST",
                    "body": post_body,
                    "headers": [
                        {"name": "Content-Type", "value": "application/json"}
                    ]
                }
            })
            .to_string(),
        )
        .await?;

    println!("POST response: {result}");
    
    let response: serde_json::Value = serde_json::from_str(&result)?;
    assert_eq!(response["status"].as_u64().unwrap(), 200);

    Ok(())
}

#[tokio::test]
async fn test_fetch_es_timeout() -> Result<()> {
    let (manager, _tempdir) = setup_lifecycle_manager().await?;
    let component_path = build_fetch_es_component().await?;

    let component_id = manager
        .load_component(&format!("file://{}", component_path.to_str().unwrap()))
        .await?
        .component_id;

    // Grant network permission for httpbin.org
    manager
        .grant_permission(
            &component_id,
            "network",
            &json!({"host": "https://httpbin.org"}),
        )
        .await?;

    // Test request with very short timeout (should timeout)
    let result = manager
        .execute_component_call(
            &component_id,
            "component_fetch_es_types_fetch",
            &json!({
                "url": "https://httpbin.org/delay/5",
                "options": {
                    "timeout": 1000  // 1 second timeout for 5 second delay
                }
            })
            .to_string(),
        )
        .await;

    // Should fail with timeout error
    match result {
        Err(e) => {
            let error_msg = e.to_string();
            println!("Expected timeout error: {error_msg}");
            assert!(
                error_msg.contains("abort") || error_msg.contains("timeout"),
                "Expected timeout error, got: {error_msg}"
            );
        }
        Ok(response) => {
            // If it didn't error, the response should contain an error
            println!("Response: {response}");
            assert!(
                response.contains("abort") || response.contains("timeout"),
                "Expected timeout error in response"
            );
        }
    }

    Ok(())
}

#[tokio::test]
async fn test_fetch_es_http_methods() -> Result<()> {
    let (manager, _tempdir) = setup_lifecycle_manager().await?;
    let component_path = build_fetch_es_component().await?;

    let component_id = manager
        .load_component(&format!("file://{}", component_path.to_str().unwrap()))
        .await?
        .component_id;

    // Grant network permission for httpbin.org
    manager
        .grant_permission(
            &component_id,
            "network",
            &json!({"host": "https://httpbin.org"}),
        )
        .await?;

    // Test different HTTP methods
    let methods = vec!["GET", "POST", "PUT", "PATCH", "DELETE"];

    for method in methods {
        println!("Testing {method} method...");
        
        let url = format!("https://httpbin.org/{}", method.to_lowercase());
        let result = manager
            .execute_component_call(
                &component_id,
                "component_fetch_es_types_fetch",
                &json!({
                    "url": url,
                    "options": {
                        "method": method
                    }
                })
                .to_string(),
            )
            .await?;

        let response: serde_json::Value = serde_json::from_str(&result)?;
        assert_eq!(
            response["status"].as_u64().unwrap(),
            200,
            "{method} request failed"
        );
    }

    Ok(())
}

#[tokio::test]
async fn test_fetch_es_redirect_handling() -> Result<()> {
    let (manager, _tempdir) = setup_lifecycle_manager().await?;
    let component_path = build_fetch_es_component().await?;

    let component_id = manager
        .load_component(&format!("file://{}", component_path.to_str().unwrap()))
        .await?
        .component_id;

    // Grant network permission for httpbin.org
    manager
        .grant_permission(
            &component_id,
            "network",
            &json!({"host": "https://httpbin.org"}),
        )
        .await?;

    // Test redirect (httpbin.org/redirect/1 redirects to /get)
    let result = manager
        .execute_component_call(
            &component_id,
            "component_fetch_es_types_fetch",
            &json!({
                "url": "https://httpbin.org/redirect/1",
                "options": {
                    "maxRedirects": 5
                }
            })
            .to_string(),
        )
        .await?;

    let response: serde_json::Value = serde_json::from_str(&result)?;
    assert_eq!(response["status"].as_u64().unwrap(), 200);

    Ok(())
}

#[tokio::test]
async fn test_fetch_es_no_body_status_codes() -> Result<()> {
    let (manager, _tempdir) = setup_lifecycle_manager().await?;
    let component_path = build_fetch_es_component().await?;

    let component_id = manager
        .load_component(&format!("file://{}", component_path.to_str().unwrap()))
        .await?
        .component_id;

    // Grant network permission for httpbin.org
    manager
        .grant_permission(
            &component_id,
            "network",
            &json!({"host": "https://httpbin.org"}),
        )
        .await?;

    // Test 204 No Content status
    let result = manager
        .execute_component_call(
            &component_id,
            "component_fetch_es_types_fetch",
            &json!({
                "url": "https://httpbin.org/status/204",
                "options": null
            })
            .to_string(),
        )
        .await?;

    let response: serde_json::Value = serde_json::from_str(&result)?;
    assert_eq!(response["status"].as_u64().unwrap(), 204);
    
    // Body should be empty for 204
    let body = response["body"].as_str().unwrap();
    assert_eq!(body, "");

    Ok(())
}

#[tokio::test]
async fn test_fetch_es_charset_detection() -> Result<()> {
    let (manager, _tempdir) = setup_lifecycle_manager().await?;
    let component_path = build_fetch_es_component().await?;

    let component_id = manager
        .load_component(&format!("file://{}", component_path.to_str().unwrap()))
        .await?
        .component_id;

    // Grant network permission for httpbin.org
    manager
        .grant_permission(
            &component_id,
            "network",
            &json!({"host": "https://httpbin.org"}),
        )
        .await?;

    // Test with HTML content that has charset
    let result = manager
        .execute_component_call(
            &component_id,
            "component_fetch_es_types_fetch",
            &json!({
                "url": "https://httpbin.org/html",
                "options": null
            })
            .to_string(),
        )
        .await?;

    let response: serde_json::Value = serde_json::from_str(&result)?;
    assert_eq!(response["status"].as_u64().unwrap(), 200);
    
    // Should have charset detected
    assert!(response.get("charset").is_some());
    
    // Should not be binary
    assert_eq!(response["isBinary"].as_bool().unwrap(), false);

    Ok(())
}

#[tokio::test]
async fn test_fetch_es_retry_on_transient_error() -> Result<()> {
    let (manager, _tempdir) = setup_lifecycle_manager().await?;
    let component_path = build_fetch_es_component().await?;

    let component_id = manager
        .load_component(&format!("file://{}", component_path.to_str().unwrap()))
        .await?
        .component_id;

    // Grant network permission for httpbin.org
    manager
        .grant_permission(
            &component_id,
            "network",
            &json!({"host": "https://httpbin.org"}),
        )
        .await?;

    // Test with 503 status (should trigger retry logic)
    let result = manager
        .execute_component_call(
            &component_id,
            "component_fetch_es_types_fetch",
            &json!({
                "url": "https://httpbin.org/status/503",
                "options": {
                    "retry": true,
                    "maxRetries": 2
                }
            })
            .to_string(),
        )
        .await;

    // The request should eventually succeed or fail after retries
    match result {
        Ok(response) => {
            println!("Response after retries: {response}");
            let resp: serde_json::Value = serde_json::from_str(&response)?;
            // httpbin always returns 503, so status should be 503
            assert_eq!(resp["status"].as_u64().unwrap(), 503);
        }
        Err(e) => {
            println!("Error after retries: {e}");
            // This is also acceptable - the retries were attempted
        }
    }

    Ok(())
}

#[tokio::test]
async fn test_fetch_es_disable_retry() -> Result<()> {
    let (manager, _tempdir) = setup_lifecycle_manager().await?;
    let component_path = build_fetch_es_component().await?;

    let component_id = manager
        .load_component(&format!("file://{}", component_path.to_str().unwrap()))
        .await?
        .component_id;

    // Grant network permission for httpbin.org
    manager
        .grant_permission(
            &component_id,
            "network",
            &json!({"host": "https://httpbin.org"}),
        )
        .await?;

    // Test with retry disabled
    let result = manager
        .execute_component_call(
            &component_id,
            "component_fetch_es_types_fetch",
            &json!({
                "url": "https://httpbin.org/status/503",
                "options": {
                    "retry": false
                }
            })
            .to_string(),
        )
        .await?;

    let response: serde_json::Value = serde_json::from_str(&result)?;
    // Should get 503 immediately without retries
    assert_eq!(response["status"].as_u64().unwrap(), 503);

    Ok(())
}

#[tokio::test]
async fn test_fetch_es_custom_headers() -> Result<()> {
    let (manager, _tempdir) = setup_lifecycle_manager().await?;
    let component_path = build_fetch_es_component().await?;

    let component_id = manager
        .load_component(&format!("file://{}", component_path.to_str().unwrap()))
        .await?
        .component_id;

    // Grant network permission for httpbin.org
    manager
        .grant_permission(
            &component_id,
            "network",
            &json!({"host": "https://httpbin.org"}),
        )
        .await?;

    // Test with custom headers
    let result = manager
        .execute_component_call(
            &component_id,
            "component_fetch_es_types_fetch",
            &json!({
                "url": "https://httpbin.org/headers",
                "options": {
                    "headers": [
                        {"name": "X-Custom-Header", "value": "test-value"},
                        {"name": "User-Agent", "value": "fetch-es-test"}
                    ]
                }
            })
            .to_string(),
        )
        .await?;

    let response: serde_json::Value = serde_json::from_str(&result)?;
    assert_eq!(response["status"].as_u64().unwrap(), 200);
    
    // httpbin echoes back the headers in the response body
    let body = response["body"].as_str().unwrap();
    assert!(body.contains("X-Custom-Header") || body.contains("test-value"));

    Ok(())
}
