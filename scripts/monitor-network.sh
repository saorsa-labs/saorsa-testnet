#!/bin/bash
# Continuous network monitoring script
# Checks all VPS nodes and registry stats at specified intervals

set -eo pipefail

LOG_DIR="/tmp/claude/saorsa-monitor"
mkdir -p "$LOG_DIR"

LOG_FILE="$LOG_DIR/monitor.log"
ALERT_FILE="$LOG_DIR/alerts.log"

# VPS nodes
VPS_NODES="bootstrap:138.197.29.195
node1:162.243.167.201
node2:159.65.221.230
fullcone:67.205.158.158
restricted:161.35.231.80
portrestricted:178.62.192.11
symmetric:159.65.90.128"

EXPECTED_NODES_PER_VPS=25
REGISTRY_URL="https://saorsa-1.saorsalabs.com"

log() {
    echo "[$(date '+%Y-%m-%d %H:%M:%S')] $1" | tee -a "$LOG_FILE"
}

alert() {
    echo "[$(date '+%Y-%m-%d %H:%M:%S')] ALERT: $1" | tee -a "$ALERT_FILE" "$LOG_FILE"
}

check_vps_nodes() {
    local failed_vps=""
    local total_running=0

    for vps in $VPS_NODES; do
        name="${vps%%:*}"
        ip="${vps##*:}"

        running=$(ssh -o ConnectTimeout=5 -o BatchMode=yes "root@$ip" "pgrep -c ant-quic 2>/dev/null || echo 0" 2>/dev/null || echo "UNREACHABLE")

        if [ "$running" = "UNREACHABLE" ]; then
            alert "$name ($ip) is UNREACHABLE"
            failed_vps="$failed_vps $name"
        elif [ "$running" -lt "$EXPECTED_NODES_PER_VPS" ]; then
            alert "$name has only $running/$EXPECTED_NODES_PER_VPS nodes running"
            failed_vps="$failed_vps $name"
        else
            log "$name: $running nodes OK"
            total_running=$((total_running + running))
        fi
    done

    log "Total nodes running: $total_running"
    echo "$failed_vps"
}

check_registry() {
    local stats
    stats=$(curl -s --connect-timeout 10 "$REGISTRY_URL/api/stats" 2>/dev/null)

    if [ -z "$stats" ]; then
        alert "Registry API not responding"
        return 1
    fi

    # Parse stats
    local active_nodes=$(echo "$stats" | python3 -c "import sys,json; print(json.load(sys.stdin).get('active_nodes', 0))" 2>/dev/null || echo 0)
    local total_connections=$(echo "$stats" | python3 -c "import sys,json; print(json.load(sys.stdin).get('total_connections', 0))" 2>/dev/null || echo 0)
    local direct=$(echo "$stats" | python3 -c "import sys,json; print(json.load(sys.stdin).get('connection_breakdown',{}).get('direct', 0))" 2>/dev/null || echo 0)
    local holepunched=$(echo "$stats" | python3 -c "import sys,json; print(json.load(sys.stdin).get('connection_breakdown',{}).get('hole_punched', 0))" 2>/dev/null || echo 0)
    local relayed=$(echo "$stats" | python3 -c "import sys,json; print(json.load(sys.stdin).get('connection_breakdown',{}).get('relayed', 0))" 2>/dev/null || echo 0)
    local uptime=$(echo "$stats" | python3 -c "import sys,json; print(json.load(sys.stdin).get('uptime_secs', 0))" 2>/dev/null || echo 0)

    log "Registry: active=$active_nodes conns=$total_connections direct=$direct punch=$holepunched relay=$relayed uptime=${uptime}s"

    # Check for concerning patterns
    if [ "$total_connections" -eq 0 ]; then
        alert "No connections reported by registry"
        return 1
    fi

    echo "$uptime"
}

gather_logs_for_vps() {
    local name="$1"
    local ip="$2"
    local timestamp=$(date '+%Y%m%d_%H%M%S')
    local log_dir="$LOG_DIR/incident_${timestamp}_${name}"

    mkdir -p "$log_dir"

    log "Gathering logs from $name..."

    # Get process list
    ssh -o ConnectTimeout=10 "root@$ip" "ps aux | grep ant-quic" > "$log_dir/processes.txt" 2>&1 || true

    # Get recent logs from all nodes
    ssh -o ConnectTimeout=10 "root@$ip" "tail -50 /var/log/saorsa-nodes/node-*.log 2>/dev/null" > "$log_dir/node_logs.txt" 2>&1 || true

    # Get system stats
    ssh -o ConnectTimeout=10 "root@$ip" "free -h; df -h; uptime" > "$log_dir/system_stats.txt" 2>&1 || true

    log "Logs saved to $log_dir"
    echo "$log_dir"
}

run_check() {
    local check_num="$1"
    local elapsed_mins="$2"

    log "=========================================="
    log "CHECK #$check_num (${elapsed_mins}m elapsed)"
    log "=========================================="

    # Check VPS nodes
    failed_vps=$(check_vps_nodes)

    # Check registry
    uptime=$(check_registry)

    # If any VPS failed, gather logs
    if [ -n "$failed_vps" ]; then
        log "ISSUES DETECTED on:$failed_vps"
        for vps in $VPS_NODES; do
            name="${vps%%:*}"
            ip="${vps##*:}"
            if echo "$failed_vps" | grep -q "$name"; then
                gather_logs_for_vps "$name" "$ip"
            fi
        done
        return 1
    fi

    log "All checks PASSED"
    return 0
}

# Main monitoring loop
log "=========================================="
log "MONITORING STARTED"
log "Expected: 7 VPS × $EXPECTED_NODES_PER_VPS = $((7 * EXPECTED_NODES_PER_VPS)) nodes"
log "Checkpoints: 10m, 20m, 40m, 80m, 240m (4h)"
log "=========================================="

START_TIME=$(date +%s)
CHECK_NUM=0

# Checkpoint intervals in minutes
CHECKPOINTS="10 20 40 80 240"

for checkpoint in $CHECKPOINTS; do
    # Calculate sleep time until this checkpoint
    now=$(date +%s)
    elapsed_secs=$((now - START_TIME))
    target_secs=$((checkpoint * 60))
    sleep_secs=$((target_secs - elapsed_secs))

    if [ $sleep_secs -gt 0 ]; then
        log "Sleeping ${sleep_secs}s until ${checkpoint}m checkpoint..."
        sleep $sleep_secs
    fi

    CHECK_NUM=$((CHECK_NUM + 1))

    if ! run_check "$CHECK_NUM" "$checkpoint"; then
        alert "CHECKPOINT ${checkpoint}m FAILED - Issues detected"
        log "Exiting for investigation. Logs in $LOG_DIR"
        exit 1
    fi

    log "CHECKPOINT ${checkpoint}m PASSED ✓"
done

log "=========================================="
log "ALL CHECKPOINTS PASSED! 4-hour test complete."
log "=========================================="
