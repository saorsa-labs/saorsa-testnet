# NAT Testnet Provisioning Toolkit

Deterministic, reproducible testnet provisioning and NAT traversal testing
across DigitalOcean and Hetzner Cloud.

## Prerequisites

- Python 3.8+
- SSH key at `~/.ssh/testnet_ed25519` (already registered with both providers)
- Environment variables set:
  ```
  export DIGITALOCEAN_TOKEN="your-do-token"
  export HCLOUD_TOKEN="your-hetzner-token"
  ```

## Quick Start

Run a complete test round:

```bash
python3 run_round.py \
    --name r1-baseline \
    --node-version v0.10.0-rc.13 \
    --layout 60node-80nat
```

This will: provision VMs, install binaries, configure NAT simulation, start
nodes, check status, run uploads, collect logs, and destroy VMs.

## Available Layouts

| Name | Description | DO VMs | HC VMs | Nodes | NAT % |
|------|-------------|--------|--------|-------|-------|
| `60node-80nat` | Full test | 15 | 5 | ~38 | ~63% |
| `20node-quick` | Smoke test | 7 | 1 | ~14 | ~57% |
| `minimal` | Hole-punch test | 4 | 1 | 4 | ~50% |

## Individual Scripts

Each phase can be run independently:

```bash
# 1. Provision VMs
python3 provision.py --name test1 --layout 20node-quick

# 2. Install binaries + configure NAT
python3 setup.py --name test1 --node-version v0.10.0-rc.13

# 3. Start nodes
python3 start.py --name test1

# 4. Check status
python3 status.py --name test1
python3 status.py --name test1 --verbose

# 5. Run uploads
python3 upload.py --name test1 --count 5 --size 1M

# 6. Collect logs
python3 logs.py --name test1

# 7. Destroy VMs
python3 destroy.py --name test1
```

## State Directory

All testnet state is saved under `state/<testnet-name>/`:
- `vms.json` -- VM IDs, IPs, roles, providers
- `round-summary.json` -- timing, metrics from a full round

The `state/` directory is gitignored.

## NAT Simulation

NAT VMs get a port-restricted cone NAT simulation using Linux network
namespaces and iptables:

- **Namespace:** `nat-sim` with private IP `10.200.0.2`
- **Mapping:** Endpoint Independent (consistent external port)
- **Filtering:** Address + Port Dependent (only accept from exact IP:port we sent to)
- **DNS:** `8.8.8.8` in `/etc/netns/nat-sim/resolv.conf`

This is the hardest common NAT type for hole-punching.

## Architecture

```
run_round.py          -- orchestrates a complete round
  provision.py        -- create VMs via DO/HC APIs
  setup.py            -- install binaries, configure NAT
  start.py            -- start ant-node processes
  status.py           -- check process counts, log metrics
  upload.py           -- run upload tests
  logs.py             -- collect logs locally
  destroy.py          -- tear down all VMs

config.py             -- centralized settings
layouts.py            -- predefined VM layouts
ssh_utils.py          -- SSH/SCP via subprocess
```

## Testing with Custom (Patched) Binaries

To test a locally-built ant-node with patches applied:

```bash
# 1. Build patched binary (from ant-node repo)
cd ant-node
git checkout v0.10.0-rc.13
cargo zigbuild --release --target x86_64-unknown-linux-gnu --bin ant-node \
    --config 'patch."https://github.com/saorsa-labs/saorsa-core.git".saorsa-core.path="../saorsa-core"' \
    --config 'patch."https://github.com/saorsa-labs/saorsa-transport.git".saorsa-transport.path="../saorsa-transport"'

# 2. Run test with patched binary
cd saorsa-testnet/scripts/nat-testnet
python3 run_patched_test.py \
    --binary ../../ant-node/target/x86_64-unknown-linux-gnu/release/ant-node \
    --name my-patch-test \
    --uploads 3

# 3. Keep VMs alive for debugging
python3 run_patched_test.py \
    --binary /path/to/ant-node \
    --name debug-session \
    --skip-destroy
```

Requires: `cargo-zigbuild` + `zig` (`cargo install cargo-zigbuild && brew install zig`).

## Test Results

See `results/` directory for recorded test outcomes:

- `2026-04-08-round1-fix-test.md` — First successful NAT testnet upload (3/3 uploads, 44-45s each)

## Known Issues / Design Decisions

- SSH is always via Python `subprocess` (bash while-read loops break cargo aliases)
- Public IP detection uses `ip -4 addr show` (not `curl ifconfig.me` which can return IPv6)
- Node start uses a `start.sh` script written to each VM (nohup via SSH doesn't detach properly)
- VMs are set up sequentially (parallel SSH had reliability issues)
- Process counting uses `ps aux | grep '[a]nt-node'` (not `pgrep -c` which triggers cargo aliases)
