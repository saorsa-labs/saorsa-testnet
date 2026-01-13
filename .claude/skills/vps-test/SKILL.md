# VPS Test Skill

Autonomous VPS deployment, testing, and fixing cycles for P2P network projects.

## Invocation

```
/vps-test [command] [parameters]
```

### Examples

```bash
# Start with interview (no parameters)
/vps-test start

# Start with project specified (interview will confirm/clarify)
/vps-test start ant-quic

# Start with project and test focus (interview will confirm/clarify)
/vps-test start ant-quic connectivity

# Start with full parameters (interview will confirm/clarify)
/vps-test start ant-quic nat-traversal threshold logs

# Check status of all loops
/vps-test status

# Stop a running test
/vps-test stop ant-quic

# View logs
/vps-test logs ant-quic

# Diagnose a node
/vps-test diagnose saorsa-4

# List all nodes
/vps-test nodes
```

## Commands

| Command | Description |
|---------|-------------|
| `/vps-test start [project] [focus] [criteria] [proof]` | Start test session with optional pre-filled parameters |
| `/vps-test status` | Show status of all active test loops |
| `/vps-test stop <project>` | Gracefully stop a running test loop |
| `/vps-test logs <project>` | Show recent activity for a project |
| `/vps-test diagnose <node>` | SSH debug a specific VPS node |
| `/vps-test nodes` | List all available VPS nodes and their status |

### Parameter Values

**project:** `ant-quic` | `saorsa-gossip` | `communitas-mcp` | `saorsa-node` | `saorsa-core`

**focus:** `connectivity` | `nat-traversal` | `gossip` | `throughput` | `full-suite`

**criteria:** `all-pass` | `threshold` | `improvement` | `custom`

**proof:** `logs` | `metrics` | `manual-verify` | `all-evidence`

---

## CRITICAL: Interview Process (MANDATORY)

**Every `/vps-test start` MUST begin with an interview using AskUserQuestion.**

Even if parameters are passed, the interview confirms and clarifies:
1. What we are testing (pre-filled if parameter provided)
2. What result we want (pre-filled if parameter provided)
3. What the proof of success will be (pre-filled if parameter provided)

### Interview Behavior

- **No parameters:** All questions asked
- **Some parameters:** Pre-fill provided values, ask remaining questions, confirm all
- **All parameters:** Show summary, ask for confirmation before proceeding

### Interview Questions

When `/vps-test start` is invoked, use `AskUserQuestion` with these questions (skip if pre-filled, but always confirm):

```
Question 1: "Which project are you testing?"
Header: "Project"
Options:
  - ant-quic: "QUIC transport with NAT traversal"
  - saorsa-gossip: "Gossip protocol layer"
  - communitas-mcp: "MCP server for Communitas"
  - saorsa-node: "Full Saorsa network node"
  - saorsa-core: "Core library (integration tests)"

Question 2: "What specific functionality are you testing?"
Header: "Test focus"
Options:
  - connectivity: "Connection matrix (direct, NAT, relay)"
  - nat-traversal: "NAT hole-punching across different NAT types"
  - gossip: "Message broadcast and CRDT sync"
  - throughput: "Data transfer performance"
  - full-suite: "All tests in sequence"

Question 3: "What result indicates success?"
Header: "Success criteria"
Options:
  - all-pass: "100% of tests must pass"
  - threshold: "Meet configured success thresholds (e.g., 85% NAT)"
  - improvement: "Better than previous run"
  - custom: "I'll specify custom criteria"

Question 4: "How should success be proven/verified?"
Header: "Proof method"
Options:
  - logs: "Gather and analyze logs from all VPS nodes"
  - metrics: "Check dashboard metrics and Prometheus data"
  - manual-verify: "I'll manually verify after tests complete"
  - all-evidence: "Collect logs, metrics, and connection reports"
```

### Interview Output

After the interview (or when parameters are provided), summarize the test plan:

```
Test Plan Summary
=================
Project: ant-quic (from parameter)
Focus: NAT traversal testing (from parameter)
Success: Meet configured thresholds (confirmed via interview)
Proof: Collect logs from all VPS nodes (confirmed via interview)

Nodes to use:
  Standard: saorsa-2, saorsa-3, saorsa-7, saorsa-8, saorsa-9
  NAT Emulation:
    - saorsa-4 (Full Cone)
    - saorsa-5 (Address Restricted)
    - saorsa-6 (Port Restricted)
    - saorsa-10 (Symmetric)

Binary source: Local build (cargo zig build)

Ready to proceed?
```

Then use AskUserQuestion for final confirmation:
```
Question: "Confirm test plan and proceed?"
Header: "Confirm"
Options:
  - proceed: "Yes, start the test loop"
  - modify: "No, I want to change something"
  - cancel: "Cancel and exit"
```

---

## VPS Infrastructure (EXACT SPECIFICATION)

**Last verified: 2025-01-10 via SSH**

### Available Nodes

| Node | Hostname | IP | Provider | Region | Role | NAT Emulation |
|------|----------|-----|----------|--------|------|---------------|
| saorsa-1 | saorsa-1.saorsalabs.com | 77.42.75.115 | Hetzner | Helsinki | **Dashboard & Registry** | None (monitoring only) |
| saorsa-2 | saorsa-2.saorsalabs.com | 142.93.199.50 | DigitalOcean | NYC1 | **Bootstrap Node** | None |
| saorsa-3 | saorsa-3.saorsalabs.com | 147.182.234.192 | DigitalOcean | SFO3 | **Bootstrap Node** | None |
| saorsa-4 | saorsa-4.saorsalabs.com | 206.189.7.117 | DigitalOcean | AMS3 | **NAT: Full Cone** | Docker: `nat-fullcone`, `node-fullcone` |
| saorsa-5 | saorsa-5.saorsalabs.com | 144.126.230.161 | DigitalOcean | LON1 | **NAT: Address Restricted** | Docker: `nat-restricted`, `node-restricted` |
| saorsa-6 | saorsa-6.saorsalabs.com | 65.21.157.229 | Hetzner | Helsinki | **NAT: Port Restricted** | Docker: `nat-portrestricted`, `node-portrestricted` |
| saorsa-7 | saorsa-7.saorsalabs.com | 116.203.101.172 | Hetzner | Nuremberg | Test Node | None |
| saorsa-8 | saorsa-8.saorsalabs.com | 149.28.156.231 | Vultr | Singapore | Test Node | None (high latency) |
| saorsa-9 | saorsa-9.saorsalabs.com | 45.77.176.184 | Vultr | Tokyo | Test Node | None (high latency) |
| saorsa-10 | saorsa-10.saorsalabs.com | 77.42.39.239 | Hetzner | Falkenstein | **NAT: Symmetric** | Netns: `natbox` service |

### NAT Emulation Details (VERIFIED)

**Nodes with Docker NAT emulation:**

| Node | NAT Type | Difficulty | Docker Containers |
|------|----------|------------|-------------------|
| saorsa-4 | Full Cone | Easy (95% success) | `nat-fullcone`, `node-fullcone` |
| saorsa-5 | Address Restricted | Medium (85% success) | `nat-restricted`, `node-restricted` |
| saorsa-6 | Port Restricted | Medium (85% success) | `nat-portrestricted`, `node-portrestricted` |

**Node with Network Namespace NAT:**

| Node | NAT Type | Difficulty | Service |
|------|----------|------------|---------|
| saorsa-10 | Symmetric | Hard (70% success) | `ant-quic-test-nat.service`, namespace `natbox` |

**Note:** If saorsa-10.saorsalabs.com doesn't resolve, use direct IP 77.42.39.239 (DNS recently added)

### Node Details

**saorsa-1 (Dashboard & Registry)**
- Role: Central coordination, NOT for P2P testing
- Docker: Yes (for monitoring: grafana, prometheus, promtail, loki)
- Services: Registry API, Prometheus, Grafana
- URLs:
  - Registry: `https://saorsa-1.saorsalabs.com/api/peers`
  - Health: `https://saorsa-1.saorsalabs.com/health`
  - Dashboard: `https://saorsa-1.saorsalabs.com`

**saorsa-2, saorsa-3 (Bootstrap Nodes)**
- Role: Initial peer discovery, always-on
- Docker: No
- Ports: 9000 (ant-quic), 9500 (saorsa-gossip), 10000 (saorsa-node), 11000 (communitas)
- These should be stable and rarely redeployed during tests

**saorsa-4 (Full Cone NAT)**
- Docker: Yes, active
- Containers: `nat-fullcone`, `node-fullcone`
- NAT behavior: Any external host can send packets to internal host once it sends outbound
- Expected success rate: ~95%

**saorsa-5 (Address Restricted NAT)**
- Docker: Yes, active
- Containers: `nat-restricted`, `node-restricted`
- NAT behavior: External host must have received packet from internal host first
- Expected success rate: ~85%

**saorsa-6 (Port Restricted NAT)**
- Docker: Yes, active
- Containers: `nat-portrestricted`, `node-portrestricted`
- NAT behavior: External host+port must have received packet from internal host first
- Expected success rate: ~85%

**saorsa-7, saorsa-8, saorsa-9 (Standard Test Nodes)**
- Docker: No
- Role: General purpose testing, direct public IP
- saorsa-8/9 have high latency to EU/US (Singapore/Tokyo)

**saorsa-10 (Symmetric NAT - HARDEST)**
- Docker: No
- Network namespace: `natbox`
- Service: `ant-quic-test-nat.service` (running)
- NAT behavior: Different external port for each destination (hardest to traverse)
- Expected success rate: ~70%
- Fallback IP if DNS issues: `ssh root@77.42.39.239`

### SSH Access

```bash
# Standard access (all nodes)
ssh root@saorsa-N.saorsalabs.com

# Direct IP fallback
ssh root@<IP>

# SSH keys configured:
# - DigitalOcean: mac (48810465), dirvine (2064413)
# - Hetzner: davidirvine@MacBook-Pro.localdomain (104686182)
```

### Port Allocation

| Port | Service | Protocol |
|------|---------|----------|
| 22 | SSH | TCP |
| 80 | HTTP | TCP |
| 443 | HTTPS | TCP |
| 9000 | ant-quic-test | UDP |
| 9500 | saorsa-gossip | UDP |
| 10000 | saorsa-node | UDP |
| 11000 | communitas-mcp | UDP |

**Note:** Services use dynamic port allocation (bind to 0) and register with registry.

---

## CRITICAL: Build Policy (NO REMOTE BUILDS)

**NEVER build binaries on VPS machines.**

### Binary Sources (Choose One)

**Option 1: Local Build with cargo-zigbuild (Recommended)**
```bash
# On local macOS machine
cargo zig build --release --target x86_64-unknown-linux-gnu

# Binary location
target/x86_64-unknown-linux-gnu/release/<binary-name>
```

**Option 2: GitHub Release**
```bash
# Download from GitHub releases
gh release download v0.14.x --pattern '*linux*' --dir /tmp/

# Or use existing deploy script
./scripts/deploy-test-network.sh deploy --version 0.14.199
```

### Local Build Requirements

```bash
# Install cargo-zigbuild (one-time)
cargo install cargo-zigbuild

# Install zig (one-time)
brew install zig
```

### Deployment Flow

```
1. BUILD (local macOS)
   cargo zig build --release --target x86_64-unknown-linux-gnu

2. UPLOAD (to each node)
   scp target/x86_64-unknown-linux-gnu/release/<binary> root@saorsa-N:/opt/<project>/

3. RESTART (on each node)
   ssh root@saorsa-N 'systemctl restart <service>'

4. VERIFY (on each node)
   ssh root@saorsa-N 'systemctl status <service>'
```

---

## Cross-Compilation Strategy

When building from macOS (especially ARM/Apple Silicon), cross-compilation for Linux x86_64
often fails due to C dependencies (ring, openssl). When cargo-zigbuild fails:

1. Push code to GitHub
2. SSH to saorsa-1 (77.42.75.115) and build natively:
   ```bash
   ssh root@77.42.75.115 'source ~/.cargo/env && cd ~/saorsa-testnet && git pull && cargo build --release'
   ```
3. Download binary and distribute to other VPS nodes

**Note:** saorsa-1 has Rust toolchain installed and the saorsa-testnet repo cloned.

---

## Binary Distribution

After building on saorsa-1 (when cross-compilation fails):

1. **Copy to local:**
   ```bash
   scp root@77.42.75.115:/root/saorsa-testnet/target/release/saorsa-quic-test /tmp/claude/ant-quic-test-linux
   ```

2. **Stop running processes on all VPS (required before overwriting):**
   ```bash
   for ip in 138.197.29.195 162.243.167.201 159.65.221.230 67.205.158.158 161.35.231.80 178.62.192.11 159.65.90.128; do
       ssh root@$ip "pkill -9 ant-quic" &
   done
   wait
   ```

3. **Distribute to all VPS (parallel):**
   ```bash
   for ip in 138.197.29.195 162.243.167.201 159.65.221.230 67.205.158.158 161.35.231.80 178.62.192.11 159.65.90.128; do
       scp /tmp/claude/ant-quic-test-linux root@$ip:/usr/local/bin/ant-quic-test &
   done
   wait
   ```

**Important:** Binary must be stopped before overwriting - "dest open: Failure" errors occur if the binary is running.

---

## Test Loop State Machine

```
INTERVIEW → SETUP → BUILD_LOCAL → DEPLOY → TEST
                                             ↓ (failure)
                         FIX ← ─────────────┘
                          ↓ (commit)
                       BUILD_LOCAL (restart)

                    TEST (success)
                         ↓
                    WAIT_1HR → TEST → WAIT_6HR → TEST → COMPLETE
                                 ↓ (failure)
                              RESTART (full loop)
```

### State Descriptions

**INTERVIEW** (NEW - MANDATORY)
- Use AskUserQuestion to gather test parameters
- Confirm test plan with user
- Only proceed after user confirms

**SETUP**
- Load project config from `.claude/vps-test.yaml`
- Verify SSH connectivity to all nodes
- Check disk space, clear old logs if needed

**BUILD_LOCAL** (renamed from BUILD)
- Build on LOCAL machine only
- Command: `cargo zig build --release --target x86_64-unknown-linux-gnu`
- If failure → FIX
- If success → DEPLOY

**DEPLOY**
- Upload binary to each node via SCP
- Restart systemd services
- Wait for registry registration
- Verify services running

**TEST**
- Run test suites based on interview selection
- Collect results and logs from all nodes
- Compare against success criteria from interview

**FIX**
- Collect logs from all nodes (proof gathering)
- Analyze error patterns
- Apply minimal code fix
- Run local tests
- Commit with descriptive message
- → BUILD_LOCAL

**WAIT_1HR / WAIT_6HR**
- Stability checkpoints
- Re-run same tests
- Any regression → RESTART

**COMPLETE**
- Gather final proof (logs, metrics)
- Voice notification
- Update dashboard

---

## Proof of Success Gathering

When tests pass, collect evidence based on interview selection:

### Log Collection
```bash
# Collect logs from all nodes
for node in saorsa-{2..10}; do
  ssh root@$node.saorsalabs.com 'journalctl -u <service> -n 500 --no-pager' \
    > ~/.claude/vps-test-state/<project>/logs/$node.log
done
```

### Metrics Collection
```bash
# Query Prometheus for test metrics
curl -s "https://saorsa-1.saorsalabs.com/api/metrics" \
  > ~/.claude/vps-test-state/<project>/metrics.json
```

### Connection Report
```bash
# Generate connectivity matrix report
# (from test binary output)
```

### Proof Summary File
After successful completion, create:
```
~/.claude/vps-test-state/<project>/proof-<timestamp>.md

# Test Proof: ant-quic
## Date: 2025-01-10 14:30:00
## Duration: 6h 30m
## Result: COMPLETE

### Success Criteria Met
- Direct connections: 99.2% (target: 99%)
- NAT traversal: 87.5% (target: 85%)
- Relay: 100% (target: 100%)

### Evidence
- Logs: ~/.claude/vps-test-state/ant-quic/logs/
- Metrics: ~/.claude/vps-test-state/ant-quic/metrics.json
- Fix commits: abc123, def456

### Nodes Tested
- saorsa-2 through saorsa-10
- NAT simulation on saorsa-10
```

---

## Supported Projects

**All test binaries are built from saorsa-testnet workspace:**
`~/Desktop/Devel/projects/saorsa-testnet`

| Test Module | Binary | Tests | Port |
|-------------|--------|-------|------|
| quic-test | saorsa-quic-test | QUIC transport, NAT traversal, connectivity matrix | 9000 |
| gossip-test | saorsa-gossip-test | Gossip broadcast, CRDT sync, membership (via quic-test) | 9500 |
| core-test | saorsa-core-test | Core library integration tests | 9000 |
| node-test | saorsa-node-test | Full node health, sync, storage, retrieval | 10000 |
| communitas-test | communitas-test | MCP server, headless daemon testing | 11000 |

### Build All Test Binaries

```bash
# From saorsa-testnet directory
cargo zig build --release --target x86_64-unknown-linux-gnu

# Individual module
cargo zig build --release -p saorsa-quic-test --target x86_64-unknown-linux-gnu
```

---

## Command Implementation

### /vps-test start

**MUST begin with interview. Do not skip.**

```
1. INTERVIEW (AskUserQuestion)
   - Which project?
   - What functionality?
   - Success criteria?
   - Proof method?

2. CONFIRM
   - Display test plan summary
   - Wait for user confirmation

3. SETUP
   - Load project config
   - Verify node connectivity

4. BUILD_LOCAL
   - cargo zig build (on LOCAL machine)
   - NEVER build on VPS

5. DEPLOY
   - SCP binary to nodes
   - Restart services

6. TEST LOOP
   - Run selected tests
   - Fix failures
   - Wait 1hr, 6hr
   - Collect proof on success
```

### /vps-test status

Show all active test loops with current state.

### /vps-test stop <project>

Gracefully stop a running loop, save state.

### /vps-test logs <project>

Show recent activity and collected logs.

### /vps-test diagnose <node>

SSH into node and check:
- Service status
- Recent logs
- Disk space
- Memory usage
- Network connections

### /vps-test nodes

List all VPS nodes with status:
```
VPS Nodes Status (verified 2025-01-10)
======================================

saorsa-1  (Dashboard)       77.42.75.115      Hetzner/Helsinki    ONLINE  [Prometheus, Grafana]
saorsa-2  (Bootstrap)       142.93.199.50     DO/NYC1             ONLINE
saorsa-3  (Bootstrap)       147.182.234.192   DO/SFO3             ONLINE
saorsa-4  (NAT: Full Cone)  206.189.7.117     DO/AMS3             ONLINE  [Docker: nat-fullcone]
saorsa-5  (NAT: Addr Restr) 144.126.230.161   DO/LON1             ONLINE  [Docker: nat-restricted]
saorsa-6  (NAT: Port Restr) 65.21.157.229     Hetzner/Helsinki    ONLINE  [Docker: nat-portrestricted]
saorsa-7  (Test)            116.203.101.172   Hetzner/Nuremberg   ONLINE
saorsa-8  (Test)            149.28.156.231    Vultr/Singapore     ONLINE  [High latency to EU/US]
saorsa-9  (Test)            45.77.176.184     Vultr/Tokyo         ONLINE  [High latency to EU/US]
saorsa-10 (NAT: Symmetric)  77.42.39.239      Hetzner/Falkenstein ONLINE  [Netns: natbox]

NAT Emulation Summary:
  saorsa-4:  Full Cone NAT        (Easy - 95% success)
  saorsa-5:  Address Restricted   (Medium - 85% success)
  saorsa-6:  Port Restricted      (Medium - 85% success)
  saorsa-10: Symmetric NAT        (Hard - 70% success)
```

---

## Configuration Schema

**All configs are in `saorsa-testnet/.claude/vps-test/`**

### Main Config: `saorsa-testnet/.claude/vps-test/config.yaml`

```yaml
# Saorsa Testnet Configuration
workspace: ~/Desktop/Devel/projects/saorsa-testnet

modules:
  quic-test:
    binary: saorsa-quic-test
    crate: crates/quic-test
    tests: [connectivity, nat-traversal, throughput, gossip]

  gossip-test:
    binary: saorsa-gossip-test  # Future module
    crate: crates/gossip-test
    tests: [broadcast, crdt-sync, membership]

  core-test:
    binary: saorsa-core-test  # Future module
    crate: crates/core-test
    tests: [unit, integration, property]

  node-test:
    binary: saorsa-node-test  # Future module
    crate: crates/node-test
    tests: [health, sync, storage, retrieval]

build:
  command: cargo zig build --release --target x86_64-unknown-linux-gnu
  target_dir: target/x86_64-unknown-linux-gnu/release

deploy:
  install_path: /opt/saorsa-test/
  service_prefix: saorsa-test
  nodes: all  # or specific list

bootstrap:
  registry: https://saorsa-1.saorsalabs.com
  nodes:
    - saorsa-2.saorsalabs.com:9000
    - saorsa-3.saorsalabs.com:9000

tests:
  connectivity:
    success_criteria:
      direct: 99
      nat_traversed: 85
      relay: 100
    nat_cooloff_seconds: 30
    timeout_seconds: 10

soak:
  wait_1hr: true
  wait_6hr: true
  regression_tolerance: 0
```

---

## Error Recovery

### Build Failure (Local)
1. Parse cargo error output
2. Analyze and fix code
3. Retry build (max 3 attempts)
4. If still fails, ask user for guidance

### Node Unreachable
1. Try hostname, then direct IP
2. Mark degraded if unreachable
3. Continue with remaining nodes
4. Fail if < 50% nodes available

### Service Won't Start
1. SSH and check journalctl
2. Check port conflicts
3. Check disk space
4. Restart service
5. If persistent, use `/vps-test diagnose <node>`

---

## Voice Notifications

Triggered on:
- Test loop start (after interview)
- First test failure
- Fix committed
- Entering WAIT_1HR, WAIT_6HR
- COMPLETE (with proof summary)
- Critical errors

Uses 11Labs via `~/.claude/hooks/notify.py`
