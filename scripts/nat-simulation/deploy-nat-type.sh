#!/bin/bash
# Deploy NAT simulation to a VPS node
#
# Usage: ./deploy-nat-type.sh <nat_type> [public_ip]
#
# NAT types: public, full_cone, addr_restricted, port_restricted, symmetric
#
# This script:
# 1. Sets up the network namespace
# 2. Configures the specified NAT type
# 3. Updates the systemd service to run in namespace
# 4. Restarts the service

set -euo pipefail

NAT_TYPE="${1:-}"
PUBLIC_IP="${2:-$(curl -s4 ifconfig.me)}"
QUIC_PORT=9000
API_PORT=8080

if [[ -z "$NAT_TYPE" ]]; then
    echo "Usage: $0 <nat_type> [public_ip]"
    echo ""
    echo "NAT types:"
    echo "  public         - No NAT (direct public IP)"
    echo "  full_cone      - Full Cone NAT (easy hole-punch)"
    echo "  addr_restricted - Address Restricted NAT (medium)"
    echo "  port_restricted - Port Restricted NAT (hard)"
    echo "  symmetric      - Symmetric NAT (very hard)"
    exit 1
fi

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

echo "======================================"
echo "Deploying NAT simulation: $NAT_TYPE"
echo "Public IP: $PUBLIC_IP"
echo "======================================"
echo ""

# Stop existing service
echo "Stopping existing service..."
systemctl stop saorsa-quic-test 2>/dev/null || true
sleep 2

# Install required packages
echo "Installing required packages..."
apt-get update -qq
apt-get install -y -qq ipset iptables iproute2

# Run base setup
echo "Setting up NAT simulation base..."
bash "$SCRIPT_DIR/setup-nat-base.sh" "$PUBLIC_IP" "$QUIC_PORT" "$API_PORT"

# Run NAT type specific setup
echo "Configuring NAT type: $NAT_TYPE..."
case "$NAT_TYPE" in
    public)
        bash "$SCRIPT_DIR/nat-type-public.sh"
        ;;
    full_cone)
        bash "$SCRIPT_DIR/nat-type-full-cone.sh"
        ;;
    addr_restricted)
        bash "$SCRIPT_DIR/nat-type-addr-restricted.sh"
        ;;
    port_restricted)
        bash "$SCRIPT_DIR/nat-type-port-restricted.sh"
        ;;
    symmetric)
        bash "$SCRIPT_DIR/nat-type-symmetric.sh"
        ;;
    *)
        echo "Unknown NAT type: $NAT_TYPE"
        exit 1
        ;;
esac

# Install the NAT simulation service (for boot persistence)
echo "Installing NAT simulation systemd service..."
cp "$SCRIPT_DIR/nat-simulation.service" /etc/systemd/system/nat-simulation.service
systemctl daemon-reload
systemctl enable nat-simulation.service

# Mark nat-simulation as started (namespace already exists from setup-nat-base.sh)
# This ensures systemd knows the service is "active"
systemctl reset-failed nat-simulation.service 2>/dev/null || true

# Install the NAT-aware test agent service
echo "Installing NAT-aware test agent service..."
cp "$SCRIPT_DIR/saorsa-quic-test-nat.service" /etc/systemd/system/saorsa-quic-test.service
systemctl daemon-reload

# Start service
echo "Starting service in NAT namespace..."
systemctl start saorsa-quic-test
sleep 3

# Verify service is running
if systemctl is-active --quiet saorsa-quic-test; then
    echo ""
    echo "======================================"
    echo "SUCCESS: NAT simulation deployed"
    echo "======================================"
    echo "NAT Type: $NAT_TYPE"
    echo "Public IP: $PUBLIC_IP"
    echo "QUIC Port: $QUIC_PORT"
    echo "API Port: $API_PORT"
    echo ""
    echo "Service status:"
    systemctl status saorsa-quic-test --no-pager | head -10
    echo ""
    echo "To verify NAT rules:"
    echo "  iptables -L FORWARD -n -v"
    echo "  iptables -t nat -L -n -v"
else
    echo "ERROR: Service failed to start"
    journalctl -u saorsa-quic-test --no-pager -n 20
    exit 1
fi
