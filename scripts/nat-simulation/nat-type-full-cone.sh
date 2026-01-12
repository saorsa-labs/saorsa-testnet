#!/bin/bash
# NAT Type: FULL CONE (EIM/EIF)
# Most permissive NAT type
#
# Behavior:
# - Endpoint Independent Mapping: Same external port for all destinations
# - Endpoint Independent Filtering: Once mapping exists, accept from ANY source
# - After internal host sends one packet out, anyone can send packets in
#
# Hole-punch difficulty: EASY
# - Just need to send one outbound packet to create mapping
# - Then any peer can send inbound

set -euo pipefail

source /etc/nat-sim/config 2>/dev/null || {
    echo "Error: Run setup-nat-base.sh first"
    exit 1
}

echo "=== Configuring FULL CONE NAT (EIM/EIF) ==="

# Flush existing forward rules
iptables -F FORWARD

# Allow outbound from namespace
iptables -A FORWARD -i $VETH_HOST -o eth0 -j ACCEPT

# For Full Cone: once we've sent ANY packet out, accept from ANY source
# We use conntrack with --ctstate to track this
# The key difference from other NAT types: we accept NEW connections on the QUIC port
# once any outbound traffic has been seen

# Accept established/related (standard)
iptables -A FORWARD -i eth0 -o $VETH_HOST -m state --state RELATED,ESTABLISHED -j ACCEPT

# Full Cone specific: Accept ANY inbound UDP to QUIC port after first outbound
# We implement this by port forwarding - once namespace sends anything out,
# the port is "open" and we accept from anyone
iptables -t nat -A PREROUTING -i eth0 -p udp --dport $QUIC_PORT -j DNAT --to-destination $PRIVATE_IP:$QUIC_PORT
iptables -A FORWARD -i eth0 -o $VETH_HOST -p udp --dport $QUIC_PORT -j ACCEPT

# Also forward API port
iptables -t nat -A PREROUTING -i eth0 -p tcp --dport $API_PORT -j DNAT --to-destination $PRIVATE_IP:$API_PORT
iptables -A FORWARD -i eth0 -o $VETH_HOST -p tcp --dport $API_PORT -j ACCEPT

# Save NAT type
echo "full_cone" > /etc/nat-sim/nat-type

echo "=== FULL CONE configuration complete ==="
echo "NAT Type: Full Cone (EIM/EIF)"
echo "Mapping: Endpoint Independent (same port for all)"
echo "Filtering: Endpoint Independent (accept from any)"
echo "Hole-punch difficulty: EASY"
echo ""
