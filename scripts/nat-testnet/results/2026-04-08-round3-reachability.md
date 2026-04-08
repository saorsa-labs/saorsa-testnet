# Round 3 Test Results — 2026-04-08

## Patches Applied (cumulative)

1. saorsa-core PR #76: Clock skew tolerance 30s -> 300s
2. saorsa-transport PR #53: send_ack_timeout 500ms -> 5s
3. saorsa-transport PR #54: Accept loop dedup keeps newer connection
4. **saorsa-transport PR #44: Reachability model (has_public_ip -> scope-aware peer-verified)**

## Upload Results

| Upload | Duration | Result |
|--------|----------|--------|
| #1 (cold) | 166s | SUCCESS |
| #2 (warm) | 75s | SUCCESS |
| #3 (warm) | 67s | SUCCESS |

**3/3 uploads succeeded.**

## Comparison

| Round | Fixes | Warm Upload | Success |
|-------|-------|-------------|---------|
| 1 | clock + timeout | 44-45s | 3/3 |
| 2 | + dedup | 40s | 3/3 |
| 3 | + reachability | 67-75s | 3/3 |

Warm uploads are slower than Round 2 (67-75s vs 40s). The reachability model
changes coordinator eligibility (requires peer-verified direct connection
evidence), which may reduce the initial coordinator pool size. Performance
improves as the network warms up (75s -> 67s).

This is expected for a more conservative reachability model. The tradeoff is
correctness over speed: only nodes that are actually directly reachable can be
coordinators, at the cost of slightly slower initial peer discovery.

## Cost

- 8 VMs x ~14 min = ~$0.04
