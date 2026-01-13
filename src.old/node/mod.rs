// Copyright 2024 Saorsa Labs Limited
//
// Node management module for Saorsa TestNet

pub mod bootstrap;
pub mod worker;

pub use bootstrap::BootstrapNode;
pub use worker::WorkerNode;


use std::sync::Arc;
use tokio::sync::RwLock;

/// Common node functionality
#[derive(Debug)]
#[allow(dead_code)]
pub struct NodeState {
    pub node_id: String,
    pub uptime: std::time::Duration,
    pub connections: usize,
    pub messages_sent: u64,
    pub messages_received: u64,
    pub bandwidth_used: u64,
}

impl Default for NodeState {
    fn default() -> Self {
        Self {
            node_id: uuid::Uuid::new_v4().to_string(),
            uptime: std::time::Duration::default(),
            connections: 0,
            messages_sent: 0,
            messages_received: 0,
            bandwidth_used: 0,
        }
    }
}

/// Shared node metrics
pub type SharedNodeState = Arc<RwLock<NodeState>>;