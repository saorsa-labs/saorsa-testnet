#!/usr/bin/env python3
"""
Run upload tests against a running testnet.

Usage:
    python3 upload.py --name my-testnet --count 5 --size 1M
    python3 upload.py --name my-testnet --count 10 --size 10M --from-vm client

Runs uploads from a client VM (or the first bootstrap VM if no client VM
exists) and reports timing for each upload.
"""

import argparse
import re
import sys
import time

from config import NODE_BASE_PORT, SECRET_KEY
from provision import load_state
from ssh_utils import ssh_run, ssh_run_quiet, write_remote_file


def parse_size(size_str):
    """Parse a human-readable size string to bytes."""
    size_str = size_str.strip().upper()
    multipliers = {
        "B": 1,
        "K": 1024,
        "KB": 1024,
        "M": 1024 * 1024,
        "MB": 1024 * 1024,
        "G": 1024 * 1024 * 1024,
        "GB": 1024 * 1024 * 1024,
    }
    for suffix, mult in sorted(multipliers.items(), key=lambda x: -len(x[0])):
        if size_str.endswith(suffix):
            num = size_str[:-len(suffix)].strip()
            try:
                return int(float(num) * mult)
            except ValueError:
                pass
    # Try as plain integer
    try:
        return int(size_str)
    except ValueError:
        print(f"ERROR: Cannot parse size '{size_str}'. Use e.g. 1M, 10M, 1G")
        sys.exit(1)


def find_upload_vm(state, preferred_role="client"):
    """Find the best VM to run uploads from."""
    vms = state["vms"]

    # Prefer client VMs
    for vm in vms:
        if vm["role"] == preferred_role and vm.get("ip"):
            return vm

    # Fall back to first bootstrap VM
    for vm in vms:
        if vm["role"] == "bootstrap" and vm.get("ip"):
            return vm

    # Fall back to first public VM
    for vm in vms:
        if vm["role"] == "public" and vm.get("ip"):
            return vm

    return None


def get_bootstrap_peers(state):
    """Get bootstrap peer addresses for the client."""
    peers = []
    nodes_per_vm = state.get("nodes_per_vm", 2)
    for vm in state["vms"]:
        if vm["role"] == "bootstrap" and vm.get("ip"):
            for i in range(nodes_per_vm):
                port = NODE_BASE_PORT + i
                peers.append(f"{vm['ip']}:{port}")
    return peers


def run_upload(testnet_name, count=5, size_str="1M", from_role="client"):
    """Run upload tests and report timing."""
    state = load_state(testnet_name)
    size_bytes = parse_size(size_str)

    vm = find_upload_vm(state, from_role)
    if not vm:
        print("ERROR: No suitable VM found for uploads")
        sys.exit(1)

    ip = vm["ip"]
    name = vm["name"]
    peers = get_bootstrap_peers(state)

    if not peers:
        print("ERROR: No bootstrap peers found")
        sys.exit(1)

    # Build peer flags
    peer_flags = " ".join(f"-b {p}" for p in peers)

    print(f"{'='*60}")
    print(f"UPLOAD TEST")
    print(f"{'='*60}")
    print(f"  Testnet:    {testnet_name}")
    print(f"  Upload VM:  {name} ({ip})")
    print(f"  File size:  {size_str} ({size_bytes} bytes)")
    print(f"  Count:      {count}")
    print(f"  Peers:      {len(peers)}")
    print()

    # Create test file on the VM
    print("Creating test file...")
    ssh_run(ip, f"dd if=/dev/urandom of=/tmp/test-upload bs={size_bytes} count=1 2>/dev/null",
            timeout=60)

    # Build the upload script
    upload_script_lines = [
        "#!/bin/bash",
        "set -e",
        "",
        f'echo "Running {count} uploads of {size_str} files..."',
        "",
    ]

    for i in range(1, count + 1):
        # Create a unique file for each upload (different content = different address)
        upload_script_lines.extend([
            f"# Upload {i}/{count}",
            f"dd if=/dev/urandom of=/tmp/test-upload-{i} bs={size_bytes} count=1 2>/dev/null",
            f'echo ""',
            f'echo "=== Upload {i}/{count} ==="',
            f"START_{i}=$(date +%s%N)",
            f"/opt/ant/ant file upload /tmp/test-upload-{i} "
            f"--secret-key {SECRET_KEY} {peer_flags} 2>&1 || echo 'UPLOAD FAILED'",
            f"END_{i}=$(date +%s%N)",
            f"ELAPSED_{i}=$(( (END_{i} - START_{i}) / 1000000 ))",
            f'echo "Upload {i} time: ${{ELAPSED_{i}}}ms"',
            "",
        ])

    # Summary
    upload_script_lines.append('echo ""')
    upload_script_lines.append('echo "=== TIMING SUMMARY ==="')
    for i in range(1, count + 1):
        upload_script_lines.append(
            f'echo "Upload {i}: ${{ELAPSED_{i}}}ms"'
        )

    upload_script = "\n".join(upload_script_lines) + "\n"

    write_remote_file(ip, "/opt/ant/upload-test.sh", upload_script)
    ssh_run(ip, "chmod +x /opt/ant/upload-test.sh", timeout=10)

    # Run the upload test
    print(f"Running {count} uploads...")
    print("-" * 60)
    try:
        result = ssh_run(
            ip,
            "bash /opt/ant/upload-test.sh",
            timeout=600,  # 10 min timeout for large uploads
            check=False,
        )
        if result.stdout:
            print(result.stdout)
        if result.stderr:
            # Only print stderr if there's something interesting
            stderr = result.stderr.strip()
            if stderr and "Warning" not in stderr:
                print(f"STDERR: {stderr[:500]}")
    except Exception as e:
        print(f"Upload test error: {e}")

    print("-" * 60)
    print("Upload test complete.")


# ---------------------------------------------------------------------------
# CLI
# ---------------------------------------------------------------------------

def main():
    parser = argparse.ArgumentParser(description="Run upload tests on a testnet")
    parser.add_argument("--name", required=True, help="Testnet name")
    parser.add_argument("--count", type=int, default=5,
                        help="Number of uploads to run (default: 5)")
    parser.add_argument("--size", default="1M",
                        help="File size per upload (default: 1M)")
    parser.add_argument("--from-vm", default="client",
                        help="Role of VM to upload from (default: client)")
    args = parser.parse_args()

    run_upload(args.name, args.count, args.size, args.from_vm)


if __name__ == "__main__":
    main()
