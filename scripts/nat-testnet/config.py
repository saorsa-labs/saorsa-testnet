"""
Centralized configuration for NAT testnet provisioning.

All tunables live here. Cloud API tokens are loaded from environment variables:
  - DIGITALOCEAN_TOKEN
  - HCLOUD_TOKEN
"""

import os
import sys

# ---------------------------------------------------------------------------
# Cloud provider SSH key IDs (already registered)
# ---------------------------------------------------------------------------
DO_KEY_ID = 55462233
HC_KEY_ID = 110444940

SSH_KEY_PATH = os.path.expanduser("~/.ssh/testnet_ed25519")

# ---------------------------------------------------------------------------
# VM sizes
# ---------------------------------------------------------------------------
DO_SIZE = "s-2vcpu-2gb"
HC_TYPE = "cpx11"

# ---------------------------------------------------------------------------
# Regions
# ---------------------------------------------------------------------------
DO_REGIONS = ["lon1", "ams3", "nyc1", "sfo3"]
HC_LOCATIONS = ["ash", "hil"]  # Only these support cpx11

# Hetzner account limit
HC_MAX_SERVERS = 5

# ---------------------------------------------------------------------------
# Binary URLs (parameterized by version)
# ---------------------------------------------------------------------------
NODE_URL_TEMPLATE = (
    "https://github.com/WithAutonomi/ant-node/releases/download/"
    "{version}/ant-node-cli-linux-x64.tar.gz"
)
CLIENT_URL_TEMPLATE = (
    "https://github.com/WithAutonomi/ant-client/releases/download/"
    "{version}/ant-{client_version}-x86_64-unknown-linux-musl.tar.gz"
)

# ---------------------------------------------------------------------------
# Node defaults
# ---------------------------------------------------------------------------
REWARDS_ADDRESS = "0x0000000000000000000000000000000000000001"
SECRET_KEY = "77579dc8a351606a8c27b9b37afd2527c66771b4dec7a69d3964e22cf74d17f2"

# Base ports for nodes (each VM can run multiple nodes on consecutive ports)
NODE_BASE_PORT = 10000

# ---------------------------------------------------------------------------
# Timeouts (seconds)
# ---------------------------------------------------------------------------
VM_CREATION_WAIT = 90       # Wait for DO droplets to get IPs
NODE_STARTUP_WAIT = 30      # Wait after starting bootstrap nodes
NETWORK_FORMATION_WAIT = 90 # Wait for mesh to form after all nodes start

# ---------------------------------------------------------------------------
# Paths
# ---------------------------------------------------------------------------
SCRIPT_DIR = os.path.dirname(os.path.abspath(__file__))
STATE_DIR = os.path.join(SCRIPT_DIR, "state")

# ---------------------------------------------------------------------------
# API token helpers
# ---------------------------------------------------------------------------

def get_do_token():
    """Return DigitalOcean API token from environment."""
    token = os.environ.get("DIGITALOCEAN_TOKEN", "")
    if not token:
        print("ERROR: DIGITALOCEAN_TOKEN environment variable not set.", file=sys.stderr)
        sys.exit(1)
    return token


def get_hc_token():
    """Return Hetzner Cloud API token from environment."""
    token = os.environ.get("HCLOUD_TOKEN", "")
    if not token:
        print("ERROR: HCLOUD_TOKEN environment variable not set.", file=sys.stderr)
        sys.exit(1)
    return token
