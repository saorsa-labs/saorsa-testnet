// Copyright 2024 Saorsa Labs Limited
//
// Saorsa TestNet - Comprehensive P2P Network Testing Tool

#![warn(clippy::unwrap_used)]
#![warn(clippy::expect_used)]
#![warn(clippy::panic)]

use anyhow::Result;
use clap::{Parser, Subcommand};
use colored::*;
use std::net::SocketAddr;
use std::path::PathBuf;
use std::time::Duration;
use tracing::info;

mod node;
mod metrics;
mod tui;
mod deployment;
mod testing;
mod logging;

use node::{BootstrapNode, WorkerNode};
use tui::Dashboard;
use deployment::DigitalOceanDeployer;
use testing::TestScenario;

#[derive(Parser)]
#[command(
    name = "saorsa-testnet",
    about = "Comprehensive testing and monitoring tool for Saorsa P2P network",
    version,
    author
)]
struct Cli {
    /// Enable verbose logging
    #[arg(short, long, global = true)]
    verbose: bool,

    /// Log output directory
    #[arg(long, global = true, default_value = "./logs")]
    log_dir: PathBuf,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Start a bootstrap node
    Bootstrap {
        /// Port to listen on
        #[arg(short, long, default_value = "9000")]
        port: u16,

        /// Enable metrics collection
        #[arg(short, long)]
        metrics: bool,

        /// Metrics port
        #[arg(long, default_value = "9090")]
        metrics_port: u16,

        /// Enable post-quantum cryptography
        #[arg(long)]
        pqc: bool,
    },

    /// Start a worker node
    Worker {
        /// Bootstrap node address
        #[arg(short, long)]
        bootstrap: SocketAddr,

        /// Enable TUI dashboard
        #[arg(short, long)]
        tui: bool,

        /// Enable metrics collection
        #[arg(short, long)]
        metrics: bool,

        /// Metrics port
        #[arg(long, default_value = "9091")]
        metrics_port: u16,

        /// Node identifier (auto-generated if not provided)
        #[arg(long)]
        id: Option<String>,

        /// Enable chat interface
        #[arg(long)]
        chat: bool,
    },

    /// Deploy nodes to DigitalOcean
    Deploy {
        /// Regions to deploy to (comma-separated)
        #[arg(short, long, value_delimiter = ',')]
        regions: Vec<String>,

        /// Number of nodes per region
        #[arg(short, long, default_value = "5")]
        nodes_per_region: usize,

        /// SSH key path for authentication
        #[arg(long, default_value = "~/.ssh/id_ed25519")]
        ssh_key: PathBuf,

        /// DigitalOcean API token (can also use DO_API_TOKEN env var)
        #[arg(long, env = "DO_API_TOKEN")]
        do_token: Option<String>,

        /// Use existing GitHub release
        #[arg(long)]
        github_release: Option<String>,
    },

    /// Run test scenarios
    Test {
        /// Test scenario to run
        #[arg(short, long)]
        scenario: TestScenarioType,

        /// Test duration
        #[arg(short, long, default_value = "1h", value_parser = parse_duration)]
        duration: Duration,

        /// Export metrics to directory
        #[arg(short, long)]
        export_metrics: Option<PathBuf>,

        /// Number of test nodes to spawn
        #[arg(short, long, default_value = "10")]
        nodes: usize,

        /// Enable churn simulation
        #[arg(long)]
        churn: bool,

        /// Churn rate (nodes per minute)
        #[arg(long, default_value = "2")]
        churn_rate: usize,
    },

    /// Monitor remote cluster
    Monitor {
        /// Cluster name or address
        #[arg(short, long)]
        cluster: String,

        /// Refresh interval
        #[arg(short, long, default_value = "5s", value_parser = parse_duration)]
        refresh: Duration,

        /// SSH key for remote access
        #[arg(long, default_value = "~/.ssh/id_ed25519")]
        ssh_key: PathBuf,

        /// Export logs to directory
        #[arg(long)]
        export_logs: Option<PathBuf>,
    },

    /// Show network statistics
    Stats {
        /// Output format (json, csv, table)
        #[arg(short, long, default_value = "table")]
        format: OutputFormat,

        /// Include detailed metrics
        #[arg(short, long)]
        detailed: bool,
    },
}

#[derive(Clone, Debug, clap::ValueEnum)]
enum TestScenarioType {
    NatTraversal,
    Churn,
    Stress,
    Geographic,
    All,
}

#[derive(Clone, Debug, clap::ValueEnum)]
enum OutputFormat {
    Json,
    Csv,
    Table,
}

fn parse_duration(s: &str) -> Result<Duration, String> {
    let s = s.trim();
    
    // Parse number and unit
    let (num_str, unit) = s.split_at(
        s.find(|c: char| c.is_alphabetic())
            .ok_or_else(|| format!("Invalid duration format: {}", s))?
    );
    
    let num: u64 = num_str.parse()
        .map_err(|_| format!("Invalid number: {}", num_str))?;
    
    let duration = match unit {
        "s" | "sec" | "secs" => Duration::from_secs(num),
        "m" | "min" | "mins" => Duration::from_secs(num * 60),
        "h" | "hr" | "hrs" => Duration::from_secs(num * 3600),
        "d" | "day" | "days" => Duration::from_secs(num * 86400),
        _ => return Err(format!("Unknown time unit: {}", unit)),
    };
    
    Ok(duration)
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();
    
    // Initialize logging
    logging::init(cli.verbose, &cli.log_dir)?;
    
    // Print banner
    print_banner();
    
    // Execute command
    match cli.command {
        Commands::Bootstrap { port, metrics, metrics_port, pqc } => {
            info!("Starting bootstrap node on port {}", port);
            let mut node = BootstrapNode::new(port, metrics, metrics_port, pqc).await?;
            node.run().await?;
        }
        
        Commands::Worker { bootstrap, tui, metrics, metrics_port, id, chat } => {
            info!("Starting worker node, connecting to {}", bootstrap);
            let node = WorkerNode::new(
                bootstrap,
                id,
                metrics,
                metrics_port,
                chat,
            ).await?;
            
            if tui {
                let dashboard = Dashboard::new(node);
                dashboard.run().await?;
            } else {
                node.run().await?;
            }
        }
        
        Commands::Deploy { regions, nodes_per_region, ssh_key, do_token, github_release } => {
            info!("Deploying {} nodes per region to {:?}", nodes_per_region, regions);
            let deployer = DigitalOceanDeployer::new(do_token, ssh_key)?;
            deployer.deploy(regions, nodes_per_region, github_release).await?;
        }
        
        Commands::Test { scenario, duration, export_metrics, nodes, churn, churn_rate } => {
            info!("Running {:?} test scenario for {:?}", scenario, duration);
            let mut test = TestScenario::new(scenario, duration, nodes);
            
            if churn {
                test.enable_churn(churn_rate);
            }
            
            let results = test.run().await?;
            
            if let Some(path) = export_metrics {
                results.export(&path)?;
            }
            
            results.print_summary();
        }
        
        Commands::Monitor { cluster, refresh, ssh_key, export_logs } => {
            info!("Monitoring cluster: {}", cluster);
            let monitor = deployment::ClusterMonitor::new(cluster, ssh_key)?;
            monitor.run(refresh, export_logs).await?;
        }
        
        Commands::Stats { format, detailed } => {
            let stats = metrics::collect_stats(detailed).await?;
            
            match format {
                OutputFormat::Json => {
                    println!("{}", serde_json::to_string_pretty(&stats)?);
                }
                OutputFormat::Csv => {
                    stats.write_csv(&mut std::io::stdout())?;
                }
                OutputFormat::Table => {
                    stats.print_table();
                }
            }
        }
    }
    
    Ok(())
}

fn print_banner() {
    println!("{}", "╔════════════════════════════════════════════════╗".cyan().bold());
    println!("{}", "║        SAORSA TESTNET - P2P Testing Tool      ║".cyan().bold());
    println!("{}", "╠════════════════════════════════════════════════╣".cyan().bold());
    println!("{}", "║  NAT Traversal │ Adaptive Network │ Metrics   ║".cyan());
    println!("{}", "║  Post-Quantum  │ ML Optimization  │ Dashboard ║".cyan());
    println!("{}", "╚════════════════════════════════════════════════╝".cyan().bold());
    println!();
}