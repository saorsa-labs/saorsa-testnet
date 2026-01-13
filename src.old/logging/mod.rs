// Copyright 2024 Saorsa Labs Limited
//
// Logging configuration and utilities for Saorsa TestNet

use anyhow::Result;
use std::path::Path;
use tracing::{info, Level};
use tracing_appender::{non_blocking, rolling};
use tracing_subscriber::{fmt, layer::SubscriberExt, util::SubscriberInitExt, EnvFilter, Layer};

/// Initialize logging system
pub fn init(verbose: bool, log_dir: &Path) -> Result<()> {
    // Create log directory if it doesn't exist
    std::fs::create_dir_all(log_dir)?;
    
    // Determine log level
    let level = if verbose {
        Level::DEBUG
    } else {
        Level::INFO
    };
    
    // Create file appender for general logs
    let file_appender = rolling::daily(log_dir, "saorsa-testnet.log");
    let (file_writer, _guard) = non_blocking(file_appender);
    
    // Create file appender for metrics
    let metrics_appender = rolling::daily(log_dir, "metrics.log");
    let (metrics_writer, _metrics_guard) = non_blocking(metrics_appender);
    
    // Create console layer
    let console_layer = fmt::layer()
        .with_target(false)
        .with_level(true)
        .with_thread_ids(false)
        .with_file(false)
        .with_line_number(false)
        .compact();
    
    // Create file layer for general logs
    let file_layer = fmt::layer()
        .with_writer(file_writer)
        .with_target(true)
        .with_level(true)
        .with_thread_ids(true)
        .with_file(true)
        .with_line_number(true)
        .json();
    
    // Create metrics layer
    let metrics_layer = fmt::layer()
        .with_writer(metrics_writer)
        .with_target(false)
        .with_level(false)
        .with_thread_ids(false)
        .with_file(false)
        .with_line_number(false)
        .json()
        .with_filter(EnvFilter::new("metrics"));
    
    // Environment filter for general logs
    let env_filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| {
            EnvFilter::new(format!(
                "saorsa_testnet={},saorsa_core={},ant_quic=info",
                level,
                level
            ))
        });
    
    // Build subscriber
    tracing_subscriber::registry()
        .with(console_layer)
        .with(file_layer.with_filter(env_filter))
        .with(metrics_layer)
        .init();
    
    info!("Logging initialized - Level: {:?}, Dir: {:?}", level, log_dir);
    
    // Store guards to prevent dropping (static storage would be better)
    std::mem::forget(_guard);
    std::mem::forget(_metrics_guard);
    
    Ok(())
}

/// Log aggregation utilities
pub mod aggregation {
    use super::*;
    use chrono::{DateTime, Utc};
    use serde::{Deserialize, Serialize};
    use std::collections::HashMap;
    use std::fs::File;
    use std::io::{BufRead, BufReader};
    use std::path::PathBuf;
    
    /// Log entry structure
    #[derive(Debug, Serialize, Deserialize)]
    pub struct LogEntry {
        pub timestamp: DateTime<Utc>,
        pub level: String,
        pub message: String,
        pub node_id: Option<String>,
        pub component: Option<String>,
        pub fields: HashMap<String, serde_json::Value>,
    }
    
    /// Log aggregator for collecting logs from multiple sources
    #[allow(dead_code)]
    pub struct LogAggregator {
        sources: Vec<LogSource>,
        output_dir: PathBuf,
    }
    
    impl LogAggregator {
        /// Create new aggregator
        #[allow(dead_code)]
        pub fn new(output_dir: PathBuf) -> Self {
            Self {
                sources: Vec::new(),
                output_dir,
            }
        }
        
        /// Add a log source
        #[allow(dead_code)]
        pub fn add_source(&mut self, source: LogSource) {
            self.sources.push(source);
        }
        
        /// Aggregate logs from all sources
        #[allow(dead_code)]
        pub async fn aggregate(&self) -> Result<Vec<LogEntry>> {
            let mut all_entries = Vec::new();
            
            for source in &self.sources {
                let entries = self.collect_from_source(source).await?;
                all_entries.extend(entries);
            }
            
            // Sort by timestamp
            all_entries.sort_by(|a, b| a.timestamp.cmp(&b.timestamp));
            
            Ok(all_entries)
        }
        
        /// Collect logs from a specific source
        #[allow(dead_code)]
        async fn collect_from_source(&self, source: &LogSource) -> Result<Vec<LogEntry>> {
            match source {
                LogSource::LocalFile(path) => self.read_local_file(path),
                LogSource::RemoteNode { host, path, ssh_key } => {
                    self.read_remote_file(host, path, ssh_key).await
                }
                LogSource::MetricsEndpoint(url) => {
                    self.collect_metrics(url).await
                }
            }
        }
        
        /// Read logs from local file
        #[allow(dead_code)]
        fn read_local_file(&self, path: &PathBuf) -> Result<Vec<LogEntry>> {
            let file = File::open(path)?;
            let reader = BufReader::new(file);
            let mut entries = Vec::new();
            
            for line in reader.lines() {
                let line = line?;
                if let Ok(entry) = serde_json::from_str::<LogEntry>(&line) {
                    entries.push(entry);
                }
            }
            
            Ok(entries)
        }
        
        /// Read logs from remote node via SSH
        #[allow(dead_code)]
        async fn read_remote_file(
            &self,
            host: &str,
            path: &str,
            ssh_key: &Path,
        ) -> Result<Vec<LogEntry>> {
            use std::process::Command;
            
            // Use scp to copy the remote file
            let temp_file = tempfile::NamedTempFile::new()?;
            let temp_path = temp_file.path();
            
            let output = Command::new("scp")
                .args([
                    "-i", &ssh_key.to_string_lossy(),
                    "-o", "StrictHostKeyChecking=no",
                    &format!("{}:{}", host, path),
                    &temp_path.to_string_lossy(),
                ])
                .output()?;
            
            if !output.status.success() {
                anyhow::bail!("SCP failed: {}", String::from_utf8_lossy(&output.stderr));
            }
            
            // Read the copied file
            self.read_local_file(&temp_path.to_path_buf())
        }
        
        /// Collect metrics from HTTP endpoint
        #[allow(dead_code)]
        async fn collect_metrics(&self, url: &str) -> Result<Vec<LogEntry>> {
            let client = reqwest::Client::new();
            let response = client.get(url).send().await?;
            let text = response.text().await?;
            
            // Parse Prometheus metrics and convert to log entries
            let entries = self.parse_prometheus_metrics(&text)?;
            
            Ok(entries)
        }
        
        /// Parse Prometheus metrics into log entries
        #[allow(dead_code)]
        fn parse_prometheus_metrics(&self, text: &str) -> Result<Vec<LogEntry>> {
            let mut entries = Vec::new();
            let timestamp = Utc::now();
            
            for line in text.lines() {
                if line.starts_with('#') || line.trim().is_empty() {
                    continue;
                }
                
                if let Some((metric_name, value)) = line.split_once(' ') {
                    let mut fields = HashMap::new();
                    
                    // Parse metric name and labels
                    if let Some((name, labels_str)) = metric_name.split_once('{') {
                        fields.insert("metric".to_string(), serde_json::Value::String(name.to_string()));
                        
                        // Parse labels
                        if let Some(labels) = labels_str.strip_suffix('}') {
                            for label_pair in labels.split(',') {
                                if let Some((key, val)) = label_pair.split_once('=') {
                                    let val = val.trim_matches('"');
                                    fields.insert(
                                        format!("label_{}", key.trim()),
                                        serde_json::Value::String(val.to_string())
                                    );
                                }
                            }
                        }
                    } else {
                        fields.insert("metric".to_string(), serde_json::Value::String(metric_name.to_string()));
                    }
                    
                    // Parse value
                    if let Ok(val) = value.trim().parse::<f64>() {
                        fields.insert("value".to_string(), serde_json::Value::Number(
                            serde_json::Number::from_f64(val).unwrap_or_else(|| serde_json::Number::from(0))
                        ));
                    }
                    
                    entries.push(LogEntry {
                        timestamp,
                        level: "INFO".to_string(),
                        message: format!("Metric: {}", metric_name),
                        node_id: None,
                        component: Some("metrics".to_string()),
                        fields,
                    });
                }
            }
            
            Ok(entries)
        }
        
        /// Export aggregated logs
        #[allow(dead_code)]
        pub async fn export(&self, entries: &[LogEntry], format: ExportFormat) -> Result<()> {
            let filename = match format {
                ExportFormat::Json => "aggregated.json",
                ExportFormat::Csv => "aggregated.csv",
                ExportFormat::Ndjson => "aggregated.ndjson",
            };
            
            let output_path = self.output_dir.join(filename);
            std::fs::create_dir_all(&self.output_dir)?;
            
            match format {
                ExportFormat::Json => {
                    let file = File::create(&output_path)?;
                    serde_json::to_writer_pretty(file, entries)?;
                }
                ExportFormat::Ndjson => {
                    use std::io::Write;
                    let mut file = File::create(&output_path)?;
                    for entry in entries {
                        writeln!(file, "{}", serde_json::to_string(entry)?)?;
                    }
                }
                ExportFormat::Csv => {
                    self.export_csv(entries, &output_path)?;
                }
            }
            
            info!("Exported {} log entries to {:?}", entries.len(), output_path);
            Ok(())
        }
        
        /// Export to CSV format
        #[allow(dead_code)]
        fn export_csv(&self, entries: &[LogEntry], path: &PathBuf) -> Result<()> {
            use std::io::Write;
            
            let mut file = File::create(path)?;
            
            // Write header
            writeln!(file, "timestamp,level,node_id,component,message")?;
            
            // Write data
            for entry in entries {
                writeln!(
                    file,
                    "{},{},{},{},\"{}\"",
                    entry.timestamp.to_rfc3339(),
                    entry.level,
                    entry.node_id.as_deref().unwrap_or(""),
                    entry.component.as_deref().unwrap_or(""),
                    entry.message.replace('"', "\"\"")
                )?;
            }
            
            Ok(())
        }
    }
    
    /// Log source types
    #[derive(Debug, Clone)]
    #[allow(dead_code)]
    pub enum LogSource {
        LocalFile(PathBuf),
        RemoteNode {
            host: String,
            path: String,
            ssh_key: PathBuf,
        },
        MetricsEndpoint(String),
    }
    
    /// Export formats
    #[derive(Debug, Clone)]
    #[allow(dead_code)]
    pub enum ExportFormat {
        Json,
        Csv,
        Ndjson,
    }
}

/// Performance logging utilities
pub mod performance {
    use std::time::{Duration, Instant};
    
    /// Performance timer for measuring operation duration
    #[allow(dead_code)]
    pub struct PerfTimer {
        start: Instant,
        operation: String,
    }
    
    impl PerfTimer {
        /// Start timing an operation
        #[allow(dead_code)]
        pub fn start(operation: impl Into<String>) -> Self {
            Self {
                start: Instant::now(),
                operation: operation.into(),
            }
        }
        
        /// Complete timing and log result
        #[allow(dead_code)]
        pub fn complete(self) -> Duration {
            let duration = self.start.elapsed();
            
            tracing::info!(
                target: "performance",
                operation = %self.operation,
                duration_ms = duration.as_millis(),
                "Operation completed"
            );
            
            duration
        }
    }
    
    /// Log structured metrics
    #[allow(dead_code)]
    pub fn log_metric(name: &str, value: f64, unit: &str, labels: Option<&[(&str, &str)]>) {
        let mut fields = vec![
            ("metric_name", name),
            ("unit", unit),
        ];
        
        if let Some(labels) = labels {
            fields.extend(labels.iter().copied());
        }
        
        tracing::info!(
            target: "metrics",
            value = value,
            fields = ?fields,
            "Metric recorded"
        );
    }
    
    /// Log NAT traversal metrics
    #[allow(dead_code)]
    pub fn log_nat_traversal(
        nat_type: &str,
        success: bool,
        duration: Duration,
        pqc_enabled: bool,
    ) {
        log_metric(
            "nat_traversal_duration_ms",
            duration.as_millis() as f64,
            "milliseconds",
            Some(&[
                ("nat_type", nat_type),
                ("success", if success { "true" } else { "false" }),
                ("pqc", if pqc_enabled { "true" } else { "false" }),
            ]),
        );
        
        log_metric(
            "nat_traversal_success",
            if success { 1.0 } else { 0.0 },
            "boolean",
            Some(&[
                ("nat_type", nat_type),
                ("pqc", if pqc_enabled { "true" } else { "false" }),
            ]),
        );
    }
    
    /// Log adaptive network metrics
    #[allow(dead_code)]
    pub fn log_adaptive_metrics(
        thompson_success_rate: f64,
        mab_reward: f64,
        cache_hit_rate: f64,
        churn_prediction: f64,
    ) {
        log_metric("thompson_sampling_success_rate", thompson_success_rate, "ratio", None);
        log_metric("mab_average_reward", mab_reward, "score", None);
        log_metric("q_learning_cache_hit_rate", cache_hit_rate, "ratio", None);
        log_metric("churn_prediction_accuracy", churn_prediction, "ratio", None);
    }
}