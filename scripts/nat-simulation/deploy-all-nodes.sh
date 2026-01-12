#!/bin/bash
# Deploy NAT simulation to ALL VPS nodes
#
# NAT Type Assignments:
# - saorsa-1  (77.42.75.115)    : public          (registry - no simulation)
# - saorsa-2  (142.93.199.50)   : full_cone
# - saorsa-3  (147.182.234.192) : full_cone
# - saorsa-4  (206.189.7.117)   : addr_restricted
# - saorsa-5  (144.126.230.161) : addr_restricted
# - saorsa-6  (65.21.157.229)   : port_restricted
# - saorsa-7  (116.203.101.172) : port_restricted
# - saorsa-8  (149.28.156.231)  : symmetric
# - saorsa-9  (45.77.176.184)   : symmetric
# - saorsa-10 (77.42.39.239)    : public

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

# Define node assignments
declare -A NODES=(
    # Skip saorsa-1 - it's the registry and should stay public without simulation
    ["142.93.199.50"]="full_cone"
    ["147.182.234.192"]="full_cone"
    ["206.189.7.117"]="addr_restricted"
    ["144.126.230.161"]="addr_restricted"
    ["65.21.157.229"]="port_restricted"
    ["116.203.101.172"]="port_restricted"
    ["149.28.156.231"]="symmetric"
    ["45.77.176.184"]="symmetric"
    ["77.42.39.239"]="public"
)

# Node names for display
declare -A NODE_NAMES=(
    ["142.93.199.50"]="saorsa-2"
    ["147.182.234.192"]="saorsa-3"
    ["206.189.7.117"]="saorsa-4"
    ["144.126.230.161"]="saorsa-5"
    ["65.21.157.229"]="saorsa-6"
    ["116.203.101.172"]="saorsa-7"
    ["149.28.156.231"]="saorsa-8"
    ["45.77.176.184"]="saorsa-9"
    ["77.42.39.239"]="saorsa-10"
)

echo "========================================"
echo "Deploying NAT simulation to ALL nodes"
echo "========================================"
echo ""
echo "Node assignments:"
for IP in "${!NODES[@]}"; do
    echo "  ${NODE_NAMES[$IP]} ($IP): ${NODES[$IP]}"
done
echo ""
echo "Press Enter to continue or Ctrl+C to abort..."
read

FAILED=()
SUCCEEDED=()

for IP in "${!NODES[@]}"; do
    NAT_TYPE="${NODES[$IP]}"
    NODE_NAME="${NODE_NAMES[$IP]}"

    echo ""
    echo "========================================"
    echo "[$NODE_NAME] Deploying $NAT_TYPE to $IP"
    echo "========================================"

    if "$SCRIPT_DIR/deploy-to-node.sh" "$IP" "$NAT_TYPE"; then
        SUCCEEDED+=("$NODE_NAME ($NAT_TYPE)")
        echo "[$NODE_NAME] SUCCESS"
    else
        FAILED+=("$NODE_NAME ($NAT_TYPE)")
        echo "[$NODE_NAME] FAILED"
    fi
done

echo ""
echo "========================================"
echo "DEPLOYMENT SUMMARY"
echo "========================================"
echo ""
echo "Succeeded (${#SUCCEEDED[@]}):"
for node in "${SUCCEEDED[@]}"; do
    echo "  - $node"
done
echo ""
if [[ ${#FAILED[@]} -gt 0 ]]; then
    echo "Failed (${#FAILED[@]}):"
    for node in "${FAILED[@]}"; do
        echo "  - $node"
    done
    echo ""
    echo "Please investigate failed deployments manually"
    exit 1
else
    echo "All deployments successful!"
fi
