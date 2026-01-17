//! Screen implementations for the TUI tabs.
//!
//! Each module provides a `draw_*_tab` function that renders
//! the corresponding tab's content area.

// MCP tab is actively used
mod mcp;
pub use mcp::draw_mcp_tab;

// Legacy tabs - kept for potential future use
#[allow(dead_code)]
mod adaptive;
#[allow(dead_code)]
mod dht;
#[allow(dead_code)]
mod eigentrust;
#[allow(dead_code)]
mod health;
#[allow(dead_code)]
mod placement;
