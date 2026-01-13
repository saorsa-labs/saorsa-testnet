// Copyright 2024 Saorsa Labs Limited
//
// Bootstrap node implementation for Saorsa TestNet

use super::{NodeState, SharedNodeState};
use anyhow::{Context, Result};
use prometheus::{Encoder, IntCounterVec, IntGaugeVec, Opts, Registry, TextEncoder};
use saorsa_core::adaptive::coordinator::{NetworkConfig, NetworkCoordinator};
use saorsa_core::adaptive::NodeIdentity;
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;
use tracing::{error, info};

/// Bootstrap node that serves as initial contact point
pub struct BootstrapNode {
    /// Network coordinator
    coordinator: Arc<NetworkCoordinator>,
    
    /// Node state
    state: SharedNodeState,
    
    /// Port to listen on
    port: u16,
    
    /// Metrics enabled
    metrics_enabled: bool,
    
    /// Metrics port
    metrics_port: u16,
    
    /// Post-quantum crypto enabled
    pqc_enabled: bool,
    
    /// Prometheus registry
    metrics_registry: Option<Registry>,
    
    /// Metrics
    connections_total: Option<IntCounterVec>,
    _active_connections: Option<IntGaugeVec>,
    _messages_total: Option<IntCounterVec>,
}

impl BootstrapNode {
    /// Create new bootstrap node
    pub async fn new(
        port: u16,
        metrics: bool,
        metrics_port: u16,
        pqc: bool,
    ) -> Result<Self> {
        info!("Creating bootstrap node on port {}", port);
        
        // Generate identity
        let identity = NodeIdentity::generate()
            .context("Failed to generate node identity")?;
        
        // Create network config
        let mut config = NetworkConfig::default();
        config.bootstrap_nodes.clear(); // Bootstrap doesn't connect to others
        config.max_connections = 1000; // Higher for bootstrap
        
        // Create coordinator
        let coordinator = Arc::new(
            NetworkCoordinator::new(identity, config)
                .await
                .context("Failed to create network coordinator")?
        );
        
        // Setup metrics if enabled
        let (registry, connections_total, active_connections, messages_total) = if metrics {
            let registry = Registry::new();
            
            let connections = IntCounterVec::new(
                Opts::new("bootstrap_connections_total", "Total connections handled"),
                &["type"]
            )?;
            registry.register(Box::new(connections.clone()))?;
            
            let active = IntGaugeVec::new(
                Opts::new("bootstrap_active_connections", "Currently active connections"),
                &["state"]
            )?;
            registry.register(Box::new(active.clone()))?;
            
            let messages = IntCounterVec::new(
                Opts::new("bootstrap_messages_total", "Total messages processed"),
                &["direction"]
            )?;
            registry.register(Box::new(messages.clone()))?;
            
            (Some(registry), Some(connections), Some(active), Some(messages))
        } else {
            (None, None, None, None)
        };
        
        Ok(Self {
            coordinator,
            state: Arc::new(RwLock::new(NodeState::default())),
            port,
            metrics_enabled: metrics,
            metrics_port,
            pqc_enabled: pqc,
            metrics_registry: registry,
            connections_total,
            _active_connections: active_connections,
            _messages_total: messages_total,
        })
    }
    
    /// Start the bootstrap node
    pub async fn run(&mut self) -> Result<()> {
        info!("Starting bootstrap node on port {} (PQC: {})", self.port, self.pqc_enabled);
        
        // Join network (bootstrap doesn't connect to others, just listens)
        self.coordinator.join_network().await
            .context("Failed to join network")?;
        
        // Start metrics server if enabled
        if self.metrics_enabled {
            self.start_metrics_server().await?;
        }
        
        // Main bootstrap loop
        self.bootstrap_loop().await?;
        
        Ok(())
    }
    
    /// Main bootstrap event loop
    async fn bootstrap_loop(&self) -> Result<()> {
        let start_time = Instant::now();
        let mut interval = tokio::time::interval(Duration::from_secs(10));
        
        info!("Bootstrap node running and accepting connections");
        
        loop {
            interval.tick().await;
            
            // Update uptime
            {
                let mut state = self.state.write().await;
                state.uptime = start_time.elapsed();
                // Simulate some activity
                state.messages_received += rand::random::<u64>() % 10;
                state.bandwidth_used += rand::random::<u64>() % 1024;
            }
            
            // Update metrics
            if let Some(counter) = &self.connections_total {
                if rand::random::<f64>() < 0.1 {
                    counter.with_label_values(&["accepted"]).inc();
                }
            }
            
            // Check for shutdown signal
            if tokio::signal::ctrl_c().await.is_ok() {
                info!("Shutdown signal received");
                break;
            }
        }
        
        Ok(())
    }
    
    /// Start Prometheus metrics server
    async fn start_metrics_server(&self) -> Result<()> {
        let registry = self.metrics_registry.clone()
            .ok_or_else(|| anyhow::anyhow!("Metrics not initialized"))?;
        
        let metrics_port = self.metrics_port;
        
        tokio::spawn(async move {
            let addr = SocketAddr::from(([0, 0, 0, 0], metrics_port));
            
            let make_svc = hyper::service::make_service_fn(move |_conn| {
                let registry = registry.clone();
                async move {
                    Ok::<_, hyper::Error>(hyper::service::service_fn(move |_req| {
                        let registry = registry.clone();
                        async move {
                            let encoder = TextEncoder::new();
                            let metric_families = registry.gather();
                            let mut buffer = Vec::new();
                            if let Err(e) = encoder.encode(&metric_families, &mut buffer) {
                                error!("Failed to encode metrics: {}", e);
                                return Ok::<_, hyper::Error>(hyper::Response::new(hyper::Body::from("Internal Server Error")));
                            }
                            
                            Ok::<_, hyper::Error>(hyper::Response::new(hyper::Body::from(buffer)))
                        }
                    }))
                }
            });
            
            let server = hyper::Server::bind(&addr).serve(make_svc);
            info!("Bootstrap metrics server listening on http://{}", addr);
            
            if let Err(e) = server.await {
                error!("Metrics server error: {}", e);
            }
        });
        
        Ok(())
    }
}