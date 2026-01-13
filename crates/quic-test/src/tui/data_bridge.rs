//! Data bridge for converting saorsa-core types to TUI display types.
//!
//! This module provides conversion functions that transform network layer
//! statistics into the simplified types used by the TUI screens.

use crate::tui::types::{
    AdaptiveStats, AlertSeverity, AnomalyEntry, ArmStats, ChurnPredictionStats, ChurnRiskEntry,
    ChurnRiskLevel, ComponentHealth, DhtOperationStats, DhtStats, EigenTrustStats, HealthAlert,
    HealthStats, HealthStatus, LatencyStats, McpConnectionStatus, McpState, PlacementStats,
    QLearningStats, RegionStats, ResourceUsage, StrategyPerformance, ThompsonSamplingStats,
    TrustEntry,
};
use std::time::{Duration, Instant};

/// Bridge for converting DHT metrics to TUI display format.
pub struct DhtBridge;

impl DhtBridge {
    /// Create DhtStats from routing table information.
    #[allow(dead_code)]
    pub fn from_routing_table(
        k_buckets: Vec<usize>,
        operations: DhtOperationStats,
        latency: LatencyStats,
        stored_records: usize,
        replication_factor: usize,
    ) -> DhtStats {
        DhtStats {
            total_routing_peers: k_buckets.iter().sum(),
            k_buckets,
            operations,
            latency,
            stored_records,
            records_by_type: std::collections::HashMap::new(),
            replication_factor,
            last_refresh: Some(Instant::now()),
            distance_samples: Vec::new(),
        }
    }

    /// Update DHT operations from counters.
    #[allow(dead_code)]
    pub fn update_operations(
        stats: &mut DhtStats,
        gets: u64,
        puts: u64,
        deletes: u64,
        get_successes: u64,
        put_successes: u64,
        delete_successes: u64,
    ) {
        stats.operations.gets = gets;
        stats.operations.puts = puts;
        stats.operations.deletes = deletes;
        stats.operations.get_successes = get_successes;
        stats.operations.put_successes = put_successes;
        stats.operations.delete_successes = delete_successes;
    }

    /// Update latency statistics.
    #[allow(dead_code)]
    pub fn update_latency(stats: &mut DhtStats, p50: u32, p95: u32, p99: u32, avg: f64) {
        stats.latency.p50_ms = p50;
        stats.latency.p95_ms = p95;
        stats.latency.p99_ms = p99;
        stats.latency.avg_ms = avg;
    }
}

/// Bridge for converting EigenTrust metrics to TUI display format.
pub struct TrustBridge;

impl TrustBridge {
    /// Create EigenTrustStats from trust engine data.
    #[allow(dead_code)]
    pub fn from_trust_data(
        local_score: f64,
        peer_scores: Vec<(String, f64, bool, bool, u64)>, // (peer_id, score, pre_trusted, suspicious, transactions)
        convergence_iterations: u32,
        converged: bool,
        trust_threshold: f64,
    ) -> EigenTrustStats {
        let pre_trusted_count = peer_scores.iter().filter(|(_, _, pt, _, _)| *pt).count();
        let suspicious_count = peer_scores.iter().filter(|(_, _, _, s, _)| *s).count();

        EigenTrustStats {
            local_trust_score: local_score,
            peer_trust_scores: peer_scores
                .into_iter()
                .map(|(peer_id, score, pre_trusted, suspicious, transactions)| TrustEntry {
                    short_id: if peer_id.len() > 8 {
                        peer_id[..8].to_string()
                    } else {
                        peer_id.clone()
                    },
                    peer_id,
                    score,
                    pre_trusted,
                    suspicious,
                    transactions,
                })
                .collect(),
            convergence_iterations,
            converged,
            pre_trusted_count,
            suspicious_count,
            trust_threshold,
            trust_history: Vec::new(),
            last_update: Some(Instant::now()),
        }
    }

    /// Update local trust score.
    #[allow(dead_code)]
    pub fn update_local_score(stats: &mut EigenTrustStats, score: f64) {
        stats.local_trust_score = score;
        stats.trust_history.push(score);
        if stats.trust_history.len() > 60 {
            stats.trust_history.remove(0);
        }
        stats.last_update = Some(Instant::now());
    }
}

/// Bridge for converting adaptive network stats to TUI display format.
pub struct AdaptiveBridge;

impl AdaptiveBridge {
    /// Create AdaptiveStats from learning system data.
    #[allow(dead_code)]
    pub fn from_learning_data(
        arms: Vec<(String, u64, u64, f64, u64)>, // (name, successes, failures, prob, pulls)
        best_arm: Option<usize>,
        exploration_ratio: f64,
        q_learning: QLearningStats,
        churn: ChurnPredictionStats,
        strategy_scores: (f64, f64, f64),
        active_strategy: &str,
    ) -> AdaptiveStats {
        let total_pulls: u64 = arms.iter().map(|(_, _, _, _, p)| p).sum();

        AdaptiveStats {
            thompson_sampling: ThompsonSamplingStats {
                arms: arms
                    .into_iter()
                    .map(|(name, successes, failures, estimated_prob, pulls)| ArmStats {
                        name,
                        successes,
                        failures,
                        estimated_prob,
                        pulls,
                    })
                    .collect(),
                total_pulls,
                best_arm,
                exploration_ratio,
            },
            q_learning,
            churn_prediction: churn,
            strategy_performance: StrategyPerformance {
                thompson_score: strategy_scores.0,
                qlearning_score: strategy_scores.1,
                random_score: strategy_scores.2,
                active_strategy: active_strategy.to_string(),
            },
        }
    }

    /// Create Q-Learning stats.
    #[allow(dead_code, clippy::too_many_arguments)]
    pub fn create_qlearning_stats(
        state_count: usize,
        action_count: usize,
        cache_hit_rate: f64,
        learning_rate: f64,
        discount_factor: f64,
        epsilon: f64,
        episodes: u64,
        avg_reward: f64,
    ) -> QLearningStats {
        QLearningStats {
            state_count,
            action_count,
            cache_hit_rate,
            cache_miss_rate: 100.0 - cache_hit_rate,
            learning_rate,
            discount_factor,
            epsilon,
            episodes,
            avg_reward,
        }
    }

    /// Create churn prediction stats.
    #[allow(dead_code)]
    pub fn create_churn_stats(
        at_risk: Vec<(String, f64, Option<Duration>)>,
        accuracy: f64,
        predictions: u64,
        correct: u64,
        false_positives: u64,
        false_negatives: u64,
    ) -> ChurnPredictionStats {
        ChurnPredictionStats {
            at_risk_peers: at_risk
                .into_iter()
                .map(|(short_id, risk, time_to_churn)| {
                    let level = if risk >= 0.8 {
                        ChurnRiskLevel::Critical
                    } else if risk >= 0.6 {
                        ChurnRiskLevel::High
                    } else if risk >= 0.4 {
                        ChurnRiskLevel::Medium
                    } else {
                        ChurnRiskLevel::Low
                    };
                    ChurnRiskEntry {
                        short_id,
                        risk,
                        time_to_churn,
                        level,
                    }
                })
                .collect(),
            accuracy,
            predictions,
            correct,
            false_positives,
            false_negatives,
        }
    }
}

/// Bridge for converting placement metrics to TUI display format.
pub struct PlacementBridge;

impl PlacementBridge {
    /// Create PlacementStats from placement metrics.
    #[allow(dead_code, clippy::type_complexity)]
    pub fn from_placement_data(
        diversity: (f64, f64, f64, f64), // geographic, rack, network, overall
        regions: Vec<(String, String, String, usize, f64, f64, f64, bool)>, // code, name, flag, nodes, data%, pct, latency, healthy
        placements: (u64, u64, u64, u64), // total, success, failures, retries
        replication: (u32, u32, f64, u64), // target, min_regions, avg, under_replicated
    ) -> PlacementStats {
        let total = placements.0;
        let success_rate = if total > 0 {
            (placements.1 as f64 / total as f64) * 100.0
        } else {
            100.0
        };

        PlacementStats {
            geographic_diversity: diversity.0,
            rack_diversity: diversity.1,
            network_diversity: diversity.2,
            overall_diversity: diversity.3,
            regions: regions
                .into_iter()
                .map(
                    |(code, name, flag, node_count, data_percentage, percentage, avg_latency_ms, healthy)| {
                        RegionStats {
                            code,
                            name,
                            flag,
                            node_count,
                            data_percentage,
                            percentage,
                            avg_latency_ms,
                            healthy,
                        }
                    },
                )
                .collect(),
            placement_success_rate: success_rate,
            total_placements: placements.0,
            successful_placements: placements.1,
            failed_placements: placements.2,
            placement_retries: placements.3,
            replication_targets: Vec::new(),
            target_replicas: replication.0,
            min_regions: replication.1,
            avg_replica_count: replication.2,
            under_replicated_count: replication.3,
        }
    }
}

/// Bridge for converting health metrics to TUI display format.
pub struct HealthBridge;

impl HealthBridge {
    /// Create HealthStats from health manager data.
    #[allow(dead_code, clippy::too_many_arguments)]
    pub fn from_health_data(
        status: HealthStatus,
        score: f64,
        uptime: u64,
        last_check_ago: u64,
        components: Vec<ComponentHealth>,
        alerts: Vec<HealthAlert>,
        resources: ResourceUsage,
        anomalies: Vec<AnomalyEntry>,
    ) -> HealthStats {
        HealthStats {
            status,
            overall_score: score,
            uptime_secs: uptime,
            last_check_secs_ago: last_check_ago,
            components,
            alerts,
            resources,
            anomalies,
            last_check: Some(Instant::now()),
        }
    }

    /// Create a component health entry.
    #[allow(dead_code)]
    pub fn create_component(
        name: &str,
        healthy: bool,
        uptime_secs: u64,
        last_error: Option<String>,
    ) -> ComponentHealth {
        let status = if healthy {
            HealthStatus::Healthy
        } else {
            HealthStatus::Unhealthy
        };
        ComponentHealth {
            name: name.to_string(),
            status,
            healthy,
            uptime_secs,
            last_error,
            message: None,
            last_check: Some(Instant::now()),
        }
    }

    /// Create a health alert entry.
    #[allow(dead_code)]
    pub fn create_alert(
        severity: AlertSeverity,
        message: &str,
        component: &str,
        secs_ago: u64,
    ) -> HealthAlert {
        HealthAlert {
            severity,
            message: message.to_string(),
            component: component.to_string(),
            timestamp: Instant::now(),
            timestamp_secs_ago: secs_ago,
            acknowledged: false,
        }
    }

    /// Create an anomaly entry.
    #[allow(dead_code)]
    pub fn create_anomaly(
        anomaly_type: &str,
        description: &str,
        score: f64,
        deviation: f64,
        peer_id: Option<String>,
    ) -> AnomalyEntry {
        AnomalyEntry {
            anomaly_type: anomaly_type.to_string(),
            description: description.to_string(),
            score,
            deviation,
            timestamp: Instant::now(),
            peer_id,
        }
    }

    /// Create resource usage from system metrics.
    #[allow(dead_code, clippy::too_many_arguments)]
    pub fn create_resources(
        cpu: f64,
        mem_percent: f64,
        mem_used: u64,
        mem_total: u64,
        disk_percent: f64,
        disk_used: u64,
        disk_total: u64,
        net_tx: u64,
        net_rx: u64,
    ) -> ResourceUsage {
        let net_total = net_tx + net_rx;
        // Estimate network utilization (assuming 1 Gbps max)
        let net_util = ((net_total as f64) / 125_000_000.0 * 100.0).min(100.0);

        ResourceUsage {
            cpu_percent: cpu,
            memory_percent: mem_percent,
            memory_used_bytes: mem_used,
            memory_total_bytes: mem_total,
            disk_percent,
            disk_used_bytes: disk_used,
            disk_total_bytes: disk_total,
            network_bytes_sec: net_total,
            network_utilization_percent: net_util,
            network_tx_bytes_sec: net_tx,
            network_rx_bytes_sec: net_rx,
            open_fds: 0,
            active_connections: 0,
        }
    }

    /// Convert saorsa-core HealthStatus to TUI HealthStatus.
    pub fn convert_status(status: saorsa_core::health::HealthStatus) -> HealthStatus {
        match status {
            saorsa_core::health::HealthStatus::Healthy => HealthStatus::Healthy,
            saorsa_core::health::HealthStatus::Degraded => HealthStatus::Degraded,
            saorsa_core::health::HealthStatus::Unhealthy => HealthStatus::Unhealthy,
        }
    }
}

/// Bridge for MCP state management.
pub struct McpBridge;

impl McpBridge {
    /// Create initial MCP state for a given endpoint.
    #[allow(dead_code)]
    pub fn initial_state(endpoint: Option<&str>) -> McpState {
        McpState {
            connection: McpConnectionStatus::Disconnected,
            server_info: None,
            tools: Vec::new(),
            selected_tool: None,
            parameter_inputs: std::collections::HashMap::new(),
            history: Vec::new(),
            last_error: None,
            endpoint: endpoint.map(|s| s.to_string()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dht_bridge() {
        let k_buckets = vec![5, 10, 15, 0, 0];
        let stats = DhtBridge::from_routing_table(
            k_buckets,
            DhtOperationStats::default(),
            LatencyStats::default(),
            1000,
            3,
        );

        assert_eq!(stats.total_routing_peers, 30);
        assert_eq!(stats.k_buckets.len(), 5);
        assert_eq!(stats.stored_records, 1000);
    }

    #[test]
    fn test_trust_bridge() {
        let stats = TrustBridge::from_trust_data(
            0.85,
            vec![("peer123456789".to_string(), 0.9, false, false, 100)],
            5,
            true,
            0.3,
        );

        assert_eq!(stats.local_trust_score, 0.85);
        assert_eq!(stats.peer_trust_scores.len(), 1);
        assert_eq!(stats.peer_trust_scores[0].short_id, "peer1234");
        assert!(stats.converged);
    }

    #[test]
    fn test_health_bridge_status_conversion() {
        assert_eq!(
            HealthBridge::convert_status(saorsa_core::health::HealthStatus::Healthy),
            HealthStatus::Healthy
        );
        assert_eq!(
            HealthBridge::convert_status(saorsa_core::health::HealthStatus::Degraded),
            HealthStatus::Degraded
        );
    }

    #[test]
    fn test_qlearning_stats() {
        let stats =
            AdaptiveBridge::create_qlearning_stats(100, 10, 85.0, 0.1, 0.99, 0.1, 1000, 0.75);

        assert_eq!(stats.state_count, 100);
        assert_eq!(stats.cache_hit_rate, 85.0);
        assert_eq!(stats.cache_miss_rate, 15.0);
        assert_eq!(stats.avg_reward, 0.75);
    }

    #[test]
    fn test_churn_risk_levels() {
        let stats = AdaptiveBridge::create_churn_stats(
            vec![
                ("peer1".to_string(), 0.9, None),
                ("peer2".to_string(), 0.5, None),
            ],
            80.0,
            100,
            80,
            10,
            10,
        );

        assert_eq!(stats.at_risk_peers[0].level, ChurnRiskLevel::Critical);
        assert_eq!(stats.at_risk_peers[1].level, ChurnRiskLevel::Medium);
    }

    #[test]
    fn test_placement_bridge() {
        let stats = PlacementBridge::from_placement_data(
            (0.9, 0.85, 0.80, 0.85), // diversity scores
            vec![(
                "US".to_string(),
                "United States".to_string(),
                "🇺🇸".to_string(),
                50,
                45.0,
                50.0,
                25.0,
                true,
            )],
            (100, 95, 5, 3), // placements
            (3, 2, 2.8, 2),  // replication
        );

        assert_eq!(stats.geographic_diversity, 0.9);
        assert_eq!(stats.regions.len(), 1);
        assert_eq!(stats.regions[0].code, "US");
        assert_eq!(stats.total_placements, 100);
        assert_eq!(stats.successful_placements, 95);
        assert!(stats.placement_success_rate > 94.0 && stats.placement_success_rate < 96.0);
    }

    #[test]
    fn test_health_bridge() {
        let stats = HealthBridge::from_health_data(
            HealthStatus::Healthy,
            0.95,
            3600,
            5,
            vec![HealthBridge::create_component("Transport", true, 3600, None)],
            vec![],
            HealthBridge::create_resources(25.0, 50.0, 8_000_000_000, 16_000_000_000, 30.0, 100_000_000_000, 500_000_000_000, 1_000_000, 2_000_000),
            vec![],
        );

        assert_eq!(stats.status, HealthStatus::Healthy);
        assert_eq!(stats.overall_score, 0.95);
        assert_eq!(stats.components.len(), 1);
        assert!(stats.components[0].healthy);
    }

    #[test]
    fn test_mcp_bridge() {
        let state = McpBridge::initial_state(Some("http://localhost:8080"));

        assert!(matches!(
            state.connection,
            McpConnectionStatus::Disconnected
        ));
        assert_eq!(state.endpoint, Some("http://localhost:8080".to_string()));
        assert!(state.tools.is_empty());
    }

    #[test]
    fn test_component_health_creation() {
        let healthy = HealthBridge::create_component("DHT", true, 7200, None);
        assert!(healthy.healthy);
        assert_eq!(healthy.status, HealthStatus::Healthy);

        let unhealthy = HealthBridge::create_component(
            "Gossip",
            false,
            100,
            Some("Connection timeout".to_string()),
        );
        assert!(!unhealthy.healthy);
        assert_eq!(unhealthy.status, HealthStatus::Unhealthy);
        assert_eq!(unhealthy.last_error, Some("Connection timeout".to_string()));
    }

    #[test]
    fn test_alert_creation() {
        let alert = HealthBridge::create_alert(
            AlertSeverity::Warning,
            "High memory usage",
            "Resources",
            120,
        );

        assert!(matches!(alert.severity, AlertSeverity::Warning));
        assert_eq!(alert.message, "High memory usage");
        assert_eq!(alert.component, "Resources");
    }

    #[test]
    fn test_anomaly_creation() {
        let anomaly = HealthBridge::create_anomaly(
            "LatencySpike",
            "P99 latency exceeded threshold",
            0.85,
            2.5,
            Some("peer123".to_string()),
        );

        assert_eq!(anomaly.anomaly_type, "LatencySpike");
        assert_eq!(anomaly.deviation, 2.5);
        assert_eq!(anomaly.peer_id, Some("peer123".to_string()));
    }

    #[test]
    fn test_adaptive_bridge() {
        let stats = AdaptiveBridge::from_learning_data(
            vec![
                ("Direct".to_string(), 80, 20, 0.80, 100),
                ("HolePunch".to_string(), 60, 40, 0.60, 100),
                ("Relay".to_string(), 95, 5, 0.95, 100),
            ],
            Some(2), // Relay is best
            0.15,
            AdaptiveBridge::create_qlearning_stats(50, 5, 90.0, 0.1, 0.99, 0.05, 500, 0.8),
            AdaptiveBridge::create_churn_stats(vec![], 85.0, 50, 42, 4, 4),
            (0.78, 0.82, 0.50),
            "Q-Learning",
        );

        assert_eq!(stats.thompson_sampling.arms.len(), 3);
        assert_eq!(stats.thompson_sampling.best_arm, Some(2));
        assert_eq!(stats.strategy_performance.active_strategy, "Q-Learning");
    }
}
