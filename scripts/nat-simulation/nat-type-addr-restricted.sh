#!/bin/bash
# NAT Type: ADDRESS RESTRICTED CONE (EIM/ADF)
# Medium restrictiveness
#
# Behavior:
# - Endpoint Independent Mapping: Same external port for all destinations
# - Address Dependent Filtering: Only accept from IPs we've previously sent to
# - Must send to an IP before that IP can send back (any port)
#
# Hole-punch difficulty: MEDIUM
# - Need to send packet to peer's IP first
# - Peer can respond from any port

set -euo pipefail

source /etc/nat-sim/config 2>/dev/null || {
    echo "Error: Run setup-nat-base.sh first"
    exit 1
}

echo "=== Configuring ADDRESS RESTRICTED CONE NAT (EIM/ADF) ==="

# Flush existing forward rules
iptables -F FORWARD

# Allow outbound from namespace
iptables -A FORWARD -i $VETH_HOST -o eth0 -j ACCEPT

# Address Restricted: Only accept from IPs we've sent to
# Linux's default conntrack behavior is Port Restricted (stricter)
# To simulate Address Restricted, we need to use ipset to track destination IPs
# and only accept from those IPs

# Create ipset for tracking destinations we've contacted
ipset destroy nat-allowed-ips 2>/dev/null || true
ipset create nat-allowed-ips hash:ip timeout 300

# Use iptables to add destination IPs to set when we send outbound
iptables -A FORWARD -i $VETH_HOST -o eth0 -p udp -j SET --add-set nat-allowed-ips dst

# Only accept inbound from IPs in our allowed set (address restricted)
iptables -A FORWARD -i eth0 -o $VETH_HOST -p udp --dport $QUIC_PORT \
    -m set --match-set nat-allowed-ips src -j ACCEPT

# Also accept established/related for other traffic
iptables -A FORWARD -i eth0 -o $VETH_HOST -m state --state RELATED,ESTABLISHED -j ACCEPT

# DNAT for the QUIC port
iptables -t nat -A PREROUTING -i eth0 -p udp --dport $QUIC_PORT -j DNAT --to-destination $PRIVATE_IP:$QUIC_PORT

# API port (always accessible for monitoring)
iptables -t nat -A PREROUTING -i eth0 -p tcp --dport $API_PORT -j DNAT --to-destination $PRIVATE_IP:$API_PORT
iptables -A FORWARD -i eth0 -o $VETH_HOST -p tcp --dport $API_PORT -j ACCEPT

# Drop other inbound UDP to QUIC port
iptables -A FORWARD -i eth0 -o $VETH_HOST -p udp --dport $QUIC_PORT -j DROP

# Save NAT type
echo "address_restricted" > /etc/nat-sim/nat-type

echo "=== ADDRESS RESTRICTED configuration complete ==="
echo "NAT Type: Address Restricted Cone (EIM/ADF)"
echo "Mapping: Endpoint Independent (same port for all)"
echo "Filtering: Address Dependent (must send to IP first)"
echo "Hole-punch difficulty: MEDIUM"
echo ""
echo "Active allowed IPs (updated dynamically):"
ipset list nat-allowed-ips 2>/dev/null | head -20
