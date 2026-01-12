#!/bin/bash
# Clean up NAT simulation
# Removes namespace, iptables rules, and ipsets

set -euo pipefail

echo "=== Cleaning up NAT simulation ==="

# Stop the service first
systemctl stop saorsa-quic-test 2>/dev/null || true

# Remove namespace
echo "Removing network namespace..."
ip netns del nat-sim 2>/dev/null || true

# Remove veth interface (should be auto-removed with namespace, but be sure)
ip link del veth-host 2>/dev/null || true

# Flush iptables rules
echo "Flushing iptables rules..."
iptables -F FORWARD 2>/dev/null || true
iptables -t nat -F PREROUTING 2>/dev/null || true
iptables -t nat -F POSTROUTING 2>/dev/null || true

# Remove ipsets
echo "Removing ipsets..."
ipset destroy nat-allowed-ips 2>/dev/null || true
ipset destroy nat-allowed-pairs 2>/dev/null || true

# Remove config
rm -rf /etc/nat-sim

# Restore default forwarding rules
echo "Restoring default rules..."
iptables -P FORWARD ACCEPT

echo "=== Cleanup complete ==="
echo "You can now restart the service without NAT simulation:"
echo "  systemctl start saorsa-quic-test"
