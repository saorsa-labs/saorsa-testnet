# Round 1 Fix Test Results — 2026-04-08

## Configuration

- **Binary**: ant-node v0.10.0-rc.13 with two patches applied:
  - saorsa-core PR #76: clock skew tolerance 30s -> 300s
  - saorsa-transport PR #53: send_ack_timeout 500ms -> 5s
- **Client**: ant-cli v0.1.2-rc.14 (unpatched release binary)
- **Provider**: DigitalOcean only (8 VMs, s-2vcpu-2gb)
- **Regions**: lon1 (London), ams3 (Amsterdam), nyc1 (NYC), sfo3 (San Francisco)
- **Network mode**: testnet
- **EVM**: arbitrum-sepolia

## VM Layout

| VM | Region | Role | IP |
|----|--------|------|----|
| anselme-r1fix2-lon1-0 | lon1 | bootstrap | 159.65.56.109 |
| anselme-r1fix2-ams3-1 | ams3 | bootstrap | 178.62.244.41 |
| anselme-r1fix2-nyc1-2 | nyc1 | bootstrap | 192.34.62.59 |
| anselme-r1fix2-sfo3-3 | sfo3 | NAT (port-restricted) | 209.38.69.190 |
| anselme-r1fix2-lon1-4 | lon1 | NAT (port-restricted) | 161.35.161.137 |
| anselme-r1fix2-ams3-5 | ams3 | NAT (port-restricted) | 159.223.0.156 |
| anselme-r1fix2-nyc1-6 | nyc1 | public | 159.89.231.255 |
| anselme-r1fix2-sfo3-7 | sfo3 | client | 165.232.155.53 |

- **Nodes per VM**: 2 (except client)
- **Total nodes**: 14
- **NAT ratio**: 3/7 = 43% behind port-restricted NAT

## NAT Simulation

Port-restricted NAT via Linux network namespaces + iptables:
- Network namespace `nat-sim` with veth pair (10.200.0.0/24)
- SNAT to VM's public IP
- FORWARD policy: ACCEPT outbound, only ESTABLISHED inbound, DROP otherwise
- DNS: 8.8.8.8 in namespace

## Upload Results

| Upload | Duration | Result | Notes |
|--------|----------|--------|-------|
| #1 | ~54s | SUCCESS | Cold start (first DHT bootstrap) |
| #2 | 45s | SUCCESS | Warm (routing table populated) |
| #3 | 44s | SUCCESS | Warm, chunk dedup detected |

- **File size**: 10 MB
- **Chunks**: 3 (self-encryption)
- **Payment**: EVM single transaction on Arbitrum Sepolia
- **Upload client**: ran from client VM (sfo3), not local Mac

## Key Observations

1. **All 3 uploads succeeded** — 100% success rate
2. **Warm uploads ~44-45s** (after initial DHT bootstrap)
3. **NAT hole-punching worked** — NAT nodes participated in storage
4. **Cross-region connectivity** — nodes in London, Amsterdam, NYC, San Francisco all interconnected
5. **Chunk dedup detected** on upload #3 ("Chunk already exists") confirming data persisted

## Without Patches (Baseline Comparison)

With the same setup but unpatched rc.13 binary:
- **0/3 uploads succeeded**
- Identity exchange timeout (15s) on all bootstrap peers
- "Rejecting future-dated message" errors (31s clock skew between Mac client and VPS nodes)
- "insufficient peers: Found 0 peers, need 7"

## Patches Applied

### 1. Clock Skew Tolerance (saorsa-core PR #76)

```
- const MAX_FUTURE_SECS: u64 = 30;
+ const MAX_FUTURE_SECS: u64 = 300;
```

A decentralized network cannot assume participants have accurate clocks.
Consumer devices commonly drift by minutes. The 30s window was causing
message rejection for any device with a slightly slow clock.

### 2. Send ACK Timeout (saorsa-transport PR #53)

```
- const DEFAULT_SEND_ACK_TIMEOUT: Duration = Duration::from_millis(500);
+ const DEFAULT_SEND_ACK_TIMEOUT: Duration = Duration::from_secs(5);
```

500ms was too tight for cross-region hole-punched connections where the
identity announce needs 3+ RTTs. The fire-and-forget send silently failed,
causing 15s identity exchange timeouts.

## Reproducing This Test

```bash
# 1. Build patched ant-node binary
cd ant-node
git checkout v0.10.0-rc.13
cargo zigbuild --release --target x86_64-unknown-linux-gnu --bin ant-node \
    --config 'patch."https://github.com/saorsa-labs/saorsa-core.git".saorsa-core.path="../saorsa-core"' \
    --config 'patch."https://github.com/saorsa-labs/saorsa-transport.git".saorsa-transport.path="../saorsa-transport"'

# Make sure saorsa-core is on fix/clock-skew-tolerance branch
# Make sure saorsa-transport is on fix/send-ack-timeout branch

# 2. Run the testnet (uses the toolkit)
cd saorsa-testnet/scripts/nat-testnet
python3 run_round.py \
    --name r1-fix-test \
    --node-version v0.10.0-rc.13 \
    --layout 20node-quick \
    --uploads 3 \
    --upload-size 10M

# Or manually with a custom binary:
# See the full Python script in this repo's scripts/nat-testnet/ directory
```

## Cost

- 8 VMs x ~$0.02/hr = ~$0.16/hr
- Total runtime: ~15 minutes
- **Total cost: ~$0.04**
