# NAT Traversal in ant-quic

## Overview

The ant-quic library implements NAT traversal using a **single unified QUIC endpoint** for all operations. There are no separate relay servers or coordinator endpoints - all nodes participate in relay and coordination through the same connection infrastructure.

## Key Design Principles

1. **Single Endpoint**: All traffic (direct, hole-punched, relayed, coordination) flows through one QUIC endpoint
2. **Gossip-Based Coordination**: NAT traversal coordination uses epidemic gossip - no centralized coordinators
3. **Peer-to-Peer Relay**: Any well-connected peer can act as a relay or coordinator
4. **Fallback Strategy**: Direct → Hole-punch → Relay (in order of preference)

## Connection Strategy

### Three-Layer Connectivity

```
Layer 1: Direct QUIC (~20% of connections)
  - Public nodes can connect directly
  - No NAT traversal needed

Layer 2: Hole-Punched (~75% target)
  - Gossip-based coordination for simultaneous punching
  - Works for most NAT types (Full Cone, Address/Port Restricted)
  - Uses existing peers as coordination relays

Layer 3: MASQUE Relay (~5% fallback)
  - Last resort when hole-punching fails
  - Traffic relayed through well-connected peers
  - Uses same QUIC connection (multiplexed)
```

### NAT Types and Expected Success

| NAT Type | Direct | Hole-Punch | Needs Relay |
|----------|--------|------------|-------------|
| Public | Yes | N/A | No |
| Full Cone | No | High success | Rarely |
| Address Restricted | No | Good success | Sometimes |
| Port Restricted | No | Moderate success | Sometimes |
| Symmetric | No | Low success | Usually |

## Gossip-Based NAT Traversal Protocol

### Message Types

All messages are JSON-encoded and flow through regular QUIC streams:

```rust
enum RelayMessage {
    CanYouReach(CanYouReachRequest),      // Query if peer can reach target
    ReachResponse(ReachResponse),          // Response to CAN_YOU_REACH
    RelayPunchMeNow(RelayPunchMeNowRequest), // Request hole-punch coordination
    RelayAck(RelayAckResponse),            // Acknowledgment
}
```

### Hole-Punch Coordination Flow

```
Node A (behind NAT) wants to connect to Node B (behind NAT):

1. A tries direct connection to B → FAILS
2. A sends CAN_YOU_REACH(target=B) to all connected peers
3. Peer C responds "YES, I can reach B"
4. A sends RELAY_PUNCH_ME_NOW(target=B, my_addresses=[...]) to C
5. C forwards the request to B
6. B receives → starts punching to A's addresses
7. A simultaneously starts punching to B's addresses
8. Both sides send UDP packets → NAT holes open
9. QUIC connection established!
```

### Timing

- CAN_YOU_REACH query: 2 second wait for responses
- Punch coordination: 3 second window for simultaneous punching
- Connect timeout: 5 seconds per address

## Implementation Details

### No Separate Infrastructure

Unlike traditional NAT traversal (STUN/TURN/ICE), ant-quic:

- Does NOT require dedicated STUN servers
- Does NOT require dedicated TURN relay servers
- Does NOT require separate coordinator services
- Uses the same QUIC endpoint for everything

### Multiplexed Services

All services share the same QUIC connection:

```
QUIC Connection
├── Gossip messages (peer discovery, announcements)
├── NAT coordination (CAN_YOU_REACH, PUNCH_ME_NOW)
├── Relay traffic (when hole-punch fails)
├── Application data
└── Health probes
```

### Peer Selection for Coordination

When selecting a peer to help coordinate:

1. Query all connected peers with CAN_YOU_REACH
2. First peer that responds positively becomes the relay
3. That peer forwards PUNCH_ME_NOW to the target
4. Both endpoints start punching simultaneously

### Relay Fallback

If hole-punching fails after coordination:

1. Connection still not established after 3s
2. Fall back to existing relay infrastructure
3. Traffic flows through well-connected peer
4. Same QUIC connection, just relayed

## Configuration

NAT traversal is automatic and requires no special configuration. The endpoint automatically:

1. Detects its NAT type via external probes
2. Announces capabilities to the gossip network
3. Responds to coordination requests from other peers
4. Acts as relay for peers that need it (if well-connected)

## Metrics

Key metrics to monitor:

- `conn_direct`: Direct connections (public nodes)
- `conn_hole_punched`: Successful hole-punched connections
- `conn_relayed`: Connections using relay fallback

Target: Minimize `conn_relayed`, maximize `conn_hole_punched`

## Troubleshooting

### Low hole-punch success rate

1. Check NAT types - symmetric NATs are hardest
2. Verify bidirectional punching is happening
3. Check timing - both sides must punch simultaneously
4. Look for "HOLE-PUNCH SUCCESS" in logs

### High relay usage

1. Many symmetric NATs in the network
2. Coordination messages not being processed
3. Check for "CAN_YOU_REACH" responses in logs

### Connection failures

1. Ensure gossip network is healthy
2. Check that peers are responding to CAN_YOU_REACH
3. Verify RELAY_PUNCH_ME_NOW is being forwarded
