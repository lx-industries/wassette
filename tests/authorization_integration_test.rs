// Copyright (c) Microsoft Corporation.
// Licensed under the MIT license.

//! Tests for MCP authorization spec compliance in streamable-http transport

use std::net::{Ipv4Addr, SocketAddrV4};
use std::time::Duration;

use anyhow::{Context, Result};
use reqwest::{header, Client, StatusCode};
use serde_json::json;
use test_log::test;
use tokio::net::TcpListener;
use tokio::time::sleep;
use wassette::LifecycleManager;

/// Find an available port for testing
async fn find_open_port() -> Result<u16> {
    TcpListener::bind(SocketAddrV4::new(Ipv4Addr::LOCALHOST, 0))
        .await
        .context("failed to bind random port")?
        .local_addr()
        .map(|addr| addr.port())
        .context("failed to get local address from opened TCP socket")
}

/// Start a wassette server with streamable-http transport for testing
async fn start_test_server(port: u16) -> Result<tokio::task::JoinHandle<()>> {
    let tempdir = tempfile::tempdir()?;

    let _manager = LifecycleManager::builder(tempdir.path())
        .with_environment_vars(std::collections::HashMap::new())
        .with_oci_client(oci_client::Client::default())
        .with_http_client(reqwest::Client::default())
        .build()
        .await
        .context("Failed to create LifecycleManager")?;

    // Keep tempdir alive
    let _tempdir_guard = tempdir;

    let handle = tokio::spawn(async move {
        let bind_address = format!("127.0.0.1:{}", port);

        // This is similar to the production code in main.rs
        use rmcp::transport::streamable_http_server::session::local::LocalSessionManager;
        use rmcp::transport::streamable_http_server::StreamableHttpService;

        // Create a simple MCP server handler (empty for testing)
        #[derive(Clone)]
        struct TestServer;

        impl rmcp::ServerHandler for TestServer {
            fn get_info(&self) -> rmcp::model::ServerInfo {
                rmcp::model::ServerInfo {
                    capabilities: Default::default(),
                    instructions: Some("Test server".into()),
                    ..Default::default()
                }
            }
        }

        let server = TestServer;

        let service = StreamableHttpService::new(
            move || Ok(server.clone()),
            LocalSessionManager::default().into(),
            Default::default(),
        );

        let base_url = format!("http://{}", bind_address);
        let auth_config =
            wassette_mcp_server::authorization::AuthorizationConfig::new(base_url.clone(), false);

        let mcp_router = axum::Router::new().nest_service("/mcp", service);
        let auth_router = wassette_mcp_server::authorization::create_auth_router(auth_config);

        let router = axum::Router::new()
            .merge(mcp_router)
            .merge(auth_router)
            .layer(axum::middleware::from_fn(
                wassette_mcp_server::authorization::add_www_authenticate_header,
            ));

        let tcp_listener = tokio::net::TcpListener::bind(&bind_address).await.unwrap();

        axum::serve(tcp_listener, router)
            .with_graceful_shutdown(async {
                // Run until task is cancelled
                std::future::pending::<()>().await
            })
            .await
            .unwrap();

        drop(_tempdir_guard);
    });

    // Give the server time to start
    sleep(Duration::from_millis(500)).await;

    Ok(handle)
}

#[test(tokio::test)]
async fn test_oauth_metadata_endpoint() -> Result<()> {
    let port = find_open_port().await?;
    let server_handle = start_test_server(port).await?;

    let client = Client::new();
    let url = format!(
        "http://127.0.0.1:{}/.well-known/oauth-authorization-server",
        port
    );

    let response = client.get(&url).send().await?;

    assert_eq!(response.status(), StatusCode::OK);

    let metadata: serde_json::Value = response.json().await?;

    // Verify required fields
    assert!(metadata.get("authorization_endpoint").is_some());
    assert!(metadata.get("token_endpoint").is_some());
    assert!(metadata.get("registration_endpoint").is_some());

    // Verify endpoint URLs
    let base_url = format!("http://127.0.0.1:{}", port);
    let expected_auth = format!("{}/authorize", base_url);
    let expected_token = format!("{}/token", base_url);
    let expected_register = format!("{}/register", base_url);

    assert_eq!(
        metadata["authorization_endpoint"].as_str(),
        Some(expected_auth.as_str())
    );
    assert_eq!(
        metadata["token_endpoint"].as_str(),
        Some(expected_token.as_str())
    );
    assert_eq!(
        metadata["registration_endpoint"].as_str(),
        Some(expected_register.as_str())
    );

    server_handle.abort();
    Ok(())
}

#[test(tokio::test)]
async fn test_unauthorized_request_has_www_authenticate_header() -> Result<()> {
    let port = find_open_port().await?;
    let server_handle = start_test_server(port).await?;

    let client = Client::new();
    let url = format!("http://127.0.0.1:{}/mcp", port);

    // Make a request with an invalid session ID
    let response = client
        .post(&url)
        .header(header::CONTENT_TYPE, "application/json")
        .header(header::ACCEPT, "application/json, text/event-stream")
        .header("Mcp-Session-Id", "invalid-session-id")
        .json(&json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "tools/list",
            "params": {}
        }))
        .send()
        .await?;

    // Should be 401 Unauthorized
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);

    // Should have WWW-Authenticate header
    let www_auth = response.headers().get(header::WWW_AUTHENTICATE);
    assert!(
        www_auth.is_some(),
        "WWW-Authenticate header should be present"
    );

    // Header should contain "Bearer"
    let www_auth_value = www_auth.unwrap().to_str()?;
    assert!(
        www_auth_value.contains("Bearer"),
        "WWW-Authenticate header should contain 'Bearer'"
    );

    server_handle.abort();
    Ok(())
}

#[test(tokio::test)]
async fn test_get_request_unauthorized_has_www_authenticate() -> Result<()> {
    let port = find_open_port().await?;
    let server_handle = start_test_server(port).await?;

    let client = Client::new();
    let url = format!("http://127.0.0.1:{}/mcp", port);

    // Make a GET request with an invalid session ID
    let response = client
        .get(&url)
        .header(header::ACCEPT, "text/event-stream")
        .header("Mcp-Session-Id", "invalid-session-id")
        .send()
        .await?;

    // Should be 401 Unauthorized
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);

    // Should have WWW-Authenticate header
    let www_auth = response.headers().get(header::WWW_AUTHENTICATE);
    assert!(
        www_auth.is_some(),
        "WWW-Authenticate header should be present on GET requests"
    );

    let www_auth_value = www_auth.unwrap().to_str()?;
    assert!(
        www_auth_value.contains("Bearer"),
        "WWW-Authenticate header should contain 'Bearer'"
    );

    server_handle.abort();
    Ok(())
}
