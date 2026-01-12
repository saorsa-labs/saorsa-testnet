#!/bin/bash
# NAT Type: PUBLIC (No NAT)
# This is for relay/coordinator nodes that should have direct public access
#
# Behavior:
# - No NAT translation (traffic goes direct)
# - All inbound allowed
# - No namespace isolation

set -euo pipefail

source /etc/nat-sim/config 2>/dev/null || {
    echo "Error: Run setup-nat-base.sh first"
    exit 1
}

echo "=== Configuring PUBLIC (No NAT) ==="
echo "This node will have direct public IP access"
echo ""

# For public nodes, we actually want to DISABLE the namespace
# and run directly on the host

# Remove namespace restrictions - allow all inbound
iptables -F FORWARD
iptables -A FORWARD -j ACCEPT

# DNAT all traffic to namespace (so test-agent still runs there for consistency)
iptables -t nat -A PREROUTING -i eth0 -p udp --dport $QUIC_PORT -j DNAT --to-destination $PRIVATE_IP:$QUIC_PORT
iptables -t nat -A PREROUTING -i eth0 -p tcp --dport $API_PORT -j DNAT --to-destination $PRIVATE_IP:$API_PORT

# Allow all inbound to forwarded ports
iptables -A FORWARD -i eth0 -o $VETH_HOST -p udp --dport $QUIC_PORT -j ACCEPT
iptables -A FORWARD -i eth0 -o $VETH_HOST -p tcp --dport $API_PORT -j ACCEPT

# Save NAT type
echo "public" > /etc/nat-sim/nat-type

echo "=== PUBLIC configuration complete ==="
echo "NAT Type: PUBLIC (No filtering)"
echo "Hole-punch difficulty: NONE (direct access)"
echo ""
