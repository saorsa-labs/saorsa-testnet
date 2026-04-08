# Round 4 Test Results — 2026-04-08

## All Patches Applied (cumulative)

1. saorsa-core PR #76: Clock skew tolerance 30s -> 300s
2. saorsa-transport PR #53: send_ack_timeout 500ms -> 5s
3. saorsa-transport PR #54: Accept loop dedup keeps newer connection
4. saorsa-transport PR #44: Reachability model (scope-aware peer-verified)
5. saorsa-transport PR #45: Relay fallback rotates through all candidates
6. saorsa-transport PR #46: Relay session reuse returns socket
7. saorsa-transport PR #47: Relay session periodic cleanup
8. saorsa-transport PR #48: Quality-aware coordinator selection

## Upload Results

| Upload | Duration | Result |
|--------|----------|--------|
| #1 (cold) | 279s | SUCCESS |
| #2 (warm) | 37s | SUCCESS |
| #3 (warm) | 41s | SUCCESS |

**3/3 uploads succeeded.**

## Full Round Comparison

| Round | Patches | Cold | Warm | Success |
|-------|---------|------|------|---------|
| Baseline | none | N/A | N/A | 0/3 |
| R1 | clock + timeout | 54s | 44-45s | 3/3 |
| R2 | + dedup | 160s | 40s | 3/3 |
| R3 | + reachability | 166s | 67-75s | 3/3 |
| R4 | + relay + coordinator | 279s | 37-41s | 3/3 |

**12/12 uploads succeeded across 4 rounds. No regressions.**

## Cost

- 4 rounds x 8 VMs x ~$0.02/hr x ~15min = ~$0.16 total
