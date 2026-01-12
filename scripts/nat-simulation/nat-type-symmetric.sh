#!/bin/bash
# NAT Type: SYMMETRIC (APDM/APDF)
# Most restrictive NAT type
#
# Behavior:
# - Address+Port Dependent Mapping: DIFFERENT external port for each destination
# - Address+Port Dependent Filtering: Only accept from exact IP:port we sent to
# - Each outbound connection gets a random source port
# - Peer cannot predict our external port
#
# Hole-punch difficulty: VERY HARD
# - External port changes per destination (unpredictable)
# - Often requires relay/TURN server
# - Port prediction techniques rarely work

set -euo pipefail

source /etc/nat-sim/config 2>/dev/null || {
    echo "Error: Run setup-nat-base.sh first"
    exit 1
}

echo "=== Configuring SYMMETRIC NAT (APDM/APDF) ==="

# Flush existing rules
iptables -F FORWARD
iptables -t nat -F POSTROUTING
iptables -t nat -F PREROUTING

# Clean up any ipsets from other NAT types
ipset destroy nat-allowed-ips 2>/dev/null || true
ipset destroy nat-allowed-pairs 2>/dev/null || true

# Create ipset for tracking allowed IP:port pairs
ipset create nat-allowed-pairs hash:ip,port timeout 300

# Allow outbound from namespace
iptables -A FORWARD -i $VETH_HOST -o $EXT_IF -j ACCEPT

# SYMMETRIC MAPPING: Use random source port for each destination
# The --random flag makes MASQUERADE choose a random port for each new connection
iptables -t nat -A POSTROUTING -s $SUBNET -o $EXT_IF -p udp -j MASQUERADE --random

# For non-UDP, use regular SNAT
iptables -t nat -A POSTROUTING -s $SUBNET -o $EXT_IF ! -p udp -j SNAT --to-source $PUBLIC_IP

# Track destination IP:port pairs (even though our source port varies, we track theirs)
iptables -A FORWARD -i $VETH_HOST -o $EXT_IF -p udp -j SET --add-set nat-allowed-pairs dst,dst

# IMPORTANT: Allow ESTABLISHED/RELATED first (so DNS responses etc get through)
iptables -A FORWARD -i $EXT_IF -o $VETH_HOST -m state --state RELATED,ESTABLISHED -j ACCEPT

# Symmetric Filtering: Accept QUIC responses from IP:port pairs we've sent to
iptables -A FORWARD -i $EXT_IF -o $VETH_HOST -p udp --dport $QUIC_PORT \
    -m set --match-set nat-allowed-pairs src,src -j ACCEPT

# DNAT for QUIC port (for responses to our outbound only)
iptables -t nat -A PREROUTING -i $EXT_IF -p udp --dport $QUIC_PORT -j DNAT --to-destination $PRIVATE_IP:$QUIC_PORT

# API port (always accessible for monitoring)
iptables -t nat -A PREROUTING -i $EXT_IF -p tcp --dport $API_PORT -j DNAT --to-destination $PRIVATE_IP:$API_PORT
iptables -A FORWARD -i $EXT_IF -o $VETH_HOST -p tcp --dport $API_PORT -j ACCEPT

# Reject all other NEW inbound UDP (strict symmetric behavior)
# This must come AFTER the ESTABLISHED/RELATED rule
iptables -A FORWARD -i $EXT_IF -o $VETH_HOST -p udp -j DROP

# Save NAT type
echo "symmetric" > /etc/nat-sim/nat-type

echo "=== SYMMETRIC configuration complete ==="
echo "NAT Type: Symmetric (APDM/APDF)"
echo "Mapping: Address+Port Dependent (random port per destination)"
echo "Filtering: Address+Port Dependent (strict stateful)"
echo "Hole-punch difficulty: VERY HARD (often needs relay)"
echo ""
echo "Note: External port will be DIFFERENT for each peer you connect to"
echo "Direct P2P connections between two symmetric NATs usually fail"
