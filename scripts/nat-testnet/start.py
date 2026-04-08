#!/usr/bin/env python3
"""
Start ant-node processes on all VMs.

Usage:
    python3 start.py --name my-testnet
    python3 start.py --name my-testnet --bootstrap-wait 45

Strategy:
  1. Start bootstrap nodes first (public, no NAT)
  2. Wait for bootstrap nodes to be reachable
  3. Start public + NAT nodes with bootstrap addresses
  4. Wait for network formation
  5. Report process counts
"""

import argparse
import sys
import time

from config import (
    NODE_BASE_PORT, REWARDS_ADDRESS, SECRET_KEY,
    NODE_STARTUP_WAIT, NETWORK_FORMATION_WAIT,
)
from provision import load_state
from ssh_utils import ssh_run, ssh_run_quiet, write_remote_file


def make_start_script(vm, nodes_per_vm, bootstrap_addrs=None):
    """
    Generate a start.sh script for a VM.

    The script is written to /opt/ant/start.sh and executed via SSH.
    This approach works around nohup-via-SSH detach issues.
    """
    role = vm["role"]
    ip = vm["ip"]
    is_nat = role == "nat"
    is_client = role == "client"

    # Client VMs don't run nodes
    if is_client:
        return "#!/bin/bash\necho 'Client VM -- no nodes to start'\n"

    run_prefix = "ip netns exec nat-sim" if is_nat else ""
    base_port = NODE_BASE_PORT

    # Build bootstrap flags
    bootstrap_flags = ""
    if bootstrap_addrs:
        for addr in bootstrap_addrs:
            bootstrap_flags += f" --peer {addr}"

    lines = [
        "#!/bin/bash",
        "set -e",
        "",
        "# Kill any existing ant-node processes",
        "pkill -f ant-node 2>/dev/null || true",
        "sleep 2",
        "",
        "# Clean up old data",
        "for d in /opt/ant/data-*; do",
        '    [ -d "$d" ] && find "$d" -mindepth 1 -delete',
        "done",
        "",
        "mkdir -p /var/log/ant-nodes",
        "",
        f"echo 'Starting {nodes_per_vm} node(s) on {ip} (role={role})'",
        "",
    ]

    for i in range(nodes_per_vm):
        port = base_port + i
        data_dir = f"/opt/ant/data-{i+1}"
        log_file = f"/var/log/ant-nodes/node-{i+1}.log"

        lines.extend([
            f"mkdir -p {data_dir}",
            f"echo 'Starting node {i+1} on port {port}...'",
            f"nohup {run_prefix} /opt/ant/ant-node \\",
            f"    --port {port} \\",
            f"    --ip 0.0.0.0 \\",
            f"    --rewards-address {REWARDS_ADDRESS} \\",
            f"    --home-network \\",
            f"    --root-dir {data_dir} \\",
        ])

        if bootstrap_flags:
            lines.append(f"    {bootstrap_flags.strip()} \\")

        lines.extend([
            f"    > {log_file} 2>&1 &",
            "",
            "sleep 1",
            "",
        ])

    lines.extend([
        "sleep 3",
        "",
        "# Count running processes",
        "count=$(ps aux | grep -c '[a]nt-node' || echo 0)",
        f'echo "Running: $count/{nodes_per_vm} ant-node processes"',
    ])

    return "\n".join(lines) + "\n"


def get_bootstrap_addresses(state):
    """
    Collect bootstrap node addresses (ip:port) from state.
    Bootstrap nodes are the ones started first that other nodes connect to.
    """
    addrs = []
    nodes_per_vm = state.get("nodes_per_vm", 2)
    for vm in state["vms"]:
        if vm["role"] == "bootstrap" and vm.get("ip"):
            for i in range(nodes_per_vm):
                port = NODE_BASE_PORT + i
                addrs.append(f"{vm['ip']}:{port}")
    return addrs


def start_vms(vms, nodes_per_vm, bootstrap_addrs=None, label=""):
    """Start nodes on a set of VMs. Returns (ok_count, fail_count)."""
    ok = 0
    fail = 0
    for vm in vms:
        ip = vm.get("ip")
        if not ip:
            print(f"  [{vm['name']}] SKIPPING (no IP)")
            fail += 1
            continue

        name = vm["name"]
        script = make_start_script(vm, nodes_per_vm, bootstrap_addrs)

        try:
            write_remote_file(ip, "/opt/ant/start.sh", script)
            ssh_run(ip, "chmod +x /opt/ant/start.sh && bash /opt/ant/start.sh",
                    timeout=60)
            print(f"  [{name}] Started ({vm['role']})")
            ok += 1
        except Exception as e:
            print(f"  [{name}] FAILED: {e}")
            fail += 1

    return ok, fail


def check_process_counts(vms):
    """Check how many ant-node processes are running on each VM."""
    total = 0
    for vm in vms:
        ip = vm.get("ip")
        if not ip or vm["role"] == "client":
            continue

        name = vm["name"]
        rc, stdout, _ = ssh_run_quiet(ip, "ps aux | grep -c '[a]nt-node'")
        count = 0
        if rc == 0 and stdout.strip().isdigit():
            count = int(stdout.strip())
        total += count
        print(f"  [{name}] {count} processes ({vm['role']})")

    return total


def start(testnet_name, bootstrap_wait=None, formation_wait=None):
    """Start all nodes in the testnet."""
    state = load_state(testnet_name)
    vms = state["vms"]
    nodes_per_vm = state.get("nodes_per_vm", 2)

    if bootstrap_wait is None:
        bootstrap_wait = NODE_STARTUP_WAIT
    if formation_wait is None:
        formation_wait = NETWORK_FORMATION_WAIT

    bootstrap_vms = [v for v in vms if v["role"] == "bootstrap"]
    public_vms = [v for v in vms if v["role"] == "public"]
    nat_vms = [v for v in vms if v["role"] == "nat"]
    client_vms = [v for v in vms if v["role"] == "client"]

    print(f"Starting testnet '{testnet_name}'")
    print(f"  Bootstrap: {len(bootstrap_vms)} VMs")
    print(f"  Public:    {len(public_vms)} VMs")
    print(f"  NAT:       {len(nat_vms)} VMs")
    print(f"  Client:    {len(client_vms)} VMs (no nodes)")
    print()

    # Phase 1: Start bootstrap nodes (no --peer flags)
    print("Phase 1: Starting bootstrap nodes...")
    ok, fail = start_vms(bootstrap_vms, nodes_per_vm)
    print(f"  Bootstrap: {ok} OK, {fail} failed")

    if ok == 0:
        print("ERROR: No bootstrap nodes started. Aborting.")
        sys.exit(1)

    # Wait for bootstrap nodes to be ready
    print(f"\nWaiting {bootstrap_wait}s for bootstrap nodes to initialize...")
    time.sleep(bootstrap_wait)

    # Collect bootstrap addresses
    bootstrap_addrs = get_bootstrap_addresses(state)
    print(f"Bootstrap addresses: {bootstrap_addrs}")
    print()

    # Phase 2: Start public nodes
    if public_vms:
        print("Phase 2a: Starting public nodes...")
        ok, fail = start_vms(public_vms, nodes_per_vm, bootstrap_addrs)
        print(f"  Public: {ok} OK, {fail} failed")

    # Phase 2b: Start NAT nodes
    if nat_vms:
        print("Phase 2b: Starting NAT nodes...")
        ok, fail = start_vms(nat_vms, nodes_per_vm, bootstrap_addrs)
        print(f"  NAT: {ok} OK, {fail} failed")

    # Wait for network formation
    print(f"\nWaiting {formation_wait}s for network formation...")
    time.sleep(formation_wait)

    # Check process counts
    print("\nProcess counts:")
    total = check_process_counts(vms)
    expected = sum(
        nodes_per_vm
        for v in vms
        if v["role"] != "client" and v.get("ip")
    )

    print(f"\nTotal: {total}/{expected} processes running")
    if total < expected:
        print("WARNING: Some nodes may have failed to start. Check logs.")


# ---------------------------------------------------------------------------
# CLI
# ---------------------------------------------------------------------------

def main():
    parser = argparse.ArgumentParser(description="Start ant-node processes")
    parser.add_argument("--name", required=True, help="Testnet name")
    parser.add_argument("--bootstrap-wait", type=int, default=None,
                        help=f"Seconds to wait after bootstrap (default: {NODE_STARTUP_WAIT})")
    parser.add_argument("--formation-wait", type=int, default=None,
                        help=f"Seconds to wait for mesh (default: {NETWORK_FORMATION_WAIT})")
    args = parser.parse_args()

    start(args.name, args.bootstrap_wait, args.formation_wait)


if __name__ == "__main__":
    main()
