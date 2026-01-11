use anyhow::{Context, Result};
use clap::{Parser, Subcommand, ValueEnum};
use saorsa_quic_test::{
    harness::{
        AgentCapabilities, AgentClient, AgentInfo, AttemptResult, CollectionResult,
        FALLBACK_SOCKET_ADDR, GetResultsRequest, GetResultsResponse, HandshakeRequest,
        HandshakeResponse, HealthCheckResponse, LocalAgent, MixedOrchestrator,
        MonitorHealthResponse, ProbeResponse, ProofsResponse, RemoteAgent, ResultFormat,
        RunStatusResponse, RunSummary, ScenarioSpec, StartRunRequest, StartRunResponse,
        StartRunResult, StatusPollResult, StopRunResponse,
    },
    orchestrator::NatTestMatrix,
};
use std::collections::HashMap;
use std::net::SocketAddr;
use std::path::PathBuf;
use std::time::Duration;
use tracing::{error, info, warn};
use uuid::Uuid;

#[derive(Parser)]
#[command(name = "saorsa-testctl")]
#[command(about = "Orchestrator for distributed P2P network testing")]
#[command(version)]
struct Cli {
    #[command(subcommand)]
    command: Commands,

    #[arg(long, default_value = "info")]
    log_level: String,

    #[arg(long)]
    config: Option<PathBuf>,
}

#[derive(Subcommand)]
enum Commands {
    Run {
        #[arg(long)]
        scenario: String,

        #[arg(long)]
        agents: Vec<String>,

        #[arg(long, default_value = "100")]
        attempts: u32,

        #[arg(long)]
        output: Option<PathBuf>,

        #[arg(long)]
        seed: Option<u64>,
    },

    Status {
        #[arg(long)]
        run_id: Uuid,

        #[arg(long)]
        agents: Vec<String>,
    },

    Stop {
        #[arg(long)]
        run_id: Uuid,

        #[arg(long)]
        agents: Vec<String>,

        #[arg(long)]
        reason: Option<String>,
    },

    Results {
        #[arg(long)]
        run_id: Uuid,

        #[arg(long)]
        agents: Vec<String>,

        #[arg(long, default_value = "jsonl")]
        format: String,

        #[arg(long)]
        output: Option<PathBuf>,
    },

    Discover {
        #[arg(long)]
        registry: Option<String>,

        #[arg(long)]
        agents: Vec<String>,
    },

    Validate {
        #[arg(long)]
        scenario: PathBuf,
    },

    Matrix {
        #[arg(long, default_value = "minimal")]
        scope: String,

        #[arg(long)]
        output: Option<PathBuf>,
    },

    Report {
        #[arg(long)]
        run_id: Uuid,

        #[arg(long, default_value = "markdown")]
        format: String,

        #[arg(long)]
        output: Option<PathBuf>,

        #[arg(long)]
        results_file: Option<PathBuf>,

        #[arg(long)]
        agents: Vec<String>,
    },

    /// Run tests using local in-process agents (no VPS required)
    LocalRun {
        /// Scenario to run (connectivity_matrix, ci_fast, gossip_coverage, oracle_suite)
        #[arg(long, default_value = "ci_fast")]
        scenario: String,

        /// Number of local agents to spawn
        #[arg(long, default_value = "2")]
        local_agents: u32,

        /// Optional VPS agents to include (URLs)
        #[arg(long)]
        remote_agents: Vec<String>,

        /// Number of attempts per test cell
        #[arg(long, default_value = "10")]
        attempts: u32,

        /// Output file for results
        #[arg(long)]
        output: Option<PathBuf>,

        /// Timeout for the entire run in seconds
        #[arg(long, default_value = "300")]
        timeout_secs: u64,
    },

    /// Run long-duration network monitoring (24-hour by default)
    Monitor {
        /// Agent URLs to monitor
        #[arg(long)]
        agents: Vec<String>,

        /// Duration of monitoring in hours
        #[arg(long, default_value = "24")]
        duration_hours: u64,

        /// Interval between checks in minutes
        #[arg(long, default_value = "60")]
        interval_mins: u64,

        /// Output directory for monitoring results
        #[arg(long)]
        output_dir: Option<PathBuf>,

        /// Collect logs when failures are detected
        #[arg(long)]
        collect_logs_on_failure: bool,
    },

    /// One-time health check of all nodes
    HealthCheck {
        /// Agent URLs to check
        #[arg(long)]
        agents: Vec<String>,

        /// Output format
        #[arg(long, default_value = "table")]
        format: OutputFormat,

        /// Output file (stdout if not specified)
        #[arg(long)]
        output: Option<PathBuf>,
    },

    /// Collect proofs from all nodes
    CollectProofs {
        /// Agent URLs to collect from
        #[arg(long)]
        agents: Vec<String>,

        /// Output file for proofs
        #[arg(long)]
        output: Option<PathBuf>,

        /// Only collect connectivity proofs
        #[arg(long)]
        connectivity_only: bool,

        /// Only collect gossip proofs
        #[arg(long)]
        gossip_only: bool,

        /// Only collect CRDT proofs
        #[arg(long)]
        crdt_only: bool,
    },
}

/// Output format for health check command
#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
enum OutputFormat {
    Table,
    Json,
    Jsonl,
}

struct Orchestrator {
    agents: HashMap<String, AgentClient>,
    run_id: Option<Uuid>,
}

impl Orchestrator {
    fn new() -> Self {
        Self {
            agents: HashMap::new(),
            run_id: None,
        }
    }

    fn add_agent(&mut self, agent_id: &str, base_url: &str, p2p_listen_addr: SocketAddr) {
        let client = AgentClient::new(base_url, agent_id, p2p_listen_addr);
        self.agents.insert(agent_id.to_string(), client);
    }

    async fn discover_agents(&mut self, agent_urls: &[String]) -> Result<Vec<AgentInfo>> {
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(10))
            .build()?;

        let mut discovered = Vec::new();

        for url in agent_urls {
            let health_url = format!("{}/health", url.trim_end_matches('/'));
            match client.get(&health_url).send().await {
                Ok(resp) if resp.status().is_success() => {
                    match resp.json::<HealthCheckResponse>().await {
                        Ok(health) => {
                            let p2p_addr = match health.p2p_listen_addr {
                                Some(addr) => addr,
                                None => {
                                    warn!(
                                        "Agent {} at {} did not report p2p_listen_addr; using fallback 0.0.0.0:0 which may cause test failures",
                                        health.agent_id, url
                                    );
                                    FALLBACK_SOCKET_ADDR
                                }
                            };
                            let agent_info = AgentInfo {
                                agent_id: health.agent_id.clone(),
                                version: health.version,
                                capabilities: AgentCapabilities::default(),
                                api_base_url: url.clone(),
                                p2p_listen_addr: p2p_addr,
                                nat_profiles_available: vec![],
                                status: health.status,
                            };
                            self.add_agent(&health.agent_id, url, p2p_addr);
                            discovered.push(agent_info);
                            info!("Discovered agent: {} at {}", health.agent_id, url);
                        }
                        Err(e) => {
                            error!(
                                "Agent at {} returned invalid JSON: {}. Schema mismatch?",
                                url, e
                            );
                        }
                    }
                }
                Ok(resp) => {
                    warn!("Agent at {} returned status {}", url, resp.status());
                }
                Err(e) => {
                    warn!("Failed to reach agent at {}: {}", url, e);
                }
            }
        }

        Ok(discovered)
    }

    async fn handshake_agents(&self) -> Result<Vec<HandshakeResponse>> {
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(10))
            .build()?;

        let request = HandshakeRequest {
            orchestrator_id: "saorsa-testctl".to_string(),
            protocol_version: 1,
            required_capabilities: vec![],
        };

        let mut responses = Vec::new();

        for (agent_id, agent_client) in &self.agents {
            let url = agent_client.handshake_url();
            match client.post(&url).json(&request).send().await {
                Ok(resp) if resp.status().is_success() => {
                    match resp.json::<HandshakeResponse>().await {
                        Ok(handshake) => {
                            if handshake.compatible {
                                info!("Handshake successful with {}", agent_id);
                            } else {
                                warn!(
                                    "Agent {} missing capabilities: {:?}",
                                    agent_id, handshake.missing_capabilities
                                );
                            }
                            responses.push(handshake);
                        }
                        Err(e) => {
                            error!("Handshake response from {} invalid JSON: {}", agent_id, e);
                        }
                    }
                }
                Ok(resp) => {
                    error!("Handshake with {} failed: {}", agent_id, resp.status());
                }
                Err(e) => {
                    error!("Failed to handshake with {}: {}", agent_id, e);
                }
            }
        }

        Ok(responses)
    }

    async fn start_run(&mut self, scenario: ScenarioSpec) -> Result<StartRunResult> {
        let run_id = Uuid::new_v4();
        self.run_id = Some(run_id);
        let mut result = StartRunResult::new(run_id);

        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(30))
            .build()?;

        let peer_agents: Vec<_> = self
            .agents
            .iter()
            .map(|(id, c)| saorsa_quic_test::harness::PeerAgentInfo {
                agent_id: id.clone(),
                api_base_url: Some(c.base_url.clone()),
                p2p_listen_addr: c.p2p_listen_addr,
                nat_profile: None,
            })
            .collect();

        for (agent_id, agent_client) in &self.agents {
            let request = StartRunRequest {
                run_id,
                scenario: scenario.clone(),
                agent_role: "peer".to_string(),
                peer_agents: peer_agents.clone(),
            };

            let url = agent_client.start_run_url();
            match client.post(&url).json(&request).send().await {
                Ok(resp) if resp.status().is_success() => {
                    match resp.json::<StartRunResponse>().await {
                        Ok(start_resp) if start_resp.success => {
                            info!("Started run {} on agent {}", run_id, agent_id);
                            result.record_success(agent_id);
                        }
                        Ok(start_resp) => {
                            let err = start_resp
                                .error
                                .unwrap_or_else(|| "Unknown error".to_string());
                            error!("Failed to start run on {}: {}", agent_id, err);
                            result.record_failure(agent_id, &err);
                        }
                        Err(e) => {
                            error!("Failed to parse response from {}: {}", agent_id, e);
                            result.record_failure(agent_id, &format!("JSON parse error: {}", e));
                        }
                    }
                }
                Ok(resp) => {
                    let err = format!("HTTP {}", resp.status());
                    error!("Start run on {} returned: {}", agent_id, err);
                    result.record_failure(agent_id, &err);
                }
                Err(e) => {
                    error!("Failed to start run on {}: {}", agent_id, e);
                    result.record_failure(agent_id, &e.to_string());
                }
            }
        }

        if !result.has_any_success() {
            anyhow::bail!(
                "Failed to start run on ANY agent. Failures: {:?}",
                result.failed_agents()
            );
        }

        if !result.all_succeeded() {
            warn!(
                "Run {} started on {}/{} agents. Failed: {:?}",
                run_id,
                result.successful_agents().len(),
                result.successful_agents().len() + result.failed_agents().len(),
                result.failed_agents()
            );
        }

        Ok(result)
    }

    async fn get_status(&self, run_id: Uuid) -> Result<StatusPollResult> {
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(10))
            .build()?;

        let mut result = StatusPollResult::new(self.agents.len());

        for (agent_id, agent_client) in &self.agents {
            let url = agent_client.status_url(run_id);
            match client.get(&url).send().await {
                Ok(resp) if resp.status().is_success() => {
                    match resp.json::<RunStatusResponse>().await {
                        Ok(status) => {
                            result.record_status(agent_id, status);
                        }
                        Err(e) => {
                            let err = format!("JSON parse error: {}", e);
                            error!("Failed to parse status response from {}: {}", agent_id, e);
                            result.record_failure(agent_id, &err);
                        }
                    }
                }
                Ok(resp) => {
                    let err = format!("HTTP {}", resp.status());
                    warn!("Status request to {} failed: {}", agent_id, err);
                    result.record_failure(agent_id, &err);
                }
                Err(e) => {
                    let err = e.to_string();
                    warn!("Failed to get status from {}: {}", agent_id, err);
                    result.record_failure(agent_id, &err);
                }
            }
        }

        Ok(result)
    }

    async fn stop_run(&self, run_id: Uuid, reason: Option<&str>) -> Result<()> {
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(10))
            .build()?;

        for (agent_id, agent_client) in &self.agents {
            let url = agent_client.stop_run_url(run_id);
            let request = saorsa_quic_test::harness::StopRunRequest {
                run_id,
                reason: reason.map(String::from),
            };

            match client.post(&url).json(&request).send().await {
                Ok(resp) if resp.status().is_success() => {
                    match resp.json::<StopRunResponse>().await {
                        Ok(stop_resp) => {
                            info!(
                                "Stopped run on {}: {} attempts completed",
                                agent_id, stop_resp.attempts_completed
                            );
                        }
                        Err(e) => {
                            error!("Failed to parse stop response from {}: {}", agent_id, e);
                        }
                    }
                }
                _ => {
                    warn!("Failed to stop run on {}", agent_id);
                }
            }
        }

        Ok(())
    }

    async fn collect_results(&self, run_id: Uuid) -> Result<CollectionResult<AttemptResult>> {
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(60))
            .build()?;

        let mut collection = CollectionResult::new();

        for (agent_id, agent_client) in &self.agents {
            let url = agent_client.results_url(run_id);
            let request = GetResultsRequest {
                run_id,
                format: ResultFormat::Json,
                include_artifacts: false,
            };

            match client.post(&url).json(&request).send().await {
                Ok(resp) if resp.status().is_success() => {
                    match resp.json::<GetResultsResponse>().await {
                        Ok(results_resp) => {
                            info!(
                                "Collected {} results from {}",
                                results_resp.results.len(),
                                agent_id
                            );
                            collection.add_items(agent_id, results_resp.results);
                        }
                        Err(e) => {
                            error!("Failed to parse results from {}: {}", agent_id, e);
                            collection
                                .record_failure(agent_id, &format!("JSON parse error: {}", e));
                        }
                    }
                }
                Ok(resp) => {
                    let err = format!("HTTP {}", resp.status());
                    warn!("Failed to collect results from {}: {}", agent_id, err);
                    collection.record_failure(agent_id, &err);
                }
                Err(e) => {
                    warn!("Failed to collect results from {}: {}", agent_id, e);
                    collection.record_failure(agent_id, &e.to_string());
                }
            }
        }

        if !collection.is_complete() {
            warn!(
                "Results collection incomplete. Failed sources: {:?}",
                collection.failed_sources
            );
        }

        Ok(collection)
    }
}

fn load_scenario(name: &str) -> Result<ScenarioSpec> {
    match name {
        "connectivity_matrix" => Ok(ScenarioSpec::connectivity_matrix()),
        "ci_fast" => Ok(ScenarioSpec::ci_fast()),
        "gossip_coverage" => Ok(ScenarioSpec::gossip_coverage()),
        "oracle_suite" => Ok(ScenarioSpec::oracle_suite()),
        _ => Err(anyhow::anyhow!("Unknown scenario: {}", name)),
    }
}

fn generate_matrix_report(scope: &str) -> String {
    let matrix = match scope {
        "minimal" => NatTestMatrix::minimal(),
        "comprehensive" | "full" => NatTestMatrix::comprehensive(),
        _ => NatTestMatrix::minimal(),
    };

    let summary = matrix.rate_summary();

    let mut report = String::new();
    report.push_str("# NAT Connectivity Matrix\n\n");
    report.push_str("## Summary\n");
    report.push_str(&format!("- Total combinations: {}\n", summary.total));
    report.push_str(&format!("- Easy (≥90%): {}\n", summary.easy));
    report.push_str(&format!("- Moderate (70-89%): {}\n", summary.moderate));
    report.push_str(&format!("- Hard (50-69%): {}\n", summary.hard));
    report.push_str(&format!("- Very Hard (<50%): {}\n", summary.very_hard));
    report.push_str(&format!(
        "- Average expected rate: {:.1}%\n\n",
        summary.avg_expected_rate * 100.0
    ));

    report.push_str("## Matrix\n\n");
    report.push_str("| Source | Destination | Method | Expected |\n");
    report.push_str("|--------|-------------|--------|----------|\n");

    for pair in &matrix.combinations {
        report.push_str(&format!(
            "| {} | {} | {} | {:.0}% |\n",
            pair.source_nat,
            pair.dest_nat,
            pair.expected_method,
            pair.expected_success_rate * 100.0
        ));
    }

    report
}

fn generate_run_report(run_id: Uuid, results: &[AttemptResult], format: &str) -> String {
    let summary = RunSummary::from_attempts(run_id, "unknown", results);

    match format {
        "markdown" => {
            let mut report = String::new();
            report.push_str(&format!("# Test Run Report: {}\n\n", run_id));
            report.push_str("## Summary\n\n");
            report.push_str(&format!("- Total attempts: {}\n", summary.total_attempts));
            report.push_str(&format!("- Successful: {}\n", summary.successful_attempts));
            report.push_str(&format!("- Failed: {}\n", summary.failed_attempts));
            report.push_str(&format!(
                "- Success rate: {:.1}%\n",
                summary.success_rate * 100.0
            ));
            report.push_str(&format!(
                "- Harness failures: {}\n",
                summary.harness_failures
            ));
            report.push_str(&format!("- SUT failures: {}\n", summary.sut_failures));
            report.push_str(&format!(
                "- Infrastructure flakes: {}\n\n",
                summary.infrastructure_failures
            ));

            if let Some(p50) = summary.latency_p50_ms {
                report.push_str("## Latency\n\n");
                report.push_str(&format!("- p50: {}ms\n", p50));
                if let Some(p95) = summary.latency_p95_ms {
                    report.push_str(&format!("- p95: {}ms\n", p95));
                }
                if let Some(p99) = summary.latency_p99_ms {
                    report.push_str(&format!("- p99: {}ms\n", p99));
                }
            }

            report.push_str("\n## By Dimension\n\n");
            report.push_str("| Dimension | Total | Success | Rate |\n");
            report.push_str("|-----------|-------|---------|------|\n");
            for (dim, stats) in &summary.by_dimension {
                report.push_str(&format!(
                    "| {} | {} | {} | {:.1}% |\n",
                    dim,
                    stats.total,
                    stats.successful,
                    stats.success_rate * 100.0
                ));
            }

            report
        }
        "json" => serde_json::to_string_pretty(&summary).unwrap_or_default(),
        _ => format!("{:?}", summary),
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    tracing_subscriber::fmt()
        .with_env_filter(&cli.log_level)
        .init();

    let mut orchestrator = Orchestrator::new();

    match cli.command {
        Commands::Run {
            scenario,
            agents,
            attempts,
            output,
            seed,
        } => {
            if agents.is_empty() {
                anyhow::bail!("At least one agent URL required (--agents)");
            }

            let discovered = orchestrator.discover_agents(&agents).await?;
            if discovered.is_empty() {
                anyhow::bail!("No agents discovered");
            }

            orchestrator.handshake_agents().await?;

            let mut scenario_spec = load_scenario(&scenario)?;
            if let Some(s) = seed {
                scenario_spec.seed = Some(s);
            }
            scenario_spec.test_matrix.attempts_per_cell = attempts;

            info!("Starting run with scenario: {}", scenario);
            let start_result = orchestrator.start_run(scenario_spec).await?;
            let run_id = start_result.run_id;
            info!(
                "Run started: {} ({}/{} agents)",
                run_id,
                start_result.successful_agents().len(),
                start_result.successful_agents().len() + start_result.failed_agents().len()
            );

            loop {
                tokio::time::sleep(Duration::from_secs(5)).await;
                let poll_result = orchestrator.get_status(run_id).await?;

                if poll_result.all_complete() {
                    break;
                }

                if !poll_result.failed_agents.is_empty() {
                    warn!(
                        "Status poll: {}/{} agents failed to respond",
                        poll_result.failed_agents.len(),
                        poll_result.expected_count
                    );
                }

                let total_progress: u32 = poll_result
                    .statuses
                    .values()
                    .map(|s| s.progress.completed_attempts)
                    .sum();
                info!(
                    "Progress: {} attempts completed ({}/{} agents reporting)",
                    total_progress,
                    poll_result.statuses.len(),
                    poll_result.expected_count
                );
            }

            let collection = orchestrator.collect_results(run_id).await?;
            let summary = RunSummary::from_attempts(run_id, &scenario, &collection.items);

            info!(
                "Run complete: {}/{} successful ({:.1}%)",
                summary.successful_attempts,
                summary.total_attempts,
                summary.success_rate * 100.0
            );

            if !collection.is_complete() {
                warn!(
                    "WARNING: Results incomplete - {} sources failed",
                    collection.failed_sources.len()
                );
            }

            if let Some(output_path) = output {
                let json = serde_json::to_string_pretty(&collection.items)?;
                std::fs::write(&output_path, json)?;
                info!("Results written to {:?}", output_path);
            }
        }

        Commands::Status { run_id, agents } => {
            if agents.is_empty() {
                anyhow::bail!("At least one agent URL required");
            }

            orchestrator.discover_agents(&agents).await?;
            let poll_result = orchestrator.get_status(run_id).await?;

            for (agent_id, status) in &poll_result.statuses {
                println!(
                    "{}: {:?} - {}/{} completed",
                    agent_id,
                    status.status,
                    status.progress.completed_attempts,
                    status.progress.total_attempts
                );
            }

            if !poll_result.failed_agents.is_empty() {
                for (agent_id, err) in &poll_result.failed_agents {
                    println!("{}: FAILED - {}", agent_id, err);
                }
            }
        }

        Commands::Stop {
            run_id,
            agents,
            reason,
        } => {
            if agents.is_empty() {
                anyhow::bail!("At least one agent URL required");
            }

            orchestrator.discover_agents(&agents).await?;
            orchestrator.stop_run(run_id, reason.as_deref()).await?;
            info!("Run {} stopped", run_id);
        }

        Commands::Results {
            run_id,
            agents,
            format,
            output,
        } => {
            if agents.is_empty() {
                anyhow::bail!("At least one agent URL required");
            }

            orchestrator.discover_agents(&agents).await?;
            let collection = orchestrator.collect_results(run_id).await?;

            if !collection.is_complete() {
                warn!(
                    "WARNING: Results incomplete - {} sources failed: {:?}",
                    collection.failed_sources.len(),
                    collection.failed_sources
                );
            }

            let output_str = match format.as_str() {
                "json" => serde_json::to_string_pretty(&collection.items)?,
                "jsonl" => collection
                    .items
                    .iter()
                    .filter_map(|r| r.to_jsonl().ok())
                    .collect::<Vec<_>>()
                    .join("\n"),
                _ => format!("{:?}", collection.items),
            };

            if let Some(output_path) = output {
                std::fs::write(&output_path, &output_str)?;
                info!("Results written to {:?}", output_path);
            } else {
                println!("{}", output_str);
            }
        }

        Commands::Discover { registry, agents } => {
            let urls = if agents.is_empty() {
                if let Some(reg) = registry {
                    vec![reg]
                } else {
                    vec!["http://localhost:8080".to_string()]
                }
            } else {
                agents
            };

            let discovered = orchestrator.discover_agents(&urls).await?;
            println!("Discovered {} agents:", discovered.len());
            for agent in discovered {
                println!(
                    "  - {}: {} ({:?})",
                    agent.agent_id, agent.version, agent.status
                );
            }
        }

        Commands::Validate { scenario } => {
            let content = std::fs::read_to_string(&scenario)?;
            let spec: ScenarioSpec = serde_json::from_str(&content)
                .or_else(|_| serde_yaml::from_str(&content))
                .context("Failed to parse scenario file")?;

            match spec.validate() {
                Ok(()) => {
                    println!("Scenario '{}' is valid", spec.id);
                    println!("  - Name: {}", spec.name);
                    println!("  - Suite: {:?}", spec.suite);
                    println!("  - NAT profiles: {}", spec.nat_profiles.len());
                    println!("  - Estimated duration: {:?}", spec.estimated_duration());
                }
                Err(errors) => {
                    eprintln!("Scenario validation failed:");
                    for err in errors {
                        eprintln!("  - {}", err);
                    }
                    std::process::exit(1);
                }
            }
        }

        Commands::Matrix { scope, output } => {
            let report = generate_matrix_report(&scope);

            if let Some(output_path) = output {
                std::fs::write(&output_path, &report)?;
                info!("Matrix report written to {:?}", output_path);
            } else {
                println!("{}", report);
            }
        }

        Commands::Report {
            run_id,
            format,
            output,
            results_file,
            agents,
        } => {
            let results: Vec<AttemptResult> = if let Some(file_path) = results_file {
                let content = std::fs::read_to_string(&file_path)?;
                if file_path.extension().is_some_and(|e| e == "jsonl") {
                    content
                        .lines()
                        .filter_map(|line| serde_json::from_str(line).ok())
                        .collect()
                } else {
                    serde_json::from_str(&content)?
                }
            } else if !agents.is_empty() {
                orchestrator.discover_agents(&agents).await?;
                let collection = orchestrator.collect_results(run_id).await?;
                if !collection.is_complete() {
                    warn!(
                        "WARNING: Results incomplete - {} sources failed",
                        collection.failed_sources.len()
                    );
                }
                collection.items
            } else {
                anyhow::bail!("Either --results-file or --agents required");
            };

            let report = generate_run_report(run_id, &results, &format);

            if let Some(output_path) = output {
                std::fs::write(&output_path, &report)?;
                info!("Report written to {:?}", output_path);
            } else {
                println!("{}", report);
            }
        }

        Commands::LocalRun {
            scenario,
            local_agents,
            remote_agents,
            attempts,
            output,
            timeout_secs,
        } => {
            info!("Starting local test run with {} local agents", local_agents);

            let mut mixed_orch = MixedOrchestrator::new();

            // Create local agents
            for i in 0..local_agents {
                let agent_id = format!("local-{}", i);
                match LocalAgent::new(&agent_id).await {
                    Ok(agent) => {
                        info!("Created local agent: {}", agent_id);
                        mixed_orch.add_local(agent);
                    }
                    Err(e) => {
                        error!("Failed to create local agent {}: {}", agent_id, e);
                    }
                }
            }

            // Add remote agents if specified
            for url in &remote_agents {
                let health_url = format!("{}/health", url.trim_end_matches('/'));
                let client = reqwest::Client::builder()
                    .timeout(Duration::from_secs(10))
                    .build()?;

                match client.get(&health_url).send().await {
                    Ok(resp) if resp.status().is_success() => {
                        match resp.json::<HealthCheckResponse>().await {
                            Ok(health) => {
                                let p2p_addr =
                                    health.p2p_listen_addr.unwrap_or(FALLBACK_SOCKET_ADDR);
                                let remote = RemoteAgent::new(&health.agent_id, url, p2p_addr);
                                info!("Added remote agent: {} at {}", health.agent_id, url);
                                mixed_orch.add_remote(remote);
                            }
                            Err(e) => {
                                warn!("Failed to parse health response from {}: {}", url, e);
                            }
                        }
                    }
                    _ => {
                        warn!("Failed to reach remote agent at {}", url);
                    }
                }
            }

            if mixed_orch.agent_count() < 2 {
                anyhow::bail!(
                    "Need at least 2 agents for connectivity testing, got {}",
                    mixed_orch.agent_count()
                );
            }

            info!(
                "Orchestrator ready: {} local + {} remote = {} total agents",
                mixed_orch.local_agent_count(),
                mixed_orch.remote_agent_count(),
                mixed_orch.agent_count()
            );

            // Load and configure scenario
            let mut scenario_spec = load_scenario(&scenario)?;
            scenario_spec.test_matrix.attempts_per_cell = attempts;

            info!(
                "Running scenario '{}' with {} attempts per cell",
                scenario, attempts
            );

            // Run tests
            let collection = mixed_orch
                .run_and_wait(
                    scenario_spec,
                    Duration::from_secs(2),
                    Duration::from_secs(timeout_secs),
                )
                .await?;

            // Generate summary
            let run_id = Uuid::new_v4();
            let summary = RunSummary::from_attempts(run_id, &scenario, &collection.items);

            println!("\n=== Test Run Complete ===");
            println!("Total attempts:    {}", summary.total_attempts);
            println!("Successful:        {}", summary.successful_attempts);
            println!("Failed:            {}", summary.failed_attempts);
            println!("Success rate:      {:.1}%", summary.success_rate * 100.0);

            if let Some(p50) = summary.latency_p50_ms {
                println!("Latency p50:       {}ms", p50);
            }
            if let Some(p95) = summary.latency_p95_ms {
                println!("Latency p95:       {}ms", p95);
            }

            if !collection.is_complete() {
                warn!(
                    "WARNING: Results incomplete - {} sources failed",
                    collection.failed_sources.len()
                );
            }

            // Save results if output specified
            if let Some(output_path) = output {
                let json = serde_json::to_string_pretty(&collection.items)?;
                std::fs::write(&output_path, json)?;
                info!("Results written to {:?}", output_path);
            }
        }

        Commands::Monitor {
            agents,
            duration_hours,
            interval_mins,
            output_dir,
            collect_logs_on_failure,
        } => {
            if agents.is_empty() {
                anyhow::bail!("At least one agent URL required (--agents)");
            }

            let output_path = output_dir.unwrap_or_else(|| PathBuf::from("./monitor-results"));
            std::fs::create_dir_all(&output_path)?;

            let total_intervals = (duration_hours * 60) / interval_mins;
            let interval_duration = Duration::from_secs(interval_mins * 60);

            info!(
                "Starting {} hour monitoring ({} intervals, {} min each)",
                duration_hours, total_intervals, interval_mins
            );
            info!("Results will be saved to {:?}", output_path);

            let client = reqwest::Client::builder()
                .timeout(Duration::from_secs(30))
                .build()?;

            let mut interval_count: u64 = 0;
            let mut total_failures = 0;
            let start_time = std::time::Instant::now();

            while interval_count < total_intervals {
                interval_count += 1;
                let interval_start = std::time::Instant::now();
                let timestamp = chrono::Utc::now().format("%Y%m%d_%H%M%S").to_string();

                info!(
                    "=== Interval {}/{} ({}) ===",
                    interval_count, total_intervals, timestamp
                );

                let mut interval_results = Vec::new();
                let mut interval_failures = 0;

                // Phase 1: Probe all agents
                for url in &agents {
                    let probe_url = format!("{}/api/probe", url.trim_end_matches('/'));
                    let probe_start = std::time::Instant::now();

                    let probe_result = match client.get(&probe_url).send().await {
                        Ok(resp) if resp.status().is_success() => {
                            match resp.json::<ProbeResponse>().await {
                                Ok(probe) => {
                                    let latency_ms = probe_start.elapsed().as_millis() as u64;
                                    info!("  {} - OK ({}ms)", probe.agent_id, latency_ms);
                                    Some((probe.agent_id.clone(), true, latency_ms, None))
                                }
                                Err(e) => {
                                    error!("  {} - Parse error: {}", url, e);
                                    interval_failures += 1;
                                    Some((
                                        url.clone(),
                                        false,
                                        0,
                                        Some(format!("Parse error: {}", e)),
                                    ))
                                }
                            }
                        }
                        Ok(resp) => {
                            error!("  {} - HTTP {}", url, resp.status());
                            interval_failures += 1;
                            Some((
                                url.clone(),
                                false,
                                0,
                                Some(format!("HTTP {}", resp.status())),
                            ))
                        }
                        Err(e) => {
                            error!("  {} - Unreachable: {}", url, e);
                            interval_failures += 1;
                            Some((url.clone(), false, 0, Some(e.to_string())))
                        }
                    };

                    if let Some(result) = probe_result {
                        interval_results.push(result);
                    }
                }

                // Phase 2: Get health details from reachable agents
                let mut health_results = Vec::new();
                for url in &agents {
                    let health_url = format!("{}/api/health", url.trim_end_matches('/'));
                    if let Ok(resp) = client.get(&health_url).send().await {
                        if resp.status().is_success() {
                            if let Ok(health) = resp.json::<MonitorHealthResponse>().await {
                                health_results.push(health);
                            }
                        }
                    }
                }

                // Phase 3: Collect logs if there were failures
                if collect_logs_on_failure && interval_failures > 0 {
                    info!(
                        "  Collecting logs from {} agents due to failures",
                        agents.len()
                    );
                    for url in &agents {
                        let logs_url = format!("{}/api/logs?limit=50", url.trim_end_matches('/'));
                        if let Ok(resp) = client.get(&logs_url).send().await {
                            if resp.status().is_success() {
                                if let Ok(logs) = resp.text().await {
                                    let log_file = output_path.join(format!(
                                        "logs_{}_{}.json",
                                        url.replace([':', '/', '.'], "_"),
                                        timestamp
                                    ));
                                    let _ = std::fs::write(&log_file, logs);
                                }
                            }
                        }
                    }
                }

                // Save interval report
                let interval_report = serde_json::json!({
                    "interval": interval_count,
                    "timestamp": timestamp,
                    "elapsed_hours": start_time.elapsed().as_secs() as f64 / 3600.0,
                    "agents_checked": agents.len(),
                    "agents_healthy": agents.len() - interval_failures,
                    "agents_failed": interval_failures,
                    "probe_results": interval_results,
                    "health_results": health_results,
                });

                let report_file = output_path.join(format!("interval_{}.json", timestamp));
                std::fs::write(
                    &report_file,
                    serde_json::to_string_pretty(&interval_report)?,
                )?;

                total_failures += interval_failures;
                let elapsed = interval_start.elapsed();
                let remaining = if elapsed < interval_duration {
                    interval_duration - elapsed
                } else {
                    Duration::ZERO
                };

                if remaining > Duration::ZERO && interval_count < total_intervals {
                    info!("  Sleeping for {:?} until next interval", remaining);
                    tokio::time::sleep(remaining).await;
                }
            }

            // Generate final summary
            let summary = serde_json::json!({
                "monitoring_duration_hours": duration_hours,
                "total_intervals": total_intervals,
                "agents_monitored": agents.len(),
                "total_failures": total_failures,
                "average_failures_per_interval": total_failures as f64 / total_intervals as f64,
                "success_rate": 1.0 - (total_failures as f64 / (total_intervals as f64 * agents.len() as f64)),
            });

            let summary_file = output_path.join("summary.json");
            std::fs::write(&summary_file, serde_json::to_string_pretty(&summary)?)?;

            info!("\n=== Monitoring Complete ===");
            info!("Duration: {} hours", duration_hours);
            info!("Total intervals: {}", total_intervals);
            info!("Total failures: {}", total_failures);
            info!("Results saved to {:?}", output_path);
        }

        Commands::HealthCheck {
            agents,
            format,
            output,
        } => {
            if agents.is_empty() {
                anyhow::bail!("At least one agent URL required (--agents)");
            }

            let client = reqwest::Client::builder()
                .timeout(Duration::from_secs(30))
                .build()?;

            let mut results: Vec<MonitorHealthResponse> = Vec::new();
            let mut failures: Vec<(String, String)> = Vec::new();

            for url in &agents {
                let health_url = format!("{}/api/health", url.trim_end_matches('/'));
                match client.get(&health_url).send().await {
                    Ok(resp) if resp.status().is_success() => {
                        match resp.json::<MonitorHealthResponse>().await {
                            Ok(health) => results.push(health),
                            Err(e) => failures.push((url.clone(), format!("Parse error: {}", e))),
                        }
                    }
                    Ok(resp) => failures.push((url.clone(), format!("HTTP {}", resp.status()))),
                    Err(e) => failures.push((url.clone(), e.to_string())),
                }
            }

            let output_str = match format {
                OutputFormat::Table => {
                    let mut table = String::new();
                    table.push_str("Agent ID          | Status | Uptime  | Conn% | Proofs\n");
                    table.push_str("------------------|--------|---------|-------|--------\n");
                    for r in &results {
                        let proof_status = format!(
                            "C:{} G:{} D:{}",
                            if r.proof_summary.connectivity_pass {
                                "OK"
                            } else {
                                "FAIL"
                            },
                            if r.proof_summary.gossip_pass {
                                "OK"
                            } else {
                                "FAIL"
                            },
                            if r.proof_summary.crdt_pass {
                                "OK"
                            } else {
                                "FAIL"
                            }
                        );
                        table.push_str(&format!(
                            "{:17} | {:6} | {:7} | {:5.1} | {}\n",
                            &r.agent_id[..r.agent_id.len().min(17)],
                            if r.healthy { "OK" } else { "FAIL" },
                            format_duration(r.uptime_secs),
                            r.connectivity_summary.reachability_percent,
                            proof_status
                        ));
                    }
                    for (url, err) in &failures {
                        table.push_str(&format!("{:17} | UNREACHABLE | {}\n", url, err));
                    }
                    table
                }
                OutputFormat::Json => serde_json::to_string_pretty(&serde_json::json!({
                    "healthy": results,
                    "failures": failures,
                }))?,
                OutputFormat::Jsonl => {
                    let mut lines = Vec::new();
                    for r in &results {
                        lines.push(serde_json::to_string(&r)?);
                    }
                    lines.join("\n")
                }
            };

            if let Some(output_path) = output {
                std::fs::write(&output_path, &output_str)?;
                info!("Health check results written to {:?}", output_path);
            } else {
                println!("{}", output_str);
            }

            if !failures.is_empty() {
                warn!("{} agents unreachable", failures.len());
                std::process::exit(1);
            }
        }

        Commands::CollectProofs {
            agents,
            output,
            connectivity_only,
            gossip_only,
            crdt_only,
        } => {
            if agents.is_empty() {
                anyhow::bail!("At least one agent URL required (--agents)");
            }

            let client = reqwest::Client::builder()
                .timeout(Duration::from_secs(60))
                .build()?;

            let mut all_proofs: Vec<ProofsResponse> = Vec::new();
            let mut failures: Vec<(String, String)> = Vec::new();

            for url in &agents {
                // Determine which endpoint to use
                let proofs_url = if connectivity_only {
                    format!("{}/api/proofs/connectivity", url.trim_end_matches('/'))
                } else if gossip_only {
                    format!("{}/api/proofs/gossip", url.trim_end_matches('/'))
                } else if crdt_only {
                    format!("{}/api/proofs/crdt", url.trim_end_matches('/'))
                } else {
                    format!("{}/api/proofs", url.trim_end_matches('/'))
                };

                match client.get(&proofs_url).send().await {
                    Ok(resp) if resp.status().is_success() => {
                        if connectivity_only || gossip_only || crdt_only {
                            // Individual proof endpoints return different types
                            // Wrap them in a ProofsResponse for consistency
                            let agent_id = url.clone();
                            let timestamp_ms = std::time::SystemTime::now()
                                .duration_since(std::time::UNIX_EPOCH)
                                .map(|d| d.as_millis() as u64)
                                .unwrap_or(0);

                            if connectivity_only {
                                if let Ok(proof) = resp
                                    .json::<saorsa_quic_test::harness::ConnectivityProofData>()
                                    .await
                                {
                                    all_proofs.push(ProofsResponse {
                                        agent_id,
                                        timestamp_ms,
                                        connectivity: Some(proof),
                                        gossip: None,
                                        crdt: None,
                                    });
                                }
                            } else if gossip_only {
                                if let Ok(proof) = resp
                                    .json::<saorsa_quic_test::harness::GossipProofData>()
                                    .await
                                {
                                    all_proofs.push(ProofsResponse {
                                        agent_id,
                                        timestamp_ms,
                                        connectivity: None,
                                        gossip: Some(proof),
                                        crdt: None,
                                    });
                                }
                            } else if crdt_only {
                                if let Ok(proof) = resp
                                    .json::<saorsa_quic_test::harness::CrdtProofData>()
                                    .await
                                {
                                    all_proofs.push(ProofsResponse {
                                        agent_id,
                                        timestamp_ms,
                                        connectivity: None,
                                        gossip: None,
                                        crdt: Some(proof),
                                    });
                                }
                            }
                        } else {
                            match resp.json::<ProofsResponse>().await {
                                Ok(proofs) => {
                                    info!("Collected proofs from {}", proofs.agent_id);
                                    all_proofs.push(proofs);
                                }
                                Err(e) => {
                                    failures.push((url.clone(), format!("Parse error: {}", e)));
                                }
                            }
                        }
                    }
                    Ok(resp) => failures.push((url.clone(), format!("HTTP {}", resp.status()))),
                    Err(e) => failures.push((url.clone(), e.to_string())),
                }
            }

            let output_str = serde_json::to_string_pretty(&serde_json::json!({
                "proofs": all_proofs,
                "failures": failures,
                "collected_at": chrono::Utc::now().to_rfc3339(),
            }))?;

            if let Some(output_path) = output {
                std::fs::write(&output_path, &output_str)?;
                info!("Proofs written to {:?}", output_path);
            } else {
                println!("{}", output_str);
            }

            // Verify all nodes see each other
            if !connectivity_only && !gossip_only && !crdt_only {
                let mut all_pass = true;
                for proof in &all_proofs {
                    if let Some(ref conn) = proof.connectivity {
                        if !conn.pass {
                            warn!("Agent {} failed connectivity proof", proof.agent_id);
                            all_pass = false;
                        }
                    }
                    if let Some(ref gossip) = proof.gossip {
                        if !gossip.pass {
                            warn!("Agent {} failed gossip proof", proof.agent_id);
                            all_pass = false;
                        }
                    }
                    if let Some(ref crdt) = proof.crdt {
                        if !crdt.pass {
                            warn!("Agent {} failed CRDT proof", proof.agent_id);
                            all_pass = false;
                        }
                    }
                }

                if all_pass {
                    info!("All {} agents passed all proofs", all_proofs.len());
                } else {
                    warn!("Some agents failed proofs - check output for details");
                    std::process::exit(1);
                }
            }
        }
    }

    Ok(())
}

/// Format duration in human-readable form
fn format_duration(secs: u64) -> String {
    if secs < 60 {
        format!("{}s", secs)
    } else if secs < 3600 {
        format!("{}m", secs / 60)
    } else if secs < 86400 {
        format!("{}h", secs / 3600)
    } else {
        format!("{}d", secs / 86400)
    }
}
