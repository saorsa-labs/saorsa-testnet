#!/bin/bash
# Local Testnet Management Script
#
# Runs a comprehensive 20-node local testnet with NAT simulation for testing
# ant-quic P2P connectivity, data transfer, and detecting mutex/deadlock issues.
#
# Usage:
#   ./scripts/local-testnet.sh build      # Build the test binary
#   ./scripts/local-testnet.sh start      # Start the testnet
#   ./scripts/local-testnet.sh stop       # Stop the testnet
#   ./scripts/local-testnet.sh restart    # Restart the testnet
#   ./scripts/local-testnet.sh status     # Show node status
#   ./scripts/local-testnet.sh logs       # View all logs
#   ./scripts/local-testnet.sh logs <node> # View specific node logs
#   ./scripts/local-testnet.sh monitor    # Live monitoring dashboard
#   ./scripts/local-testnet.sh clean      # Remove all containers and volumes
#   ./scripts/local-testnet.sh test       # Run extended test (default 1 hour)
#   ./scripts/local-testnet.sh test <duration> # Run test for specified duration

set -euo pipefail

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
CYAN='\033[0;36m'
MAGENTA='\033[0;35m'
NC='\033[0m' # No Color

# Script directory
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(dirname "$SCRIPT_DIR")"
DOCKER_DIR="$PROJECT_ROOT/docker/local-testnet"
BIN_DIR="$DOCKER_DIR/bin"

# Configuration
# Use simple compose file on macOS (no privileged networking)
# Use full compose file on Linux (with NAT simulation)
if [[ "$(uname)" == "Darwin" ]]; then
    COMPOSE_FILE="$DOCKER_DIR/docker-compose-simple.yml"
else
    COMPOSE_FILE="$DOCKER_DIR/docker-compose.yml"
fi
DEFAULT_TEST_DURATION="3600"  # 1 hour in seconds

log_info() { echo -e "${BLUE}[INFO]${NC} $1"; }
log_success() { echo -e "${GREEN}[SUCCESS]${NC} $1"; }
log_warn() { echo -e "${YELLOW}[WARN]${NC} $1"; }
log_error() { echo -e "${RED}[ERROR]${NC} $1"; }
log_header() { echo -e "\n${CYAN}=== $1 ===${NC}\n"; }

# Check prerequisites
check_prerequisites() {
    log_info "Checking prerequisites..."

    if ! command -v docker &> /dev/null; then
        log_error "Docker is not installed. Please install Docker first."
        exit 1
    fi

    if ! docker compose version &> /dev/null && ! docker-compose version &> /dev/null; then
        log_error "Docker Compose is not installed. Please install Docker Compose first."
        exit 1
    fi

    if ! docker info &> /dev/null; then
        log_error "Docker daemon is not running. Please start Docker first."
        exit 1
    fi

    if [[ ! -f "$COMPOSE_FILE" ]]; then
        log_error "Docker Compose file not found at: $COMPOSE_FILE"
        exit 1
    fi

    log_success "Prerequisites check passed"
}

# Build the test binary
build_binary() {
    log_header "Building Test Binary"

    cd "$PROJECT_ROOT"

    # Check if cargo is available
    if ! command -v cargo &> /dev/null; then
        log_error "Rust/Cargo not installed. Please install Rust first."
        exit 1
    fi

    # Determine target - if on macOS, we need to cross-compile for Linux
    local target=""
    local build_cmd="cargo build --release -p quic-test"

    if [[ "$(uname)" == "Darwin" ]]; then
        log_info "Detected macOS - cross-compiling for Linux (x86_64-unknown-linux-musl)"

        # Check if cross or cargo-zigbuild is available
        if command -v cross &> /dev/null; then
            target="x86_64-unknown-linux-musl"
            build_cmd="cross build --release -p quic-test --target $target"
        elif command -v cargo-zigbuild &> /dev/null; then
            target="x86_64-unknown-linux-musl"
            build_cmd="cargo zigbuild --release -p quic-test --target $target"
        else
            log_warn "Neither 'cross' nor 'cargo-zigbuild' found."
            log_info "Installing cargo-zigbuild..."
            cargo install cargo-zigbuild

            # Also need zig
            if ! command -v zig &> /dev/null; then
                log_info "Installing zig via Homebrew..."
                brew install zig
            fi

            target="x86_64-unknown-linux-musl"
            build_cmd="cargo zigbuild --release -p quic-test --target $target"
        fi

        # Add musl target if needed
        if ! rustup target list --installed | grep -q "$target"; then
            log_info "Adding Rust target: $target"
            rustup target add "$target"
        fi
    fi

    log_info "Building with: $build_cmd"
    eval "$build_cmd"

    # Create bin directory if it doesn't exist
    mkdir -p "$BIN_DIR"

    # Copy binary to the docker bin directory
    local binary_name="saorsa-quic-test"
    local source_path

    if [[ -n "$target" ]]; then
        source_path="$PROJECT_ROOT/target/$target/release/$binary_name"
    else
        source_path="$PROJECT_ROOT/target/release/$binary_name"
    fi

    if [[ -f "$source_path" ]]; then
        cp "$source_path" "$BIN_DIR/$binary_name"
        chmod +x "$BIN_DIR/$binary_name"
        log_success "Binary copied to: $BIN_DIR/$binary_name"
    else
        log_error "Binary not found at: $source_path"
        exit 1
    fi
}

# Start the testnet
start_testnet() {
    log_header "Starting Local Testnet"

    check_prerequisites

    # Check if binary exists
    if [[ ! -f "$BIN_DIR/saorsa-quic-test" ]]; then
        log_warn "Test binary not found. Building first..."
        build_binary
    fi

    log_info "Using compose file: $COMPOSE_FILE"

    log_info "Building Docker images..."
    docker compose -f "$COMPOSE_FILE" build

    log_info "Starting containers..."
    docker compose -f "$COMPOSE_FILE" up -d

    log_success "Local testnet started!"
    echo ""
    log_info "Nodes starting up:"
    echo "  - Registry:        http://localhost:18080/api/stats"
    echo "  - Public nodes:    3"
    echo "  - Full Cone NAT:   4 nodes"
    echo "  - Port-Restricted: 4 nodes"
    echo "  - Symmetric NAT:   4 nodes"
    echo "  - CGNAT:           4 nodes"
    echo "  - Total:           20 nodes (including registry)"
    echo ""
    log_info "Use './scripts/local-testnet.sh status' to check node health"
    log_info "Use './scripts/local-testnet.sh monitor' for live monitoring"
}

# Stop the testnet
stop_testnet() {
    log_header "Stopping Local Testnet"

    docker compose -f "$COMPOSE_FILE" down

    log_success "Local testnet stopped"
}

# Restart the testnet
restart_testnet() {
    stop_testnet
    start_testnet
}

# Show status
show_status() {
    log_header "Local Testnet Status"

    echo "Container Status:"
    docker compose -f "$COMPOSE_FILE" ps

    echo ""
    log_info "Registry API:"
    if curl -s "http://localhost:18080/api/stats" 2>/dev/null | head -c 500; then
        echo ""
    else
        log_warn "Registry not responding yet (may still be starting)"
    fi

    echo ""
    log_info "Peer Count:"
    local peer_count
    peer_count=$(curl -s "http://localhost:18080/api/peers" 2>/dev/null | jq 'length' 2>/dev/null || echo "0")
    echo "  Registered peers: $peer_count"
}

# View logs
view_logs() {
    if [[ -n "${1:-}" ]]; then
        # Specific container
        docker compose -f "$COMPOSE_FILE" logs -f "$1"
    else
        # All containers
        docker compose -f "$COMPOSE_FILE" logs -f
    fi
}

# Live monitoring dashboard
monitor() {
    log_header "Live Monitoring Dashboard"
    log_info "Press Ctrl+C to exit"
    echo ""

    while true; do
        clear
        echo -e "${CYAN}=== Local Testnet Monitor ===${NC}"
        echo -e "Time: $(date)"
        echo ""

        # Container status summary
        echo -e "${BLUE}Container Status:${NC}"
        local running
        running=$(docker compose -f "$COMPOSE_FILE" ps --status running -q 2>/dev/null | wc -l | tr -d ' ')
        local total
        total=$(docker compose -f "$COMPOSE_FILE" ps -q 2>/dev/null | wc -l | tr -d ' ')
        echo "  Running: $running / $total containers"
        echo ""

        # Registry stats
        echo -e "${BLUE}Registry Stats:${NC}"
        if curl -s "http://localhost:18080/api/stats" 2>/dev/null | jq -r '
            "  Total Peers: \(.total_peers // 0)",
            "  Active: \(.active_peers // 0)",
            "  Connections: \(.total_connections // 0)"
        ' 2>/dev/null; then
            :
        else
            echo "  Registry not responding"
        fi
        echo ""

        # NAT type distribution
        echo -e "${BLUE}Peer Distribution by NAT Type:${NC}"
        if curl -s "http://localhost:18080/api/peers" 2>/dev/null | jq -r '
            group_by(.nat_type) |
            map({nat_type: .[0].nat_type, count: length}) |
            sort_by(-.count) |
            .[] |
            "  \(.nat_type // "unknown"): \(.count)"
        ' 2>/dev/null; then
            :
        else
            echo "  No peer data available"
        fi
        echo ""

        # Recent connections
        echo -e "${BLUE}Recent Activity (last 10 log lines from registry):${NC}"
        docker logs testnet-registry 2>&1 | tail -10
        echo ""

        echo -e "${YELLOW}Refreshing in 5 seconds... (Ctrl+C to exit)${NC}"
        sleep 5
    done
}

# Clean up everything
clean_all() {
    log_header "Cleaning Up Local Testnet"

    log_info "Stopping and removing containers..."
    docker compose -f "$COMPOSE_FILE" down -v --remove-orphans

    log_info "Removing images..."
    docker compose -f "$COMPOSE_FILE" down --rmi local 2>/dev/null || true

    log_info "Removing binary..."
    rm -rf "$BIN_DIR"

    log_success "Cleanup complete"
}

# Run extended test
run_test() {
    local duration="${1:-$DEFAULT_TEST_DURATION}"
    local end_time=$(($(date +%s) + duration))

    log_header "Starting Extended Test"
    log_info "Test duration: $duration seconds ($(echo "scale=1; $duration/3600" | bc) hours)"
    log_info "End time: $(date -d "@$end_time" 2>/dev/null || date -r "$end_time")"
    echo ""

    # Start testnet if not running
    if ! docker compose -f "$COMPOSE_FILE" ps --status running -q 2>/dev/null | grep -q .; then
        start_testnet
        log_info "Waiting 60 seconds for nodes to initialize..."
        sleep 60
    fi

    local test_start=$(date +%s)
    local check_interval=60
    local issues_found=0
    local log_file="$PROJECT_ROOT/testnet-$(date +%Y%m%d-%H%M%S).log"

    log_info "Logging to: $log_file"
    echo ""

    {
        echo "=== Local Testnet Extended Test ==="
        echo "Started: $(date)"
        echo "Duration: $duration seconds"
        echo ""
    } > "$log_file"

    while [[ $(date +%s) -lt $end_time ]]; do
        local elapsed=$(($(date +%s) - test_start))
        local remaining=$((end_time - $(date +%s)))
        local progress=$((elapsed * 100 / duration))

        echo -ne "\r${BLUE}Progress: ${progress}% | Elapsed: ${elapsed}s | Remaining: ${remaining}s | Issues: ${issues_found}${NC}   "

        # Check container health
        local running
        running=$(docker compose -f "$COMPOSE_FILE" ps --status running -q 2>/dev/null | wc -l | tr -d ' ')

        # Get peer count
        local peer_count
        peer_count=$(curl -s "http://localhost:18080/api/peers" 2>/dev/null | jq 'length' 2>/dev/null || echo "0")

        # Log status
        {
            echo "[$(date '+%Y-%m-%d %H:%M:%S')] Running: $running containers, Peers: $peer_count"
        } >> "$log_file"

        # Check for issues
        if [[ "$running" -lt 15 ]]; then
            ((issues_found++))
            {
                echo "[ISSUE] Low container count: $running"
                docker compose -f "$COMPOSE_FILE" ps
            } >> "$log_file"
        fi

        # Check for potential deadlocks by looking for stuck processes
        for container in $(docker compose -f "$COMPOSE_FILE" ps -q 2>/dev/null); do
            local name
            name=$(docker inspect --format '{{.Name}}' "$container" | sed 's/^\///')

            # Check if container is responsive
            if ! docker exec "$container" echo "alive" &>/dev/null; then
                ((issues_found++))
                {
                    echo "[ISSUE] Container unresponsive: $name"
                } >> "$log_file"
            fi
        done

        # Check for error patterns in logs
        local error_count
        error_count=$(docker compose -f "$COMPOSE_FILE" logs --since 1m 2>&1 | grep -ci "error\|panic\|deadlock\|timeout" 2>/dev/null || true)
        error_count=${error_count:-0}
        error_count=$(echo "$error_count" | tr -d '[:space:]' | head -c 10)
        if [[ -z "$error_count" ]] || ! [[ "$error_count" =~ ^[0-9]+$ ]]; then
            error_count=0
        fi
        if [[ "$error_count" -gt 0 ]]; then
            ((issues_found++))
            {
                echo "[ISSUE] Errors detected in logs ($error_count occurrences)"
                docker compose -f "$COMPOSE_FILE" logs --since 1m 2>&1 | grep -i "error\|panic\|deadlock\|timeout" | head -20
            } >> "$log_file"
        fi

        sleep "$check_interval"
    done

    echo ""
    echo ""
    log_header "Test Complete"

    {
        echo ""
        echo "=== Test Summary ==="
        echo "Completed: $(date)"
        echo "Duration: $duration seconds"
        echo "Issues found: $issues_found"
    } >> "$log_file"

    if [[ "$issues_found" -eq 0 ]]; then
        log_success "Test completed with no issues detected!"
    else
        log_warn "Test completed with $issues_found issues detected."
        log_info "See log file for details: $log_file"
    fi

    # Final status
    show_status
}

# Print usage
print_usage() {
    echo "Local Testnet Management Script"
    echo ""
    echo "Usage: $0 <command> [options]"
    echo ""
    echo "Commands:"
    echo "  build           Build the test binary for Docker"
    echo "  start           Start the 20-node local testnet"
    echo "  stop            Stop the testnet"
    echo "  restart         Restart the testnet"
    echo "  status          Show node status and peer count"
    echo "  logs [node]     View logs (all or specific node)"
    echo "  monitor         Live monitoring dashboard"
    echo "  clean           Remove all containers, volumes, and images"
    echo "  test [seconds]  Run extended test (default: 3600s = 1 hour)"
    echo ""
    echo "Examples:"
    echo "  $0 build                    # Build binary"
    echo "  $0 start                    # Start testnet"
    echo "  $0 test 7200                # Run 2-hour test"
    echo "  $0 logs testnet-registry    # View registry logs"
    echo "  $0 monitor                  # Live dashboard"
}

# Main entry point
main() {
    if [[ $# -lt 1 ]]; then
        print_usage
        exit 1
    fi

    local command="$1"
    shift

    case "$command" in
        build)
            build_binary
            ;;
        start)
            start_testnet
            ;;
        stop)
            stop_testnet
            ;;
        restart)
            restart_testnet
            ;;
        status)
            show_status
            ;;
        logs)
            view_logs "$@"
            ;;
        monitor)
            monitor
            ;;
        clean)
            clean_all
            ;;
        test)
            run_test "$@"
            ;;
        -h|--help)
            print_usage
            ;;
        *)
            log_error "Unknown command: $command"
            print_usage
            exit 1
            ;;
    esac
}

main "$@"
