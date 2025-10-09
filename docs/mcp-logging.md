# MCP Logging

Wassette implements the Model Context Protocol (MCP) logging specification, allowing MCP clients to receive structured log messages from the server.

## Overview

The MCP logging feature provides:

- **Structured log output**: Log messages are sent as JSON-RPC notifications to MCP clients
- **Level filtering**: Clients can set a minimum log level to control verbosity
- **Syslog severity levels**: Supports all standard syslog levels (Emergency, Alert, Critical, Error, Warning, Notice, Info, Debug)
- **Automatic forwarding**: Internal tracing logs are automatically converted to MCP notifications

## Logging Capability

Wassette declares the `logging` capability in its server capabilities:

```json
{
  "capabilities": {
    "logging": {},
    "tools": { "listChanged": true }
  }
}
```

## Setting Log Level

Clients can control log verbosity using the `logging/setLevel` request:

```json
{
  "jsonrpc": "2.0",
  "method": "logging/setLevel",
  "params": {
    "level": "info"
  },
  "id": 2
}
```

**Supported levels** (in order of decreasing severity):
- `emergency` - System is unusable
- `alert` - Action must be taken immediately
- `critical` - Critical conditions
- `error` - Error conditions
- `warning` - Warning conditions
- `notice` - Normal but significant events
- `info` - General informational messages
- `debug` - Detailed debugging information

## Log Message Notifications

After setting a log level, the server sends log messages as `notifications/message` notifications:

```json
{
  "jsonrpc": "2.0",
  "method": "notifications/message",
  "params": {
    "level": "info",
    "logger": "wassette",
    "data": {
      "message": "Component loaded successfully",
      "target": "wassette::lifecycle",
      "timestamp": "2025-01-09T12:34:56.789Z"
    }
  }
}
```

## Log Level Filtering

Only log messages at or above the configured minimum level are sent to clients. For example, if the level is set to `info`:

- ✅ `emergency`, `alert`, `critical`, `error`, `warning`, `notice`, and `info` messages are sent
- ❌ `debug` messages are filtered out

## Implementation Details

### Architecture

Wassette uses a custom `tracing` subscriber layer (`McpLoggingLayer`) that:

1. Intercepts log events from the `tracing` framework
2. Converts them to MCP `LoggingMessageNotificationParam` structures
3. Filters based on the client-configured minimum level
4. Sends notifications to connected MCP clients

### Level Mapping

Tracing levels are mapped to MCP levels as follows:

| Tracing Level | MCP Level |
|---------------|-----------|
| `ERROR`       | `error`   |
| `WARN`        | `warning` |
| `INFO`        | `info`    |
| `DEBUG`       | `debug`   |
| `TRACE`       | `debug`   |

### Message Structure

Each log notification includes:

- **level**: The log severity level
- **logger**: The source module or component (optional)
- **data**: A JSON object containing:
  - `message`: The log message text
  - `target`: The tracing target (typically module path)
  - `timestamp`: RFC3339-formatted timestamp

## Example Usage

### With MCP Inspector

```bash
# Start Wassette server
cargo run -- serve --sse

# In another terminal, connect with MCP inspector
npx @modelcontextprotocol/inspector --cli http://127.0.0.1:9001/sse

# Set log level to info
# (Use inspector UI to send logging/setLevel request)
```

### Programmatic Example

```javascript
// Connect to Wassette MCP server
const client = new MCPClient(transport);

// Initialize connection
await client.initialize({
  protocolVersion: "2024-11-05",
  clientInfo: { name: "my-client", version: "1.0.0" }
});

// Set log level
await client.request({
  method: "logging/setLevel",
  params: { level: "info" }
});

// Listen for log notifications
client.on("notifications/message", (notification) => {
  const { level, logger, data } = notification.params;
  console.log(`[${level}] ${logger}: ${data.message}`);
});
```

## Security Considerations

- Log messages should not contain sensitive information (credentials, tokens, etc.)
- The logging layer automatically filters out logs when no client has set a log level
- Each client can set their own log level independently

## Troubleshooting

### No log messages received

1. Verify the logging capability is declared in server capabilities
2. Ensure you've sent a `logging/setLevel` request
3. Check that the log level is appropriate (e.g., `debug` for maximum verbosity)
4. Verify the MCP client is listening for `notifications/message`

### Too many/too few log messages

Adjust the log level:
- Use `debug` for detailed troubleshooting
- Use `info` for normal operation
- Use `warning` or `error` for production monitoring

## Reference

- [MCP Logging Specification](https://modelcontextprotocol.io/specification/2025-06-18/server/utilities/logging)
- [Tracing Documentation](https://docs.rs/tracing/latest/tracing/)
- [Syslog Severity Levels (RFC 5424)](https://datatracker.ietf.org/doc/html/rfc5424#section-6.2.1)
