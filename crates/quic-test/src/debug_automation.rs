//! Automated debugging module.
//!
//! When tests fail, this module automatically investigates by:
//!
//! 1. Collecting logs from all nodes
//! 2. Parsing and correlating by timestamp
//! 3. Identifying anomalies (errors, warnings, unusual patterns)
//! 4. Mapping anomalies to potential causes
//! 5. Generating fix suggestions
//!
//! # Known Patterns
//!
//! The debugger recognizes common failure patterns:
//! - "0 active peers" when nodes should have peers
//! - Address accumulation (>10 addresses per peer)
//! - Connection timeouts to specific IPs
//! - CRDT state divergence
//! - Gossip message drops

use crate::registry::TestAnomaly;
use std::collections::HashMap;
use std::time::{SystemTime, UNIX_EPOCH};

/// Configuration for automated debugging.
#[derive(Debug, Clone)]
pub struct DebuggerConfig {
    /// Maximum number of log lines to analyze.
    pub max_log_lines: usize,
    /// Time window for correlation (ms).
    pub correlation_window_ms: u64,
    /// Known error patterns to detect.
    pub error_patterns: Vec<ErrorPattern>,
    /// Minimum severity to report.
    pub min_severity: Severity,
}

impl Default for DebuggerConfig {
    fn default() -> Self {
        Self {
            max_log_lines: 10_000,
            correlation_window_ms: 5000,
            error_patterns: default_error_patterns(),
            min_severity: Severity::Warning,
        }
    }
}

/// Severity levels for anomalies.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Severity {
    /// Informational message.
    Info,
    /// Warning - potential issue.
    Warning,
    /// Error - definite problem.
    Error,
    /// Critical - system failure.
    Critical,
}

impl std::fmt::Display for Severity {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Info => write!(f, "INFO"),
            Self::Warning => write!(f, "WARN"),
            Self::Error => write!(f, "ERROR"),
            Self::Critical => write!(f, "CRITICAL"),
        }
    }
}

/// A pattern to detect in logs.
#[derive(Debug, Clone)]
pub struct ErrorPattern {
    /// Name of the pattern.
    pub name: String,
    /// Regex pattern to match.
    pub pattern: String,
    /// Severity when matched.
    pub severity: Severity,
    /// Suggested cause.
    pub suggested_cause: String,
    /// Suggested fix.
    pub suggested_fix: String,
}

impl ErrorPattern {
    /// Create a new error pattern.
    pub fn new(
        name: impl Into<String>,
        pattern: impl Into<String>,
        severity: Severity,
        cause: impl Into<String>,
        fix: impl Into<String>,
    ) -> Self {
        Self {
            name: name.into(),
            pattern: pattern.into(),
            severity,
            suggested_cause: cause.into(),
            suggested_fix: fix.into(),
        }
    }
}

/// Default error patterns to detect.
fn default_error_patterns() -> Vec<ErrorPattern> {
    vec![
        ErrorPattern::new(
            "zero_active_peers",
            "active peers|0 active|active_view_size: 0|active view: 0",
            Severity::Error,
            "HyParView not bootstrapping - no initial peers or join failed",
            "Check registry connectivity, verify peer list is non-empty",
        ),
        ErrorPattern::new(
            "address_accumulation",
            "too many addresses|address overflow|addresses accumulated",
            Severity::Warning,
            "Address accumulation - peers not pruning stale addresses",
            "Check address TTL settings, verify cleanup task is running",
        ),
        ErrorPattern::new(
            "connection_timeout",
            "connection timeout|timed out|ConnectTimeout|connect failed",
            Severity::Warning,
            "Connection timeouts - network issues or firewall blocking",
            "Check firewall rules, verify QUIC ports are open (UDP)",
        ),
        ErrorPattern::new(
            "state_divergence",
            "divergent state|state mismatch|convergence failed|not converged",
            Severity::Critical,
            "CRDT state divergence - nodes not converging",
            "Check vector clock sync, verify gossip message delivery",
        ),
        ErrorPattern::new(
            "gossip_drop",
            "message dropped|gossip failed|broadcast error|delivery failed",
            Severity::Warning,
            "Gossip messages being dropped",
            "Check message queue sizes, verify network bandwidth",
        ),
        ErrorPattern::new(
            "memory_pressure",
            "out of memory|OOM|memory exhausted|allocation failed",
            Severity::Critical,
            "Memory exhaustion - likely a leak or unbounded growth",
            "Check for unbounded collections, profile memory usage",
        ),
        ErrorPattern::new(
            "certificate_error",
            "certificate invalid|cert error|TLS failed|handshake failed",
            Severity::Error,
            "Certificate/TLS issues - likely expired or misconfigured",
            "Check certificate dates, verify crypto configuration",
        ),
        ErrorPattern::new(
            "panic",
            "panic|PANIC|panicked|unwrap failed",
            Severity::Critical,
            "Code panic - unexpected error condition",
            "Check stack trace for source location",
        ),
        ErrorPattern::new(
            "swim_false_positive",
            "false positive|incorrectly marked dead|alive but dead",
            Severity::Warning,
            "SWIM false positive - live node marked as dead",
            "Check SWIM timeout settings, reduce suspicion threshold",
        ),
        ErrorPattern::new(
            "nat_traversal_failed",
            "NAT traversal failed|hole punch failed|relay required",
            Severity::Warning,
            "NAT traversal failing - fallback to relay needed",
            "Check relay availability, verify STUN server connectivity",
        ),
    ]
}

/// A log entry parsed from a node.
#[derive(Debug, Clone)]
pub struct LogEntry {
    /// Source node.
    pub node_id: String,
    /// Log timestamp.
    pub timestamp: SystemTime,
    /// Log level.
    pub level: String,
    /// Log message.
    pub message: String,
    /// Source file (if available).
    pub source_file: Option<String>,
    /// Line number (if available).
    pub line_number: Option<u32>,
}

impl LogEntry {
    /// Create a new log entry.
    pub fn new(
        node_id: impl Into<String>,
        timestamp: SystemTime,
        level: impl Into<String>,
        message: impl Into<String>,
    ) -> Self {
        Self {
            node_id: node_id.into(),
            timestamp,
            level: level.into(),
            message: message.into(),
            source_file: None,
            line_number: None,
        }
    }

    /// Get timestamp as milliseconds since Unix epoch.
    pub fn timestamp_ms(&self) -> u64 {
        self.timestamp
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_millis() as u64)
            .unwrap_or(0)
    }
}

/// Timeline of correlated events.
#[derive(Debug, Clone, Default)]
pub struct Timeline {
    /// Events sorted by timestamp.
    events: Vec<TimelineEvent>,
    /// Events indexed by node.
    by_node: HashMap<String, Vec<usize>>,
}

impl Timeline {
    /// Create a new timeline.
    pub fn new() -> Self {
        Self::default()
    }

    /// Add an event to the timeline.
    pub fn add_event(&mut self, event: TimelineEvent) {
        let idx = self.events.len();
        let node_id = event.node_id.clone();
        self.events.push(event);
        self.by_node.entry(node_id).or_default().push(idx);
    }

    /// Get all events.
    pub fn events(&self) -> &[TimelineEvent] {
        &self.events
    }

    /// Get events for a specific node.
    pub fn events_for_node(&self, node_id: &str) -> Vec<&TimelineEvent> {
        self.by_node
            .get(node_id)
            .map(|indices| indices.iter().map(|&i| &self.events[i]).collect())
            .unwrap_or_default()
    }

    /// Get events in a time window.
    pub fn events_in_window(&self, start_ms: u64, end_ms: u64) -> Vec<&TimelineEvent> {
        self.events
            .iter()
            .filter(|e| {
                let ts = e.timestamp_ms();
                ts >= start_ms && ts <= end_ms
            })
            .collect()
    }

    /// Sort events by timestamp.
    pub fn sort_by_time(&mut self) {
        self.events.sort_by_key(|e| e.timestamp_ms());
        // Rebuild node index
        self.by_node.clear();
        for (idx, event) in self.events.iter().enumerate() {
            self.by_node
                .entry(event.node_id.clone())
                .or_default()
                .push(idx);
        }
    }
}

/// A single event in the timeline.
#[derive(Debug, Clone)]
pub struct TimelineEvent {
    /// Source node.
    pub node_id: String,
    /// Event timestamp.
    pub timestamp: SystemTime,
    /// Event type.
    pub event_type: EventType,
    /// Event description.
    pub description: String,
    /// Associated log entries.
    pub log_entries: Vec<LogEntry>,
}

impl TimelineEvent {
    /// Get timestamp as milliseconds since Unix epoch.
    pub fn timestamp_ms(&self) -> u64 {
        self.timestamp
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_millis() as u64)
            .unwrap_or(0)
    }
}

/// Types of timeline events.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EventType {
    /// Node started.
    NodeStart,
    /// Node stopped.
    NodeStop,
    /// Connection established.
    ConnectionEstablished,
    /// Connection lost.
    ConnectionLost,
    /// Error occurred.
    Error,
    /// Warning detected.
    Warning,
    /// State change.
    StateChange,
    /// Test result.
    TestResult,
}

/// An anomaly detected in the logs.
#[derive(Debug, Clone)]
pub struct Anomaly {
    /// Severity of the anomaly.
    pub severity: Severity,
    /// Pattern that matched.
    pub pattern_name: String,
    /// Node where detected.
    pub node_id: String,
    /// When detected.
    pub timestamp: SystemTime,
    /// Original log message.
    pub message: String,
    /// Suggested cause.
    pub suggested_cause: String,
    /// Suggested fix.
    pub suggested_fix: String,
    /// Related anomalies (by index).
    pub related: Vec<usize>,
}

impl Anomaly {
    /// Convert to TestAnomaly for proof reporting.
    pub fn to_test_anomaly(&self) -> TestAnomaly {
        let severity_num = match self.severity {
            Severity::Critical => 5,
            Severity::Error => 4,
            Severity::Warning => 3,
            Severity::Info => 2,
        };
        TestAnomaly {
            anomaly_type: self.pattern_name.clone(),
            description: format!(
                "[{}] {}: {}",
                self.severity, self.node_id, self.message
            ),
            severity: severity_num,
            nodes_involved: vec![self.node_id.clone()],
            detected_at: self.timestamp,
            suggested_location: None,
        }
    }
}

/// Root cause analysis result.
#[derive(Debug, Clone)]
pub struct RootCause {
    /// Most likely root cause.
    pub primary_cause: String,
    /// Confidence level (0.0-1.0).
    pub confidence: f64,
    /// Supporting evidence.
    pub evidence: Vec<String>,
    /// Alternative explanations.
    pub alternatives: Vec<(String, f64)>,
}

/// Suggested fix for an issue.
#[derive(Debug, Clone)]
pub struct SuggestedFix {
    /// Description of the fix.
    pub description: String,
    /// Priority (higher is more important).
    pub priority: u32,
    /// Affected component.
    pub component: String,
    /// Code location (if known).
    pub code_location: Option<CodeLocation>,
}

/// A location in the source code.
#[derive(Debug, Clone)]
pub struct CodeLocation {
    /// File path.
    pub file: String,
    /// Line number (if known).
    pub line: Option<u32>,
    /// Function name (if known).
    pub function: Option<String>,
}

/// Complete debug report.
#[derive(Debug, Clone)]
pub struct DebugReport {
    /// When the investigation started.
    pub started_at: SystemTime,
    /// When completed.
    pub completed_at: SystemTime,
    /// Timeline of events.
    pub timeline: Timeline,
    /// Detected anomalies.
    pub anomalies: Vec<Anomaly>,
    /// Identified root cause.
    pub root_cause: Option<RootCause>,
    /// Suggested fixes.
    pub suggested_fixes: Vec<SuggestedFix>,
    /// Summary statistics.
    pub stats: DebugStats,
}

/// Statistics from the debug investigation.
#[derive(Debug, Clone, Default)]
pub struct DebugStats {
    /// Number of log lines analyzed.
    pub log_lines_analyzed: usize,
    /// Number of nodes examined.
    pub nodes_examined: usize,
    /// Time span of logs.
    pub time_span_ms: u64,
    /// Anomalies by severity.
    pub anomalies_by_severity: HashMap<Severity, usize>,
}

/// Automated debugger.
pub struct AutomatedDebugger {
    config: DebuggerConfig,
    logs: Vec<LogEntry>,
}

impl AutomatedDebugger {
    /// Create a new automated debugger.
    pub fn new() -> Self {
        Self::with_config(DebuggerConfig::default())
    }

    /// Create a new automated debugger with custom config.
    pub fn with_config(config: DebuggerConfig) -> Self {
        Self {
            config,
            logs: Vec::new(),
        }
    }

    /// Check if a message matches a pattern (simple substring matching).
    fn matches_pattern(message: &str, pattern: &str) -> bool {
        // Simple pattern matching: supports | for OR, case-insensitive
        let message_lower = message.to_lowercase();
        let pattern_lower = pattern.to_lowercase();

        // Handle OR patterns (e.g., "error|warn")
        if pattern_lower.contains('|') {
            pattern_lower
                .split('|')
                .any(|p| message_lower.contains(p.trim()))
        } else {
            message_lower.contains(&pattern_lower)
        }
    }

    /// Get the configuration.
    pub fn config(&self) -> &DebuggerConfig {
        &self.config
    }

    /// Add log entries from a node.
    pub fn add_logs(&mut self, logs: impl IntoIterator<Item = LogEntry>) {
        for log in logs {
            if self.logs.len() < self.config.max_log_lines {
                self.logs.push(log);
            }
        }
    }

    /// Parse a raw log line.
    pub fn parse_log_line(node_id: &str, line: &str) -> Option<LogEntry> {
        // Common log format: [timestamp] [level] message
        // Or: timestamp level target: message
        let line = line.trim();
        if line.is_empty() {
            return None;
        }

        // Try to extract level
        let level = if line.contains("ERROR") || line.contains("error") {
            "ERROR"
        } else if line.contains("WARN") || line.contains("warn") {
            "WARN"
        } else if line.contains("INFO") || line.contains("info") {
            "INFO"
        } else if line.contains("DEBUG") || line.contains("debug") {
            "DEBUG"
        } else if line.contains("TRACE") || line.contains("trace") {
            "TRACE"
        } else {
            "INFO"
        };

        Some(LogEntry::new(
            node_id.to_string(),
            SystemTime::now(),
            level,
            line,
        ))
    }

    /// Build timeline from collected logs.
    pub fn build_timeline(&self) -> Timeline {
        let mut timeline = Timeline::new();

        for log in &self.logs {
            let event_type = if log.level.to_uppercase().contains("ERROR") {
                EventType::Error
            } else if log.level.to_uppercase().contains("WARN") {
                EventType::Warning
            } else if log.message.to_lowercase().contains("connected") {
                EventType::ConnectionEstablished
            } else if log.message.to_lowercase().contains("disconnected")
                || log.message.to_lowercase().contains("connection lost")
            {
                EventType::ConnectionLost
            } else if log.message.to_lowercase().contains("starting")
                || log.message.to_lowercase().contains("initialized")
            {
                EventType::NodeStart
            } else if log.message.to_lowercase().contains("stopping")
                || log.message.to_lowercase().contains("shutdown")
            {
                EventType::NodeStop
            } else {
                EventType::StateChange
            };

            timeline.add_event(TimelineEvent {
                node_id: log.node_id.clone(),
                timestamp: log.timestamp,
                event_type,
                description: log.message.clone(),
                log_entries: vec![log.clone()],
            });
        }

        timeline.sort_by_time();
        timeline
    }

    /// Detect anomalies in collected logs.
    pub fn detect_anomalies(&self) -> Vec<Anomaly> {
        let mut anomalies = Vec::new();

        for log in &self.logs {
            for error_pattern in &self.config.error_patterns {
                if Self::matches_pattern(&log.message, &error_pattern.pattern)
                    && error_pattern.severity >= self.config.min_severity
                {
                    anomalies.push(Anomaly {
                        severity: error_pattern.severity,
                        pattern_name: error_pattern.name.clone(),
                        node_id: log.node_id.clone(),
                        timestamp: log.timestamp,
                        message: log.message.clone(),
                        suggested_cause: error_pattern.suggested_cause.clone(),
                        suggested_fix: error_pattern.suggested_fix.clone(),
                        related: Vec::new(),
                    });
                }
            }
        }

        // Correlate related anomalies
        self.correlate_anomalies(&mut anomalies);

        anomalies
    }

    /// Correlate anomalies that might be related.
    fn correlate_anomalies(&self, anomalies: &mut [Anomaly]) {
        let window = self.config.correlation_window_ms;

        for i in 0..anomalies.len() {
            let ts_i = anomalies[i]
                .timestamp
                .duration_since(UNIX_EPOCH)
                .map(|d| d.as_millis() as u64)
                .unwrap_or(0);

            for j in (i + 1)..anomalies.len() {
                let ts_j = anomalies[j]
                    .timestamp
                    .duration_since(UNIX_EPOCH)
                    .map(|d| d.as_millis() as u64)
                    .unwrap_or(0);

                // If within correlation window, mark as related
                if ts_i.abs_diff(ts_j) <= window {
                    anomalies[i].related.push(j);
                    anomalies[j].related.push(i);
                }
            }
        }
    }

    /// Identify the root cause from anomalies.
    pub fn identify_root_cause(&self, anomalies: &[Anomaly]) -> Option<RootCause> {
        if anomalies.is_empty() {
            return None;
        }

        // Count anomalies by pattern
        let mut pattern_counts: HashMap<&str, usize> = HashMap::new();
        for anomaly in anomalies {
            *pattern_counts.entry(&anomaly.pattern_name).or_insert(0) += 1;
        }

        // Most frequent pattern is likely root cause
        let (primary_pattern, count) = pattern_counts
            .iter()
            .max_by_key(|(_, c)| *c)?;

        let primary_anomaly = anomalies
            .iter()
            .find(|a| a.pattern_name == *primary_pattern)?;

        // Calculate confidence based on frequency and severity
        let total_anomalies = anomalies.len();
        let frequency_score = *count as f64 / total_anomalies as f64;
        let severity_score = match primary_anomaly.severity {
            Severity::Critical => 1.0,
            Severity::Error => 0.8,
            Severity::Warning => 0.6,
            Severity::Info => 0.4,
        };
        let confidence = (frequency_score + severity_score) / 2.0;

        // Collect evidence
        let evidence: Vec<String> = anomalies
            .iter()
            .filter(|a| a.pattern_name == *primary_pattern)
            .take(5)
            .map(|a| format!("[{}] {}", a.node_id, a.message))
            .collect();

        // Alternative explanations
        let primary_name = *primary_pattern;
        let alternatives: Vec<(String, f64)> = pattern_counts
            .iter()
            .filter(|(p, _)| **p != primary_name)
            .map(|(p, c)| {
                let freq = *c as f64 / total_anomalies as f64;
                ((*p).to_string(), freq)
            })
            .collect();

        Some(RootCause {
            primary_cause: primary_anomaly.suggested_cause.clone(),
            confidence,
            evidence,
            alternatives,
        })
    }

    /// Generate fix suggestions from anomalies.
    pub fn generate_suggestions(&self, anomalies: &[Anomaly]) -> Vec<SuggestedFix> {
        let mut suggestions: Vec<SuggestedFix> = Vec::new();
        let mut seen_fixes: std::collections::HashSet<String> = std::collections::HashSet::new();

        for anomaly in anomalies {
            if !seen_fixes.contains(&anomaly.suggested_fix) {
                seen_fixes.insert(anomaly.suggested_fix.clone());

                let priority = match anomaly.severity {
                    Severity::Critical => 100,
                    Severity::Error => 75,
                    Severity::Warning => 50,
                    Severity::Info => 25,
                };

                suggestions.push(SuggestedFix {
                    description: anomaly.suggested_fix.clone(),
                    priority,
                    component: anomaly.pattern_name.clone(),
                    code_location: None,
                });
            }
        }

        suggestions.sort_by(|a, b| b.priority.cmp(&a.priority));
        suggestions
    }

    /// Run complete investigation and generate report.
    pub fn investigate(&self) -> DebugReport {
        let started_at = SystemTime::now();

        let timeline = self.build_timeline();
        let anomalies = self.detect_anomalies();
        let root_cause = self.identify_root_cause(&anomalies);
        let suggested_fixes = self.generate_suggestions(&anomalies);

        // Calculate stats
        let mut stats = DebugStats {
            log_lines_analyzed: self.logs.len(),
            nodes_examined: self.logs
                .iter()
                .map(|l| l.node_id.as_str())
                .collect::<std::collections::HashSet<_>>()
                .len(),
            time_span_ms: 0,
            anomalies_by_severity: HashMap::new(),
        };

        // Count anomalies by severity
        for anomaly in &anomalies {
            *stats
                .anomalies_by_severity
                .entry(anomaly.severity)
                .or_insert(0) += 1;
        }

        // Calculate time span
        if !self.logs.is_empty() {
            let min_ts = self.logs.iter().map(|l| l.timestamp_ms()).min().unwrap_or(0);
            let max_ts = self.logs.iter().map(|l| l.timestamp_ms()).max().unwrap_or(0);
            stats.time_span_ms = max_ts.saturating_sub(min_ts);
        }

        DebugReport {
            started_at,
            completed_at: SystemTime::now(),
            timeline,
            anomalies,
            root_cause,
            suggested_fixes,
            stats,
        }
    }

    /// Clear all collected logs.
    pub fn clear(&mut self) {
        self.logs.clear();
    }
}

impl Default for AutomatedDebugger {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Display for DebugReport {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "Debug Investigation Report")?;
        writeln!(f, "==========================")?;
        writeln!(f)?;
        writeln!(f, "Statistics:")?;
        writeln!(f, "  Log lines analyzed: {}", self.stats.log_lines_analyzed)?;
        writeln!(f, "  Nodes examined: {}", self.stats.nodes_examined)?;
        writeln!(f, "  Time span: {} ms", self.stats.time_span_ms)?;
        writeln!(f)?;

        writeln!(f, "Anomalies by Severity:")?;
        for severity in &[Severity::Critical, Severity::Error, Severity::Warning, Severity::Info] {
            let count = self.stats.anomalies_by_severity.get(severity).copied().unwrap_or(0);
            if count > 0 {
                writeln!(f, "  {}: {}", severity, count)?;
            }
        }
        writeln!(f)?;

        if let Some(ref root_cause) = self.root_cause {
            writeln!(f, "Root Cause (confidence: {:.0}%):", root_cause.confidence * 100.0)?;
            writeln!(f, "  {}", root_cause.primary_cause)?;
            writeln!(f)?;
            writeln!(f, "Evidence:")?;
            for evidence in &root_cause.evidence {
                writeln!(f, "  - {}", evidence)?;
            }
            writeln!(f)?;
        }

        if !self.suggested_fixes.is_empty() {
            writeln!(f, "Suggested Fixes (by priority):")?;
            for fix in &self.suggested_fixes {
                writeln!(f, "  [{}] {}: {}", fix.priority, fix.component, fix.description)?;
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_log_parsing() {
        let entry = AutomatedDebugger::parse_log_line(
            "node1",
            "[2024-01-15 10:30:45] ERROR connection timeout",
        );
        assert!(entry.is_some());
        let entry = entry.unwrap();
        assert_eq!(entry.level, "ERROR");
    }

    #[test]
    fn test_anomaly_detection() {
        let mut debugger = AutomatedDebugger::new();
        debugger.add_logs(vec![
            LogEntry::new("node1", SystemTime::now(), "ERROR", "active peers 0"),
            LogEntry::new("node2", SystemTime::now(), "WARN", "connection timeout"),
        ]);

        let anomalies = debugger.detect_anomalies();
        assert!(!anomalies.is_empty());
    }

    #[test]
    fn test_timeline_building() {
        let mut debugger = AutomatedDebugger::new();
        debugger.add_logs(vec![
            LogEntry::new("node1", SystemTime::now(), "INFO", "Node started"),
            LogEntry::new("node2", SystemTime::now(), "INFO", "Connected to peer"),
        ]);

        let timeline = debugger.build_timeline();
        assert_eq!(timeline.events().len(), 2);
    }

    #[test]
    fn test_full_investigation() {
        let mut debugger = AutomatedDebugger::new();
        debugger.add_logs(vec![
            LogEntry::new("node1", SystemTime::now(), "ERROR", "0 active peers"),
            LogEntry::new("node1", SystemTime::now(), "WARN", "connection timeout"),
            LogEntry::new("node2", SystemTime::now(), "ERROR", "active peers 0"),
        ]);

        let report = debugger.investigate();
        assert!(report.root_cause.is_some());
        assert!(!report.suggested_fixes.is_empty());
        println!("{}", report);
    }

    #[test]
    fn test_severity_ordering() {
        assert!(Severity::Critical > Severity::Error);
        assert!(Severity::Error > Severity::Warning);
        assert!(Severity::Warning > Severity::Info);
    }
}
