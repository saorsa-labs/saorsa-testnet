//! TUI type definitions for the network test interface.
//!
//! This module defines the data structures used by the terminal UI
//! to display network state and peer connections.

use crate::registry::{ConnectionDirection, ConnectionMethod, ConnectivityMatrix, NatType};
use std::collections::HashSet;
use std::net::SocketAddr;
use std::time::{Duration, Instant};

/// Format a duration in seconds as a short human-readable string (e.g., "30s", "5m", "2h").
pub fn format_elapsed_short(secs: u64) -> String {
    if secs < 60 {
        format!("{}s", secs)
    } else if secs < 3600 {
        format!("{}m", secs / 60)
    } else {
        format!("{}h", secs / 3600)
    }
}

/// Historical connection record for tracking peers we've connected to.
#[derive(Debug, Clone)]
pub struct ConnectionHistoryEntry {
    /// Short peer ID (first 8 chars)
    pub short_id: String,
    /// Full peer ID
    pub full_id: String,
    /// Location (country flag + code)
    pub location: String,
    /// Last connection method observed
    pub method: Option<ConnectionMethod>,
    /// Last connection direction observed
    pub direction: Option<ConnectionDirection>,
    /// Current status
    pub status: ConnectionStatus,
    /// Outbound connection summary (us -> them)
    pub outbound: DirectionalMethodStats,
    /// Inbound connection summary (them -> us)
    pub inbound: DirectionalMethodStats,
    /// When first connected
    pub first_connected: Instant,
    /// When last seen
    pub last_seen: Instant,
    /// Total packets exchanged
    pub total_packets: u64,
    /// Best RTT ever recorded
    pub best_rtt: Option<Duration>,
    /// Number of times connected
    pub connection_count: u32,
    /// Whether NAT traversal was verified
    pub nat_verified: bool,
    /// Peer's NAT type (for connectivity matrix display)
    pub nat_type: NatType,
}

/// Outcome for a connection method attempt.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum MethodOutcome {
    /// Not attempted
    #[default]
    Unknown,
    /// Attempt succeeded
    Success,
    /// Attempt failed
    Failed,
}

impl MethodOutcome {
    /// Compact status symbol.
    pub fn symbol(&self) -> &'static str {
        match self {
            Self::Unknown => "·",
            Self::Success => "✓",
            Self::Failed => "×",
        }
    }
}

/// Per-direction connection outcomes with IPv4/IPv6 granularity.
///
/// Each connection method (direct, NAT traversal, relay) is tracked separately
/// for IPv4 and IPv6, giving 6 distinct path outcomes.
#[derive(Debug, Clone, Default)]
pub struct DirectionalMethodStats {
    pub last_method: Option<ConnectionMethod>,
    pub attempts: u32,
    pub successes: u32,
    pub failures: u32,
    /// Direct connection over IPv4
    pub direct_ipv4: MethodOutcome,
    /// Direct connection over IPv6
    pub direct_ipv6: MethodOutcome,
    /// NAT traversal (hole punching) over IPv4
    pub nat_ipv4: MethodOutcome,
    /// NAT traversal (hole punching) over IPv6
    pub nat_ipv6: MethodOutcome,
    /// Relayed connection over IPv4
    pub relay_ipv4: MethodOutcome,
    /// Relayed connection over IPv6
    pub relay_ipv6: MethodOutcome,
}

impl DirectionalMethodStats {
    /// Record an attempt outcome with IP version.
    ///
    /// This is the primary method for recording connection outcomes.
    /// It routes to the appropriate IPv4/IPv6 field based on the connection method.
    pub fn record(&mut self, method: ConnectionMethod, success: bool) {
        // Default to IPv4 if IP version not specified
        self.record_with_ip_version(method, success, false);
    }

    /// Record an attempt outcome with explicit IP version.
    ///
    /// Routes the outcome to the correct field based on (method, is_ipv6) combination.
    pub fn record_with_ip_version(
        &mut self,
        method: ConnectionMethod,
        success: bool,
        is_ipv6: bool,
    ) {
        self.attempts += 1;
        if success {
            self.successes += 1;
        } else {
            self.failures += 1;
        }
        self.last_method = Some(method);

        let outcome = if success {
            MethodOutcome::Success
        } else {
            MethodOutcome::Failed
        };

        // Route to the correct field based on method and IP version
        let slot = match (method, is_ipv6) {
            (ConnectionMethod::Direct, false) => &mut self.direct_ipv4,
            (ConnectionMethod::Direct, true) => &mut self.direct_ipv6,
            (ConnectionMethod::HolePunched, false) => &mut self.nat_ipv4,
            (ConnectionMethod::HolePunched, true) => &mut self.nat_ipv6,
            (ConnectionMethod::Relayed, false) => &mut self.relay_ipv4,
            (ConnectionMethod::Relayed, true) => &mut self.relay_ipv6,
        };

        Self::update_outcome(slot, outcome);
    }

    fn update_outcome(slot: &mut MethodOutcome, outcome: MethodOutcome) {
        match outcome {
            MethodOutcome::Success => {
                *slot = MethodOutcome::Success;
            }
            MethodOutcome::Failed if *slot == MethodOutcome::Unknown => {
                *slot = MethodOutcome::Failed;
            }
            _ => {}
        }
    }

    /// Compact summary showing all 6 path outcomes.
    pub fn summary_compact(&self) -> String {
        format!(
            "D4{}D6{}N4{}N6{}R4{}R6{}",
            self.direct_ipv4.symbol(),
            self.direct_ipv6.symbol(),
            self.nat_ipv4.symbol(),
            self.nat_ipv6.symbol(),
            self.relay_ipv4.symbol(),
            self.relay_ipv6.symbol()
        )
    }

    /// Check if any IPv4 path was tested.
    pub fn has_ipv4(&self) -> bool {
        self.direct_ipv4 != MethodOutcome::Unknown
            || self.nat_ipv4 != MethodOutcome::Unknown
            || self.relay_ipv4 != MethodOutcome::Unknown
    }

    /// Check if any IPv6 path was tested.
    pub fn has_ipv6(&self) -> bool {
        self.direct_ipv6 != MethodOutcome::Unknown
            || self.nat_ipv6 != MethodOutcome::Unknown
            || self.relay_ipv6 != MethodOutcome::Unknown
    }

    /// Get the best direct outcome (prefers success, then failure, then unknown).
    pub fn direct_best(&self) -> MethodOutcome {
        Self::best_of(self.direct_ipv4, self.direct_ipv6)
    }

    /// Get the best NAT outcome.
    pub fn nat_best(&self) -> MethodOutcome {
        Self::best_of(self.nat_ipv4, self.nat_ipv6)
    }

    /// Get the best relay outcome.
    pub fn relay_best(&self) -> MethodOutcome {
        Self::best_of(self.relay_ipv4, self.relay_ipv6)
    }

    fn best_of(a: MethodOutcome, b: MethodOutcome) -> MethodOutcome {
        match (a, b) {
            (MethodOutcome::Success, _) | (_, MethodOutcome::Success) => MethodOutcome::Success,
            (MethodOutcome::Failed, _) | (_, MethodOutcome::Failed) => MethodOutcome::Failed,
            _ => MethodOutcome::Unknown,
        }
    }
}

/// Connection status for history entries.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConnectionStatus {
    /// Currently connected
    Connected,
    /// Disconnected (with time since disconnection)
    Disconnected,
    /// Connection failed
    Failed,
    /// Seen coordination frames but not connected
    Coordinating,
}

impl ConnectionStatus {
    /// Get status emoji
    pub fn emoji(&self) -> &'static str {
        match self {
            Self::Connected => "●",
            Self::Disconnected => "○",
            Self::Failed => "✗",
            Self::Coordinating => "◌",
        }
    }

    /// Get status color name
    pub fn color_name(&self) -> &'static str {
        match self {
            Self::Connected => "green",
            Self::Disconnected => "gray",
            Self::Failed => "red",
            Self::Coordinating => "yellow",
        }
    }
}

impl ConnectionHistoryEntry {
    /// Create a new history entry for a peer (no successful connection yet).
    pub fn new(peer_id: &str) -> Self {
        let short_id = if peer_id.len() > 8 {
            peer_id[..8].to_string()
        } else {
            peer_id.to_string()
        };
        let now = Instant::now();
        Self {
            short_id,
            full_id: peer_id.to_string(),
            location: "---".to_string(), // Unknown location until geo lookup completes
            method: None,
            direction: None,
            status: ConnectionStatus::Coordinating,
            outbound: DirectionalMethodStats::default(),
            inbound: DirectionalMethodStats::default(),
            first_connected: now,
            last_seen: now,
            total_packets: 0,
            best_rtt: None,
            connection_count: 0,
            nat_verified: false,
            nat_type: NatType::Unknown,
        }
    }

    /// Create a new history entry from a connected peer.
    pub fn from_connected_peer(peer: &super::ConnectedPeer) -> Self {
        let mut entry = Self {
            short_id: peer.short_id.clone(),
            full_id: peer.full_id.clone(),
            location: peer.location.clone(),
            method: Some(peer.method),
            direction: Some(peer.direction),
            status: ConnectionStatus::Connected,
            outbound: DirectionalMethodStats::default(),
            inbound: DirectionalMethodStats::default(),
            first_connected: peer.connected_at,
            last_seen: Instant::now(),
            total_packets: peer.packets_sent + peer.packets_received,
            best_rtt: peer.rtt,
            connection_count: 1,
            nat_verified: peer.is_nat_verified(),
            nat_type: peer.nat_type,
        };

        entry.record_attempt(peer.direction, peer.method, true);
        entry.status = ConnectionStatus::Connected;
        entry
    }

    /// Update from a connected peer (when reconnecting).
    pub fn update_from_peer(&mut self, peer: &super::ConnectedPeer) {
        self.status = ConnectionStatus::Connected;
        self.last_seen = Instant::now();
        self.method = Some(peer.method);
        self.direction = Some(peer.direction);
        self.record_attempt(peer.direction, peer.method, true);
        self.total_packets += peer.packets_sent + peer.packets_received;
        self.connection_count += 1;
        if let Some(rtt) = peer.rtt {
            match self.best_rtt {
                Some(best) if rtt < best => self.best_rtt = Some(rtt),
                None => self.best_rtt = Some(rtt),
                _ => {}
            }
        }
        if peer.is_nat_verified() {
            self.nat_verified = true;
        }
        // Update NAT type from peer (may have been discovered since last update)
        self.nat_type = peer.nat_type;
    }

    pub fn record_attempt(
        &mut self,
        direction: ConnectionDirection,
        method: ConnectionMethod,
        success: bool,
    ) {
        self.record_attempt_with_ip(direction, method, success, false);
    }

    pub fn record_attempt_with_ip(
        &mut self,
        direction: ConnectionDirection,
        method: ConnectionMethod,
        success: bool,
        is_ipv6: bool,
    ) {
        self.last_seen = Instant::now();
        self.method = Some(method);
        self.direction = Some(direction);

        match direction {
            ConnectionDirection::Outbound => {
                self.outbound
                    .record_with_ip_version(method, success, is_ipv6);
            }
            ConnectionDirection::Inbound => {
                self.inbound
                    .record_with_ip_version(method, success, is_ipv6);
            }
        }

        if self.status != ConnectionStatus::Connected {
            if success {
                if self.status == ConnectionStatus::Failed
                    || self.status == ConnectionStatus::Coordinating
                {
                    self.status = ConnectionStatus::Disconnected;
                }
            } else if self.status == ConnectionStatus::Coordinating {
                self.status = ConnectionStatus::Failed;
            }
        }
    }

    /// Mark as disconnected.
    pub fn mark_disconnected(&mut self) {
        self.status = ConnectionStatus::Disconnected;
        self.last_seen = Instant::now();
    }

    /// Get time since last seen as a formatted string.
    pub fn time_since_seen(&self) -> String {
        format_elapsed_short(self.last_seen.elapsed().as_secs())
    }

    /// Get best RTT as a formatted string.
    pub fn rtt_string(&self) -> String {
        match self.best_rtt {
            Some(rtt) => format!("{}ms", rtt.as_millis()),
            None => "-".to_string(),
        }
    }

    /// Get IPv4/IPv6 indicator based on connection history.
    pub fn ip_version_indicator(&self) -> &'static str {
        let has_v4 = self.outbound.has_ipv4() || self.inbound.has_ipv4();
        let has_v6 = self.outbound.has_ipv6() || self.inbound.has_ipv6();
        match (has_v4, has_v6) {
            (true, true) => "4+6",
            (true, false) => "v4",
            (false, true) => "v6",
            (false, false) => "-",
        }
    }
}

/// Connection quality indicator (5 levels).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConnectionQuality {
    /// Excellent connection (< 50ms RTT)
    Excellent,
    /// Good connection (50-100ms RTT)
    Good,
    /// Fair connection (100-200ms RTT)
    Fair,
    /// Poor connection (200-500ms RTT)
    Poor,
    /// Very poor connection (> 500ms RTT)
    VeryPoor,
}

impl ConnectionQuality {
    /// Create quality indicator from RTT measurement.
    pub fn from_rtt(rtt: Duration) -> Self {
        let ms = rtt.as_millis();
        if ms < 50 {
            Self::Excellent
        } else if ms < 100 {
            Self::Good
        } else if ms < 200 {
            Self::Fair
        } else if ms < 500 {
            Self::Poor
        } else {
            Self::VeryPoor
        }
    }

    /// Get the quality bar representation (5 dots).
    pub fn as_bar(&self) -> &'static str {
        match self {
            Self::Excellent => "●●●●●",
            Self::Good => "●●●●○",
            Self::Fair => "●●●○○",
            Self::Poor => "●●○○○",
            Self::VeryPoor => "●○○○○",
        }
    }
}

/// Traffic direction indicator.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TrafficDirection {
    Sending,
    Receiving,
    Idle,
}

/// NAT test state for a peer - tracks the connect-back verification flow.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum PeerNatTestState {
    /// Not yet tested
    #[default]
    Pending,
    /// Attempting outbound connection
    ConnectingOutbound,
    /// Outbound succeeded, waiting for peer to connect back
    WaitingForConnectBack { seconds_remaining: u32 },
    /// Peer successfully connected back - NAT traversal verified
    Verified,
    /// Timed out waiting for connect-back
    TimedOut,
    /// Retrying to verify peer is still online
    Retrying,
    /// Peer is unreachable (may have gone offline)
    Unreachable,
}

impl PeerNatTestState {
    pub fn status_symbol(&self) -> &'static str {
        match self {
            Self::Pending => "○",
            Self::ConnectingOutbound => "→",
            Self::WaitingForConnectBack { .. } => "⏳",
            Self::Verified => "✓",
            Self::TimedOut => "⏱",
            Self::Retrying => "↻",
            Self::Unreachable => "✗",
        }
    }

    pub fn status_text(&self) -> String {
        match self {
            Self::Pending => "pending".to_string(),
            Self::ConnectingOutbound => "connecting...".to_string(),
            Self::WaitingForConnectBack { seconds_remaining } => {
                format!("wait {}s", seconds_remaining)
            }
            Self::Verified => "verified".to_string(),
            Self::TimedOut => "timeout".to_string(),
            Self::Retrying => "retrying...".to_string(),
            Self::Unreachable => "offline".to_string(),
        }
    }
}

/// Information about a connected peer for display.
#[derive(Debug, Clone)]
pub struct ConnectedPeer {
    /// Short peer ID (first 8 chars)
    pub short_id: String,
    /// Full peer ID (QUIC PeerId hex - canonical identifier)
    pub full_id: String,
    /// Gossip peer ID (BLAKE3 hash) - for gossip transport correlation
    pub gossip_peer_id: Option<String>,
    /// Country code with flag emoji
    pub location: String,
    /// Connection method used
    pub method: ConnectionMethod,
    /// Connection direction (who initiated the current/most recent connection)
    pub direction: ConnectionDirection,
    /// Current RTT measurement
    pub rtt: Option<Duration>,
    /// Connection quality
    pub quality: ConnectionQuality,
    /// TX traffic indicator
    pub tx_active: bool,
    /// RX traffic indicator
    pub rx_active: bool,
    /// Packets sent
    pub packets_sent: u64,
    /// Packets received
    pub packets_received: u64,
    /// Connection established time
    pub connected_at: Instant,
    /// Remote addresses
    pub addresses: Vec<SocketAddr>,
    /// Connectivity matrix showing all tested paths
    pub connectivity: ConnectivityMatrix,
    /// Outbound connection verified (we successfully connected to them)
    pub outbound_verified: bool,
    /// Inbound connection verified (they successfully connected to us - proves NAT traversal!)
    pub inbound_verified: bool,
    /// When we last tested NAT traversal with this peer
    pub last_nat_test_time: Option<Instant>,
    /// Last time we had a connection to/from this peer (for 30-second rule)
    pub last_connection_time: Instant,
    /// Current NAT traversal phase
    pub nat_phase: NatTraversalPhase,
    /// Peer ID coordinating NAT traversal (if applicable)
    pub coordinator_id: Option<String>,
    /// Protocol traffic (TX)
    pub protocol_tx: bool,
    /// Protocol traffic (RX)
    pub protocol_rx: bool,
    /// Test data traffic (TX)
    pub data_tx: bool,
    /// Test data traffic (RX)
    pub data_rx: bool,
    /// NAT test state for connect-back verification
    pub nat_test_state: PeerNatTestState,
    /// Peer's detected NAT type (from registry)
    pub nat_type: NatType,
}

impl ConnectedPeer {
    /// Create a new connected peer.
    pub fn new(peer_id: &str, method: ConnectionMethod) -> Self {
        Self::with_direction(peer_id, method, ConnectionDirection::Outbound)
    }

    /// Create a new connected peer with explicit direction.
    pub fn with_direction(
        peer_id: &str,
        method: ConnectionMethod,
        direction: ConnectionDirection,
    ) -> Self {
        let short_id = if peer_id.len() > 8 {
            peer_id[..8].to_string()
        } else {
            peer_id.to_string()
        };

        let now = Instant::now();
        let (outbound_verified, inbound_verified) = match direction {
            ConnectionDirection::Outbound => (true, false),
            ConnectionDirection::Inbound => (false, true),
        };

        Self {
            short_id,
            full_id: peer_id.to_string(),
            gossip_peer_id: None,
            location: "??".to_string(),
            method,
            direction,
            rtt: None,
            quality: ConnectionQuality::Fair,
            tx_active: false,
            rx_active: false,
            packets_sent: 0,
            packets_received: 0,
            connected_at: now,
            addresses: Vec::new(),
            connectivity: ConnectivityMatrix::default(),
            outbound_verified,
            inbound_verified,
            last_nat_test_time: None,
            last_connection_time: now,
            nat_phase: NatTraversalPhase::Discovering,
            coordinator_id: None,
            protocol_tx: false,
            protocol_rx: false,
            data_tx: false,
            data_rx: false,
            nat_test_state: PeerNatTestState::Pending,
            nat_type: NatType::Unknown,
        }
    }

    /// Set the gossip peer ID for correlation.
    pub fn set_gossip_peer_id(&mut self, gossip_id: &str) {
        self.gossip_peer_id = Some(gossip_id.to_string());
    }

    /// Mark that outbound connection was verified (we connected to them).
    pub fn mark_outbound_verified(&mut self) {
        self.outbound_verified = true;
        self.last_connection_time = Instant::now();
    }

    /// Mark that inbound connection was verified (they connected to us).
    pub fn mark_inbound_verified(&mut self) {
        self.inbound_verified = true;
        self.last_connection_time = Instant::now();
    }

    /// Check if NAT traversal is fully verified (both directions tested).
    pub fn is_nat_verified(&self) -> bool {
        self.outbound_verified && self.inbound_verified
    }

    /// Check if this peer is eligible for a NAT callback test (30-second rule).
    /// Returns true if we haven't had a connection for 30+ seconds.
    pub fn needs_nat_callback_test(&self) -> bool {
        self.last_connection_time.elapsed() > Duration::from_secs(30)
    }

    /// Update RTT measurement.
    pub fn update_rtt(&mut self, rtt: Duration) {
        self.rtt = Some(rtt);
        self.quality = ConnectionQuality::from_rtt(rtt);
    }

    /// Get formatted RTT string.
    pub fn rtt_string(&self) -> String {
        match self.rtt {
            Some(rtt) => format!("{}ms", rtt.as_millis()),
            None => "---".to_string(),
        }
    }

    /// Get TX/RX indicator string.
    pub fn traffic_indicator(&self) -> String {
        let tx = if self.tx_active { ">>" } else { "  " };
        let rx = if self.rx_active { "<<" } else { "  " };
        format!("[{}] [{}]", tx, rx)
    }

    /// Get connectivity summary string.
    pub fn connectivity_summary(&self) -> String {
        self.connectivity.summary()
    }

    /// Update connectivity matrix from test results.
    pub fn update_connectivity(&mut self, matrix: ConnectivityMatrix) {
        self.connectivity = matrix;
    }
}

/// Local node information for display.
#[derive(Debug, Clone)]
pub struct LocalNodeInfo {
    /// Peer ID (full)
    pub peer_id: String,
    /// Short peer ID (first 8 chars)
    pub short_id: String,
    /// Detected NAT type
    pub nat_type: NatType,
    /// Local IPv4 address
    pub local_ipv4: Option<SocketAddr>,
    /// External IPv4 address (discovered)
    pub external_ipv4: Option<SocketAddr>,
    /// Local IPv6 address
    pub local_ipv6: Option<SocketAddr>,
    /// External IPv6 address (discovered)
    pub external_ipv6: Option<SocketAddr>,
    /// Connection words (external IPv4:port encoded as 4 memorable words)
    /// Share these out-of-band for first-time peer connections
    pub connection_words: Option<String>,
    /// Whether registered with central registry
    pub registered: bool,
    /// Time until registration expires
    pub registration_expires_in: Option<Duration>,
    /// Last heartbeat sent
    pub last_heartbeat: Option<Instant>,
}

impl Default for LocalNodeInfo {
    fn default() -> Self {
        Self {
            peer_id: String::new(),
            short_id: String::new(),
            nat_type: NatType::Unknown,
            local_ipv4: None,
            external_ipv4: None,
            local_ipv6: None,
            external_ipv6: None,
            connection_words: None,
            registered: false,
            registration_expires_in: None,
            last_heartbeat: None,
        }
    }
}

impl LocalNodeInfo {
    /// Set the peer ID.
    pub fn set_peer_id(&mut self, peer_id: &str) {
        self.peer_id = peer_id.to_string();
        self.short_id = if peer_id.len() > 8 {
            peer_id[..8].to_string()
        } else {
            peer_id.to_string()
        };
    }

    /// Get registration status string.
    pub fn registration_status(&self) -> &'static str {
        if self.registered { "✓" } else { "✗" }
    }

    /// Get last heartbeat string.
    pub fn heartbeat_status(&self) -> String {
        match self.last_heartbeat {
            Some(instant) => {
                let elapsed = instant.elapsed().as_secs();
                format!("{}s ago", elapsed)
            }
            None => "Never".to_string(),
        }
    }
}

/// Network-wide statistics for display.
#[derive(Debug, Clone, Default)]
pub struct NetworkStatistics {
    /// Total connection attempts
    pub connection_attempts: u64,
    /// Successful connections
    pub connection_successes: u64,
    /// Failed connections
    pub connection_failures: u64,
    /// Direct connections
    pub direct_connections: u64,
    /// Hole-punched connections
    pub hole_punched_connections: u64,
    /// Relayed connections
    pub relayed_connections: u64,
    /// Inbound connections (they connected to us - proves NAT traversal works!)
    pub inbound_connections: u64,
    /// Outbound connections (we connected to them)
    pub outbound_connections: u64,
    /// IPv4 connections
    pub ipv4_connections: u64,
    /// IPv6 connections
    pub ipv6_connections: u64,
    /// Test packets sent
    pub packets_sent: u64,
    /// Test packets received
    pub packets_received: u64,
    /// Total bytes sent
    pub bytes_sent: u64,
    /// Total bytes received
    pub bytes_received: u64,
    /// Node start time
    pub started_at: Option<Instant>,
    /// Total registered nodes in network
    pub total_registered_nodes: usize,
    /// Peers discovered via gossip network
    pub gossip_peers_discovered: u64,
    /// Relays discovered via gossip network
    pub gossip_relays_discovered: u64,
    /// SWIM liveness: peers marked alive
    pub swim_alive: usize,
    /// SWIM liveness: peers marked suspect
    pub swim_suspect: usize,
    /// SWIM liveness: peers marked dead
    pub swim_dead: usize,
    /// HyParView: active view size
    pub hyparview_active: usize,
    /// HyParView: passive view size
    pub hyparview_passive: usize,
    /// Unique peers we attempted to connect to
    pub unique_peers_attempted: HashSet<String>,
    /// Unique peers we successfully connected to
    pub unique_peers_connected: HashSet<String>,
}

impl NetworkStatistics {
    /// Get connection success rate as percentage (unique peers connected / unique peers attempted).
    pub fn success_rate(&self) -> f64 {
        let attempted = self.unique_peers_attempted.len();
        if attempted == 0 {
            0.0
        } else {
            (self.unique_peers_connected.len() as f64 / attempted as f64) * 100.0
        }
    }

    /// Get unique peer counts as (connected, attempted).
    pub fn unique_peer_counts(&self) -> (usize, usize) {
        (
            self.unique_peers_connected.len(),
            self.unique_peers_attempted.len(),
        )
    }

    /// Get uptime string.
    pub fn uptime(&self) -> String {
        match self.started_at {
            Some(started) => {
                let elapsed = started.elapsed();
                let hours = elapsed.as_secs() / 3600;
                let minutes = (elapsed.as_secs() % 3600) / 60;
                let seconds = elapsed.as_secs() % 60;
                format!("{:02}:{:02}:{:02}", hours, minutes, seconds)
            }
            None => "00:00:00".to_string(),
        }
    }

    /// Get formatted bytes sent.
    pub fn bytes_sent_formatted(&self) -> String {
        format_bytes(self.bytes_sent)
    }

    /// Get formatted bytes received.
    pub fn bytes_received_formatted(&self) -> String {
        format_bytes(self.bytes_received)
    }
}

/// Format bytes into human-readable string.
fn format_bytes(bytes: u64) -> String {
    const KB: u64 = 1024;
    const MB: u64 = KB * 1024;
    const GB: u64 = MB * 1024;

    if bytes >= GB {
        format!("{:.1} GB", bytes as f64 / GB as f64)
    } else if bytes >= MB {
        format!("{:.1} MB", bytes as f64 / MB as f64)
    } else if bytes >= KB {
        format!("{:.1} KB", bytes as f64 / KB as f64)
    } else {
        format!("{} B", bytes)
    }
}

/// Country code to flag emoji mapping.
pub fn country_flag(country_code: &str) -> &'static str {
    match country_code.to_uppercase().as_str() {
        "US" => "🇺🇸",
        "GB" | "UK" => "🇬🇧",
        "DE" => "🇩🇪",
        "FR" => "🇫🇷",
        "JP" => "🇯🇵",
        "CN" => "🇨🇳",
        "KR" => "🇰🇷",
        "AU" => "🇦🇺",
        "CA" => "🇨🇦",
        "BR" => "🇧🇷",
        "IN" => "🇮🇳",
        "RU" => "🇷🇺",
        "IT" => "🇮🇹",
        "ES" => "🇪🇸",
        "NL" => "🇳🇱",
        "SE" => "🇸🇪",
        "NO" => "🇳🇴",
        "FI" => "🇫🇮",
        "DK" => "🇩🇰",
        "PL" => "🇵🇱",
        "CH" => "🇨🇭",
        "AT" => "🇦🇹",
        "BE" => "🇧🇪",
        "IE" => "🇮🇪",
        "SG" => "🇸🇬",
        "HK" => "🇭🇰",
        "NZ" => "🇳🇿",
        "ZA" => "🇿🇦",
        "MX" => "🇲🇽",
        "AR" => "🇦🇷",
        _ => "🌍",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_connection_quality_from_rtt() {
        assert_eq!(
            ConnectionQuality::from_rtt(Duration::from_millis(10)),
            ConnectionQuality::Excellent
        );
        assert_eq!(
            ConnectionQuality::from_rtt(Duration::from_millis(75)),
            ConnectionQuality::Good
        );
        assert_eq!(
            ConnectionQuality::from_rtt(Duration::from_millis(150)),
            ConnectionQuality::Fair
        );
        assert_eq!(
            ConnectionQuality::from_rtt(Duration::from_millis(300)),
            ConnectionQuality::Poor
        );
        assert_eq!(
            ConnectionQuality::from_rtt(Duration::from_millis(1000)),
            ConnectionQuality::VeryPoor
        );
    }

    #[test]
    fn test_format_bytes() {
        assert_eq!(format_bytes(500), "500 B");
        assert_eq!(format_bytes(1500), "1.5 KB");
        assert_eq!(format_bytes(1_500_000), "1.4 MB");
        assert_eq!(format_bytes(1_500_000_000), "1.4 GB");
    }

    #[test]
    fn test_country_flag() {
        assert_eq!(country_flag("US"), "🇺🇸");
        assert_eq!(country_flag("GB"), "🇬🇧");
        assert_eq!(country_flag("XX"), "🌍");
    }

    #[test]
    fn test_nat_type_stats() {
        let mut stats = NatTypeStats::default();

        // Test empty stats
        assert_eq!(stats.success_rate(), 0.0);
        assert_eq!(stats.attempts, 0);

        // Test recording attempts
        stats.record_attempt();
        stats.record_success();
        stats.record_failure();
        assert_eq!(stats.attempts, 3);
        assert_eq!(stats.successes, 1);
        assert_eq!(stats.failures, 1);

        // Test success rate calculation
        let expected_rate = (1.0 / 3.0) * 100.0;
        assert!((stats.success_rate() - expected_rate).abs() < 0.001);
    }

    #[test]
    fn test_nat_type_analytics() {
        let mut analytics = NatTypeAnalytics::default();

        // Test recording different NAT types
        use crate::registry::NatType;
        analytics.record_success(NatType::FullCone);
        analytics.record_failure(NatType::Symmetric);
        analytics.record_attempt(NatType::Cgnat);

        assert_eq!(analytics.full_cone.successes, 1);
        assert_eq!(analytics.full_cone.attempts, 1);
        assert_eq!(analytics.symmetric.failures, 1);
        assert_eq!(analytics.symmetric.attempts, 1);
        assert_eq!(analytics.cgnat.attempts, 1);

        // Test overall success rate
        assert_eq!(analytics.total_attempts(), 3);
        assert_eq!(analytics.total_successes(), 1);
        assert!((analytics.overall_success_rate() - (1.0 / 3.0 * 100.0)).abs() < 0.001);
    }

    #[test]
    fn test_geographic_distribution() {
        let mut geo = GeographicDistribution::new();

        // Test adding peers from different regions
        geo.add_peer("US".to_string());
        geo.add_peer("GB".to_string());
        geo.add_peer("JP".to_string());
        geo.add_peer("US".to_string()); // Duplicate

        assert_eq!(geo.total_peers, 4);
        assert_eq!(geo.regions.get("US"), Some(&2));
        assert_eq!(geo.regions.get("GB"), Some(&1));
        assert_eq!(geo.regions.get("JP"), Some(&1));

        // Test percentages
        assert!((geo.region_percentage("US") - 50.0).abs() < 0.001);
        assert!((geo.region_percentage("GB") - 25.0).abs() < 0.001);
        assert!((geo.region_percentage("JP") - 25.0).abs() < 0.001);

        // Test diversity
        assert!(geo.is_diverse());
        assert!(geo.diversity_score() > 0.5);

        // Test top regions
        let top = geo.top_regions(3);
        assert_eq!(top.len(), 3);
        assert!(top.iter().any(|(region, _)| *region == "US"));
    }

    #[test]
    fn test_cache_health() {
        use std::time::Duration;

        let health = CacheHealth {
            total_peers: 100,
            valid_peers: 80,
            public_peers: 25,
            average_quality: 0.75,
            cache_age: Duration::from_secs(3600),
            last_updated: None,
            cache_hits: 150,
            cache_misses: 50,
            fresh_peers: 60,
            stale_peers: 10,
            private_peers: 75,
            public_quality: 0.8,
            private_quality: 0.7,
        };

        assert_eq!(health.validity_percentage(), 80.0);
        assert_eq!(health.public_percentage(), 25.0);

        // Test with zero peers
        let empty_health = CacheHealth {
            total_peers: 0,
            valid_peers: 0,
            public_peers: 0,
            average_quality: 0.0,
            cache_age: Duration::from_secs(0),
            last_updated: None,
            cache_hits: 0,
            cache_misses: 0,
            fresh_peers: 0,
            stale_peers: 0,
            private_peers: 0,
            public_quality: 0.0,
            private_quality: 0.0,
        };

        assert_eq!(empty_health.validity_percentage(), 0.0);
        assert_eq!(empty_health.public_percentage(), 0.0);
    }
}

/// NAT Traversal phase for detailed connection status
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NatTraversalPhase {
    /// Discovering external addresses via OBSERVED_ADDRESS frames
    Discovering,
    /// Coordinating with a peer to schedule hole punching
    Coordinating,
    /// Active hole punching in progress
    Punching,
    /// Direct connection established
    Connected,
    /// Fallback to relay connection
    Relayed,
}

impl NatTraversalPhase {
    /// Get emoji representation for UI
    pub fn emoji(&self) -> &'static str {
        match self {
            Self::Discovering => "🔍",
            Self::Coordinating => "🤝",
            Self::Punching => "💥",
            Self::Connected => "✅",
            Self::Relayed => "🌐",
        }
    }

    /// Get color name for UI display
    pub fn color_name(&self) -> &'static str {
        match self {
            Self::Discovering => "blue",
            Self::Coordinating => "yellow",
            Self::Punching => "orange",
            Self::Connected => "green",
            Self::Relayed => "red",
        }
    }
}

/// Protocol frame information for real-time tracking
#[derive(Debug, Clone)]
pub struct ProtocolFrame {
    /// Peer ID this frame is associated with
    pub peer_id: String,
    /// Frame type (ADD_ADDRESS, PUNCH_ME_NOW, etc.)
    pub frame_type: String,
    /// Direction: SENT or RECEIVED
    pub direction: FrameDirection,
    /// Timestamp when frame was processed
    pub timestamp: Instant,
    /// Additional context (optional)
    pub context: Option<String>,
}

/// Direction of protocol frame
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FrameDirection {
    Sent,
    Received,
}

impl FrameDirection {
    /// Get arrow representation
    pub fn arrow(&self) -> &'static str {
        match self {
            Self::Sent => "→",
            Self::Received => "←",
        }
    }

    /// Get color name for UI
    pub fn color_name(&self) -> &'static str {
        match self {
            Self::Sent => "cyan",      // Outgoing frames
            Self::Received => "green", // Incoming frames
        }
    }
}

/// Enhanced traffic type classification
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TrafficType {
    /// Protocol overhead (gossip, heartbeats, NAT frames)
    Protocol,
    /// Test data packets (5KB test packets)
    TestData,
    /// Relay traffic (MASQUE protocol)
    Relay,
}

impl TrafficType {
    /// Get emoji for UI display
    pub fn emoji(&self) -> &'static str {
        match self {
            Self::Protocol => "🔄",
            Self::TestData => "📦",
            Self::Relay => "🌐",
        }
    }

    /// Get color name for UI display
    pub fn color_name(&self) -> &'static str {
        match self {
            Self::Protocol => "blue",
            Self::TestData => "green",
            Self::Relay => "magenta",
        }
    }
}

#[derive(Debug, Clone)]
pub struct CacheHealth {
    pub total_peers: usize,
    pub valid_peers: usize,
    pub public_peers: usize,
    pub average_quality: f32,
    pub cache_age: std::time::Duration,
    pub last_updated: Option<Instant>,
    pub cache_hits: u64,
    pub cache_misses: u64,
    pub fresh_peers: usize,
    pub stale_peers: usize,
    pub private_peers: usize,
    pub public_quality: f32,
    pub private_quality: f32,
}

impl CacheHealth {
    pub fn validity_percentage(&self) -> f32 {
        if self.total_peers == 0 {
            0.0
        } else {
            (self.valid_peers as f32 / self.total_peers as f32) * 100.0
        }
    }

    pub fn public_percentage(&self) -> f32 {
        if self.total_peers == 0 {
            0.0
        } else {
            (self.public_peers as f32 / self.total_peers as f32) * 100.0
        }
    }

    pub fn cache_hit_rate(&self) -> f32 {
        let total_requests = self.cache_hits + self.cache_misses;
        if total_requests == 0 {
            0.0
        } else {
            (self.cache_hits as f32 / total_requests as f32) * 100.0
        }
    }

    pub fn freshness_percentage(&self) -> f32 {
        if self.total_peers == 0 {
            0.0
        } else {
            (self.fresh_peers as f32 / self.total_peers as f32) * 100.0
        }
    }

    pub fn staleness_percentage(&self) -> f32 {
        if self.total_peers == 0 {
            0.0
        } else {
            (self.stale_peers as f32 / self.total_peers as f32) * 100.0
        }
    }

    pub fn private_percentage(&self) -> f32 {
        if self.total_peers == 0 {
            0.0
        } else {
            (self.private_peers as f32 / self.total_peers as f32) * 100.0
        }
    }

    pub fn health_score(&self) -> f32 {
        let validity_weight = 0.3;
        let freshness_weight = 0.25;
        let quality_weight = 0.25;
        let hit_rate_weight = 0.2;

        (self.validity_percentage() / 100.0 * validity_weight)
            + (self.freshness_percentage() / 100.0 * freshness_weight)
            + (self.average_quality * quality_weight)
            + (self.cache_hit_rate() / 100.0 * hit_rate_weight)
    }
}

#[derive(Debug, Clone, Default)]
pub struct NatTypeAnalytics {
    pub full_cone: NatTypeStats,
    pub restricted_cone: NatTypeStats,
    pub port_restricted: NatTypeStats,
    pub symmetric: NatTypeStats,
    pub cgnat: NatTypeStats,
    pub rtt_by_nat_type: std::collections::HashMap<String, f64>,
    pub connection_methods_by_nat_type: std::collections::HashMap<String, ConnectionMethodStats>,
}

#[derive(Debug, Clone, Default)]
pub struct ConnectionMethodStats {
    pub direct: u64,
    pub hole_punched: u64,
    pub relayed: u64,
}

#[derive(Debug, Clone, Default)]
pub struct NatTypeStats {
    pub attempts: u64,
    pub successes: u64,
    pub failures: u64,
    pub direct_connections: u64,
    pub hole_punched_connections: u64,
    pub relayed_connections: u64,
    pub total_rtt: f64,
    pub rtt_samples: u64,
}

impl NatTypeStats {
    pub fn success_rate(&self) -> f64 {
        if self.attempts == 0 {
            0.0
        } else {
            (self.successes as f64 / self.attempts as f64) * 100.0
        }
    }

    pub fn average_rtt(&self) -> f64 {
        if self.rtt_samples == 0 {
            0.0
        } else {
            self.total_rtt / self.rtt_samples as f64
        }
    }

    pub fn record_attempt(&mut self) {
        self.attempts += 1;
    }

    pub fn record_success(&mut self) {
        self.successes += 1;
        self.attempts = self.attempts.saturating_add(1);
    }

    pub fn record_failure(&mut self) {
        self.failures += 1;
        self.attempts = self.attempts.saturating_add(1);
    }

    pub fn record_direct_connection(&mut self) {
        self.direct_connections += 1;
    }

    pub fn record_hole_punched_connection(&mut self) {
        self.hole_punched_connections += 1;
    }

    pub fn record_relayed_connection(&mut self) {
        self.relayed_connections += 1;
    }

    pub fn record_rtt(&mut self, rtt: f64) {
        self.total_rtt += rtt;
        self.rtt_samples += 1;
    }
}

impl NatTypeAnalytics {
    pub fn get_stats(&self, nat_type: NatType) -> &NatTypeStats {
        match nat_type {
            NatType::FullCone => &self.full_cone,
            NatType::AddressRestricted => &self.restricted_cone,
            NatType::PortRestricted => &self.port_restricted,
            NatType::Symmetric => &self.symmetric,
            NatType::Cgnat => &self.cgnat,
            NatType::Unknown => &self.restricted_cone,
            _ => &self.restricted_cone,
        }
    }

    pub fn get_stats_mut(&mut self, nat_type: NatType) -> &mut NatTypeStats {
        match nat_type {
            NatType::FullCone => &mut self.full_cone,
            NatType::AddressRestricted => &mut self.restricted_cone,
            NatType::PortRestricted => &mut self.port_restricted,
            NatType::Symmetric => &mut self.symmetric,
            NatType::Cgnat => &mut self.cgnat,
            NatType::Unknown => &mut self.restricted_cone,
            _ => &mut self.restricted_cone,
        }
    }

    pub fn record_attempt(&mut self, nat_type: NatType) {
        self.get_stats_mut(nat_type).record_attempt();
    }

    pub fn record_success(&mut self, nat_type: NatType) {
        self.get_stats_mut(nat_type).record_success();
    }

    pub fn record_failure(&mut self, nat_type: NatType) {
        self.get_stats_mut(nat_type).record_failure();
    }

    pub fn record_connection_method(&mut self, nat_type: NatType, method: ConnectionMethod) {
        let key = format!("{:?}", nat_type);
        let stats = self.connection_methods_by_nat_type.entry(key).or_default();

        match method {
            ConnectionMethod::Direct => stats.direct += 1,
            ConnectionMethod::HolePunched => stats.hole_punched += 1,
            ConnectionMethod::Relayed => stats.relayed += 1,
        }

        let nat_stats = self.get_stats_mut(nat_type);
        match method {
            ConnectionMethod::Direct => nat_stats.record_direct_connection(),
            ConnectionMethod::HolePunched => nat_stats.record_hole_punched_connection(),
            ConnectionMethod::Relayed => nat_stats.record_relayed_connection(),
        }
    }

    pub fn record_rtt(&mut self, nat_type: NatType, rtt: f64) {
        let key = format!("{:?}", nat_type);
        self.rtt_by_nat_type.insert(key, rtt);
        self.get_stats_mut(nat_type).record_rtt(rtt);
    }

    pub fn total_attempts(&self) -> u64 {
        self.full_cone.attempts
            + self.restricted_cone.attempts
            + self.port_restricted.attempts
            + self.symmetric.attempts
            + self.cgnat.attempts
    }

    pub fn total_successes(&self) -> u64 {
        self.full_cone.successes
            + self.restricted_cone.successes
            + self.port_restricted.successes
            + self.symmetric.successes
            + self.cgnat.successes
    }

    pub fn overall_success_rate(&self) -> f64 {
        let total = self.total_attempts();
        if total == 0 {
            0.0
        } else {
            (self.total_successes() as f64 / total as f64) * 100.0
        }
    }

    pub fn success_rate_color(success_rate: f64) -> &'static str {
        if success_rate >= 90.0 {
            "green"
        } else if success_rate >= 70.0 {
            "yellow"
        } else if success_rate >= 50.0 {
            "orange"
        } else {
            "red"
        }
    }

    pub fn get_average_rtt(&self, nat_type: NatType) -> f64 {
        self.get_stats(nat_type).average_rtt()
    }

    pub fn get_connection_method_breakdown(
        &self,
        nat_type: NatType,
    ) -> Option<&ConnectionMethodStats> {
        let key = format!("{:?}", nat_type);
        self.connection_methods_by_nat_type.get(&key)
    }
}

pub use crate::node::{
    ConnectivityMethod as TestConnectivityMethod, ConnectivityTestPhase, PeerConnectivityResult,
};

/// Proof verification status for display in TUI.
/// This tracks the last run of ProofOrchestrator and shows pass/fail status.
#[derive(Debug, Clone, Default)]
pub struct ProofStatus {
    /// Overall connectivity test passed
    pub connectivity_pass: bool,
    /// Number of reachable nodes out of total
    pub connectivity_nodes: (usize, usize),

    /// Gossip protocols test passed
    pub gossip_pass: bool,
    /// HyParView active view size
    pub hyparview_active: usize,
    /// SWIM alive count
    pub swim_alive: usize,
    /// Plumtree tree valid
    pub tree_valid: bool,

    /// CRDT convergence test passed
    pub crdt_pass: bool,
    /// Number of nodes that converged
    pub crdt_nodes: usize,
    /// First 4 chars of state hash (if available)
    pub crdt_hash_short: Option<String>,

    /// NAT traversal test passed
    pub nat_pass: bool,
    /// Direct connections count
    pub nat_direct: usize,
    /// Hole-punched connections count
    pub nat_punched: usize,
    /// Relayed connections count
    pub nat_relayed: usize,

    /// When the last proof was generated
    pub last_proof_time: Option<Instant>,
    /// Session ID from last proof run
    pub session_id: Option<String>,
    /// Whether a proof run is currently in progress
    pub running: bool,
}

impl ProofStatus {
    /// Create a new ProofStatus with all tests as not-run.
    pub fn new() -> Self {
        Self::default()
    }

    /// Check if all tests passed.
    pub fn all_passed(&self) -> bool {
        self.connectivity_pass && self.gossip_pass && self.crdt_pass && self.nat_pass
    }

    /// Get seconds since last proof, or None if never run.
    pub fn seconds_since_last(&self) -> Option<u64> {
        self.last_proof_time.map(|t| t.elapsed().as_secs())
    }

    /// Get formatted time since last proof.
    pub fn time_since_last_str(&self) -> String {
        match self.seconds_since_last() {
            Some(secs) => format!("{} ago", format_elapsed_short(secs)),
            None => "never".to_string(),
        }
    }
}

#[allow(dead_code)]
#[derive(Debug, Clone)]
pub struct ConnectivityTestResults {
    /// Current phase of the connectivity test
    pub phase: ConnectivityTestPhase,
    /// When the test started
    pub started_at: Option<Instant>,
    /// Per-peer connectivity results (peer_id -> results)
    pub peer_results: std::collections::HashMap<String, PeerConnectivityResult>,
    /// Peers we're expecting to receive inbound connections from
    pub expected_inbound_peers: Vec<String>,
    /// When the inbound phase started (for countdown)
    pub inbound_phase_started: Option<Instant>,
    /// Total inbound connections received
    pub total_inbound: u32,
    /// Total outbound connections tested
    pub total_outbound: u32,
    /// Successful inbound connections
    pub successful_inbound: u32,
    /// Successful outbound connections
    pub successful_outbound: u32,
}

impl Default for ConnectivityTestResults {
    fn default() -> Self {
        Self::new()
    }
}

#[allow(dead_code)]
impl ConnectivityTestResults {
    pub fn new() -> Self {
        Self {
            phase: ConnectivityTestPhase::Registering,
            started_at: None,
            peer_results: std::collections::HashMap::new(),
            expected_inbound_peers: Vec::new(),
            inbound_phase_started: None,
            total_inbound: 0,
            total_outbound: 0,
            successful_inbound: 0,
            successful_outbound: 0,
        }
    }

    /// Start the connectivity test.
    pub fn start(&mut self) {
        self.started_at = Some(Instant::now());
        self.phase = ConnectivityTestPhase::Registering;
    }

    /// Transition to waiting for inbound connections.
    pub fn start_inbound_phase(&mut self) {
        self.phase = ConnectivityTestPhase::WaitingForInbound;
        self.inbound_phase_started = Some(Instant::now());
    }

    /// Get seconds remaining in the countdown (30 second wait).
    pub fn countdown_seconds(&self) -> u32 {
        const WAIT_SECONDS: u64 = 30;
        match self.inbound_phase_started {
            Some(started) => {
                let elapsed = started.elapsed().as_secs();
                if elapsed >= WAIT_SECONDS {
                    0
                } else {
                    (WAIT_SECONDS - elapsed) as u32
                }
            }
            None => WAIT_SECONDS as u32,
        }
    }

    /// Check if countdown is complete.
    pub fn countdown_complete(&self) -> bool {
        self.countdown_seconds() == 0
    }

    /// Record an inbound connection attempt.
    pub fn record_inbound(
        &mut self,
        peer_id: &str,
        method: TestConnectivityMethod,
        success: bool,
        rtt_ms: Option<u32>,
        error: Option<String>,
    ) {
        self.total_inbound += 1;
        if success {
            self.successful_inbound += 1;
        }

        let result = self
            .peer_results
            .entry(peer_id.to_string())
            .or_insert_with(|| {
                let short_id = if peer_id.len() > 8 {
                    peer_id[..8].to_string()
                } else {
                    peer_id.to_string()
                };
                PeerConnectivityResult::new(short_id, peer_id.to_string())
            });

        result.record_inbound(method, success, rtt_ms, error);
    }

    /// Record an outbound connection attempt.
    pub fn record_outbound(
        &mut self,
        peer_id: &str,
        method: TestConnectivityMethod,
        success: bool,
        rtt_ms: Option<u32>,
        error: Option<String>,
    ) {
        self.total_outbound += 1;
        if success {
            self.successful_outbound += 1;
        }

        let result = self
            .peer_results
            .entry(peer_id.to_string())
            .or_insert_with(|| {
                let short_id = if peer_id.len() > 8 {
                    peer_id[..8].to_string()
                } else {
                    peer_id.to_string()
                };
                PeerConnectivityResult::new(short_id, peer_id.to_string())
            });

        result.record_outbound(method, success, rtt_ms, error);
    }

    /// Get overall inbound success rate.
    pub fn inbound_success_rate(&self) -> f32 {
        if self.total_inbound == 0 {
            0.0
        } else {
            (self.successful_inbound as f32 / self.total_inbound as f32) * 100.0
        }
    }

    /// Get overall outbound success rate.
    pub fn outbound_success_rate(&self) -> f32 {
        if self.total_outbound == 0 {
            0.0
        } else {
            (self.successful_outbound as f32 / self.total_outbound as f32) * 100.0
        }
    }

    /// Get sorted peer results for display.
    pub fn sorted_results(&self) -> Vec<&PeerConnectivityResult> {
        let mut results: Vec<_> = self.peer_results.values().collect();
        // Sort by peer_id for consistent display
        results.sort_by(|a, b| a.peer_id.cmp(&b.peer_id));
        results
    }

    /// Reset the test for a re-run.
    pub fn reset(&mut self) {
        *self = Self::new();
    }
}

#[derive(Debug, Clone, Default)]
pub struct GeographicDistribution {
    /// Peers by region/country code
    pub regions: std::collections::HashMap<String, usize>,
    /// Total peers across all regions
    pub total_peers: usize,
}

// ============================================================================
// NEW TYPES FOR EXPANDED TUI SCREENS
// ============================================================================

// These types are defined for the new TUI screens and will be used once
// the screen implementations are complete.

/// DHT statistics for the DHT tab [5].
/// Displays routing table, operations, latency, and stored records.
#[allow(dead_code)]
#[derive(Debug, Clone, Default)]
pub struct DhtStats {
    /// Number of peers in routing table by k-bucket distance
    pub k_buckets: Vec<usize>,
    /// Total peers in routing table
    pub total_routing_peers: usize,
    /// DHT operation counts
    pub operations: DhtOperationStats,
    /// Operation latency statistics
    pub latency: LatencyStats,
    /// Number of records stored locally
    pub stored_records: usize,
    /// Records by type (for breakdown display)
    pub records_by_type: std::collections::HashMap<String, usize>,
    /// Replication factor (target copies)
    pub replication_factor: usize,
    /// Last refresh time
    pub last_refresh: Option<Instant>,
    /// Node's distance from various keys (for k-bucket visualization)
    pub distance_samples: Vec<u8>,
}

/// DHT operation counters.
#[allow(dead_code)]
#[derive(Debug, Clone, Default)]
pub struct DhtOperationStats {
    /// GET operations
    pub gets: u64,
    /// GET successes
    pub get_successes: u64,
    /// PUT operations
    pub puts: u64,
    /// PUT successes
    pub put_successes: u64,
    /// DELETE operations
    pub deletes: u64,
    /// DELETE successes
    pub delete_successes: u64,
    /// Routing queries
    pub routing_queries: u64,
    /// Routing query successes
    pub routing_successes: u64,
}

#[allow(dead_code)]
impl DhtOperationStats {
    /// Get overall success rate.
    pub fn success_rate(&self) -> f64 {
        let total = self.gets + self.puts + self.deletes;
        let successes = self.get_successes + self.put_successes + self.delete_successes;
        if total == 0 {
            100.0
        } else {
            (successes as f64 / total as f64) * 100.0
        }
    }
}

/// Latency statistics with percentiles.
#[allow(dead_code)]
#[derive(Debug, Clone, Default)]
pub struct LatencyStats {
    /// Minimum latency in ms
    pub min_ms: u32,
    /// Maximum latency in ms
    pub max_ms: u32,
    /// Average latency in ms
    pub avg_ms: f64,
    /// P50 latency in ms
    pub p50_ms: u32,
    /// P95 latency in ms
    pub p95_ms: u32,
    /// P99 latency in ms
    pub p99_ms: u32,
    /// Sample count
    pub samples: u64,
    /// Recent latency history (for sparkline)
    pub history: Vec<u32>,
}

#[allow(dead_code)]
impl LatencyStats {
    /// Add a latency sample.
    pub fn record(&mut self, latency_ms: u32) {
        self.samples += 1;
        if latency_ms < self.min_ms || self.samples == 1 {
            self.min_ms = latency_ms;
        }
        if latency_ms > self.max_ms {
            self.max_ms = latency_ms;
        }
        // Rolling average
        self.avg_ms =
            ((self.avg_ms * (self.samples - 1) as f64) + latency_ms as f64) / self.samples as f64;
        // Keep last 60 samples for sparkline
        self.history.push(latency_ms);
        if self.history.len() > 60 {
            self.history.remove(0);
        }
    }
}

/// EigenTrust statistics for the Trust tab [6].
/// Displays trust scores, node statistics, and trust evolution.
#[allow(dead_code)]
#[derive(Debug, Clone, Default)]
pub struct EigenTrustStats {
    /// Local node's global trust score (0.0 - 1.0)
    pub local_trust_score: f64,
    /// Trust scores for known peers (peer_id -> score)
    pub peer_trust_scores: Vec<TrustEntry>,
    /// Number of iterations to convergence
    pub convergence_iterations: u32,
    /// Whether trust has converged
    pub converged: bool,
    /// Pre-trusted peers count
    pub pre_trusted_count: usize,
    /// Suspicious peers (below threshold)
    pub suspicious_count: usize,
    /// Trust threshold for marking suspicious
    pub trust_threshold: f64,
    /// Trust evolution history (for chart)
    pub trust_history: Vec<f64>,
    /// Last update time
    pub last_update: Option<Instant>,
}

/// Individual peer trust entry.
#[allow(dead_code)]
#[derive(Debug, Clone)]
pub struct TrustEntry {
    /// Short peer ID
    pub short_id: String,
    /// Full peer ID
    pub peer_id: String,
    /// Trust score (0.0 - 1.0)
    pub score: f64,
    /// Whether this peer is pre-trusted
    pub pre_trusted: bool,
    /// Whether below suspicion threshold
    pub suspicious: bool,
    /// Transaction count with this peer
    pub transactions: u64,
}

#[allow(dead_code)]
impl TrustEntry {
    /// Get trust level as a category.
    pub fn trust_level(&self) -> TrustLevel {
        if self.score >= 0.8 {
            TrustLevel::High
        } else if self.score >= 0.5 {
            TrustLevel::Medium
        } else if self.score >= 0.2 {
            TrustLevel::Low
        } else {
            TrustLevel::Untrusted
        }
    }
}

/// Trust level categories.
#[allow(dead_code)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TrustLevel {
    High,
    Medium,
    Low,
    Untrusted,
}

#[allow(dead_code)]
impl TrustLevel {
    /// Get color for display.
    pub fn color_name(&self) -> &'static str {
        match self {
            Self::High => "green",
            Self::Medium => "yellow",
            Self::Low => "orange",
            Self::Untrusted => "red",
        }
    }

    /// Get symbol for display.
    pub fn symbol(&self) -> &'static str {
        match self {
            Self::High => "●●●",
            Self::Medium => "●●○",
            Self::Low => "●○○",
            Self::Untrusted => "○○○",
        }
    }
}

/// Adaptive network statistics for the Adaptive tab [7].
/// Displays Thompson Sampling, Q-Learning, and churn predictions.
#[allow(dead_code)]
#[derive(Debug, Clone, Default)]
pub struct AdaptiveStats {
    /// Thompson Sampling arm statistics
    pub thompson_sampling: ThompsonSamplingStats,
    /// Q-Learning statistics
    pub q_learning: QLearningStats,
    /// Churn prediction statistics
    pub churn_prediction: ChurnPredictionStats,
    /// Overall adaptive strategy performance
    pub strategy_performance: StrategyPerformance,
}

/// Thompson Sampling arm statistics.
#[allow(dead_code)]
#[derive(Debug, Clone, Default)]
pub struct ThompsonSamplingStats {
    /// Per-arm statistics
    pub arms: Vec<ArmStats>,
    /// Total pulls across all arms
    pub total_pulls: u64,
    /// Current best arm index
    pub best_arm: Option<usize>,
    /// Exploration vs exploitation ratio
    pub exploration_ratio: f64,
}

/// Individual arm statistics.
#[allow(dead_code)]
#[derive(Debug, Clone)]
pub struct ArmStats {
    /// Arm name/identifier
    pub name: String,
    /// Success count (alpha - 1)
    pub successes: u64,
    /// Failure count (beta - 1)
    pub failures: u64,
    /// Estimated reward probability
    pub estimated_prob: f64,
    /// Number of times pulled
    pub pulls: u64,
}

#[allow(dead_code)]
impl ArmStats {
    /// Calculate success rate.
    pub fn success_rate(&self) -> f64 {
        if self.pulls == 0 {
            0.0
        } else {
            (self.successes as f64 / self.pulls as f64) * 100.0
        }
    }
}

/// Q-Learning statistics.
#[allow(dead_code)]
#[derive(Debug, Clone, Default)]
pub struct QLearningStats {
    /// Number of states
    pub state_count: usize,
    /// Number of actions
    pub action_count: usize,
    /// Cache hit rate
    pub cache_hit_rate: f64,
    /// Cache miss rate
    pub cache_miss_rate: f64,
    /// Learning rate (alpha)
    pub learning_rate: f64,
    /// Discount factor (gamma)
    pub discount_factor: f64,
    /// Exploration rate (epsilon)
    pub epsilon: f64,
    /// Total episodes
    pub episodes: u64,
    /// Average reward
    pub avg_reward: f64,
}

/// Churn prediction statistics.
#[allow(dead_code)]
#[derive(Debug, Clone, Default)]
pub struct ChurnPredictionStats {
    /// Peers predicted to churn soon
    pub at_risk_peers: Vec<ChurnRiskEntry>,
    /// Prediction accuracy (historical)
    pub accuracy: f64,
    /// Total predictions made
    pub predictions: u64,
    /// Correct predictions
    pub correct: u64,
    /// False positives
    pub false_positives: u64,
    /// False negatives
    pub false_negatives: u64,
}

/// Churn risk entry for a peer.
#[allow(dead_code)]
#[derive(Debug, Clone)]
pub struct ChurnRiskEntry {
    /// Short peer ID
    pub short_id: String,
    /// Churn probability (0.0 - 1.0)
    pub risk: f64,
    /// Time until predicted churn
    pub time_to_churn: Option<Duration>,
    /// Risk level
    pub level: ChurnRiskLevel,
}

/// Churn risk levels.
#[allow(dead_code)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChurnRiskLevel {
    Low,
    Medium,
    High,
    Critical,
}

#[allow(dead_code)]
impl ChurnRiskLevel {
    /// Get color for display.
    pub fn color_name(&self) -> &'static str {
        match self {
            Self::Low => "green",
            Self::Medium => "yellow",
            Self::High => "orange",
            Self::Critical => "red",
        }
    }
}

/// Strategy performance comparison.
#[allow(dead_code)]
#[derive(Debug, Clone, Default)]
pub struct StrategyPerformance {
    /// Thompson Sampling performance score
    pub thompson_score: f64,
    /// Q-Learning performance score
    pub qlearning_score: f64,
    /// Random baseline score
    pub random_score: f64,
    /// Currently active strategy
    pub active_strategy: String,
}

/// Placement statistics for the Placement tab [8].
/// Displays diversity scores and regional distribution.
#[allow(dead_code)]
#[derive(Debug, Clone, Default)]
pub struct PlacementStats {
    /// Geographic diversity score (0.0 - 1.0)
    pub geographic_diversity: f64,
    /// Rack diversity score (0.0 - 1.0)
    pub rack_diversity: f64,
    /// Network diversity score (0.0 - 1.0)
    pub network_diversity: f64,
    /// Overall diversity score
    pub overall_diversity: f64,
    /// Regional distribution
    pub regions: Vec<RegionStats>,
    /// Placement success rate
    pub placement_success_rate: f64,
    /// Total placement operations
    pub total_placements: u64,
    /// Replicas placed successfully
    pub successful_placements: u64,
    /// Placement failures
    pub failed_placements: u64,
    /// Placement retries
    pub placement_retries: u64,
    /// Current replication targets
    pub replication_targets: Vec<String>,
    /// Target replica count
    pub target_replicas: u32,
    /// Minimum regions for placement
    pub min_regions: u32,
    /// Average replica count achieved
    pub avg_replica_count: f64,
    /// Count of under-replicated data
    pub under_replicated_count: u64,
}

/// Regional distribution entry.
#[allow(dead_code)]
#[derive(Debug, Clone)]
pub struct RegionStats {
    /// Region/country code
    pub code: String,
    /// Region name
    pub name: String,
    /// Country flag emoji
    pub flag: String,
    /// Number of nodes in region
    pub node_count: usize,
    /// Percentage of total nodes (data distribution)
    pub data_percentage: f64,
    /// Percentage of total nodes
    pub percentage: f64,
    /// Average latency to region
    pub avg_latency_ms: f64,
    /// Whether the region is healthy
    pub healthy: bool,
}

/// Health statistics for the Health tab [9].
/// Displays component health, anomalies, and alerts.
#[allow(dead_code)]
#[derive(Debug, Clone, Default)]
pub struct HealthStats {
    /// Overall health score (0.0 - 1.0)
    pub overall_score: f64,
    /// Overall health status
    pub status: HealthStatus,
    /// Component health states
    pub components: Vec<ComponentHealth>,
    /// Active alerts
    pub alerts: Vec<HealthAlert>,
    /// Recent anomalies
    pub anomalies: Vec<AnomalyEntry>,
    /// Resource usage
    pub resources: ResourceUsage,
    /// Last health check time
    pub last_check: Option<Instant>,
    /// System uptime in seconds
    pub uptime_secs: u64,
    /// Seconds since last health check
    pub last_check_secs_ago: u64,
}

/// Overall health status.
#[allow(dead_code)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum HealthStatus {
    #[default]
    Unknown,
    Healthy,
    Degraded,
    Unhealthy,
    Critical,
}

#[allow(dead_code)]
impl HealthStatus {
    /// Get color for display.
    pub fn color_name(&self) -> &'static str {
        match self {
            Self::Unknown => "gray",
            Self::Healthy => "green",
            Self::Degraded => "yellow",
            Self::Unhealthy => "orange",
            Self::Critical => "red",
        }
    }

    /// Get symbol for display.
    pub fn symbol(&self) -> &'static str {
        match self {
            Self::Unknown => "?",
            Self::Healthy => "●",
            Self::Degraded => "◐",
            Self::Unhealthy => "◑",
            Self::Critical => "✗",
        }
    }
}

/// Individual component health.
#[allow(dead_code)]
#[derive(Debug, Clone)]
pub struct ComponentHealth {
    /// Component name
    pub name: String,
    /// Component status
    pub status: HealthStatus,
    /// Whether the component is healthy
    pub healthy: bool,
    /// Component uptime in seconds
    pub uptime_secs: u64,
    /// Last error message (if any)
    pub last_error: Option<String>,
    /// Status message
    pub message: Option<String>,
    /// Last check time
    pub last_check: Option<Instant>,
}

/// Health alert entry.
#[allow(dead_code)]
#[derive(Debug, Clone)]
pub struct HealthAlert {
    /// Alert severity
    pub severity: AlertSeverity,
    /// Alert message
    pub message: String,
    /// Component that triggered the alert
    pub component: String,
    /// When the alert was raised
    pub timestamp: Instant,
    /// Seconds since the alert was raised
    pub timestamp_secs_ago: u64,
    /// Whether the alert has been acknowledged
    pub acknowledged: bool,
}

/// Alert severity levels.
#[allow(dead_code)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AlertSeverity {
    Info,
    Warning,
    Error,
    Critical,
}

#[allow(dead_code)]
impl AlertSeverity {
    /// Get color for display.
    pub fn color_name(&self) -> &'static str {
        match self {
            Self::Info => "blue",
            Self::Warning => "yellow",
            Self::Error => "orange",
            Self::Critical => "red",
        }
    }

    /// Get symbol for display.
    pub fn symbol(&self) -> &'static str {
        match self {
            Self::Info => "ℹ",
            Self::Warning => "⚠",
            Self::Error => "✗",
            Self::Critical => "☠",
        }
    }
}

/// Anomaly detection entry.
#[allow(dead_code)]
#[derive(Debug, Clone)]
pub struct AnomalyEntry {
    /// Anomaly type
    pub anomaly_type: String,
    /// Description
    pub description: String,
    /// Anomaly score (0.0 - 1.0, higher = more anomalous)
    pub score: f64,
    /// Deviation in standard deviations (sigma)
    pub deviation: f64,
    /// When detected
    pub timestamp: Instant,
    /// Related peer (if applicable)
    pub peer_id: Option<String>,
}

/// Resource usage statistics.
#[allow(dead_code)]
#[derive(Debug, Clone, Default)]
pub struct ResourceUsage {
    /// CPU usage percentage (0.0 - 100.0)
    pub cpu_percent: f64,
    /// Memory usage percentage (0.0 - 100.0)
    pub memory_percent: f64,
    /// Memory used in bytes
    pub memory_used_bytes: u64,
    /// Total memory in bytes
    pub memory_total_bytes: u64,
    /// Disk usage percentage (0.0 - 100.0)
    pub disk_percent: f64,
    /// Disk used in bytes
    pub disk_used_bytes: u64,
    /// Total disk in bytes
    pub disk_total_bytes: u64,
    /// Network bandwidth usage (bytes/sec)
    pub network_bytes_sec: u64,
    /// Network utilization percentage (0.0 - 100.0)
    pub network_utilization_percent: f64,
    /// Network TX bytes per second
    pub network_tx_bytes_sec: u64,
    /// Network RX bytes per second
    pub network_rx_bytes_sec: u64,
    /// Open file descriptors
    pub open_fds: u32,
    /// Active connections
    pub active_connections: u32,
}

/// MCP tool category for sub-tab organization.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum McpToolCategory {
    #[default]
    Auth,
    Entities,
    Messages,
    Files,
    Kanban,
    Network,
    Social,
}

impl McpToolCategory {
    /// All categories in display order.
    pub const ALL: [McpToolCategory; 7] = [
        McpToolCategory::Auth,
        McpToolCategory::Entities,
        McpToolCategory::Messages,
        McpToolCategory::Files,
        McpToolCategory::Kanban,
        McpToolCategory::Network,
        McpToolCategory::Social,
    ];

    /// Get display name for the category.
    pub fn display_name(&self) -> &'static str {
        match self {
            McpToolCategory::Auth => "Auth",
            McpToolCategory::Entities => "Entities",
            McpToolCategory::Messages => "Messages",
            McpToolCategory::Files => "Files",
            McpToolCategory::Kanban => "Kanban",
            McpToolCategory::Network => "Network",
            McpToolCategory::Social => "Social",
        }
    }

    /// Get short key hint for the category.
    pub fn key_hint(&self) -> &'static str {
        match self {
            McpToolCategory::Auth => "1",
            McpToolCategory::Entities => "2",
            McpToolCategory::Messages => "3",
            McpToolCategory::Files => "4",
            McpToolCategory::Kanban => "5",
            McpToolCategory::Network => "6",
            McpToolCategory::Social => "7",
        }
    }

    /// Next category (wrapping).
    pub fn next(&self) -> McpToolCategory {
        match self {
            McpToolCategory::Auth => McpToolCategory::Entities,
            McpToolCategory::Entities => McpToolCategory::Messages,
            McpToolCategory::Messages => McpToolCategory::Files,
            McpToolCategory::Files => McpToolCategory::Kanban,
            McpToolCategory::Kanban => McpToolCategory::Network,
            McpToolCategory::Network => McpToolCategory::Social,
            McpToolCategory::Social => McpToolCategory::Auth,
        }
    }

    /// Previous category (wrapping).
    pub fn prev(&self) -> McpToolCategory {
        match self {
            McpToolCategory::Auth => McpToolCategory::Social,
            McpToolCategory::Entities => McpToolCategory::Auth,
            McpToolCategory::Messages => McpToolCategory::Entities,
            McpToolCategory::Files => McpToolCategory::Messages,
            McpToolCategory::Kanban => McpToolCategory::Files,
            McpToolCategory::Network => McpToolCategory::Kanban,
            McpToolCategory::Social => McpToolCategory::Network,
        }
    }
}

/// MCP client state for the MCP tab [0].
/// Displays connection, tools, and invocation history.
#[derive(Debug, Clone, Default)]
pub struct McpState {
    /// Four-word identity (e.g., "ocean-forest-moon-star")
    pub four_words: Option<String>,
    /// Connection status
    pub connection: McpConnectionStatus,
    /// Server information
    pub server_info: Option<McpServerInfo>,
    /// Available tools
    pub tools: Vec<McpTool>,
    /// Currently selected category sub-tab
    pub selected_category: McpToolCategory,
    /// Currently selected tool index (within category)
    pub selected_tool: Option<usize>,
    /// Parameter inputs for selected tool (param_name -> value)
    pub parameter_inputs: std::collections::HashMap<String, String>,
    /// Input mode: which parameter index is being edited (None = not editing)
    pub editing_param: Option<usize>,
    /// Invocation history
    pub history: Vec<McpInvocation>,
    /// Last error message
    pub last_error: Option<String>,
    /// MCP endpoint URL
    pub endpoint: Option<String>,
    /// Contacts list
    pub contacts: Vec<ContactDisplay>,
    /// Currently selected contact index (for navigation)
    pub selected_contact: Option<usize>,
    /// Contact add mode: typing a four-word ID to add
    pub adding_contact: Option<String>,
    /// Message being composed (None = not composing)
    pub composing_message: Option<String>,
    /// Messages with currently selected contact
    pub current_messages: Vec<MessageDisplay>,
}

/// Message display information for TUI.
#[derive(Debug, Clone)]
pub struct MessageDisplay {
    /// Message ID
    pub id: String,
    /// Message text
    pub text: String,
    /// Author's display name or four-word ID
    pub author: String,
    /// Whether this is from us (true) or them (false)
    pub is_outgoing: bool,
    /// Formatted timestamp
    pub timestamp: String,
    /// Whether edited
    pub edited: bool,
}

/// MCP connection status.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum McpConnectionStatus {
    #[default]
    Disconnected,
    Connecting,
    Connected,
    Error,
}

/// MCP server information.
#[allow(dead_code)]
#[derive(Debug, Clone)]
pub struct McpServerInfo {
    /// Server name
    pub name: String,
    /// Server version
    pub version: String,
    /// Protocol version
    pub protocol_version: String,
    /// Server capabilities
    pub capabilities: Vec<String>,
}

/// MCP tool definition.
#[derive(Debug, Clone)]
pub struct McpTool {
    /// Tool name
    pub name: String,
    /// Tool description
    pub description: String,
    /// Tool category
    pub category: McpToolCategory,
    /// Input schema (JSON schema)
    pub parameters: Vec<McpParameter>,
}

/// MCP tool parameter.
#[allow(dead_code)]
#[derive(Debug, Clone)]
pub struct McpParameter {
    /// Parameter name
    pub name: String,
    /// Parameter type (string, number, boolean, etc.)
    pub param_type: String,
    /// Description (optional)
    pub description: Option<String>,
    /// Whether required
    pub required: bool,
    /// Default value (if any)
    pub default: Option<String>,
}

/// MCP tool invocation record.
#[allow(dead_code)]
#[derive(Debug, Clone)]
pub struct McpInvocation {
    /// Tool name
    pub tool_name: String,
    /// Parameters used
    pub parameters: std::collections::HashMap<String, String>,
    /// Result (success or error)
    pub result: McpInvocationResult,
    /// When invoked
    pub timestamp: Instant,
    /// Duration of invocation
    pub duration: Duration,
}

/// MCP invocation result.
#[allow(dead_code)]
#[derive(Debug, Clone)]
pub enum McpInvocationResult {
    /// Success with output
    Success(String),
    /// Error with message
    Error(String),
    /// Pending (still executing)
    Pending,
}

/// Contact information for TUI display.
#[derive(Debug, Clone)]
pub struct ContactDisplay {
    /// Contact ID
    pub id: String,
    /// Display name
    pub display_name: String,
    /// Four-word identity (if linked to network)
    pub four_words: Option<String>,
    /// Whether favourite
    pub is_favourite: bool,
    /// Online status (based on presence)
    pub status: ContactOnlineStatus,
    /// Last seen timestamp (formatted string)
    pub last_seen: Option<String>,
}

/// Contact online status.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ContactOnlineStatus {
    #[default]
    Unknown,
    Online,
    Away,
    Offline,
}

impl ContactOnlineStatus {
    /// Get status indicator dot.
    pub fn dot(&self) -> &'static str {
        match self {
            Self::Online => "●",  // Green in UI
            Self::Away => "◐",    // Yellow in UI
            Self::Offline => "○", // Gray in UI
            Self::Unknown => "?",
        }
    }

    /// Get color name for display.
    pub fn color_name(&self) -> &'static str {
        match self {
            Self::Online => "green",
            Self::Away => "yellow",
            Self::Offline => "gray",
            Self::Unknown => "gray",
        }
    }
}

#[allow(dead_code)]
impl McpInvocationResult {
    /// Get color for display.
    pub fn color_name(&self) -> &'static str {
        match self {
            Self::Success(_) => "green",
            Self::Error(_) => "red",
            Self::Pending => "yellow",
        }
    }

    /// Get symbol for display.
    pub fn symbol(&self) -> &'static str {
        match self {
            Self::Success(_) => "✓",
            Self::Error(_) => "✗",
            Self::Pending => "⏳",
        }
    }
}

impl GeographicDistribution {
    /// Create new geographic distribution
    pub fn new() -> Self {
        Self::default()
    }

    /// Add a peer from a region
    pub fn add_peer(&mut self, region: String) {
        *self.regions.entry(region).or_insert(0) += 1;
        self.total_peers += 1;
    }

    /// Get percentage for a region
    pub fn region_percentage(&self, region: &str) -> f32 {
        if self.total_peers == 0 {
            0.0
        } else {
            (self.regions.get(region).copied().unwrap_or(0) as f32 / self.total_peers as f32)
                * 100.0
        }
    }

    /// Get top N regions by peer count
    pub fn top_regions(&self, n: usize) -> Vec<(&String, &usize)> {
        let mut regions: Vec<_> = self.regions.iter().collect();
        regions.sort_by(|a, b| b.1.cmp(a.1));
        regions.into_iter().take(n).collect()
    }

    /// Check if network is geographically diverse (peers in 3+ regions)
    pub fn is_diverse(&self) -> bool {
        self.regions.len() >= 3
    }

    /// Get diversity score (0.0 - 1.0)
    pub fn diversity_score(&self) -> f32 {
        if self.total_peers == 0 {
            0.0
        } else {
            // Use Shannon entropy for diversity
            let mut entropy = 0.0;
            for count in self.regions.values() {
                let p = *count as f32 / self.total_peers as f32;
                if p > 0.0 {
                    entropy -= p * p.log2();
                }
            }
            // Normalize by maximum possible entropy
            let max_entropy = (self.regions.len() as f32).log2();
            if max_entropy > 0.0 {
                entropy / max_entropy
            } else {
                0.0
            }
        }
    }
}
