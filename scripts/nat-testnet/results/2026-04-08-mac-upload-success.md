# First Successful Mac Upload to NAT Testnet — 2026-04-08

## THE RESULT

3/3 uploads from a macOS client to a NAT-protected network across 4 regions.

| Upload | Duration | Result |
|--------|----------|--------|
| #1 (cold) | 61s | SUCCESS |
| #2 (warm) | 30s | SUCCESS |
| #3 (warm) | 33s | SUCCESS |

Client: macOS ARM, 31 seconds of clock skew against VPS nodes.
File: 10MB, self-encrypted into 3 chunks, stored on 4 peers each.
Payment: EVM batch on Arbitrum Sepolia (3 transactions).

## Testnet

- 7 VMs on DigitalOcean: lon1, ams3, nyc1, sfo3
- 14 node processes: 6 bootstrap + 6 NAT + 2 public
- NAT simulation: port-restricted via Linux namespace + iptables
- Node binary: ant-node v0.10.0-rc.13 with all 8 transport + 1 core patches
- Client binary: ant-cli v0.1.2-rc.14 with clock skew patch

## Patches required

### saorsa-core (1 change)
- `MAX_FUTURE_SECS`: 30 -> 300 (clock skew tolerance)

### saorsa-transport (8 changes)
1. `send_ack_timeout`: 500ms -> 5s
2. Accept loop dedup: keep newer connection, close older
3. Reachability model: `has_public_ip` -> scope-aware peer-verified
4. Review feedback: double-count fix, relay capability check
5. Relay: rotate through all candidates
6. Relay: session reuse returns socket
7. Relay: periodic session cleanup
8. Coordinator: RTT-weighted quality selection

## How to reproduce

```bash
# 1. Prepare repos
cd saorsa-core && git checkout fix/clock-skew-tolerance
cd ../saorsa-transport && git checkout round4-combined

# 2. Build node (Linux x64 for VMs)
cd ../ant-node && git checkout v0.10.0-rc.13
cargo zigbuild --release --target x86_64-unknown-linux-gnu --bin ant-node \
    --config 'patch."https://github.com/saorsa-labs/saorsa-core.git".saorsa-core.path="../saorsa-core"' \
    --config 'patch."https://github.com/saorsa-labs/saorsa-transport.git".saorsa-transport.path="../saorsa-transport"'

# 3. Build client (macOS for local upload)
cd ../ant-client && git checkout ant-cli-v0.1.2-rc.14
cargo build --release --bin ant \
    --config 'patch."https://github.com/saorsa-labs/saorsa-core.git".saorsa-core.path="../saorsa-core"' \
    --config 'patch."https://github.com/saorsa-labs/saorsa-transport.git".saorsa-transport.path="../saorsa-transport"'

# 4. Run testnet
DIGITALOCEAN_TOKEN=... python3 scripts/nat-testnet/run_patched_test.py \
    --binary ../ant-node/target/x86_64-unknown-linux-gnu/release/ant-node \
    --name mac-test --skip-destroy

# 5. Upload from Mac (while testnet is running)
SECRET_KEY=... target/release/ant \
    -b <bootstrap-ip-1>:10000 -b <bootstrap-ip-2>:10000 \
    --evm-network arbitrum-sepolia file upload /path/to/file
```
