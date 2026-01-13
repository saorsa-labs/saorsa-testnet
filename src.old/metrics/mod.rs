// Copyright 2024 Saorsa Labs Limited
//
// Metrics collection and export module for Saorsa TestNet

use anyhow::Result;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::io::Write;

/// Network statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkStats {
    pub timestamp: DateTime<Utc>,
    pub nodes: NodeMetrics,
    pub network: NetworkMetrics,
    pub nat: NatMetrics,
    pub adaptive: AdaptiveMetrics,
    pub performance: PerformanceMetrics,
}

/// Node-level metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeMetrics {
    pub total_nodes: usize,
    pub active_nodes: usize,
    pub bootstrap_nodes: usize,
    pub worker_nodes: usize,
    pub average_uptime: f64,
    pub churn_rate: f64,
}

/// Network-level metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkMetrics {
    pub total_connections: usize,
    pub messages_per_second: f64,
    pub bandwidth_mbps: f64,
    pub average_latency_ms: f64,
    pub packet_loss_rate: f64,
}

/// NAT traversal metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NatMetrics {
    pub traversal_attempts: u64,
    pub successful_traversals: u64,
    pub success_rate: f64,
    pub nat_types: HashMap<String, usize>,
    pub average_punch_time_ms: f64,
    pub pqc_connections: usize,
}

/// Adaptive network metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdaptiveMetrics {
    pub thompson_sampling_success_rate: f64,
    pub mab_average_reward: f64,
    pub q_learning_cache_hit_rate: f64,
    pub churn_prediction_accuracy: f64,
    pub eigentrust_convergence: f64,
    pub hyperbolic_routing_efficiency: f64,
    pub som_clustering_quality: f64,
}

/// Performance metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceMetrics {
    pub dht_lookup_latency_ms: f64,
    pub storage_operations_per_sec: f64,
    pub retrieval_success_rate: f64,
    pub replication_health: f64,
    pub gossip_propagation_time_ms: f64,
    pub cpu_usage_percent: f64,
    pub memory_usage_mb: f64,
}

impl NetworkStats {
    /// Print stats as a formatted table
    pub fn print_table(&self) {
        println!("\n╔════════════════════════════════════════════════════════════╗");
        println!("║                    NETWORK STATISTICS                     ║");
        println!("╠════════════════════════════════════════════════════════════╣");
        println!("║ Timestamp: {:<47} ║", self.timestamp.format("%Y-%m-%d %H:%M:%S UTC"));
        println!("╠════════════════════════════════════════════════════════════╣");
        
        // Node metrics
        println!("║ NODES                                                      ║");
        println!("║   Total: {:<10} Active: {:<10} Churn: {:<6.2}%     ║", 
            self.nodes.total_nodes, 
            self.nodes.active_nodes,
            self.nodes.churn_rate * 100.0
        );
        
        // Network metrics
        println!("║                                                            ║");
        println!("║ NETWORK                                                    ║");
        println!("║   Connections: {:<8} Messages/s: {:<8.1} Latency: {:<6.1}ms ║",
            self.network.total_connections,
            self.network.messages_per_second,
            self.network.average_latency_ms
        );
        
        // NAT metrics
        println!("║                                                            ║");
        println!("║ NAT TRAVERSAL                                              ║");
        println!("║   Success Rate: {:<6.1}% Avg Punch Time: {:<6.1}ms         ║",
            self.nat.success_rate * 100.0,
            self.nat.average_punch_time_ms
        );
        
        // Adaptive metrics
        println!("║                                                            ║");
        println!("║ ADAPTIVE NETWORK                                           ║");
        println!("║   Thompson: {:<6.1}% MAB: {:<6.3} Cache: {:<6.1}%          ║",
            self.adaptive.thompson_sampling_success_rate * 100.0,
            self.adaptive.mab_average_reward,
            self.adaptive.q_learning_cache_hit_rate * 100.0
        );
        
        // Performance metrics
        println!("║                                                            ║");
        println!("║ PERFORMANCE                                                ║");
        println!("║   DHT Latency: {:<6.1}ms Storage Ops/s: {:<8.1}          ║",
            self.performance.dht_lookup_latency_ms,
            self.performance.storage_operations_per_sec
        );
        
        println!("╚════════════════════════════════════════════════════════════╝\n");
    }
    
    /// Write stats to CSV
    pub fn write_csv<W: Write>(&self, writer: &mut W) -> Result<()> {
        // Write header if needed
        writeln!(writer, "timestamp,total_nodes,active_nodes,churn_rate,connections,messages_per_sec,latency_ms,nat_success_rate,thompson_success,mab_reward,cache_hit_rate")?;
        
        // Write data row
        writeln!(
            writer,
            "{},{},{},{:.4},{},{:.2},{:.2},{:.4},{:.4},{:.4},{:.4}",
            self.timestamp.to_rfc3339(),
            self.nodes.total_nodes,
            self.nodes.active_nodes,
            self.nodes.churn_rate,
            self.network.total_connections,
            self.network.messages_per_second,
            self.network.average_latency_ms,
            self.nat.success_rate,
            self.adaptive.thompson_sampling_success_rate,
            self.adaptive.mab_average_reward,
            self.adaptive.q_learning_cache_hit_rate
        )?;
        
        Ok(())
    }
}

/// Collect current network statistics
pub async fn collect_stats(detailed: bool) -> Result<NetworkStats> {
    // This would collect real metrics from running nodes
    // For now, return mock data for testing
    
    let mut nat_types = HashMap::new();
    nat_types.insert("Full Cone".to_string(), 25);
    nat_types.insert("Restricted".to_string(), 35);
    nat_types.insert("Port Restricted".to_string(), 20);
    nat_types.insert("Symmetric".to_string(), 15);
    nat_types.insert("CGNAT".to_string(), 5);
    
    let stats = NetworkStats {
        timestamp: Utc::now(),
        nodes: NodeMetrics {
            total_nodes: 100,
            active_nodes: 95,
            bootstrap_nodes: 3,
            worker_nodes: 92,
            average_uptime: 3600.0,
            churn_rate: 0.05,
        },
        network: NetworkMetrics {
            total_connections: 450,
            messages_per_second: 1250.5,
            bandwidth_mbps: 125.3,
            average_latency_ms: 45.2,
            packet_loss_rate: 0.001,
        },
        nat: NatMetrics {
            traversal_attempts: 1000,
            successful_traversals: 920,
            success_rate: 0.92,
            nat_types,
            average_punch_time_ms: 250.5,
            pqc_connections: if detailed { 30 } else { 0 },
        },
        adaptive: AdaptiveMetrics {
            thompson_sampling_success_rate: 0.85,
            mab_average_reward: 0.72,
            q_learning_cache_hit_rate: 0.68,
            churn_prediction_accuracy: 0.81,
            eigentrust_convergence: 0.95,
            hyperbolic_routing_efficiency: 0.88,
            som_clustering_quality: 0.76,
        },
        performance: PerformanceMetrics {
            dht_lookup_latency_ms: 120.5,
            storage_operations_per_sec: 250.0,
            retrieval_success_rate: 0.98,
            replication_health: 0.95,
            gossip_propagation_time_ms: 450.0,
            cpu_usage_percent: if detailed { 35.5 } else { 0.0 },
            memory_usage_mb: if detailed { 512.0 } else { 0.0 },
        },
    };
    
    Ok(stats)
}

/// Export metrics to file
#[allow(dead_code)]
pub fn export_metrics(stats: &NetworkStats, path: &std::path::Path) -> Result<()> {
    use std::fs::File;
    
    // Determine format from extension
    let ext = path.extension()
        .and_then(|s| s.to_str())
        .unwrap_or("json");
    
    match ext {
        "json" => {
            let file = File::create(path)?;
            serde_json::to_writer_pretty(file, stats)?;
        }
        "csv" => {
            let mut file = File::create(path)?;
            stats.write_csv(&mut file)?;
        }
        _ => {
            anyhow::bail!("Unsupported export format: {}", ext);
        }
    }
    
    Ok(())
}