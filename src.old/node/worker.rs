// Copyright 2024 Saorsa Labs Limited
//
// Worker node implementation for Saorsa TestNet

use super::{NodeState, SharedNodeState};
use anyhow::{Context, Result};
use prometheus::{Encoder, HistogramVec, IntCounterVec, IntGaugeVec, Opts, Registry, TextEncoder, CounterVec};
use std::collections::HashMap;
use saorsa_core::adaptive::coordinator::{NetworkConfig, NetworkCoordinator};
use saorsa_core::adaptive::NodeIdentity;
use saorsa_core::adaptive::ContentHash;
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;
use tracing::{error, info};

/// Worker node that connects to bootstrap and participates in network
pub struct WorkerNode {
    /// Network coordinator
    coordinator: Arc<NetworkCoordinator>,
    
    /// Node state
    state: SharedNodeState,
    
    /// Bootstrap address
    bootstrap_addr: SocketAddr,
    
    /// Node ID (optional, auto-generated if not provided)
    node_id: String,
    
    /// Metrics enabled
    metrics_enabled: bool,
    
    /// Metrics port
    metrics_port: u16,
    
    /// Chat interface enabled
    chat_enabled: bool,
    
    /// Prometheus registry
    metrics_registry: Option<Registry>,
    
    /// Metrics
    _connections_total: Option<IntCounterVec>,
    _active_connections: Option<IntGaugeVec>,
    _messages_total: Option<IntCounterVec>,
    latency_histogram: Option<HistogramVec>,
    crypto_operations: Option<CounterVec>,
    nat_traversal_attempts: Option<IntCounterVec>,
    data_exchanges: Option<CounterVec>,
    connection_types: Option<IntGaugeVec>,
    
    /// Shared data store for cross-node communication
    shared_data_hashes: Arc<RwLock<HashMap<String, ContentHash>>>,
}

impl WorkerNode {
    /// Create new worker node
    pub async fn new(
        bootstrap: SocketAddr,
        id: Option<String>,
        metrics: bool,
        metrics_port: u16,
        chat: bool,
    ) -> Result<Self> {
        let node_id = id.unwrap_or_else(|| uuid::Uuid::new_v4().to_string());
        info!("Creating worker node with ID: {}", node_id);
        
        // Generate identity
        let identity = NodeIdentity::generate()
            .context("Failed to generate node identity")?;
        
        // Create network config
        let config = NetworkConfig {
            bootstrap_nodes: vec![bootstrap.to_string()],
            ..Default::default()
        };
        
        // Create coordinator
        let coordinator = Arc::new(
            NetworkCoordinator::new(identity, config)
                .await
                .context("Failed to create network coordinator")?
        );
        
        // Setup metrics if enabled
        let (registry, connections_total, active_connections, messages_total, latency_histogram, crypto_operations, nat_traversal_attempts, data_exchanges, connection_types) = if metrics {
            let registry = Registry::new();
            
            let connections = IntCounterVec::new(
                Opts::new("worker_connections_total", "Total connections"),
                &["type"]
            )?;
            registry.register(Box::new(connections.clone()))?;
            
            let active = IntGaugeVec::new(
                Opts::new("worker_active_connections", "Active connections"),
                &["state"]
            )?;
            registry.register(Box::new(active.clone()))?;
            
            let messages = IntCounterVec::new(
                Opts::new("worker_messages_total", "Total messages"),
                &["direction", "type"]
            )?;
            registry.register(Box::new(messages.clone()))?;
            
            let latency = HistogramVec::new(
                Opts::new("worker_message_latency_seconds", "Message latency").into(),
                &["operation"]
            )?;
            registry.register(Box::new(latency.clone()))?;
            
            let crypto_ops = CounterVec::new(
                Opts::new("worker_crypto_operations_total", "Cryptographic operations"),
                &["algorithm", "operation"]
            )?;
            registry.register(Box::new(crypto_ops.clone()))?;
            
            let nat_attempts = IntCounterVec::new(
                Opts::new("worker_nat_traversal_attempts_total", "NAT traversal attempts"),
                &["type", "result"]
            )?;
            registry.register(Box::new(nat_attempts.clone()))?;
            
            let data_exch = CounterVec::new(
                Opts::new("worker_data_exchanges_total", "Random data exchanges"),
                &["peer_type", "data_size"]
            )?;
            registry.register(Box::new(data_exch.clone()))?;
            
            let conn_types = IntGaugeVec::new(
                Opts::new("worker_connection_types", "Active connection types"),
                &["connection_type", "encryption"]
            )?;
            registry.register(Box::new(conn_types.clone()))?;
            
            (Some(registry), Some(connections), Some(active), Some(messages), Some(latency), Some(crypto_ops), Some(nat_attempts), Some(data_exch), Some(conn_types))
        } else {
            (None, None, None, None, None, None, None, None, None)
        };
        
        let state = NodeState {
            node_id: node_id.clone(),
            ..Default::default()
        };
        
        Ok(Self {
            coordinator,
            state: Arc::new(RwLock::new(state)),
            bootstrap_addr: bootstrap,
            node_id,
            metrics_enabled: metrics,
            metrics_port,
            chat_enabled: chat,
            metrics_registry: registry,
            _connections_total: connections_total,
            _active_connections: active_connections,
            _messages_total: messages_total,
            latency_histogram,
            crypto_operations,
            nat_traversal_attempts,
            data_exchanges,
            connection_types,
            shared_data_hashes: Arc::new(RwLock::new(HashMap::new())),
        })
    }
    
    /// Run the worker node
    pub async fn run(self) -> Result<()> {
        info!("Starting worker node {}", self.node_id);
        
        // Connect to bootstrap (simulated)
        info!("Connecting to bootstrap at {}", self.bootstrap_addr);
        
        // Join network through coordinator
        self.coordinator.join_network().await
            .context("Failed to join network")?;
        
        // Start metrics server if enabled
        if self.metrics_enabled {
            self.start_metrics_server().await?;
        }
        
        // Start chat interface if enabled
        if self.chat_enabled {
            info!("Chat interface enabled for worker {}", self.node_id);
        }
        
        // Main worker loop
        self.worker_loop().await?;
        
        Ok(())
    }
    
    /// Main worker loop
    async fn worker_loop(&self) -> Result<()> {
        let start_time = Instant::now();
        let mut network_interval = tokio::time::interval(Duration::from_secs(10));
        let mut random_data_interval = tokio::time::interval(Duration::from_millis(2000 + rand::random::<u64>() % 5000)); // Random 2-7 seconds
        let mut data_reading_interval = tokio::time::interval(Duration::from_millis(3000 + rand::random::<u64>() % 4000)); // Random 3-7 seconds
        
        loop {
            tokio::select! {
                _ = network_interval.tick() => {
                    // Update uptime and perform standard operations
                    {
                        let mut state = self.state.write().await;
                        state.uptime = start_time.elapsed();
                        state.connections = 1; // Connected to bootstrap
                    }
                    
                    // Perform periodic tasks
                    self.perform_network_operations().await?;
                    
                    // Update connection type metrics
                    self.update_connection_metrics().await?;
                }
                
                _ = random_data_interval.tick() => {
                    // Randomly send data to other nodes
                    if rand::random::<f64>() < 0.7 { // 70% chance to send data
                        self.send_random_data().await?;
                    }
                    
                    // Reset interval to random value
                    random_data_interval = tokio::time::interval(Duration::from_millis(1500 + rand::random::<u64>() % 6000));
                }
                
                _ = data_reading_interval.tick() => {
                    // Read data from other nodes
                    if rand::random::<f64>() < 0.6 { // 60% chance to read data
                        self.read_shared_data().await?;
                    }
                    
                    // Reset interval to random value
                    data_reading_interval = tokio::time::interval(Duration::from_millis(2500 + rand::random::<u64>() % 5000));
                }
                
                _ = tokio::signal::ctrl_c() => {
                    info!("Shutdown signal received");
                    break;
                }
            }
        }
        
        Ok(())
    }
    
    /// Perform periodic network operations
    async fn perform_network_operations(&self) -> Result<()> {
        // Store some test data
        let test_data = format!("test_data_from_{}", self.node_id);
        let hash = self.coordinator.store(test_data.as_bytes().to_vec()).await?;
        
        // Update metrics
        if let Some(counter) = &self._messages_total {
            counter.with_label_values(&["sent", "store"]).inc();
        }
        
        // Simulate cryptographic operation
        if let Some(crypto_counter) = &self.crypto_operations {
            crypto_counter.with_label_values(&["ML-KEM-768", "encapsulate"]).inc();
        }
        
        // Retrieve the data
        let start = Instant::now();
        let _retrieved = self.coordinator.retrieve(&hash).await?;
        let latency = start.elapsed();
        
        // Update metrics
        if let Some(counter) = &self._messages_total {
            counter.with_label_values(&["received", "retrieve"]).inc();
        }
        if let Some(histogram) = &self.latency_histogram {
            histogram.with_label_values(&["retrieve"])
                .observe(latency.as_secs_f64());
        }
        
        // Simulate signature verification
        if let Some(crypto_counter) = &self.crypto_operations {
            crypto_counter.with_label_values(&["ML-DSA-65", "verify"]).inc();
        }
        
        // Publish to gossip
        let message = format!("gossip_from_{}", self.node_id);
        self.coordinator.publish("test_topic", message.as_bytes().to_vec()).await?;
        
        // Update metrics
        if let Some(counter) = &self._messages_total {
            counter.with_label_values(&["sent", "gossip"]).inc();
        }
        
        // Update state
        {
            let mut state = self.state.write().await;
            state.messages_sent += 3;
            state.messages_received += 1;
            state.bandwidth_used += test_data.len() as u64 * 2 + message.len() as u64;
        }
        
        Ok(())
    }
    
    /// Send random data to other nodes
    async fn send_random_data(&self) -> Result<()> {
        let data_types = ["telemetry", "heartbeat", "discovery", "routing_update", "challenge"];
        let data_type = data_types[rand::random::<usize>() % data_types.len()];
        
        // Generate random data of varying sizes
        let size = match data_type {
            "telemetry" => 256 + rand::random::<usize>() % 512,
            "heartbeat" => 32 + rand::random::<usize>() % 64,
            "discovery" => 128 + rand::random::<usize>() % 256,
            "routing_update" => 64 + rand::random::<usize>() % 128,
            "challenge" => 512 + rand::random::<usize>() % 1024,
            _ => 128,
        };
        
        let random_data: Vec<u8> = (0..size).map(|_| rand::random::<u8>()).collect();
        let peer_types = ["bootstrap", "worker", "relay"];
        let peer_type = peer_types[rand::random::<usize>() % peer_types.len()];
        
        info!("Sending {} bytes of {} data to {} peer", size, data_type, peer_type);
        
        // Store the random data (simulating sending to peer)
        let data_key = format!("random_{}_{}", self.node_id, rand::random::<u32>());
        let hash = self.coordinator.store(random_data.clone()).await?;
        
        // Store the hash in shared data for other nodes to read
        {
            let mut shared_data = self.shared_data_hashes.write().await;
            shared_data.insert(data_key, hash);
        }
        
        // Update metrics
        if let Some(counter) = &self.data_exchanges {
            let size_category = match size {
                0..=128 => "small",
                129..=512 => "medium", 
                _ => "large",
            };
            counter.with_label_values(&[peer_type, size_category]).inc();
        }
        
        // Simulate NAT traversal attempt
        if rand::random::<f64>() < 0.3 { // 30% chance of NAT traversal
            let nat_types = ["full_cone", "restricted", "port_restricted", "symmetric"];
            let nat_type = nat_types[rand::random::<usize>() % nat_types.len()];
            let success = rand::random::<f64>() > 0.2; // 80% success rate
            
            if let Some(counter) = &self.nat_traversal_attempts {
                counter.with_label_values(&[nat_type, if success { "success" } else { "failure" }]).inc();
            }
        }
        
        // Simulate cryptographic operations
        if let Some(crypto_counter) = &self.crypto_operations {
            // Random crypto operations
            let operations = [
                ("ChaCha20Poly1305", "encrypt"),
                ("ML-KEM-768", "decapsulate"),
                ("X25519", "derive"),
                ("BLAKE3", "hash"),
            ];
            let (alg, op) = operations[rand::random::<usize>() % operations.len()];
            crypto_counter.with_label_values(&[alg, op]).inc();
        }
        
        // Update state
        {
            let mut state = self.state.write().await;
            state.messages_sent += 1;
            state.bandwidth_used += random_data.len() as u64;
        }
        
        Ok(())
    }
    
    /// Read shared data from other nodes
    async fn read_shared_data(&self) -> Result<()> {
        // Get a random hash from shared data
        let hash_to_read = {
            let shared_data = self.shared_data_hashes.read().await;
            if shared_data.is_empty() {
                return Ok(()); // No data to read yet
            }
            
            let keys: Vec<_> = shared_data.keys().collect();
            let random_key = keys[rand::random::<usize>() % keys.len()];
            *shared_data.get(random_key).ok_or_else(|| anyhow::anyhow!("Key not found in shared data"))?
        };
        
        info!("Reading shared data from DHT: {:?}", hash_to_read);
        
        // Try to retrieve the data
        let start = Instant::now();
        match self.coordinator.retrieve(&hash_to_read).await {
            Ok(data) => {
                let latency = start.elapsed();
                info!("Successfully read {} bytes from peer ({}ms)", data.len(), latency.as_millis());
                
                // Update metrics
                if let Some(histogram) = &self.latency_histogram {
                    histogram.with_label_values(&["cross_node_read"])
                        .observe(latency.as_secs_f64());
                }
                
                if let Some(counter) = &self._messages_total {
                    counter.with_label_values(&["received", "cross_node"]).inc();
                }
                
                // Simulate cryptographic verification
                if let Some(crypto_counter) = &self.crypto_operations {
                    crypto_counter.with_label_values(&["BLAKE3", "verify"]).inc();
                }
                
                // Update state
                {
                    let mut state = self.state.write().await;
                    state.messages_received += 1;
                    state.bandwidth_used += data.len() as u64;
                }
            }
            Err(e) => {
                info!("Failed to read shared data: {}", e);
            }
        }
        
        Ok(())
    }
    
    /// Update connection type metrics
    async fn update_connection_metrics(&self) -> Result<()> {
        if let Some(gauge) = &self.connection_types {
            // Simulate different connection types
            let connection_configs = vec![
                ("quic", "post_quantum"),
                ("tcp", "tls13"),
                ("udp", "quantum_resistant"),
                ("relay", "encrypted"),
            ];
            
            for (conn_type, encryption) in connection_configs {
                let count = 1 + rand::random::<i64>() % 3; // 1-3 connections
                gauge.with_label_values(&[conn_type, encryption]).set(count);
            }
        }
        
        Ok(())
    }
    
    /// Start metrics server
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
            info!("Worker metrics server listening on http://{}", addr);
            
            if let Err(e) = server.await {
                error!("Metrics server error: {}", e);
            }
        });
        
        Ok(())
    }
    
    /// Get current state for monitoring
    #[allow(dead_code)]
    pub fn get_state(&self) -> SharedNodeState {
        self.state.clone()
    }
}