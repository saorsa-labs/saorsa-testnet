#!/usr/bin/env python3
"""
Destroy all VMs for a testnet.

Usage:
    python3 destroy.py --name my-testnet
    python3 destroy.py --name my-testnet --yes  # Skip confirmation

Destroys all VMs by their cloud provider IDs (from state file),
then cleans up the state directory.
"""

import argparse
import json
import os
import sys
import urllib.request

from config import STATE_DIR, get_do_token, get_hc_token
from provision import load_state, do_api, hc_api


def destroy_do_droplet(token, droplet_id, name):
    """Destroy a single DO droplet."""
    try:
        do_api("DELETE", f"/droplets/{droplet_id}", token)
        print(f"  Destroyed DO droplet: {name} (id={droplet_id})")
        return True
    except Exception as e:
        print(f"  FAILED to destroy DO droplet {name} (id={droplet_id}): {e}")
        return False


def destroy_hc_server(token, server_id, name):
    """Destroy a single Hetzner Cloud server."""
    try:
        hc_api("DELETE", f"/servers/{server_id}", token)
        print(f"  Destroyed HC server: {name} (id={server_id})")
        return True
    except Exception as e:
        print(f"  FAILED to destroy HC server {name} (id={server_id}): {e}")
        return False


def destroy(testnet_name, skip_confirm=False):
    """Destroy all VMs for a testnet and clean up state."""
    state = load_state(testnet_name)
    vms = state["vms"]

    do_vms = [v for v in vms if v["provider"] == "do"]
    hc_vms = [v for v in vms if v["provider"] == "hc"]

    print(f"{'='*60}")
    print(f"DESTROY TESTNET: {testnet_name}")
    print(f"{'='*60}")
    print(f"  DO droplets: {len(do_vms)}")
    print(f"  HC servers:  {len(hc_vms)}")
    print(f"  Total VMs:   {len(vms)}")
    print()

    for vm in vms:
        print(f"  {vm['name']} ({vm['provider']}/{vm.get('region', '?')}) "
              f"ip={vm.get('ip', 'none')} id={vm.get('id', '?')}")
    print()

    if not skip_confirm:
        answer = input("Type 'yes' to destroy all VMs: ").strip()
        if answer != "yes":
            print("Aborted.")
            sys.exit(0)

    ok_count = 0
    fail_count = 0

    # Destroy DO droplets
    if do_vms:
        do_token = get_do_token()
        for vm in do_vms:
            if destroy_do_droplet(do_token, vm["id"], vm["name"]):
                ok_count += 1
            else:
                fail_count += 1

    # Destroy HC servers
    if hc_vms:
        hc_token = get_hc_token()
        for vm in hc_vms:
            if destroy_hc_server(hc_token, vm["id"], vm["name"]):
                ok_count += 1
            else:
                fail_count += 1

    # Clean up state (mark as destroyed, don't delete the file for audit trail)
    state_path = os.path.join(STATE_DIR, testnet_name, "vms.json")
    state["destroyed_at"] = __import__("time").strftime("%Y-%m-%dT%H:%M:%SZ",
                                                         __import__("time").gmtime())
    state["destroyed"] = True
    with open(state_path, "w") as f:
        json.dump(state, f, indent=2)

    print()
    print(f"{'='*60}")
    print(f"DESTROY COMPLETE")
    print(f"{'='*60}")
    print(f"  Destroyed: {ok_count}")
    print(f"  Failed:    {fail_count}")
    if fail_count > 0:
        print(f"  WARNING: {fail_count} VMs could not be destroyed.")
        print(f"  Check cloud provider dashboards manually.")
    print(f"  State: {state_path} (marked as destroyed)")
    print(f"{'='*60}")


# ---------------------------------------------------------------------------
# CLI
# ---------------------------------------------------------------------------

def main():
    parser = argparse.ArgumentParser(description="Destroy testnet VMs")
    parser.add_argument("--name", required=True, help="Testnet name")
    parser.add_argument("--yes", "-y", action="store_true",
                        help="Skip confirmation prompt")
    args = parser.parse_args()

    destroy(args.name, skip_confirm=args.yes)


if __name__ == "__main__":
    main()
