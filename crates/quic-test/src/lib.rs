#![recursion_limit = "512"]

//! ant-quic Test Network Infrastructure
//!
//! This crate provides the large-scale network testing infrastructure for ant-quic,
//! including:
//!
//! - **Gossip-First Peer Discovery**: Epidemic gossip distributes peer information
//! - **Bootstrap Peers**: Hardcoded VPS nodes ensure network connectivity
//! - **Terminal UI**: Interactive display of network status and connections
//! - **Test Protocol**: 5KB packet exchange for connectivity verification
//!
//! # "We will be legion!!"
//!
//! The goal is to prove that our quantum-secure NAT traversal P2P network works
//! at scale, with users simply downloading and running the binary.
//!
//! # Architecture
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────────────────────┐
//! │                     Gossip-First Peer Discovery                          │
//! ├─────────────────────────────────────────────────────────────────────────┤
//! │                                                                          │
//! │   ┌─────────────────────────────────────────────────────────────────┐   │
//! │   │                 Bootstrap Peers (VPS Nodes)                      │   │
//! │   │   saorsa-1 (relay)  saorsa-2  saorsa-3  ...  saorsa-9           │   │
//! │   └──────────────────────────────┬──────────────────────────────────┘   │
//! │                                  │                                       │
//! │                      Epidemic Gossip (saorsa-gossip)                     │
//! │                                  │                                       │
//! │   ┌──────────────────────────────┼──────────────────────────────────┐   │
//! │   │                              │                                   │   │
//! │   ▼                              ▼                                   ▼   │
//! │   ┌─────────┐             ┌─────────┐                         ┌─────────┐│
//! │   │ Node A  │◄───────────►│ Node B  │◄───────────────────────►│ Node C  ││
//! │   │  (TUI)  │   Direct/   │  (TUI)  │   Gossip distributes    │  (TUI)  ││
//! │   │         │   Punched   │         │   peer cache            │         ││
//! │   └─────────┘             └─────────┘                         └─────────┘│
//! └─────────────────────────────────────────────────────────────────────────┘
//! ```
//!
//! # Usage
//!
//! ## As Test Node (default - gossip-first discovery)
//!
//! ```bash
//! ant-quic-test
//! ```
//!
//! The node will:
//! 1. Connect to hardcoded VPS bootstrap peers
//! 2. Sync peer cache via epidemic gossip
//! 3. Discover and connect to other peers
//! 4. Exchange 5KB test packets
//! 5. Display real-time statistics in TUI
//!
//! ## As Relay/Coordinator (on saorsa-1)
//!
//! ```bash
//! ant-quic-test --relay
//! ```

pub mod bootstrap_peers;
pub mod crdt_verification;
pub mod dashboard;
pub mod debug_automation;
pub mod epidemic_gossip;
pub mod gossip;
pub mod gossip_tests;
pub mod gossip_verification;
pub mod harness;
pub mod history;
pub mod lib_verification;
pub mod node;
pub mod orchestrator;
pub mod peer_discovery;
pub mod proof_orchestrator;
pub mod registry;
pub mod tui;

// Re-export key types for convenience
pub use registry::{
    ConnectionMethod,
    // Proof-based testing types
    CrdtConvergenceProof,
    CrdtOperation,
    CrdtType,
    CrossValidation,
    GossipProtocolProof,
    HyParViewProof,
    NatType,
    NetworkConnectivityProof,
    NetworkEvent,
    NetworkStats,
    NodeCapabilities,
    NodeHeartbeat,
    NodeRegistration,
    PeerInfo,
    PeerStore,
    PlumtreeProof,
    ProofBasedTestReport,
    ProofType,
    ProofValidationResult,
    RegistrationResponse,
    RegistryClient,
    RegistryConfig,
    SignedAttestation,
    SwimProof,
    TestAnomaly,
    start_registry_server,
};

pub use tui::{
    App, AppState, ConnectedPeer, ConnectionQuality, InputEvent, LocalNodeInfo, NetworkStatistics,
    TuiConfig, TuiEvent, run_tui, send_tui_event,
};

pub use node::{GlobalStats, TestNode, TestNodeConfig, TestPacket, TestResult};

pub use gossip::{
    CacheStatus, CoordinatorAnnouncement, GossipConfig, GossipDiscovery, GossipEvent,
    GossipIntegration, GossipMetrics, PeerAnnouncement, PeerCapabilities, PeerConnectionQuery,
    PeerConnectionResponse, RelayAnnouncement, TOPIC_COORDINATORS, TOPIC_PEER_QUERY,
    TOPIC_PEER_RESPONSE, TOPIC_PEERS, TOPIC_RELAYS,
};

pub use dashboard::{
    ConnectedPeerApi, ConnectionEntryApi, ConnectionsResponse, DirectionalStatsApi, FramesQuery,
    FramesResponse, GossipMessageStatsApi, GossipResponse, HyParViewStatusApi, LocalNodeApi,
    NetworkStatsApi, OverviewResponse, PlumtreeStatusApi, ProofStatusApi, ProtocolFrameApi,
    SwimStatusApi, dashboard_routes,
};

pub use orchestrator::{
    OrchestratorConfig, OrchestratorStatus, PeerTestResult, TestCommand, TestOrchestrator,
    TestRound, TestTarget,
};

pub use gossip_tests::{
    CrateTestResult, GossipTestCoordinator, GossipTestResults, TestDetail, TestStatus,
};

pub use history::{
    ConnectivityStatus, GossipResults, GossipStatus, HistoryConfig, HistoryEntry, HistoryFile,
    HistoryManager, HistoryStorage, PeerConnectivity,
};

pub use harness::{
    AgentCapabilities, AgentClient, AgentInfo, AgentStatus, ApplyProfileRequest,
    ApplyProfileResponse, ArtifactBundle, ArtifactEntry, ArtifactManifest, ArtifactSpec,
    ArtifactType, AttemptResult, BarrierRequest, BarrierResponse, ClassifiedFailure,
    DimensionStats, FailureBreakdown, FailureCategory, FailureEvidence, FrameCounters,
    GetResultsRequest, GetResultsResponse, HandshakeRequest, HandshakeResponse,
    HealthCheckResponse, IpMode, NatBehaviorProfile, NatProfileSpec, RunProgress, RunStatus,
    RunStatusRequest, RunStatusResponse, RunSummary, ScenarioSpec, StartRunRequest,
    StartRunResponse, StopRunRequest, StopRunResponse, TechniqueResult, TestMatrixSpec,
    ThresholdSpec, TimingSpec, TopologySpec, TopologyType,
};

pub use gossip_verification::{GossipVerifier, GossipVerifierConfig, VerificationSummary};

pub use crdt_verification::{
    ConflictResolutionResult, ConvergenceState, CrdtVerifier, CrdtVerifierConfig, OperationTracker,
    compute_state_hash,
};

pub use debug_automation::{
    Anomaly, AutomatedDebugger, DebugReport, DebuggerConfig, ErrorPattern, LogEntry, RootCause,
    Severity, SuggestedFix, Timeline,
};

pub use proof_orchestrator::{
    OrchestratorReport, ProofOrchestrator, ProofOrchestratorConfig, StepResult,
};

pub use lib_verification::{
    LibraryVerificationResult, TestResult as LibTestResult, TestStatus as LibTestStatus,
    VerificationConfig, print_summary as print_verification_summary, verify_all_libraries,
};
