//! API response types for the web dashboard.
//!
//! These types are designed for JSON serialization to the frontend,
//! providing clean API contracts separate from internal TUI types.

use serde::{Deserialize, Serialize};

/// Overview page response containing proof status, network stats, and connected peers.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OverviewResponse {
    /// Current proof status
    pub proof_status: ProofStatusApi,
    /// Network-wide statistics
    pub network_stats: NetworkStatsApi,
    /// Currently connected peers
    pub connected_peers: Vec<ConnectedPeerApi>,
    /// Local node information
    pub local_node: LocalNodeApi,
    /// Uptime in seconds
    pub uptime_secs: u64,
}

/// Proof status for API consumption.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProofStatusApi {
    /// Overall connectivity test passed
    pub connectivity_pass: bool,
    /// Reachable nodes count
    pub connectivity_reachable: usize,
    /// Total nodes count
    pub connectivity_total: usize,

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
    /// Short state hash (first 8 chars)
    pub crdt_hash_short: Option<String>,

    /// NAT traversal test passed
    pub nat_pass: bool,
    /// Direct connections count
    pub nat_direct: usize,
    /// Hole-punched connections count
    pub nat_punched: usize,
    /// Relayed connections count
    pub nat_relayed: usize,

    /// Session ID from last proof run
    pub session_id: Option<String>,
    /// Whether a proof run is currently in progress
    pub running: bool,
    /// Milliseconds since last proof
    pub last_proof_ms: Option<u64>,
}

/// Network statistics for API consumption.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkStatsApi {
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
    /// Inbound connections
    pub inbound_connections: u64,
    /// Outbound connections
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
    /// Total registered nodes in network
    pub total_registered_nodes: usize,
    /// Peers discovered via gossip
    pub gossip_peers_discovered: u64,
    /// Relays discovered via gossip
    pub gossip_relays_discovered: u64,
}

/// Local node information for API consumption.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LocalNodeApi {
    /// Full peer ID
    pub peer_id: String,
    /// Short peer ID (first 8 chars)
    pub short_id: String,
    /// Detected NAT type
    pub nat_type: String,
    /// Local IPv4 address
    pub local_ipv4: Option<String>,
    /// External IPv4 address
    pub external_ipv4: Option<String>,
    /// Local IPv6 address
    pub local_ipv6: Option<String>,
    /// External IPv6 address
    pub external_ipv6: Option<String>,
    /// Whether registered with central registry
    pub registered: bool,
}

/// Connected peer for API consumption.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConnectedPeerApi {
    /// Short peer ID (first 8 chars)
    pub short_id: String,
    /// Full peer ID
    pub full_id: String,
    /// Location (country code)
    pub location: String,
    /// Connection method: "direct", "hole_punched", "relayed"
    pub method: String,
    /// Connection direction: "inbound", "outbound"
    pub direction: String,
    /// Current RTT in milliseconds
    pub rtt_ms: Option<u32>,
    /// Connection quality: "excellent", "good", "fair", "poor", "unknown"
    pub quality: String,
    /// Packets sent
    pub packets_sent: u64,
    /// Packets received
    pub packets_received: u64,
    /// Seconds since connection established
    pub connected_secs: u64,
}

/// Connection matrix response for connectivity display.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConnectionsResponse {
    /// Connection entries for all known peers
    pub connections: Vec<ConnectionEntryApi>,
    /// Total peers in history
    pub total_peers: usize,
    /// Currently connected count
    pub connected_count: usize,
}

/// Single connection entry for the matrix view.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConnectionEntryApi {
    /// Short peer ID
    pub short_id: String,
    /// Full peer ID
    pub full_id: String,
    /// Location (country code)
    pub location: String,
    /// NAT type: "direct", "full_cone", "restricted", "port_restricted", "symmetric", "unknown"
    pub nat_type: String,
    /// Status: "connected", "disconnected", "never_connected"
    pub status: String,
    /// Outbound connection stats (us -> them)
    pub outbound: DirectionalStatsApi,
    /// Inbound connection stats (them -> us)
    pub inbound: DirectionalStatsApi,
    /// Best RTT ever recorded in milliseconds
    pub best_rtt_ms: Option<u32>,
    /// Total packets exchanged
    pub total_packets: u64,
    /// Connection count
    pub connection_count: u32,
    /// Seconds since first connection
    pub first_connected_secs: u64,
    /// Seconds since last seen
    pub last_seen_secs: u64,
}

/// Directional connection stats with IPv4/IPv6 granularity.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DirectionalStatsApi {
    /// Total attempts
    pub attempts: u32,
    /// Successful attempts
    pub successes: u32,
    /// Failed attempts
    pub failures: u32,
    /// Direct IPv4 outcome: "unknown", "success", "failed"
    pub direct_ipv4: String,
    /// Direct IPv6 outcome
    pub direct_ipv6: String,
    /// NAT traversal IPv4 outcome
    pub nat_ipv4: String,
    /// NAT traversal IPv6 outcome
    pub nat_ipv6: String,
    /// Relay IPv4 outcome
    pub relay_ipv4: String,
    /// Relay IPv6 outcome
    pub relay_ipv6: String,
}

impl DirectionalStatsApi {
    /// Create from TUI DirectionalMethodStats.
    #[allow(clippy::too_many_arguments)]
    pub fn from_method_outcome(
        attempts: u32,
        successes: u32,
        failures: u32,
        direct_ipv4: &str,
        direct_ipv6: &str,
        nat_ipv4: &str,
        nat_ipv6: &str,
        relay_ipv4: &str,
        relay_ipv6: &str,
    ) -> Self {
        Self {
            attempts,
            successes,
            failures,
            direct_ipv4: direct_ipv4.to_string(),
            direct_ipv6: direct_ipv6.to_string(),
            nat_ipv4: nat_ipv4.to_string(),
            nat_ipv6: nat_ipv6.to_string(),
            relay_ipv4: relay_ipv4.to_string(),
            relay_ipv6: relay_ipv6.to_string(),
        }
    }

    /// Compact summary: D4✓D6·N4✓N6·R4·R6·
    pub fn summary(&self) -> String {
        format!(
            "D4{}D6{}N4{}N6{}R4{}R6{}",
            outcome_symbol(&self.direct_ipv4),
            outcome_symbol(&self.direct_ipv6),
            outcome_symbol(&self.nat_ipv4),
            outcome_symbol(&self.nat_ipv6),
            outcome_symbol(&self.relay_ipv4),
            outcome_symbol(&self.relay_ipv6),
        )
    }
}

fn outcome_symbol(outcome: &str) -> &'static str {
    match outcome {
        "success" => "✓",
        "failed" => "×",
        _ => "·",
    }
}

/// Protocol frames response for log display.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FramesResponse {
    /// Recent protocol frames
    pub frames: Vec<ProtocolFrameApi>,
    /// Total frames recorded (may be limited)
    pub total_recorded: usize,
}

/// Single protocol frame for API consumption.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProtocolFrameApi {
    /// Peer ID associated with this frame
    pub peer_id: String,
    /// Frame type (ADD_ADDRESS, PUNCH_ME_NOW, etc.)
    pub frame_type: String,
    /// Direction: "sent" or "received"
    pub direction: String,
    /// Milliseconds since this frame was processed
    pub elapsed_ms: u64,
    /// Additional context
    pub context: Option<String>,
}

/// Gossip health response for gossip protocols display.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GossipResponse {
    /// HyParView membership status
    pub hyparview: HyParViewStatusApi,
    /// SWIM failure detection status
    pub swim: SwimStatusApi,
    /// Plumtree broadcast tree status
    pub plumtree: PlumtreeStatusApi,
    /// Gossip message statistics
    pub message_stats: GossipMessageStatsApi,
}

/// HyParView membership protocol status.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HyParViewStatusApi {
    /// Active view size (directly connected peers)
    pub active_view_size: usize,
    /// Maximum active view size
    pub active_view_max: usize,
    /// Passive view size (known but not connected)
    pub passive_view_size: usize,
    /// Maximum passive view size
    pub passive_view_max: usize,
    /// Peer IDs in active view
    pub active_peers: Vec<String>,
    /// Health status: "healthy", "degraded", "unhealthy"
    pub health: String,
}

/// SWIM failure detection status.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SwimStatusApi {
    /// Peers in alive state
    pub alive_count: usize,
    /// Peers in suspect state
    pub suspect_count: usize,
    /// Peers confirmed failed
    pub failed_count: usize,
    /// Total membership size
    pub membership_size: usize,
    /// Health status: "healthy", "degraded", "unhealthy"
    pub health: String,
}

/// Plumtree broadcast tree status.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlumtreeStatusApi {
    /// Eager push peers (tree edges)
    pub eager_peers: usize,
    /// Lazy push peers (backup)
    pub lazy_peers: usize,
    /// Tree depth (hops from root)
    pub tree_depth: Option<usize>,
    /// Whether tree structure is valid
    pub tree_valid: bool,
    /// Messages broadcast
    pub messages_broadcast: u64,
    /// Messages received
    pub messages_received: u64,
    /// Health status: "healthy", "degraded", "unhealthy"
    pub health: String,
}

/// Gossip message statistics.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GossipMessageStatsApi {
    /// Announcements sent
    pub announcements_sent: u64,
    /// Announcements received
    pub announcements_received: u64,
    /// Peer queries sent
    pub peer_queries_sent: u64,
    /// Peer queries received
    pub peer_queries_received: u64,
    /// Peer responses sent
    pub peer_responses_sent: u64,
    /// Peer responses received
    pub peer_responses_received: u64,
    /// Cache updates
    pub cache_updates: u64,
    /// Cache hits
    pub cache_hits: u64,
    /// Cache misses
    pub cache_misses: u64,
    /// Current cache size
    pub cache_size: u64,
}
