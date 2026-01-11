//! Peer Registry Module
//!
//! This module provides a central registry for peer discovery in the ant-quic
//! network test infrastructure. Nodes register with the registry on startup,
//! receive a list of other peers, and maintain their registration via heartbeats.
//!
//! # Architecture
//!
//! ```text
//!                     ┌─────────────────────────┐
//!                     │    Registry Server      │
//!                     │    (saorsa-1)           │
//!                     │                         │
//!                     │  POST /api/register     │
//!                     │  POST /api/heartbeat    │
//!                     │  GET  /api/peers        │
//!                     │  GET  /api/stats        │
//!                     │  WS   /ws/live          │
//!                     └───────────┬─────────────┘
//!                                 │
//!         ┌───────────────────────┼───────────────────────┐
//!         │                       │                       │
//!         ▼                       ▼                       ▼
//!    ┌─────────┐            ┌─────────┐            ┌─────────┐
//!    │ Node A  │◄──────────►│ Node B  │◄──────────►│ Node C  │
//!    └─────────┘            └─────────┘            └─────────┘
//! ```
//!
//! # Usage
//!
//! ## Running as Registry Server
//!
//! ```bash
//! ant-quic --registry --port 8080
//! ```
//!
//! ## Connecting as a Node
//!
//! ```rust,ignore
//! use ant_quic::registry::{RegistryClient, NodeRegistration};
//!
//! let client = RegistryClient::new("https://saorsa-1.saorsalabs.com");
//!
//! // Register with the network
//! let registration = NodeRegistration { /* ... */ };
//! let response = client.register(&registration).await?;
//!
//! // Get list of peers to connect to
//! let peers = response.peers;
//!
//! // Send periodic heartbeats
//! let heartbeat = NodeHeartbeat { /* ... */ };
//! client.heartbeat(&heartbeat).await?;
//! ```

mod api;
pub mod geo;
pub mod persistence;
mod store;
mod types;

// Re-export main types
pub use api::{RegistryClient, RegistryConfig, start_registry_server};
pub use geo::BgpGeoProvider;
pub use persistence::{PersistedData, PersistenceConfig, PersistentStorage, StatsSnapshot};
pub use store::{PeerStore, ProofValidationResult};
pub use types::{
    ConnectionBreakdown,
    ConnectionDirection,
    ConnectionMethod,
    ConnectionReport,
    ConnectionTechnique,
    ConnectivityMatrix,
    // Proof-based testing types
    CrdtConvergenceProof,
    CrdtOperation,
    CrdtType,
    CrossValidation,
    DataProof,
    ExperimentResults,
    FailureReasonCode,
    FilteringBehavior,
    FullMeshProbeResult,
    GossipProtocolProof,
    HyParViewProof,
    ImpairmentMetrics,
    MappingBehavior,
    MethodProof,
    MigrationMetrics,
    NatBehavior,
    NatScenario,
    NatStats,
    NatType,
    NetworkConnectivityProof,
    NetworkEvent,
    NetworkProfile,
    NetworkStats,
    NodeCapabilities,
    NodeGossipStats,
    NodeHeartbeat,
    NodeRegistration,
    PathTuple,
    PeerInfo,
    PeerStatus,
    PlumtreeProof,
    PortPreservation,
    ProofBasedTestReport,
    ProofType,
    RegistrationResponse,
    RelayMetrics,
    SignedAttestation,
    SuccessLevel,
    SwimProof,
    TechniqueAttempt,
    TemporalMetrics,
    TemporalScenario,
    TestAnomaly,
    TestPattern,
    TestReport,
    TestSuite,
    TestSuiteConfig,
    unix_timestamp_ms,
};
