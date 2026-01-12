#!/bin/bash
# Deploy NAT simulation to a remote VPS node
#
# Usage: ./deploy-to-node.sh <ip_address> <nat_type>
#
# Example: ./deploy-to-node.sh 142.93.199.50 full_cone

set -euo pipefail

IP="${1:-}"
NAT_TYPE="${2:-}"

if [[ -z "$IP" ]] || [[ -z "$NAT_TYPE" ]]; then
    echo "Usage: $0 <ip_address> <nat_type>"
    echo ""
    echo "NAT types: public, full_cone, addr_restricted, port_restricted, symmetric"
    exit 1
fi

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REMOTE_DIR="/root/nat-simulation"

echo "========================================"
echo "Deploying NAT simulation to $IP"
echo "NAT Type: $NAT_TYPE"
echo "========================================"
echo ""

# Copy scripts to remote
echo "Copying scripts to $IP..."
ssh root@$IP "mkdir -p $REMOTE_DIR"
scp -q "$SCRIPT_DIR"/*.sh root@$IP:$REMOTE_DIR/
scp -q "$SCRIPT_DIR"/*.service root@$IP:$REMOTE_DIR/

# Make scripts executable
ssh root@$IP "chmod +x $REMOTE_DIR/*.sh"

# Run deployment
echo "Running deployment script..."
ssh root@$IP "cd $REMOTE_DIR && ./deploy-nat-type.sh $NAT_TYPE"

echo ""
echo "========================================"
echo "Deployment complete for $IP"
echo "========================================"
echo ""

# Verify
echo "Running verification..."
ssh root@$IP "cd $REMOTE_DIR && ./verify-nat-type.sh"
