#!/usr/bin/env python3
"""
Provision VMs across DigitalOcean and Hetzner Cloud for NAT testnet testing.

Usage:
    python3 provision.py --name my-testnet --layout 60node-80nat
    python3 provision.py --name my-testnet --layout 60node-80nat --node-version v0.10.0-rc.13

Creates VMs according to the layout, waits for IPs, and saves state to
  state/<testnet-name>/vms.json
"""

import argparse
import json
import os
import sys
import time
import urllib.request

from config import (
    DO_KEY_ID, DO_REGIONS, DO_SIZE,
    HC_KEY_ID, HC_LOCATIONS, HC_MAX_SERVERS, HC_TYPE,
    STATE_DIR, VM_CREATION_WAIT,
    get_do_token, get_hc_token,
)
from layouts import get_layout, describe_layout


# ---------------------------------------------------------------------------
# DigitalOcean API helpers
# ---------------------------------------------------------------------------

def do_api(method, path, token, data=None):
    """Make a DigitalOcean API request. Returns parsed JSON or None."""
    url = f"https://api.digitalocean.com/v2{path}"
    body = json.dumps(data).encode() if data else None
    req = urllib.request.Request(
        url,
        data=body,
        method=method,
        headers={
            "Authorization": f"Bearer {token}",
            "Content-Type": "application/json",
        },
    )
    try:
        with urllib.request.urlopen(req, timeout=30) as resp:
            if resp.status == 204:
                return None
            return json.loads(resp.read().decode())
    except urllib.error.HTTPError as e:
        error_body = e.read().decode() if e.fp else ""
        print(f"  DO API error {e.code}: {error_body[:300]}", file=sys.stderr)
        raise


def do_create_droplet(token, name, region, tag):
    """Create a single DigitalOcean droplet. Returns droplet ID."""
    data = {
        "name": name,
        "region": region,
        "size": DO_SIZE,
        "image": "ubuntu-24-04-x64",
        "ssh_keys": [DO_KEY_ID],
        "tags": [tag],
        "monitoring": True,
    }
    result = do_api("POST", "/droplets", token, data)
    if not result or "droplet" not in result:
        print(f"  ERROR: Failed to create droplet {name}", file=sys.stderr)
        raise RuntimeError(f"Failed to create droplet {name}")
    droplet = result["droplet"]
    return droplet["id"]


def do_get_droplet(token, droplet_id):
    """Get droplet details including IP."""
    result = do_api("GET", f"/droplets/{droplet_id}", token)
    if not result or "droplet" not in result:
        return None
    return result["droplet"]


def do_wait_for_ip(token, droplet_id, timeout=120):
    """Poll until a droplet has a public IPv4 address."""
    deadline = time.time() + timeout
    while time.time() < deadline:
        droplet = do_get_droplet(token, droplet_id)
        if droplet:
            for net in droplet.get("networks", {}).get("v4", []):
                if net.get("type") == "public":
                    return net["ip_address"]
        time.sleep(5)
    return None


# ---------------------------------------------------------------------------
# Hetzner Cloud API helpers
# ---------------------------------------------------------------------------

def hc_api(method, path, token, data=None):
    """Make a Hetzner Cloud API request. Returns parsed JSON or None."""
    url = f"https://api.hetzner.cloud/v1{path}"
    body = json.dumps(data).encode() if data else None
    req = urllib.request.Request(
        url,
        data=body,
        method=method,
        headers={
            "Authorization": f"Bearer {token}",
            "Content-Type": "application/json",
        },
    )
    try:
        with urllib.request.urlopen(req, timeout=30) as resp:
            if resp.status == 204:
                return None
            return json.loads(resp.read().decode())
    except urllib.error.HTTPError as e:
        error_body = e.read().decode() if e.fp else ""
        print(f"  HC API error {e.code}: {error_body[:300]}", file=sys.stderr)
        raise


def hc_create_server(token, name, location, label_key, label_value):
    """Create a Hetzner Cloud server. Returns server dict with ID and IP."""
    data = {
        "name": name,
        "server_type": HC_TYPE,
        "location": location,
        "image": "ubuntu-24.04",
        "ssh_keys": [HC_KEY_ID],
        "labels": {label_key: label_value},
        "public_net": {
            "enable_ipv4": True,
            "enable_ipv6": False,
        },
    }
    result = hc_api("POST", "/servers", token, data)
    if not result or "server" not in result:
        print(f"  ERROR: Failed to create server {name}", file=sys.stderr)
        raise RuntimeError(f"Failed to create server {name}")
    server = result["server"]
    ip = server.get("public_net", {}).get("ipv4", {}).get("ip", "")
    return {"id": server["id"], "ip": ip}


# ---------------------------------------------------------------------------
# Provisioning logic
# ---------------------------------------------------------------------------

def provision_do_vms(token, testnet_name, role, count, tag):
    """
    Create `count` DO droplets for a given role.
    Returns a list of VM dicts: {id, name, ip, provider, role, region}.
    """
    vms = []
    for i in range(count):
        region = DO_REGIONS[i % len(DO_REGIONS)]
        name = f"{testnet_name}-do-{role}-{i+1}"
        print(f"  Creating DO droplet: {name} ({region}, {role})")
        droplet_id = do_create_droplet(token, name, region, tag)
        vms.append({
            "id": droplet_id,
            "name": name,
            "ip": None,  # Filled in after waiting
            "provider": "do",
            "role": role,
            "region": region,
        })
    return vms


def provision_hc_vms(token, testnet_name, role, count, label_key, label_value):
    """
    Create `count` HC servers for a given role.
    Returns a list of VM dicts.
    """
    vms = []
    for i in range(count):
        location = HC_LOCATIONS[i % len(HC_LOCATIONS)]
        name = f"{testnet_name}-hc-{role}-{i+1}"
        print(f"  Creating HC server: {name} ({location}, {role})")
        server = hc_create_server(token, name, location, label_key, label_value)
        vms.append({
            "id": server["id"],
            "name": name,
            "ip": server["ip"] or None,
            "provider": "hc",
            "role": role,
            "region": location,
        })
    return vms


def provision(testnet_name, layout_name, node_version=None, client_version=None):
    """
    Provision all VMs for a testnet. Saves state to disk and returns the VM list.
    """
    layout = get_layout(layout_name)
    do_cfg = layout.get("do", {})
    hc_cfg = layout.get("hc", {})

    # Check Hetzner limits
    hc_total = sum(v for k, v in hc_cfg.items() if k != "client")
    if hc_total > HC_MAX_SERVERS:
        print(f"ERROR: Hetzner layout needs {hc_total} servers but account limit is {HC_MAX_SERVERS}")
        sys.exit(1)

    tag = f"testnet-{testnet_name}"

    describe_layout(layout_name)
    print()

    all_vms = []

    # --- DigitalOcean ---
    if any(v > 0 for v in do_cfg.values()):
        do_token = get_do_token()
        for role, count in do_cfg.items():
            if count > 0:
                vms = provision_do_vms(do_token, testnet_name, role, count, tag)
                all_vms.extend(vms)

        # Wait for DO droplets to get IPs
        print(f"\nWaiting up to {VM_CREATION_WAIT}s for DO droplets to get IPs...")
        do_vms = [v for v in all_vms if v["provider"] == "do"]
        for vm in do_vms:
            ip = do_wait_for_ip(do_token, vm["id"], timeout=VM_CREATION_WAIT)
            if ip:
                vm["ip"] = ip
                print(f"  {vm['name']}: {ip}")
            else:
                print(f"  WARNING: {vm['name']} did not get an IP in time")

    # --- Hetzner Cloud ---
    if any(v > 0 for v in hc_cfg.values()):
        hc_token = get_hc_token()
        for role, count in hc_cfg.items():
            if count > 0:
                vms = provision_hc_vms(
                    hc_token, testnet_name, role, count,
                    "testnet", testnet_name,
                )
                all_vms.extend(vms)

        # Hetzner usually returns IPs immediately, but verify
        hc_vms = [v for v in all_vms if v["provider"] == "hc"]
        for vm in hc_vms:
            if vm["ip"]:
                print(f"  {vm['name']}: {vm['ip']}")
            else:
                print(f"  WARNING: {vm['name']} has no IP")

    # Verify all VMs have IPs
    missing_ip = [v for v in all_vms if not v.get("ip")]
    if missing_ip:
        print(f"\nWARNING: {len(missing_ip)} VMs have no IP address:")
        for v in missing_ip:
            print(f"  - {v['name']}")

    # Save state
    state_dir = os.path.join(STATE_DIR, testnet_name)
    os.makedirs(state_dir, exist_ok=True)
    state = {
        "testnet_name": testnet_name,
        "layout": layout_name,
        "node_version": node_version,
        "client_version": client_version,
        "nodes_per_vm": layout["nodes_per_vm"],
        "created_at": time.strftime("%Y-%m-%dT%H:%M:%SZ", time.gmtime()),
        "tag": tag,
        "vms": all_vms,
    }
    state_path = os.path.join(state_dir, "vms.json")
    with open(state_path, "w") as f:
        json.dump(state, f, indent=2)
    print(f"\nState saved to {state_path}")

    # Summary
    print(f"\n{'='*60}")
    print(f"PROVISIONING COMPLETE")
    print(f"{'='*60}")
    print(f"Testnet:  {testnet_name}")
    print(f"Layout:   {layout_name}")
    print(f"Total VMs: {len(all_vms)}")
    for role in ["bootstrap", "public", "nat", "client"]:
        role_vms = [v for v in all_vms if v["role"] == role]
        if role_vms:
            print(f"  {role}: {len(role_vms)}")
            for v in role_vms:
                print(f"    {v['name']} ({v['provider']}/{v['region']}): {v.get('ip', 'NO IP')}")
    print(f"{'='*60}")

    return state


def load_state(testnet_name):
    """Load testnet state from disk. Returns the state dict or exits."""
    state_path = os.path.join(STATE_DIR, testnet_name, "vms.json")
    if not os.path.exists(state_path):
        print(f"ERROR: No state found for testnet '{testnet_name}'")
        print(f"  Expected: {state_path}")
        print(f"  Run provision.py first.")
        sys.exit(1)
    with open(state_path) as f:
        return json.load(f)


# ---------------------------------------------------------------------------
# CLI
# ---------------------------------------------------------------------------

def main():
    parser = argparse.ArgumentParser(description="Provision VMs for NAT testnet")
    parser.add_argument("--name", required=True, help="Testnet name (used for tags/state)")
    parser.add_argument("--layout", required=True, help="Layout name from layouts.py")
    parser.add_argument("--node-version", default=None, help="ant-node release version (e.g. v0.10.0-rc.13)")
    parser.add_argument("--client-version", default=None, help="ant client release version")
    args = parser.parse_args()

    # Check state directory doesn't already exist for this name
    state_dir = os.path.join(STATE_DIR, args.name)
    if os.path.exists(os.path.join(state_dir, "vms.json")):
        print(f"ERROR: Testnet '{args.name}' already exists. Destroy it first or pick a new name.")
        sys.exit(1)

    provision(args.name, args.layout, args.node_version, args.client_version)


if __name__ == "__main__":
    main()
