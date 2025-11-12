# Fetch Example (JavaScript/ECMAScript)

This example demonstrates a comprehensive HTTP fetch client component written in JavaScript with advanced features including streaming, retries, timeout handling, and robust error handling.

## Features

1. **Full HTTP Method Support**: GET, HEAD, POST, PUT, PATCH, DELETE, OPTIONS
2. **Request/Response Streaming**: Efficient handling without buffering entire bodies in memory
3. **Timeouts & Cancellation**: Configurable request timeouts with automatic cancellation
4. **Robust Redirect & Status Handling**: 
   - Handles 1xx informational responses (e.g., 100-Continue)
   - Properly handles 204/304 no-body responses
   - Configurable redirect following
5. **Charset & Content Handling**: 
   - Automatic charset detection from Content-Type headers
   - Binary content detection and base64 encoding
   - Text content with proper encoding
6. **Retry Logic**: 
   - Automatic retries for transient failures
   - Exponential backoff with jitter
   - Respects Retry-After headers for 429/503 responses

## Building

This example uses the JavaScript Component Model toolchain:

```bash
# Build the component
just build

# Or manually:
npm install
npm run build
```

From the repository root, inject WIT documentation into the component:

```bash
just inject-docs examples/fetch-es/fetch.wasm examples/fetch-es/wit
```

## Usage

### Simple GET Request

```javascript
// Load the component from OCI registry
Please load the component from oci://ghcr.io/microsoft/fetch-es:latest

// Fetch content
Please fetch the content of https://example.com
```

### POST Request with JSON

```javascript
Please make a POST request to https://api.example.com/data with JSON body: {"name": "test"}
```

### Request with Timeout

```javascript
Please fetch https://slow.example.com with a 5 second timeout
```

### Disable Retries

```javascript
Please fetch https://example.com without retries
```

## Options

The `fetch` function accepts an optional `request-options` record with the following fields:

- `method`: HTTP method (default: "GET")
- `headers`: List of header name-value pairs
- `body`: Request body as string
- `timeout`: Timeout in milliseconds (default: 30000, 0 = no timeout)
- `max-redirects`: Maximum number of redirects to follow (default: 10, 0 = no redirects)
- `retry`: Whether to retry on transient failures (default: true)
- `max-retries`: Maximum number of retry attempts (default: 3)

## Response

The `fetch` function returns a `response` record with:

- `status`: HTTP status code
- `status-text`: HTTP status text
- `headers`: List of response headers
- `body`: Response body (text or base64-encoded binary)
- `is-binary`: Whether body is binary
- `content-type`: Content-Type header value
- `charset`: Detected or specified character encoding

## Policy

By default, WebAssembly components do not have network access. The `policy.yaml` file defines allowed network resources:

```yaml
version: "1.0"
description: "Permission policy for fetch-es example in wassette"
permissions:
  network:
    allow:
      - host: "https://example.com/"
      - host: "https://httpbin.org/"
```

## Implementation Details

The component implements:

- **Transient Error Detection**: Automatically retries on network errors and 408/429/502/503/504 status codes
- **Exponential Backoff**: Retry delays increase exponentially with each attempt (1s, 2s, 4s, ...)
- **Jitter**: Random jitter (0-25%) added to retry delays to prevent thundering herd
- **Retry-After Header**: Respects HTTP Retry-After header for 429 and 503 responses
- **Charset Detection**: Parses Content-Type header for charset parameter
- **Binary Detection**: Distinguishes binary content types and returns base64-encoded data
- **No-Body Status Codes**: Properly handles 204 No Content and 304 Not Modified

## Source Code

The source code can be found in [`fetch.js`](fetch.js) with the WIT interface defined in [`wit/world.wit`](wit/world.wit).
