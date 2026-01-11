# Saorsa VPS Infrastructure

Complete documentation for the Saorsa P2P network testing infrastructure.

**Last verified: 2026-01-10 via SSH**

## Overview

The Saorsa testnet consists of 10 VPS nodes distributed across 3 cloud providers in multiple geographic regions. This infrastructure supports comprehensive P2P network testing including NAT traversal, gossip protocol verification, and distributed system validation.

```
                              ┌──────────────────────┐
                              │  saorsa-1 (Registry) │
                              │  Helsinki / Hetzner  │
                              │  Dashboard + Metrics │
                              └──────────┬───────────┘
                                         │
        ┌────────────────────────────────┼────────────────────────────────┐
        │                                │                                │
   Bootstrap                        Test Nodes                    NAT Simulation
        │                                │                                │
┌───────┴───────┐    ┌───────────────────┴───────────────────┐   ┌───────┴───────┐
│  saorsa-2,3   │    │  saorsa-7 (EU), 8 (SG), 9 (JP)        │   │  saorsa-4,5,6 │
│  NYC1, SFO3   │    │  Direct public IPs, varied latency    │   │  Docker NAT   │
│  DigitalOcean │    │  Multi-provider (Hetzner, Vultr)      │   │               │
└───────────────┘    └───────────────────────────────────────┘   │  saorsa-10    │
                                                                  │  Symmetric NAT │
                                                                  └───────────────┘
```

## Node Inventory

### Complete Node Table

| Node | Hostname | IP Address | Provider | Region | Role | SSH | NAT Type |
|------|----------|------------|----------|--------|------|-----|----------|
| saorsa-1 | saorsa-1.saorsalabs.com | 77.42.75.115 | Hetzner | Helsinki, FI | Dashboard & Registry | ✓ | None |
| saorsa-2 | saorsa-2.saorsalabs.com | 142.93.199.50 | DigitalOcean | NYC1, US | Bootstrap | ✓ | None |
| saorsa-3 | saorsa-3.saorsalabs.com | 147.182.234.192 | DigitalOcean | SFO3, US | Bootstrap | ✓ | None |
| saorsa-4 | saorsa-4.saorsalabs.com | 206.189.7.117 | DigitalOcean | AMS3, NL | NAT Test | ✓ | Full Cone |
| saorsa-5 | saorsa-5.saorsalabs.com | 144.126.230.161 | DigitalOcean | LON1, UK | NAT Test | ✓ | Addr Restricted |
| saorsa-6 | saorsa-6.saorsalabs.com | 65.21.157.229 | Hetzner | Helsinki, FI | NAT Test | ✓ | Port Restricted |
| saorsa-7 | saorsa-7.saorsalabs.com | 116.203.101.172 | Hetzner | Nuremberg, DE | General Test | ✓ | None |
| saorsa-8 | saorsa-8.saorsalabs.com | 149.28.156.231 | Vultr | Singapore, SG | High Latency | ✓ | None |
| saorsa-9 | saorsa-9.saorsalabs.com | 45.77.176.184 | Vultr | Tokyo, JP | High Latency | ✓ | None |
| saorsa-10 | saorsa-10.saorsalabs.com | 77.42.39.239 | Hetzner | Falkenstein, DE | NAT Test | ✓ | Symmetric |

### Provider Distribution

```
DigitalOcean (4 nodes):
  ├── saorsa-2 (NYC1) - Bootstrap
  ├── saorsa-3 (SFO3) - Bootstrap
  ├── saorsa-4 (AMS3) - NAT: Full Cone
  └── saorsa-5 (LON1) - NAT: Address Restricted

Hetzner (4 nodes):
  ├── saorsa-1 (Helsinki) - Dashboard/Registry
  ├── saorsa-6 (Helsinki) - NAT: Port Restricted
  ├── saorsa-7 (Nuremberg) - General Test
  └── saorsa-10 (Falkenstein) - NAT: Symmetric

Vultr (2 nodes):
  ├── saorsa-8 (Singapore) - High Latency Test
  └── saorsa-9 (Tokyo) - High Latency Test
```

## NAT Simulation Infrastructure

### NAT Type Matrix

| Node | NAT Type | Difficulty | Implementation | Expected Success |
|------|----------|------------|----------------|------------------|
| saorsa-4 | Full Cone | Easy | Docker + iptables | 95% |
| saorsa-5 | Address Restricted | Medium | Docker + iptables | 85% |
| saorsa-6 | Port Restricted | Medium | Docker + iptables | 85% |
| saorsa-10 | Symmetric | Hard | Network Namespace | 70% |

### NAT Behaviors Explained

#### Full Cone NAT (saorsa-4)
- **Behavior**: Once an internal address:port is mapped to external address:port, ANY external host can send packets to that external endpoint
- **Traversal**: Easiest to traverse - just need to send one outbound packet
- **Docker containers**: `nat-fullcone`, `node-fullcone`
- **iptables rules**:
```bash
iptables -t nat -A POSTROUTING -o eth0 -j MASQUERADE
iptables -A FORWARD -i eth0 -o docker0 -m state --state RELATED,ESTABLISHED -j ACCEPT
iptables -A FORWARD -i docker0 -o eth0 -j ACCEPT
```

#### Address Restricted NAT (saorsa-5)
- **Behavior**: External host can send packets only if internal host has previously sent to that external IP
- **Traversal**: Need prior communication to same IP, but any port on that IP works
- **Docker containers**: `nat-restricted`, `node-restricted`
- **iptables rules**:
```bash
iptables -t nat -A POSTROUTING -o eth0 -j MASQUERADE
iptables -A FORWARD -i eth0 -o docker0 -m state --state RELATED,ESTABLISHED -j ACCEPT
iptables -A FORWARD -i docker0 -o eth0 -j ACCEPT
iptables -A FORWARD -i eth0 -s 0/0 -j DROP  # Drop unless established
```

#### Port Restricted NAT (saorsa-6)
- **Behavior**: External host can send packets only if internal host has previously sent to that exact IP:port
- **Traversal**: Need prior communication to exact IP:port combination
- **Docker containers**: `nat-portrestricted`, `node-portrestricted`
- **iptables rules**:
```bash
iptables -t nat -A POSTROUTING -o eth0 -j MASQUERADE
iptables -A FORWARD -i eth0 -o docker0 -m state --state RELATED,ESTABLISHED -j ACCEPT
iptables -A FORWARD -i docker0 -o eth0 -j ACCEPT
```

#### Symmetric NAT (saorsa-10)
- **Behavior**: Different external port for EACH destination IP:port. Hardest to traverse.
- **Traversal**: Requires port prediction or relay fallback
- **Implementation**: Network namespace with nftables
- **Service**: `ant-quic-test-nat.service`
- **Namespace**: `natbox`
- **Commands**:
```bash
# Enter NAT namespace
ip netns exec natbox bash

# Check NAT rules
ip netns exec natbox nft list ruleset

# View connections
ip netns exec natbox conntrack -L
```

### Docker NAT Management

```bash
# Check Docker containers on NAT nodes
ssh root@saorsa-4.saorsalabs.com 'docker ps'
ssh root@saorsa-5.saorsalabs.com 'docker ps'
ssh root@saorsa-6.saorsalabs.com 'docker ps'

# Restart NAT containers
ssh root@saorsa-4.saorsalabs.com 'docker restart nat-fullcone node-fullcone'
ssh root@saorsa-5.saorsalabs.com 'docker restart nat-restricted node-restricted'
ssh root@saorsa-6.saorsalabs.com 'docker restart nat-portrestricted node-portrestricted'

# View container logs
ssh root@saorsa-4.saorsalabs.com 'docker logs node-fullcone --tail 100'
```

### Network Namespace Management (saorsa-10)

```bash
# Check service status
ssh root@saorsa-10.saorsalabs.com 'systemctl status ant-quic-test-nat'

# Restart NAT service
ssh root@saorsa-10.saorsalabs.com 'systemctl restart ant-quic-test-nat'

# List namespaces
ssh root@saorsa-10.saorsalabs.com 'ip netns list'

# Run command in NAT namespace
ssh root@saorsa-10.saorsalabs.com 'ip netns exec natbox ip addr'

# Check connectivity from NAT namespace
ssh root@saorsa-10.saorsalabs.com 'ip netns exec natbox ping -c 3 8.8.8.8'
```

## Node Roles

### saorsa-1: Dashboard & Registry

**Purpose**: Central coordination and monitoring (NOT for P2P testing)

**Services**:
- Registry API (peers registration, discovery)
- Prometheus (metrics collection)
- Grafana (dashboards)
- Loki (log aggregation)
- Promtail (log forwarding)

**Endpoints**:
| URL | Purpose |
|-----|---------|
| https://saorsa-1.saorsalabs.com | Main dashboard |
| https://saorsa-1.saorsalabs.com/api/peers | Peer registry API |
| https://saorsa-1.saorsalabs.com/health | Health check |
| https://saorsa-1.saorsalabs.com/metrics | Prometheus metrics |

**Docker Services**:
```bash
ssh root@saorsa-1.saorsalabs.com 'docker compose ps'
# grafana, prometheus, promtail, loki, registry
```

### saorsa-2, saorsa-3: Bootstrap Nodes

**Purpose**: Always-on entry points for peer discovery

**Characteristics**:
- High availability (rarely restarted)
- Direct public IPs (no NAT)
- US East (NYC1) and US West (SFO3) for geographic distribution
- First contact for new peers joining network

**Services Running**:
| Port | Service | Protocol |
|------|---------|----------|
| 9000 | saorsa-quic-test | UDP |
| 9500 | saorsa-gossip | UDP |
| 10000 | saorsa-node | UDP |
| 11000 | communitas | UDP |

### saorsa-7, saorsa-8, saorsa-9: General Test Nodes

**Purpose**: Standard test endpoints with varied latency profiles

| Node | Region | Latency to EU | Latency to US | Use Case |
|------|--------|---------------|---------------|----------|
| saorsa-7 | Nuremberg, DE | Low | Medium | General EU testing |
| saorsa-8 | Singapore | High | Very High | High-latency scenarios |
| saorsa-9 | Tokyo | High | Very High | Asia-Pacific testing |

**Note**: saorsa-8 and saorsa-9 are especially useful for testing timeout handling and connection resilience under latency.

## Port Allocation

### Standard Ports

| Port | Service | Protocol | Description |
|------|---------|----------|-------------|
| 22 | SSH | TCP | Remote access |
| 80 | HTTP | TCP | Web services |
| 443 | HTTPS | TCP | Secure web services |
| 9000 | saorsa-quic-test | UDP | QUIC/NAT traversal testing |
| 9500 | saorsa-gossip | UDP | Gossip protocol |
| 10000 | saorsa-node | UDP | Full node services |
| 11000 | communitas | UDP | Communitas services |

### Dynamic Port Allocation

All P2P services use **dynamic port allocation** (bind to port 0). The OS assigns an available port, which is then registered with the registry.

```rust
// Example: bind to dynamic port
let socket = UdpSocket::bind("0.0.0.0:0").await?;
let local_addr = socket.local_addr()?;
println!("Bound to {}", local_addr);  // e.g., 0.0.0.0:54321

// Register with registry
registry.register(peer_id, local_addr).await?;
```

## SSH Access

### SSH Configuration

Add to `~/.ssh/config`:

```
Host saorsa-*
    User root
    IdentityFile ~/.ssh/id_ed25519
    StrictHostKeyChecking accept-new

Host saorsa-1
    HostName saorsa-1.saorsalabs.com

Host saorsa-2
    HostName saorsa-2.saorsalabs.com

# ... etc for all nodes
```

### Quick Access Commands

```bash
# SSH to any node
ssh root@saorsa-N.saorsalabs.com

# Direct IP fallback (if DNS issues)
ssh root@142.93.199.50  # saorsa-2
ssh root@77.42.39.239   # saorsa-10

# Multi-node command execution
for n in {2..10}; do
  echo "=== saorsa-$n ==="
  ssh root@saorsa-$n.saorsalabs.com 'hostname && uptime'
done
```

### SSH Key Configuration

**DigitalOcean nodes** (saorsa-2,3,4,5):
- Key name: `mac` (ID: 48810465)
- Key name: `dirvine` (ID: 2064413)

**Hetzner nodes** (saorsa-1,6,7,10):
- Key name: `davidirvine@MacBook-Pro.localdomain` (ID: 104686182)

**Vultr nodes** (saorsa-8,9):
- Standard SSH key authentication

## Service Management

### Systemd Services

```bash
# Check service status
ssh root@saorsa-N.saorsalabs.com 'systemctl status saorsa-quic-test'

# Restart service
ssh root@saorsa-N.saorsalabs.com 'systemctl restart saorsa-quic-test'

# View logs
ssh root@saorsa-N.saorsalabs.com 'journalctl -u saorsa-quic-test -n 100 --no-pager'

# Follow logs in real-time
ssh root@saorsa-N.saorsalabs.com 'journalctl -u saorsa-quic-test -f'
```

### Service Files Location

```
/etc/systemd/system/saorsa-quic-test.service
/etc/systemd/system/saorsa-gossip.service
/etc/systemd/system/saorsa-node.service
/etc/systemd/system/ant-quic-test-nat.service  # saorsa-10 only
```

### Binary Locations

```
/opt/saorsa-test/saorsa-quic-test
/opt/saorsa-test/saorsa-gossip-test
/opt/saorsa-test/saorsa-node-test
```

## Deployment

### CRITICAL: No Remote Builds

**NEVER build binaries on VPS machines.**

All binaries must be:
1. Built locally on macOS using `cargo zig build`
2. Or downloaded from GitHub releases

### Local Cross-Compilation

```bash
# Prerequisites (one-time)
cargo install cargo-zigbuild
brew install zig

# Build for Linux
cargo zig build --release --target x86_64-unknown-linux-gnu

# Binary location
ls target/x86_64-unknown-linux-gnu/release/saorsa-quic-test
```

### Deployment Script

```bash
#!/bin/bash
# deploy-all.sh

BINARY="target/x86_64-unknown-linux-gnu/release/saorsa-quic-test"
INSTALL_PATH="/opt/saorsa-test/"
SERVICE="saorsa-quic-test"

for node in saorsa-{2..10}; do
    echo "Deploying to $node..."
    scp "$BINARY" "root@$node.saorsalabs.com:$INSTALL_PATH"
    ssh "root@$node.saorsalabs.com" "systemctl restart $SERVICE"
    ssh "root@$node.saorsalabs.com" "systemctl status $SERVICE --no-pager"
done
```

## Monitoring & Diagnostics

### Health Checks

```bash
# Quick health check all nodes
for n in {1..10}; do
    echo -n "saorsa-$n: "
    ssh -o ConnectTimeout=5 root@saorsa-$n.saorsalabs.com 'echo OK' 2>/dev/null || echo "FAILED"
done
```

### Disk Space

```bash
ssh root@saorsa-N.saorsalabs.com 'df -h'

# Clear old logs if needed
ssh root@saorsa-N.saorsalabs.com 'journalctl --vacuum-time=1d'
```

### Memory Usage

```bash
ssh root@saorsa-N.saorsalabs.com 'free -h'
```

### Network Diagnostics

```bash
# Check open ports
ssh root@saorsa-N.saorsalabs.com 'ss -tulpn'

# Test connectivity between nodes
ssh root@saorsa-2.saorsalabs.com 'nc -zvu saorsa-3.saorsalabs.com 9000'
```

### Registry API Queries

```bash
# List registered peers
curl -s https://saorsa-1.saorsalabs.com/api/peers | jq

# Health check
curl -s https://saorsa-1.saorsalabs.com/health
```

## Troubleshooting

### Node Unreachable

1. Check DNS resolution:
   ```bash
   dig saorsa-N.saorsalabs.com
   ```

2. Try direct IP:
   ```bash
   ssh root@<IP_ADDRESS>
   ```

3. Check if node is up:
   ```bash
   ping -c 3 <IP_ADDRESS>
   ```

### Service Won't Start

1. Check logs:
   ```bash
   ssh root@saorsa-N 'journalctl -u <service> -n 200 --no-pager'
   ```

2. Check for port conflicts:
   ```bash
   ssh root@saorsa-N 'ss -tulpn | grep <port>'
   ```

3. Check disk space:
   ```bash
   ssh root@saorsa-N 'df -h'
   ```

### NAT Container Issues

1. Check container status:
   ```bash
   ssh root@saorsa-N 'docker ps -a'
   ```

2. View container logs:
   ```bash
   ssh root@saorsa-N 'docker logs <container> --tail 100'
   ```

3. Restart containers:
   ```bash
   ssh root@saorsa-N 'docker restart <container>'
   ```

### High Latency Issues

1. Check network path:
   ```bash
   ssh root@saorsa-N 'traceroute saorsa-M.saorsalabs.com'
   ```

2. Measure latency:
   ```bash
   ssh root@saorsa-N 'ping -c 10 saorsa-M.saorsalabs.com'
   ```

## Appendix: IP Address Quick Reference

```
saorsa-1:  77.42.75.115      (Hetzner/Helsinki)
saorsa-2:  142.93.199.50     (DO/NYC1)
saorsa-3:  147.182.234.192   (DO/SFO3)
saorsa-4:  206.189.7.117     (DO/AMS3)
saorsa-5:  144.126.230.161   (DO/LON1)
saorsa-6:  65.21.157.229     (Hetzner/Helsinki)
saorsa-7:  116.203.101.172   (Hetzner/Nuremberg)
saorsa-8:  149.28.156.231    (Vultr/Singapore)
saorsa-9:  45.77.176.184     (Vultr/Tokyo)
saorsa-10: 77.42.39.239      (Hetzner/Falkenstein)
```
