//! MCP (Model Context Protocol) Integration Module
//!
//! Provides integration with Communitas via the MCP protocol, enabling:
//! - Auto-demo user creation on startup
//! - Access to all 133 MCP tools
//! - Real-time presence and messaging
//!
//! # Architecture
//!
//! The MCP client wraps `CommunitasApp` and provides:
//! - Automatic four-word identity generation
//! - Tool invocation with formatted results
//! - Event streaming to TUI

mod client;

pub use client::{McpClient, McpClientConfig, McpToolCategory, ToolInfo};
