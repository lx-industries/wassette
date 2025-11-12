// Copyright (c) Microsoft Corporation.
// Licensed under the MIT license.

//! Integration tests for IPC server

use std::sync::Arc;
use std::time::Duration;

use anyhow::Result;
use tempfile::TempDir;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::time::timeout;
use wassette::{IpcServer, IpcServerConfig, SecretsManager};

#[cfg(unix)]
#[tokio::test]
async fn test_ipc_server_basic_connection() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let socket_path = temp_dir.path().join("test.sock");
    let secrets_dir = temp_dir.path().join("secrets");

    let secrets_manager = Arc::new(SecretsManager::new(secrets_dir));
    secrets_manager.ensure_secrets_dir().await?;

    let config = IpcServerConfig::new(socket_path.clone(), secrets_manager.clone());
    let mut server = IpcServer::new(config);

    // Start server in background
    let _server_handle = tokio::spawn(async move { server.start().await });

    // Give server time to start
    tokio::time::sleep(Duration::from_millis(100)).await;

    // Connect to the server
    let stream = tokio::net::UnixStream::connect(&socket_path).await?;
    let mut reader = BufReader::new(stream);

    // Send ping request
    reader
        .get_mut()
        .write_all(b"{\"command\":\"ping\"}\n")
        .await?;

    // Read response
    let mut response_line = String::new();
    timeout(Duration::from_secs(2), reader.read_line(&mut response_line)).await??;

    // Parse response
    let response: serde_json::Value = serde_json::from_str(&response_line)?;
    assert_eq!(response["status"], "success");
    assert_eq!(response["message"], "pong");

    // Clean up
    drop(reader);
    tokio::time::sleep(Duration::from_millis(50)).await;

    Ok(())
}

#[cfg(unix)]
#[tokio::test]
async fn test_ipc_server_set_and_list_secrets() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let socket_path = temp_dir.path().join("test.sock");
    let secrets_dir = temp_dir.path().join("secrets");

    let secrets_manager = Arc::new(SecretsManager::new(secrets_dir));
    secrets_manager.ensure_secrets_dir().await?;

    let config = IpcServerConfig::new(socket_path.clone(), secrets_manager.clone());
    let mut server = IpcServer::new(config);

    // Start server in background
    let _server_handle = tokio::spawn(async move { server.start().await });

    // Give server time to start
    tokio::time::sleep(Duration::from_millis(100)).await;

    // Connect to the server
    let stream = tokio::net::UnixStream::connect(&socket_path).await?;
    let mut reader = BufReader::new(stream);

    // Set a secret
    let set_request = r#"{"command":"set_secret","component_id":"test-component","key":"API_KEY","value":"secret123"}"#;
    reader.get_mut().write_all(set_request.as_bytes()).await?;
    reader.get_mut().write_all(b"\n").await?;

    let mut response_line = String::new();
    timeout(Duration::from_secs(2), reader.read_line(&mut response_line)).await??;

    let response: serde_json::Value = serde_json::from_str(&response_line)?;
    assert_eq!(response["status"], "success");

    // List secrets
    response_line.clear();
    let list_request = r#"{"command":"list_secrets","component_id":"test-component"}"#;
    reader.get_mut().write_all(list_request.as_bytes()).await?;
    reader.get_mut().write_all(b"\n").await?;

    timeout(Duration::from_secs(2), reader.read_line(&mut response_line)).await??;

    let response: serde_json::Value = serde_json::from_str(&response_line)?;
    assert_eq!(response["status"], "success");
    assert!(response["data"]["keys"]
        .as_array()
        .unwrap()
        .contains(&serde_json::json!("API_KEY")));

    Ok(())
}

#[cfg(unix)]
#[tokio::test]
async fn test_ipc_server_delete_secret() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let socket_path = temp_dir.path().join("test.sock");
    let secrets_dir = temp_dir.path().join("secrets");

    let secrets_manager = Arc::new(SecretsManager::new(secrets_dir));
    secrets_manager.ensure_secrets_dir().await?;

    // Pre-populate a secret
    secrets_manager
        .set_component_secrets(
            "test-component",
            &[("API_KEY".to_string(), "secret123".to_string())],
        )
        .await?;

    let config = IpcServerConfig::new(socket_path.clone(), secrets_manager.clone());
    let mut server = IpcServer::new(config);

    // Start server in background
    let _server_handle = tokio::spawn(async move { server.start().await });

    // Give server time to start
    tokio::time::sleep(Duration::from_millis(100)).await;

    // Connect to the server
    let stream = tokio::net::UnixStream::connect(&socket_path).await?;
    let mut reader = BufReader::new(stream);

    // Delete the secret
    let delete_request =
        r#"{"command":"delete_secret","component_id":"test-component","key":"API_KEY"}"#;
    reader
        .get_mut()
        .write_all(delete_request.as_bytes())
        .await?;
    reader.get_mut().write_all(b"\n").await?;

    let mut response_line = String::new();
    timeout(Duration::from_secs(2), reader.read_line(&mut response_line)).await??;

    let response: serde_json::Value = serde_json::from_str(&response_line)?;
    assert_eq!(response["status"], "success");

    // Verify the secret was deleted
    let secrets = secrets_manager
        .list_component_secrets("test-component", true)
        .await?;
    assert!(!secrets.contains_key("API_KEY"));

    Ok(())
}

#[cfg(unix)]
#[tokio::test]
async fn test_ipc_server_invalid_request() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let socket_path = temp_dir.path().join("test.sock");
    let secrets_dir = temp_dir.path().join("secrets");

    let secrets_manager = Arc::new(SecretsManager::new(secrets_dir));
    secrets_manager.ensure_secrets_dir().await?;

    let config = IpcServerConfig::new(socket_path.clone(), secrets_manager.clone());
    let mut server = IpcServer::new(config);

    // Start server in background
    let _server_handle = tokio::spawn(async move { server.start().await });

    // Give server time to start
    tokio::time::sleep(Duration::from_millis(100)).await;

    // Connect to the server
    let stream = tokio::net::UnixStream::connect(&socket_path).await?;
    let mut reader = BufReader::new(stream);

    // Send invalid JSON
    reader.get_mut().write_all(b"not valid json\n").await?;

    let mut response_line = String::new();
    timeout(Duration::from_secs(2), reader.read_line(&mut response_line)).await??;

    let response: serde_json::Value = serde_json::from_str(&response_line)?;
    assert_eq!(response["status"], "error");
    assert!(response["message"]
        .as_str()
        .unwrap()
        .contains("Invalid request"));

    Ok(())
}
