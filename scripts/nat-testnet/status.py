#!/usr/bin/env python3
"""
Check network status for a running testnet.

Usage:
    python3 status.py --name my-testnet
    python3 status.py --name my-testnet --verbose

Reports:
  - Process counts per VM
  - Peer connection counts from logs
  - Hole-punch success rates from logs
  - Identity exchange failures from logs
"""

import argparse
import re
import sys

from provision import load_state
from ssh_utils import ssh_run_quiet


def get_process_count(ip):
    """Get the number of running ant-node processes."""
    rc, stdout, _ = ssh_run_quiet(ip, "ps aux | grep -c '[a]nt-node'")
    if rc == 0 and stdout.strip().isdigit():
        return int(stdout.strip())
    return 0


def get_log_stats(ip, verbose=False):
    """
    Parse ant-node logs for key metrics.
    Returns a dict with stats.
    """
    stats = {
        "peer_count": 0,
        "hole_punch_attempts": 0,
        "hole_punch_success": 0,
        "hole_punch_fail": 0,
        "identity_exchange_fail": 0,
        "connections_established": 0,
        "errors": 0,
    }

    # Get recent log content (last 500 lines from each node log)
    rc, stdout, _ = ssh_run_quiet(
        ip,
        "for f in /var/log/ant-nodes/node-*.log; do "
        "  [ -f \"$f\" ] && tail -500 \"$f\"; "
        "done",
        timeout=30,
    )
    if rc != 0 or not stdout:
        return stats

    for line in stdout.split("\n"):
        # Peer counts (look for "peers: N" or "connected_peers: N" patterns)
        m = re.search(r'(?:peers|connected_peers)[=: ]+(\d+)', line, re.IGNORECASE)
        if m:
            count = int(m.group(1))
            if count > stats["peer_count"]:
                stats["peer_count"] = count

        # Hole-punch attempts
        if "hole_punch" in line.lower() or "holepunch" in line.lower():
            stats["hole_punch_attempts"] += 1
            if "success" in line.lower() or "completed" in line.lower():
                stats["hole_punch_success"] += 1
            elif "fail" in line.lower() or "error" in line.lower() or "timeout" in line.lower():
                stats["hole_punch_fail"] += 1

        # Identity exchange failures
        if "identity" in line.lower() and ("fail" in line.lower() or "error" in line.lower()):
            stats["identity_exchange_fail"] += 1

        # Connection established
        if "connection established" in line.lower() or "new connection" in line.lower():
            stats["connections_established"] += 1

        # General errors
        if "ERROR" in line:
            stats["errors"] += 1

    return stats


def status(testnet_name, verbose=False):
    """Print full status report for a testnet."""
    state = load_state(testnet_name)
    vms = state["vms"]

    print(f"{'='*70}")
    print(f"TESTNET STATUS: {testnet_name}")
    print(f"{'='*70}")
    print(f"Layout: {state.get('layout', 'unknown')}")
    print(f"Nodes per VM: {state.get('nodes_per_vm', '?')}")
    print(f"Created: {state.get('created_at', 'unknown')}")
    print()

    # Process counts
    print(f"{'VM Name':<35} {'Role':<12} {'Procs':<8} {'Peers':<8} {'HP OK':<8} {'HP Fail':<8}")
    print("-" * 70)

    total_procs = 0
    total_expected = 0
    total_hp_ok = 0
    total_hp_fail = 0
    total_hp_attempts = 0
    total_id_fail = 0
    total_errors = 0

    for vm in vms:
        ip = vm.get("ip")
        name = vm["name"]
        role = vm["role"]

        if not ip:
            print(f"{name:<35} {role:<12} {'NO IP':<8}")
            continue

        if role == "client":
            print(f"{name:<35} {role:<12} {'(client)':<8}")
            continue

        procs = get_process_count(ip)
        expected = state.get("nodes_per_vm", 2)
        total_procs += procs
        total_expected += expected

        stats = get_log_stats(ip, verbose)
        total_hp_ok += stats["hole_punch_success"]
        total_hp_fail += stats["hole_punch_fail"]
        total_hp_attempts += stats["hole_punch_attempts"]
        total_id_fail += stats["identity_exchange_fail"]
        total_errors += stats["errors"]

        procs_str = f"{procs}/{expected}"
        print(f"{name:<35} {role:<12} {procs_str:<8} {stats['peer_count']:<8} "
              f"{stats['hole_punch_success']:<8} {stats['hole_punch_fail']:<8}")

        if verbose and stats["errors"] > 0:
            print(f"  {'':35} Errors: {stats['errors']}, "
                  f"ID exchange fails: {stats['identity_exchange_fail']}")

    print("-" * 70)

    # Summary
    print()
    print(f"{'='*70}")
    print(f"SUMMARY")
    print(f"{'='*70}")
    print(f"  Processes:    {total_procs}/{total_expected}")
    print(f"  Hole-punch:   {total_hp_ok} success / {total_hp_fail} fail "
          f"/ {total_hp_attempts} total attempts")
    if total_hp_attempts > 0:
        rate = total_hp_ok / total_hp_attempts * 100
        print(f"  HP success:   {rate:.1f}%")
    print(f"  ID exchange failures: {total_id_fail}")
    print(f"  Log errors:   {total_errors}")
    print(f"{'='*70}")

    # Return summary for use by run_round
    return {
        "total_procs": total_procs,
        "total_expected": total_expected,
        "hp_ok": total_hp_ok,
        "hp_fail": total_hp_fail,
        "hp_attempts": total_hp_attempts,
        "id_fail": total_id_fail,
        "errors": total_errors,
    }


# ---------------------------------------------------------------------------
# CLI
# ---------------------------------------------------------------------------

def main():
    parser = argparse.ArgumentParser(description="Check testnet status")
    parser.add_argument("--name", required=True, help="Testnet name")
    parser.add_argument("--verbose", "-v", action="store_true",
                        help="Show detailed per-VM stats")
    args = parser.parse_args()

    status(args.name, verbose=args.verbose)


if __name__ == "__main__":
    main()
