#!/bin/bash
# Deploy 25 node processes per VPS (10 VPS × 25 = 250 total nodes)
# Part of the test-debug-fix-deploy cycle

set -eo pipefail

REGISTRY_URL="https://saorsa-1.saorsalabs.com"
NODES_PER_VPS=25
MAX_PEERS=50  # Each node connects to up to 50 peers

# VPS nodes from bootstrap_peers.rs (saorsa-1 through saorsa-10)
# Skip saorsa-1 (registry server) - deploy to nodes 2-10
VPS_NODES="saorsa-2:142.93.199.50
saorsa-3:147.182.234.192
saorsa-4:206.189.7.117
saorsa-5:144.126.230.161
saorsa-6:65.21.157.229
saorsa-7:116.203.101.172
saorsa-8:149.28.156.231
saorsa-9:45.77.176.184
saorsa-10:77.42.39.239"

echo "======================================"
echo "Deploying $NODES_PER_VPS nodes per VPS"
echo "Total expected: $(echo "$VPS_NODES" | wc -l | tr -d ' ') VPS × $NODES_PER_VPS = $((9 * NODES_PER_VPS)) nodes"
echo "======================================"
echo ""

for vps in $VPS_NODES; do
    name="${vps%%:*}"
    ip="${vps##*:}"
    echo ">>> Setting up $name ($ip) with $NODES_PER_VPS nodes..."

    # Deploy and start nodes
    ssh -o ConnectTimeout=10 -o StrictHostKeyChecking=no "root@$ip" bash -s <<EOF
        set -e

        # Kill all existing ant-quic processes
        pkill -9 ant-quic 2>/dev/null || true
        sleep 2

        # Create log directory
        mkdir -p /var/log/saorsa-nodes

        # Clean up old identity files (all nodes had same identity - the bug we're fixing)
        rm -rf /tmp/saorsa-node-*

        # Start $NODES_PER_VPS nodes, each with a UNIQUE data directory for identity
        echo "Starting $NODES_PER_VPS nodes with unique identities..."
        for i in \$(seq 1 $NODES_PER_VPS); do
            # Create unique data directory for this node's identity keypair
            mkdir -p /tmp/saorsa-node-\$i

            nohup /usr/local/bin/ant-quic-test \\
                --registry-url $REGISTRY_URL \\
                --max-peers $MAX_PEERS \\
                --bind-port 0 \\
                --data-dir /tmp/saorsa-node-\$i \\
                --quiet \\
                > /var/log/saorsa-nodes/node-\$i.log 2>&1 &

            # Small delay to avoid port conflicts during startup
            sleep 0.2
        done

        # Wait for processes to stabilize
        sleep 3

        # Count running processes
        running=\$(pgrep -c ant-quic 2>/dev/null || echo 0)
        echo "Started \$running/$NODES_PER_VPS nodes on $name"

        if [ "\$running" -lt "$NODES_PER_VPS" ]; then
            echo "WARNING: Only \$running nodes started, expected $NODES_PER_VPS"
            ps aux | grep ant-quic | grep -v grep | head -5
        fi
EOF

    if [ $? -eq 0 ]; then
        echo "    [OK] $name configured"
    else
        echo "    [FAIL] $name failed"
    fi
    echo ""
done

echo "======================================"
echo "Deployment complete!"
echo "======================================"
echo ""
echo "Waiting 30s for nodes to register with registry..."
sleep 30

echo ""
echo "Checking network stats..."
curl -s https://saorsa-1.saorsalabs.com/api/stats | python3 -c "
import sys, json
try:
    d = json.load(sys.stdin)
    print(f\"Total nodes: {d.get('total_nodes', 0)}\")
    print(f\"Active nodes: {d.get('active_nodes', 0)}\")
    print(f\"Connections: {d.get('total_connections', 0)}\")
    print(f\"Uptime: {d.get('uptime_secs', 0)}s\")
except:
    print('Could not parse stats')
" 2>/dev/null || echo "Stats unavailable"
