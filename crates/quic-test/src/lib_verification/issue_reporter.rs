//! GitHub Issue Reporter
//!
//! Generates and creates GitHub issues for library defects found during
//! verification testing. Issues are created via the `gh` CLI tool.
//!
//! # Philosophy
//!
//! When external libraries don't work as documented, we report the issue
//! rather than working around it. This ensures:
//!
//! - Library maintainers are aware of the bug
//! - Other users can find the issue
//! - The root cause gets fixed upstream
//!
//! # Repository Mapping
//!
//! - `saorsa-gossip` -> `dirvine/saorsa-gossip`
//! - `ant-quic` -> `dirvine/ant-quic`

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Repository mapping for libraries
const LIBRARY_REPOS: &[(&str, &str)] = &[
    ("saorsa-gossip", "dirvine/saorsa-gossip"),
    ("ant-quic", "dirvine/ant-quic"),
];

/// A report for a GitHub issue to be created
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IssueReport {
    /// Library name
    pub library: String,
    /// Issue title
    pub title: String,
    /// Issue body (markdown)
    pub body: String,
    /// Labels to apply
    pub labels: Vec<String>,
    /// Test that discovered the issue
    pub test_name: String,
    /// Timestamp of discovery
    pub timestamp: DateTime<Utc>,
    /// Environment information
    pub environment: EnvironmentInfo,
}

/// Environment information for issue reports
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct EnvironmentInfo {
    /// Operating system
    pub os: String,
    /// OS version
    pub os_version: String,
    /// Rust version
    pub rust_version: String,
    /// Library version
    pub library_version: String,
    /// Git commit hash (if known)
    pub commit_hash: Option<String>,
    /// Additional context
    pub additional: HashMap<String, String>,
}

impl EnvironmentInfo {
    /// Capture current environment information
    #[must_use]
    pub fn capture() -> Self {
        Self {
            os: std::env::consts::OS.to_string(),
            os_version: get_os_version(),
            rust_version: get_rust_version(),
            library_version: String::new(),
            commit_hash: None,
            additional: HashMap::new(),
        }
    }

    /// Set the library version
    #[must_use]
    pub fn with_library_version(mut self, version: &str) -> Self {
        self.library_version = version.to_string();
        self
    }

    /// Set the commit hash
    #[must_use]
    pub fn with_commit_hash(mut self, hash: &str) -> Self {
        self.commit_hash = Some(hash.to_string());
        self
    }

    /// Add additional context
    pub fn add_context(&mut self, key: &str, value: &str) {
        self.additional.insert(key.to_string(), value.to_string());
    }
}

/// Builder for creating issue reports
#[derive(Debug, Clone, Default)]
pub struct IssueReportBuilder {
    library: String,
    title: String,
    body: String,
    labels: Vec<String>,
    test_name: String,
    expected: Option<String>,
    actual: Option<String>,
    steps_to_reproduce: Vec<String>,
    additional_notes: Option<String>,
}

impl IssueReportBuilder {
    /// Create a new issue report builder
    #[must_use]
    pub fn new(library: &str) -> Self {
        Self {
            library: library.to_string(),
            ..Default::default()
        }
    }

    /// Set the issue title
    #[must_use]
    pub fn title(mut self, title: &str) -> Self {
        self.title = title.to_string();
        self
    }

    /// Set the test name that discovered this issue
    #[must_use]
    pub fn test_name(mut self, name: &str) -> Self {
        self.test_name = name.to_string();
        self
    }

    /// Set the expected behavior
    #[must_use]
    pub fn expected(mut self, expected: &str) -> Self {
        self.expected = Some(expected.to_string());
        self
    }

    /// Set the actual behavior
    #[must_use]
    pub fn actual(mut self, actual: &str) -> Self {
        self.actual = Some(actual.to_string());
        self
    }

    /// Set the raw body (overrides expected/actual formatting)
    #[must_use]
    pub fn body(mut self, body: &str) -> Self {
        self.body = body.to_string();
        self
    }

    /// Add a label
    #[must_use]
    pub fn label(mut self, label: &str) -> Self {
        self.labels.push(label.to_string());
        self
    }

    /// Add multiple labels
    #[must_use]
    pub fn labels(mut self, labels: &[&str]) -> Self {
        self.labels.extend(labels.iter().map(|l| l.to_string()));
        self
    }

    /// Add a step to reproduce
    #[must_use]
    pub fn step(mut self, step: &str) -> Self {
        self.steps_to_reproduce.push(step.to_string());
        self
    }

    /// Add additional notes
    #[must_use]
    pub fn notes(mut self, notes: &str) -> Self {
        self.additional_notes = Some(notes.to_string());
        self
    }

    /// Build the issue report
    #[must_use]
    pub fn build(self) -> IssueReport {
        let body = if self.body.is_empty() {
            self.format_body()
        } else {
            self.body
        };

        IssueReport {
            library: self.library,
            title: self.title,
            body,
            labels: self.labels,
            test_name: self.test_name,
            timestamp: Utc::now(),
            environment: EnvironmentInfo::capture(),
        }
    }

    /// Format the issue body from builder fields
    fn format_body(&self) -> String {
        let mut parts = Vec::new();

        // Expected vs Actual
        if let Some(ref expected) = self.expected {
            parts.push(format!("**Expected behavior:**\n{}", expected));
        }
        if let Some(ref actual) = self.actual {
            parts.push(format!("**Actual behavior:**\n{}", actual));
        }

        // Steps to reproduce
        if !self.steps_to_reproduce.is_empty() {
            let steps = self
                .steps_to_reproduce
                .iter()
                .enumerate()
                .map(|(i, s)| format!("{}. {}", i + 1, s))
                .collect::<Vec<_>>()
                .join("\n");
            parts.push(format!("**Steps to reproduce:**\n{}", steps));
        }

        // Additional notes
        if let Some(ref notes) = self.additional_notes {
            parts.push(format!("**Additional notes:**\n{}", notes));
        }

        parts.join("\n\n")
    }
}

impl IssueReport {
    /// Create a new issue report builder
    #[must_use]
    pub fn builder(library: &str) -> IssueReportBuilder {
        IssueReportBuilder::new(library)
    }

    /// Get the GitHub repository for this library
    #[must_use]
    pub fn github_repo(&self) -> Option<&'static str> {
        LIBRARY_REPOS
            .iter()
            .find(|(lib, _)| *lib == self.library)
            .map(|(_, repo)| *repo)
    }

    /// Generate the full GitHub issue markdown
    #[must_use]
    pub fn to_github_markdown(&self) -> String {
        format!(
            r#"## Bug Report

**Library**: {} v{}
**Test**: {}
**Timestamp**: {}

### Environment
- OS: {} {}
- Rust: {}
- Commit: {}

### Description
{}

### Labels
{}

---
*Generated by saorsa-quic-test verification harness*
"#,
            self.library,
            self.environment.library_version,
            self.test_name,
            self.timestamp.format("%Y-%m-%d %H:%M:%S UTC"),
            self.environment.os,
            self.environment.os_version,
            self.environment.rust_version,
            self.environment
                .commit_hash
                .as_deref()
                .unwrap_or("unknown"),
            self.body,
            self.labels.join(", ")
        )
    }

    /// Create a GitHub issue via the gh CLI
    ///
    /// # Errors
    ///
    /// Returns an error if the gh CLI is not available or the issue
    /// creation fails.
    pub async fn create_github_issue(&self) -> Result<String, IssueCreationError> {
        let repo = self.github_repo().ok_or_else(|| {
            IssueCreationError::UnknownLibrary(self.library.clone())
        })?;

        // Check if gh CLI is available
        let gh_check = tokio::process::Command::new("gh")
            .arg("--version")
            .output()
            .await
            .map_err(|e| IssueCreationError::GhNotAvailable(e.to_string()))?;

        if !gh_check.status.success() {
            return Err(IssueCreationError::GhNotAvailable(
                "gh command failed".to_string(),
            ));
        }

        // Build the labels argument
        let labels_arg = if self.labels.is_empty() {
            Vec::new()
        } else {
            vec!["--label".to_string(), self.labels.join(",")]
        };

        // Create the issue
        let mut cmd = tokio::process::Command::new("gh");
        cmd.args(["issue", "create", "--repo", repo, "--title", &self.title, "--body", &self.to_github_markdown()]);

        for arg in &labels_arg {
            cmd.arg(arg);
        }

        let output = cmd.output().await.map_err(|e| {
            IssueCreationError::CommandFailed(e.to_string())
        })?;

        if output.status.success() {
            let url = String::from_utf8_lossy(&output.stdout).trim().to_string();
            Ok(url)
        } else {
            let error = String::from_utf8_lossy(&output.stderr).to_string();
            Err(IssueCreationError::CommandFailed(error))
        }
    }

    /// Check if an issue with similar title already exists
    ///
    /// # Errors
    ///
    /// Returns an error if the gh CLI is not available or the search fails.
    pub async fn check_for_duplicate(&self) -> Result<Option<String>, IssueCreationError> {
        let repo = self.github_repo().ok_or_else(|| {
            IssueCreationError::UnknownLibrary(self.library.clone())
        })?;

        let output = tokio::process::Command::new("gh")
            .args([
                "issue", "list",
                "--repo", repo,
                "--state", "all",
                "--search", &self.title,
                "--json", "url,title",
            ])
            .output()
            .await
            .map_err(|e| IssueCreationError::CommandFailed(e.to_string()))?;

        if output.status.success() {
            let stdout = String::from_utf8_lossy(&output.stdout);
            if stdout.contains(&self.title) {
                // Parse JSON to get existing issue URL
                if let Ok(issues) = serde_json::from_str::<Vec<ExistingIssue>>(&stdout) {
                    for issue in issues {
                        if issue.title.contains(&self.title) || self.title.contains(&issue.title) {
                            return Ok(Some(issue.url));
                        }
                    }
                }
            }
            Ok(None)
        } else {
            Err(IssueCreationError::CommandFailed(
                String::from_utf8_lossy(&output.stderr).to_string(),
            ))
        }
    }

    /// Create the issue only if no duplicate exists
    ///
    /// # Errors
    ///
    /// Returns an error if the duplicate check or issue creation fails.
    pub async fn create_if_new(&self) -> Result<IssueCreationResult, IssueCreationError> {
        match self.check_for_duplicate().await? {
            Some(existing_url) => Ok(IssueCreationResult::AlreadyExists(existing_url)),
            None => {
                let url = self.create_github_issue().await?;
                Ok(IssueCreationResult::Created(url))
            }
        }
    }
}

/// JSON structure for existing issues from gh CLI
#[derive(Debug, Deserialize)]
struct ExistingIssue {
    url: String,
    title: String,
}

/// Result of issue creation
#[derive(Debug, Clone)]
pub enum IssueCreationResult {
    /// Issue was created, contains URL
    Created(String),
    /// Issue already exists, contains URL of existing issue
    AlreadyExists(String),
}

/// Error during issue creation
#[derive(Debug, Clone)]
pub enum IssueCreationError {
    /// Unknown library - no repository mapping
    UnknownLibrary(String),
    /// gh CLI is not available
    GhNotAvailable(String),
    /// Command failed
    CommandFailed(String),
}

impl std::fmt::Display for IssueCreationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::UnknownLibrary(lib) => write!(f, "Unknown library: {}", lib),
            Self::GhNotAvailable(msg) => write!(f, "gh CLI not available: {}", msg),
            Self::CommandFailed(msg) => write!(f, "Command failed: {}", msg),
        }
    }
}

impl std::error::Error for IssueCreationError {}

/// Get the OS version
fn get_os_version() -> String {
    #[cfg(target_os = "macos")]
    {
        std::process::Command::new("sw_vers")
            .arg("-productVersion")
            .output()
            .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
            .unwrap_or_else(|_| "unknown".to_string())
    }
    #[cfg(target_os = "linux")]
    {
        std::fs::read_to_string("/etc/os-release")
            .ok()
            .and_then(|content| {
                content
                    .lines()
                    .find(|l| l.starts_with("PRETTY_NAME="))
                    .map(|l| l.trim_start_matches("PRETTY_NAME=").trim_matches('"').to_string())
            })
            .unwrap_or_else(|| "unknown".to_string())
    }
    #[cfg(not(any(target_os = "macos", target_os = "linux")))]
    {
        "unknown".to_string()
    }
}

/// Get the Rust version
fn get_rust_version() -> String {
    std::process::Command::new("rustc")
        .arg("--version")
        .output()
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
        .unwrap_or_else(|_| "unknown".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_issue_report_builder() {
        let report = IssueReport::builder("saorsa-gossip")
            .title("HyParView returns empty active view")
            .test_name("test_hyparview_active_view_size")
            .expected("At least 1 peer in active view")
            .actual("0 peers returned")
            .label("bug")
            .build();

        assert_eq!(report.library, "saorsa-gossip");
        assert_eq!(report.title, "HyParView returns empty active view");
        assert_eq!(report.labels, vec!["bug"]);
        assert!(report.body.contains("Expected"));
        assert!(report.body.contains("Actual"));
    }

    #[test]
    fn test_github_repo_mapping() {
        let report = IssueReport::builder("saorsa-gossip")
            .title("test")
            .build();
        assert_eq!(report.github_repo(), Some("dirvine/saorsa-gossip"));

        let report = IssueReport::builder("ant-quic")
            .title("test")
            .build();
        assert_eq!(report.github_repo(), Some("dirvine/ant-quic"));

        let report = IssueReport::builder("unknown-lib")
            .title("test")
            .build();
        assert_eq!(report.github_repo(), None);
    }

    #[test]
    fn test_github_markdown_format() {
        let report = IssueReport::builder("saorsa-gossip")
            .title("Test issue")
            .test_name("test_example")
            .body("This is a test")
            .build();

        let markdown = report.to_github_markdown();
        assert!(markdown.contains("## Bug Report"));
        assert!(markdown.contains("saorsa-gossip"));
        assert!(markdown.contains("test_example"));
        assert!(markdown.contains("This is a test"));
    }

    #[test]
    fn test_environment_info_capture() {
        let env = EnvironmentInfo::capture();
        assert!(!env.os.is_empty());
        // rust_version might be empty if rustc isn't available in test env
    }

    #[test]
    fn test_issue_report_builder_with_steps() {
        let report = IssueReport::builder("ant-quic")
            .title("NAT traversal fails")
            .step("Create two nodes behind NAT")
            .step("Attempt connection")
            .step("Observe timeout")
            .build();

        assert!(report.body.contains("Steps to reproduce"));
        assert!(report.body.contains("1. Create two nodes"));
        assert!(report.body.contains("2. Attempt connection"));
        assert!(report.body.contains("3. Observe timeout"));
    }
}
