// Copyright (c) Microsoft Corporation.
// Licensed under the MIT license.

//! MCP logging layer for tracing
//!
//! This module provides a tracing layer that forwards log messages to MCP clients
//! via logging notifications according to the MCP protocol specification.

use std::sync::{Arc, Mutex};

use rmcp::model::{LoggingLevel, LoggingMessageNotificationParam};
use tracing::{Event, Level, Subscriber};
use tracing_subscriber::layer::Context;
use tracing_subscriber::Layer;

/// A tracing layer that forwards log messages to MCP clients
pub struct McpLoggingLayer {
    peer: Arc<Mutex<Option<rmcp::Peer<rmcp::RoleServer>>>>,
    min_level: Arc<Mutex<Option<LoggingLevel>>>,
}

impl McpLoggingLayer {
    /// Create a new MCP logging layer
    pub fn new(
        peer: Arc<Mutex<Option<rmcp::Peer<rmcp::RoleServer>>>>,
        min_level: Arc<Mutex<Option<LoggingLevel>>>,
    ) -> Self {
        Self { peer, min_level }
    }

    /// Convert tracing Level to MCP LoggingLevel
    fn tracing_to_mcp_level(level: &Level) -> LoggingLevel {
        match *level {
            Level::TRACE => LoggingLevel::Debug,
            Level::DEBUG => LoggingLevel::Debug,
            Level::INFO => LoggingLevel::Info,
            Level::WARN => LoggingLevel::Warning,
            Level::ERROR => LoggingLevel::Error,
        }
    }

    /// Check if a log level should be forwarded based on the current minimum level
    fn should_forward(&self, level: LoggingLevel) -> bool {
        let min_level = self.min_level.lock().unwrap();
        if let Some(min) = *min_level {
            // Compare log levels - lower numeric value means higher severity
            // Emergency=0, Alert=1, Critical=2, Error=3, Warning=4, Notice=5, Info=6, Debug=7
            (level as u8) <= (min as u8)
        } else {
            // If no level is set, don't forward any logs
            false
        }
    }
}

impl<S: Subscriber> Layer<S> for McpLoggingLayer {
    fn on_event(&self, event: &Event<'_>, _ctx: Context<'_, S>) {
        let metadata = event.metadata();
        let mcp_level = Self::tracing_to_mcp_level(metadata.level());

        // Check if we should forward this log level
        if !self.should_forward(mcp_level) {
            return;
        }

        // Get the peer
        let peer_guard = self.peer.lock().unwrap();
        let peer = match peer_guard.as_ref() {
            Some(p) => p.clone(),
            None => return, // No peer available yet
        };
        drop(peer_guard);

        // Extract message and fields from the event
        let mut visitor = McpVisitor::default();
        event.record(&mut visitor);

        let data = serde_json::json!({
            "message": visitor.message,
            "target": metadata.target(),
            "timestamp": chrono::Utc::now().to_rfc3339(),
        });

        let logger = Some(metadata.target().to_string());

        // Send the notification in a fire-and-forget manner
        let param = LoggingMessageNotificationParam {
            level: mcp_level,
            logger,
            data,
        };

        tokio::spawn(async move {
            if let Err(e) = peer.notify_logging_message(param).await {
                eprintln!("Failed to send MCP logging notification: {}", e);
            }
        });
    }
}

/// Visitor to extract message from tracing events
#[derive(Default)]
struct McpVisitor {
    message: String,
}

impl tracing::field::Visit for McpVisitor {
    fn record_debug(&mut self, field: &tracing::field::Field, value: &dyn std::fmt::Debug) {
        if field.name() == "message" {
            self.message = format!("{:?}", value);
        }
    }

    fn record_str(&mut self, field: &tracing::field::Field, value: &str) {
        if field.name() == "message" {
            self.message = value.to_string();
        }
    }
}
