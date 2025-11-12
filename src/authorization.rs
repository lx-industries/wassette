// Copyright (c) Microsoft Corporation.
// Licensed under the MIT license.

//! Authorization support for MCP server using OAuth 2.1
//!
//! This module provides optional authorization support for the streamable-HTTP transport,
//! conforming to the MCP authorization specification.

use std::collections::HashMap;

use anyhow::Result;
use axum::extract::Request;
use axum::http::{header, HeaderValue, StatusCode};
use axum::middleware::Next;
use axum::response::Response;
use axum::{Json, Router};
use serde::{Deserialize, Serialize};

/// OAuth 2.0 Authorization Server Metadata (RFC 8414)
///
/// This structure is compatible with rmcp's AuthorizationMetadata
/// but defined here to avoid dependency issues.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthorizationMetadata {
    /// The authorization endpoint URL
    pub authorization_endpoint: String,
    /// The token endpoint URL
    pub token_endpoint: String,
    /// The registration endpoint URL for dynamic client registration
    pub registration_endpoint: String,
    /// The issuer identifier (optional)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub issuer: Option<String>,
    /// The JWKS URI (optional)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub jwks_uri: Option<String>,
    /// Supported scopes (optional)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub scopes_supported: Option<Vec<String>>,
    /// Additional fields
    #[serde(flatten)]
    pub additional_fields: HashMap<String, serde_json::Value>,
}

impl AuthorizationMetadata {
    /// Create default authorization metadata for a given base URL
    pub fn new(base_url: &str) -> Self {
        Self {
            authorization_endpoint: format!("{}/authorize", base_url),
            token_endpoint: format!("{}/token", base_url),
            registration_endpoint: format!("{}/register", base_url),
            issuer: Some(base_url.to_string()),
            jwks_uri: None,
            scopes_supported: Some(vec!["mcp".to_string()]),
            additional_fields: HashMap::new(),
        }
    }
}

/// Authorization configuration for the MCP server
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct AuthorizationConfig {
    /// The base URL for the server (e.g., "http://localhost:9001")
    pub base_url: String,
    /// Whether authorization is required for all requests
    pub required: bool,
    /// The authorization metadata
    pub metadata: AuthorizationMetadata,
}

impl AuthorizationConfig {
    /// Create a new authorization configuration
    pub fn new(base_url: String, required: bool) -> Self {
        let metadata = AuthorizationMetadata::new(&base_url);
        Self {
            base_url,
            required,
            metadata,
        }
    }

    /// Get the metadata discovery URL
    #[allow(dead_code)]
    pub fn metadata_url(&self) -> String {
        format!("{}/.well-known/oauth-authorization-server", self.base_url)
    }
}

/// Create a router that serves the OAuth authorization metadata endpoint
pub fn create_auth_router(config: AuthorizationConfig) -> Router {
    Router::new().route(
        "/.well-known/oauth-authorization-server",
        axum::routing::get(move || async move { Json(config.metadata.clone()) }),
    )
}

/// Middleware that adds WWW-Authenticate header to 401 responses
pub async fn add_www_authenticate_header(
    request: Request,
    next: Next,
) -> Result<Response, StatusCode> {
    let response = next.run(request).await;

    // If the response is 401 Unauthorized, add WWW-Authenticate header
    if response.status() == StatusCode::UNAUTHORIZED {
        let (mut parts, body) = response.into_parts();

        // Add WWW-Authenticate header with Bearer scheme
        parts
            .headers
            .insert(header::WWW_AUTHENTICATE, HeaderValue::from_static("Bearer"));

        Ok(Response::from_parts(parts, body))
    } else {
        Ok(response)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_authorization_metadata_creation() {
        let metadata = AuthorizationMetadata::new("http://localhost:9001");
        assert_eq!(
            metadata.authorization_endpoint,
            "http://localhost:9001/authorize"
        );
        assert_eq!(metadata.token_endpoint, "http://localhost:9001/token");
        assert_eq!(
            metadata.registration_endpoint,
            "http://localhost:9001/register"
        );
        assert_eq!(metadata.issuer, Some("http://localhost:9001".to_string()));
    }

    #[test]
    fn test_authorization_config_metadata_url() {
        let config = AuthorizationConfig::new("http://localhost:9001".to_string(), true);
        assert_eq!(
            config.metadata_url(),
            "http://localhost:9001/.well-known/oauth-authorization-server"
        );
    }

    #[test]
    fn test_authorization_metadata_serialization() {
        let metadata = AuthorizationMetadata::new("http://localhost:9001");
        let json = serde_json::to_string(&metadata).unwrap();
        assert!(json.contains("authorization_endpoint"));
        assert!(json.contains("token_endpoint"));
        assert!(json.contains("registration_endpoint"));
    }
}
