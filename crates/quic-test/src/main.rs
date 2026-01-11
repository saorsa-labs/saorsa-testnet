//! ant-quic Test Network Binary
//!
//! "We will be legion!!"
//!
//! This binary provides both registry server and test node functionality
//! for the large-scale ant-quic network testing infrastructure.

use saorsa_quic_test::{
    TestNode,
    node::TestNodeConfig,
    proof_orchestrator::{ProofOrchestrator, ProofOrchestratorConfig},
    registry::{RegistryConfig, start_registry_server},
    tui::{App, TuiEvent, run_tui},
};
use std::net::SocketAddr;
use tokio::sync::mpsc;

/// Command-line arguments for the test network binary.
#[derive(Debug)]
struct Args {
    /// Run as registry server
    registry: bool,
    /// Run proof-based network test
    proof_test: bool,
    /// HTTP server port (for registry mode)
    port: u16,
    /// QUIC port for address discovery (registry mode, 0 to disable)
    quic_port: u16,
    /// QUIC bind port (for client mode)
    bind_port: u16,
    /// Registry URL to connect to (for client mode)
    registry_url: String,
    /// Maximum peer connections
    max_peers: usize,
    /// Disable TUI (log mode only)
    quiet: bool,
    /// Local-only mode: Disable external VPS connections (for Docker/local testing)
    local_only: bool,
    /// Minimum nodes required for proof test
    min_proof_nodes: usize,
    /// Gossip-first mode: Use epidemic gossip for peer discovery instead of registry
    gossip_first: bool,
}

impl Default for Args {
    fn default() -> Self {
        Self {
            registry: false,
            proof_test: false,
            port: 8080,
            quic_port: 9001, // Registry QUIC port for address discovery (9001 to avoid conflict with P2P node on 9000)
            bind_port: 0,    // 0 = random available port
            registry_url: "https://saorsa-1.saorsalabs.com".to_string(),
            max_peers: 10,
            quiet: false,
            local_only: false, // Disabled by default - connect to external VPS nodes
            min_proof_nodes: 2,
            gossip_first: true, // Enabled by default - use epidemic gossip for peer discovery
        }
    }
}

fn parse_args() -> Args {
    let mut args = Args::default();
    let mut argv = std::env::args().skip(1);

    while let Some(arg) = argv.next() {
        match arg.as_str() {
            "--registry" => args.registry = true,
            "--port" => {
                if let Some(port) = argv.next() {
                    if let Ok(p) = port.parse() {
                        args.port = p;
                    }
                }
            }
            "--registry-url" => {
                if let Some(url) = argv.next() {
                    args.registry_url = url;
                }
            }
            "--max-peers" => {
                if let Some(max) = argv.next() {
                    if let Ok(m) = max.parse() {
                        args.max_peers = m;
                    }
                }
            }
            "--bind-port" => {
                if let Some(port) = argv.next() {
                    if let Ok(p) = port.parse() {
                        args.bind_port = p;
                    }
                }
            }
            "--quic-port" => {
                if let Some(port) = argv.next() {
                    if let Ok(p) = port.parse() {
                        args.quic_port = p;
                    }
                }
            }
            "-q" | "--quiet" => args.quiet = true,
            "--local-only" => args.local_only = true,
            "--proof-test" => args.proof_test = true,
            "--gossip-first" => args.gossip_first = true,
            "--no-gossip-first" => args.gossip_first = false,
            "--min-proof-nodes" => {
                if let Some(n) = argv.next() {
                    if let Ok(num) = n.parse() {
                        args.min_proof_nodes = num;
                    }
                }
            }
            "-h" | "--help" => {
                print_help();
                std::process::exit(0);
            }
            _ => {
                eprintln!("Unknown argument: {}", arg);
                print_help();
                std::process::exit(1);
            }
        }
    }

    args
}

fn print_help() {
    println!(
        r#"
ant-quic Test Network - "We will be legion!!"

Large-scale network testing for quantum-secure P2P connectivity.

USAGE:
    ant-quic-test [OPTIONS]

OPTIONS:
    --registry              Run as central registry server
    --proof-test            Run proof-based network verification test
    --port <PORT>           HTTP server port (registry mode) [default: 8080]
    --quic-port <PORT>      QUIC port for address discovery (registry mode, 0 to disable) [default: 9001]
    --bind-port <PORT>      QUIC UDP bind port (client mode) [default: 0 = random]
    --registry-url <URL>    Registry URL to connect to [default: https://saorsa-1.saorsalabs.com]
    --max-peers <N>         Maximum peer connections [default: 10]
    --min-proof-nodes <N>   Minimum nodes for proof test [default: 2]
    --local-only            Disable external VPS connections (for Docker/local testing)
    --gossip-first          Use epidemic gossip for peer discovery (default: enabled)
    --no-gossip-first       Use registry-based peer discovery instead of gossip
    -q, --quiet             Disable TUI, log mode only
    -h, --help              Print this help message

EXAMPLES:
    # Run as registry server
    ant-quic-test --registry --port 8080

    # Run as test node (default mode, random port)
    ant-quic-test

    # Run proof-based verification test
    ant-quic-test --proof-test --registry-url https://saorsa-1.saorsalabs.com

    # Run multiple local instances (each on different random ports)
    ant-quic-test &
    ant-quic-test &

    # Run on specific port
    ant-quic-test --bind-port 9001

    # Connect to custom registry
    ant-quic-test --registry-url https://my-registry.example.com
"#
    );
}

// Use 8 worker threads for better parallelism in network operations
// Note: std::sync locks have been replaced with parking_lot locks to prevent deadlocks
#[tokio::main(flavor = "multi_thread", worker_threads = 8)]
async fn main() -> anyhow::Result<()> {
    // CRITICAL: Install rustls crypto provider before any TLS/QUIC operations
    // This must happen early, before TestNode::new() which uses rustls internally.
    // Using aws-lc-rs as the default provider for FIPS-compliant cryptography.
    let _ = rustls::crypto::aws_lc_rs::default_provider().install_default();

    let args = parse_args();

    // Only initialize logging for non-TUI modes (registry or quiet)
    // TUI mode handles its own display - tracing to stderr ruins the interface
    if args.registry || args.quiet {
        tracing_subscriber::fmt::init();
    }

    if args.registry {
        // Run as registry server
        println!("Starting registry server on port {}...", args.port);
        println!("\"We will be legion!!\"");

        let quic_addr = if args.quic_port > 0 {
            Some(
                format!("[::]:{}", args.quic_port)
                    .parse()
                    .expect("valid QUIC address"),
            )
        } else {
            None
        };

        let config = RegistryConfig {
            bind_addr: format!("[::]:{}", args.port)
                .parse()
                .expect("valid bind address"),
            // QUIC endpoint for native address discovery via OBSERVED_ADDRESS frames
            quic_addr,
            ttl_secs: 120,
            cleanup_interval_secs: 30,
            data_dir: std::path::PathBuf::from("./data"),
            persistence_enabled: true,
        };

        start_registry_server(config).await?;
    } else if args.proof_test {
        // Run proof-based network verification test
        println!("Starting proof-based network verification test...");
        println!("Registry: {}", args.registry_url);
        println!("Minimum nodes required: {}", args.min_proof_nodes);
        println!("\"We will be legion!!\"");
        println!();

        run_proof_test(&args).await?;
    } else {
        // Run as test node with TUI
        println!("Starting ant-quic test node...");
        if args.gossip_first {
            println!("Mode: Gossip-first peer discovery (epidemic gossip)");
            println!("Bootstrap: Connecting to hardcoded VPS peers");
        } else {
            println!("Mode: Registry-based peer discovery");
            println!("Registry: {}", args.registry_url);
        }
        println!("\"We will be legion!!\"");

        // Create event channel for TUI updates
        // Use large capacity (1000) to prevent event drops during high activity periods
        let (event_tx, event_rx) = mpsc::channel::<TuiEvent>(1000);

        // Create TUI application
        let app = App::new();

        // Use dual-stack (IPv6 + IPv4) by binding to [::] instead of 0.0.0.0
        // On most systems, [::]:port accepts both IPv4 and IPv6 connections
        let bind_addr: SocketAddr = format!("[::]:{}", args.bind_port).parse()?;
        let node_config = TestNodeConfig {
            registry_url: args.registry_url.clone(),
            max_peers: args.max_peers,
            bind_addr,
            local_only: args.local_only,
            gossip_first: args.gossip_first,
            ..Default::default()
        };

        let tui_event_tx = event_tx.clone();
        let test_node = TestNode::new(node_config, event_tx).await?;

        let use_quiet_mode = args.quiet || !std::io::IsTerminal::is_terminal(&std::io::stdout());

        if use_quiet_mode && !args.quiet {
            eprintln!("INFO: No TTY detected, falling back to quiet mode");
            // CRITICAL: Initialize tracing for auto-detected quiet mode
            // The initial check at line 159 only inits when --quiet is explicit,
            // but when running as a systemd service without TTY, we also need logging!
            tracing_subscriber::fmt::init();
        }

        if use_quiet_mode {
            // Quiet mode: run without TUI
            println!("Running in quiet mode (no TUI)...");
            println!("Press Ctrl+C to quit");

            // CRITICAL: Spawn a task to drain the event channel
            // Without this, the channel fills up (capacity 100) and send().await blocks,
            // causing heartbeat and other background tasks to hang!
            tokio::spawn(async move {
                let mut rx = event_rx;
                while rx.recv().await.is_some() {
                    // Just drain the events, don't process them
                }
            });

            // Run test node directly
            test_node.run().await?;
        } else {
            // Spawn the test node in the background
            let node_handle = tokio::spawn(async move {
                if let Err(e) = test_node.run().await {
                    tracing::error!("Test node error: {}", e);
                }
            });

            // Run TUI in foreground
            run_tui(app, event_rx, tui_event_tx).await?;

            // When TUI exits, abort the node
            node_handle.abort();
        }
    }

    Ok(())
}

/// Convert NodeGossipStats to epidemic_gossip::GossipStats for the proof orchestrator.
fn convert_gossip_stats(
    node_stats: &saorsa_quic_test::registry::NodeGossipStats,
) -> saorsa_quic_test::epidemic_gossip::GossipStats {
    use saorsa_quic_test::epidemic_gossip::{
        ConnectionBreakdown, CoordinatorStats, CrdtStats, GossipStats, GroupStats, HyParViewStats,
        PlumtreeStats, RendezvousStats, SwimStats,
    };

    GossipStats {
        hyparview: HyParViewStats {
            active_view_size: node_stats.hyparview_active,
            passive_view_size: node_stats.hyparview_passive,
            shuffles: node_stats.hyparview_shuffles,
            joins: node_stats.hyparview_joins,
        },
        swim: SwimStats {
            alive_count: node_stats.swim_alive,
            suspect_count: node_stats.swim_suspect,
            dead_count: node_stats.swim_dead,
            pings_sent: node_stats.swim_pings_sent,
            acks_received: node_stats.swim_acks_received,
        },
        plumtree: PlumtreeStats {
            eager_peers: node_stats.plumtree_eager,
            lazy_peers: node_stats.plumtree_lazy,
            messages_sent: node_stats.plumtree_sent,
            messages_received: node_stats.plumtree_received,
            duplicates: 0,
            grafts: 0,
            prunes: 0,
        },
        connection_types: ConnectionBreakdown {
            direct_ipv4: node_stats.conn_direct_ipv4,
            direct_ipv6: node_stats.conn_direct_ipv6,
            hole_punched: node_stats.conn_hole_punched,
            relayed: node_stats.conn_relayed,
        },
        crdt: CrdtStats {
            entries: node_stats.crdt_entries,
            merges: node_stats.crdt_merges,
            vector_clock_len: node_stats.crdt_vector_clock_len,
            last_sync_age_secs: 0,
        },
        coordinator: CoordinatorStats {
            is_coordinator: node_stats.coordinator_active > 0,
            active_coordinators: node_stats.coordinator_active,
            coordination_success: node_stats.coordinator_success,
            coordination_failed: node_stats.coordinator_failed,
        },
        groups: GroupStats {
            groups_count: node_stats.groups_count,
            total_members: node_stats.groups_total_members,
        },
        rendezvous: RendezvousStats {
            registrations: node_stats.rendezvous_registrations,
            discoveries: node_stats.rendezvous_discoveries,
            active_providers: node_stats.rendezvous_points,
        },
    }
}

/// Run proof-based network verification test.
async fn run_proof_test(args: &Args) -> anyhow::Result<()> {
    use saorsa_quic_test::registry::RegistryClient;

    println!("Fetching peer list from registry...");

    // Create registry client
    let client = RegistryClient::new(&args.registry_url);

    // Get current peers from registry
    let peers = client.get_peers().await?;

    if peers.len() < args.min_proof_nodes {
        println!(
            "Insufficient nodes: found {} but need at least {}",
            peers.len(),
            args.min_proof_nodes
        );
        println!("Waiting for more nodes to join...");

        // Poll until we have enough nodes
        let mut attempts = 0;
        loop {
            tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
            let current_peers = client.get_peers().await?;
            if current_peers.len() >= args.min_proof_nodes {
                println!("Found {} nodes, proceeding with test", current_peers.len());
                break;
            }
            attempts += 1;
            if attempts > 60 {
                // 5 minute timeout
                anyhow::bail!(
                    "Timeout waiting for nodes. Have {} but need {}",
                    current_peers.len(),
                    args.min_proof_nodes
                );
            }
            println!(
                "Still waiting... {} nodes found (need {})",
                current_peers.len(),
                args.min_proof_nodes
            );
        }
    }

    // Refresh peer list
    let peers = client.get_peers().await?;
    println!("Found {} nodes for testing", peers.len());
    println!();

    // Create proof orchestrator
    let mut orchestrator = ProofOrchestrator::with_config(ProofOrchestratorConfig {
        observer_id: "proof-test-cli".to_string(),
        min_nodes: args.min_proof_nodes,
        debug_on_failure: true,
        ..Default::default()
    });

    // Register all known peers and record their gossip stats
    println!("Registering nodes and collecting gossip stats...");
    let mut nodes_with_gossip = 0;
    for peer in &peers {
        orchestrator.register_node(peer.peer_id.clone());
        // Build connection list from peer's known peers
        // For now, we assume each peer can see all others
        let other_peers: Vec<String> = peers
            .iter()
            .filter(|p| p.peer_id != peer.peer_id)
            .map(|p| p.peer_id.clone())
            .collect();
        orchestrator.record_connections(&peer.peer_id, other_peers);

        // Record gossip stats if available
        if let Some(ref node_stats) = peer.gossip_stats {
            let gossip_stats = convert_gossip_stats(node_stats);
            println!(
                "  Node {}...: HyParView active={}, SWIM alive={}, Plumtree eager={}",
                &peer.peer_id[..8.min(peer.peer_id.len())],
                gossip_stats.hyparview.active_view_size,
                gossip_stats.swim.alive_count,
                gossip_stats.plumtree.eager_peers,
            );
            orchestrator.record_gossip_stats(&peer.peer_id, gossip_stats);
            nodes_with_gossip += 1;
        } else {
            println!(
                "  Node {}...: No gossip stats available",
                &peer.peer_id[..8.min(peer.peer_id.len())],
            );
        }
    }
    println!(
        "Collected gossip stats from {}/{} nodes",
        nodes_with_gossip,
        peers.len()
    );

    // Run comprehensive test
    println!("Running proof-based verification...");
    println!();
    let report = orchestrator.run_comprehensive_test();

    // Print report
    println!("{}", report);

    // Return status code based on result
    if report.passed {
        println!("All verifications PASSED!");
        Ok(())
    } else {
        println!("Some verifications FAILED. See report above for details.");
        // Return error to indicate failure
        anyhow::bail!("Proof-based test failed")
    }
}
