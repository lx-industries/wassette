// Copyright (c) Microsoft Corporation.
// Licensed under the MIT license.

//! OCI registry authentication support
//!
//! This module provides authentication for OCI registries by reading Docker config files
//! and extracting credentials. It supports both username/password and identity token
//! authentication methods.

use anyhow::{Context, Result};
use docker_credential::{CredentialRetrievalError, DockerCredential};
use oci_client::secrets::RegistryAuth;
use oci_client::Reference;
use tracing::{debug, warn};

/// OCI registry credentials provided explicitly via CLI or environment variables
#[derive(Debug, Clone)]
pub struct OciCredentials {
    /// Registry username
    pub username: String,
    /// Registry password or token
    pub password: String,
}

/// Get authentication credentials for an OCI registry reference
///
/// This function attempts to read credentials from the Docker config file
/// (typically `~/.docker/config.json`). It follows the standard Docker credential
/// resolution process:
///
/// 1. Check `$DOCKER_CONFIG/config.json` if the env var is set
/// 2. Check `~/.docker/config.json` as the default location
/// 3. Fall back to `Anonymous` if no config or credentials are found
///
/// # Arguments
///
/// * `reference` - The OCI reference to get credentials for
///
/// # Returns
///
/// Returns a `RegistryAuth` enum that can be one of:
/// - `Anonymous` - No credentials found or config doesn't exist
/// - `Basic(username, password)` - Username/password credentials
/// - `Bearer(token)` - Not currently supported, falls back to Anonymous
///
/// # Errors
///
/// Returns an error if the Docker config file exists but cannot be parsed
/// or if credential retrieval fails for reasons other than missing config.
pub fn get_registry_auth(reference: &Reference) -> Result<RegistryAuth> {
    get_registry_auth_with_credentials(reference, None)
}

/// Get authentication credentials for an OCI registry reference with optional explicit credentials
///
/// This function implements a priority-based credential resolution:
///
/// 1. Use explicit credentials if provided (CLI flags or environment variables)
/// 2. Fall back to Docker config file credentials
/// 3. Fall back to Anonymous if no credentials are found
///
/// # Arguments
///
/// * `reference` - The OCI reference to get credentials for
/// * `explicit_credentials` - Optional explicit credentials from CLI flags
///
/// # Returns
///
/// Returns a `RegistryAuth` enum that can be one of:
/// - `Anonymous` - No credentials found
/// - `Basic(username, password)` - Username/password credentials
///
/// # Errors
///
/// Returns an error if the Docker config file exists but cannot be parsed
/// or if credential retrieval fails for reasons other than missing config.
pub fn get_registry_auth_with_credentials(
    reference: &Reference,
    explicit_credentials: Option<OciCredentials>,
) -> Result<RegistryAuth> {
    // Priority 1: Use explicit credentials if provided
    if let Some(creds) = explicit_credentials {
        debug!(
            "Using explicit credentials for registry: {}",
            reference.resolve_registry()
        );
        return Ok(RegistryAuth::Basic(creds.username, creds.password));
    }

    // Priority 2: Try Docker config file
    // Get the registry server address from the reference
    // Strip trailing slash if present for consistent matching
    let server = reference
        .resolve_registry()
        .strip_suffix('/')
        .unwrap_or_else(|| reference.resolve_registry());

    debug!("Looking up credentials for registry: {}", server);

    // Attempt to retrieve credentials using docker_credential crate
    match docker_credential::get_credential(server) {
        Ok(DockerCredential::UsernamePassword(username, password)) => {
            debug!("Found Docker credentials for registry: {}", server);
            Ok(RegistryAuth::Basic(username, password))
        }
        Ok(DockerCredential::IdentityToken(_)) => {
            // Identity tokens are not supported by oci-client yet
            warn!(
                "Identity token authentication found for {} but is not supported. Using anonymous access.",
                server
            );
            Ok(RegistryAuth::Anonymous)
        }
        Err(CredentialRetrievalError::ConfigNotFound) => {
            debug!("Docker config file not found, using anonymous authentication");
            Ok(RegistryAuth::Anonymous)
        }
        Err(CredentialRetrievalError::ConfigReadError) => {
            debug!("Unable to read Docker config file, using anonymous authentication");
            Ok(RegistryAuth::Anonymous)
        }
        Err(CredentialRetrievalError::NoCredentialConfigured) => {
            debug!(
                "No credentials configured for registry {}, using anonymous authentication",
                server
            );
            Ok(RegistryAuth::Anonymous)
        }
        Err(e) => {
            // For other errors (helper failures, decoding errors, etc.), return the error
            Err(e).context(format!(
                "Failed to retrieve credentials for registry {}",
                server
            ))
        }
    }
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::PathBuf;

    use tempfile::TempDir;

    use super::*;

    fn create_test_docker_config(dir: &TempDir, config_content: &str) -> PathBuf {
        let config_dir = dir.path().join(".docker");
        fs::create_dir_all(&config_dir).unwrap();
        let config_path = config_dir.join("config.json");
        fs::write(&config_path, config_content).unwrap();
        config_path
    }

    #[test]
    fn test_get_registry_auth_with_basic_credentials() {
        use temp_env;

        let temp_dir = TempDir::new().unwrap();

        // Create a test Docker config with basic auth
        let config_content = r#"{
            "auths": {
                "ghcr.io": {
                    "auth": "dGVzdHVzZXI6dGVzdHBhc3M="
                }
            }
        }"#;

        let config_path = create_test_docker_config(&temp_dir, config_content);

        // Set DOCKER_CONFIG to point to our test directory
        let docker_config_dir = config_path.parent().unwrap();

        temp_env::with_var("DOCKER_CONFIG", Some(docker_config_dir), || {
            let reference: Reference = "ghcr.io/test/image:latest".parse().unwrap();
            let auth = get_registry_auth(&reference).unwrap();

            match auth {
                RegistryAuth::Basic(username, password) => {
                    assert_eq!(username, "testuser");
                    assert_eq!(password, "testpass");
                }
                _ => panic!("Expected Basic auth, got: {:?}", auth),
            }
        });
    }

    #[test]
    fn test_get_registry_auth_no_config() {
        use temp_env;

        let temp_dir = TempDir::new().unwrap();

        // Set DOCKER_CONFIG to empty temp dir (no config.json)
        temp_env::with_var("DOCKER_CONFIG", Some(temp_dir.path()), || {
            let reference: Reference = "docker.io/library/nginx:latest".parse().unwrap();
            let auth = get_registry_auth(&reference).unwrap();

            assert!(
                matches!(auth, RegistryAuth::Anonymous),
                "Expected Anonymous auth when config not found"
            );
        });
    }

    #[test]
    fn test_get_registry_auth_no_credentials_for_registry() {
        use temp_env;

        let temp_dir = TempDir::new().unwrap();

        // Create config with credentials for a different registry
        let config_content = r#"{
            "auths": {
                "other-registry.io": {
                    "auth": "dGVzdHVzZXI6dGVzdHBhc3M="
                }
            }
        }"#;

        let config_path = create_test_docker_config(&temp_dir, config_content);
        let docker_config_dir = config_path.parent().unwrap();

        temp_env::with_var("DOCKER_CONFIG", Some(docker_config_dir), || {
            // Try to get auth for a registry not in the config
            let reference: Reference = "docker.io/library/nginx:latest".parse().unwrap();
            let auth = get_registry_auth(&reference).unwrap();

            assert!(
                matches!(auth, RegistryAuth::Anonymous),
                "Expected Anonymous auth when no credentials for registry"
            );
        });
    }

    #[test]
    fn test_reference_parsing() {
        // Test that various OCI references parse correctly
        let test_cases = vec![
            "ghcr.io/microsoft/wassette:latest",
            "docker.io/library/nginx:1.0",
            "localhost:5000/myimage:v1",
        ];

        for reference_str in test_cases {
            let reference: Reference = reference_str.parse().unwrap();
            let registry = reference.resolve_registry();
            assert!(!registry.is_empty(), "Registry should not be empty");
        }
    }

    #[test]
    fn test_registry_server_stripping() {
        // Test that trailing slashes are handled correctly
        let reference: Reference = "ghcr.io/test/image:latest".parse().unwrap();
        let server = reference
            .resolve_registry()
            .strip_suffix('/')
            .unwrap_or_else(|| reference.resolve_registry());

        // Should not have trailing slash
        assert!(!server.ends_with('/'), "Server should not end with slash");
    }

    #[test]
    fn test_explicit_credentials_take_precedence() {
        use temp_env;

        let temp_dir = TempDir::new().unwrap();

        // Create a test Docker config with basic auth
        let config_content = r#"{
            "auths": {
                "ghcr.io": {
                    "auth": "ZG9ja2VydXNlcjpkb2NrZXJwYXNz"
                }
            }
        }"#;

        let config_path = create_test_docker_config(&temp_dir, config_content);
        let docker_config_dir = config_path.parent().unwrap();

        temp_env::with_var("DOCKER_CONFIG", Some(docker_config_dir), || {
            let reference: Reference = "ghcr.io/test/image:latest".parse().unwrap();

            // Provide explicit credentials that should override Docker config
            let explicit_creds = OciCredentials {
                username: "explicituser".to_string(),
                password: "explicitpass".to_string(),
            };

            let auth =
                get_registry_auth_with_credentials(&reference, Some(explicit_creds)).unwrap();

            match auth {
                RegistryAuth::Basic(username, password) => {
                    assert_eq!(username, "explicituser");
                    assert_eq!(password, "explicitpass");
                }
                _ => panic!(
                    "Expected Basic auth with explicit credentials, got: {:?}",
                    auth
                ),
            }
        });
    }

    #[test]
    fn test_fallback_to_docker_config_when_no_explicit_credentials() {
        use temp_env;

        let temp_dir = TempDir::new().unwrap();

        // Create a test Docker config with basic auth
        let config_content = r#"{
            "auths": {
                "ghcr.io": {
                    "auth": "dGVzdHVzZXI6dGVzdHBhc3M="
                }
            }
        }"#;

        let config_path = create_test_docker_config(&temp_dir, config_content);
        let docker_config_dir = config_path.parent().unwrap();

        temp_env::with_var("DOCKER_CONFIG", Some(docker_config_dir), || {
            let reference: Reference = "ghcr.io/test/image:latest".parse().unwrap();

            // Call with None for explicit credentials - should use Docker config
            let auth = get_registry_auth_with_credentials(&reference, None).unwrap();

            match auth {
                RegistryAuth::Basic(username, password) => {
                    assert_eq!(username, "testuser");
                    assert_eq!(password, "testpass");
                }
                _ => panic!("Expected Basic auth from Docker config, got: {:?}", auth),
            }
        });
    }

    #[test]
    fn test_explicit_credentials_without_docker_config() {
        use temp_env;

        let temp_dir = TempDir::new().unwrap();

        // Set DOCKER_CONFIG to empty temp dir (no config.json)
        temp_env::with_var("DOCKER_CONFIG", Some(temp_dir.path()), || {
            let reference: Reference = "ghcr.io/test/image:latest".parse().unwrap();

            // Provide explicit credentials
            let explicit_creds = OciCredentials {
                username: "explicituser".to_string(),
                password: "explicitpass".to_string(),
            };

            let auth =
                get_registry_auth_with_credentials(&reference, Some(explicit_creds)).unwrap();

            match auth {
                RegistryAuth::Basic(username, password) => {
                    assert_eq!(username, "explicituser");
                    assert_eq!(password, "explicitpass");
                }
                _ => panic!(
                    "Expected Basic auth with explicit credentials, got: {:?}",
                    auth
                ),
            }
        });
    }

    #[test]
    fn test_anonymous_when_no_credentials_available() {
        use temp_env;

        let temp_dir = TempDir::new().unwrap();

        // Set DOCKER_CONFIG to empty temp dir (no config.json)
        temp_env::with_var("DOCKER_CONFIG", Some(temp_dir.path()), || {
            let reference: Reference = "docker.io/library/nginx:latest".parse().unwrap();

            // Call with None for explicit credentials and no Docker config
            let auth = get_registry_auth_with_credentials(&reference, None).unwrap();

            assert!(
                matches!(auth, RegistryAuth::Anonymous),
                "Expected Anonymous auth when no credentials available"
            );
        });
    }
}
