#!/usr/bin/env python3
"""
Run a NAT testnet with a custom (patched) ant-node binary.

Usage:
    # Build your patched binary first:
    cd ant-node && cargo zigbuild --release --target x86_64-unknown-linux-gnu --bin ant-node \
        --config 'patch."https://github.com/saorsa-labs/saorsa-core.git".saorsa-core.path="../saorsa-core"' \
        --config 'patch."https://github.com/saorsa-labs/saorsa-transport.git".saorsa-transport.path="../saorsa-transport"'

    # Then run:
    python3 run_patched_test.py \
        --binary /path/to/ant-node \
        --name my-test \
        --uploads 3

Requires:
    - DIGITALOCEAN_TOKEN env var
    - SSH key at ~/.ssh/testnet_ed25519 (registered with DO key ID 55462233)
    - ant client binary will be downloaded from GitHub releases
"""

import argparse
import json
import os
import re
import subprocess
import sys
import time

# ============================================================
# CONFIG
# ============================================================

SSH_KEY = os.path.expanduser("~/.ssh/testnet_ed25519")
SSH_BASE = ["ssh", "-o", "StrictHostKeyChecking=no", "-o", "UserKnownHostsFile=/dev/null",
            "-o", "ConnectTimeout=15", "-i", SSH_KEY]
SCP_BASE = ["scp", "-o", "StrictHostKeyChecking=no", "-o", "UserKnownHostsFile=/dev/null",
            "-o", "ConnectTimeout=15", "-i", SSH_KEY]

DO_KEY = 55462233
REWARDS = "0x0000000000000000000000000000000000000001"
SECRET_KEY = "77579dc8a351606a8c27b9b37afd2527c66771b4dec7a69d3964e22cf74d17f2"
CLIENT_URL = "https://github.com/WithAutonomi/ant-client/releases/download/ant-cli-v0.1.2-rc.14/ant-0.1.2-rc.14-x86_64-unknown-linux-musl.tar.gz"


def log(msg):
    print(f"[{time.strftime('%H:%M:%S')}] {msg}", flush=True)


def curl_api(method, url, token, data=None):
    cmd = ["curl", "-s"]
    if method != "GET":
        cmd += ["-X", method]
    cmd += ["-H", f"Authorization: Bearer {token}", "-H", "Content-Type: application/json"]
    if data:
        cmd += ["-d", json.dumps(data)]
    cmd.append(url)
    r = subprocess.run(cmd, capture_output=True, text=True, timeout=30)
    return json.loads(r.stdout) if r.stdout else {}


def ssh_run(ip, cmd, timeout=120):
    r = subprocess.run(SSH_BASE + [f"root@{ip}", cmd],
                       capture_output=True, text=True, timeout=timeout)
    return r.stdout.strip(), r.returncode


def scp_to(local, ip, remote):
    return subprocess.run(SCP_BASE + ["-q", local, f"root@{ip}:{remote}"],
                          capture_output=True, timeout=180).returncode == 0


def wait_ssh(ip, max_tries=24):
    for _ in range(max_tries):
        try:
            _, rc = ssh_run(ip, "true", timeout=10)
            if rc == 0:
                return True
        except Exception:
            pass
        time.sleep(5)
    return False


# ============================================================
# VM LAYOUT
# ============================================================

LAYOUT = [
    ("lon1", "bootstrap"),
    ("ams3", "bootstrap"),
    ("nyc1", "bootstrap"),
    ("sfo3", "nat"),
    ("lon1", "nat"),
    ("ams3", "nat"),
    ("nyc1", "public"),
    ("sfo3", "client"),
]


def main():
    parser = argparse.ArgumentParser(description="Run NAT testnet with custom binary")
    parser.add_argument("--binary", required=True, help="Path to patched ant-node Linux binary")
    parser.add_argument("--name", default="patched-test", help="Test name (used for VM tags)")
    parser.add_argument("--uploads", type=int, default=3, help="Number of upload tests")
    parser.add_argument("--upload-size-mb", type=int, default=10, help="Upload file size in MB")
    parser.add_argument("--skip-destroy", action="store_true", help="Keep VMs alive after test")
    args = parser.parse_args()

    do_token = os.environ.get("DIGITALOCEAN_TOKEN")
    if not do_token:
        print("Error: DIGITALOCEAN_TOKEN env var required")
        sys.exit(1)

    if not os.path.isfile(args.binary):
        print(f"Error: binary not found: {args.binary}")
        sys.exit(1)

    prefix = f"anselme-{args.name}"

    # ============================================================
    # PROVISION
    # ============================================================
    log(f"PROVISIONING {len(LAYOUT)} VMs (prefix: {prefix})")

    vms = []
    for i, (region, role) in enumerate(LAYOUT):
        name = f"{prefix}-{region}-{i}"
        log(f"  Creating {name} ({region}, {role})")
        resp = curl_api("POST", "https://api.digitalocean.com/v2/droplets", do_token, {
            "name": name, "region": region, "size": "s-2vcpu-2gb",
            "image": "ubuntu-24-04-x64", "ssh_keys": [DO_KEY], "tags": [prefix]
        })
        did = resp.get("droplet", {}).get("id", "ERR")
        vms.append({"id": str(did), "name": name, "role": role, "ip": ""})

    log("Waiting 90s for IPs...")
    time.sleep(90)

    do_resp = curl_api("GET",
                       f"https://api.digitalocean.com/v2/droplets?tag_name={prefix}&per_page=100",
                       do_token)
    do_ips = {}
    for d in do_resp.get("droplets", []):
        for net in d.get("networks", {}).get("v4", []):
            if net.get("type") == "public":
                do_ips[str(d["id"])] = net["ip_address"]

    for vm in vms:
        vm["ip"] = do_ips.get(vm["id"], "")

    log("VMs:")
    for vm in vms:
        log(f"  {vm['name']:35} {vm['role']:10} {vm['ip']}")

    # ============================================================
    # SETUP
    # ============================================================
    log("SETUP: uploading binary + configuring NAT")

    for vm in vms:
        ip, role, name = vm["ip"], vm["role"], vm["name"]
        if not ip:
            continue
        log(f"  [{name}] waiting SSH...")
        if not wait_ssh(ip):
            log(f"  [{name}] SSH FAILED")
            continue

        ssh_run(ip, "mkdir -p /opt/ant /var/log/ant")
        if not scp_to(args.binary, ip, "/opt/ant/ant-node"):
            log(f"  [{name}] SCP FAILED")
            continue
        ssh_run(ip, "chmod +x /opt/ant/ant-node")

        if role == "client":
            ssh_run(ip, f"cd /tmp && wget -q '{CLIENT_URL}' -O c.tar.gz && tar xzf c.tar.gz && cp ant-*/ant /opt/ant/ant && chmod +x /opt/ant/ant",
                    timeout=60)

        if role == "nat":
            ssh_run(ip, """
apt-get update -qq >/dev/null 2>&1; apt-get install -y -qq iptables conntrack >/dev/null 2>&1
ip netns add nat-sim 2>/dev/null || true
ip link add veth-host type veth peer name veth-ns 2>/dev/null || true
ip link set veth-ns netns nat-sim 2>/dev/null || true
ip addr add 10.200.0.1/24 dev veth-host 2>/dev/null || true
ip netns exec nat-sim ip addr add 10.200.0.2/24 dev veth-ns 2>/dev/null || true
ip link set veth-host up; ip netns exec nat-sim ip link set veth-ns up
ip netns exec nat-sim ip link set lo up
ip netns exec nat-sim ip route add default via 10.200.0.1
sysctl -w net.ipv4.ip_forward=1 >/dev/null
EXT_IF=$(ip route show default | head -1 | awk '{print $5}')
PUBLIC_IP=$(ip -4 addr show $EXT_IF | grep -oP 'inet \\K[0-9.]+' | head -1)
iptables -t nat -F; iptables -F FORWARD
iptables -t nat -A POSTROUTING -s 10.200.0.0/24 -o $EXT_IF -j SNAT --to-source $PUBLIC_IP
iptables -A FORWARD -i veth-host -o $EXT_IF -j ACCEPT
iptables -A FORWARD -i $EXT_IF -o veth-host -m state --state ESTABLISHED -j ACCEPT
iptables -A FORWARD -i $EXT_IF -o veth-host -j DROP
mkdir -p /etc/netns/nat-sim; echo 'nameserver 8.8.8.8' > /etc/netns/nat-sim/resolv.conf
""", timeout=180)
            log(f"  [{name}] NAT OK")
        else:
            log(f"  [{name}] OK")

    # ============================================================
    # START NODES
    # ============================================================
    bootstrap_ips = [vm["ip"] for vm in vms if vm["role"] == "bootstrap" and vm["ip"]]
    bargs = " ".join(f"--bootstrap {ip}:10000" for ip in bootstrap_ips)

    log("STARTING NODES")

    for vm in vms:
        ip, role, name = vm["ip"], vm["role"], vm["name"]
        if not ip or role == "client":
            continue
        ns = "ip netns exec nat-sim " if role == "nat" else ""
        p1 = "10000" if role == "bootstrap" else "0"
        p2 = "10001" if role == "bootstrap" else "0"

        script = f"""#!/bin/bash
pkill -f ant-node 2>/dev/null; sleep 1
mkdir -p /opt/ant/data-1 /opt/ant/data-2 /var/log/ant
{ns}/opt/ant/ant-node --port {p1} --network-mode testnet --evm-network arbitrum-sepolia --rewards-address {REWARDS} --root-dir /opt/ant/data-1 {bargs} > /var/log/ant/node-1.log 2>&1 &
sleep 0.5
{ns}/opt/ant/ant-node --port {p2} --network-mode testnet --evm-network arbitrum-sepolia --rewards-address {REWARDS} --root-dir /opt/ant/data-2 {bargs} > /var/log/ant/node-2.log 2>&1 &
sleep 2
ps aux | grep -c '[a]nt-node'
"""
        ssh_run(ip, f"cat > /opt/ant/start.sh << 'SCRIPT'\n{script}\nSCRIPT\nchmod +x /opt/ant/start.sh")

    for phase in ["bootstrap", "nat", "public"]:
        if phase == "nat":
            log("Waiting 30s for bootstrap network...")
            time.sleep(30)
        for vm in vms:
            if vm["role"] != phase or not vm["ip"]:
                continue
            out, _ = ssh_run(vm["ip"], "bash /opt/ant/start.sh")
            procs = out.strip().split("\n")[-1] if out else "?"
            log(f"  {vm['name']}: {procs} procs")

    log("Waiting 120s for network formation + NAT hole-punching...")
    time.sleep(120)

    # ============================================================
    # UPLOAD TEST
    # ============================================================
    client_ip = next((vm["ip"] for vm in vms if vm["role"] == "client"), None)
    if not client_ip:
        log("ERROR: no client VM found")
    else:
        bflags = " ".join(f"-b {ip}:10000" for ip in bootstrap_ips)
        log(f"UPLOAD TEST from {client_ip}")

        results = []
        for attempt in range(1, args.uploads + 1):
            log(f"  Upload {attempt}/{args.uploads}...")
            upload_cmd = f"""
dd if=/dev/urandom of=/tmp/test.bin bs=1M count={args.upload_size_mb} 2>/dev/null
START=$(date +%s)
SECRET_KEY={SECRET_KEY} /opt/ant/ant {bflags} --evm-network arbitrum-sepolia --timeout-secs 300 file upload /tmp/test.bin 2>&1
RC=$?
END=$(date +%s)
echo "EXIT_CODE:$RC"
echo "DURATION:$((END - START))"
"""
            out, _ = ssh_run(client_ip, upload_cmd, timeout=600)

            duration = "?"
            exit_code = "?"
            for line in out.split("\n"):
                if line.startswith("DURATION:"):
                    duration = line.split(":")[1]
                if line.startswith("EXIT_CODE:"):
                    exit_code = line.split(":")[1]

            success = exit_code == "0" and "Error" not in out
            results.append({"attempt": attempt, "duration": duration, "success": success})
            status = "SUCCESS" if success else "FAILED"
            log(f"  Upload {attempt}: {status} in {duration}s")

        # Summary
        log("")
        log("RESULTS")
        log(f"{'Attempt':>8} {'Duration':>10} {'Result':>8}")
        for r in results:
            s = "OK" if r["success"] else "FAIL"
            log(f"{r['attempt']:>8} {r['duration']:>9}s {s:>8}")
        succeeded = sum(1 for r in results if r["success"])
        log(f"\n{succeeded}/{len(results)} uploads succeeded")

    # ============================================================
    # DESTROY
    # ============================================================
    if args.skip_destroy:
        log(f"VMs kept alive (tag: {prefix}). Destroy with:")
        log(f"  curl -X DELETE -H 'Authorization: Bearer $DIGITALOCEAN_TOKEN' "
            f"'https://api.digitalocean.com/v2/droplets?tag_name={prefix}'")
    else:
        log("DESTROYING VMs...")
        curl_api("DELETE", f"https://api.digitalocean.com/v2/droplets?tag_name={prefix}", do_token)
        log("All VMs destroyed.")

    log("DONE")


if __name__ == "__main__":
    main()
