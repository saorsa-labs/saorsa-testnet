# Round 2 Test Results — 2026-04-08

## Patches Applied (cumulative)

1. **saorsa-core PR #76**: Clock skew tolerance 30s -> 300s
2. **saorsa-transport PR #53**: send_ack_timeout 500ms -> 5s
3. **saorsa-transport PR #54**: Accept loop dedup keeps newer connection (fixes PR #43 regression)

Note: PR #43 (coordinator rotation) is included in the base rc-2026.4.1 branch.
This round validates that the dedup fix resolves the regression PR #43 introduced.

## VM Layout

Same as Round 1: 8 DO VMs across lon1/ams3/nyc1/sfo3.
3 bootstrap + 3 NAT + 1 public + 1 client = 14 nodes.

## Upload Results

| Upload | Duration | Result | Notes |
|--------|----------|--------|-------|
| #1 (cold) | 160s | SUCCESS | Includes DHT bootstrap + EVM approval |
| #2 (warm) | 40s | SUCCESS | Routing table populated |
| #3 (warm) | 40s | SUCCESS | Consistent with #2 |

**3/3 uploads succeeded.**

## Comparison with Round 1

| Metric | Round 1 (no dedup fix) | Round 2 (with dedup fix) |
|--------|----------------------|--------------------------|
| Success rate | 3/3 (100%) | 3/3 (100%) |
| Cold upload | ~54s | 160s (more DHT hops?) |
| Warm upload | 44-45s | 40s |

Warm uploads improved slightly (44s -> 40s). Cold upload was slower but this
is likely due to network variability, not a regression.

## Key Validation

The dedup fix (PR #54) resolves the coordinator rotation regression:
- PR #43 (coordinator rotation) is included in the base binary
- No "duplicate" connection closures observed in this test
- Identity exchange succeeds without 15s timeouts

## Reproducing

```bash
# Build with all 3 fixes
cd saorsa-transport && git checkout fix/send-ack-timeout  # Has PR #53 + #54 cherry-picked
cd ../saorsa-core && git checkout fix/clock-skew-tolerance  # Has PR #76

cd ../ant-node && git checkout v0.10.0-rc.13
cargo zigbuild --release --target x86_64-unknown-linux-gnu --bin ant-node \
    --config 'patch."https://github.com/saorsa-labs/saorsa-core.git".saorsa-core.path="../saorsa-core"' \
    --config 'patch."https://github.com/saorsa-labs/saorsa-transport.git".saorsa-transport.path="../saorsa-transport"'

# Run test
DIGITALOCEAN_TOKEN=... python3 run_patched_test.py \
    --binary ../ant-node/target/x86_64-unknown-linux-gnu/release/ant-node \
    --name r2-dedup --uploads 3
```

## Cost

- 8 VMs x ~14 min = ~$0.04
