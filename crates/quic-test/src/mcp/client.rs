//! MCP Client Implementation
//!
//! Provides a high-level client for interacting with Communitas via Command/Query API.

use communitas_core::{
    app::CommunitasApp,
    command::{Command, Event, Query, QueryResponse},
    conn_from_words, generate_id_words,
};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, error, info, warn};

/// MCP tool categories for UI organization
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum McpToolCategory {
    /// Authentication and vault management (8 tools)
    Auth,
    /// Entity/group/channel management (16 tools)
    Entities,
    /// Messaging and threads (14 tools)
    Messages,
    /// File operations (6 tools)
    Files,
    /// Kanban project management (23 tools)
    Kanban,
    /// Network and DHT operations (26 tools)
    Network,
    /// Social features and calls (10 tools)
    Social,
}

impl McpToolCategory {
    /// Get all categories in display order
    pub fn all() -> &'static [McpToolCategory] {
        &[
            McpToolCategory::Auth,
            McpToolCategory::Entities,
            McpToolCategory::Messages,
            McpToolCategory::Files,
            McpToolCategory::Kanban,
            McpToolCategory::Network,
            McpToolCategory::Social,
        ]
    }

    /// Get display name for category
    pub fn display_name(&self) -> &'static str {
        match self {
            McpToolCategory::Auth => "Auth",
            McpToolCategory::Entities => "Entities",
            McpToolCategory::Messages => "Messages",
            McpToolCategory::Files => "Files",
            McpToolCategory::Kanban => "Kanban",
            McpToolCategory::Network => "Network",
            McpToolCategory::Social => "Social",
        }
    }

    /// Get tool count for category
    pub fn tool_count(&self) -> usize {
        match self {
            McpToolCategory::Auth => 8,
            McpToolCategory::Entities => 16,
            McpToolCategory::Messages => 14,
            McpToolCategory::Files => 6,
            McpToolCategory::Kanban => 23,
            McpToolCategory::Network => 26,
            McpToolCategory::Social => 10,
        }
    }

    /// Categorize a tool by name
    pub fn from_tool_name(name: &str) -> Self {
        match name {
            // Auth tools
            n if n.starts_with("authenticate")
                || n.starts_with("create_vault")
                || n.starts_with("list_vault")
                || n.starts_with("delete_vault")
                || n.starts_with("import_vault")
                || n.starts_with("export_vault")
                || n == "health_check"
                || n == "core_status"
                || n == "get_session"
                || n == "logout" =>
            {
                McpToolCategory::Auth
            }

            // Entity tools
            n if n.contains("entity") || n.contains("member") || n == "join_entity" => {
                McpToolCategory::Entities
            }

            // Message tools
            n if n.contains("message")
                || n.contains("thread")
                || n.contains("reaction")
                || n == "get_unread_count" =>
            {
                McpToolCategory::Messages
            }

            // File tools
            n if n.contains("file") || n == "get_disk_stats" || n.contains("upload") => {
                McpToolCategory::Files
            }

            // Kanban tools
            n if n.contains("kanban")
                || n.contains("card")
                || n.contains("column")
                || n.contains("board")
                || n.contains("step")
                || n.contains("tag") =>
            {
                McpToolCategory::Kanban
            }

            // Network tools
            n if n.starts_with("network_")
                || n.starts_with("dht_")
                || n.contains("metrics")
                || n.contains("trust")
                || n.contains("placement")
                || n.contains("presence") =>
            {
                McpToolCategory::Network
            }

            // Social tools
            n if n.contains("poll")
                || n.contains("story")
                || n.contains("call")
                || n.contains("location")
                || n.contains("screen")
                || n.contains("presentation")
                || n.contains("website")
                || n.contains("workspace") =>
            {
                McpToolCategory::Social
            }

            // Default to Network for unknown
            _ => McpToolCategory::Network,
        }
    }
}

/// Information about an MCP tool
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolInfo {
    /// Tool name (e.g., "send_message")
    pub name: String,
    /// Human-readable description
    pub description: String,
    /// Tool category
    pub category: McpToolCategory,
    /// Parameter schema (JSON Schema)
    pub parameters: Option<Value>,
}

/// Configuration for MCP client
#[derive(Debug, Clone)]
pub struct McpClientConfig {
    /// Storage directory for user data
    pub storage_dir: PathBuf,
    /// Display name for the demo user
    pub display_name: String,
    /// Device name for identification
    pub device_name: String,
    /// Optional pre-set four-word identity (auto-generated if None)
    pub four_words: Option<String>,
}

impl Default for McpClientConfig {
    fn default() -> Self {
        Self {
            storage_dir: std::env::temp_dir().join("saorsa-quic-test"),
            display_name: "Saorsa Demo User".to_string(),
            device_name: hostname::get()
                .map(|h| h.to_string_lossy().to_string())
                .unwrap_or_else(|_| "unknown".to_string()),
            four_words: None,
        }
    }
}

/// Result of an MCP tool invocation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolResult {
    /// Whether the tool succeeded
    pub success: bool,
    /// Result data (if success)
    pub data: Option<Value>,
    /// Error message (if failure)
    pub error: Option<String>,
    /// Execution time in milliseconds
    pub duration_ms: u64,
}

/// Presence information for display
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PresenceInfo {
    /// Hex-encoded public key
    pub pubkey: String,
    /// Connection words (four-word encoded IP:port)
    pub connection_words: String,
    /// Unix timestamp when presence was announced
    pub timestamp: u64,
}

/// MCP Client for Communitas integration
pub struct McpClient {
    /// The underlying Communitas app
    app: Arc<RwLock<Option<CommunitasApp>>>,
    /// User's four-word identity
    four_words: String,
    /// Configuration
    config: McpClientConfig,
    /// Whether networking has started
    networking_started: bool,
    /// Connection identity (four-word encoded address)
    connection_identity: Option<String>,
}

impl McpClient {
    /// Create a new MCP client with auto-generated demo identity
    pub async fn new(config: McpClientConfig) -> Result<Self, String> {
        // Generate four-word identity if not provided
        let four_words = match &config.four_words {
            Some(fw) => fw.clone(),
            None => generate_id_words().map_err(|e| format!("Failed to generate identity: {e}"))?,
        };

        info!(
            four_words = %four_words,
            display_name = %config.display_name,
            "Creating MCP client with demo identity"
        );

        // Ensure storage directory exists
        std::fs::create_dir_all(&config.storage_dir)
            .map_err(|e| format!("Failed to create storage directory: {e}"))?;

        // Create the Communitas app
        let app = CommunitasApp::new(
            four_words.clone(),
            config.display_name.clone(),
            config.device_name.clone(),
            config.storage_dir.to_string_lossy().to_string(),
        )
        .await
        .map_err(|e| format!("Failed to create CommunitasApp: {e}"))?;

        info!(four_words = %four_words, "MCP client initialized successfully");

        Ok(Self {
            app: Arc::new(RwLock::new(Some(app))),
            four_words,
            config,
            networking_started: false,
            connection_identity: None,
        })
    }

    /// Get the user's four-word identity
    pub fn four_words(&self) -> &str {
        &self.four_words
    }

    /// Get the connection identity (four-word encoded address)
    pub fn connection_identity(&self) -> Option<&str> {
        self.connection_identity.as_deref()
    }

    /// Check if networking is started
    pub fn is_networking_started(&self) -> bool {
        self.networking_started
    }

    /// Get the storage directory
    pub fn storage_dir(&self) -> &PathBuf {
        &self.config.storage_dir
    }

    /// Start networking (required before P2P operations)
    pub async fn start_networking(
        &mut self,
        preferred_port: Option<u16>,
    ) -> Result<String, String> {
        let app_guard = self.app.read().await;
        let app = app_guard
            .as_ref()
            .ok_or_else(|| "App not initialized".to_string())?;

        let result = app
            .execute(Command::StartNetworking { preferred_port })
            .await;

        match result {
            Ok(events) => {
                self.networking_started = true;
                // Look for connection identity in events
                for event in &events {
                    if let Some(conn_id) = extract_connection_identity(event) {
                        self.connection_identity = Some(conn_id.clone());
                        info!(connection_identity = %conn_id, "Networking started");
                    }
                }
                Ok(format!("Networking started with {} events", events.len()))
            }
            Err(e) => {
                error!(error = ?e, "Failed to start networking");
                Err(format!("{e:?}"))
            }
        }
    }

    /// Execute a command
    pub async fn execute(&self, command: Command) -> ToolResult {
        let start = std::time::Instant::now();

        let app_guard = self.app.read().await;
        let app = match app_guard.as_ref() {
            Some(app) => app,
            None => {
                return ToolResult {
                    success: false,
                    data: None,
                    error: Some("App not initialized".to_string()),
                    duration_ms: start.elapsed().as_millis() as u64,
                };
            }
        };

        debug!(command = ?command, "Executing command");

        let result = app.execute(command).await;
        let duration_ms = start.elapsed().as_millis() as u64;

        match result {
            Ok(events) => {
                debug!(event_count = %events.len(), duration_ms = %duration_ms, "Command succeeded");
                ToolResult {
                    success: true,
                    data: Some(serde_json::to_value(&events).unwrap_or(Value::Null)),
                    error: None,
                    duration_ms,
                }
            }
            Err(e) => {
                warn!(error = ?e, duration_ms = %duration_ms, "Command failed");
                ToolResult {
                    success: false,
                    data: None,
                    error: Some(format!("{e:?}")),
                    duration_ms,
                }
            }
        }
    }

    /// Execute a query
    pub async fn query(&self, query: Query) -> Result<QueryResponse, String> {
        let app_guard = self.app.read().await;
        let app = app_guard
            .as_ref()
            .ok_or_else(|| "App not initialized".to_string())?;

        debug!(query = ?query, "Executing query");

        app.query(query).await.map_err(|e| format!("{e:?}"))
    }

    /// Get list of available tools by category
    pub fn get_tools_by_category(&self, category: McpToolCategory) -> Vec<ToolInfo> {
        get_tools_for_category(category)
    }

    /// Get all available tools
    pub fn get_all_tools(&self) -> Vec<ToolInfo> {
        let mut tools = Vec::new();
        for category in McpToolCategory::all() {
            tools.extend(get_tools_for_category(*category));
        }
        tools
    }

    // =====================================================
    // Presence Methods
    // =====================================================

    /// Announce our presence to connected peers
    ///
    /// Broadcasts a signed PresenceRecord containing our current connection
    /// words to the gossip network so other peers can find us.
    pub async fn announce_presence(&self) -> Result<String, String> {
        let result = self.execute(Command::AnnouncePresence).await;

        if result.success {
            // Try to get connection words to include in response
            if let Ok(QueryResponse::OptionalString(Some(words))) =
                self.query(Query::GetConnectionWords).await
            {
                Ok(format!("Presence announced: {}", words))
            } else {
                Ok("Presence announced".to_string())
            }
        } else {
            Err(result
                .error
                .unwrap_or_else(|| "Failed to announce presence".to_string()))
        }
    }

    /// Query for a peer's presence by their public key
    ///
    /// Sends a PresenceQuery to find another peer's current location.
    /// The pubkey can be provided as hex or base64 encoded.
    ///
    /// Returns `Ok(Some(info))` if presence found, `Ok(None)` if not found,
    /// or `Err` on failure.
    pub async fn query_presence(&self, pubkey: String) -> Result<Option<PresenceInfo>, String> {
        // Parse the pubkey from hex or base64
        let pubkey_bytes = parse_pubkey(&pubkey)?;

        let app_guard = self.app.read().await;
        let app = app_guard
            .as_ref()
            .ok_or_else(|| "App not initialized".to_string())?;

        let result = app
            .execute(Command::QueryPeerPresence {
                target_pubkey: pubkey_bytes,
            })
            .await;

        match result {
            Ok(events) => {
                // Check for PeerPresenceReceived event
                for event in &events {
                    if let Event::PeerPresenceReceived { record } = event {
                        return Ok(Some(PresenceInfo {
                            pubkey: hex::encode(&record.pubkey),
                            connection_words: record.connection_words.clone(),
                            timestamp: record.timestamp,
                        }));
                    }
                }
                // Query sent but no immediate response
                Ok(None)
            }
            Err(e) => Err(format!("Failed to query presence: {}", e.message)),
        }
    }

    /// Get our own presence record
    ///
    /// Returns our most recently created PresenceRecord with current
    /// connection words and timestamp.
    pub async fn get_our_presence(&self) -> Result<PresenceInfo, String> {
        let response = self.query(Query::GetOurPresenceRecord).await?;

        match response {
            QueryResponse::OurPresenceRecord(Some(record)) => Ok(PresenceInfo {
                pubkey: hex::encode(&record.pubkey),
                connection_words: record.connection_words,
                timestamp: record.timestamp,
            }),
            QueryResponse::OurPresenceRecord(None) => {
                Err("No presence record available. Start networking first.".to_string())
            }
            _ => Err("Unexpected response type".to_string()),
        }
    }

    /// Get cached presence for a peer
    ///
    /// Checks our local presence cache for a known peer's location.
    /// This does not send a network query.
    pub async fn get_cached_presence(
        &self,
        pubkey: String,
    ) -> Result<Option<PresenceInfo>, String> {
        let pubkey_bytes = parse_pubkey(&pubkey)?;

        let response = self
            .query(Query::GetCachedPeerPresence {
                pubkey: pubkey_bytes,
            })
            .await?;

        match response {
            QueryResponse::CachedPeerPresence(Some(record)) => Ok(Some(PresenceInfo {
                pubkey: hex::encode(&record.pubkey),
                connection_words: record.connection_words,
                timestamp: record.timestamp,
            })),
            QueryResponse::CachedPeerPresence(None) => Ok(None),
            _ => Err("Unexpected response type".to_string()),
        }
    }

    // =====================================================
    // Contact Management Methods
    // =====================================================

    /// Create a new contact from a four-word identity
    pub async fn create_contact(
        &self,
        four_words: String,
        display_name: Option<String>,
    ) -> Result<String, String> {
        let result = self
            .execute(Command::CreateContact {
                display_name: display_name.unwrap_or_else(|| four_words.clone()),
                four_words: Some(four_words),
                is_favourite: false,
            })
            .await;

        if result.success {
            Ok("Contact created".to_string())
        } else {
            Err(result.error.unwrap_or_else(|| "Unknown error".to_string()))
        }
    }

    /// List all contacts
    pub async fn list_contacts(&self) -> Result<Vec<ContactInfo>, String> {
        let response = self.query(Query::ListContacts).await?;

        match response {
            QueryResponse::ContactList(contacts) => Ok(contacts
                .into_iter()
                .map(|c| ContactInfo {
                    id: c.id,
                    display_name: c.display_name,
                    four_words: c.four_words,
                    is_favourite: c.is_favourite,
                    is_online: c.is_online,
                    last_seen: c.last_seen,
                })
                .collect()),
            _ => Err("Unexpected response type".to_string()),
        }
    }

    /// Get a specific contact by ID
    pub async fn get_contact(&self, contact_id: String) -> Result<ContactInfo, String> {
        let response = self
            .query(Query::GetContact {
                contact_id: contact_id.clone(),
            })
            .await?;

        match response {
            QueryResponse::Contact(c) => Ok(ContactInfo {
                id: c.id,
                display_name: c.display_name,
                four_words: c.four_words,
                is_favourite: c.is_favourite,
                is_online: c.is_online,
                last_seen: c.last_seen,
            }),
            _ => Err("Contact not found".to_string()),
        }
    }

    /// Search contacts by query string
    pub async fn search_contacts(&self, query: String) -> Result<Vec<ContactInfo>, String> {
        let response = self.query(Query::SearchContacts { query }).await?;

        match response {
            QueryResponse::ContactList(contacts) => Ok(contacts
                .into_iter()
                .map(|c| ContactInfo {
                    id: c.id,
                    display_name: c.display_name,
                    four_words: c.four_words,
                    is_favourite: c.is_favourite,
                    is_online: c.is_online,
                    last_seen: c.last_seen,
                })
                .collect()),
            _ => Err("Unexpected response type".to_string()),
        }
    }

    /// Set a contact as favourite
    pub async fn set_favourite(&self, four_words: String) -> Result<(), String> {
        let result = self
            .execute(Command::SetFavouriteContact { four_words })
            .await;

        if result.success {
            Ok(())
        } else {
            Err(result.error.unwrap_or_else(|| "Unknown error".to_string()))
        }
    }

    /// Delete a contact
    pub async fn delete_contact(&self, contact_id: String) -> Result<(), String> {
        let result = self.execute(Command::DeleteContact { contact_id }).await;

        if result.success {
            Ok(())
        } else {
            Err(result.error.unwrap_or_else(|| "Unknown error".to_string()))
        }
    }

    /// Connect to a peer using 4-word encoded address
    ///
    /// The words encode an IPv4:port address. After connecting, the peer's
    /// identity packet will be received via the gossip protocol.
    pub async fn connect_by_words(&self, words: String) -> Result<String, String> {
        // Decode the 4 words to a SocketAddr
        let addr =
            conn_from_words(&words).map_err(|e| format!("Invalid connection words: {}", e))?;

        info!("Connecting to {} (decoded from '{}')", addr, words);

        // Connect to the decoded address
        let result = self
            .execute(Command::ConnectToPeer {
                peer_four_words: addr.to_string(),
            })
            .await;

        if result.success {
            Ok(format!("Connecting to {}", addr))
        } else {
            Err(result
                .error
                .unwrap_or_else(|| "Connection failed".to_string()))
        }
    }

    // ==========================================================================
    // Messaging Methods
    // ==========================================================================

    /// Send a direct message to one or more recipients
    pub async fn send_direct_message(
        &self,
        recipients: Vec<String>,
        text: String,
    ) -> Result<String, String> {
        let author = self.four_words.clone();

        let result = self
            .execute(Command::SendDirectMessage {
                recipients,
                text,
                author,
            })
            .await;

        if result.success {
            // Try to extract message ID from events
            if let Some(data) = &result.data {
                if let Some(events) = data.as_array() {
                    for event in events {
                        if let Some(msg_id) = event.get("message_id").and_then(|v| v.as_str()) {
                            return Ok(msg_id.to_string());
                        }
                    }
                }
            }
            Ok("message_sent".to_string())
        } else {
            Err(result.error.unwrap_or_else(|| "Unknown error".to_string()))
        }
    }

    /// Get direct messages with another peer
    pub async fn get_direct_messages(
        &self,
        other_peer_id: String,
    ) -> Result<Vec<MessageInfo>, String> {
        let response = self
            .query(Query::GetDirectMessages { other_peer_id })
            .await?;

        match response {
            QueryResponse::Messages(messages) => Ok(messages
                .into_iter()
                .map(|m| MessageInfo {
                    id: m.id,
                    text: m.text,
                    author: m.author,
                    timestamp: format_timestamp(m.timestamp),
                    edited: m.edited_at.is_some(),
                })
                .collect()),
            _ => Err("Unexpected response type".to_string()),
        }
    }

    /// Get all recent direct messages (for polling new messages)
    /// Returns messages from all conversations, sorted by timestamp
    pub async fn get_recent_messages(&self) -> Result<Vec<MessageInfo>, String> {
        // Get all contacts first
        let contacts = self.list_contacts().await?;
        let mut all_messages = Vec::new();

        // Fetch messages from each contact
        for contact in contacts {
            if let Some(ref four_words) = contact.four_words {
                if let Ok(messages) = self.get_direct_messages(four_words.clone()).await {
                    all_messages.extend(messages);
                }
            }
        }

        // Sort by timestamp (newest last)
        all_messages.sort_by(|a, b| a.timestamp.cmp(&b.timestamp));

        Ok(all_messages)
    }
}

/// Contact information for display
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContactInfo {
    /// Contact ID
    pub id: String,
    /// Display name
    pub display_name: String,
    /// Four-word identity (if linked to network)
    pub four_words: Option<String>,
    /// Whether favourite
    pub is_favourite: bool,
    /// Whether online (based on presence)
    pub is_online: bool,
    /// Last seen timestamp (Unix seconds)
    pub last_seen: Option<i64>,
}

/// Message information for display
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MessageInfo {
    /// Message ID
    pub id: String,
    /// Message text content
    pub text: String,
    /// Author's four-word ID
    pub author: String,
    /// Timestamp (ISO 8601)
    pub timestamp: String,
    /// Whether message has been edited
    pub edited: bool,
}

/// Parse a public key from hex or base64 encoding
fn parse_pubkey(input: &str) -> Result<Vec<u8>, String> {
    // Try hex first (most common)
    if let Ok(bytes) = hex::decode(input) {
        return Ok(bytes);
    }

    // Try base64 standard
    use base64::Engine;
    if let Ok(bytes) = base64::engine::general_purpose::STANDARD.decode(input) {
        return Ok(bytes);
    }

    // Try base64 URL-safe
    if let Ok(bytes) = base64::engine::general_purpose::URL_SAFE.decode(input) {
        return Ok(bytes);
    }

    Err(format!(
        "Invalid pubkey format: expected hex or base64, got '{}'",
        if input.len() > 16 {
            format!("{}...", &input[..16])
        } else {
            input.to_string()
        }
    ))
}

/// Format a Unix timestamp to a human-readable string
fn format_timestamp(timestamp: i64) -> String {
    use chrono::{TimeZone, Utc};

    match Utc.timestamp_opt(timestamp, 0) {
        chrono::LocalResult::Single(dt) => dt.format("%Y-%m-%d %H:%M:%S").to_string(),
        _ => timestamp.to_string(),
    }
}

/// Extract connection identity from an event (if present)
fn extract_connection_identity(event: &Event) -> Option<String> {
    match event {
        Event::NetworkingStarted {
            connection_identity,
            ..
        } => Some(connection_identity.clone()),
        _ => None,
    }
}

/// Get tools for a specific category (static tool definitions)
fn get_tools_for_category(category: McpToolCategory) -> Vec<ToolInfo> {
    let tool = |name: &str, desc: &str| ToolInfo {
        name: name.to_string(),
        description: desc.to_string(),
        category,
        parameters: None,
    };

    match category {
        McpToolCategory::Auth => vec![
            tool("authenticate", "Authenticate with vault password"),
            tool("create_vault", "Create a new identity vault"),
            tool("authenticate_token", "Authenticate with delegate token"),
            tool("health_check", "Check MCP server health"),
            tool("core_status", "Get core system status"),
            tool("list_vaults", "List available vaults"),
            tool("delete_vault", "Delete a vault"),
            tool("import_vault", "Import a vault from backup"),
        ],
        McpToolCategory::Entities => vec![
            tool("create_entity", "Create org/project/group/channel"),
            tool("update_entity", "Update entity details"),
            tool("delete_entity", "Delete an entity"),
            tool("get_entity", "Get entity details"),
            tool("list_entities", "List all entities"),
            tool("add_member", "Add member to entity"),
            tool("remove_member", "Remove member from entity"),
            tool("list_members", "List entity members"),
            tool("join_entity", "Join an entity"),
            tool("create_invite", "Create entity invite"),
            tool("accept_invite", "Accept an invite"),
            tool("list_pending_invites", "List pending invites"),
            tool("get_profile", "Get user profile"),
            tool("update_profile", "Update user profile"),
            tool("create_delegate_token", "Create delegate token"),
            tool("export_vault", "Export vault backup"),
        ],
        McpToolCategory::Messages => vec![
            tool("send_message", "Send a message"),
            tool("delete_message", "Delete a message"),
            tool("edit_message", "Edit a message"),
            tool("get_messages", "Get messages for entity"),
            tool("create_thread", "Create a message thread"),
            tool("get_thread_messages", "Get thread messages"),
            tool("add_reaction", "Add reaction to message"),
            tool("remove_reaction", "Remove reaction"),
            tool("get_reactions", "Get message reactions"),
            tool("create_custom_reaction", "Create custom reaction"),
            tool("get_available_reactions", "List available reactions"),
            tool("get_unread_count", "Get unread message count"),
            tool("create_contact", "Create a contact"),
            tool("list_contacts", "List all contacts"),
        ],
        McpToolCategory::Files => vec![
            tool("write_file", "Write file to virtual disk"),
            tool("read_file", "Read file from virtual disk"),
            tool("list_files", "List files in directory"),
            tool("delete_file", "Delete a file"),
            tool("get_disk_stats", "Get disk usage stats"),
            tool("upload_with_metadata", "Upload with metadata"),
        ],
        McpToolCategory::Kanban => vec![
            tool("create_kanban_board", "Create kanban board"),
            tool("update_kanban_board", "Update board details"),
            tool("delete_kanban_board", "Delete a board"),
            tool("list_kanban_boards", "List all boards"),
            tool("get_kanban_board", "Get board details"),
            tool("create_kanban_card", "Create a card"),
            tool("update_kanban_card", "Update card details"),
            tool("delete_kanban_card", "Delete a card"),
            tool("list_kanban_cards", "List cards on board"),
            tool("move_kanban_card", "Move card between columns"),
            tool("get_kanban_card", "Get card details"),
            tool("create_kanban_column", "Create a column"),
            tool("list_kanban_columns", "List board columns"),
            tool("get_kanban_column", "Get column details"),
            tool("update_kanban_column", "Update column"),
            tool("delete_kanban_column", "Delete a column"),
            tool("move_kanban_column", "Reorder columns"),
            tool("change_card_state", "Change card state"),
            tool("assign_user", "Assign user to card"),
            tool("unassign_user", "Unassign user from card"),
            tool("create_kanban_tag", "Create a tag"),
            tool("list_kanban_tags", "List board tags"),
            tool("tag_card", "Add tag to card"),
        ],
        McpToolCategory::Network => vec![
            tool("network_start", "Start P2P networking"),
            tool("network_stop", "Stop networking"),
            tool("network_status", "Get network status"),
            tool("network_connect", "Connect to peer"),
            tool("network_disconnect", "Disconnect from peer"),
            tool("network_peers", "List connected peers"),
            tool(
                "network_request_external_address",
                "Request external address",
            ),
            tool("get_connection_words", "Get your connection words"),
            tool("connect_by_words", "Connect using 4-word address"),
            tool("announce_presence", "Announce presence to network"),
            tool("query_presence", "Query peer presence by pubkey"),
            tool("get_our_presence", "Get our presence record"),
            tool("get_cached_presence", "Get cached peer presence"),
            tool("dht_start", "Start DHT"),
            tool("dht_stop", "Stop DHT"),
            tool("dht_status", "Get DHT status"),
            tool("dht_store", "Store value in DHT"),
            tool("dht_retrieve", "Retrieve from DHT"),
            tool("dht_exists", "Check if key exists"),
            tool("dht_closest_peers", "Get closest peers to key"),
            tool("dht_health_metrics", "Get DHT health"),
            tool("dht_network_stats", "Get DHT network stats"),
            tool("security_metrics", "Get security metrics"),
            tool("trust_metrics", "Get trust metrics"),
            tool("placement_metrics", "Get placement metrics"),
            tool("metrics_summary", "Get metrics summary"),
        ],
        McpToolCategory::Social => vec![
            tool("create_poll", "Create a poll"),
            tool("vote_in_poll", "Vote in a poll"),
            tool("create_story", "Create a story"),
            tool("get_media_metadata", "Get media metadata"),
            tool("share_location", "Share location"),
            tool("share_screen", "Share screen"),
            tool("start_presentation", "Start presentation"),
            tool("start_voice_call", "Start voice call"),
            tool("join_call", "Join a call"),
            tool("end_call", "End a call"),
        ],
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tool_categorization() {
        assert_eq!(
            McpToolCategory::from_tool_name("authenticate"),
            McpToolCategory::Auth
        );
        assert_eq!(
            McpToolCategory::from_tool_name("create_entity"),
            McpToolCategory::Entities
        );
        assert_eq!(
            McpToolCategory::from_tool_name("send_message"),
            McpToolCategory::Messages
        );
        assert_eq!(
            McpToolCategory::from_tool_name("write_file"),
            McpToolCategory::Files
        );
        assert_eq!(
            McpToolCategory::from_tool_name("create_kanban_board"),
            McpToolCategory::Kanban
        );
        assert_eq!(
            McpToolCategory::from_tool_name("network_start"),
            McpToolCategory::Network
        );
        assert_eq!(
            McpToolCategory::from_tool_name("create_poll"),
            McpToolCategory::Social
        );
        assert_eq!(
            McpToolCategory::from_tool_name("announce_presence"),
            McpToolCategory::Network
        );
    }

    #[test]
    fn test_category_display_names() {
        assert_eq!(McpToolCategory::Auth.display_name(), "Auth");
        assert_eq!(McpToolCategory::Entities.display_name(), "Entities");
    }

    #[test]
    fn test_all_categories() {
        assert_eq!(McpToolCategory::all().len(), 7);
    }

    #[test]
    fn test_parse_pubkey_hex() {
        let hex_key = "0123456789abcdef";
        let result = parse_pubkey(hex_key);
        assert!(result.is_ok());
        assert_eq!(
            result.unwrap(),
            vec![0x01, 0x23, 0x45, 0x67, 0x89, 0xab, 0xcd, 0xef]
        );
    }

    #[test]
    fn test_parse_pubkey_invalid() {
        let invalid = "not_valid_hex_or_base64!!!";
        let result = parse_pubkey(invalid);
        assert!(result.is_err());
    }
}
