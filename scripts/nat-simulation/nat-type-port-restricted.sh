#!/bin/bash
# NAT Type: PORT RESTRICTED CONE (EIM/APDF)
# Strict filtering, but consistent mapping
#
# Behavior:
# - Endpoint Independent Mapping: Same external port for all destinations
# - Address+Port Dependent Filtering: Only accept from exact IP:port we sent to
# - This is Linux's default conntrack behavior
#
# Hole-punch difficulty: HARD
# - Need to send packet to peer's exact IP:port
# - Peer must respond from same IP:port
# - Requires coordination for simultaneous open

set -euo pipefail

source /etc/nat-sim/config 2>/dev/null || {
    echo "Error: Run setup-nat-base.sh first"
    exit 1
}

echo "=== Configuring PORT RESTRICTED CONE NAT (EIM/APDF) ==="

# Flush existing forward rules
iptables -F FORWARD

# Clean up any ipsets from other NAT types
ipset destroy nat-allowed-ips 2>/dev/null || true
ipset destroy nat-allowed-pairs 2>/dev/null || true

# Create ipset for tracking destination IP:port pairs
ipset create nat-allowed-pairs hash:ip,port timeout 300

# Allow outbound from namespace
iptables -A FORWARD -i $VETH_HOST -o eth0 -j ACCEPT

# Track destination IP:port pairs when sending outbound
iptables -A FORWARD -i $VETH_HOST -o eth0 -p udp -j SET --add-set nat-allowed-pairs dst,dst

# Port Restricted: Only accept from IP:port pairs we've sent to
iptables -A FORWARD -i eth0 -o $VETH_HOST -p udp --dport $QUIC_PORT \
    -m set --match-set nat-allowed-pairs src,src -j ACCEPT

# Also accept established/related (backup for TCP etc)
iptables -A FORWARD -i eth0 -o $VETH_HOST -m state --state RELATED,ESTABLISHED -j ACCEPT

# DNAT for the QUIC port
iptables -t nat -A PREROUTING -i eth0 -p udp --dport $QUIC_PORT -j DNAT --to-destination $PRIVATE_IP:$QUIC_PORT

# API port (always accessible for monitoring)
iptables -t nat -A PREROUTING -i eth0 -p tcp --dport $API_PORT -j DNAT --to-destination $PRIVATE_IP:$API_PORT
iptables -A FORWARD -i eth0 -o $VETH_HOST -p tcp --dport $API_PORT -j ACCEPT

# Drop other inbound UDP to QUIC port
iptables -A FORWARD -i eth0 -o $VETH_HOST -p udp --dport $QUIC_PORT -j DROP

# Save NAT type
echo "port_restricted" > /etc/nat-sim/nat-type

echo "=== PORT RESTRICTED configuration complete ==="
echo "NAT Type: Port Restricted Cone (EIM/APDF)"
echo "Mapping: Endpoint Independent (same port for all)"
echo "Filtering: Address+Port Dependent (must send to exact IP:port)"
echo "Hole-punch difficulty: HARD"
echo ""
echo "Active allowed IP:port pairs (updated dynamically):"
ipset list nat-allowed-pairs 2>/dev/null | head -20
