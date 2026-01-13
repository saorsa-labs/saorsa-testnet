#!/bin/bash
# Restart all VPS nodes with --max-peers 25
# Part of the test-debug-fix-deploy cycle

set -eo pipefail

REGISTRY_URL="https://saorsa-1.saorsalabs.com"
MAX_PEERS=25

# Node definitions: name:ip
NODES="bootstrap:138.197.29.195
node1:162.243.167.201
node2:159.65.221.230
fullcone:67.205.158.158
restricted:161.35.231.80
portrestricted:178.62.192.11
symmetric:159.65.90.128"

echo "======================================"
echo "Restarting all nodes with --max-peers $MAX_PEERS"
echo "======================================"
echo ""

for node in $NODES; do
    name="${node%%:*}"
    ip="${node##*:}"
    echo ">>> Restarting $name ($ip) with --max-peers $MAX_PEERS..."

    # Kill existing process and restart with new config
    ssh -o ConnectTimeout=10 -o StrictHostKeyChecking=no "root@$ip" bash -s <<EOF
        # Kill existing ant-quic process
        pkill -9 ant-quic 2>/dev/null || true
        sleep 1

        # Start with new config (nohup to keep running after SSH disconnects)
        nohup /usr/local/bin/ant-quic-test --registry-url $REGISTRY_URL --max-peers $MAX_PEERS --quiet > /var/log/ant-quic-test.log 2>&1 &

        sleep 2

        # Verify it started
        if pgrep ant-quic > /dev/null; then
            echo "OK: ant-quic-test started with --max-peers $MAX_PEERS"
            ps aux | grep ant-quic | grep -v grep | head -1
        else
            echo "ERROR: Failed to start ant-quic-test"
            exit 1
        fi
EOF

    if [ $? -eq 0 ]; then
        echo "    [OK] $name restarted successfully"
    else
        echo "    [FAIL] $name failed to restart"
    fi
    echo ""
done

echo "======================================"
echo "All nodes restarted with --max-peers $MAX_PEERS"
echo "======================================"
echo ""
echo "Verify with: curl -s https://saorsa-1.saorsalabs.com/api/stats"
