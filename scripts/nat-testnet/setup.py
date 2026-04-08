#!/usr/bin/env python3
"""
Set up VMs with ant-node binary and NAT simulation.

Usage:
    python3 setup.py --name my-testnet --node-version v0.10.0-rc.13
    python3 setup.py --name my-testnet --node-version v0.10.0-rc.13 --client-version v0.5.0

Reads state/<testnet-name>/vms.json and for each VM:
  1. Waits for SSH to become available
  2. Downloads ant-node binary from GitHub releases
  3. For NAT VMs: configures network namespace + iptables rules
  4. For client VMs: also downloads ant CLI client
  5. Verifies binary works and (for NAT VMs) namespace connectivity
"""

import argparse
import sys
import time

from config import (
    NODE_URL_TEMPLATE, CLIENT_URL_TEMPLATE,
    NODE_BASE_PORT, REWARDS_ADDRESS,
)
from provision import load_state
from ssh_utils import ssh_run, ssh_run_quiet, wait_for_ssh, write_remote_file


# ---------------------------------------------------------------------------
# NAT setup script (written to each NAT VM and executed)
# ---------------------------------------------------------------------------

NAT_SETUP_SCRIPT = r"""#!/bin/bash
set -euo pipefail

# Detect external interface and public IP without curl (avoids IPv6)
EXT_IF=$(ip route get 8.8.8.8 | head -1 | sed -n 's/.*dev \([^ ]*\).*/\1/p')
PUBLIC_IP=$(ip -4 addr show "$EXT_IF" | grep -oP 'inet \K[0-9.]+' | head -1)

echo "External interface: $EXT_IF"
echo "Public IP: $PUBLIC_IP"

NAMESPACE="nat-sim"
VETH_HOST="veth-host"
VETH_NS="veth-ns"
PRIVATE_IP="10.200.0.2"
HOST_IP="10.200.0.1"
SUBNET="10.200.0.0/24"

# Clean up any existing setup
ip netns del $NAMESPACE 2>/dev/null || true
ip link del $VETH_HOST 2>/dev/null || true

# Install ipset if not present
apt-get update -qq && apt-get install -y -qq ipset >/dev/null 2>&1 || true

# Create network namespace
ip netns add $NAMESPACE

# DNS for the namespace (systemd-resolved uses 127.0.0.53 which won't work)
mkdir -p /etc/netns/$NAMESPACE
cat > /etc/netns/$NAMESPACE/resolv.conf <<DNSEOF
nameserver 8.8.8.8
nameserver 1.1.1.1
nameserver 8.8.4.4
DNSEOF

# Create veth pair
ip link add $VETH_HOST type veth peer name $VETH_NS
ip link set $VETH_NS netns $NAMESPACE

# Configure host end
ip addr add $HOST_IP/24 dev $VETH_HOST
ip link set $VETH_HOST up

# Configure namespace end
ip netns exec $NAMESPACE ip addr add $PRIVATE_IP/24 dev $VETH_NS
ip netns exec $NAMESPACE ip link set $VETH_NS up
ip netns exec $NAMESPACE ip link set lo up
ip netns exec $NAMESPACE ip route add default via $HOST_IP

# Enable IP forwarding
echo 1 > /proc/sys/net/ipv4/ip_forward

# Flush existing rules
iptables -t nat -F POSTROUTING 2>/dev/null || true
iptables -t nat -F PREROUTING 2>/dev/null || true
iptables -F FORWARD 2>/dev/null || true

# SNAT for outbound
iptables -t nat -A POSTROUTING -s $SUBNET -o "$EXT_IF" -j SNAT --to-source $PUBLIC_IP

# ----- Port-Restricted Cone NAT (EIM/APDF) -----
# This is the hardest common NAT type to punch through.

# Clean up any ipsets from previous runs
ipset destroy nat-allowed-pairs 2>/dev/null || true

# Track destination IP:port pairs
ipset create nat-allowed-pairs hash:ip,port timeout 300

# Allow outbound from namespace
iptables -A FORWARD -i $VETH_HOST -o "$EXT_IF" -j ACCEPT

# Track where we send to
iptables -A FORWARD -i $VETH_HOST -o "$EXT_IF" -p udp \
    -j SET --add-set nat-allowed-pairs dst,dst

# Only accept inbound from IP:port pairs we've previously sent to
iptables -A FORWARD -i "$EXT_IF" -o $VETH_HOST -p udp \
    -m set --match-set nat-allowed-pairs src,src -j ACCEPT

# Also accept established/related as backup
iptables -A FORWARD -i "$EXT_IF" -o $VETH_HOST \
    -m state --state RELATED,ESTABLISHED -j ACCEPT

# Drop other inbound UDP
iptables -A FORWARD -i "$EXT_IF" -o $VETH_HOST -p udp -j DROP

# DNAT for all node ports (base port through base+10 to cover multiple nodes)
for port_offset in $(seq 0 10); do
    port=$((BASE_PORT + port_offset))
    iptables -t nat -A PREROUTING -i "$EXT_IF" -p udp --dport $port \
        -j DNAT --to-destination $PRIVATE_IP:$port
done

# Save config
mkdir -p /etc/nat-sim
cat > /etc/nat-sim/config <<CONFEOF
PUBLIC_IP=$PUBLIC_IP
NAMESPACE=$NAMESPACE
VETH_HOST=$VETH_HOST
VETH_NS=$VETH_NS
PRIVATE_IP=$PRIVATE_IP
HOST_IP=$HOST_IP
SUBNET=$SUBNET
EXT_IF=$EXT_IF
BASE_PORT=$BASE_PORT
CONFEOF

echo "port_restricted" > /etc/nat-sim/nat-type

echo "=== NAT setup complete ==="
echo "Type: Port-Restricted Cone (EIM/APDF)"
echo "Namespace: $NAMESPACE"
echo "Private IP: $PRIVATE_IP"

# Verify namespace can reach internet
if ip netns exec $NAMESPACE ping -c 1 -W 5 8.8.8.8 >/dev/null 2>&1; then
    echo "Namespace connectivity: OK"
else
    echo "WARNING: Namespace cannot reach internet"
    exit 1
fi
"""


# ---------------------------------------------------------------------------
# Setup functions
# ---------------------------------------------------------------------------

def setup_vm_base(vm):
    """Basic setup: create /opt/ant directory, install dependencies."""
    ip = vm["ip"]
    name = vm["name"]
    print(f"  [{name}] Base setup...")

    ssh_run(ip, "mkdir -p /opt/ant /var/log/ant-nodes")
    ssh_run(ip, "apt-get update -qq && apt-get install -y -qq tar gzip curl >/dev/null 2>&1",
            timeout=180)


def download_node_binary(vm, node_version):
    """Download and extract ant-node binary on the VM."""
    ip = vm["ip"]
    name = vm["name"]
    url = NODE_URL_TEMPLATE.format(version=node_version)

    print(f"  [{name}] Downloading ant-node {node_version}...")
    cmd = (
        f"cd /opt/ant && "
        f"curl -sSL '{url}' -o ant-node.tar.gz && "
        f"tar xzf ant-node.tar.gz && "
        f"chmod +x ant-node && "
        f"./ant-node --version"
    )
    result = ssh_run(ip, cmd, timeout=180)
    version_line = (result.stdout or "").strip().split("\n")[-1] if result.stdout else "unknown"
    print(f"  [{name}] ant-node installed: {version_line}")


def download_client_binary(vm, client_version):
    """Download and extract ant client binary on a client VM."""
    ip = vm["ip"]
    name = vm["name"]
    url = CLIENT_URL_TEMPLATE.format(version=client_version, client_version=client_version)

    print(f"  [{name}] Downloading ant client {client_version}...")
    cmd = (
        f"cd /opt/ant && "
        f"curl -sSL '{url}' -o ant-client.tar.gz && "
        f"tar xzf ant-client.tar.gz && "
        f"chmod +x ant && "
        f"./ant --version"
    )
    result = ssh_run(ip, cmd, timeout=180)
    version_line = (result.stdout or "").strip().split("\n")[-1] if result.stdout else "unknown"
    print(f"  [{name}] ant client installed: {version_line}")


def setup_nat(vm, base_port):
    """Configure port-restricted NAT simulation on a VM."""
    ip = vm["ip"]
    name = vm["name"]

    print(f"  [{name}] Setting up port-restricted NAT...")

    # Write the NAT setup script with the base port substituted
    script = NAT_SETUP_SCRIPT.replace("$BASE_PORT", str(base_port)).replace(
        "BASE_PORT=$BASE_PORT", f"BASE_PORT={base_port}"
    )
    # We need to handle the variable correctly -- use a simpler approach:
    # Write script, then run it with BASE_PORT as env var
    write_remote_file(ip, "/opt/ant/setup-nat.sh", NAT_SETUP_SCRIPT)
    ssh_run(ip, f"chmod +x /opt/ant/setup-nat.sh && BASE_PORT={base_port} bash /opt/ant/setup-nat.sh",
            timeout=120)
    print(f"  [{name}] NAT configured (port-restricted)")


def verify_vm(vm):
    """Verify that the VM is properly set up."""
    ip = vm["ip"]
    name = vm["name"]
    role = vm["role"]

    # Check binary
    rc, stdout, _ = ssh_run_quiet(ip, "/opt/ant/ant-node --version")
    if rc != 0:
        print(f"  [{name}] WARNING: ant-node binary not working")
        return False

    if role == "nat":
        # Check namespace exists
        rc, stdout, _ = ssh_run_quiet(ip, "ip netns list")
        if "nat-sim" not in (stdout or ""):
            print(f"  [{name}] WARNING: NAT namespace not found")
            return False

        # Check namespace connectivity
        rc, _, _ = ssh_run_quiet(ip, "ip netns exec nat-sim ping -c 1 -W 5 8.8.8.8")
        if rc != 0:
            print(f"  [{name}] WARNING: NAT namespace cannot reach internet")
            return False

        # Check binary works inside namespace
        rc, stdout, _ = ssh_run_quiet(ip, "ip netns exec nat-sim /opt/ant/ant-node --version")
        if rc != 0:
            print(f"  [{name}] WARNING: ant-node not working inside NAT namespace")
            return False

    print(f"  [{name}] Verified OK ({role})")
    return True


# ---------------------------------------------------------------------------
# Main setup orchestration
# ---------------------------------------------------------------------------

def setup(testnet_name, node_version, client_version=None):
    """
    Set up all VMs for a testnet.
    Processes VMs sequentially (parallel SSH had issues in testing).
    """
    state = load_state(testnet_name)
    vms = state["vms"]
    base_port = NODE_BASE_PORT

    # Use version from state if not provided
    if not node_version:
        node_version = state.get("node_version")
    if not node_version:
        print("ERROR: --node-version is required (not saved in state)")
        sys.exit(1)

    if not client_version:
        client_version = state.get("client_version")

    print(f"Setting up {len(vms)} VMs for testnet '{testnet_name}'")
    print(f"  Node version: {node_version}")
    if client_version:
        print(f"  Client version: {client_version}")
    print()

    # Wait for all VMs to be SSH-reachable
    print("Waiting for SSH on all VMs...")
    for vm in vms:
        ip = vm.get("ip")
        if not ip:
            print(f"  {vm['name']}: SKIPPING (no IP)")
            continue
        if not wait_for_ssh(ip):
            print(f"  {vm['name']}: SSH UNREACHABLE -- setup will fail")
            continue
        print(f"  {vm['name']}: SSH OK")
    print()

    # Setup each VM sequentially
    ok_count = 0
    fail_count = 0

    for vm in vms:
        ip = vm.get("ip")
        if not ip:
            print(f"  [{vm['name']}] SKIPPING (no IP)")
            fail_count += 1
            continue

        name = vm["name"]
        role = vm["role"]

        try:
            # Base setup
            setup_vm_base(vm)

            # Download binary
            download_node_binary(vm, node_version)

            # Client VMs also get the ant client
            if role == "client" and client_version:
                download_client_binary(vm, client_version)

            # NAT setup
            if role == "nat":
                setup_nat(vm, base_port)

            # Verify
            if verify_vm(vm):
                ok_count += 1
            else:
                fail_count += 1

        except Exception as e:
            print(f"  [{name}] SETUP FAILED: {e}")
            fail_count += 1

    print(f"\n{'='*60}")
    print(f"SETUP COMPLETE")
    print(f"{'='*60}")
    print(f"  OK:     {ok_count}")
    print(f"  Failed: {fail_count}")
    print(f"{'='*60}")

    if fail_count > 0:
        print("\nWARNING: Some VMs failed setup. Check output above.")
        return False
    return True


# ---------------------------------------------------------------------------
# CLI
# ---------------------------------------------------------------------------

def main():
    parser = argparse.ArgumentParser(description="Set up VMs for NAT testnet")
    parser.add_argument("--name", required=True, help="Testnet name")
    parser.add_argument("--node-version", default=None,
                        help="ant-node release version (e.g. v0.10.0-rc.13)")
    parser.add_argument("--client-version", default=None,
                        help="ant client release version")
    args = parser.parse_args()

    setup(args.name, args.node_version, args.client_version)


if __name__ == "__main__":
    main()
