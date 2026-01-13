#!/bin/bash
# Deploy 25 node processes per VPS (9 VPS × 25 = 225 total nodes)
# Part of the test-debug-fix-deploy cycle

set -eo pipefail

REGISTRY_URL="https://saorsa-1.saorsalabs.com"
NODES_PER_VPS=25
MAX_PEERS=50  # Each node connects to up to 50 peers

# Node definitions: name:ip
VPS_NODES="bootstrap:138.197.29.195
node1:162.243.167.201
node2:159.65.221.230
fullcone:67.205.158.158
restricted:161.35.231.80
portrestricted:178.62.192.11
symmetric:159.65.90.128"

echo "======================================"
echo "Deploying $NODES_PER_VPS nodes per VPS"
echo "Total expected: $(echo "$VPS_NODES" | wc -l | tr -d ' ') VPS × $NODES_PER_VPS = $((7 * NODES_PER_VPS)) nodes"
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

        # Start $NODES_PER_VPS nodes, each on a random port (--bind-port 0)
        echo "Starting $NODES_PER_VPS nodes..."
        for i in \$(seq 1 $NODES_PER_VPS); do
            nohup /usr/local/bin/ant-quic-test \\
                --registry-url $REGISTRY_URL \\
                --max-peers $MAX_PEERS \\
                --bind-port 0 \\
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
