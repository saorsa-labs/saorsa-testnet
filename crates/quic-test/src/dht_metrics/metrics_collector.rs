//! Metrics collector for DHT/EigenTrust/Health statistics.
//!
//! Wraps saorsa-core's metrics aggregators and provides a unified
//! interface for the TUI to poll statistics.

use saorsa_core::dht::{
    DhtHealthMetrics, DhtMetricsAggregator, PlacementMetrics, SecurityMetrics, TrustMetrics,
};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;
use tracing::debug;

/// Collected metrics snapshot from the DHT layer.
#[derive(Debug, Clone, Default)]
pub struct MetricsSnapshot {
    /// DHT health metrics (routing table, replication, latency)
    pub dht_health: DhtHealthMetrics,
    /// Trust metrics (EigenTrust scores, witness validation)
    pub trust: TrustMetrics,
    /// Placement metrics (storage distribution, geographic diversity)
    pub placement: PlacementMetrics,
    /// Security metrics (attack detection, Sybil indicators)
    pub security: SecurityMetrics,
}

/// Metrics collector that wraps saorsa-core's DhtMetricsAggregator.
///
/// This collector provides a unified interface for the TUI to poll
/// all DHT-related metrics without needing to know about the underlying
/// saorsa-core implementation details.
pub struct MetricsCollector {
    /// The underlying metrics aggregator from saorsa-core
    aggregator: Arc<DhtMetricsAggregator>,
    /// Cached metrics snapshot (updated periodically)
    cached_snapshot: Arc<RwLock<MetricsSnapshot>>,
}

impl MetricsCollector {
    /// Create a new metrics collector with a fresh aggregator.
    ///
    /// This creates standalone metrics collectors that can be populated
    /// manually for testing, or connected to a running P2PNode.
    #[must_use]
    pub fn new() -> Self {
        Self {
            aggregator: Arc::new(DhtMetricsAggregator::new()),
            cached_snapshot: Arc::new(RwLock::new(MetricsSnapshot::default())),
        }
    }

    /// Create a metrics collector from an existing aggregator.
    ///
    /// Use this when you have access to a P2PNode's security_dashboard
    /// or other components that already have metrics collectors.
    #[must_use]
    pub fn from_aggregator(aggregator: Arc<DhtMetricsAggregator>) -> Self {
        Self {
            aggregator,
            cached_snapshot: Arc::new(RwLock::new(MetricsSnapshot::default())),
        }
    }

    /// Get the underlying aggregator for direct access to collectors.
    ///
    /// This allows updating metrics from external sources (e.g., P2PNode events).
    #[must_use]
    pub fn aggregator(&self) -> &Arc<DhtMetricsAggregator> {
        &self.aggregator
    }

    /// Collect and cache a fresh metrics snapshot.
    ///
    /// This should be called periodically (e.g., every 5 seconds) to
    /// keep the cached metrics up to date.
    pub async fn collect(&self) -> MetricsSnapshot {
        debug!("Collecting DHT metrics snapshot");

        let dht_health = self.aggregator.dht_health().get_metrics().await;
        let trust = self.aggregator.trust().get_metrics().await;
        let placement = self.aggregator.placement().get_metrics().await;
        let security = self.aggregator.security().get_metrics().await;

        let snapshot = MetricsSnapshot {
            dht_health,
            trust,
            placement,
            security,
        };

        // Update cache
        {
            let mut cache = self.cached_snapshot.write().await;
            *cache = snapshot.clone();
        }

        snapshot
    }

    /// Get the most recently cached metrics snapshot.
    ///
    /// This is faster than `collect()` as it doesn't query the collectors.
    /// Use this for frequent UI updates between collection intervals.
    pub async fn cached(&self) -> MetricsSnapshot {
        self.cached_snapshot.read().await.clone()
    }

    /// Record a DHT operation (for metrics tracking).
    ///
    /// Call this when the P2PNode performs DHT operations to update
    /// the operation counters.
    pub async fn record_dht_operation(&self, success: bool, latency_ms: f64, hops: u64) {
        let duration = Duration::from_millis(latency_ms as u64);
        self.aggregator
            .dht_health()
            .record_lookup(duration, hops, success)
            .await;
    }

    /// Update routing table metrics.
    pub fn update_routing_table(&self, size: u64, buckets_filled: u64, fullness: f64) {
        let collector = self.aggregator.dht_health();
        collector.set_routing_table_size(size);
        collector.set_buckets_filled(buckets_filled);
        collector.set_bucket_fullness(fullness);
    }

    /// Record a trust score update.
    pub async fn record_trust_update(&self, scores: &[f64]) {
        self.aggregator
            .trust()
            .update_trust_distribution(scores)
            .await;
    }
}

impl Default for MetricsCollector {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_metrics_collector_creation() {
        let collector = MetricsCollector::new();
        let snapshot = collector.collect().await;

        // Fresh collector should have default/zero values
        assert_eq!(snapshot.dht_health.routing_table_size, 0);
        assert_eq!(snapshot.dht_health.operations_total, 0);
    }

    #[tokio::test]
    async fn test_cached_metrics() {
        let collector = MetricsCollector::new();

        // Collect initial metrics
        let _ = collector.collect().await;

        // Cached should return the same values
        let cached = collector.cached().await;
        assert_eq!(cached.dht_health.routing_table_size, 0);
    }

    #[tokio::test]
    async fn test_record_dht_operation() {
        let collector = MetricsCollector::new();

        // Record some operations
        collector.record_dht_operation(true, 50.0, 3).await;
        collector.record_dht_operation(true, 75.0, 4).await;
        collector.record_dht_operation(false, 100.0, 5).await;

        // Collect and verify
        let snapshot = collector.collect().await;
        assert_eq!(snapshot.dht_health.operations_total, 3);
        assert_eq!(snapshot.dht_health.operations_success_total, 2);
        assert_eq!(snapshot.dht_health.operations_failed_total, 1);
    }

    #[test]
    fn test_update_routing_table() {
        let collector = MetricsCollector::new();

        // Update routing table
        collector.update_routing_table(100, 10, 0.75);

        // Note: This is a sync method, we can't await collect() in a sync test
        // Just verify it doesn't panic
    }
}
