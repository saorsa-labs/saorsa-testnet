# Saorsa Testnet

Comprehensive testing infrastructure for the Saorsa P2P network ecosystem.

[![Build](https://img.shields.io/badge/Build-Passing-brightgreen.svg)]()
[![Rust](https://img.shields.io/badge/Rust-1.85+-orange.svg)]()
[![License](https://img.shields.io/badge/License-MIT%20OR%20Apache--2.0-blue.svg)]()

## Overview

Saorsa Testnet is a centralized workspace for all P2P network testing binaries. It consolidates testing for:

- **ant-quic** - QUIC transport with NAT traversal
- **saorsa-gossip** - Gossip protocol and CRDT sync
- **saorsa-core** - Core P2P library
- **saorsa-node** - Full network nodes

## Workspace Structure

```
saorsa-testnet/
├── Cargo.toml                          # Workspace manifest
├── crates/
│   ├── quic-test/                      # QUIC and NAT traversal testing
│   │   ├── src/
│   │   │   ├── main.rs                 # saorsa-quic-test binary
│   │   │   ├── bin/
│   │   │   │   ├── saorsa-testctl.rs   # Test controller
│   │   │   │   └── test-agent.rs       # VPS test agent
│   │   │   ├── harness/                # Test harness framework
│   │   │   ├── registry/               # Peer registry
│   │   │   ├── tui/                    # Terminal UI
│   │   │   └── ...
│   │   └── Cargo.toml
│   ├── gossip-test/                    # Gossip protocol testing (planned)
│   ├── core-test/                      # Core library testing (planned)
│   └── node-test/                      # Full node testing (planned)
├── docs/
│   └── infrastructure/
│       └── VPS_INFRASTRUCTURE.md       # Complete VPS documentation
└── .claude/
    └── skills/
        └── vps-test/                   # Autonomous VPS testing skill
            └── SKILL.md
```

## Quick Start

### Prerequisites

```bash
# Install cargo-zigbuild for cross-compilation
cargo install cargo-zigbuild

# Install zig (macOS)
brew install zig
```

### Build Test Binaries

```bash
# Standard build
cargo build --release

# Cross-compile for Linux VPS
cargo zig build --release --target x86_64-unknown-linux-gnu
```

### Run QUIC Tests Locally

```bash
# Run the main test orchestrator
cargo run -p saorsa-quic-test -- --help

# Run with TUI dashboard
cargo run -p saorsa-quic-test -- --dashboard

# Run test controller
cargo run -p saorsa-quic-test --bin saorsa-testctl -- --help
```

### Embedded Communitas Demo

Every `saorsa-quic-test` node now boots a Communitas demo identity and MCP server alongside the QUIC tester. This lets you open two TUIs and exchange real Communitas messages/files without any extra setup:

```bash
# Terminal 1
cargo run -p saorsa-quic-test -- --data-dir /tmp/node-a

# Terminal 2
cargo run -p saorsa-quic-test -- --data-dir /tmp/node-b
```

Each node derives a deterministic four-word identity from its peer ID, auto-creates a per-node Communitas vault under `<data_dir>/communitas/<four-words>`, and exposes bespoke MCP forms in the TUI. To disable the embedded Communitas stack (for very low-memory test rigs) pass `--no-communitas`.

## Test Binaries

| Binary | Description | Command |
|--------|-------------|---------|
| `saorsa-quic-test` | Main QUIC/NAT test orchestrator | `cargo run -p saorsa-quic-test` |
| `saorsa-testctl` | Test controller for VPS deployments | `cargo run -p saorsa-quic-test --bin saorsa-testctl` |
| `test-agent` | VPS agent for remote test execution | `cargo run -p saorsa-quic-test --bin test-agent` |

## VPS Infrastructure

10 nodes across 3 cloud providers with full NAT simulation:

| Node | Provider | Role | NAT Type |
|------|----------|------|----------|
| saorsa-1 | Hetzner | Dashboard & Registry | - |
| saorsa-2, 3 | DigitalOcean | Bootstrap Nodes | Direct |
| saorsa-4 | DigitalOcean | NAT Simulation | Full Cone |
| saorsa-5 | DigitalOcean | NAT Simulation | Address Restricted |
| saorsa-6 | Hetzner | NAT Simulation | Port Restricted |
| saorsa-7 | Hetzner | General Test | Direct |
| saorsa-8 | Vultr | High Latency Test | Direct |
| saorsa-9 | Vultr | High Latency Test | Direct |
| saorsa-10 | Hetzner | NAT Simulation | Symmetric |

See [docs/infrastructure/VPS_INFRASTRUCTURE.md](docs/infrastructure/VPS_INFRASTRUCTURE.md) for complete details including IP addresses, SSH access, and NAT configuration.

## VPS Deployment

### CRITICAL: No Remote Builds

**NEVER build on VPS machines.** All binaries must be:
1. Built locally using `cargo zig build`
2. Uploaded via SCP

### Deploy to All Nodes

```bash
# Build for Linux
cargo zig build --release --target x86_64-unknown-linux-gnu

# Deploy
BINARY="target/x86_64-unknown-linux-gnu/release/saorsa-quic-test"
for n in {2..10}; do
    scp "$BINARY" root@saorsa-$n.saorsalabs.com:/opt/saorsa-test/
    ssh root@saorsa-$n.saorsalabs.com 'systemctl restart saorsa-quic-test'
done
```

## VPS Test Skill

The `/vps-test` Claude Code skill provides autonomous deployment and testing:

```bash
# In Claude Code session
/vps-test start           # Interactive test setup with interview
/vps-test status          # Show all active test loops
/vps-test nodes           # List VPS infrastructure
/vps-test diagnose saorsa-4   # Debug specific node
/vps-test logs ant-quic   # View test logs
```

Features:
- Mandatory interview process before each test
- Local-only builds with cargo-zigbuild
- Automated fix cycles on failure
- Graduated success verification (1hr, 6hr soak tests)
- Voice notifications via 11Labs

See [.claude/skills/vps-test/SKILL.md](.claude/skills/vps-test/SKILL.md) for complete documentation.

## Test Modules

### quic-test (Active)

Tests QUIC transport and NAT traversal:
- **Connectivity Matrix**: All-to-all connection testing
- **NAT Traversal**: Full Cone, Address Restricted, Port Restricted, Symmetric
- **Throughput**: Bandwidth measurement across nodes
- **Gossip Integration**: Message broadcast verification

### gossip-test (Planned)

Tests gossip protocol layer:
- Message broadcast verification
- CRDT state synchronization
- Membership protocol
- Epidemic spread analysis

### core-test (Planned)

Tests saorsa-core library:
- Integration test runner
- Property-based tests
- Stress tests

### node-test (Planned)

Tests full saorsa-node:
- Health check endpoint
- Sync verification
- Storage tests
- Retrieval tests

## Development

### Build Commands

```bash
# Check compilation
cargo check

# Run tests
cargo test

# Build release
cargo build --release

# Cross-compile for Linux
cargo zig build --release --target x86_64-unknown-linux-gnu

# Lint
cargo clippy --all-targets -- -D warnings

# Format
cargo fmt --all
```

### Adding New Test Modules

1. Create new crate:
   ```bash
   mkdir -p crates/my-test/src
   ```

2. Add `Cargo.toml` using workspace dependencies:
   ```toml
   [package]
   name = "saorsa-my-test"
   version.workspace = true
   edition.workspace = true

   [dependencies]
   tokio.workspace = true
   anyhow.workspace = true
   ```

3. Add to workspace in root `Cargo.toml`:
   ```toml
   [workspace]
   members = [
       "crates/quic-test",
       "crates/my-test",  # Add here
   ]
   ```

## Registry API

The registry at saorsa-1 provides peer discovery:

```bash
# List registered peers
curl https://saorsa-1.saorsalabs.com/api/peers

# Health check
curl https://saorsa-1.saorsalabs.com/health

# Dashboard
open https://saorsa-1.saorsalabs.com
```

## Metrics

Prometheus metrics available at `/metrics` on each node:

- `saorsa_connections_total` - Total connection attempts
- `saorsa_nat_traversal_success` - NAT traversal success rate
- `saorsa_gossip_messages_total` - Gossip messages sent/received
- `saorsa_throughput_bytes` - Data transferred

Dashboard: https://saorsa-1.saorsalabs.com

## License

MIT OR Apache-2.0

## Related Projects

- [ant-quic](https://github.com/dirvine/ant-quic) - QUIC transport with NAT traversal
- [saorsa-gossip](https://github.com/dirvine/saorsa-gossip) - Gossip protocol crates
- [saorsa-core](https://github.com/dirvine/saorsa-core) - Core P2P library
- [saorsa-node](https://github.com/dirvine/saorsa-node) - Full network node
