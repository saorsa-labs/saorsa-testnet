#!/bin/bash
# Verify NAT simulation is working correctly
#
# Run this after deploy-nat-type.sh to verify the setup

set -euo pipefail

echo "=== NAT Simulation Verification ==="
echo ""

# Check if config exists
if [[ ! -f /etc/nat-sim/config ]]; then
    echo "ERROR: NAT simulation not configured"
    echo "Run deploy-nat-type.sh first"
    exit 1
fi

source /etc/nat-sim/config

# Get NAT type
NAT_TYPE=$(cat /etc/nat-sim/nat-type 2>/dev/null || echo "unknown")

echo "Configuration:"
echo "  NAT Type: $NAT_TYPE"
echo "  Public IP: $PUBLIC_IP"
echo "  Private IP: $PRIVATE_IP"
echo "  QUIC Port: $QUIC_PORT"
echo "  API Port: $API_PORT"
echo ""

# Check namespace exists
echo "Checking namespace..."
if ip netns list | grep -q $NAMESPACE; then
    echo "  ✓ Namespace '$NAMESPACE' exists"
else
    echo "  ✗ Namespace '$NAMESPACE' not found"
    exit 1
fi

# Check veth interfaces
echo "Checking interfaces..."
if ip link show $VETH_HOST &>/dev/null; then
    echo "  ✓ Host interface '$VETH_HOST' exists"
else
    echo "  ✗ Host interface '$VETH_HOST' not found"
    exit 1
fi

if ip netns exec $NAMESPACE ip link show $VETH_NS &>/dev/null; then
    echo "  ✓ Namespace interface '$VETH_NS' exists"
else
    echo "  ✗ Namespace interface '$VETH_NS' not found"
    exit 1
fi

# Check routing
echo "Checking routing..."
ROUTE=$(ip netns exec $NAMESPACE ip route show default)
if [[ -n "$ROUTE" ]]; then
    echo "  ✓ Default route configured: $ROUTE"
else
    echo "  ✗ No default route in namespace"
    exit 1
fi

# Check service is running
echo "Checking service..."
if systemctl is-active --quiet saorsa-quic-test; then
    echo "  ✓ Service is running"
else
    echo "  ✗ Service is not running"
    journalctl -u saorsa-quic-test --no-pager -n 5
    exit 1
fi

# Check API is responding
echo "Checking API..."
API_RESPONSE=$(curl -s --connect-timeout 5 "http://localhost:$API_PORT/api/probe" 2>/dev/null || echo "")
if [[ -n "$API_RESPONSE" ]]; then
    echo "  ✓ API responding: $API_RESPONSE"
else
    echo "  ✗ API not responding on port $API_PORT"
fi

# Check external connectivity
echo "Checking external connectivity..."
EXTERNAL_TEST=$(ip netns exec $NAMESPACE curl -s --connect-timeout 5 ifconfig.me 2>/dev/null || echo "")
if [[ "$EXTERNAL_TEST" == "$PUBLIC_IP" ]]; then
    echo "  ✓ External IP correct: $EXTERNAL_TEST"
else
    echo "  ⚠ External IP mismatch: got '$EXTERNAL_TEST', expected '$PUBLIC_IP'"
fi

# Show iptables rules
echo ""
echo "=== NAT Rules ==="
echo ""
echo "FORWARD chain:"
iptables -L FORWARD -n -v --line-numbers | head -20
echo ""
echo "NAT PREROUTING:"
iptables -t nat -L PREROUTING -n -v --line-numbers | head -10
echo ""
echo "NAT POSTROUTING:"
iptables -t nat -L POSTROUTING -n -v --line-numbers | head -10

# Show ipsets if any
echo ""
echo "=== Active IPsets ==="
ipset list -t 2>/dev/null || echo "No ipsets configured"

echo ""
echo "=== Verification Complete ==="
echo "NAT Type: $NAT_TYPE"

case "$NAT_TYPE" in
    public)
        echo "Expected behavior: All inbound connections accepted"
        ;;
    full_cone)
        echo "Expected behavior: Inbound accepted after any outbound sent"
        ;;
    addr_restricted)
        echo "Expected behavior: Inbound only from IPs we've contacted"
        ;;
    port_restricted)
        echo "Expected behavior: Inbound only from exact IP:port we've contacted"
        ;;
    symmetric)
        echo "Expected behavior: Random source ports, strict stateful filtering"
        ;;
esac
