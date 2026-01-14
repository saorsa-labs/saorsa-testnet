//! MCP/DHT integration module for TUI metrics.
//!
//! This module provides integration with saorsa-core's DHT metrics system
//! to populate TUI tabs with real data from an embedded P2P node.
//!
//! # Architecture
//!
//! When `--with-dht` is enabled:
//! - An embedded saorsa-core P2PNode is started
//! - Metrics are collected periodically from the node's aggregators
//! - Metrics are converted to TUI display types and sent to the TUI
//!
//! Without `--with-dht`:
//! - DHT/EigenTrust/Health tabs show default/empty states
//! - Only gossip-layer statistics are displayed

mod metrics_collector;
mod stats_bridge;

pub use metrics_collector::{MetricsCollector, MetricsSnapshot};
pub use stats_bridge::StatsBridge;
