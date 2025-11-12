// Copyright (c) Microsoft Corporation.
// Licensed under the MIT license.

use spin_sdk::http::{send, Method, Request, Response};
use url::Url;

#[allow(warnings)]
mod bindings;

use base64::Engine;
use bindings::component::fetch_rs::types::{HttpHeader, HttpMethod, HttpResponse, RequestOptions};
use bindings::Guest;
use encoding_rs::Encoding;
use serde_json::Value;

struct Component;

impl Guest for Component {
    fn fetch(url: String) -> Result<String, String> {
        spin_executor::run(async move {
            let request = Request::get(url);
            let response: Response = send(request).await.map_err(|e| e.to_string())?;
            let status = response.status();
            if !(200..300).contains(status) {
                return Err(format!("Request failed with status code: {}", status));
            }
            let body = String::from_utf8_lossy(response.body());

            if let Some(content_type) = response.header("content-type").and_then(|v| v.as_str()) {
                if content_type.contains("application/json") {
                    let json: Value = serde_json::from_str(&body).map_err(|e| e.to_string())?;
                    return Ok(json_to_markdown(&json));
                } else if content_type.contains("text/html") {
                    return Ok(html_to_markdown(&body));
                }
            }

            Ok(body.into_owned())
        })
    }

    fn fetch_advanced(url: String, options: RequestOptions) -> Result<HttpResponse, String> {
        spin_executor::run(async move {
            let result = fetch_with_retry(&url, &options, 3).await;
            result.map_err(|e| e.to_string())
        })
    }
}

/// Fetch with retry logic for transient errors
async fn fetch_with_retry(
    url: &str,
    options: &RequestOptions,
    max_retries: u32,
) -> Result<HttpResponse, Box<dyn std::error::Error>> {
    let method = options.method.as_ref().unwrap_or(&HttpMethod::Get);
    // Per RFC 7231, GET, HEAD, OPTIONS, PUT, and DELETE are considered idempotent.
    // PUT is idempotent because it should replace the entire resource with the given representation.
    let is_idempotent = matches!(
        method,
        HttpMethod::Get
            | HttpMethod::Head
            | HttpMethod::Options
            | HttpMethod::Put
            | HttpMethod::Delete
    );

    let mut attempt = 0;
    let mut last_error = None;

    while attempt <= max_retries {
        match fetch_once(url, options).await {
            Ok(response) => {
                // Check if we should retry based on status code
                if is_transient_error(response.status) && attempt < max_retries {
                    // For transient errors, retry if idempotent
                    if !is_idempotent {
                        return Ok(response);
                    }

                    last_error = Some(format!("Transient error: status {}", response.status));
                    attempt += 1;
                    continue;
                }

                return Ok(response);
            }
            Err(e) => {
                // Check if it's a retryable network error
                let error_str = e.to_string();
                if is_retryable_error(&error_str) && attempt < max_retries && is_idempotent {
                    last_error = Some(error_str);
                    attempt += 1;
                    continue;
                }

                return Err(e);
            }
        }
    }

    Err(last_error
        .unwrap_or_else(|| "Max retries exceeded".to_string())
        .into())
}

/// Check if the error is retryable
fn is_retryable_error(error: &str) -> bool {
    error.contains("timeout")
        || error.contains("connection")
        || error.contains("network")
        || error.contains("dns")
}

/// Check if status code represents a transient error
fn is_transient_error(status: u16) -> bool {
    matches!(status, 429 | 502 | 503 | 504)
}

/// Perform a single fetch request
async fn fetch_once(
    url: &str,
    options: &RequestOptions,
) -> Result<HttpResponse, Box<dyn std::error::Error>> {
    let mut current_url = url.to_string();
    let mut current_method = options.method;
    let current_headers = options.headers.clone();
    let mut current_body = options.body.clone();

    let max_redirects = if options.follow_redirects.unwrap_or(true) {
        options.max_redirects.unwrap_or(10)
    } else {
        0
    };
    let mut redirect_count = 0;

    loop {
        // Build the request
        let method = current_method.as_ref().unwrap_or(&HttpMethod::Get);
        let spin_method = convert_method(method);

        let mut builder = Request::builder();
        builder.method(spin_method);
        builder.uri(&current_url);

        // Add custom headers
        if let Some(headers) = &current_headers {
            for header in headers {
                builder.header(&header.name, &header.value);
            }
        }

        // Add request body for methods that support it
        if let Some(body) = &current_body {
            if matches!(
                method,
                HttpMethod::Post | HttpMethod::Put | HttpMethod::Patch
            ) {
                builder.body(body.as_str());
            }
        }

        // Build the request
        let request = builder.build();

        // Send the request
        let response: Response = send(request).await?;

        // Check for redirects
        if is_redirect_status(response.status()) && redirect_count < max_redirects {
            let location = response
                .header("location")
                .and_then(|v| v.as_str())
                .ok_or("Redirect response missing Location header")?;

            // Resolve relative URLs using proper URL parsing
            current_url = resolve_relative_url(&current_url, location)?;

            // Per HTTP spec: 301 and 302 with POST should convert to GET (historical behavior)
            // 303 always converts to GET
            let status = *response.status();
            if status == 303
                || ((status == 301 || status == 302) && matches!(method, HttpMethod::Post))
            {
                current_method = Some(HttpMethod::Get);
                current_body = None; // Clear body when converting to GET
            }

            redirect_count += 1;
            continue;
        }

        // No more redirects, process the response
        return process_response(response).await;
    }
}

/// Check if status code is a redirect
fn is_redirect_status(status: &u16) -> bool {
    matches!(status, 301 | 302 | 303 | 307 | 308)
}

/// Resolve relative URL against base URL using proper URL parsing
fn resolve_relative_url(base: &str, relative: &str) -> Result<String, Box<dyn std::error::Error>> {
    let base_url = Url::parse(base)?;
    let resolved = base_url.join(relative)?;
    Ok(resolved.to_string())
}

/// Process the HTTP response
async fn process_response(response: Response) -> Result<HttpResponse, Box<dyn std::error::Error>> {
    let status = *response.status();

    // Extract headers
    let mut headers = Vec::new();
    for (name, value) in response.headers() {
        if let Some(value_str) = value.as_str() {
            headers.push(HttpHeader {
                name: name.to_string(),
                value: value_str.to_string(),
            });
        }
    }

    // Handle special status codes
    if status == 204 || status == 304 {
        // No Content or Not Modified - no body expected
        return Ok(HttpResponse {
            status,
            headers,
            body: String::new(),
            is_binary: false,
        });
    }

    // Handle 1xx informational responses
    // These are interim responses and don't have a body.
    // The client should continue waiting for the final response.
    if (100..200).contains(&status) {
        return Ok(HttpResponse {
            status,
            headers,
            body: String::new(),
            is_binary: false,
        });
    }

    // Get the response body
    let body_bytes = response.body();

    // Detect charset from Content-Type header
    let content_type = response
        .header("content-type")
        .and_then(|v| v.as_str())
        .unwrap_or("");

    let (is_binary, body_string) = decode_body(body_bytes, content_type)?;

    Ok(HttpResponse {
        status,
        headers,
        body: body_string,
        is_binary,
    })
}

/// Decode response body based on content type and charset
fn decode_body(
    body_bytes: &[u8],
    content_type: &str,
) -> Result<(bool, String), Box<dyn std::error::Error>> {
    // Check if content type indicates binary data
    if is_binary_content_type(content_type) {
        // Encode binary data as base64
        let encoded = base64::prelude::BASE64_STANDARD.encode(body_bytes);
        return Ok((true, encoded));
    }

    // Try to extract charset from Content-Type header
    let encoding = extract_charset(content_type);

    // Attempt to decode with the detected/specified encoding
    let (decoded, _, had_errors) = encoding.decode(body_bytes);

    if had_errors {
        // If decoding failed, try UTF-8
        match String::from_utf8(body_bytes.to_vec()) {
            Ok(s) => Ok((false, s)),
            Err(_) => {
                // Fall back to lossy UTF-8 conversion
                Ok((false, String::from_utf8_lossy(body_bytes).into_owned()))
            }
        }
    } else {
        Ok((false, decoded.into_owned()))
    }
}

/// Check if content type indicates binary data
fn is_binary_content_type(content_type: &str) -> bool {
    let binary_types = [
        "image/",
        "audio/",
        "video/",
        "application/octet-stream",
        "application/pdf",
        "application/zip",
        "application/x-",
    ];

    binary_types
        .iter()
        .any(|&t| content_type.to_lowercase().contains(t))
}

/// Extract charset from Content-Type header
fn extract_charset(content_type: &str) -> &'static Encoding {
    // Look for charset parameter in Content-Type
    if let Some(charset_start) = content_type.find("charset=") {
        let charset_value = &content_type[charset_start + 8..];
        let charset_name = charset_value
            .split(';')
            .next()
            .unwrap_or("")
            .trim()
            .trim_matches('"')
            .trim_matches('\'');

        if let Some(encoding) = Encoding::for_label(charset_name.as_bytes()) {
            return encoding;
        }
    }

    // Default to UTF-8
    encoding_rs::UTF_8
}

/// Convert WIT HTTP method to spin-sdk Method
fn convert_method(method: &HttpMethod) -> Method {
    match method {
        HttpMethod::Get => Method::Get,
        HttpMethod::Head => Method::Head,
        HttpMethod::Post => Method::Post,
        HttpMethod::Put => Method::Put,
        HttpMethod::Patch => Method::Patch,
        HttpMethod::Delete => Method::Delete,
        HttpMethod::Options => Method::Options,
    }
}

fn html_to_markdown(html: &str) -> String {
    let mut markdown = String::new();
    let fragment = scraper::Html::parse_fragment(html);
    let text_selector = scraper::Selector::parse("h1, h2, h3, h4, h5, h6, p, a, div").unwrap();

    for element in fragment.select(&text_selector) {
        let tag_name = element.value().name();
        let text = element
            .text()
            .collect::<Vec<_>>()
            .join(" ")
            .trim()
            .to_string();

        if text.is_empty() {
            continue;
        }

        match tag_name {
            "h1" => markdown.push_str(&format!("# {}\n\n", text)),
            "h2" => markdown.push_str(&format!("## {}\n\n", text)),
            "h3" => markdown.push_str(&format!("### {}\n\n", text)),
            "h4" => markdown.push_str(&format!("#### {}\n\n", text)),
            "h5" => markdown.push_str(&format!("##### {}\n\n", text)),
            "h6" => markdown.push_str(&format!("###### {}\n\n", text)),
            "p" => markdown.push_str(&format!("{}\n\n", text)),
            "a" => {
                if let Some(href) = element.value().attr("href") {
                    markdown.push_str(&format!("[{}]({})\n\n", text, href));
                } else {
                    markdown.push_str(&format!("{}\n\n", text));
                }
            }
            _ => markdown.push_str(&format!("{}\n\n", text)),
        }
    }

    markdown.trim().to_string()
}

fn json_to_markdown(value: &Value) -> String {
    match value {
        Value::Object(map) => {
            let mut markdown = String::new();
            for (key, val) in map {
                markdown.push_str(&format!("### {}\n\n{}\n\n", key, json_to_markdown(val)));
            }
            markdown
        }
        Value::Array(arr) => {
            let mut markdown = String::new();
            for (i, val) in arr.iter().enumerate() {
                markdown.push_str(&format!("1. {}\n", json_to_markdown(val)));
                if i < arr.len() - 1 {
                    markdown.push('\n');
                }
            }
            markdown
        }
        Value::String(s) => s.clone(),
        Value::Number(n) => n.to_string(),
        Value::Bool(b) => b.to_string(),
        Value::Null => "null".to_string(),
    }
}
bindings::export!(Component with_types_in bindings);
