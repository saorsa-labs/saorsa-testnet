//! Stats bridge for converting saorsa-core metrics to TUI display types.
//!
//! This module bridges the gap between saorsa-core's DHT metrics types
//! and the TUI's display types, providing conversion functions that
//! transform raw metrics into user-friendly display formats.

use super::MetricsSnapshot;
use crate::tui::{
    AlertSeverity, AnomalyEntry, ComponentHealth, DhtOperationStats, DhtStats, EigenTrustStats,
    HealthAlert, HealthStats, HealthStatus, LatencyStats, McpConnectionStatus, McpState,
    PlacementStats, RegionStats, ResourceUsage, TrustEntry,
};
use std::time::Instant;

/// Stats bridge for converting metrics to TUI types.
pub struct StatsBridge;

impl StatsBridge {
    /// Convert a MetricsSnapshot to TUI DhtStats.
    #[must_use]
    pub fn to_dht_stats(snapshot: &MetricsSnapshot) -> DhtStats {
        let dht = &snapshot.dht_health;

        // Create k-bucket distribution (estimate from routing table size)
        // Real implementation would track actual bucket distribution
        let k_buckets = Self::estimate_k_buckets(dht.routing_table_size, dht.buckets_filled);

        let operations = DhtOperationStats {
            gets: dht.operations_total / 2, // Estimate: half are gets
            get_successes: dht.operations_success_total / 2,
            puts: dht.operations_total / 2,
            put_successes: dht.operations_success_total / 2,
            deletes: 0,
            delete_successes: 0,
            routing_queries: dht.bucket_refresh_total,
            routing_successes: dht.bucket_refresh_total,
        };

        let latency = LatencyStats {
            min_ms: dht.lookup_latency_p50_ms as u32, // Use p50 as min estimate
            max_ms: dht.lookup_latency_p99_ms as u32, // Use p99 as max
            avg_ms: (dht.lookup_latency_p50_ms + dht.lookup_latency_p95_ms) / 2.0,
            p50_ms: dht.lookup_latency_p50_ms as u32,
            p95_ms: dht.lookup_latency_p95_ms as u32,
            p99_ms: dht.lookup_latency_p99_ms as u32,
            samples: dht.operations_total,
            history: Vec::new(),
        };

        DhtStats {
            k_buckets,
            total_routing_peers: dht.routing_table_size as usize,
            operations,
            latency,
            stored_records: dht.under_replicated_keys as usize, // Placeholder
            records_by_type: std::collections::HashMap::new(),
            replication_factor: dht.replication_factor as usize,
            last_refresh: Some(Instant::now()),
            distance_samples: Vec::new(),
        }
    }

    /// Convert a MetricsSnapshot to TUI EigenTrustStats.
    #[must_use]
    pub fn to_eigentrust_stats(snapshot: &MetricsSnapshot) -> EigenTrustStats {
        let trust = &snapshot.trust;

        // Convert trust distribution to peer entries
        let peer_trust_scores: Vec<TrustEntry> = trust
            .trust_distribution
            .iter()
            .map(|(bucket, count)| TrustEntry {
                short_id: bucket.clone(),
                peer_id: bucket.clone(),
                score: Self::bucket_to_score(bucket),
                pre_trusted: false,
                suspicious: Self::bucket_to_score(bucket) < 0.3,
                transactions: *count,
            })
            .collect();

        EigenTrustStats {
            local_trust_score: trust.eigentrust_avg,
            peer_trust_scores,
            convergence_iterations: trust.eigentrust_epochs_total as u32,
            converged: trust.eigentrust_epochs_total > 0,
            pre_trusted_count: 0,
            suspicious_count: trust.low_trust_nodes as usize,
            trust_threshold: 0.3, // Default threshold
            trust_history: vec![trust.eigentrust_avg],
            last_update: Some(Instant::now()),
        }
    }

    /// Convert a MetricsSnapshot to TUI PlacementStats.
    #[must_use]
    pub fn to_placement_stats(snapshot: &MetricsSnapshot) -> PlacementStats {
        let placement = &snapshot.placement;

        let regions = if placement.regions_covered > 0 {
            (0..placement.regions_covered)
                .map(|i| {
                    let node_count = (placement.storage_nodes / placement.regions_covered) as usize;
                    RegionStats {
                        name: format!("Region-{}", i + 1),
                        code: format!("R{}", i + 1),
                        flag: "🌐".to_string(),
                        node_count,
                        data_percentage: 100.0 / placement.regions_covered as f64,
                        percentage: 100.0 / placement.regions_covered as f64,
                        avg_latency_ms: 50.0, // Placeholder
                        healthy: true,
                    }
                })
                .collect()
        } else {
            Vec::new()
        };

        PlacementStats {
            geographic_diversity: placement.geographic_diversity,
            rack_diversity: 0.0, // Not tracked in saorsa-core
            network_diversity: placement.load_balance_score,
            overall_diversity: (placement.geographic_diversity + placement.load_balance_score)
                / 2.0,
            regions,
            placement_success_rate: if placement.audits_total > 0 {
                (placement.audits_total - placement.audit_failures_total) as f64
                    / placement.audits_total as f64
            } else {
                1.0
            },
            total_placements: placement.total_records,
            successful_placements: placement.total_records - placement.audit_failures_total,
            failed_placements: placement.audit_failures_total,
            placement_retries: placement.rebalance_operations_total,
            replication_targets: Vec::new(),
            target_replicas: 8, // Default replication factor
            min_regions: 3,
            avg_replica_count: 8.0,
            under_replicated_count: placement.audit_failures_total,
        }
    }

    /// Convert a MetricsSnapshot to TUI HealthStats.
    #[must_use]
    pub fn to_health_stats(snapshot: &MetricsSnapshot, uptime_secs: u64) -> HealthStats {
        let security = &snapshot.security;
        let dht = &snapshot.dht_health;

        // Calculate overall health score
        let overall_score = Self::calculate_health_score(snapshot);
        let status = Self::score_to_status(overall_score);

        // Create component health entries
        let components = vec![
            ComponentHealth {
                name: "DHT".to_string(),
                status: if dht.routing_table_size > 0 {
                    HealthStatus::Healthy
                } else {
                    HealthStatus::Degraded
                },
                healthy: dht.routing_table_size > 0,
                uptime_secs,
                last_error: None,
                message: Some(format!("{} peers in routing table", dht.routing_table_size)),
                last_check: Some(Instant::now()),
            },
            ComponentHealth {
                name: "Replication".to_string(),
                status: if dht.replication_health > 0.9 {
                    HealthStatus::Healthy
                } else if dht.replication_health > 0.7 {
                    HealthStatus::Degraded
                } else {
                    HealthStatus::Unhealthy
                },
                healthy: dht.replication_health > 0.7,
                uptime_secs,
                last_error: None,
                message: Some(format!(
                    "{} under-replicated keys",
                    dht.under_replicated_keys
                )),
                last_check: Some(Instant::now()),
            },
            ComponentHealth {
                name: "Security".to_string(),
                status: if security.sybil_score < 0.1 {
                    HealthStatus::Healthy
                } else if security.sybil_score < 0.3 {
                    HealthStatus::Degraded
                } else {
                    HealthStatus::Unhealthy
                },
                healthy: security.sybil_score < 0.3,
                uptime_secs,
                last_error: None,
                message: Some(format!(
                    "Sybil risk score: {:.1}%",
                    security.sybil_score * 100.0
                )),
                last_check: Some(Instant::now()),
            },
        ];

        // Create alerts from security metrics
        let mut alerts = Vec::new();
        if security.sybil_score > 0.1 {
            alerts.push(HealthAlert {
                severity: if security.sybil_score > 0.3 {
                    AlertSeverity::Critical
                } else {
                    AlertSeverity::Warning
                },
                message: format!(
                    "Elevated Sybil attack risk: {:.1}%",
                    security.sybil_score * 100.0
                ),
                component: "Security".to_string(),
                timestamp: Instant::now(),
                timestamp_secs_ago: 0,
                acknowledged: false,
            });
        }

        if security.eclipse_score > 0.1 {
            alerts.push(HealthAlert {
                severity: AlertSeverity::Warning,
                message: format!(
                    "Eclipse attack risk: {:.1}%",
                    security.eclipse_score * 100.0
                ),
                component: "Security".to_string(),
                timestamp: Instant::now(),
                timestamp_secs_ago: 0,
                acknowledged: false,
            });
        }

        // Create anomaly entries
        let anomalies: Vec<AnomalyEntry> = if security.sybil_nodes_detected_total > 0 {
            vec![AnomalyEntry {
                anomaly_type: "SybilNodes".to_string(),
                description: format!(
                    "{} Sybil nodes detected",
                    security.sybil_nodes_detected_total
                ),
                score: security.sybil_score,
                deviation: security.sybil_score * 3.0, // Estimate sigma
                timestamp: Instant::now(),
                peer_id: None,
            }]
        } else {
            Vec::new()
        };

        HealthStats {
            overall_score,
            status,
            components,
            alerts,
            anomalies,
            resources: ResourceUsage::default(),
            last_check: Some(Instant::now()),
            uptime_secs,
            last_check_secs_ago: 0,
        }
    }

    /// Create a default MCP state (disconnected).
    #[must_use]
    pub fn default_mcp_state() -> McpState {
        McpState {
            connection: McpConnectionStatus::Disconnected,
            server_info: None,
            tools: Vec::new(),
            selected_tool: None,
            parameter_inputs: std::collections::HashMap::new(),
            history: Vec::new(),
            last_error: None,
            endpoint: None,
        }
    }

    /// Estimate k-bucket distribution from routing table size.
    fn estimate_k_buckets(routing_size: u64, buckets_filled: u64) -> Vec<usize> {
        if buckets_filled == 0 {
            return vec![0; 256]; // Empty routing table
        }

        let avg_per_bucket = routing_size / buckets_filled;
        let mut buckets = vec![0usize; 256];

        // Distribute peers across filled buckets (closer buckets have more peers)
        let fill_count = (buckets_filled as usize).min(256);
        for (i, bucket) in buckets.iter_mut().take(fill_count).enumerate() {
            // More peers in closer buckets (lower distance)
            let weight = (256 - i) as f64 / 256.0;
            *bucket = (avg_per_bucket as f64 * (1.0 + weight)) as usize;
        }

        buckets
    }

    /// Convert bucket name to score.
    fn bucket_to_score(bucket: &str) -> f64 {
        // Bucket names like "0.0-0.1", "0.1-0.2", etc.
        if let Some(mid) = bucket.split('-').next() {
            mid.parse().unwrap_or(0.5)
        } else {
            0.5
        }
    }

    /// Calculate overall health score from metrics.
    fn calculate_health_score(snapshot: &MetricsSnapshot) -> f64 {
        let dht_score = snapshot.dht_health.success_rate;
        let security_score = 1.0 - snapshot.security.sybil_score;
        let trust_score = snapshot.trust.eigentrust_avg;
        let placement_score = snapshot.placement.geographic_diversity;

        // Weighted average
        (dht_score * 0.3 + security_score * 0.3 + trust_score * 0.2 + placement_score * 0.2)
            .clamp(0.0, 1.0)
    }

    /// Convert health score to status.
    fn score_to_status(score: f64) -> HealthStatus {
        if score >= 0.9 {
            HealthStatus::Healthy
        } else if score >= 0.7 {
            HealthStatus::Degraded
        } else if score >= 0.5 {
            HealthStatus::Unhealthy
        } else {
            HealthStatus::Critical
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use saorsa_core::dht::{DhtHealthMetrics, PlacementMetrics, SecurityMetrics, TrustMetrics};

    #[test]
    fn test_to_dht_stats() {
        let snapshot = MetricsSnapshot {
            dht_health: DhtHealthMetrics {
                routing_table_size: 100,
                buckets_filled: 10,
                operations_total: 50,
                operations_success_total: 45,
                success_rate: 0.9,
                lookup_latency_p50_ms: 50.0,
                lookup_latency_p95_ms: 100.0,
                lookup_latency_p99_ms: 150.0,
                ..Default::default()
            },
            ..Default::default()
        };

        let dht_stats = StatsBridge::to_dht_stats(&snapshot);
        assert_eq!(dht_stats.total_routing_peers, 100);
        assert_eq!(dht_stats.operations.gets + dht_stats.operations.puts, 50);
    }

    #[test]
    fn test_to_eigentrust_stats() {
        let snapshot = MetricsSnapshot {
            trust: TrustMetrics {
                eigentrust_avg: 0.75,
                eigentrust_epochs_total: 10,
                low_trust_nodes: 2,
                ..Default::default()
            },
            ..Default::default()
        };

        let trust_stats = StatsBridge::to_eigentrust_stats(&snapshot);
        assert!((trust_stats.local_trust_score - 0.75).abs() < 0.001);
        assert_eq!(trust_stats.suspicious_count, 2);
        assert!(trust_stats.converged);
    }

    #[test]
    fn test_health_score_calculation() {
        let healthy_snapshot = MetricsSnapshot {
            dht_health: DhtHealthMetrics {
                success_rate: 0.95,
                ..Default::default()
            },
            security: SecurityMetrics {
                sybil_score: 0.05,
                eclipse_score: 0.02,
                ..Default::default()
            },
            trust: TrustMetrics {
                eigentrust_avg: 0.9,
                ..Default::default()
            },
            placement: PlacementMetrics {
                geographic_diversity: 0.85,
                ..Default::default()
            },
        };

        let score = StatsBridge::calculate_health_score(&healthy_snapshot);
        assert!(score > 0.8, "Healthy metrics should produce high score");
    }
}
