#!/usr/bin/env python3
"""
Collect logs from all VMs in a testnet.

Usage:
    python3 logs.py --name my-testnet
    python3 logs.py --name my-testnet --output ./my-logs

Downloads all ant-node log files to a local directory organized by VM name:
    logs/<testnet-name>/<timestamp>/<vm-name>/node-1.log
    logs/<testnet-name>/<timestamp>/<vm-name>/node-2.log
    ...
"""

import argparse
import os
import subprocess
import sys
import time

from config import SCRIPT_DIR
from provision import load_state
from ssh_utils import ssh_run_quiet


def collect_logs(testnet_name, output_dir=None):
    """Download logs from all VMs."""
    state = load_state(testnet_name)
    vms = state["vms"]

    timestamp = time.strftime("%Y%m%d-%H%M%S")

    if output_dir:
        base_dir = output_dir
    else:
        base_dir = os.path.join(SCRIPT_DIR, "logs", testnet_name, timestamp)

    os.makedirs(base_dir, exist_ok=True)

    print(f"Collecting logs from {len(vms)} VMs...")
    print(f"Output: {base_dir}")
    print()

    collected = 0
    failed = 0

    for vm in vms:
        ip = vm.get("ip")
        name = vm["name"]
        role = vm["role"]

        if not ip:
            print(f"  [{name}] SKIPPING (no IP)")
            failed += 1
            continue

        if role == "client":
            print(f"  [{name}] SKIPPING (client VM)")
            continue

        vm_dir = os.path.join(base_dir, name)
        os.makedirs(vm_dir, exist_ok=True)

        # Check if logs exist
        rc, stdout, _ = ssh_run_quiet(ip, "ls /var/log/ant-nodes/node-*.log 2>/dev/null")
        if rc != 0 or not stdout.strip():
            print(f"  [{name}] No logs found")
            failed += 1
            continue

        # Also grab NAT config if it exists
        ssh_run_quiet(ip, "cat /etc/nat-sim/nat-type 2>/dev/null")

        # Download logs using scp with wildcard via bash
        try:
            key_path = os.path.expanduser("~/.ssh/testnet_ed25519")
            result = subprocess.run(
                [
                    "bash", "-c",
                    f'scp -o ConnectTimeout=15 -o StrictHostKeyChecking=no '
                    f'-o UserKnownHostsFile=/dev/null -o LogLevel=ERROR '
                    f'-i {key_path} '
                    f'"root@{ip}:/var/log/ant-nodes/node-*.log" '
                    f'"{vm_dir}/"'
                ],
                capture_output=True,
                text=True,
                timeout=120,
            )
            if result.returncode == 0:
                # Count downloaded files
                log_files = [f for f in os.listdir(vm_dir) if f.endswith(".log")]
                print(f"  [{name}] Collected {len(log_files)} log file(s)")
                collected += 1
            else:
                print(f"  [{name}] SCP failed: {result.stderr[:200]}")
                failed += 1
        except subprocess.TimeoutExpired:
            print(f"  [{name}] Timeout downloading logs")
            failed += 1
        except Exception as e:
            print(f"  [{name}] Error: {e}")
            failed += 1

        # Also save NAT type info
        rc, stdout, _ = ssh_run_quiet(ip, "cat /etc/nat-sim/nat-type 2>/dev/null")
        if rc == 0 and stdout.strip():
            nat_type_path = os.path.join(vm_dir, "nat-type.txt")
            with open(nat_type_path, "w") as f:
                f.write(stdout.strip() + "\n")

    print()
    print(f"{'='*60}")
    print(f"LOG COLLECTION COMPLETE")
    print(f"{'='*60}")
    print(f"  Collected: {collected} VMs")
    print(f"  Failed:    {failed} VMs")
    print(f"  Output:    {base_dir}")
    print(f"{'='*60}")

    return base_dir


# ---------------------------------------------------------------------------
# CLI
# ---------------------------------------------------------------------------

def main():
    parser = argparse.ArgumentParser(description="Collect logs from testnet VMs")
    parser.add_argument("--name", required=True, help="Testnet name")
    parser.add_argument("--output", "-o", default=None,
                        help="Output directory (default: logs/<name>/<timestamp>)")
    args = parser.parse_args()

    collect_logs(args.name, args.output)


if __name__ == "__main__":
    main()
