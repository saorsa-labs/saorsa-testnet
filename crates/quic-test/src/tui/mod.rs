//! Terminal User Interface Module
//!
//! This module provides an interactive terminal UI for the Saorsa TestNet
//! infrastructure. Users see real-time network status, connected peers,
//! and traffic statistics.
//!
//! # Architecture
//!
//! ```text
//! ╔══════════════════════════════════════════════════════════════════════════════╗
//! ║                            Saorsa TestNet                                    ║
//! ║                         "We will be legion!!"                                ║
//! ╠══════════════════════════════════════════════════════════════════════════════╣
//! ║  YOUR NODE                                                                   ║
//! ╟──────────────────────────────────────────────────────────────────────────────╢
//! ║  Peer ID: a3b7c9d2...    NAT Type: Port Restricted    Registered: ✓         ║
//! ╠══════════════════════════════════════════════════════════════════════════════╣
//! ║  CONNECTED PEERS (3 of 142 registered)                      [Auto-connecting]║
//! ╟──────────────────────────────────────────────────────────────────────────────╢
//! ║  Peer         Location    Method        RTT      TX/RX       Status          ║
//! ╠══════════════════════════════════════════════════════════════════════════════╣
//! ║  NETWORK STATS                                                               ║
//! ╠══════════════════════════════════════════════════════════════════════════════╣
//! ║  [Q] Quit    Dashboard: https://saorsa-1.saorsalabs.com  ML-KEM-768 | ML-DSA-65║
//! ╚══════════════════════════════════════════════════════════════════════════════╝
//! ```
//!
//! # Usage
//!
//! ```rust,ignore
//! use ant_quic::tui::{App, run_tui};
//!
//! #[tokio::main]
//! async fn main() -> anyhow::Result<()> {
//!     let app = App::new();
//!     run_tui(app).await
//! }
//! ```

mod app;
mod screens;
mod types;
mod ui;

pub use app::{App, AppState, InputEvent, Tab};
pub use types::{
    AlertSeverity, AnomalyEntry, CacheHealth, ComponentHealth, ConnectedPeer, ConnectionQuality,
    ConnectivityTestResults, ContactDisplay, ContactOnlineStatus, DhtOperationStats, DhtStats,
    EigenTrustStats, FrameDirection, GeographicDistribution, HealthAlert, HealthStats,
    HealthStatus, LatencyStats, LocalNodeInfo, McpConnectionStatus, McpState, McpTool,
    McpToolCategory, MessageDisplay, NatTraversalPhase, NatTypeAnalytics, NetworkStatistics,
    PlacementStats, ProofStatus, ProtocolFrame, RegionStats, ResourceUsage, TestConnectivityMethod,
    TrafficDirection, TrafficType, TrustEntry, country_flag,
};

use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyEventKind},
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use ratatui::{Terminal, backend::CrosstermBackend};
use std::io;
use std::time::Duration;
use tokio::sync::mpsc;
use tracing::warn;

/// Send a TUI event, logging if the channel is full.
///
/// This is a helper to replace bare `try_send()` calls that silently drop events.
/// Critical events like `PeerConnected` should use this function.
pub fn send_tui_event(tx: &mpsc::Sender<TuiEvent>, event: TuiEvent) {
    if let Err(e) = tx.try_send(event) {
        match e {
            mpsc::error::TrySendError::Full(ev) => {
                warn!(
                    "TUI event channel full, dropping event: {:?}",
                    event_name(&ev)
                );
            }
            mpsc::error::TrySendError::Closed(ev) => {
                warn!(
                    "TUI event channel closed, dropping event: {:?}",
                    event_name(&ev)
                );
            }
        }
    }
}

/// Get a short name for the event type (for logging).
fn event_name(event: &TuiEvent) -> &'static str {
    match event {
        TuiEvent::UpdateLocalNode(_) => "UpdateLocalNode",
        TuiEvent::UpdatePeer(_) => "UpdatePeer",
        TuiEvent::RemovePeer(_) => "RemovePeer",
        TuiEvent::UpdateRegisteredCount(_) => "UpdateRegisteredCount",
        TuiEvent::PacketSent(_) => "PacketSent",
        TuiEvent::PacketReceived(_) => "PacketReceived",
        TuiEvent::RegistrationUpdated(_) => "RegistrationUpdated",
        TuiEvent::HeartbeatSent => "HeartbeatSent",
        TuiEvent::Error(_) => "Error",
        TuiEvent::Info(_) => "Info",
        TuiEvent::ClearMessages => "ClearMessages",
        TuiEvent::Quit => "Quit",
        TuiEvent::RegistrationComplete => "RegistrationComplete",
        TuiEvent::PeerConnected(_) => "PeerConnected",
        TuiEvent::TestPacketResult { .. } => "TestPacketResult",
        TuiEvent::ConnectionFailed => "ConnectionFailed",
        TuiEvent::ConnectionAttempted => "ConnectionAttempted",
        TuiEvent::InboundConnection => "InboundConnection",
        TuiEvent::OutboundConnection => "OutboundConnection",
        TuiEvent::Ipv4Connection => "Ipv4Connection",
        TuiEvent::Ipv6Connection => "Ipv6Connection",
        TuiEvent::GossipPeerDiscovered { .. } => "GossipPeerDiscovered",
        TuiEvent::GossipRelayDiscovered { .. } => "GossipRelayDiscovered",
        TuiEvent::PeerSeen(_) => "PeerSeen",
        TuiEvent::SwimLivenessUpdate { .. } => "SwimLivenessUpdate",
        TuiEvent::ProtocolFrame(_) => "ProtocolFrame",
        TuiEvent::NatPhaseUpdate { .. } => "NatPhaseUpdate",
        TuiEvent::TrafficTypeUpdate { .. } => "TrafficTypeUpdate",
        TuiEvent::CacheHealthUpdate(_) => "CacheHealthUpdate",
        TuiEvent::NatAnalyticsUpdate(_) => "NatAnalyticsUpdate",
        TuiEvent::GeographicDistributionUpdate(_) => "GeographicDistributionUpdate",
        TuiEvent::ConnectivityTestInbound { .. } => "ConnectivityTestInbound",
        TuiEvent::ConnectivityTestStart => "ConnectivityTestStart",
        TuiEvent::ConnectivityTestOutbound { .. } => "ConnectivityTestOutbound",
        TuiEvent::ConnectivityTestComplete => "ConnectivityTestComplete",
        TuiEvent::NatTestOutbound { .. } => "NatTestOutbound",
        TuiEvent::NatTestWaitingForConnectBack { .. } => "NatTestWaitingForConnectBack",
        TuiEvent::NatTestConnectBackSuccess { .. } => "NatTestConnectBackSuccess",
        TuiEvent::NatTestConnectBackTimeout { .. } => "NatTestConnectBackTimeout",
        TuiEvent::NatTestRetrying { .. } => "NatTestRetrying",
        TuiEvent::NatTestPeerUnreachable { .. } => "NatTestPeerUnreachable",
        TuiEvent::FirewallDetected { .. } => "FirewallDetected",
        TuiEvent::GossipTestsStarted => "GossipTestsStarted",
        TuiEvent::GossipTestsComplete(_) => "GossipTestsComplete",
        TuiEvent::GossipCrateTestComplete { .. } => "GossipCrateTestComplete",
        TuiEvent::UpdateGossipStats(_) => "UpdateGossipStats",
        TuiEvent::ProofStatusUpdate(_) => "ProofStatusUpdate",
        // New stats events
        TuiEvent::UpdateDhtStats(_) => "UpdateDhtStats",
        TuiEvent::UpdateEigenTrustStats(_) => "UpdateEigenTrustStats",
        TuiEvent::UpdateAdaptiveStats(_) => "UpdateAdaptiveStats",
        TuiEvent::UpdatePlacementStats(_) => "UpdatePlacementStats",
        TuiEvent::UpdateHealthStats(_) => "UpdateHealthStats",
        TuiEvent::UpdateMcpState(_) => "UpdateMcpState",
        TuiEvent::ContactCreated(_) => "ContactCreated",
        TuiEvent::ContactCreateFailed { .. } => "ContactCreateFailed",
        TuiEvent::ContactsUpdated(_) => "ContactsUpdated",
        TuiEvent::MessageSent { .. } => "MessageSent",
        TuiEvent::MessageSendFailed { .. } => "MessageSendFailed",
        TuiEvent::MessagesLoaded(_) => "MessagesLoaded",
        TuiEvent::MessageReceived(_) => "MessageReceived",
    }
}

/// Events that can be sent to the TUI from other parts of the application.
#[derive(Debug, Clone)]
pub enum TuiEvent {
    /// Update local node information
    UpdateLocalNode(LocalNodeInfo),
    /// Add or update a connected peer
    UpdatePeer(ConnectedPeer),
    /// Remove a disconnected peer
    RemovePeer(String),
    /// Update total registered nodes count
    UpdateRegisteredCount(usize),
    /// Record packet sent
    PacketSent(String),
    /// Record packet received
    PacketReceived(String),
    /// Update registration status
    RegistrationUpdated(bool),
    /// Heartbeat sent
    HeartbeatSent,
    /// Set error message
    Error(String),
    /// Set info message
    Info(String),
    /// Clear messages
    ClearMessages,
    /// Force quit
    Quit,
    /// Registration with registry completed successfully
    RegistrationComplete,
    /// A new peer connected (from TestNode)
    PeerConnected(ConnectedPeer),
    /// Test packet exchange result
    TestPacketResult {
        /// The peer ID the test was with
        peer_id: String,
        /// Whether the test succeeded
        success: bool,
        /// Round-trip time if successful
        rtt: Option<std::time::Duration>,
    },
    /// Connection attempt failed
    ConnectionFailed,
    /// Connection attempt started
    ConnectionAttempted,
    /// Inbound connection received (they connected to us - proves NAT traversal works!)
    InboundConnection,
    /// Outbound connection established (we connected to them)
    OutboundConnection,
    /// IPv4 connection established
    Ipv4Connection,
    /// IPv6 connection established
    Ipv6Connection,
    /// Gossip: peer discovered via gossip network
    GossipPeerDiscovered {
        /// Peer ID of discovered peer
        peer_id: String,
        /// Addresses reported by the peer
        addresses: Vec<String>,
        /// Whether the peer is publicly reachable
        is_public: bool,
    },
    /// Gossip: relay discovered via gossip network
    GossipRelayDiscovered {
        /// Peer ID of the relay
        peer_id: String,
        /// Addresses where relay can be reached
        addresses: Vec<String>,
        /// Current load (active connections)
        load: u32,
    },
    /// A peer was seen/communicated with (for tracking "nodes known alive")
    PeerSeen(String),
    /// SWIM liveness update from saorsa-gossip
    SwimLivenessUpdate {
        /// Peers marked alive by SWIM
        alive: usize,
        /// Peers marked suspect by SWIM
        suspect: usize,
        /// Peers marked dead by SWIM
        dead: usize,
        /// HyParView active view size
        active: usize,
        /// HyParView passive view size
        passive: usize,
    },
    /// Protocol frame logged
    ProtocolFrame(ProtocolFrame),
    /// NAT traversal phase updated for a peer
    NatPhaseUpdate {
        /// Peer ID
        peer_id: String,
        /// New phase
        phase: NatTraversalPhase,
        /// Optional coordinator ID
        coordinator_id: Option<String>,
    },
    /// Traffic type updated for a peer
    TrafficTypeUpdate {
        /// Peer ID
        peer_id: String,
        /// Traffic type
        traffic_type: TrafficType,
        /// Direction
        direction: FrameDirection,
    },
    /// Bootstrap cache health updated
    CacheHealthUpdate(CacheHealth),
    /// NAT type analytics updated
    NatAnalyticsUpdate(NatTypeAnalytics),
    /// Geographic distribution updated
    GeographicDistributionUpdate(GeographicDistribution),
    /// Connectivity test: record inbound connection from VPS node
    ConnectivityTestInbound {
        peer_id: String,
        method: TestConnectivityMethod,
        success: bool,
        rtt_ms: Option<u32>,
    },
    /// Connectivity test: start test (move to inbound wait phase)
    ConnectivityTestStart,
    /// Connectivity test: record outbound connection result
    ConnectivityTestOutbound {
        peer_id: String,
        method: TestConnectivityMethod,
        success: bool,
        rtt_ms: Option<u32>,
    },
    /// Connectivity test: mark phase as complete
    ConnectivityTestComplete,

    /// NAT test: attempting outbound connection to peer
    NatTestOutbound { peer_id: String, address: String },
    /// NAT test: outbound succeeded, now waiting for connect-back
    NatTestWaitingForConnectBack {
        peer_id: String,
        seconds_remaining: u32,
    },
    /// NAT test: peer connected back successfully (full NAT traversal verified)
    NatTestConnectBackSuccess { peer_id: String },
    /// NAT test: connect-back timeout, will retry to verify peer is alive
    NatTestConnectBackTimeout { peer_id: String },
    /// NAT test: retry attempt to verify peer is still reachable
    NatTestRetrying { peer_id: String },
    /// NAT test: peer unreachable after retry (they may have gone offline)
    NatTestPeerUnreachable { peer_id: String },
    /// Firewall detected: cannot connect outbound to any peer
    FirewallDetected { attempted_count: usize },
    /// Gossip tests: started running all 9 crate tests
    GossipTestsStarted,
    /// Gossip tests: all 9 crate tests completed
    GossipTestsComplete(crate::gossip_tests::GossipTestResults),
    /// Gossip tests: single crate test completed
    GossipCrateTestComplete {
        crate_name: String,
        passed: bool,
        tests_passed: u32,
        tests_total: u32,
    },
    /// Update gossip stats for TUI display
    UpdateGossipStats(crate::registry::NodeGossipStats),
    /// Update proof verification status
    ProofStatusUpdate(types::ProofStatus),
    // === New stats events for expanded TUI tabs ===
    /// Update DHT statistics [Tab 5]
    UpdateDhtStats(types::DhtStats),
    /// Update EigenTrust statistics [Tab 6]
    UpdateEigenTrustStats(types::EigenTrustStats),
    /// Update Adaptive networking statistics [Tab 7]
    UpdateAdaptiveStats(types::AdaptiveStats),
    /// Update Placement statistics [Tab 8]
    UpdatePlacementStats(types::PlacementStats),
    /// Update Health monitoring statistics [Tab 9]
    UpdateHealthStats(types::HealthStats),
    /// Update MCP client state [Tab 0]
    UpdateMcpState(types::McpState),
    /// Contact created successfully
    ContactCreated(types::ContactDisplay),
    /// Contact creation failed
    ContactCreateFailed {
        /// The four-word ID that failed
        four_words: String,
        /// Error message
        error: String,
    },
    /// Contacts list updated
    ContactsUpdated(Vec<types::ContactDisplay>),
    /// Message sent successfully
    MessageSent {
        /// Message ID
        message_id: String,
        /// Recipient
        recipient: String,
    },
    /// Message send failed
    MessageSendFailed {
        /// Recipient
        recipient: String,
        /// Error message
        error: String,
    },
    /// Messages loaded for a conversation
    MessagesLoaded(Vec<types::MessageDisplay>),
    /// Incoming message received
    MessageReceived(types::MessageDisplay),
}

/// MCP request from TUI to McpClient
#[derive(Debug, Clone)]
pub enum McpRequest {
    /// Connect to a peer using 4-word encoded address
    ConnectByWords {
        /// Four words encoding the peer's IP:port
        words: String,
    },
    /// Create a contact from four-word ID
    CreateContact {
        /// Four-word identity to add
        four_words: String,
        /// Optional display name
        display_name: Option<String>,
    },
    /// List all contacts
    ListContacts,
    /// Delete a contact
    DeleteContact {
        /// Contact ID to delete
        contact_id: String,
    },
    /// Toggle favourite status
    ToggleFavourite {
        /// Four-word ID of contact
        four_words: String,
    },
    /// Send a direct message
    SendMessage {
        /// Recipient four-word ID or peer ID
        recipient: String,
        /// Message text
        text: String,
    },
    /// Load messages for a conversation with a contact
    LoadMessages {
        /// Contact's peer ID or four-word ID
        contact_id: String,
    },
    /// Announce our presence to the network
    AnnouncePresence,
    /// Query for a peer's presence by pubkey
    QueryPresence {
        /// Public key (hex or base64)
        pubkey: String,
    },
    /// Get our own presence record
    GetOurPresence,
    /// Get cached presence for a peer
    GetCachedPresence {
        /// Public key (hex or base64)
        pubkey: String,
    },
}

/// Configuration for the TUI.
#[derive(Debug, Clone)]
pub struct TuiConfig {
    /// Tick rate for UI updates
    pub tick_rate: Duration,
    /// Registry URL to display
    pub registry_url: String,
    /// Dashboard URL to display
    pub dashboard_url: String,
}

impl Default for TuiConfig {
    fn default() -> Self {
        Self {
            tick_rate: Duration::from_millis(250),
            registry_url: "https://saorsa-1.saorsalabs.com".to_string(),
            dashboard_url: "https://saorsa-1.saorsalabs.com".to_string(),
        }
    }
}

/// Run the terminal UI with the given application state.
///
/// Returns when the user quits (Q key or Esc).
///
/// # Arguments
/// * `app` - The application state
/// * `event_rx` - Receiver for TUI events from background tasks
/// * `_event_tx` - Sender for TUI events (unused, kept for API compatibility)
/// * `mcp_request_tx` - Optional sender for MCP requests (contact management, tool invocation)
pub async fn run_tui(
    mut app: App,
    mut event_rx: mpsc::Receiver<TuiEvent>,
    _event_tx: mpsc::Sender<TuiEvent>,
    mcp_request_tx: Option<mpsc::Sender<McpRequest>>,
) -> anyhow::Result<()> {
    use std::io::Write;

    // Setup terminal with panic handler to ensure cleanup
    let original_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |panic_info| {
        // Restore terminal on panic
        let _ = disable_raw_mode();
        let _ = execute!(io::stdout(), LeaveAlternateScreen, DisableMouseCapture);
        let _ = io::stdout().flush();
        original_hook(panic_info);
    }));

    enable_raw_mode()?;
    let mut stdout = io::stdout();
    stdout.flush()?;
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    stdout.flush()?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    terminal.clear()?;

    let tick_rate = Duration::from_millis(250);
    let mut last_tick = std::time::Instant::now();

    // Process any pending events BEFORE first draw
    // This ensures local node info is displayed immediately
    while let Ok(event) = event_rx.try_recv() {
        handle_tui_event(&mut app, event);
    }

    loop {
        terminal.draw(|frame| ui::draw(frame, &mut app))?;

        // Calculate timeout for event polling
        let timeout = tick_rate
            .checked_sub(last_tick.elapsed())
            .unwrap_or_else(|| Duration::from_millis(0));

        // Poll for terminal events with timeout
        if crossterm::event::poll(timeout)? {
            if let Event::Key(key) = event::read()? {
                // Only handle key press events (not release)
                if key.kind == KeyEventKind::Press {
                    use crossterm::event::KeyCode;

                    // MCP tab has special key handling for category/tool navigation and parameter editing
                    let handled_by_mcp = if app.active_tab == app::Tab::Mcp {
                        // Contact add mode has highest priority
                        if app.contact_is_adding() {
                            match key.code {
                                KeyCode::Esc => {
                                    app.contact_cancel_add();
                                    true
                                }
                                KeyCode::Backspace => {
                                    app.contact_backspace();
                                    true
                                }
                                KeyCode::Enter => {
                                    // Submit the contact - returns four-word ID if valid
                                    // Connection words encode IP:port, not identity - we need to
                                    // connect first to receive the peer's identity packet
                                    if let Some(four_words) = app.contact_submit_add() {
                                        // Send MCP request if channel is available
                                        if let Some(ref tx) = mcp_request_tx {
                                            let _ = tx.try_send(McpRequest::ConnectByWords {
                                                words: four_words.clone(),
                                            });
                                            // Show pending state
                                            app.info_message =
                                                Some(format!("Connecting to {}...", four_words));
                                        } else {
                                            // No MCP client - show error
                                            app.error_message =
                                                Some("MCP client not available".to_string());
                                        }
                                    }
                                    true
                                }
                                KeyCode::Char(c) => {
                                    app.contact_type_char(c);
                                    true
                                }
                                _ => false,
                            }
                        }
                        // Message compose mode has second priority
                        else if app.message_is_composing() {
                            match key.code {
                                KeyCode::Esc => {
                                    app.message_cancel_compose();
                                    true
                                }
                                KeyCode::Backspace => {
                                    app.message_backspace();
                                    true
                                }
                                KeyCode::Enter => {
                                    // Submit the message
                                    if let Some((recipient, text)) = app.message_submit() {
                                        // Send MCP request if channel is available
                                        if let Some(ref tx) = mcp_request_tx {
                                            let _ = tx.try_send(McpRequest::SendMessage {
                                                recipient: recipient.clone(),
                                                text: text.clone(),
                                            });
                                            app.info_message =
                                                Some(format!("Sending to {}...", recipient));
                                        } else {
                                            // Stub: show message in UI directly
                                            use crate::tui::types::MessageDisplay;
                                            app.mcp_state.current_messages.push(MessageDisplay {
                                                id: uuid::Uuid::new_v4().to_string(),
                                                text,
                                                author: "me".to_string(),
                                                is_outgoing: true,
                                                timestamp: chrono::Utc::now()
                                                    .format("%H:%M")
                                                    .to_string(),
                                                edited: false,
                                            });
                                        }
                                    }
                                    true
                                }
                                KeyCode::Char(c) => {
                                    app.message_type_char(c);
                                    true
                                }
                                _ => false,
                            }
                        }
                        // When in tool parameter edit mode, capture all character input
                        else if app.mcp_is_editing() {
                            match key.code {
                                KeyCode::Esc => {
                                    app.mcp_exit_edit();
                                    true
                                }
                                KeyCode::Tab => {
                                    app.mcp_next_param();
                                    true
                                }
                                KeyCode::BackTab => {
                                    app.mcp_prev_param();
                                    true
                                }
                                KeyCode::Backspace => {
                                    app.mcp_backspace();
                                    true
                                }
                                KeyCode::Enter => {
                                    // Invoke the tool with current parameters
                                    app.mcp_invoke_tool();
                                    true
                                }
                                KeyCode::Char(c) => {
                                    app.mcp_type_char(c);
                                    true
                                }
                                KeyCode::Up => {
                                    app.mcp_prev_param();
                                    true
                                }
                                KeyCode::Down => {
                                    app.mcp_next_param();
                                    true
                                }
                                _ => false,
                            }
                        } else {
                            // Not in edit mode - normal MCP navigation
                            match key.code {
                                // Arrow keys for MCP navigation
                                KeyCode::Left => {
                                    app.mcp_prev_category();
                                    true
                                }
                                KeyCode::Right => {
                                    app.mcp_next_category();
                                    true
                                }
                                KeyCode::Up => {
                                    app.mcp_tool_up();
                                    true
                                }
                                KeyCode::Down => {
                                    app.mcp_tool_down();
                                    true
                                }
                                KeyCode::Enter => {
                                    // Enter edit mode if tool has parameters
                                    if app.mcp_get_selected_tool().is_some() {
                                        app.mcp_enter_edit();
                                    }
                                    true
                                }
                                // 'a' key to add a contact (when in Messages category)
                                KeyCode::Char('a') => {
                                    if app.mcp_state.selected_category
                                        == crate::tui::types::McpToolCategory::Messages
                                    {
                                        app.contact_start_add();
                                        true
                                    } else {
                                        false // Let 'a' fall through to global handler
                                    }
                                }
                                // 'm' key to compose a message to selected contact
                                KeyCode::Char('m') => {
                                    if app.mcp_state.selected_category
                                        == crate::tui::types::McpToolCategory::Messages
                                    {
                                        if app.message_start_compose() {
                                            true
                                        } else {
                                            // No contact selected - show hint
                                            app.info_message =
                                                Some("Select a contact first (↑/↓)".to_string());
                                            true
                                        }
                                    } else {
                                        false // Let 'm' fall through to global handler
                                    }
                                }
                                // Number keys 1-7 for category selection on MCP tab
                                KeyCode::Char('1') => {
                                    app.mcp_select_category(0);
                                    true
                                }
                                KeyCode::Char('2') => {
                                    app.mcp_select_category(1);
                                    true
                                }
                                KeyCode::Char('3') => {
                                    app.mcp_select_category(2);
                                    true
                                }
                                KeyCode::Char('4') => {
                                    app.mcp_select_category(3);
                                    true
                                }
                                KeyCode::Char('5') => {
                                    app.mcp_select_category(4);
                                    true
                                }
                                KeyCode::Char('6') => {
                                    app.mcp_select_category(5);
                                    true
                                }
                                KeyCode::Char('7') => {
                                    app.mcp_select_category(6);
                                    true
                                }
                                _ => false, // Let other keys fall through
                            }
                        }
                    } else {
                        false
                    };

                    // Only process global input events if not handled by MCP tab
                    if !handled_by_mcp {
                        match InputEvent::from_key(key.code) {
                            InputEvent::Quit => {
                                // If help overlay is open, close it instead of quitting
                                if app.show_proof_help {
                                    app.show_proof_help = false;
                                } else {
                                    app.quit();
                                }
                            }
                            InputEvent::ToggleAutoConnect => {
                                app.auto_connecting = !app.auto_connecting;
                            }
                            InputEvent::Refresh => {
                                terminal.clear()?;
                            }
                            InputEvent::ResetConnectivityTest => {
                                app.connectivity_test.reset();
                            }
                            InputEvent::ScrollUp => {
                                app.scroll_connections_up();
                            }
                            InputEvent::ScrollDown => {
                                app.scroll_connections_down();
                            }
                            InputEvent::PageUp => {
                                app.scroll_connections_page_up();
                            }
                            InputEvent::PageDown => {
                                app.scroll_connections_page_down();
                            }
                            InputEvent::NextTab => {
                                app.next_tab();
                            }
                            InputEvent::PrevTab => {
                                app.prev_tab();
                            }
                            InputEvent::TabOverview => {
                                app.active_tab = app::Tab::Overview;
                            }
                            InputEvent::TabGossipHealth => {
                                app.active_tab = app::Tab::GossipHealth;
                            }
                            InputEvent::TabProtocolLog => {
                                app.active_tab = app::Tab::ProtocolLog;
                            }
                            InputEvent::TabMcp => {
                                app.active_tab = app::Tab::Mcp;
                            }
                            InputEvent::ToggleProofHelp => {
                                app.toggle_proof_help();
                            }
                            InputEvent::Unknown => {}
                        }
                    }
                }
            }
        }

        // Check for application events (non-blocking)
        while let Ok(event) = event_rx.try_recv() {
            handle_tui_event(&mut app, event);
        }

        // Handle tick
        if last_tick.elapsed() >= tick_rate {
            // Clear traffic indicators periodically
            app.clear_traffic_indicators();
            last_tick = std::time::Instant::now();
        }

        // Check if we should quit
        if app.should_quit() {
            break;
        }
    }

    // Restore terminal - ensure complete cleanup to prevent screen corruption
    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    // Flush stdout to ensure all escape sequences are written
    let _ = io::stdout().flush();

    Ok(())
}

/// Handle a TUI event.
fn handle_tui_event(app: &mut App, event: TuiEvent) {
    match event {
        TuiEvent::UpdateLocalNode(node_info) => {
            let was_registered = app.local_node.registered;
            app.local_node = node_info;
            if was_registered {
                app.local_node.registered = true;
            }
        }
        TuiEvent::UpdatePeer(peer) => {
            app.update_peer(peer);
        }
        TuiEvent::RemovePeer(peer_id) => {
            app.remove_peer(&peer_id);
        }
        TuiEvent::UpdateRegisteredCount(count) => {
            app.total_registered_nodes = count;
        }
        TuiEvent::PacketSent(peer_id) => {
            app.packet_sent(&peer_id);
        }
        TuiEvent::PacketReceived(peer_id) => {
            app.packet_received(&peer_id);
        }
        TuiEvent::RegistrationUpdated(registered) => {
            app.set_registered(registered);
        }
        TuiEvent::HeartbeatSent => {
            app.heartbeat_sent();
        }
        TuiEvent::Error(msg) => {
            app.set_error(&msg);
        }
        TuiEvent::Info(msg) => {
            app.set_info(&msg);
        }
        TuiEvent::ClearMessages => {
            app.clear_error();
            app.clear_info();
        }
        TuiEvent::Quit => {
            app.quit();
        }
        TuiEvent::RegistrationComplete => {
            app.set_registered(true);
            app.set_info("Registered with network registry");
        }
        TuiEvent::PeerConnected(peer) => {
            app.peer_seen(&peer.full_id);
            let peer_id = peer.full_id.clone();
            let is_ipv6 = peer.connectivity.active_is_ipv6;
            let method_str = match peer.method {
                crate::registry::ConnectionMethod::Direct => {
                    app.stats.direct_connections += 1;
                    "DIRECT"
                }
                crate::registry::ConnectionMethod::HolePunched => {
                    app.stats.hole_punched_connections += 1;
                    "PUNCHED"
                }
                crate::registry::ConnectionMethod::Relayed => {
                    app.stats.relayed_connections += 1;
                    "RELAYED"
                }
            };
            app.add_protocol_frame(ProtocolFrame {
                peer_id: peer_id.clone(),
                frame_type: "CONNECTED".to_string(),
                direction: FrameDirection::Received,
                timestamp: std::time::Instant::now(),
                context: Some(method_str.to_string()),
            });

            // Record method outcome for connectivity matrix auto-population
            let test_method = TestConnectivityMethod::from_registry_method(peer.method, is_ipv6);

            // Record based on connection direction
            match peer.direction {
                crate::registry::ConnectionDirection::Inbound => {
                    app.record_inbound_connection(&peer_id, test_method, true, None);
                }
                crate::registry::ConnectionDirection::Outbound => {
                    app.record_outbound_connection(&peer_id, test_method, true, None);
                }
            }

            app.update_peer(peer);
            app.stats.unique_peers_attempted.insert(peer_id.clone());
            app.stats.unique_peers_connected.insert(peer_id);
            app.stats.connection_successes += 1;
            app.stats.connection_attempts += 1;
        }
        TuiEvent::ConnectionFailed => {
            app.stats.connection_failures += 1;
            app.stats.connection_attempts += 1;
        }
        TuiEvent::ConnectionAttempted => {
            app.stats.connection_attempts += 1;
        }
        TuiEvent::TestPacketResult {
            peer_id,
            success,
            rtt,
        } => {
            if success {
                // Mark peer as seen (successful communication)
                app.peer_seen(&peer_id);
                app.packet_sent(&peer_id);
                app.packet_received(&peer_id);
                if let Some(rtt) = rtt {
                    app.update_peer_rtt(&peer_id, rtt);
                }
            }
        }
        TuiEvent::InboundConnection => {
            app.stats.inbound_connections += 1;
        }
        TuiEvent::OutboundConnection => {
            app.stats.outbound_connections += 1;
        }
        TuiEvent::Ipv4Connection => {
            app.stats.ipv4_connections += 1;
        }
        TuiEvent::Ipv6Connection => {
            app.stats.ipv6_connections += 1;
        }
        TuiEvent::GossipPeerDiscovered {
            peer_id,
            addresses,
            is_public,
        } => {
            // Log gossip peer discovery - could track in stats later
            tracing::debug!(
                "Gossip discovered peer {} ({} addresses, public={})",
                &peer_id[..16.min(peer_id.len())],
                addresses.len(),
                is_public
            );
            app.stats.gossip_peers_discovered += 1;
            // Mark peer as seen (we received gossip from them)
            app.peer_seen(&peer_id);
        }
        TuiEvent::GossipRelayDiscovered {
            peer_id,
            addresses,
            load,
        } => {
            // Log gossip relay discovery - could track in stats later
            tracing::debug!(
                "Gossip discovered relay {} ({} addresses, load={})",
                &peer_id[..16.min(peer_id.len())],
                addresses.len(),
                load
            );
            app.stats.gossip_relays_discovered += 1;
            // Also mark relay as seen
            app.peer_seen(&peer_id);
        }
        TuiEvent::PeerSeen(peer_id) => {
            app.peer_seen(&peer_id);
        }
        TuiEvent::SwimLivenessUpdate {
            alive,
            suspect,
            dead,
            active,
            passive,
        } => {
            app.stats.swim_alive = alive;
            app.stats.swim_suspect = suspect;
            app.stats.swim_dead = dead;
            app.stats.hyparview_active = active;
            app.stats.hyparview_passive = passive;
        }
        TuiEvent::ProtocolFrame(frame) => {
            app.add_protocol_frame(frame);
        }
        TuiEvent::NatPhaseUpdate {
            peer_id,
            phase,
            coordinator_id,
        } => {
            app.update_nat_phase(&peer_id, phase, coordinator_id);
        }
        TuiEvent::TrafficTypeUpdate {
            peer_id,
            traffic_type,
            direction,
        } => {
            app.update_traffic_type(&peer_id, traffic_type, direction);
        }
        TuiEvent::CacheHealthUpdate(health) => {
            app.update_cache_health(health);
        }
        TuiEvent::NatAnalyticsUpdate(analytics) => {
            app.update_nat_analytics(analytics);
        }
        TuiEvent::GeographicDistributionUpdate(distribution) => {
            app.update_geographic_distribution(distribution);
        }
        TuiEvent::ConnectivityTestInbound {
            peer_id,
            method,
            success,
            rtt_ms,
        } => {
            app.record_inbound_connection(&peer_id, method, success, rtt_ms);
        }
        TuiEvent::ConnectivityTestStart => {
            app.connectivity_test_inbound_phase();
        }
        TuiEvent::ConnectivityTestOutbound {
            peer_id,
            method,
            success,
            rtt_ms,
        } => {
            app.record_outbound_connection(&peer_id, method, success, rtt_ms);
        }
        TuiEvent::ConnectivityTestComplete => {
            app.connectivity_test.phase = types::ConnectivityTestPhase::Complete;
        }
        TuiEvent::NatTestOutbound { peer_id, address } => {
            app.set_info(&format!(
                "Connecting to {} at {}",
                &peer_id[..8.min(peer_id.len())],
                address
            ));
            app.update_peer_nat_test_state(&peer_id, types::PeerNatTestState::ConnectingOutbound);
        }
        TuiEvent::NatTestWaitingForConnectBack {
            peer_id,
            seconds_remaining,
        } => {
            let short_id = &peer_id[..8.min(peer_id.len())];
            app.set_info(&format!(
                "Waiting {}s for {} to connect back",
                seconds_remaining, short_id
            ));
            app.update_peer_nat_test_state(
                &peer_id,
                types::PeerNatTestState::WaitingForConnectBack { seconds_remaining },
            );
        }
        TuiEvent::NatTestConnectBackSuccess { peer_id } => {
            let short_id = &peer_id[..8.min(peer_id.len())];
            app.set_info(&format!(
                "✓ {} connected back - NAT traversal verified!",
                short_id
            ));
            app.update_peer_nat_test_state(&peer_id, types::PeerNatTestState::Verified);
        }
        TuiEvent::NatTestConnectBackTimeout { peer_id } => {
            let short_id = &peer_id[..8.min(peer_id.len())];
            app.set_info(&format!(
                "Timeout waiting for {} - retrying to check if peer is alive",
                short_id
            ));
            app.update_peer_nat_test_state(&peer_id, types::PeerNatTestState::TimedOut);
        }
        TuiEvent::NatTestRetrying { peer_id } => {
            let short_id = &peer_id[..8.min(peer_id.len())];
            app.set_info(&format!(
                "Retrying connection to {} to verify peer is still online",
                short_id
            ));
            app.update_peer_nat_test_state(&peer_id, types::PeerNatTestState::Retrying);
        }
        TuiEvent::NatTestPeerUnreachable { peer_id } => {
            let short_id = &peer_id[..8.min(peer_id.len())];
            app.set_info(&format!(
                "✗ {} unreachable - peer may have gone offline",
                short_id
            ));
            app.update_peer_nat_test_state(&peer_id, types::PeerNatTestState::Unreachable);
        }
        TuiEvent::FirewallDetected { attempted_count } => {
            app.set_error(&format!(
                "⚠ Cannot connect to any peers ({} attempted). Your firewall or network may be blocking outbound connections.",
                attempted_count
            ));
        }
        TuiEvent::GossipTestsStarted => {
            app.start_gossip_tests();
            app.set_info("Running gossip crate tests...");
        }
        TuiEvent::GossipTestsComplete(results) => {
            let summary = results.summary();
            app.update_gossip_results(results);
            app.set_info(&format!("Gossip tests complete: {}", summary));
        }
        TuiEvent::GossipCrateTestComplete {
            crate_name,
            passed,
            tests_passed,
            tests_total,
        } => {
            let status = if passed { "✓" } else { "✗" };
            tracing::debug!(
                "Gossip crate {} {}: {}/{} tests passed",
                crate_name,
                status,
                tests_passed,
                tests_total
            );
        }
        TuiEvent::UpdateGossipStats(stats) => {
            app.update_gossip_stats(stats);
        }
        TuiEvent::ProofStatusUpdate(status) => {
            app.update_proof_status(status);
        }
        // === New stats event handlers ===
        TuiEvent::UpdateDhtStats(stats) => {
            app.update_dht_stats(stats);
        }
        TuiEvent::UpdateEigenTrustStats(stats) => {
            app.update_eigentrust_stats(stats);
        }
        TuiEvent::UpdateAdaptiveStats(stats) => {
            app.update_adaptive_stats(stats);
        }
        TuiEvent::UpdatePlacementStats(stats) => {
            app.update_placement_stats(stats);
        }
        TuiEvent::UpdateHealthStats(stats) => {
            app.update_health_stats(stats);
        }
        TuiEvent::UpdateMcpState(state) => {
            app.update_mcp_state(state);
        }
        TuiEvent::ContactCreated(contact) => {
            // Add the new contact to the list
            app.mcp_state.contacts.push(contact);
        }
        TuiEvent::ContactCreateFailed { four_words, error } => {
            // Show error message
            app.error_message = Some(format!("Failed to add {}: {}", four_words, error));
        }
        TuiEvent::ContactsUpdated(contacts) => {
            // Replace the entire contacts list
            app.mcp_state.contacts = contacts;
        }
        TuiEvent::MessageSent { recipient, .. } => {
            app.info_message = Some(format!("Message sent to {}", recipient));
        }
        TuiEvent::MessageSendFailed { recipient, error } => {
            app.error_message = Some(format!("Failed to send to {}: {}", recipient, error));
        }
        TuiEvent::MessagesLoaded(messages) => {
            app.message_set_current(messages);
        }
        TuiEvent::MessageReceived(message) => {
            // Add incoming message to current conversation
            app.mcp_state.current_messages.push(message);
            app.info_message = Some("New message received".to_string());
        }
    }
}

/// Create a standalone TUI for visual testing and development.
///
/// Creates the TUI with an empty event channel (no TestNode backend).
pub async fn run_standalone() -> anyhow::Result<()> {
    let app = App::new();
    let (tx, rx) = mpsc::channel(100);
    run_tui(app, rx, tx, None).await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tui_config_default() {
        let config = TuiConfig::default();
        assert_eq!(config.tick_rate, Duration::from_millis(250));
        assert!(config.registry_url.contains("saorsalabs"));
    }

    #[test]
    fn test_tui_event_variants() {
        // Just verify all variants can be created
        let _ = TuiEvent::UpdateLocalNode(LocalNodeInfo::default());
        let _ = TuiEvent::UpdatePeer(ConnectedPeer::new(
            "test",
            crate::registry::ConnectionMethod::Direct,
        ));
        let _ = TuiEvent::RemovePeer("test".to_string());
        let _ = TuiEvent::UpdateRegisteredCount(10);
        let _ = TuiEvent::PacketSent("test".to_string());
        let _ = TuiEvent::PacketReceived("test".to_string());
        let _ = TuiEvent::RegistrationUpdated(true);
        let _ = TuiEvent::HeartbeatSent;
        let _ = TuiEvent::Error("error".to_string());
        let _ = TuiEvent::Info("info".to_string());
        let _ = TuiEvent::ClearMessages;
        let _ = TuiEvent::Quit;
        let _ = TuiEvent::SwimLivenessUpdate {
            alive: 5,
            suspect: 1,
            dead: 0,
            active: 4,
            passive: 20,
        };
        // Test new Phase 1 events
        let _ = TuiEvent::ProtocolFrame(ProtocolFrame {
            peer_id: "test_peer".to_string(),
            frame_type: "ADD_ADDRESS".to_string(),
            direction: FrameDirection::Sent,
            timestamp: std::time::Instant::now(),
            context: Some("test context".to_string()),
        });
        let _ = TuiEvent::NatPhaseUpdate {
            peer_id: "test_peer".to_string(),
            phase: crate::tui::types::NatTraversalPhase::Punching,
            coordinator_id: Some("coordinator".to_string()),
        };
        let _ = TuiEvent::TrafficTypeUpdate {
            peer_id: "test_peer".to_string(),
            traffic_type: crate::tui::types::TrafficType::TestData,
            direction: FrameDirection::Sent,
        };
        let _ = TuiEvent::CacheHealthUpdate(CacheHealth {
            total_peers: 100,
            valid_peers: 80,
            public_peers: 20,
            average_quality: 0.75,
            cache_age: std::time::Duration::from_secs(3600),
            last_updated: Some(std::time::Instant::now()),
            cache_hits: 800,
            cache_misses: 200,
            fresh_peers: 70,
            stale_peers: 30,
            private_peers: 80,
            public_quality: 0.85,
            private_quality: 0.65,
        });
    }

    #[test]
    fn test_handle_tui_event() {
        let mut app = App::new();

        // Test registration update
        handle_tui_event(&mut app, TuiEvent::RegistrationUpdated(true));
        assert!(app.local_node.registered);

        // Test packet events
        let peer = ConnectedPeer::new("test_peer", crate::registry::ConnectionMethod::Direct);
        handle_tui_event(&mut app, TuiEvent::UpdatePeer(peer));
        handle_tui_event(&mut app, TuiEvent::PacketSent("test_peer".to_string()));
        assert_eq!(app.stats.packets_sent, 1);

        // Test quit
        handle_tui_event(&mut app, TuiEvent::Quit);
        assert!(app.should_quit());
    }

    #[test]
    fn test_swim_liveness_update() {
        let mut app = App::new();

        // Test SWIM liveness stats update
        handle_tui_event(
            &mut app,
            TuiEvent::SwimLivenessUpdate {
                alive: 7,
                suspect: 2,
                dead: 1,
                active: 5,
                passive: 30,
            },
        );

        assert_eq!(app.stats.swim_alive, 7);
        assert_eq!(app.stats.swim_suspect, 2);
        assert_eq!(app.stats.swim_dead, 1);
        assert_eq!(app.stats.hyparview_active, 5);
        assert_eq!(app.stats.hyparview_passive, 30);
    }
}
