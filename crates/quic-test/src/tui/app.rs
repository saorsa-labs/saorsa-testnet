//! TUI application state and event handling.
//!
//! This module manages the terminal UI state, handles user input,
//! and coordinates updates from the network layer.

use crate::gossip_tests::GossipTestResults;
use crate::tui::types::{
    AdaptiveStats, CacheHealth, ConnectedPeer, ConnectionHistoryEntry, ConnectionStatus,
    ConnectivityTestResults, DhtStats, EigenTrustStats, FrameDirection, GeographicDistribution,
    HealthStats, LocalNodeInfo, McpState, NatTraversalPhase, NatTypeAnalytics, NetworkStatistics,
    PlacementStats, ProofStatus, ProtocolFrame, TestConnectivityMethod, TrafficType,
};
use ratatui::widgets::TableState;
use std::collections::{HashMap, HashSet};
use std::time::Instant;

/// Application running state.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AppState {
    /// Application is running normally
    Running,
    /// Application is shutting down
    Quitting,
}

/// TUI Tab for navigation between different views.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Tab {
    /// Main overview (default - current layout) [1]
    #[default]
    Overview,
    /// Detailed gossip health view for all 9 saorsa-gossip crates [2]
    GossipHealth,
    /// N×N peer connectivity matrix [3]
    ConnectivityMatrix,
    /// Protocol frame log (detailed message flow) [4]
    ProtocolLog,
    /// DHT statistics and routing table [5]
    Dht,
    /// EigenTrust reputation scores [6]
    EigenTrust,
    /// Adaptive network (Thompson Sampling, Q-Learning) [7]
    Adaptive,
    /// Data placement and diversity [8]
    Placement,
    /// Overall node/network health [9]
    Health,
    /// MCP client for tool invocation [0]
    Mcp,
}

/// Main TUI application state.
#[derive(Debug)]
pub struct App {
    /// Current application state
    pub state: AppState,
    /// Local node information
    pub local_node: LocalNodeInfo,
    /// Connected peers (peer_id -> peer info)
    pub connected_peers: HashMap<String, ConnectedPeer>,
    /// Connection history (peer_id -> history entry) - persists after disconnection
    pub connection_history: HashMap<String, ConnectionHistoryEntry>,
    /// Network statistics
    pub stats: NetworkStatistics,
    /// Auto-connect enabled
    pub auto_connecting: bool,
    /// Total registered nodes in network (from registry)
    pub total_registered_nodes: usize,
    /// Peers we've actually communicated with (seen alive)
    pub peers_seen: HashSet<String>,
    /// Registry URL
    pub registry_url: String,
    /// Dashboard URL (for display)
    pub dashboard_url: String,
    /// Last UI refresh time
    pub last_refresh: Instant,
    /// Error message to display (if any)
    pub error_message: Option<String>,
    /// Info message to display (if any)
    pub info_message: Option<String>,
    /// Protocol frame log (last 20 frames)
    pub protocol_frames: Vec<ProtocolFrame>,
    /// Bootstrap cache health information
    pub cache_health: Option<CacheHealth>,
    /// NAT type analytics for connection success rates
    pub nat_analytics: Option<NatTypeAnalytics>,
    /// Geographic distribution of peers for network diversity
    pub geographic_distribution: Option<GeographicDistribution>,
    /// Connectivity test results (inbound/outbound test matrix)
    pub connectivity_test: ConnectivityTestResults,
    /// Scroll state for the connections table
    pub connections_table_state: TableState,
    /// Gossip crate test results (all 9 saorsa-gossip crates)
    pub gossip_test_results: Option<GossipTestResults>,
    /// Whether gossip tests are currently running
    pub gossip_tests_running: bool,
    /// Current active tab for navigation
    pub active_tab: Tab,
    /// Local gossip stats from epidemic gossip system
    pub gossip_stats: Option<crate::registry::NodeGossipStats>,
    /// Proof verification status (auto-run every 60s)
    pub proof_status: ProofStatus,
    /// Show proof help overlay (press P to toggle)
    pub show_proof_help: bool,
    // === New state for expanded TUI ===
    /// DHT statistics for DHT tab [5]
    pub dht_stats: DhtStats,
    /// EigenTrust statistics for Trust tab [6]
    pub eigentrust_stats: EigenTrustStats,
    /// Adaptive network stats for Adaptive tab [7]
    pub adaptive_stats: AdaptiveStats,
    /// Placement stats for Placement tab [8]
    pub placement_stats: PlacementStats,
    /// Health stats for Health tab [9]
    pub health_stats: HealthStats,
    /// MCP client state for MCP tab [0]
    pub mcp_state: McpState,
}

impl Default for App {
    fn default() -> Self {
        Self::new()
    }
}

impl App {
    /// Create a new application instance.
    pub fn new() -> Self {
        Self {
            state: AppState::Running,
            local_node: LocalNodeInfo::default(),
            connected_peers: HashMap::new(),
            connection_history: HashMap::new(),
            stats: NetworkStatistics {
                started_at: Some(Instant::now()),
                ..Default::default()
            },
            auto_connecting: true,
            total_registered_nodes: 0,
            peers_seen: HashSet::new(),
            registry_url: "https://saorsa-1.saorsalabs.com".to_string(),
            dashboard_url: "https://saorsa-1.saorsalabs.com".to_string(),
            last_refresh: Instant::now(),
            error_message: None,
            info_message: None,
            protocol_frames: Vec::new(),
            cache_health: None,
            nat_analytics: None,
            geographic_distribution: None,
            connectivity_test: ConnectivityTestResults::new(),
            connections_table_state: TableState::default(),
            gossip_test_results: None,
            gossip_tests_running: false,
            active_tab: Tab::default(),
            gossip_stats: None,
            proof_status: ProofStatus::new(),
            show_proof_help: false,
            // Initialize new state
            dht_stats: DhtStats::default(),
            eigentrust_stats: EigenTrustStats::default(),
            adaptive_stats: AdaptiveStats::default(),
            placement_stats: PlacementStats::default(),
            health_stats: HealthStats::default(),
            mcp_state: McpState::default(),
        }
    }

    /// Update proof verification status.
    pub fn update_proof_status(&mut self, status: ProofStatus) {
        self.proof_status = status;
    }

    /// Toggle the proof help overlay.
    pub fn toggle_proof_help(&mut self) {
        self.show_proof_help = !self.show_proof_help;
    }

    /// Cycle to the next tab.
    pub fn next_tab(&mut self) {
        self.active_tab = match self.active_tab {
            Tab::Overview => Tab::GossipHealth,
            Tab::GossipHealth => Tab::ConnectivityMatrix,
            Tab::ConnectivityMatrix => Tab::ProtocolLog,
            Tab::ProtocolLog => Tab::Dht,
            Tab::Dht => Tab::EigenTrust,
            Tab::EigenTrust => Tab::Adaptive,
            Tab::Adaptive => Tab::Placement,
            Tab::Placement => Tab::Health,
            Tab::Health => Tab::Mcp,
            Tab::Mcp => Tab::Overview,
        };
    }

    /// Cycle to the previous tab.
    pub fn prev_tab(&mut self) {
        self.active_tab = match self.active_tab {
            Tab::Overview => Tab::Mcp,
            Tab::GossipHealth => Tab::Overview,
            Tab::ConnectivityMatrix => Tab::GossipHealth,
            Tab::ProtocolLog => Tab::ConnectivityMatrix,
            Tab::Dht => Tab::ProtocolLog,
            Tab::EigenTrust => Tab::Dht,
            Tab::Adaptive => Tab::EigenTrust,
            Tab::Placement => Tab::Adaptive,
            Tab::Health => Tab::Placement,
            Tab::Mcp => Tab::Health,
        };
    }

    /// Update gossip stats from epidemic gossip system.
    pub fn update_gossip_stats(&mut self, stats: crate::registry::NodeGossipStats) {
        self.gossip_stats = Some(stats);
    }

    /// Update gossip test results.
    pub fn update_gossip_results(&mut self, results: GossipTestResults) {
        self.gossip_test_results = Some(results);
        self.gossip_tests_running = false;
    }

    /// Mark gossip tests as running.
    pub fn start_gossip_tests(&mut self) {
        self.gossip_tests_running = true;
    }

    /// Mark a peer as seen (we've communicated with them).
    pub fn peer_seen(&mut self, peer_id: &str) {
        self.peers_seen.insert(peer_id.to_string());
    }

    /// Get count of peers we've actually seen.
    pub fn peers_seen_count(&self) -> usize {
        self.peers_seen.len()
    }

    /// Check if the application should quit.
    pub fn should_quit(&self) -> bool {
        self.state == AppState::Quitting
    }

    /// Request application quit.
    pub fn quit(&mut self) {
        self.state = AppState::Quitting;
    }

    /// Add or update a connected peer.
    pub fn update_peer(&mut self, peer: ConnectedPeer) {
        let peer_id = peer.full_id.clone();

        if let Some(history) = self.connection_history.get_mut(&peer_id) {
            history.update_from_peer(&peer);
        } else {
            self.connection_history.insert(
                peer_id.clone(),
                ConnectionHistoryEntry::from_connected_peer(&peer),
            );
        }

        self.connected_peers.insert(peer_id, peer);
    }

    /// Remove a disconnected peer.
    pub fn remove_peer(&mut self, peer_id: &str) {
        if let Some(history) = self.connection_history.get_mut(peer_id) {
            history.mark_disconnected();
        }
        self.connected_peers.remove(peer_id);
    }

    /// Get the number of connected peers.
    pub fn connected_count(&self) -> usize {
        self.connected_peers.len()
    }

    /// Mark that we sent a packet to a peer.
    pub fn packet_sent(&mut self, peer_id: &str) {
        self.stats.packets_sent += 1;
        self.stats.bytes_sent += 5120; // 5KB test packet

        if let Some(peer) = self.connected_peers.get_mut(peer_id) {
            peer.packets_sent += 1;
            peer.tx_active = true;
        }
        if let Some(history) = self.connection_history.get_mut(peer_id) {
            history.total_packets += 1;
            history.last_seen = Instant::now();
        }
    }

    /// Mark that we received a packet from a peer.
    pub fn packet_received(&mut self, peer_id: &str) {
        self.stats.packets_received += 1;
        self.stats.bytes_received += 5120; // 5KB test packet

        if let Some(peer) = self.connected_peers.get_mut(peer_id) {
            peer.packets_received += 1;
            peer.rx_active = true;
        }
        if let Some(history) = self.connection_history.get_mut(peer_id) {
            history.total_packets += 1;
            history.last_seen = Instant::now();
        }
    }

    /// Clear traffic indicators (call periodically).
    pub fn clear_traffic_indicators(&mut self) {
        for peer in self.connected_peers.values_mut() {
            peer.tx_active = false;
            peer.rx_active = false;
        }
    }

    /// Record a connection attempt.
    pub fn connection_attempted(&mut self) {
        self.stats.connection_attempts += 1;
    }

    /// Record a successful connection.
    pub fn connection_succeeded(&mut self, method: crate::registry::ConnectionMethod) {
        self.stats.connection_successes += 1;
        match method {
            crate::registry::ConnectionMethod::Direct => {
                self.stats.direct_connections += 1;
            }
            crate::registry::ConnectionMethod::HolePunched => {
                self.stats.hole_punched_connections += 1;
            }
            crate::registry::ConnectionMethod::Relayed => {
                self.stats.relayed_connections += 1;
            }
        }
    }

    /// Record a failed connection.
    pub fn connection_failed(&mut self) {
        self.stats.connection_failures += 1;
    }

    /// Set an error message.
    pub fn set_error(&mut self, message: &str) {
        self.error_message = Some(message.to_string());
    }

    /// Clear the error message.
    pub fn clear_error(&mut self) {
        self.error_message = None;
    }

    /// Set an info message.
    pub fn set_info(&mut self, message: &str) {
        self.info_message = Some(message.to_string());
    }

    /// Clear the info message.
    pub fn clear_info(&mut self) {
        self.info_message = None;
    }

    /// Update node registration status.
    pub fn set_registered(&mut self, registered: bool) {
        self.local_node.registered = registered;
        if registered {
            self.local_node.last_heartbeat = Some(Instant::now());
        }
    }

    /// Update heartbeat timestamp.
    pub fn heartbeat_sent(&mut self) {
        self.local_node.last_heartbeat = Some(Instant::now());
    }

    /// Update RTT measurement for a peer.
    pub fn update_peer_rtt(&mut self, peer_id: &str, rtt: std::time::Duration) {
        if let Some(peer) = self.connected_peers.get_mut(peer_id) {
            peer.update_rtt(rtt);
        }
    }

    /// Get sorted list of connected peers for display.
    pub fn peers_sorted(&self) -> Vec<&ConnectedPeer> {
        let mut peers: Vec<_> = self.connected_peers.values().collect();
        peers.sort_by(|a, b| match (a.rtt, b.rtt) {
            (Some(a_rtt), Some(b_rtt)) => a_rtt.cmp(&b_rtt),
            (Some(_), None) => std::cmp::Ordering::Less,
            (None, Some(_)) => std::cmp::Ordering::Greater,
            (None, None) => a.connected_at.cmp(&b.connected_at),
        });
        peers
    }

    /// Get sorted connection history for display (connected first, then by last_seen).
    pub fn history_sorted(&self) -> Vec<&ConnectionHistoryEntry> {
        let mut history: Vec<_> = self.connection_history.values().collect();
        history.sort_by(|a, b| match (&a.status, &b.status) {
            (ConnectionStatus::Connected, ConnectionStatus::Connected) => {
                b.last_seen.cmp(&a.last_seen)
            }
            (ConnectionStatus::Connected, _) => std::cmp::Ordering::Less,
            (_, ConnectionStatus::Connected) => std::cmp::Ordering::Greater,
            _ => b.last_seen.cmp(&a.last_seen),
        });
        history
    }

    /// Get count of currently connected peers in history.
    pub fn history_connected_count(&self) -> usize {
        self.connection_history
            .values()
            .filter(|h| h.status == ConnectionStatus::Connected)
            .count()
    }

    /// Get count of disconnected peers in history.
    pub fn history_disconnected_count(&self) -> usize {
        self.connection_history
            .values()
            .filter(|h| h.status == ConnectionStatus::Disconnected)
            .count()
    }

    pub fn add_protocol_frame(&mut self, frame: ProtocolFrame) {
        if !frame.peer_id.is_empty() && frame.peer_id != "registry" {
            let peer_id = frame.peer_id.clone();
            self.connection_history
                .entry(peer_id.clone())
                .or_insert_with(|| ConnectionHistoryEntry::new(&peer_id))
                .last_seen = std::time::Instant::now();
        }

        self.protocol_frames.push(frame);
        if self.protocol_frames.len() > 200 {
            self.protocol_frames
                .drain(0..self.protocol_frames.len() - 200);
        }
        self.prune_history_if_needed();
    }

    /// Update NAT traversal phase for a peer
    pub fn update_nat_phase(
        &mut self,
        peer_id: &str,
        phase: NatTraversalPhase,
        coordinator_id: Option<String>,
    ) {
        if let Some(peer) = self.connected_peers.get_mut(peer_id) {
            peer.nat_phase = phase;
            peer.coordinator_id = coordinator_id;
        }
    }

    /// Update traffic type for a peer
    pub fn update_traffic_type(
        &mut self,
        peer_id: &str,
        traffic_type: TrafficType,
        direction: FrameDirection,
    ) {
        if let Some(peer) = self.connected_peers.get_mut(peer_id) {
            match traffic_type {
                TrafficType::Protocol => {
                    peer.protocol_tx = direction == FrameDirection::Sent;
                    peer.protocol_rx = direction == FrameDirection::Received;
                }
                TrafficType::TestData => {
                    peer.data_tx = direction == FrameDirection::Sent;
                    peer.data_rx = direction == FrameDirection::Received;
                }
                TrafficType::Relay => {
                    // Relay traffic counts as both TX and RX for visibility
                    if direction == FrameDirection::Sent {
                        peer.protocol_tx = true;
                    } else {
                        peer.protocol_rx = true;
                    }
                }
            }
        }
    }

    /// Update cache health information
    pub fn update_cache_health(&mut self, health: CacheHealth) {
        self.cache_health = Some(health);
    }

    /// Update NAT type analytics
    pub fn update_nat_analytics(&mut self, analytics: NatTypeAnalytics) {
        self.nat_analytics = Some(analytics);
    }

    pub fn update_peer_nat_test_state(
        &mut self,
        peer_id: &str,
        state: crate::tui::types::PeerNatTestState,
    ) {
        if let Some(peer) = self.connected_peers.get_mut(peer_id) {
            peer.nat_test_state = state;
        }
    }

    pub fn update_geographic_distribution(&mut self, distribution: GeographicDistribution) {
        self.geographic_distribution = Some(distribution);
    }

    pub fn start_connectivity_test(&mut self) {
        self.connectivity_test.start();
    }

    pub fn connectivity_test_inbound_phase(&mut self) {
        self.connectivity_test.start_inbound_phase();
    }

    pub fn record_inbound_connection(
        &mut self,
        peer_id: &str,
        method: TestConnectivityMethod,
        success: bool,
        rtt_ms: Option<u32>,
    ) {
        self.connectivity_test
            .record_inbound(peer_id, method, success, rtt_ms, None);
        self.record_history_attempt(
            peer_id,
            crate::registry::ConnectionDirection::Inbound,
            method,
            success,
        );
    }

    pub fn record_outbound_connection(
        &mut self,
        peer_id: &str,
        method: TestConnectivityMethod,
        success: bool,
        rtt_ms: Option<u32>,
    ) {
        self.connectivity_test
            .record_outbound(peer_id, method, success, rtt_ms, None);
        self.record_history_attempt(
            peer_id,
            crate::registry::ConnectionDirection::Outbound,
            method,
            success,
        );
    }

    fn record_history_attempt(
        &mut self,
        peer_id: &str,
        direction: crate::registry::ConnectionDirection,
        method: TestConnectivityMethod,
        success: bool,
    ) {
        let (mapped_method, is_ipv6) = method.to_registry_method();

        let entry = self
            .connection_history
            .entry(peer_id.to_string())
            .or_insert_with(|| ConnectionHistoryEntry::new(peer_id));
        entry.record_attempt_with_ip(direction, mapped_method, success, is_ipv6);

        self.prune_history_if_needed();
    }

    const MAX_HISTORY_ENTRIES: usize = 1000;

    fn prune_history_if_needed(&mut self) {
        if self.connection_history.len() <= Self::MAX_HISTORY_ENTRIES {
            return;
        }

        let mut entries: Vec<_> = self.connection_history.iter().collect();
        entries.sort_by(|a, b| a.1.last_seen.cmp(&b.1.last_seen));

        let to_remove = entries.len() - Self::MAX_HISTORY_ENTRIES;
        let keys_to_remove: Vec<_> = entries
            .into_iter()
            .take(to_remove)
            .filter(|(_, e)| e.status != ConnectionStatus::Connected)
            .map(|(k, _)| k.clone())
            .collect();

        for key in keys_to_remove {
            self.connection_history.remove(&key);
        }
    }

    pub fn connectivity_countdown(&self) -> u32 {
        self.connectivity_test.countdown_seconds()
    }

    pub fn connectivity_countdown_complete(&self) -> bool {
        self.connectivity_test.countdown_complete()
    }

    /// Core scroll logic: scrolls by `amount` (negative = up, positive = down).
    fn scroll_connections_by(&mut self, amount: isize) {
        let len = self.connection_history.len();
        if len == 0 {
            return;
        }
        let current = self.connections_table_state.selected().unwrap_or(0);
        let new_idx = if amount < 0 {
            current.saturating_sub(amount.unsigned_abs())
        } else {
            (current + amount as usize).min(len - 1)
        };
        self.connections_table_state.select(Some(new_idx));
    }

    pub fn scroll_connections_up(&mut self) {
        self.scroll_connections_by(-1);
    }

    pub fn scroll_connections_down(&mut self) {
        self.scroll_connections_by(1);
    }

    pub fn scroll_connections_page_up(&mut self) {
        self.scroll_connections_by(-10);
    }

    pub fn scroll_connections_page_down(&mut self) {
        self.scroll_connections_by(10);
    }

    // =====================================================
    // New Stats Update Methods
    // =====================================================

    /// Update DHT statistics.
    pub fn update_dht_stats(&mut self, stats: DhtStats) {
        self.dht_stats = stats;
    }

    /// Update EigenTrust statistics.
    pub fn update_eigentrust_stats(&mut self, stats: EigenTrustStats) {
        self.eigentrust_stats = stats;
    }

    /// Update adaptive networking statistics.
    pub fn update_adaptive_stats(&mut self, stats: AdaptiveStats) {
        self.adaptive_stats = stats;
    }

    /// Update placement diversity statistics.
    pub fn update_placement_stats(&mut self, stats: PlacementStats) {
        self.placement_stats = stats;
    }

    /// Update health monitoring statistics.
    pub fn update_health_stats(&mut self, stats: HealthStats) {
        self.health_stats = stats;
    }

    /// Update MCP client state.
    pub fn update_mcp_state(&mut self, state: McpState) {
        self.mcp_state = state;
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InputEvent {
    Quit,
    ToggleAutoConnect,
    Refresh,
    ResetConnectivityTest,
    ScrollUp,
    ScrollDown,
    PageUp,
    PageDown,
    NextTab,
    PrevTab,
    // Existing tabs [1-4]
    TabOverview,
    TabGossipHealth,
    TabConnectivityMatrix,
    TabProtocolLog,
    // New tabs [5-9, 0]
    TabDht,
    TabEigenTrust,
    TabAdaptive,
    TabPlacement,
    TabHealth,
    TabMcp,
    ToggleProofHelp,
    Unknown,
}

impl InputEvent {
    pub fn from_key(key: crossterm::event::KeyCode) -> Self {
        use crossterm::event::KeyCode;

        match key {
            KeyCode::Char('q') | KeyCode::Char('Q') => Self::Quit,
            KeyCode::Char('a') | KeyCode::Char('A') => Self::ToggleAutoConnect,
            KeyCode::Char('r') | KeyCode::Char('R') => Self::Refresh,
            KeyCode::Char('t') | KeyCode::Char('T') => Self::ResetConnectivityTest,
            KeyCode::Up | KeyCode::Char('k') => Self::ScrollUp,
            KeyCode::Down | KeyCode::Char('j') => Self::ScrollDown,
            KeyCode::PageUp => Self::PageUp,
            KeyCode::PageDown => Self::PageDown,
            // Tab navigation
            KeyCode::Tab => Self::NextTab,
            KeyCode::BackTab => Self::PrevTab,
            // Existing tabs [1-4]
            KeyCode::Char('1') => Self::TabOverview,
            KeyCode::Char('2') | KeyCode::Char('g') | KeyCode::Char('G') => Self::TabGossipHealth,
            KeyCode::Char('3') | KeyCode::Char('c') | KeyCode::Char('C') => {
                Self::TabConnectivityMatrix
            }
            KeyCode::Char('4') | KeyCode::Char('l') | KeyCode::Char('L') => Self::TabProtocolLog,
            // New tabs [5-9, 0]
            KeyCode::Char('5') | KeyCode::Char('d') | KeyCode::Char('D') => Self::TabDht,
            KeyCode::Char('6') | KeyCode::Char('e') | KeyCode::Char('E') => Self::TabEigenTrust,
            KeyCode::Char('7') => Self::TabAdaptive,
            KeyCode::Char('8') => Self::TabPlacement,
            KeyCode::Char('9') | KeyCode::Char('h') | KeyCode::Char('H') => Self::TabHealth,
            KeyCode::Char('0') | KeyCode::Char('m') | KeyCode::Char('M') => Self::TabMcp,
            KeyCode::Char('p') | KeyCode::Char('P') => Self::ToggleProofHelp,
            KeyCode::Esc => Self::Quit,
            _ => Self::Unknown,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::registry::ConnectionMethod;
    use crate::tui::types::HealthStatus;

    #[test]
    fn test_app_creation() {
        let app = App::new();
        assert_eq!(app.state, AppState::Running);
        assert!(app.connected_peers.is_empty());
        assert!(app.auto_connecting);
    }

    #[test]
    fn test_connection_stats() {
        let mut app = App::new();

        // Track unique peers (success_rate uses unique peer tracking)
        app.stats.unique_peers_attempted.insert("peer1".to_string());
        app.connection_attempted();
        app.connection_succeeded(ConnectionMethod::Direct);
        app.stats.unique_peers_connected.insert("peer1".to_string());

        app.stats.unique_peers_attempted.insert("peer2".to_string());
        app.connection_attempted();
        app.connection_failed();

        assert_eq!(app.stats.connection_attempts, 2);
        assert_eq!(app.stats.connection_successes, 1);
        assert_eq!(app.stats.connection_failures, 1);
        assert_eq!(app.stats.direct_connections, 1);
        // success_rate is based on unique peers: 1 connected / 2 attempted = 50%
        assert!((app.stats.success_rate() - 50.0).abs() < 0.001);
    }

    #[test]
    fn test_peer_management() {
        let mut app = App::new();

        let peer = ConnectedPeer::new("test_peer_id_12345", ConnectionMethod::HolePunched);
        app.update_peer(peer);

        assert_eq!(app.connected_count(), 1);
        assert!(app.connected_peers.contains_key("test_peer_id_12345"));

        app.remove_peer("test_peer_id_12345");
        assert_eq!(app.connected_count(), 0);
    }

    #[test]
    fn test_input_events() {
        use crossterm::event::KeyCode;

        assert_eq!(InputEvent::from_key(KeyCode::Char('q')), InputEvent::Quit);
        assert_eq!(InputEvent::from_key(KeyCode::Char('Q')), InputEvent::Quit);
        assert_eq!(InputEvent::from_key(KeyCode::Esc), InputEvent::Quit);
        assert_eq!(
            InputEvent::from_key(KeyCode::Char('a')),
            InputEvent::ToggleAutoConnect
        );
        assert_eq!(
            InputEvent::from_key(KeyCode::Char('x')),
            InputEvent::Unknown
        );
    }

    #[test]
    fn test_new_stats_updates() {
        let mut app = App::new();

        // Test DHT stats update
        let dht_stats = DhtStats {
            total_routing_peers: 150,
            ..Default::default()
        };
        app.update_dht_stats(dht_stats);
        assert_eq!(app.dht_stats.total_routing_peers, 150);

        // Test EigenTrust stats update
        let eigen_stats = EigenTrustStats {
            local_trust_score: 0.95,
            ..Default::default()
        };
        app.update_eigentrust_stats(eigen_stats);
        assert!((app.eigentrust_stats.local_trust_score - 0.95).abs() < 0.001);

        // Test Adaptive stats update
        let adaptive_stats = AdaptiveStats::default();
        app.update_adaptive_stats(adaptive_stats);
        assert!(app.adaptive_stats.thompson_sampling.arms.is_empty());

        // Test Placement stats update
        let placement_stats = PlacementStats {
            geographic_diversity: 0.85,
            ..Default::default()
        };
        app.update_placement_stats(placement_stats);
        assert!((app.placement_stats.geographic_diversity - 0.85).abs() < 0.001);

        // Test Health stats update
        let health_stats = HealthStats {
            overall_score: 0.99,
            status: HealthStatus::Healthy,
            ..Default::default()
        };
        app.update_health_stats(health_stats);
        assert!((app.health_stats.overall_score - 0.99).abs() < 0.001);
        assert_eq!(app.health_stats.status, HealthStatus::Healthy);

        // Test MCP state update
        let mcp_state = McpState {
            endpoint: Some("http://test:8080".to_string()),
            ..Default::default()
        };
        app.update_mcp_state(mcp_state);
        assert_eq!(app.mcp_state.endpoint, Some("http://test:8080".to_string()));
    }

    #[test]
    fn test_tab_navigation_new_tabs() {
        let mut app = App::new();

        // Start at Overview
        assert_eq!(app.active_tab, Tab::Overview);

        // Navigate through all tabs
        app.next_tab(); // GossipHealth
        app.next_tab(); // ConnectivityMatrix
        app.next_tab(); // ProtocolLog
        app.next_tab(); // Dht
        assert_eq!(app.active_tab, Tab::Dht);

        app.next_tab(); // EigenTrust
        assert_eq!(app.active_tab, Tab::EigenTrust);

        app.next_tab(); // Adaptive
        assert_eq!(app.active_tab, Tab::Adaptive);

        app.next_tab(); // Placement
        assert_eq!(app.active_tab, Tab::Placement);

        app.next_tab(); // Health
        assert_eq!(app.active_tab, Tab::Health);

        app.next_tab(); // Mcp
        assert_eq!(app.active_tab, Tab::Mcp);

        app.next_tab(); // Back to Overview
        assert_eq!(app.active_tab, Tab::Overview);

        // Test prev_tab for new tabs
        app.prev_tab(); // Mcp
        assert_eq!(app.active_tab, Tab::Mcp);

        app.prev_tab(); // Health
        assert_eq!(app.active_tab, Tab::Health);
    }

    #[test]
    fn test_new_tab_input_events() {
        use crossterm::event::KeyCode;

        assert_eq!(InputEvent::from_key(KeyCode::Char('5')), InputEvent::TabDht);
        assert_eq!(InputEvent::from_key(KeyCode::Char('d')), InputEvent::TabDht);
        assert_eq!(
            InputEvent::from_key(KeyCode::Char('6')),
            InputEvent::TabEigenTrust
        );
        assert_eq!(
            InputEvent::from_key(KeyCode::Char('7')),
            InputEvent::TabAdaptive
        );
        assert_eq!(
            InputEvent::from_key(KeyCode::Char('8')),
            InputEvent::TabPlacement
        );
        assert_eq!(
            InputEvent::from_key(KeyCode::Char('9')),
            InputEvent::TabHealth
        );
        assert_eq!(InputEvent::from_key(KeyCode::Char('0')), InputEvent::TabMcp);
    }
}
