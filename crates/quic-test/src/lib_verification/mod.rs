//! External Library Verification Harness
//!
//! Tests saorsa-gossip and ant-quic APIs to confirm 100% functionality.
//! Any failures are documented as GitHub issues, NOT fixed locally.
//!
//! # Philosophy
//!
//! We act as an absolute referee and judge. Our role is to verify that
//! external libraries work correctly according to their documented APIs.
//! When they don't, we create GitHub issues - we do NOT work around bugs
//! or fix them locally.
//!
//! # Supported Libraries
//!
//! - **saorsa-gossip**: 9 crates providing epidemic gossip protocols
//!   - HyParView membership management
//!   - SWIM failure detection
//!   - Plumtree broadcast
//!   - CRDT state synchronization
//!
//! - **ant-quic**: NAT traversal and relay functionality
//!   - Direct QUIC connections
//!   - NAT traversal (hole punching)
//!   - MASQUE relay fallback
//!   - Post-quantum cryptography
//!
//! # Usage
//!
//! ```rust,ignore
//! use saorsa_quic_test::lib_verification::{verify_all_libraries, LibraryVerificationResult};
//!
//! // Run all library verification tests
//! let results = verify_all_libraries().await;
//!
//! for result in &results {
//!     println!("{}: {}/{} tests passed",
//!         result.library,
//!         result.tests_passed,
//!         result.tests_run);
//!
//!     // Report any issues found
//!     for issue in &result.issues_to_report {
//!         issue.create_github_issue().await?;
//!     }
//! }
//! ```

pub mod gossip_tests;
pub mod issue_reporter;
pub mod quic_tests;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Result of verifying an entire library
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LibraryVerificationResult {
    /// Library name (e.g., "saorsa-gossip", "ant-quic")
    pub library: String,
    /// Version string
    pub version: String,
    /// Total tests run
    pub tests_run: usize,
    /// Tests that passed
    pub tests_passed: usize,
    /// Tests that failed
    pub tests_failed: usize,
    /// Tests that produced warnings
    pub tests_warned: usize,
    /// Tests that were skipped (e.g., missing feature)
    pub tests_skipped: usize,
    /// Issues to report to GitHub
    pub issues_to_report: Vec<issue_reporter::IssueReport>,
    /// Timestamp of verification run
    pub verified_at: DateTime<Utc>,
    /// Duration of verification in milliseconds
    pub duration_ms: u64,
}

impl LibraryVerificationResult {
    /// Create a new verification result
    #[must_use]
    pub fn new(library: &str, version: &str) -> Self {
        Self {
            library: library.to_string(),
            version: version.to_string(),
            tests_run: 0,
            tests_passed: 0,
            tests_failed: 0,
            tests_warned: 0,
            tests_skipped: 0,
            issues_to_report: Vec::new(),
            verified_at: Utc::now(),
            duration_ms: 0,
        }
    }

    /// Check if all tests passed
    #[must_use]
    pub fn all_passed(&self) -> bool {
        self.tests_failed == 0 && self.tests_run > 0
    }

    /// Get pass rate as percentage
    #[must_use]
    pub fn pass_rate(&self) -> f64 {
        if self.tests_run == 0 {
            0.0
        } else {
            (self.tests_passed as f64 / self.tests_run as f64) * 100.0
        }
    }

    /// Add a test result
    pub fn add_result(&mut self, result: &TestResult) {
        self.tests_run += 1;
        match result.status {
            TestStatus::Passed => self.tests_passed += 1,
            TestStatus::Failed => {
                self.tests_failed += 1;
                if let Some(ref issue) = result.issue_to_report {
                    self.issues_to_report.push(issue.clone());
                }
            }
            TestStatus::Warning => self.tests_warned += 1,
            TestStatus::Skipped => self.tests_skipped += 1,
        }
    }
}

/// Result of a single verification test
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestResult {
    /// Test name
    pub name: String,
    /// Test status
    pub status: TestStatus,
    /// Optional message
    pub message: Option<String>,
    /// Duration in milliseconds
    pub duration_ms: u64,
    /// Issue to report (if test failed)
    pub issue_to_report: Option<issue_reporter::IssueReport>,
}

impl TestResult {
    /// Create a passing test result
    #[must_use]
    pub fn pass(name: &str) -> Self {
        Self {
            name: name.to_string(),
            status: TestStatus::Passed,
            message: None,
            duration_ms: 0,
            issue_to_report: None,
        }
    }

    /// Create a passing test result with message
    #[must_use]
    pub fn pass_with_message(name: &str, message: &str) -> Self {
        Self {
            name: name.to_string(),
            status: TestStatus::Passed,
            message: Some(message.to_string()),
            duration_ms: 0,
            issue_to_report: None,
        }
    }

    /// Create a failing test result
    #[must_use]
    pub fn fail(name: &str, message: &str, issue: issue_reporter::IssueReport) -> Self {
        Self {
            name: name.to_string(),
            status: TestStatus::Failed,
            message: Some(message.to_string()),
            duration_ms: 0,
            issue_to_report: Some(issue),
        }
    }

    /// Create a warning test result
    #[must_use]
    pub fn warn(name: &str, message: &str) -> Self {
        Self {
            name: name.to_string(),
            status: TestStatus::Warning,
            message: Some(message.to_string()),
            duration_ms: 0,
            issue_to_report: None,
        }
    }

    /// Create a skipped test result
    #[must_use]
    pub fn skip(name: &str, reason: &str) -> Self {
        Self {
            name: name.to_string(),
            status: TestStatus::Skipped,
            message: Some(reason.to_string()),
            duration_ms: 0,
            issue_to_report: None,
        }
    }

    /// Set the duration
    #[must_use]
    pub fn with_duration(mut self, duration_ms: u64) -> Self {
        self.duration_ms = duration_ms;
        self
    }
}

/// Status of a verification test
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TestStatus {
    /// Test passed completely
    Passed,
    /// Test failed - issue should be created
    Failed,
    /// Test passed with warnings
    Warning,
    /// Test was skipped
    Skipped,
}

/// Configuration for the verification harness
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VerificationConfig {
    /// Whether to run saorsa-gossip tests
    pub test_saorsa_gossip: bool,
    /// Whether to run ant-quic tests
    pub test_ant_quic: bool,
    /// Whether to automatically create GitHub issues for failures
    pub auto_create_issues: bool,
    /// Timeout for individual tests (in milliseconds)
    pub test_timeout_ms: u64,
    /// Number of nodes to use in cluster tests
    pub cluster_size: usize,
    /// Whether to run stress tests
    pub include_stress_tests: bool,
}

impl Default for VerificationConfig {
    fn default() -> Self {
        Self {
            test_saorsa_gossip: true,
            test_ant_quic: true,
            auto_create_issues: false,
            test_timeout_ms: 30_000,
            cluster_size: 5,
            include_stress_tests: false,
        }
    }
}

impl VerificationConfig {
    /// Create CI configuration (faster, less thorough)
    #[must_use]
    pub fn ci() -> Self {
        Self {
            test_saorsa_gossip: true,
            test_ant_quic: true,
            auto_create_issues: false,
            test_timeout_ms: 10_000,
            cluster_size: 3,
            include_stress_tests: false,
        }
    }

    /// Create production configuration (thorough)
    #[must_use]
    pub fn production() -> Self {
        Self {
            test_saorsa_gossip: true,
            test_ant_quic: true,
            auto_create_issues: true,
            test_timeout_ms: 60_000,
            cluster_size: 10,
            include_stress_tests: true,
        }
    }
}

/// Run all library verification tests
///
/// # Arguments
///
/// * `config` - Verification configuration
///
/// # Returns
///
/// Results for each library tested
pub async fn verify_all_libraries(config: &VerificationConfig) -> Vec<LibraryVerificationResult> {
    let mut results = Vec::new();

    if config.test_saorsa_gossip {
        results.push(gossip_tests::verify_saorsa_gossip(config).await);
    }

    if config.test_ant_quic {
        results.push(quic_tests::verify_ant_quic(config).await);
    }

    results
}

/// Print a summary report of verification results
pub fn print_summary(results: &[LibraryVerificationResult]) {
    println!("\n=== Library Verification Report ===\n");

    for result in results {
        let status_emoji = if result.all_passed() { "✓" } else { "✗" };
        println!(
            "{} {} v{}\n  Tests: {}/{} passed ({:.1}%)\n  Issues: {}",
            status_emoji,
            result.library,
            result.version,
            result.tests_passed,
            result.tests_run,
            result.pass_rate(),
            result.issues_to_report.len()
        );
    }

    let total_issues: usize = results.iter().map(|r| r.issues_to_report.len()).sum();
    if total_issues > 0 {
        println!("\n=== Issues to Report ===\n");
        for result in results {
            for issue in &result.issues_to_report {
                println!(
                    "  [{}/{}] {}\n    Labels: {}",
                    result.library,
                    issue.test_name,
                    issue.title,
                    issue.labels.join(", ")
                );
            }
        }
    }

    println!("\n=== Summary ===\n");
    let total_passed: usize = results.iter().map(|r| r.tests_passed).sum();
    let total_run: usize = results.iter().map(|r| r.tests_run).sum();
    let total_failed: usize = results.iter().map(|r| r.tests_failed).sum();

    println!("Libraries verified: {}", results.len());
    println!("Total tests: {}", total_run);
    println!("Passed: {}", total_passed);
    println!("Failed: {}", total_failed);
    println!("Issues to create: {}", total_issues);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_library_verification_result_new() {
        let result = LibraryVerificationResult::new("test-lib", "1.0.0");
        assert_eq!(result.library, "test-lib");
        assert_eq!(result.version, "1.0.0");
        assert_eq!(result.tests_run, 0);
        assert!(!result.all_passed()); // No tests run
    }

    #[test]
    fn test_library_verification_result_all_passed() {
        let mut result = LibraryVerificationResult::new("test-lib", "1.0.0");
        result.tests_run = 5;
        result.tests_passed = 5;
        assert!(result.all_passed());
    }

    #[test]
    fn test_library_verification_result_pass_rate() {
        let mut result = LibraryVerificationResult::new("test-lib", "1.0.0");
        result.tests_run = 10;
        result.tests_passed = 8;
        assert!((result.pass_rate() - 80.0).abs() < 0.01);
    }

    #[test]
    fn test_test_result_pass() {
        let result = TestResult::pass("my_test");
        assert_eq!(result.name, "my_test");
        assert_eq!(result.status, TestStatus::Passed);
        assert!(result.message.is_none());
        assert!(result.issue_to_report.is_none());
    }

    #[test]
    fn test_test_result_warn() {
        let result = TestResult::warn("my_test", "Something iffy");
        assert_eq!(result.status, TestStatus::Warning);
        assert_eq!(result.message, Some("Something iffy".to_string()));
    }

    #[test]
    fn test_test_result_skip() {
        let result = TestResult::skip("my_test", "Feature not available");
        assert_eq!(result.status, TestStatus::Skipped);
    }

    #[test]
    fn test_verification_config_default() {
        let config = VerificationConfig::default();
        assert!(config.test_saorsa_gossip);
        assert!(config.test_ant_quic);
        assert!(!config.auto_create_issues);
    }

    #[test]
    fn test_verification_config_ci() {
        let config = VerificationConfig::ci();
        assert_eq!(config.cluster_size, 3);
        assert!(!config.include_stress_tests);
    }

    #[test]
    fn test_verification_config_production() {
        let config = VerificationConfig::production();
        assert!(config.auto_create_issues);
        assert!(config.include_stress_tests);
    }
}
