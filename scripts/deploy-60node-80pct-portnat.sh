#!/bin/bash
# =============================================================================
# Deterministic 60-node testnet with 80% port-restricted NAT
# =============================================================================
#
# This script pins EVERY deployment parameter to reproduce the working
# configuration from 2026-04-07 where uploads succeeded on a 60-node
# testnet with 80% port-restricted NAT.
#
# Context:
#   Chris's AI-driven deployments via /vps-test varied parameters between
#   runs, making it unclear which configuration actually worked. This script
#   removes all ambiguity.
#
# Usage:
#   # Full deploy (build + NAT setup + node launch):
#   ./deploy-60node-80pct-portnat.sh deploy
#
#   # Just rebuild and redeploy binary (NAT already configured):
#   ./deploy-60node-80pct-portnat.sh redeploy
#
#   # Just restart nodes (binary already deployed):
#   ./deploy-60node-80pct-portnat.sh restart
#
#   # Check status:
#   ./deploy-60node-80pct-portnat.sh status
#
#   # Tear down:
#   ./deploy-60node-80pct-portnat.sh stop
#
#   # Run upload test:
#   ./deploy-60node-80pct-portnat.sh upload <file_path>
#
# =============================================================================

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

# =============================================================================
# PINNED CONFIGURATION — DO NOT LET AI CHANGE THESE
# =============================================================================

# Transport + core versions are pinned via Cargo.lock in the testnet repo.
# The testnet repo's Cargo.lock references:
#   saorsa-transport from git branch rc-2026.4.1
#   saorsa-core from git branch rc-2026.4.1
# If you need to update, do it explicitly in Cargo.toml and commit.

# Binary
BINARY_NAME="saorsa-quic-test"
BINARY_PATH="/usr/local/bin/${BINARY_NAME}"

# Registry
REGISTRY_HOST="saorsa-1.saorsalabs.com"
REGISTRY_URL="https://${REGISTRY_HOST}"

# Network parameters — these are FIXED, do not vary
NODES_PER_VPS=7          # 7 nodes x 9 VPSes = 63 nodes (close to 60)
MAX_PEERS=50             # Must match the working run
BIND_PORT=0              # OS assigns random port per node
STARTUP_DELAY_SECS=0.5   # Delay between node starts to avoid port races

# NAT configuration — 80% port-restricted
# 7 VPSes get port_restricted, 1 gets public, 1 gets full_cone
# That's 49/63 = 78% port-restricted (closest to 80% with 9 VPSes)
declare -A NAT_ASSIGNMENTS=(
    ["142.93.199.50"]="port_restricted"    # saorsa-2 (was full_cone)
    ["147.182.234.192"]="port_restricted"  # saorsa-3 (was full_cone)
    ["206.189.7.117"]="port_restricted"    # saorsa-4
    ["144.126.230.161"]="port_restricted"  # saorsa-5
    ["65.21.157.229"]="port_restricted"    # saorsa-6
    ["116.203.101.172"]="port_restricted"  # saorsa-7
    ["149.28.156.231"]="port_restricted"   # saorsa-8 (was symmetric)
    ["45.77.176.184"]="full_cone"          # saorsa-9 (some non-restricted)
    ["77.42.39.239"]="public"             # saorsa-10 (relay/coordinator)
)

# VPS nodes (skip saorsa-1 which is registry)
declare -A VPS_NAMES=(
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

# =============================================================================
# HELPERS
# =============================================================================

log() { echo "[$(date +%H:%M:%S)] $*"; }
err() { echo "[$(date +%H:%M:%S)] ERROR: $*" >&2; }

ssh_node() {
    local ip="$1"
    shift
    ssh -o ConnectTimeout=10 -o StrictHostKeyChecking=no "root@${ip}" "$@"
}

all_ips() {
    echo "${!VPS_NAMES[@]}"
}

print_config() {
    log "============================================"
    log "DEPLOYMENT CONFIGURATION (deterministic)"
    log "============================================"
    log "Nodes per VPS:  ${NODES_PER_VPS}"
    log "Total VPSes:    ${#VPS_NAMES[@]}"
    log "Total nodes:    $((NODES_PER_VPS * ${#VPS_NAMES[@]}))"
    log "Max peers:      ${MAX_PEERS}"
    log "Bind port:      ${BIND_PORT} (dynamic)"
    log "Registry:       ${REGISTRY_URL}"
    log ""
    log "NAT assignments:"
    for ip in $(all_ips); do
        local name="${VPS_NAMES[$ip]}"
        local nat="${NAT_ASSIGNMENTS[$ip]}"
        log "  ${name} (${ip}): ${nat}"
    done

    local port_restricted_count=0
    for ip in $(all_ips); do
        if [[ "${NAT_ASSIGNMENTS[$ip]}" == "port_restricted" ]]; then
            port_restricted_count=$((port_restricted_count + 1))
        fi
    done
    local total_vps=${#VPS_NAMES[@]}
    local pct=$((port_restricted_count * 100 / total_vps))
    log ""
    log "Port-restricted: ${port_restricted_count}/${total_vps} VPSes (${pct}%)"
    log "============================================"
}

# =============================================================================
# BUILD
# =============================================================================

do_build() {
    log "Building ${BINARY_NAME} for x86_64-unknown-linux-gnu..." >&2

    local repo_dir="${SCRIPT_DIR}/.."
    cd "${repo_dir}"

    # Record the exact commit being built
    local commit
    commit=$(git rev-parse HEAD)
    log "Commit: ${commit}" >&2
    log "Branch: $(git branch --show-current)" >&2

    # Check for cargo-zigbuild
    if ! command -v cargo-zigbuild &>/dev/null; then
        err "cargo-zigbuild not found. Install: cargo install cargo-zigbuild && brew install zig"
        exit 1
    fi

    cargo zigbuild --release --target x86_64-unknown-linux-gnu --bin "${BINARY_NAME}" 2>&1 | tail -5 >&2

    local built="target/x86_64-unknown-linux-gnu/release/${BINARY_NAME}"
    if [[ ! -f "${built}" ]]; then
        err "Build failed: ${built} not found"
        exit 1
    fi

    log "Built: ${built} ($(stat -f%z "${built}" 2>/dev/null || stat -c%s "${built}") bytes)" >&2
    echo "${built}"
}

# =============================================================================
# DEPLOY BINARY
# =============================================================================

do_deploy_binary() {
    local binary_path="$1"

    log "Deploying binary to all VPS nodes..."
    for ip in $(all_ips); do
        local name="${VPS_NAMES[$ip]}"
        log "  Uploading to ${name} (${ip})..."
        scp -q "${binary_path}" "root@${ip}:${BINARY_PATH}"
        ssh_node "${ip}" "chmod +x ${BINARY_PATH}"
    done
    log "Binary deployed to all nodes."
}

# =============================================================================
# NAT SETUP
# =============================================================================

do_setup_nat() {
    log "Setting up NAT simulation on all nodes..."

    local nat_scripts="${SCRIPT_DIR}/nat-simulation"
    if [[ ! -d "${nat_scripts}" ]]; then
        err "NAT simulation scripts not found at ${nat_scripts}"
        exit 1
    fi

    for ip in $(all_ips); do
        local name="${VPS_NAMES[$ip]}"
        local nat_type="${NAT_ASSIGNMENTS[$ip]}"
        log "  [${name}] Setting up ${nat_type}..."

        if "${nat_scripts}/deploy-to-node.sh" "${ip}" "${nat_type}"; then
            log "  [${name}] NAT ${nat_type} OK"
        else
            err "  [${name}] NAT setup FAILED -- aborting to preserve determinism"
            exit 1
        fi
    done
    log "NAT simulation configured on all nodes."
}

# =============================================================================
# START NODES
# =============================================================================

do_start_nodes() {
    log "Starting ${NODES_PER_VPS} nodes per VPS..."

    for ip in $(all_ips); do
        local name="${VPS_NAMES[$ip]}"
        local nat_type="${NAT_ASSIGNMENTS[$ip]}"
        log "  [${name}] Starting ${NODES_PER_VPS} nodes (NAT: ${nat_type})..."

        # Determine if nodes run inside NAT namespace
        local run_prefix=""
        if [[ "${nat_type}" != "public" ]]; then
            run_prefix="ip netns exec nat-sim"
        fi

        ssh_node "${ip}" bash -s <<REMOTE_EOF
            set -e

            # Kill existing nodes
            pkill -9 ${BINARY_NAME} 2>/dev/null || true
            sleep 2

            # Clean up old data dirs
            for old_dir in /tmp/saorsa-node-*; do
                [ -d "\$old_dir" ] && rm -rf "\$old_dir"
            done

            # Create log directory
            mkdir -p /var/log/saorsa-nodes

            # Start nodes with unique identities
            for i in \$(seq 1 ${NODES_PER_VPS}); do
                mkdir -p /tmp/saorsa-node-\$i

                nohup ${run_prefix} ${BINARY_PATH} \\
                    --registry-url ${REGISTRY_URL} \\
                    --max-peers ${MAX_PEERS} \\
                    --bind-port ${BIND_PORT} \\
                    --data-dir /tmp/saorsa-node-\$i \\
                    --quiet \\
                    > /var/log/saorsa-nodes/node-\$i.log 2>&1 &

                sleep ${STARTUP_DELAY_SECS}
            done

            sleep 3
            running=\$(pgrep -fc ${BINARY_NAME} 2>/dev/null | tr -d '\n' || echo 0)
            echo "\$running/${NODES_PER_VPS} nodes running on ${name}"
REMOTE_EOF
    done

    log "Waiting 30s for nodes to register..."
    sleep 30
    do_status
}

# =============================================================================
# STOP NODES
# =============================================================================

do_stop_nodes() {
    log "Stopping all nodes..."
    for ip in $(all_ips); do
        log "  Stopping ${VPS_NAMES[$ip]}..."
        ssh_node "${ip}" "pkill -9 ${BINARY_NAME} 2>/dev/null || true" &
    done
    wait
    log "All nodes stopped."
}

# =============================================================================
# STATUS
# =============================================================================

do_status() {
    log "============================================"
    log "NETWORK STATUS"
    log "============================================"

    local total_running=0
    for ip in $(all_ips); do
        local name="${VPS_NAMES[$ip]}"
        local nat="${NAT_ASSIGNMENTS[$ip]}"
        local count
        count=$(ssh_node "${ip}" "pgrep -fc ${BINARY_NAME} 2>/dev/null || echo 0" 2>/dev/null | tr -d '\n')
        log "  ${name} (${nat}): ${count} nodes"
        total_running=$((total_running + count))
    done

    log ""
    log "Total running: ${total_running}"
    log ""

    log "Registry stats:"
    curl -s "${REGISTRY_URL}/api/stats" 2>/dev/null | python3 -c "
import sys, json
try:
    d = json.load(sys.stdin)
    print(f'  Total nodes:   {d.get(\"total_nodes\", 0)}')
    print(f'  Active nodes:  {d.get(\"active_nodes\", 0)}')
    print(f'  Connections:   {d.get(\"total_connections\", 0)}')
except:
    print('  (stats unavailable)')
" 2>/dev/null || log "  (registry unreachable)"
}

# =============================================================================
# UPLOAD TEST
# =============================================================================

do_upload() {
    local file_path="${1:-}"
    if [[ -z "${file_path}" ]]; then
        err "Usage: $0 upload <file_path>"
        exit 1
    fi

    if [[ ! -f "${file_path}" ]]; then
        err "File not found: ${file_path}"
        exit 1
    fi

    local file_size
    file_size=$(stat -f%z "${file_path}" 2>/dev/null || stat -c%s "${file_path}")
    log "Uploading ${file_path} (${file_size} bytes)..."
    log "Using registry: ${REGISTRY_URL}"

    # Upload via the client on saorsa-10 (public node, can reach everything)
    local client_ip="77.42.39.239"
    log "Copying file to saorsa-10..."
    scp -q "${file_path}" "root@${client_ip}:/tmp/upload-test-file"

    log "Running upload..."
    ssh_node "${client_ip}" bash -s <<REMOTE_EOF
        set -e
        cd /tmp
        time ${BINARY_PATH} upload \\
            --registry-url ${REGISTRY_URL} \\
            --file /tmp/upload-test-file \\
            2>&1
REMOTE_EOF
}

# =============================================================================
# COLLECT LOGS
# =============================================================================

do_logs() {
    local dest="${SCRIPT_DIR}/../logs/$(date +%Y%m%d-%H%M%S)"
    mkdir -p "${dest}"
    log "Collecting logs to ${dest}..."

    for ip in $(all_ips); do
        local name="${VPS_NAMES[$ip]}"
        local node_dest="${dest}/${name}"
        mkdir -p "${node_dest}"
        scp -q "root@${ip}:/var/log/saorsa-nodes/*.log" "${node_dest}/" 2>/dev/null || true
        log "  ${name}: collected"
    done
    log "Logs saved to ${dest}"
}

# =============================================================================
# MAIN
# =============================================================================

CMD="${1:-}"
shift || true

case "${CMD}" in
    deploy)
        print_config
        echo ""
        log "This will build, setup NAT, and deploy nodes."
        log "Press Enter to continue or Ctrl+C to abort..."
        read -r
        binary=$(do_build)
        do_deploy_binary "${binary}"
        do_setup_nat
        do_start_nodes
        ;;
    redeploy)
        print_config
        binary=$(do_build)
        do_stop_nodes
        do_deploy_binary "${binary}"
        do_start_nodes
        ;;
    restart)
        do_stop_nodes
        do_start_nodes
        ;;
    stop)
        do_stop_nodes
        ;;
    status)
        do_status
        ;;
    upload)
        do_upload "$@"
        ;;
    logs)
        do_logs
        ;;
    config)
        print_config
        ;;
    *)
        echo "Usage: $0 {deploy|redeploy|restart|stop|status|upload|logs|config}"
        echo ""
        echo "  deploy    - Full deploy: build + NAT setup + start nodes"
        echo "  redeploy  - Rebuild binary, stop, redeploy, restart"
        echo "  restart   - Stop and restart nodes (no rebuild)"
        echo "  stop      - Stop all nodes"
        echo "  status    - Show running nodes and registry stats"
        echo "  upload    - Run upload test: $0 upload <file_path>"
        echo "  logs      - Collect logs from all nodes"
        echo "  config    - Print deployment configuration"
        exit 1
        ;;
esac
