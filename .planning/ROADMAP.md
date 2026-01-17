# GSD-Hybrid Roadmap - Communitas MCP Integration

> Milestone and phase tracking for comprehensive MCP testing

## Overview

**Goal**: Expose 100% of Communitas-MCP functions in TUI, enable multi-user demo mode, and build comprehensive automated testing infrastructure.

**Architecture Decision**: Library dependency (tight coupling with communitas-core)

**Success Criteria**:
1. Network connectivity verified (existing)
2. All TUI pages functional (gossip, logs, connectivity)
3. MCP tab exposes all 133 functions via category sub-tabs
4. Demo user auto-created on TUI startup (four-word ID in header)
5. Cross-instance DM messaging (manual testing)
6. Automated test scripts for all MCP functions
7. 10-node VPS testnet with random messaging

---

## Codebase Foundation

### What EXISTS ✅
| Component | Location | Description |
|-----------|----------|-------------|
| Four-word identity | `communitas-core/src/identity.rs` | Generation, validation, seed derivation |
| Presence beacons | `communitas-core/src/gossip/presence.rs` | SWIM integration, TTL, group-scoped |
| Contact storage | `communitas-core/src/gossip/contact_storage.rs` | Endpoints, reliability tracking |
| FOAF discovery | `communitas-core/src/gossip/discovery.rs` | 2-hop, introducer nodes, cycle detection |
| Invite system | `communitas-core/src/invite.rs` | Four-word codes, permissions, CRDT |
| MCP tools | `communitas-mcp/src/tools.rs` | 133 tools, JSON-RPC 2.0 |
| TUI MCP tab | `quic-test/src/tui/screens/mcp.rs` | UI placeholder ready |

### What NEEDS BUILDING ❌
| Component | Priority | Notes |
|-----------|----------|-------|
| Identity packet protocol | M2 | Wire format, signing |
| Contact exchange protocol | M2 | Mutual verification |
| Invite links/QR codes | M3 | Shareable URLs |
| Extended presence records | M2 | Capabilities |

---

## Milestone 1: MCP Integration Foundation

> Connect quic-test TUI to communitas-core/communitas-mcp

### Phase 1.1: Library Integration
**Status**: ✅ COMPLETE
**Decision**: Library dependency (tight coupling)

Tasks:
- [x] Add communitas-core to Cargo.toml dependencies
- [x] Add communitas-mcp to Cargo.toml dependencies
- [x] Create `src/mcp/mod.rs` module
- [x] Initialize GossipContext with auto-demo mode
- [x] Wire to TuiEvent channel (UpdateMcpState)
- [x] Display four-word ID in TUI header

Files:
- `crates/quic-test/Cargo.toml` (modify)
- `crates/quic-test/src/mcp/mod.rs` (new)
- `crates/quic-test/src/mcp/client.rs` (new)
- `crates/quic-test/src/main.rs` (modify)
- `crates/quic-test/src/tui/ui.rs` (modify - header)

### Phase 1.2: MCP Tab Category Sub-Tabs
**Status**: ✅ COMPLETE
**Decision**: Category sub-tabs layout

Categories (7):
1. **Auth** (8 tools) - authenticate, create_vault, health_check...
2. **Entities** (16 tools) - create_entity, add_member, join_entity...
3. **Messages** (14 tools) - send_message, create_thread, reactions...
4. **Files** (6 tools) - write_file, read_file, list_files...
5. **Kanban** (23 tools) - boards, cards, columns, tags...
6. **Network** (22 tools) - network_*, dht_*, metrics...
7. **Social** (10 tools) - polls, calls, stories...

Tasks:
- [x] Create sub-tab navigation within MCP tab
- [x] Group tools by category (McpToolCategory enum)
- [x] Tool list with filter per category
- [x] Keyboard navigation (←/→ categories, ↑/↓ tools, 1-7 direct select)
- [ ] Tool parameter form (Phase 1.4)
- [ ] Formatted result display (Phase 1.4)

### Phase 1.3: Demo Mode Activation
**Status**: ✅ COMPLETE
**Decision**: Always auto-create demo user

Tasks:
- [x] Remove --demo flag requirement (always demo - no flag ever existed)
- [x] Generate unique four-word identity on startup
- [x] Display identity prominently in header
- [x] Configure unique storage per instance (--data-dir)
- [x] Auto-authenticate GossipContext

### Phase 1.4: Tool Invocation UI
**Status**: ✅ COMPLETE
**Decision**: Simple parameter form with JSON result display

Tasks:
- [x] Tool parameter input form (text fields for each parameter)
- [x] Enter key invokes selected tool (stub - real invocation needs McpClient channel)
- [x] Display invocation result (success/error)
- [x] Invocation history tracking
- [ ] Wire McpClient channel for real tool invocation (future: M2 scope)

---

## Milestone 2: Direct Messaging (First Test)

> Get DM working between two TUI instances

**Decision**: Manual two-instance test first, DM as first scenario

### Phase 2.1: Contact Discovery
**Status**: ✅ COMPLETE

Tasks:
- [x] Implement contact creation from four-word ID
- [x] Wire FOAF discovery for unknown contacts (infrastructure ready)
- [x] Display contact list with status dots (green/yellow/red)
- [x] Show contact's last-seen endpoint

Files:
- `crates/quic-test/src/tui/types.rs` - ContactDisplay, ContactOnlineStatus
- `crates/quic-test/src/tui/screens/mcp.rs` - draw_contacts_list, header with contacts count
- `crates/quic-test/src/tui/app.rs` - contact navigation methods
- `crates/quic-test/src/tui/mod.rs` - McpRequest, TuiEvent contact variants, keyboard handling
- `crates/quic-test/src/mcp/client.rs` - ContactInfo, contact CRUD methods

### Phase 2.2: Message Send/Receive
**Status**: ✅ COMPLETE

Tasks:
- [x] Implement send_message MCP tool invocation
- [x] Create simple message composition UI
- [x] Subscribe to incoming messages
- [x] Display received messages in conversation view
- [x] Implement reply flow

Files:
- `crates/quic-test/src/tui/types.rs` - MessageDisplay, composing_message, current_messages
- `crates/quic-test/src/tui/screens/mcp.rs` - draw_conversation_view function
- `crates/quic-test/src/tui/app.rs` - message composition methods
- `crates/quic-test/src/tui/mod.rs` - McpRequest::SendMessage/LoadMessages, TuiEvent message variants
- `crates/quic-test/src/mcp/client.rs` - send_direct_message, get_direct_messages, MessageInfo
- `crates/quic-test/src/main.rs` - handle_mcp_requests, MCP request handler task

### Phase 2.3: End-to-End DM Test
**Status**: PENDING
**Decision**: Mix local + remote testing

Test Procedure:
1. Launch TUI Instance A (gets identity: word1.word2.word3.word4)
2. Launch TUI Instance B (gets identity: word5.word6.word7.word8)
3. Instance A: Add contact with B's four-word ID
4. Instance A: Send "Hello from A"
5. Instance B: Verify receipt
6. Instance B: Reply "Hello from B"
7. Instance A: Verify receipt

---

## Milestone 3: Full MCP Function Coverage

> Expose all 133 tools with working invocation

### Phase 3.1: Entity & Membership Tools (16)
Tasks: Create/join/leave entities, member management

### Phase 3.2: Messaging & Reaction Tools (14)
Tasks: Threads, editing, reactions

### Phase 3.3: File Operation Tools (6)
Tasks: Read, write, list, sync

### Phase 3.4: Kanban Tools (23)
Tasks: Boards, cards, columns, tags, assignments

### Phase 3.5: Network & DHT Tools (22)
Tasks: Network control, DHT ops, metrics

### Phase 3.6: Social & Call Tools (10)
Tasks: Polls, stories, calls (UI only for WebRTC)

---

## Milestone 4: Automated Test Infrastructure

> Scripts testing all MCP functions

### Phase 4.1: Test Framework
- Test runner script
- Result collection
- Reporting

### Phase 4.2-4.6: Category Test Suites
- One phase per tool category
- Coverage tracking

---

## Milestone 5: VPS Testnet Deployment

> 10-node testnet with random messaging

### Phase 5.1: Testnet Setup
### Phase 5.2: Random Messaging Automation
### Phase 5.3: Full E2E Validation

---

## Summary

| Milestone | Phases | Priority | Status |
|-----------|--------|----------|--------|
| M1: MCP Integration | 4 | HIGH | ✅ **COMPLETE** |
| M2: Direct Messaging | 3 | HIGH | 🔄 IN PROGRESS (2/3 phases done) |
| M3: Full MCP Coverage | 6 | MEDIUM | PENDING |
| M4: Test Infrastructure | 6 | MEDIUM | PENDING |
| M5: VPS Testnet | 3 | LOW | PENDING |

**Total Phases**: 22
**Completed Phases**: 6 (M1: 4, M2: 2)
**Next Action**: M2 Phase 2.3 - End-to-End DM Test
