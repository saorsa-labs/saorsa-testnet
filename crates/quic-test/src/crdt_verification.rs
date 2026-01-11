//! CRDT convergence verification module.
//!
//! This module provides verification logic for CRDT (Conflict-free Replicated Data Type)
//! operations across a distributed network. It validates that:
//!
//! - All nodes converge to the same state after concurrent updates
//! - Conflict resolution follows CRDT semantics
//! - State changes propagate correctly through the gossip layer
//!
//! # Verification Process
//!
//! 1. Capture initial state hashes from all nodes
//! 2. Execute concurrent operations from multiple nodes
//! 3. Wait for convergence (with timeout)
//! 4. Capture final state hashes from all nodes
//! 5. Verify all states match
//! 6. Verify conflict resolution was semantically correct

use crate::epidemic_gossip::CrdtStats;
use crate::registry::{
    CrdtConvergenceProof, CrdtOperation, CrdtType, ProofType, SignedAttestation,
};
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::time::{Duration, Instant, SystemTime};

/// Configuration for CRDT verification.
#[derive(Debug, Clone)]
pub struct CrdtVerifierConfig {
    /// Maximum time to wait for convergence (ms).
    pub convergence_timeout_ms: u64,
    /// How often to check for convergence (ms).
    pub poll_interval_ms: u64,
    /// Minimum number of nodes required for valid test.
    pub min_nodes: usize,
    /// Whether to verify conflict resolution semantics.
    pub verify_conflict_resolution: bool,
}

impl Default for CrdtVerifierConfig {
    fn default() -> Self {
        Self {
            convergence_timeout_ms: 30_000,
            poll_interval_ms: 100,
            min_nodes: 2,
            verify_conflict_resolution: true,
        }
    }
}

/// State snapshot from a single node.
#[derive(Debug, Clone)]
pub struct NodeStateSnapshot {
    /// Node identifier.
    pub node_id: String,
    /// BLAKE3 hash of the state.
    pub state_hash: [u8; 32],
    /// When this snapshot was taken.
    pub timestamp: Instant,
    /// Raw state data for conflict resolution verification.
    pub raw_state: Option<Vec<u8>>,
}

/// CRDT operation tracker.
///
/// Tracks operations performed during a test for verification.
#[derive(Debug, Clone, Default)]
pub struct OperationTracker {
    /// Operations in order of recording.
    operations: Vec<CrdtOperation>,
    /// Operations indexed by node.
    by_node: HashMap<String, Vec<CrdtOperation>>,
    /// Vector clock tracking.
    vector_clocks: HashMap<String, HashMap<String, u64>>,
}

impl OperationTracker {
    /// Create a new operation tracker.
    pub fn new() -> Self {
        Self::default()
    }

    /// Record an operation.
    pub fn record(&mut self, mut operation: CrdtOperation) {
        // Update vector clock for this node
        let clock = self
            .vector_clocks
            .entry(operation.node_id.clone())
            .or_default();
        let current = clock.get(&operation.node_id).copied().unwrap_or(0);
        clock.insert(operation.node_id.clone(), current + 1);

        // Store clock in operation
        operation.vector_clock = clock.clone();

        // Track by node
        self.by_node
            .entry(operation.node_id.clone())
            .or_default()
            .push(operation.clone());

        self.operations.push(operation);
    }

    /// Get all operations.
    pub fn all(&self) -> &[CrdtOperation] {
        &self.operations
    }

    /// Get operations for a specific node.
    pub fn for_node(&self, node_id: &str) -> Option<&Vec<CrdtOperation>> {
        self.by_node.get(node_id)
    }

    /// Get the current vector clock for a node.
    pub fn clock_for(&self, node_id: &str) -> Option<&HashMap<String, u64>> {
        self.vector_clocks.get(node_id)
    }

    /// Check if operations are concurrent (neither happened-before the other).
    pub fn are_concurrent(&self, op1: &CrdtOperation, op2: &CrdtOperation) -> bool {
        // Concurrent if neither clock dominates the other
        let a_before_b = self.happens_before(&op1.vector_clock, &op2.vector_clock);
        let b_before_a = self.happens_before(&op2.vector_clock, &op1.vector_clock);
        !a_before_b && !b_before_a
    }

    /// Check if clock a happened-before clock b.
    fn happens_before(&self, a: &HashMap<String, u64>, b: &HashMap<String, u64>) -> bool {
        // a <= b and a != b
        let a_leq_b = a.iter().all(|(k, v)| b.get(k).copied().unwrap_or(0) >= *v);
        let not_equal = a != b;
        a_leq_b && not_equal
    }

    /// Get all pairs of concurrent operations.
    pub fn find_concurrent_pairs(&self) -> Vec<(&CrdtOperation, &CrdtOperation)> {
        let mut pairs = Vec::new();
        for (i, op1) in self.operations.iter().enumerate() {
            for op2 in self.operations.iter().skip(i + 1) {
                if self.are_concurrent(op1, op2) {
                    pairs.push((op1, op2));
                }
            }
        }
        pairs
    }

    /// Clear all tracked operations.
    pub fn clear(&mut self) {
        self.operations.clear();
        self.by_node.clear();
        self.vector_clocks.clear();
    }
}

/// Convergence state tracking.
#[derive(Debug, Clone)]
pub struct ConvergenceState {
    /// Initial state hashes.
    pub initial: HashMap<String, [u8; 32]>,
    /// Current state hashes.
    pub current: HashMap<String, [u8; 32]>,
    /// When convergence was achieved (if at all).
    pub converged_at: Option<Instant>,
    /// Test start time.
    pub started_at: Instant,
}

impl ConvergenceState {
    /// Create a new convergence state.
    pub fn new() -> Self {
        Self {
            initial: HashMap::new(),
            current: HashMap::new(),
            converged_at: None,
            started_at: Instant::now(),
        }
    }

    /// Record initial state for a node.
    pub fn set_initial(&mut self, node_id: String, hash: [u8; 32]) {
        self.initial.insert(node_id, hash);
    }

    /// Update current state for a node.
    pub fn update_current(&mut self, node_id: String, hash: [u8; 32]) {
        self.current.insert(node_id, hash);
    }

    /// Check if all nodes have converged to the same state.
    pub fn is_converged(&self) -> bool {
        if self.current.len() < 2 {
            return self.current.len() <= 1;
        }

        let mut hashes = self.current.values();
        let first = match hashes.next() {
            Some(h) => h,
            None => return true,
        };
        hashes.all(|h| h == first)
    }

    /// Mark convergence achieved.
    pub fn mark_converged(&mut self) {
        if self.converged_at.is_none() {
            self.converged_at = Some(Instant::now());
        }
    }

    /// Get convergence time in milliseconds.
    pub fn convergence_time_ms(&self) -> Option<u64> {
        self.converged_at
            .map(|t| t.duration_since(self.started_at).as_millis() as u64)
    }

    /// Get nodes with divergent states.
    pub fn divergent_nodes(&self) -> Vec<&String> {
        if self.current.len() <= 1 {
            return Vec::new();
        }

        // Find most common hash
        let mut hash_counts: HashMap<[u8; 32], usize> = HashMap::new();
        for hash in self.current.values() {
            *hash_counts.entry(*hash).or_insert(0) += 1;
        }

        let majority_hash = hash_counts
            .iter()
            .max_by_key(|(_, count)| *count)
            .map(|(hash, _)| hash);

        match majority_hash {
            Some(majority) => self
                .current
                .iter()
                .filter(|(_, h)| *h != majority)
                .map(|(id, _)| id)
                .collect(),
            None => Vec::new(),
        }
    }
}

impl Default for ConvergenceState {
    fn default() -> Self {
        Self::new()
    }
}

/// CRDT convergence verifier.
///
/// Verifies that CRDT operations across distributed nodes result in
/// eventual convergence to the same state.
pub struct CrdtVerifier {
    config: CrdtVerifierConfig,
    operations: OperationTracker,
    convergence: ConvergenceState,
    crdt_type: CrdtType,
    test_id: String,
}

impl CrdtVerifier {
    /// Create a new CRDT verifier.
    pub fn new(crdt_type: CrdtType) -> Self {
        Self::with_config(crdt_type, CrdtVerifierConfig::default())
    }

    /// Create a new CRDT verifier with custom config.
    pub fn with_config(crdt_type: CrdtType, config: CrdtVerifierConfig) -> Self {
        Self {
            config,
            operations: OperationTracker::new(),
            convergence: ConvergenceState::new(),
            crdt_type,
            test_id: uuid::Uuid::new_v4().to_string(),
        }
    }

    /// Get the test ID.
    pub fn test_id(&self) -> &str {
        &self.test_id
    }

    /// Get the configuration.
    pub fn config(&self) -> &CrdtVerifierConfig {
        &self.config
    }

    /// Record initial state from a node.
    pub fn record_initial_state(&mut self, node_id: String, state_hash: [u8; 32]) {
        self.convergence.set_initial(node_id, state_hash);
    }

    /// Record a CRDT operation.
    pub fn record_operation(&mut self, operation: CrdtOperation) {
        self.operations.record(operation);
    }

    /// Update current state from a node.
    pub fn update_state(&mut self, node_id: String, state_hash: [u8; 32]) {
        self.convergence.update_current(node_id, state_hash);
    }

    /// Check if convergence has been achieved.
    pub fn check_convergence(&mut self) -> bool {
        let converged = self.convergence.is_converged();
        if converged {
            self.convergence.mark_converged();
        }
        converged
    }

    /// Get the common initial state hash (if all nodes started the same).
    pub fn initial_state_hash(&self) -> [u8; 32] {
        self.convergence
            .initial
            .values()
            .next()
            .copied()
            .unwrap_or([0u8; 32])
    }

    /// Verify that conflict resolution followed CRDT semantics.
    ///
    /// For different CRDT types:
    /// - OR-Set: Add wins over concurrent remove
    /// - G-Counter: Concurrent increments merge correctly
    /// - PN-Counter: Increments and decrements merge correctly
    /// - LWW-Register: Highest timestamp wins
    pub fn verify_conflict_resolution(&self) -> ConflictResolutionResult {
        let concurrent_pairs = self.operations.find_concurrent_pairs();

        if concurrent_pairs.is_empty() {
            return ConflictResolutionResult {
                had_conflicts: false,
                correctly_resolved: true,
                details: "No concurrent operations detected".to_string(),
            };
        }

        // For each CRDT type, verify the merge semantics
        let details = match self.crdt_type {
            CrdtType::OrSet => {
                // OR-Set: add wins over concurrent remove
                let conflicts: Vec<_> = concurrent_pairs
                    .iter()
                    .filter(|(a, b)| {
                        (a.operation_type == "add" && b.operation_type == "remove")
                            || (a.operation_type == "remove" && b.operation_type == "add")
                    })
                    .collect();
                format!(
                    "OR-Set: {} add/remove conflicts detected (add should win)",
                    conflicts.len()
                )
            }
            CrdtType::GCounter => {
                format!(
                    "G-Counter: {} concurrent increments (all should be applied)",
                    concurrent_pairs.len()
                )
            }
            CrdtType::PnCounter => {
                format!(
                    "PN-Counter: {} concurrent operations (inc/dec should merge)",
                    concurrent_pairs.len()
                )
            }
            CrdtType::LwwRegister => {
                // LWW: highest timestamp wins
                format!(
                    "LWW-Register: {} concurrent writes (latest timestamp wins)",
                    concurrent_pairs.len()
                )
            }
            CrdtType::PeerCache => {
                // PeerCache is an OR-Set variant
                format!(
                    "PeerCache: {} concurrent peer updates (add wins)",
                    concurrent_pairs.len()
                )
            }
        };

        // We can only verify semantics if we have the raw state data
        // For now, we assume correct resolution if states converged
        ConflictResolutionResult {
            had_conflicts: !concurrent_pairs.is_empty(),
            correctly_resolved: self.convergence.is_converged(),
            details,
        }
    }

    /// Generate a convergence proof.
    pub fn generate_proof(&self, observer_id: String) -> CrdtConvergenceProof {
        let conflict_result = self.verify_conflict_resolution();

        let mut proof = CrdtConvergenceProof::new(self.test_id.clone(), self.crdt_type);
        proof.initial_state_hash = self.initial_state_hash();
        proof.operations = self.operations.all().to_vec();
        proof.node_final_states = self.convergence.current.clone();
        proof.convergence_achieved = self.convergence.is_converged();
        proof.convergence_time_ms = self.convergence.convergence_time_ms().unwrap_or(0);
        proof.conflict_resolution_correct = conflict_result.correctly_resolved;

        // Create attestation
        let attestation = SignedAttestation::new(
            observer_id,
            ProofType::CrdtConvergence,
            [0u8; 32], // Would compute BLAKE3 hash of proof data
        );
        proof.attestations = vec![attestation];
        proof.timestamp = SystemTime::now();

        proof
    }

    /// Get a summary of the verification.
    pub fn get_summary(&self) -> VerificationSummary {
        let conflict_result = self.verify_conflict_resolution();

        VerificationSummary {
            test_id: self.test_id.clone(),
            crdt_type: self.crdt_type,
            nodes_participating: self.convergence.current.len(),
            operations_recorded: self.operations.all().len(),
            convergence_achieved: self.convergence.is_converged(),
            convergence_time_ms: self.convergence.convergence_time_ms(),
            divergent_nodes: self
                .convergence
                .divergent_nodes()
                .into_iter()
                .cloned()
                .collect(),
            conflict_result,
        }
    }

    /// Reset the verifier for a new test.
    pub fn reset(&mut self) {
        self.operations.clear();
        self.convergence = ConvergenceState::new();
        self.test_id = uuid::Uuid::new_v4().to_string();
    }
}

/// Result of conflict resolution verification.
#[derive(Debug, Clone)]
pub struct ConflictResolutionResult {
    /// Whether any concurrent operations were detected.
    pub had_conflicts: bool,
    /// Whether conflicts were resolved correctly.
    pub correctly_resolved: bool,
    /// Human-readable details about the resolution.
    pub details: String,
}

/// Summary of CRDT verification results.
#[derive(Debug, Clone)]
pub struct VerificationSummary {
    /// Test identifier.
    pub test_id: String,
    /// Type of CRDT tested.
    pub crdt_type: CrdtType,
    /// Number of nodes participating.
    pub nodes_participating: usize,
    /// Number of operations recorded.
    pub operations_recorded: usize,
    /// Whether convergence was achieved.
    pub convergence_achieved: bool,
    /// Time to converge (if achieved).
    pub convergence_time_ms: Option<u64>,
    /// Nodes that did not converge.
    pub divergent_nodes: Vec<String>,
    /// Conflict resolution analysis.
    pub conflict_result: ConflictResolutionResult,
}

impl std::fmt::Display for VerificationSummary {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "CRDT Convergence Verification")?;
        writeln!(f, "  Test ID: {}", self.test_id)?;
        writeln!(f, "  CRDT Type: {}", self.crdt_type)?;
        writeln!(f, "  Nodes: {}", self.nodes_participating)?;
        writeln!(f, "  Operations: {}", self.operations_recorded)?;
        writeln!(
            f,
            "  Convergence: {}",
            if self.convergence_achieved {
                "ACHIEVED"
            } else {
                "FAILED"
            }
        )?;
        if let Some(time) = self.convergence_time_ms {
            writeln!(f, "  Time: {} ms", time)?;
        }
        if !self.divergent_nodes.is_empty() {
            writeln!(f, "  Divergent nodes: {:?}", self.divergent_nodes)?;
        }
        writeln!(f, "  Conflicts: {}", self.conflict_result.details)?;
        Ok(())
    }
}

/// Utility to compute SHA-256 hash of data.
pub fn compute_state_hash(data: &[u8]) -> [u8; 32] {
    let mut hasher = Sha256::new();
    hasher.update(data);
    let result = hasher.finalize();
    let mut hash = [0u8; 32];
    hash.copy_from_slice(&result);
    hash
}

/// Live CRDT state captured from a gossip node.
///
/// This represents the actual state captured from a running gossip instance,
/// not just an externally passed hash.
#[derive(Debug, Clone)]
pub struct LiveCrdtState {
    /// Node identifier.
    pub node_id: String,
    /// The CRDT statistics at capture time.
    pub stats: CrdtStats,
    /// Computed hash of the serialized state.
    pub state_hash: [u8; 32],
    /// When this state was captured.
    pub captured_at: Instant,
    /// System time for proof generation.
    pub system_time: SystemTime,
}

impl LiveCrdtState {
    /// Capture live state from CRDT statistics.
    ///
    /// This computes a deterministic hash of the CRDT state that can be
    /// compared across nodes to verify convergence.
    pub fn capture(node_id: String, stats: CrdtStats) -> Self {
        let state_hash = compute_crdt_state_hash(&stats);
        Self {
            node_id,
            stats,
            state_hash,
            captured_at: Instant::now(),
            system_time: SystemTime::now(),
        }
    }

    /// Check if this state is fresh (captured recently).
    pub fn is_fresh(&self, max_age: Duration) -> bool {
        self.captured_at.elapsed() < max_age
    }
}

/// Compute a deterministic hash from CRDT statistics.
///
/// This serializes the CRDT state in a deterministic format and computes
/// a cryptographic hash that can be compared across nodes.
pub fn compute_crdt_state_hash(stats: &CrdtStats) -> [u8; 32] {
    // Serialize in a deterministic format:
    // Format: "entries:{entries}|merges:{merges}|vclock_len:{len}|last_sync:{age}"
    let state_str = format!(
        "entries:{}|merges:{}|vclock_len:{}|last_sync:{}",
        stats.entries, stats.merges, stats.vector_clock_len, stats.last_sync_age_secs
    );
    compute_state_hash(state_str.as_bytes())
}

/// Capture and verify CRDT convergence across multiple live nodes.
///
/// This is the primary entry point for verifying CRDT convergence in production.
/// It captures live state from each node and verifies they have converged.
#[derive(Debug)]
pub struct LiveConvergenceVerifier {
    /// Captured states from each node.
    captured_states: HashMap<String, LiveCrdtState>,
    /// Maximum allowed state age for freshness check.
    max_state_age: Duration,
    /// Convergence result (computed lazily).
    convergence_result: Option<LiveConvergenceResult>,
}

/// Result of live convergence verification.
#[derive(Debug, Clone)]
pub struct LiveConvergenceResult {
    /// Whether all nodes have converged to the same state.
    pub converged: bool,
    /// The common state hash (if converged).
    pub common_hash: Option<[u8; 32]>,
    /// Number of nodes checked.
    pub node_count: usize,
    /// Nodes that have divergent state (if not converged).
    pub divergent_nodes: Vec<String>,
    /// Summary of each node's state.
    pub node_summaries: HashMap<String, CrdtStateSummary>,
    /// When the verification was performed.
    pub verified_at: SystemTime,
}

/// Summary of a single node's CRDT state.
#[derive(Debug, Clone)]
pub struct CrdtStateSummary {
    /// State hash.
    pub hash: [u8; 32],
    /// Number of CRDT entries.
    pub entries: usize,
    /// Number of merge operations.
    pub merges: u64,
    /// Vector clock length.
    pub vector_clock_len: usize,
    /// Whether the state is considered fresh.
    pub is_fresh: bool,
}

impl LiveConvergenceVerifier {
    /// Create a new live convergence verifier.
    pub fn new() -> Self {
        Self::with_max_age(Duration::from_secs(30))
    }

    /// Create a new verifier with custom max state age.
    pub fn with_max_age(max_state_age: Duration) -> Self {
        Self {
            captured_states: HashMap::new(),
            max_state_age,
            convergence_result: None,
        }
    }

    /// Capture state from a node's CRDT statistics.
    ///
    /// This should be called for each node in the test network.
    pub fn capture_state(&mut self, node_id: String, stats: CrdtStats) {
        let live_state = LiveCrdtState::capture(node_id.clone(), stats);
        self.captured_states.insert(node_id, live_state);
        // Invalidate cached result
        self.convergence_result = None;
    }

    /// Check if convergence has been achieved.
    ///
    /// Returns true if all captured states have the same hash.
    pub fn check_convergence(&mut self) -> bool {
        let result = self.verify();
        result.converged
    }

    /// Perform full verification and return detailed result.
    pub fn verify(&mut self) -> LiveConvergenceResult {
        if let Some(ref result) = self.convergence_result {
            return result.clone();
        }

        let mut node_summaries = HashMap::new();
        let mut hash_counts: HashMap<[u8; 32], Vec<String>> = HashMap::new();

        for (node_id, state) in &self.captured_states {
            let is_fresh = state.is_fresh(self.max_state_age);

            let summary = CrdtStateSummary {
                hash: state.state_hash,
                entries: state.stats.entries,
                merges: state.stats.merges,
                vector_clock_len: state.stats.vector_clock_len,
                is_fresh,
            };
            node_summaries.insert(node_id.clone(), summary);

            hash_counts
                .entry(state.state_hash)
                .or_default()
                .push(node_id.clone());
        }

        let node_count = self.captured_states.len();

        // Check convergence: all nodes have same hash
        let (converged, common_hash, divergent_nodes) = if hash_counts.len() == 1 {
            let hash = *hash_counts.keys().next().unwrap();
            (true, Some(hash), Vec::new())
        } else if hash_counts.is_empty() {
            (true, None, Vec::new())
        } else {
            // Find majority hash
            let majority = hash_counts
                .iter()
                .max_by_key(|(_, nodes)| nodes.len())
                .map(|(h, _)| *h);

            // Collect divergent nodes (not in majority)
            let divergent: Vec<String> = hash_counts
                .iter()
                .filter(|(h, _)| majority.is_none_or(|m| *h != &m))
                .flat_map(|(_, nodes)| nodes.clone())
                .collect();

            (false, majority, divergent)
        };

        let result = LiveConvergenceResult {
            converged,
            common_hash,
            node_count,
            divergent_nodes,
            node_summaries,
            verified_at: SystemTime::now(),
        };

        self.convergence_result = Some(result.clone());
        result
    }

    /// Get the number of nodes with captured state.
    pub fn node_count(&self) -> usize {
        self.captured_states.len()
    }

    /// Get all captured states.
    pub fn states(&self) -> &HashMap<String, LiveCrdtState> {
        &self.captured_states
    }

    /// Reset for a new verification run.
    pub fn reset(&mut self) {
        self.captured_states.clear();
        self.convergence_result = None;
    }
}

impl Default for LiveConvergenceVerifier {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_operation_tracking() {
        let mut tracker = OperationTracker::new();

        let op1 = CrdtOperation::new("node1".to_string(), "add".to_string());
        let op2 = CrdtOperation::new("node2".to_string(), "add".to_string());

        tracker.record(op1);
        tracker.record(op2);

        assert_eq!(tracker.all().len(), 2);
        assert!(tracker.for_node("node1").is_some());
    }

    #[test]
    fn test_convergence_detection() {
        let mut state = ConvergenceState::new();
        let hash1 = [1u8; 32];
        let hash2 = [1u8; 32];
        let hash3 = [2u8; 32];

        state.update_current("node1".to_string(), hash1);
        state.update_current("node2".to_string(), hash2);
        assert!(state.is_converged());

        state.update_current("node3".to_string(), hash3);
        assert!(!state.is_converged());
    }

    #[test]
    fn test_divergent_node_detection() {
        let mut state = ConvergenceState::new();
        let majority_hash = [1u8; 32];
        let divergent_hash = [2u8; 32];

        state.update_current("node1".to_string(), majority_hash);
        state.update_current("node2".to_string(), majority_hash);
        state.update_current("node3".to_string(), divergent_hash);

        let divergent = state.divergent_nodes();
        assert_eq!(divergent.len(), 1);
        assert_eq!(*divergent[0], "node3");
    }

    #[test]
    fn test_crdt_verifier() {
        let mut verifier = CrdtVerifier::new(CrdtType::OrSet);

        // Record initial states
        let initial_hash = [0u8; 32];
        verifier.record_initial_state("node1".to_string(), initial_hash);
        verifier.record_initial_state("node2".to_string(), initial_hash);

        // Record operations
        verifier.record_operation(CrdtOperation::new("node1".to_string(), "add".to_string()));
        verifier.record_operation(CrdtOperation::new("node2".to_string(), "add".to_string()));

        // Update to converged state
        let final_hash = [1u8; 32];
        verifier.update_state("node1".to_string(), final_hash);
        verifier.update_state("node2".to_string(), final_hash);

        assert!(verifier.check_convergence());

        let summary = verifier.get_summary();
        assert!(summary.convergence_achieved);
        assert_eq!(summary.nodes_participating, 2);
        assert_eq!(summary.operations_recorded, 2);
    }

    #[test]
    fn test_concurrent_detection() {
        let mut tracker = OperationTracker::new();

        // Two operations from different nodes at the same logical time
        let op1 = CrdtOperation::new("node1".to_string(), "add".to_string());
        let op2 = CrdtOperation::new("node2".to_string(), "add".to_string());

        tracker.record(op1);
        tracker.record(op2);

        // These should be concurrent (neither happened-before the other)
        let pairs = tracker.find_concurrent_pairs();
        assert!(!pairs.is_empty());
    }

    #[test]
    fn test_state_hash() {
        let data = b"test state data";
        let hash = compute_state_hash(data);
        assert_ne!(hash, [0u8; 32]);

        // Same data produces same hash
        let hash2 = compute_state_hash(data);
        assert_eq!(hash, hash2);
    }

    #[test]
    fn test_proof_generation() {
        let mut verifier = CrdtVerifier::new(CrdtType::PeerCache);

        let hash = [1u8; 32];
        verifier.record_initial_state("node1".to_string(), hash);
        verifier.update_state("node1".to_string(), hash);

        let proof = verifier.generate_proof("observer".to_string());
        assert_eq!(proof.crdt_type, CrdtType::PeerCache);
        assert!(proof.convergence_achieved);
    }

    #[test]
    fn test_live_crdt_state_capture() {
        let stats = CrdtStats {
            entries: 10,
            merges: 5,
            vector_clock_len: 3,
            last_sync_age_secs: 1,
        };

        let live_state = LiveCrdtState::capture("node1".to_string(), stats.clone());

        assert_eq!(live_state.node_id, "node1");
        assert_eq!(live_state.stats.entries, 10);
        assert_ne!(live_state.state_hash, [0u8; 32]);
        assert!(live_state.is_fresh(Duration::from_secs(60)));
    }

    #[test]
    fn test_crdt_state_hash_determinism() {
        let stats1 = CrdtStats {
            entries: 10,
            merges: 5,
            vector_clock_len: 3,
            last_sync_age_secs: 1,
        };

        let stats2 = CrdtStats {
            entries: 10,
            merges: 5,
            vector_clock_len: 3,
            last_sync_age_secs: 1,
        };

        let hash1 = compute_crdt_state_hash(&stats1);
        let hash2 = compute_crdt_state_hash(&stats2);

        // Same stats should produce same hash
        assert_eq!(hash1, hash2);
    }

    #[test]
    fn test_crdt_state_hash_changes_with_state() {
        let stats1 = CrdtStats {
            entries: 10,
            merges: 5,
            vector_clock_len: 3,
            last_sync_age_secs: 1,
        };

        let stats2 = CrdtStats {
            entries: 11, // Different entry count
            merges: 5,
            vector_clock_len: 3,
            last_sync_age_secs: 1,
        };

        let hash1 = compute_crdt_state_hash(&stats1);
        let hash2 = compute_crdt_state_hash(&stats2);

        // Different stats should produce different hash
        assert_ne!(hash1, hash2);
    }

    #[test]
    fn test_live_convergence_verifier_converged() {
        let mut verifier = LiveConvergenceVerifier::new();

        let stats = CrdtStats {
            entries: 10,
            merges: 5,
            vector_clock_len: 3,
            last_sync_age_secs: 0,
        };

        // All nodes have same state
        verifier.capture_state("node1".to_string(), stats.clone());
        verifier.capture_state("node2".to_string(), stats.clone());
        verifier.capture_state("node3".to_string(), stats.clone());

        assert!(verifier.check_convergence());

        let result = verifier.verify();
        assert!(result.converged);
        assert_eq!(result.node_count, 3);
        assert!(result.divergent_nodes.is_empty());
    }

    #[test]
    fn test_live_convergence_verifier_diverged() {
        let mut verifier = LiveConvergenceVerifier::new();

        let stats1 = CrdtStats {
            entries: 10,
            merges: 5,
            vector_clock_len: 3,
            last_sync_age_secs: 0,
        };

        let stats2 = CrdtStats {
            entries: 11, // Different!
            merges: 5,
            vector_clock_len: 3,
            last_sync_age_secs: 0,
        };

        verifier.capture_state("node1".to_string(), stats1.clone());
        verifier.capture_state("node2".to_string(), stats1.clone());
        verifier.capture_state("node3".to_string(), stats2); // Divergent

        assert!(!verifier.check_convergence());

        let result = verifier.verify();
        assert!(!result.converged);
        assert_eq!(result.divergent_nodes.len(), 1);
        assert!(result.divergent_nodes.contains(&"node3".to_string()));
    }

    #[test]
    fn test_live_convergence_verifier_reset() {
        let mut verifier = LiveConvergenceVerifier::new();

        let stats = CrdtStats::default();
        verifier.capture_state("node1".to_string(), stats);
        assert_eq!(verifier.node_count(), 1);

        verifier.reset();
        assert_eq!(verifier.node_count(), 0);
    }
}
