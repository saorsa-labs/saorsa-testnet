//! Web dashboard for the ant-quic test network.
//!
//! Provides a Three.js globe visualization and real-time statistics.
//!
//! # Dashboard Pages
//!
//! - `/` - Globe visualization (index.html)
//! - `/overview` - Overview page with proofs, stats, peers
//! - `/gossip` - Gossip health (HyParView, SWIM, Plumtree)
//! - `/matrix` - Connectivity matrix (N×N)
//! - `/log` - Protocol log (real-time frames)
//!
//! # API Endpoints
//!
//! - `GET /api/stats` - Network statistics
//! - `GET /api/peers` - All registered peers
//! - `GET /api/overview` - Aggregated overview data
//! - `GET /api/connections` - Connection history with directional stats
//! - `GET /api/frames` - Recent protocol frames
//! - `GET /api/gossip` - Gossip protocol health

pub mod types;

pub use types::*;

use rust_embed::Embed;
use std::sync::Arc;
use warp::Filter;

use crate::registry::PeerStore;

/// Embedded static files from the static/ directory.
#[derive(Embed)]
#[folder = "static/"]
struct StaticFiles;

/// Create dashboard routes.
pub fn dashboard_routes(
    store: Arc<PeerStore>,
) -> impl Filter<Extract = impl warp::Reply, Error = warp::Rejection> + Clone {
    // Page routes
    let index = warp::path::end().and_then(serve_index);
    let overview = warp::path("overview")
        .and(warp::path::end())
        .and_then(serve_overview);
    let gossip = warp::path("gossip")
        .and(warp::path::end())
        .and_then(serve_gossip);
    let matrix = warp::path("matrix")
        .and(warp::path::end())
        .and_then(serve_matrix);
    let log = warp::path("log").and(warp::path::end()).and_then(serve_log);

    // Static files
    let static_files = warp::path("static")
        .and(warp::path::tail())
        .and_then(serve_static);

    // Existing API endpoints
    let api_stats = warp::path!("api" / "stats")
        .and(warp::get())
        .and(with_store(store.clone()))
        .and_then(get_stats);

    let api_peers = warp::path!("api" / "peers")
        .and(warp::get())
        .and(with_store(store.clone()))
        .and_then(get_peers);

    // New API endpoints
    let api_overview = warp::path!("api" / "overview")
        .and(warp::get())
        .and(with_store(store.clone()))
        .and_then(get_overview);

    let api_connections = warp::path!("api" / "connections")
        .and(warp::get())
        .and(with_store(store.clone()))
        .and_then(get_connections);

    let api_frames = warp::path!("api" / "frames")
        .and(warp::get())
        .and(warp::query::<FramesQuery>())
        .and(with_store(store.clone()))
        .and_then(get_frames);

    let api_gossip = warp::path!("api" / "gossip")
        .and(warp::get())
        .and(with_store(store.clone()))
        .and_then(get_gossip);

    // WebSocket
    let ws_live = warp::path!("ws" / "live")
        .and(warp::ws())
        .and(with_store(store))
        .map(|ws: warp::ws::Ws, store: Arc<PeerStore>| {
            ws.on_upgrade(move |socket| handle_websocket(socket, store))
        });

    // Combine routes in groups to avoid type recursion issues
    // Box intermediate groups to break the deeply nested Or<Or<Or<...>>> type chain
    let pages = index
        .or(overview)
        .or(gossip)
        .or(matrix)
        .or(log)
        .boxed();

    let api = api_stats
        .or(api_peers)
        .or(api_overview)
        .or(api_connections)
        .or(api_frames)
        .or(api_gossip)
        .boxed();

    pages.or(static_files).or(api).or(ws_live)
}

/// Query parameters for frames endpoint.
#[derive(Debug, serde::Deserialize)]
pub struct FramesQuery {
    /// Maximum number of frames to return (default: 200)
    #[serde(default = "default_limit")]
    pub limit: usize,
}

fn default_limit() -> usize {
    200
}

fn with_store(
    store: Arc<PeerStore>,
) -> impl Filter<Extract = (Arc<PeerStore>,), Error = std::convert::Infallible> + Clone {
    warp::any().map(move || store.clone())
}

async fn serve_index() -> Result<impl warp::Reply, warp::Rejection> {
    match StaticFiles::get("index.html") {
        Some(content) => Ok(warp::reply::html(
            String::from_utf8_lossy(content.data.as_ref()).to_string(),
        )),
        None => Err(warp::reject::not_found()),
    }
}

async fn serve_overview() -> Result<impl warp::Reply, warp::Rejection> {
    serve_html_page("overview.html").await
}

async fn serve_gossip() -> Result<impl warp::Reply, warp::Rejection> {
    serve_html_page("gossip.html").await
}

async fn serve_matrix() -> Result<impl warp::Reply, warp::Rejection> {
    serve_html_page("matrix.html").await
}

async fn serve_log() -> Result<impl warp::Reply, warp::Rejection> {
    serve_html_page("log.html").await
}

async fn serve_html_page(filename: &str) -> Result<impl warp::Reply, warp::Rejection> {
    match StaticFiles::get(filename) {
        Some(content) => Ok(warp::reply::html(
            String::from_utf8_lossy(content.data.as_ref()).to_string(),
        )),
        None => Err(warp::reject::not_found()),
    }
}

async fn serve_static(path: warp::path::Tail) -> Result<impl warp::Reply, warp::Rejection> {
    let path = path.as_str();
    match StaticFiles::get(path) {
        Some(content) => {
            let mime = mime_guess::from_path(path).first_or_octet_stream();
            Ok(warp::reply::with_header(
                content.data.to_vec(),
                "Content-Type",
                mime.as_ref(),
            ))
        }
        None => Err(warp::reject::not_found()),
    }
}

async fn get_stats(store: Arc<PeerStore>) -> Result<impl warp::Reply, warp::Rejection> {
    let stats = store.get_stats();
    Ok(warp::reply::json(&stats))
}

async fn get_peers(store: Arc<PeerStore>) -> Result<impl warp::Reply, warp::Rejection> {
    let peers = store.get_all_peers();
    Ok(warp::reply::json(&peers))
}

/// Get aggregated overview data for the overview page.
async fn get_overview(store: Arc<PeerStore>) -> Result<impl warp::Reply, warp::Rejection> {
    let response = store.get_overview_data();
    Ok(warp::reply::json(&response))
}

/// Get connection history for the connectivity matrix.
async fn get_connections(store: Arc<PeerStore>) -> Result<impl warp::Reply, warp::Rejection> {
    let response = store.get_connections_data();
    Ok(warp::reply::json(&response))
}

/// Get recent protocol frames for the log display.
async fn get_frames(
    query: FramesQuery,
    store: Arc<PeerStore>,
) -> Result<impl warp::Reply, warp::Rejection> {
    let response = store.get_frames_data(query.limit);
    Ok(warp::reply::json(&response))
}

/// Get gossip protocol health data.
async fn get_gossip(store: Arc<PeerStore>) -> Result<impl warp::Reply, warp::Rejection> {
    let response = store.get_gossip_data();
    Ok(warp::reply::json(&response))
}

async fn handle_websocket(ws: warp::ws::WebSocket, store: Arc<PeerStore>) {
    use futures_util::{SinkExt, StreamExt};
    use tokio::time::{Duration, interval};

    let (mut tx, mut rx) = ws.split();

    // Send initial full state
    let initial_state = serde_json::json!({
        "type": "full_state",
        "nodes": store.get_all_peers(),
        "stats": store.get_stats(),
    });

    if tx
        .send(warp::ws::Message::text(initial_state.to_string()))
        .await
        .is_err()
    {
        return;
    }

    let mut event_rx = store.subscribe();

    // Spawn task to forward events to WebSocket
    let forward_task = tokio::spawn(async move {
        while let Ok(event) = event_rx.recv().await {
            let msg = match event {
                crate::registry::NetworkEvent::NodeRegistered {
                    peer_id,
                    country_code,
                    latitude,
                    longitude,
                } => serde_json::json!({
                    "type": "node_registered",
                    "peer_id": peer_id,
                    "country_code": country_code,
                    "latitude": latitude,
                    "longitude": longitude,
                }),
                crate::registry::NetworkEvent::NodeOffline { peer_id } => serde_json::json!({
                    "type": "node_offline",
                    "peer_id": peer_id,
                }),
                crate::registry::NetworkEvent::ConnectionEstablished {
                    from_peer,
                    to_peer,
                    method,
                    rtt_ms,
                } => serde_json::json!({
                    "type": "connection_established",
                    "from_peer": from_peer,
                    "to_peer": to_peer,
                    "method": format!("{:?}", method).to_lowercase(),
                    "rtt_ms": rtt_ms,
                }),
                crate::registry::NetworkEvent::StatsUpdate(stats) => serde_json::json!({
                    "type": "stats_update",
                    "stats": stats,
                }),
                crate::registry::NetworkEvent::ConnectivityTestRequest {
                    peer_id,
                    addresses,
                    relay_addr,
                    timestamp_ms,
                } => serde_json::json!({
                    "type": "connectivity_test_request",
                    "peer_id": peer_id,
                    "addresses": addresses.iter().map(|a| a.to_string()).collect::<Vec<_>>(),
                    "relay_addr": relay_addr.map(|a| a.to_string()),
                    "timestamp_ms": timestamp_ms,
                }),
            };

            if tx
                .send(warp::ws::Message::text(msg.to_string()))
                .await
                .is_err()
            {
                break;
            }
        }
    });

    // Keep connection alive with pings and handle incoming messages
    let mut ping_interval = interval(Duration::from_secs(30));

    loop {
        tokio::select! {
            _ = ping_interval.tick() => {
                // Ping is handled by warp internally
            }
            msg = rx.next() => {
                match msg {
                    Some(Ok(msg)) => {
                        if msg.is_close() {
                            break;
                        }
                    }
                    Some(Err(_)) | None => break,
                }
            }
        }
    }

    forward_task.abort();
}
