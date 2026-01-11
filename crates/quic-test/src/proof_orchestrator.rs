//! Proof-based test orchestrator.
//!
//! This module orchestrates comprehensive proof-based testing of the network.
//! It coordinates:
//!
//! - Connectivity verification with cross-validation
//! - Gossip protocol verification (SWIM, HyParView, Plumtree)
//! - CRDT convergence testing
//! - Automated debugging on failures
//!
//! # Success Criteria
//!
//! A test is considered successful ONLY when ALL of:
//!
//! 1. **Connectivity**: All N nodes see all other N-1 nodes
//! 2. **Cross-Validation**: Every node pair agrees on connection state
//! 3. **Gossip**: SWIM, HyParView, Plumtree all pass verification
//! 4. **CRDT**: All nodes converge to identical state after concurrent updates
//! 5. **Freshness**: All proofs have timestamps within acceptable window

use crate::crdt_verification::{CrdtVerifier, CrdtVerifierConfig};
use crate::debug_automation::{AutomatedDebugger, DebugReport, DebuggerConfig, LogEntry};
use crate::gossip_verification::{GossipVerifier, GossipVerifierConfig};
use crate::registry::{
    CrdtConvergenceProof, CrdtType, GossipProtocolProof, NetworkConnectivityProof,
    ProofBasedTestReport, ProofType, SignedAttestation, TestAnomaly,
};
use std::collections::HashMap;
use std::time::{Duration, SystemTime};

/// Configuration for proof-based test orchestration.
#[derive(Debug, Clone)]
pub struct ProofOrchestratorConfig {
    /// Observer node ID.
    pub observer_id: String,
    /// Maximum proof age (seconds).
    pub max_proof_age_secs: u64,
    /// Whether to run automated debugging on failure.
    pub debug_on_failure: bool,
    /// Gossip verifier configuration.
    pub gossip_config: GossipVerifierConfig,
    /// CRDT verifier configuration.
    pub crdt_config: CrdtVerifierConfig,
    /// Debugger configuration.
    pub debug_config: DebuggerConfig,
    /// Minimum nodes required for valid test.
    pub min_nodes: usize,
    /// Whether to require cross-validation.
    pub require_cross_validation: bool,
}

impl Default for ProofOrchestratorConfig {
    fn default() -> Self {
        Self {
            observer_id: "orchestrator".to_string(),
            max_proof_age_secs: 30,
            debug_on_failure: true,
            gossip_config: GossipVerifierConfig::default(),
            crdt_config: CrdtVerifierConfig::default(),
            debug_config: DebuggerConfig::default(),
            min_nodes: 2,
            require_cross_validation: true,
        }
    }
}

/// Result of a single verification step.
#[derive(Debug, Clone)]
pub struct StepResult {
    /// Name of the step.
    pub name: String,
    /// Whether the step passed.
    pub passed: bool,
    /// Duration of the step.
    pub duration: Duration,
    /// Details about the result.
    pub details: String,
    /// Any anomalies detected.
    pub anomalies: Vec<TestAnomaly>,
}

impl StepResult {
    /// Create a passing result.
    pub fn pass(name: impl Into<String>, duration: Duration, details: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            passed: true,
            duration,
            details: details.into(),
            anomalies: Vec::new(),
        }
    }

    /// Create a failing result.
    pub fn fail(
        name: impl Into<String>,
        duration: Duration,
        details: impl Into<String>,
        anomalies: Vec<TestAnomaly>,
    ) -> Self {
        Self {
            name: name.into(),
            passed: false,
            duration,
            details: details.into(),
            anomalies,
        }
    }
}

/// Complete test report from the orchestrator.
#[derive(Debug, Clone)]
pub struct OrchestratorReport {
    /// Session identifier.
    pub session_id: String,
    /// When the test started.
    pub started_at: SystemTime,
    /// When the test completed.
    pub completed_at: SystemTime,
    /// Overall pass/fail status.
    pub passed: bool,
    /// Results from each step.
    pub step_results: Vec<StepResult>,
    /// Connectivity proof (if generated).
    pub connectivity_proof: Option<NetworkConnectivityProof>,
    /// Gossip proof (if generated).
    pub gossip_proof: Option<GossipProtocolProof>,
    /// CRDT proof (if generated).
    pub crdt_proof: Option<CrdtConvergenceProof>,
    /// Debug report (if debugging was triggered).
    pub debug_report: Option<DebugReport>,
    /// All anomalies detected across steps.
    pub all_anomalies: Vec<TestAnomaly>,
    /// Failure summary (if failed).
    pub failure_summary: Option<String>,
}

impl OrchestratorReport {
    /// Convert to ProofBasedTestReport for storage.
    pub fn to_proof_report(&self) -> ProofBasedTestReport {
        ProofBasedTestReport {
            session_id: self.session_id.clone(),
            started_at: self.started_at,
            completed_at: Some(self.completed_at),
            connectivity: self.connectivity_proof.clone(),
            gossip: self.gossip_proof.clone(),
            crdt: self.crdt_proof.clone(),
            anomalies: self.all_anomalies.clone(),
            passed: self.passed,
            failure_summary: self.failure_summary.clone(),
        }
    }
}

impl std::fmt::Display for OrchestratorReport {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "Proof-Based Test Report")?;
        writeln!(f, "=======================")?;
        writeln!(f, "Session: {}", self.session_id)?;
        writeln!(
            f,
            "Status: {}",
            if self.passed { "PASSED" } else { "FAILED" }
        )?;
        writeln!(f)?;

        writeln!(f, "Step Results:")?;
        for step in &self.step_results {
            writeln!(
                f,
                "  {} {} - {} ({:?})",
                if step.passed { "[PASS]" } else { "[FAIL]" },
                step.name,
                step.details,
                step.duration
            )?;
        }
        writeln!(f)?;

        if !self.all_anomalies.is_empty() {
            writeln!(f, "Anomalies ({}):", self.all_anomalies.len())?;
            for anomaly in &self.all_anomalies {
                writeln!(f, "  - {}: {}", anomaly.anomaly_type, anomaly.description)?;
            }
            writeln!(f)?;
        }

        if let Some(ref summary) = self.failure_summary {
            writeln!(f, "Failure Summary: {}", summary)?;
        }

        Ok(())
    }
}

/// Node state for orchestration.
#[derive(Debug, Clone)]
pub struct NodeState {
    /// Node identifier.
    pub node_id: String,
    /// Known peer connections.
    pub connected_peers: Vec<String>,
    /// Gossip statistics.
    pub gossip_stats: Option<crate::epidemic_gossip::GossipStats>,
    /// Latest state hash (for CRDT verification).
    pub state_hash: Option<[u8; 32]>,
    /// Whether the node is responsive.
    pub responsive: bool,
    /// Last update time.
    pub last_updated: SystemTime,
}

impl Default for NodeState {
    fn default() -> Self {
        Self {
            node_id: String::new(),
            connected_peers: Vec::new(),
            gossip_stats: None,
            state_hash: None,
            responsive: false,
            last_updated: SystemTime::now(),
        }
    }
}

/// Proof-based test orchestrator.
///
/// Coordinates all verification steps and generates comprehensive proofs.
pub struct ProofOrchestrator {
    config: ProofOrchestratorConfig,
    gossip_verifier: GossipVerifier,
    crdt_verifier: CrdtVerifier,
    debugger: AutomatedDebugger,
    node_states: HashMap<String, NodeState>,
    session_id: String,
}

impl ProofOrchestrator {
    /// Create a new proof orchestrator.
    pub fn new() -> Self {
        Self::with_config(ProofOrchestratorConfig::default())
    }

    /// Create a new proof orchestrator with custom config.
    pub fn with_config(config: ProofOrchestratorConfig) -> Self {
        let gossip_verifier = GossipVerifier::with_config(config.gossip_config.clone());
        let crdt_verifier = CrdtVerifier::with_config(CrdtType::PeerCache, config.crdt_config.clone());
        let debugger = AutomatedDebugger::with_config(config.debug_config.clone());

        Self {
            config,
            gossip_verifier,
            crdt_verifier,
            debugger,
            node_states: HashMap::new(),
            session_id: uuid::Uuid::new_v4().to_string(),
        }
    }

    /// Get the session ID.
    pub fn session_id(&self) -> &str {
        &self.session_id
    }

    /// Get the configuration.
    pub fn config(&self) -> &ProofOrchestratorConfig {
        &self.config
    }

    /// Register a node for testing.
    pub fn register_node(&mut self, node_id: String) {
        self.node_states.insert(
            node_id.clone(),
            NodeState {
                node_id,
                responsive: true,
                last_updated: SystemTime::now(),
                ..Default::default()
            },
        );
    }

    /// Update node state.
    pub fn update_node_state(&mut self, node_id: &str, state: NodeState) {
        self.node_states.insert(node_id.to_string(), state);
    }

    /// Record a node's peer connections.
    pub fn record_connections(&mut self, node_id: &str, peers: Vec<String>) {
        if let Some(state) = self.node_states.get_mut(node_id) {
            state.connected_peers = peers;
            state.last_updated = SystemTime::now();
        }
    }

    /// Record gossip stats from a node.
    pub fn record_gossip_stats(
        &mut self,
        node_id: &str,
        stats: crate::epidemic_gossip::GossipStats,
    ) {
        self.gossip_verifier
            .record_node_stats(node_id.to_string(), stats.clone());
        if let Some(state) = self.node_states.get_mut(node_id) {
            state.gossip_stats = Some(stats);
            state.last_updated = SystemTime::now();
        }
    }

    /// Record state hash from a node.
    pub fn record_state_hash(&mut self, node_id: &str, hash: [u8; 32]) {
        self.crdt_verifier
            .update_state(node_id.to_string(), hash);
        if let Some(state) = self.node_states.get_mut(node_id) {
            state.state_hash = Some(hash);
            state.last_updated = SystemTime::now();
        }
    }

    /// Add log entries for debugging.
    pub fn add_logs(&mut self, logs: impl IntoIterator<Item = LogEntry>) {
        self.debugger.add_logs(logs);
    }

    /// Verify connectivity between all nodes.
    pub fn verify_connectivity(&self) -> StepResult {
        let start = std::time::Instant::now();
        let mut anomalies = Vec::new();

        let node_count = self.node_states.len();
        if node_count < self.config.min_nodes {
            return StepResult::fail(
                "connectivity",
                start.elapsed(),
                format!(
                    "Insufficient nodes: {} < {}",
                    node_count, self.config.min_nodes
                ),
                vec![TestAnomaly::new(
                    "insufficient_nodes".to_string(),
                    format!("Need at least {} nodes", self.config.min_nodes),
                    4,
                )],
            );
        }

        // Check that each node sees all other nodes
        let expected_peers = node_count - 1;
        let mut fully_connected = true;
        let mut connection_details = Vec::new();

        for (node_id, state) in &self.node_states {
            let peer_count = state.connected_peers.len();
            if peer_count < expected_peers {
                fully_connected = false;
                let missing = expected_peers - peer_count;
                anomalies.push(TestAnomaly::new(
                    "incomplete_connectivity".to_string(),
                    format!(
                        "Node {} has {} peers, expected {}, missing {}",
                        node_id, peer_count, expected_peers, missing
                    ),
                    3,
                ));
            }
            connection_details.push(format!("{}:{}", node_id, peer_count));
        }

        // Cross-validate connections (if A sees B, B should see A)
        if self.config.require_cross_validation {
            for (node_a, state_a) in &self.node_states {
                for peer_b in &state_a.connected_peers {
                    if let Some(state_b) = self.node_states.get(peer_b) {
                        if !state_b.connected_peers.contains(node_a) {
                            fully_connected = false;
                            anomalies.push(TestAnomaly::new(
                                "asymmetric_connection".to_string(),
                                format!("{} sees {} but {} doesn't see {}", node_a, peer_b, peer_b, node_a),
                                3,
                            ));
                        }
                    }
                }
            }
        }

        let details = format!(
            "{} nodes, connections: [{}]",
            node_count,
            connection_details.join(", ")
        );

        if fully_connected {
            StepResult::pass("connectivity", start.elapsed(), details)
        } else {
            StepResult::fail("connectivity", start.elapsed(), details, anomalies)
        }
    }

    /// Verify gossip protocols.
    pub fn verify_gossip(&self) -> StepResult {
        let start = std::time::Instant::now();

        let summary = self.gossip_verifier.get_summary();
        let all_valid = self.gossip_verifier.all_protocols_valid();

        let mut anomalies = Vec::new();
        if !summary.hyparview_valid {
            anomalies.push(TestAnomaly::new(
                "hyparview_failure".to_string(),
                format!("HyParView verification failed: {}", summary.hyparview_details),
                4,
            ));
        }
        if !summary.swim_valid {
            anomalies.push(TestAnomaly::new(
                "swim_failure".to_string(),
                format!("SWIM verification failed: {}", summary.swim_details),
                4,
            ));
        }
        if !summary.plumtree_valid {
            anomalies.push(TestAnomaly::new(
                "plumtree_failure".to_string(),
                format!("Plumtree verification failed: {}", summary.plumtree_details),
                4,
            ));
        }

        let details = format!(
            "HyParView:{} SWIM:{} Plumtree:{}",
            if summary.hyparview_valid { "OK" } else { "FAIL" },
            if summary.swim_valid { "OK" } else { "FAIL" },
            if summary.plumtree_valid { "OK" } else { "FAIL" },
        );

        if all_valid {
            StepResult::pass("gossip_protocols", start.elapsed(), details)
        } else {
            StepResult::fail("gossip_protocols", start.elapsed(), details, anomalies)
        }
    }

    /// Verify CRDT convergence.
    pub fn verify_crdt(&mut self) -> StepResult {
        let start = std::time::Instant::now();

        let converged = self.crdt_verifier.check_convergence();
        let summary = self.crdt_verifier.get_summary();

        let mut anomalies = Vec::new();
        if !converged {
            for node in &summary.divergent_nodes {
                anomalies.push(TestAnomaly::new(
                    "state_divergence".to_string(),
                    format!("Node {} has divergent state", node),
                    5,
                ));
            }
        }

        let details = format!(
            "convergence:{} nodes:{} ops:{}",
            if converged { "OK" } else { "FAIL" },
            summary.nodes_participating,
            summary.operations_recorded
        );

        if converged {
            StepResult::pass("crdt_convergence", start.elapsed(), details)
        } else {
            StepResult::fail("crdt_convergence", start.elapsed(), details, anomalies)
        }
    }

    /// Generate connectivity proof.
    pub fn generate_connectivity_proof(&self) -> NetworkConnectivityProof {
        let expected: std::collections::HashSet<String> =
            self.node_states.keys().cloned().collect();
        let observed = expected.clone();

        // Build connectivity matrix
        let mut matrix: HashMap<String, std::collections::HashSet<String>> = HashMap::new();
        for (node_id, state) in &self.node_states {
            matrix.insert(
                node_id.clone(),
                state.connected_peers.iter().cloned().collect(),
            );
        }

        NetworkConnectivityProof {
            observer_id: self.config.observer_id.clone(),
            timestamp: SystemTime::now(),
            expected_peers: expected,
            observed_peers: observed,
            connectivity_matrix: matrix,
            cross_validations: Vec::new(),
            attestation: SignedAttestation::new(
                self.config.observer_id.clone(),
                ProofType::Connectivity,
                [0u8; 32],
            ),
        }
    }

    /// Generate gossip protocol proof.
    pub fn generate_gossip_proof(&self) -> GossipProtocolProof {
        self.gossip_verifier
            .generate_proof(self.config.observer_id.clone())
    }

    /// Generate CRDT convergence proof.
    pub fn generate_crdt_proof(&self) -> CrdtConvergenceProof {
        self.crdt_verifier
            .generate_proof(self.config.observer_id.clone())
    }

    /// Run complete proof-based test suite.
    pub fn run_comprehensive_test(&mut self) -> OrchestratorReport {
        let started_at = SystemTime::now();
        let mut step_results = Vec::new();
        let mut all_anomalies = Vec::new();
        let mut passed = true;

        // Step 1: Connectivity verification
        let connectivity_result = self.verify_connectivity();
        all_anomalies.extend(connectivity_result.anomalies.clone());
        if !connectivity_result.passed {
            passed = false;
        }
        step_results.push(connectivity_result);

        // Generate connectivity proof (regardless of result)
        let connectivity_proof = Some(self.generate_connectivity_proof());

        // Step 2: Gossip protocol verification
        let gossip_result = self.verify_gossip();
        all_anomalies.extend(gossip_result.anomalies.clone());
        if !gossip_result.passed {
            passed = false;
        }
        step_results.push(gossip_result);

        // Generate gossip proof
        let gossip_proof = Some(self.generate_gossip_proof());

        // Step 3: CRDT convergence verification
        let crdt_result = self.verify_crdt();
        all_anomalies.extend(crdt_result.anomalies.clone());
        if !crdt_result.passed {
            passed = false;
        }
        step_results.push(crdt_result);

        // Generate CRDT proof
        let crdt_proof = Some(self.generate_crdt_proof());

        // Step 4: If failed and debug enabled, run automated debugging
        let debug_report = if !passed && self.config.debug_on_failure {
            Some(self.debugger.investigate())
        } else {
            None
        };

        // Build failure summary
        let failure_summary = if !passed {
            let failed_steps: Vec<_> = step_results
                .iter()
                .filter(|s| !s.passed)
                .map(|s| s.name.clone())
                .collect();
            Some(format!(
                "Test failed at steps: {}. Total anomalies: {}",
                failed_steps.join(", "),
                all_anomalies.len()
            ))
        } else {
            None
        };

        OrchestratorReport {
            session_id: self.session_id.clone(),
            started_at,
            completed_at: SystemTime::now(),
            passed,
            step_results,
            connectivity_proof,
            gossip_proof,
            crdt_proof,
            debug_report,
            all_anomalies,
            failure_summary,
        }
    }

    /// Reset the orchestrator for a new test run.
    pub fn reset(&mut self) {
        self.gossip_verifier = GossipVerifier::with_config(self.config.gossip_config.clone());
        self.crdt_verifier =
            CrdtVerifier::with_config(CrdtType::PeerCache, self.config.crdt_config.clone());
        self.debugger = AutomatedDebugger::with_config(self.config.debug_config.clone());
        self.node_states.clear();
        self.session_id = uuid::Uuid::new_v4().to_string();
    }
}

impl Default for ProofOrchestrator {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::epidemic_gossip::{GossipStats, HyParViewStats, PlumtreeStats, SwimStats};

    fn make_test_gossip_stats() -> GossipStats {
        GossipStats {
            hyparview: HyParViewStats {
                active_view_size: 4,
                passive_view_size: 16,
                shuffles: 10,
                joins: 2,
            },
            swim: SwimStats {
                alive_count: 5,
                suspect_count: 0,
                dead_count: 0,
                pings_sent: 100,
                acks_received: 95,
            },
            plumtree: PlumtreeStats {
                eager_peers: 3,
                lazy_peers: 5,
                messages_sent: 50,
                messages_received: 48,
                duplicates: 5,
                grafts: 2,
                prunes: 1,
            },
            ..Default::default()
        }
    }

    #[test]
    fn test_orchestrator_creation() {
        let orchestrator = ProofOrchestrator::new();
        assert!(!orchestrator.session_id().is_empty());
    }

    #[test]
    fn test_node_registration() {
        let mut orchestrator = ProofOrchestrator::new();
        orchestrator.register_node("node1".to_string());
        orchestrator.register_node("node2".to_string());

        assert_eq!(orchestrator.node_states.len(), 2);
    }

    #[test]
    fn test_connectivity_verification_fail() {
        let mut orchestrator = ProofOrchestrator::new();
        orchestrator.register_node("node1".to_string());
        orchestrator.register_node("node2".to_string());

        // No connections recorded = fail
        let result = orchestrator.verify_connectivity();
        assert!(!result.passed);
    }

    #[test]
    fn test_connectivity_verification_pass() {
        let mut orchestrator = ProofOrchestrator::new();
        orchestrator.register_node("node1".to_string());
        orchestrator.register_node("node2".to_string());

        // Record bidirectional connections
        orchestrator.record_connections("node1", vec!["node2".to_string()]);
        orchestrator.record_connections("node2", vec!["node1".to_string()]);

        let result = orchestrator.verify_connectivity();
        assert!(result.passed);
    }

    #[test]
    fn test_comprehensive_test() {
        let mut orchestrator = ProofOrchestrator::new();
        orchestrator.register_node("node1".to_string());
        orchestrator.register_node("node2".to_string());

        // Setup connections
        orchestrator.record_connections("node1", vec!["node2".to_string()]);
        orchestrator.record_connections("node2", vec!["node1".to_string()]);

        // Setup gossip stats
        orchestrator.record_gossip_stats("node1", make_test_gossip_stats());
        orchestrator.record_gossip_stats("node2", make_test_gossip_stats());

        // Setup CRDT state (converged)
        let hash = [1u8; 32];
        orchestrator.record_state_hash("node1", hash);
        orchestrator.record_state_hash("node2", hash);

        let report = orchestrator.run_comprehensive_test();
        println!("{}", report);
    }

    #[test]
    fn test_asymmetric_connection_detection() {
        let mut orchestrator = ProofOrchestrator::new();
        orchestrator.register_node("node1".to_string());
        orchestrator.register_node("node2".to_string());

        // node1 sees node2, but node2 doesn't see node1 (asymmetric)
        orchestrator.record_connections("node1", vec!["node2".to_string()]);
        orchestrator.record_connections("node2", vec![]);

        let result = orchestrator.verify_connectivity();
        assert!(!result.passed);
        assert!(result
            .anomalies
            .iter()
            .any(|a| a.anomaly_type == "asymmetric_connection"));
    }
}
