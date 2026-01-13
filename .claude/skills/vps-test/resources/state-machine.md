# VPS Test State Machine

## State Diagram

```
                              ┌──────────────────────────────────────┐
                              │                                      │
                              ▼                                      │
┌──────┐     ┌───────┐     ┌───────┐     ┌────────┐     ┌──────┐   │
│ IDLE │ ──▶ │ SETUP │ ──▶ │ BUILD │ ──▶ │ DEPLOY │ ──▶ │ TEST │   │
└──────┘     └───────┘     └───┬───┘     └────────┘     └──┬───┘   │
                               │                            │       │
                               │ (build failure)            │       │
                               │                            │       │
                               ▼                            │       │
                           ┌───────┐                        │       │
                           │  FIX  │ ◀──────────────────────┘       │
                           └───┬───┘   (test failure)               │
                               │                                    │
                               │ (commit)                           │
                               │                                    │
                               └────────────────────────────────────┘


                         TEST (success, first time)
                                    │
                                    ▼
                            ┌─────────────┐
                            │  WAIT_1HR   │
                            └──────┬──────┘
                                   │
                                   ▼
                            ┌─────────────┐
                            │    TEST     │ ──────┐
                            └──────┬──────┘       │ (failure)
                                   │              │
                                   │ (success)    │
                                   ▼              │
                            ┌─────────────┐       │
                            │  WAIT_6HR   │       │
                            └──────┬──────┘       │
                                   │              │
                                   ▼              │
                            ┌─────────────┐       │
                            │    TEST     │ ──────┤
                            └──────┬──────┘       │
                                   │              │
                                   │ (success)    │
                                   ▼              ▼
                            ┌─────────────┐  ┌─────────┐
                            │  COMPLETE   │  │ RESTART │ ──▶ (back to SETUP)
                            └─────────────┘  └─────────┘
```

## States

### IDLE
- **Entry:** Initial state, no test loop running
- **Exit:** User invokes `/vps-test start <project>`
- **Actions:** None
- **Next:** SETUP

### SETUP
- **Entry:** Test loop initiated
- **Exit:** All nodes verified reachable
- **Actions:**
  1. Load and validate project config
  2. SSH connectivity check to all nodes
  3. Check disk space on all nodes (warn if < 1GB)
  4. Clear old logs if disk space critical
  5. Create/update state file
- **Next:** BUILD
- **Error:** If < 50% nodes reachable → STOPPED with error

### BUILD
- **Entry:** Setup complete or fix committed
- **Exit:** Binary successfully built
- **Actions:**
  1. Run build command from config
  2. Parse build output for errors/warnings
  3. Record build duration
- **Next:** DEPLOY (on success), FIX (on failure)
- **Max attempts:** 3 (then STOPPED with error)

### DEPLOY
- **Entry:** Build successful
- **Exit:** All nodes running new binary
- **Actions:**
  1. For each node:
     - Upload binary via SCP
     - Restart systemd service
     - Verify service running
  2. Wait for registry registration (30s timeout)
  3. Verify all nodes appear in registry
- **Next:** TEST
- **Error:** If any node fails → log and continue (unless < 50%)

### TEST
- **Entry:** Deployment complete or wait period elapsed
- **Exit:** All tests run with results recorded
- **Actions:**
  1. Run each enabled test suite
  2. Collect results from all nodes
  3. Calculate success rates
  4. Compare against criteria
- **Next:**
  - If any test fails → FIX
  - If first success → WAIT_1HR
  - If success after 1hr → WAIT_6HR
  - If success after 6hr → COMPLETE

### FIX
- **Entry:** Build or test failure
- **Exit:** Fix committed
- **Actions:**
  1. Collect logs from all nodes
  2. Analyze error patterns
  3. Identify root cause
  4. Apply code fix
  5. Run local tests (cargo test)
  6. Commit changes
  7. Increment fix_attempts counter
- **Next:** BUILD
- **Max attempts:** Unlimited (full autonomy)

### WAIT_1HR
- **Entry:** First successful test
- **Exit:** 1 hour elapsed
- **Actions:**
  1. Save timestamp in state file
  2. Update dashboard status
  3. Send voice notification
  4. Sleep/poll (check every 5 minutes)
- **Next:** TEST

### WAIT_6HR
- **Entry:** Successful test after 1hr wait
- **Exit:** 6 hours elapsed
- **Actions:**
  1. Save timestamp in state file
  2. Update dashboard status
  3. Send voice notification
  4. Sleep/poll (check every 15 minutes)
- **Next:** TEST

### COMPLETE
- **Entry:** Successful test after 6hr wait
- **Exit:** Terminal state
- **Actions:**
  1. Update state file with completion time
  2. Update dashboard (green status)
  3. Send voice notification: "VPS test complete for <project>"
  4. Log final statistics
- **Next:** None (terminal)

### STOPPED
- **Entry:** User stop or unrecoverable error
- **Exit:** Terminal state
- **Actions:**
  1. Update state file
  2. Kill any running processes
  3. Log stop reason
- **Next:** None (terminal)

## State File Format

```json
{
  "project": "ant-quic",
  "state": "WAIT_1HR",
  "phase": "soak",

  "timestamps": {
    "started_at": "2025-01-10T10:00:00Z",
    "last_state_change": "2025-01-10T14:30:00Z",
    "last_test_at": "2025-01-10T14:30:00Z",
    "wait_until": "2025-01-10T15:30:00Z"
  },

  "counters": {
    "fix_attempts": 3,
    "build_count": 4,
    "deploy_count": 4,
    "test_count": 5
  },

  "commits": [
    {"sha": "abc123", "message": "fix: timeout", "time": "2025-01-10T10:15:00Z"},
    {"sha": "def456", "message": "fix: buffer", "time": "2025-01-10T11:30:00Z"}
  ],

  "test_results": {
    "last_run": "2025-01-10T14:30:00Z",
    "passed": true,
    "suites": {
      "connectivity": {
        "direct": {"attempts": 72, "success": 71, "rate": 98.6},
        "nat_traversed": {"attempts": 72, "success": 63, "rate": 87.5},
        "relay": {"attempts": 72, "success": 72, "rate": 100.0}
      },
      "throughput": {
        "avg_mbps": 45.3,
        "min_mbps": 12.1,
        "max_mbps": 89.7
      }
    }
  },

  "nodes": {
    "saorsa-2": {"status": "online", "last_check": "2025-01-10T14:30:00Z"},
    "saorsa-3": {"status": "online", "last_check": "2025-01-10T14:30:00Z"},
    "saorsa-4": {"status": "degraded", "last_check": "2025-01-10T14:30:00Z", "error": "high latency"}
  },

  "logs": [
    {"time": "2025-01-10T10:00:00Z", "state": "SETUP", "message": "Loading config"},
    {"time": "2025-01-10T10:00:05Z", "state": "SETUP", "message": "Verified 9/10 nodes"},
    {"time": "2025-01-10T10:00:30Z", "state": "BUILD", "message": "Starting build"},
    {"time": "2025-01-10T10:02:15Z", "state": "BUILD", "message": "Build complete (1m 45s)"}
  ]
}
```

## Transition Rules

### Success Path
```
IDLE → SETUP → BUILD → DEPLOY → TEST (pass) → WAIT_1HR → TEST (pass) → WAIT_6HR → TEST (pass) → COMPLETE
```

### Typical Fix Path
```
IDLE → SETUP → BUILD → DEPLOY → TEST (fail) → FIX → BUILD → DEPLOY → TEST (pass) → WAIT_1HR → ...
```

### Build Failure Path
```
IDLE → SETUP → BUILD (fail) → FIX → BUILD → DEPLOY → ...
```

### Soak Test Failure Path
```
... → WAIT_1HR → TEST (pass) → WAIT_6HR → TEST (fail) → RESTART → SETUP → BUILD → ...
```

## Checkpoint and Resume

The state machine supports resumption after Claude Code restart:

1. **On start:** Check for existing state file
2. **If found:** Resume from saved state
   - WAIT_1HR/WAIT_6HR: Check if wait period elapsed
   - BUILD/DEPLOY/TEST: Restart from that state
   - FIX: Continue fix analysis
3. **State file updated:** After every state transition

## Concurrency

Multiple projects can run simultaneously:
- Each project has its own state file
- All projects share the VPS nodes (bootstrap separation)
- State machine checks do not interfere

## Monitoring Integration

State changes emit events to:
1. State file (JSON)
2. Dashboard API (POST to registry)
3. Voice notification (via notify.py hook)
4. Logs (timestamped entries)
