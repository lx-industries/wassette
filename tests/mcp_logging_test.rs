// Copyright (c) Microsoft Corporation.
// Licensed under the MIT license.

//! Integration tests for MCP logging functionality

use std::process::Stdio;
use std::time::Duration;

use anyhow::{Context, Result};
use serde_json::Value;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::process::Command;

/// Test that the server declares logging capability
#[tokio::test]
async fn test_logging_capability_declared() -> Result<()> {
    // Build the binary first
    let binary_path = std::env::current_dir()
        .context("Failed to get current directory")?
        .join("target/debug/wassette");

    // Start wassette mcp server with stdio transport (default)
    let mut child = Command::new(&binary_path)
        .args(["serve"])
        .env("RUST_LOG", "off") // Disable logs to avoid stdout pollution
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .context("Failed to start wassette server")?;

    let stdin = child.stdin.take().context("Failed to get stdin handle")?;
    let stdout = child.stdout.take().context("Failed to get stdout handle")?;

    let mut stdin = stdin;
    let mut stdout = BufReader::new(stdout);

    // Give the server time to start
    tokio::time::sleep(Duration::from_millis(1000)).await;

    // Send MCP initialize request
    let initialize_request = r#"{"jsonrpc": "2.0", "method": "initialize", "params": {"protocolVersion": "2024-11-05", "capabilities": {}, "clientInfo": {"name": "test-client", "version": "1.0.0"}}, "id": 1}
"#;

    stdin.write_all(initialize_request.as_bytes()).await?;
    stdin.flush().await?;

    // Read and verify initialize response
    let mut response_line = String::new();
    tokio::time::timeout(
        Duration::from_secs(10),
        stdout.read_line(&mut response_line),
    )
    .await
    .context("Timeout waiting for initialize response")??;

    let response: Value =
        serde_json::from_str(&response_line).context("Failed to parse initialize response")?;

    assert_eq!(response["jsonrpc"], "2.0");
    assert_eq!(response["id"], 1);
    assert!(response["result"].is_object());

    // Check that logging capability is declared
    let capabilities = &response["result"]["capabilities"];
    assert!(
        capabilities["logging"].is_object(),
        "Logging capability should be declared as an object"
    );

    // Cleanup
    let _ = child.kill().await;

    Ok(())
}

/// Test that set_level request is handled without error
#[tokio::test]
async fn test_set_level_request() -> Result<()> {
    // Build the binary first
    let binary_path = std::env::current_dir()
        .context("Failed to get current directory")?
        .join("target/debug/wassette");

    // Start wassette mcp server with stdio transport (default)
    let mut child = Command::new(&binary_path)
        .args(["serve"])
        .env("RUST_LOG", "info") // Enable info logs
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .context("Failed to start wassette server")?;

    let stdin = child.stdin.take().context("Failed to get stdin handle")?;
    let stdout = child.stdout.take().context("Failed to get stdout handle")?;

    let mut stdin = stdin;
    let mut stdout = BufReader::new(stdout);

    // Give the server time to start
    tokio::time::sleep(Duration::from_millis(1000)).await;

    // Send MCP initialize request
    let initialize_request = r#"{"jsonrpc": "2.0", "method": "initialize", "params": {"protocolVersion": "2024-11-05", "capabilities": {}, "clientInfo": {"name": "test-client", "version": "1.0.0"}}, "id": 1}
"#;

    stdin.write_all(initialize_request.as_bytes()).await?;
    stdin.flush().await?;

    // Read initialize response
    let mut response_line = String::new();
    tokio::time::timeout(
        Duration::from_secs(10),
        stdout.read_line(&mut response_line),
    )
    .await
    .context("Timeout waiting for initialize response")??;

    // Send initialized notification
    let initialized_notification = r#"{"jsonrpc": "2.0", "method": "notifications/initialized", "params": {}}
"#;
    stdin.write_all(initialized_notification.as_bytes()).await?;
    stdin.flush().await?;

    // Send logging/setLevel request
    let set_level_request = r#"{"jsonrpc": "2.0", "method": "logging/setLevel", "params": {"level": "info"}, "id": 2}
"#;

    stdin.write_all(set_level_request.as_bytes()).await?;
    stdin.flush().await?;

    // Read set_level response
    let mut set_level_response = String::new();
    tokio::time::timeout(
        Duration::from_secs(10),
        stdout.read_line(&mut set_level_response),
    )
    .await
    .context("Timeout waiting for set_level response")??;

    let response: Value =
        serde_json::from_str(&set_level_response).context("Failed to parse set_level response")?;

    assert_eq!(response["jsonrpc"], "2.0");
    assert_eq!(response["id"], 2);
    // Result should be empty object or null
    assert!(
        response["result"].is_null() || response["result"].is_object(),
        "set_level should return empty result"
    );

    // Cleanup
    let _ = child.kill().await;

    Ok(())
}

/// Test setting different log levels
#[tokio::test]
async fn test_multiple_log_levels() -> Result<()> {
    // Build the binary first
    let binary_path = std::env::current_dir()
        .context("Failed to get current directory")?
        .join("target/debug/wassette");

    // Start wassette mcp server
    let mut child = Command::new(&binary_path)
        .args(["serve"])
        .env("RUST_LOG", "debug")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .context("Failed to start wassette server")?;

    let stdin = child.stdin.take().context("Failed to get stdin handle")?;
    let stdout = child.stdout.take().context("Failed to get stdout handle")?;

    let mut stdin = stdin;
    let mut stdout = BufReader::new(stdout);

    // Give the server time to start
    tokio::time::sleep(Duration::from_millis(1000)).await;

    // Initialize
    let initialize_request = r#"{"jsonrpc": "2.0", "method": "initialize", "params": {"protocolVersion": "2024-11-05", "capabilities": {}, "clientInfo": {"name": "test-client", "version": "1.0.0"}}, "id": 1}
"#;
    stdin.write_all(initialize_request.as_bytes()).await?;
    stdin.flush().await?;

    let mut response_line = String::new();
    stdout.read_line(&mut response_line).await?;

    // Send initialized notification
    let initialized_notification = r#"{"jsonrpc": "2.0", "method": "notifications/initialized", "params": {}}
"#;
    stdin.write_all(initialized_notification.as_bytes()).await?;
    stdin.flush().await?;

    // Test different log levels
    let levels = ["debug", "info", "warning", "error"];
    for (idx, level) in levels.iter().enumerate() {
        let set_level_request = format!(
            r#"{{"jsonrpc": "2.0", "method": "logging/setLevel", "params": {{"level": "{}"}}, "id": {}}}
"#,
            level,
            idx + 2
        );

        stdin.write_all(set_level_request.as_bytes()).await?;
        stdin.flush().await?;

        let mut response = String::new();
        tokio::time::timeout(Duration::from_secs(5), stdout.read_line(&mut response))
            .await
            .context(format!("Timeout for level {}", level))??;

        let parsed: Value =
            serde_json::from_str(&response).context(format!("Parse error for level {}", level))?;
        assert_eq!(parsed["id"], idx + 2);
        assert!(
            !parsed.get("error").is_some(),
            "Should not have error for level {}",
            level
        );
    }

    // Cleanup
    let _ = child.kill().await;

    Ok(())
}
