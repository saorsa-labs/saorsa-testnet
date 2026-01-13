//! Screen implementations for the TUI tabs.
//!
//! Each module provides a `draw_*_tab` function that renders
//! the corresponding tab's content area.

mod adaptive;
mod dht;
mod eigentrust;
mod health;
mod mcp;
mod placement;

pub use adaptive::draw_adaptive_tab;
pub use dht::draw_dht_tab;
pub use eigentrust::draw_eigentrust_tab;
pub use health::draw_health_tab;
pub use mcp::draw_mcp_tab;
pub use placement::draw_placement_tab;
