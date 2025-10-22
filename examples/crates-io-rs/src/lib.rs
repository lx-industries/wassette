// Copyright (c) Microsoft Corporation.
// Licensed under the MIT license.

use serde::{Deserialize, Serialize};
use spin_sdk::http::{send, Request, Response};

#[allow(warnings)]
mod bindings;

use bindings::Guest;

struct Component;

#[derive(Debug, Deserialize, Serialize)]
struct CrateResponse {
    #[serde(rename = "crate")]
    crate_info: CrateInfo,
    versions: Vec<Version>,
}

#[derive(Debug, Deserialize, Serialize)]
struct CrateInfo {
    name: String,
    #[serde(default)]
    description: Option<String>,
    #[serde(default)]
    max_version: String,
    #[serde(default)]
    downloads: u64,
    #[serde(default)]
    recent_downloads: Option<u64>,
}

#[derive(Debug, Deserialize, Serialize)]
struct Version {
    num: String,
    #[serde(default)]
    downloads: u64,
    #[serde(default)]
    created_at: String,
}

impl Guest for Component {
    fn get_crate_info(crate_name: String) -> Result<String, String> {
        spin_executor::run(async move {
            // Build the crates.io API URL
            let url = format!("https://crates.io/api/v1/crates/{}", crate_name);

            // Create and send the request
            let request = Request::get(url)
                .header("User-Agent", "wassette-crates-io-example")
                .build();

            let response: Response = send(request).await.map_err(|e| e.to_string())?;
            let status = response.status();

            if *status == 404 {
                return Err(format!("Crate '{}' not found", crate_name));
            }

            if !(200..300).contains(status) {
                return Err(format!("Request failed with status code: {}", status));
            }

            let body = String::from_utf8_lossy(response.body());

            // Parse the JSON response
            let crate_response: CrateResponse = serde_json::from_str(&body)
                .map_err(|e| format!("Failed to parse response: {}", e))?;

            // Get the latest 5 versions
            let recent_versions: Vec<String> = crate_response
                .versions
                .iter()
                .take(5)
                .map(|v| v.num.clone())
                .collect();

            // Format the response as markdown
            let mut output = format!("# {}\n\n", crate_response.crate_info.name);

            if let Some(desc) = &crate_response.crate_info.description {
                output.push_str(&format!("**Description:** {}\n\n", desc));
            }

            output.push_str(&format!(
                "**Latest version:** {}\n\n",
                crate_response.crate_info.max_version
            ));
            output.push_str(&format!(
                "**Total downloads:** {}\n\n",
                crate_response.crate_info.downloads
            ));

            if let Some(recent) = crate_response.crate_info.recent_downloads {
                output.push_str(&format!("**Recent downloads:** {}\n\n", recent));
            }

            if !recent_versions.is_empty() {
                output.push_str("**Recent versions:**\n\n");
                for version in recent_versions {
                    output.push_str(&format!("- {}\n", version));
                }
            }

            Ok(output)
        })
    }
}

bindings::export!(Component with_types_in bindings);
