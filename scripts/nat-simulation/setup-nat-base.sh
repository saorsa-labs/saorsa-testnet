#!/bin/bash
# NAT Simulation Base Setup
# Creates network namespace infrastructure for NAT simulation
#
# Usage: ./setup-nat-base.sh [public_ip] [quic_port]
#
# This creates:
# - Network namespace 'nat-sim'
# - veth pair connecting host to namespace
# - Base forwarding rules
# - The test-agent runs inside namespace with private IP

set -euo pipefail

PUBLIC_IP="${1:-$(curl -s4 ifconfig.me)}"
QUIC_PORT="${2:-9000}"
API_PORT="${3:-8080}"

NAMESPACE="nat-sim"
VETH_HOST="veth-host"
VETH_NS="veth-ns"
PRIVATE_IP="10.200.0.2"
HOST_IP="10.200.0.1"
SUBNET="10.200.0.0/24"

# Auto-detect external interface (eth0 or enp1s0 etc)
EXT_IF=$(ip route get 8.8.8.8 | head -1 | awk '{print $5}')

echo "=== NAT Simulation Base Setup ==="
echo "Public IP: $PUBLIC_IP"
echo "External Interface: $EXT_IF"
echo "QUIC Port: $QUIC_PORT"
echo "API Port: $API_PORT"
echo ""

# Clean up any existing setup
echo "Cleaning up existing setup..."
ip netns del $NAMESPACE 2>/dev/null || true
ip link del $VETH_HOST 2>/dev/null || true

# Create network namespace
echo "Creating network namespace '$NAMESPACE'..."
ip netns add $NAMESPACE

# Copy DNS configuration to namespace
# Note: On systems using systemd-resolved, /etc/resolv.conf points to 127.0.0.53
# which won't work inside a network namespace. Use public DNS instead.
echo "Setting up DNS in namespace..."
mkdir -p /etc/netns/$NAMESPACE
if grep -q "127.0.0.53" /etc/resolv.conf 2>/dev/null; then
    echo "Using public DNS (systemd-resolved detected)..."
    cat > /etc/netns/$NAMESPACE/resolv.conf <<DNSEOF
nameserver 8.8.8.8
nameserver 1.1.1.1
nameserver 8.8.4.4
DNSEOF
else
    cp /etc/resolv.conf /etc/netns/$NAMESPACE/resolv.conf
fi

# Create veth pair
echo "Creating veth pair..."
ip link add $VETH_HOST type veth peer name $VETH_NS

# Move one end to namespace
ip link set $VETH_NS netns $NAMESPACE

# Configure host end
echo "Configuring host interface..."
ip addr add $HOST_IP/24 dev $VETH_HOST
ip link set $VETH_HOST up

# Configure namespace end
echo "Configuring namespace interface..."
ip netns exec $NAMESPACE ip addr add $PRIVATE_IP/24 dev $VETH_NS
ip netns exec $NAMESPACE ip link set $VETH_NS up
ip netns exec $NAMESPACE ip link set lo up

# Set default route in namespace (via host)
ip netns exec $NAMESPACE ip route add default via $HOST_IP

# Enable IP forwarding
echo "Enabling IP forwarding..."
echo 1 > /proc/sys/net/ipv4/ip_forward

# Flush existing NAT rules for our setup
echo "Flushing existing NAT rules..."
iptables -t nat -F POSTROUTING 2>/dev/null || true
iptables -t nat -F PREROUTING 2>/dev/null || true
iptables -F FORWARD 2>/dev/null || true

# Basic forwarding rules (will be modified by NAT type script)
echo "Setting up basic forwarding..."
iptables -A FORWARD -i $VETH_HOST -o $EXT_IF -j ACCEPT
iptables -A FORWARD -i eth0 -o $VETH_HOST -m state --state RELATED,ESTABLISHED -j ACCEPT

# Basic SNAT for outbound traffic
iptables -t nat -A POSTROUTING -s $SUBNET -o $EXT_IF -j SNAT --to-source $PUBLIC_IP

# Save config for NAT type scripts
mkdir -p /etc/nat-sim
cat > /etc/nat-sim/config <<EOF
PUBLIC_IP=$PUBLIC_IP
QUIC_PORT=$QUIC_PORT
API_PORT=$API_PORT
NAMESPACE=$NAMESPACE
VETH_HOST=$VETH_HOST
VETH_NS=$VETH_NS
PRIVATE_IP=$PRIVATE_IP
HOST_IP=$HOST_IP
SUBNET=$SUBNET
EXT_IF=$EXT_IF
EOF

echo ""
echo "=== Base setup complete ==="
echo "Namespace '$NAMESPACE' created with private IP $PRIVATE_IP"
echo "Now run one of the NAT type scripts to configure filtering behavior"
echo ""
echo "To run a command in the namespace:"
echo "  ip netns exec $NAMESPACE <command>"
echo ""
