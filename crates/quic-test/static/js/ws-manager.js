/**
 * WebSocket Manager for Dashboard Pages
 *
 * Provides automatic reconnection, event handling, and state management
 * for real-time dashboard updates.
 */

class WebSocketManager {
  constructor(options = {}) {
    this.url = options.url || `ws://${window.location.host}/ws/live`;
    this.reconnectInterval = options.reconnectInterval || 3000;
    this.maxReconnectAttempts = options.maxReconnectAttempts || 10;

    this.ws = null;
    this.reconnectAttempts = 0;
    this.handlers = new Map();
    this.connected = false;
    this.initialStateReceived = false;

    // State storage
    this.nodes = [];
    this.stats = {};

    // Status callback
    this.onStatusChange = options.onStatusChange || (() => {});
  }

  /**
   * Connect to WebSocket server
   */
  connect() {
    if (this.ws && this.ws.readyState === WebSocket.OPEN) {
      return;
    }

    this.ws = new WebSocket(this.url);

    this.ws.onopen = () => {
      console.log('WebSocket connected');
      this.connected = true;
      this.reconnectAttempts = 0;
      this.onStatusChange('connected');
    };

    this.ws.onclose = () => {
      console.log('WebSocket disconnected');
      this.connected = false;
      this.initialStateReceived = false;
      this.onStatusChange('disconnected');
      this.scheduleReconnect();
    };

    this.ws.onerror = (error) => {
      console.error('WebSocket error:', error);
      this.onStatusChange('error');
    };

    this.ws.onmessage = (event) => {
      this.handleMessage(event.data);
    };
  }

  /**
   * Schedule reconnection attempt
   */
  scheduleReconnect() {
    if (this.reconnectAttempts >= this.maxReconnectAttempts) {
      console.log('Max reconnect attempts reached');
      this.onStatusChange('failed');
      return;
    }

    this.reconnectAttempts++;
    console.log(`Reconnecting in ${this.reconnectInterval}ms (attempt ${this.reconnectAttempts})`);

    setTimeout(() => {
      this.connect();
    }, this.reconnectInterval);
  }

  /**
   * Handle incoming WebSocket message
   */
  handleMessage(data) {
    try {
      const message = JSON.parse(data);
      const type = message.type;

      // Update internal state for common message types
      switch (type) {
        case 'full_state':
          this.nodes = message.nodes || [];
          this.stats = message.stats || {};
          this.initialStateReceived = true;
          break;

        case 'node_registered':
          this.handleNodeRegistered(message);
          break;

        case 'node_offline':
          this.handleNodeOffline(message);
          break;

        case 'stats_update':
          this.stats = message.stats;
          break;

        case 'connection_established':
          // Could update connection tracking
          break;
      }

      // Call registered handlers
      const handlers = this.handlers.get(type) || [];
      handlers.forEach(handler => handler(message));

      // Also call wildcard handlers
      const wildcardHandlers = this.handlers.get('*') || [];
      wildcardHandlers.forEach(handler => handler(message));

    } catch (error) {
      console.error('Error parsing WebSocket message:', error);
    }
  }

  /**
   * Handle node registration event
   */
  handleNodeRegistered(message) {
    const existingIndex = this.nodes.findIndex(n => n.peer_id === message.peer_id);
    const nodeInfo = {
      peer_id: message.peer_id,
      country_code: message.country_code,
      latitude: message.latitude,
      longitude: message.longitude,
    };

    if (existingIndex >= 0) {
      this.nodes[existingIndex] = { ...this.nodes[existingIndex], ...nodeInfo };
    } else {
      this.nodes.push(nodeInfo);
    }
  }

  /**
   * Handle node offline event
   */
  handleNodeOffline(message) {
    this.nodes = this.nodes.filter(n => n.peer_id !== message.peer_id);
  }

  /**
   * Register event handler
   * @param {string} eventType - Event type to handle ('full_state', 'node_registered', etc.)
   * @param {function} handler - Handler function
   */
  on(eventType, handler) {
    if (!this.handlers.has(eventType)) {
      this.handlers.set(eventType, []);
    }
    this.handlers.get(eventType).push(handler);
  }

  /**
   * Remove event handler
   */
  off(eventType, handler) {
    if (this.handlers.has(eventType)) {
      const handlers = this.handlers.get(eventType);
      const index = handlers.indexOf(handler);
      if (index >= 0) {
        handlers.splice(index, 1);
      }
    }
  }

  /**
   * Disconnect WebSocket
   */
  disconnect() {
    if (this.ws) {
      this.ws.close();
      this.ws = null;
    }
  }

  /**
   * Get current connection status
   */
  isConnected() {
    return this.connected;
  }

  /**
   * Get cached nodes
   */
  getNodes() {
    return this.nodes;
  }

  /**
   * Get cached stats
   */
  getStats() {
    return this.stats;
  }
}

/**
 * Dashboard Data Manager
 *
 * Handles fetching data from API endpoints and caching.
 */
class DashboardData {
  constructor() {
    this.cache = new Map();
    this.cacheExpiry = 5000; // 5 seconds
  }

  /**
   * Fetch data from API endpoint with caching
   */
  async fetch(endpoint, options = {}) {
    const cacheKey = endpoint;
    const cached = this.cache.get(cacheKey);

    if (cached && Date.now() - cached.timestamp < this.cacheExpiry && !options.fresh) {
      return cached.data;
    }

    try {
      const response = await fetch(endpoint);
      if (!response.ok) {
        throw new Error(`HTTP ${response.status}`);
      }

      const data = await response.json();

      this.cache.set(cacheKey, {
        data,
        timestamp: Date.now()
      });

      return data;
    } catch (error) {
      console.error(`Error fetching ${endpoint}:`, error);
      throw error;
    }
  }

  /**
   * Fetch overview data
   */
  async getOverview() {
    return this.fetch('/api/overview');
  }

  /**
   * Fetch connections data
   */
  async getConnections() {
    return this.fetch('/api/connections');
  }

  /**
   * Fetch protocol frames
   */
  async getFrames(limit = 200) {
    return this.fetch(`/api/frames?limit=${limit}`);
  }

  /**
   * Fetch gossip data
   */
  async getGossip() {
    return this.fetch('/api/gossip');
  }

  /**
   * Fetch basic stats
   */
  async getStats() {
    return this.fetch('/api/stats');
  }

  /**
   * Fetch peers
   */
  async getPeers() {
    return this.fetch('/api/peers');
  }

  /**
   * Clear cache
   */
  clearCache() {
    this.cache.clear();
  }
}

/**
 * Utility functions for dashboard rendering
 */
const DashboardUtils = {
  /**
   * Format duration in seconds to human readable string
   */
  formatDuration(secs) {
    if (secs < 60) return `${secs}s`;
    if (secs < 3600) return `${Math.floor(secs / 60)}m`;
    if (secs < 86400) return `${Math.floor(secs / 3600)}h`;
    return `${Math.floor(secs / 86400)}d`;
  },

  /**
   * Format bytes to human readable string
   */
  formatBytes(bytes) {
    if (bytes < 1024) return `${bytes} B`;
    if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`;
    if (bytes < 1024 * 1024 * 1024) return `${(bytes / (1024 * 1024)).toFixed(1)} MB`;
    return `${(bytes / (1024 * 1024 * 1024)).toFixed(2)} GB`;
  },

  /**
   * Format number with thousands separator
   */
  formatNumber(num) {
    return num.toLocaleString();
  },

  /**
   * Get status badge HTML
   */
  statusBadge(status, text) {
    const classMap = {
      'success': 'badge-success',
      'warning': 'badge-warning',
      'error': 'badge-error',
      'info': 'badge-info'
    };
    return `<span class="badge ${classMap[status] || ''}">${text}</span>`;
  },

  /**
   * Get health indicator HTML
   */
  healthIndicator(status) {
    const icon = `<span class="health-icon ${status}"></span>`;
    const text = status.charAt(0).toUpperCase() + status.slice(1);
    return `<div class="health-status">${icon}<span>${text}</span></div>`;
  },

  /**
   * Get outcome symbol
   */
  outcomeSymbol(outcome) {
    switch (outcome) {
      case 'success': return '<span class="text-success">&#x2713;</span>';
      case 'failed': return '<span class="text-error">&#x2717;</span>';
      default: return '<span class="text-dim">&#xb7;</span>';
    }
  },

  /**
   * Render directional stats (D4/D6/N4/N6/R4/R6)
   */
  renderDirectionalStats(stats) {
    if (!stats) return '<span class="text-dim">-</span>';

    const items = [
      { label: 'D4', value: stats.direct_ipv4 },
      { label: 'D6', value: stats.direct_ipv6 },
      { label: 'N4', value: stats.nat_ipv4 },
      { label: 'N6', value: stats.nat_ipv6 },
      { label: 'R4', value: stats.relay_ipv4 },
      { label: 'R6', value: stats.relay_ipv6 },
    ];

    return `<div class="directional-stats">${
      items.map(item => {
        const className = item.value === 'success' ? 'success' :
                          item.value === 'failed' ? 'failed' : 'unknown';
        return `<span class="directional-stat ${className}">${item.label}</span>`;
      }).join('')
    }</div>`;
  },

  /**
   * Truncate peer ID
   */
  shortPeerId(peerId, length = 8) {
    return peerId ? peerId.substring(0, length) : '';
  },

  /**
   * Update status indicator in header
   */
  updateStatusIndicator(connected) {
    const dot = document.querySelector('.status-dot');
    const text = document.querySelector('.status-text');

    if (dot) {
      dot.classList.toggle('disconnected', !connected);
    }
    if (text) {
      text.textContent = connected ? 'Connected' : 'Disconnected';
    }
  }
};

// Export for use in page scripts
window.WebSocketManager = WebSocketManager;
window.DashboardData = DashboardData;
window.DashboardUtils = DashboardUtils;
