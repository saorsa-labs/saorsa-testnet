# GSD-Hybrid State - Communitas MCP Integration

> Cross-session memory for project continuity

## Current Position

- **Milestone**: M1 - MCP Integration Foundation
- **Phase**: 0 - Discovery & Planning (COMPLETE)
- **Task**: Ready to begin Phase 1.1

## Interview Decisions (2026-01-14)

### Architecture
| Question | Decision |
|----------|----------|
| MCP Connection | **Library dependency** - communitas-core as tight coupling |
| Function Priority | **All categories equally** - systematic coverage of 133 tools |
| Testing Priority | **Manual two-instance first** - get DMs working ASAP |
| Demo UX | **Four-word ID in header** - prominently displayed |
| Tab Layout | **Category sub-tabs** - Auth \| Entities \| Messages \| Files \| Kanban \| Network \| Social |
| Instance Testing | **Mix of local and remote** - start local, expand to VPS |
| Auto-Demo | **Always auto-create** - fresh demo user on every launch |
| First Test | **Direct message** - User A DMs User B, B replies |
| Results Display | **Formatted summary** - parse JSON to human-readable |
| Agent Workflow | **Full autonomy** - agents work independently |

### Identity Packet System
| Question | Decision |
|----------|----------|
| Packet Contents | **Full presence record** - ID, endpoint, pubkey, profile, status, capabilities |
| Storage | **FOAF gossip** - contacts share directly, DHT backup later |
| Signing | **Mandatory** - prevent impersonation |
| Publishing | **On startup to contacts** - also exchange when adding contacts |
| Bootstrap | **Invite link/code** - generate encrypted identity packet link |
| Presence UI | **Status dots** - green/yellow/red next to names |

## Codebase Analysis

### What EXISTS in communitas-core ✅
1. **Four-word identity** - `identity.rs`, `generate_id_words()`, dictionary validation
2. **Presence beacons** - `gossip/presence.rs`, SWIM integration, TTL management
3. **Contact storage** - `gossip/contact_storage.rs`, endpoint tracking, reliability scores
4. **FOAF discovery** - `gossip/discovery.rs`, 2-hop traversal, introducer nodes
5. **Invite system** - `invite.rs`, four-word codes, entity targeting, permissions

### What NEEDS TO BE BUILT ❌
1. **Identity packet protocol** - wire format, signing, publishing
2. **Contact exchange protocol** - mutual verification, endpoint validation
3. **Invite links/QR codes** - shareable URLs, deep links
4. **Extended presence records** - capabilities, multi-endpoints
5. **Presence announcements** - selective sharing to contacts

## Session Progress

### Completed
- [x] GSD-Hybrid initialization
- [x] Communitas-MCP exploration (133 tools catalogued)
- [x] saorsa-testnet TUI analysis (10 tabs, MCP ready)
- [x] User interview (10 questions, all answered)
- [x] Identity packet system analysis
- [x] Planning documents created

### Decisions Made
See "Interview Decisions" section above

## Active Blockers
- None

## Next Actions

1. **Phase 1.1**: Add communitas-core as library dependency
2. **Phase 1.1**: Create MCP client module in quic-test
3. **Phase 1.1**: Wire demo mode auto-creation on startup
4. **Phase 1.2**: Build category sub-tabs in MCP screen
5. **First Test**: Implement DM send/receive between two instances

## Files Modified This Session
- `.planning/STATE.md` - Created, updated with interview results
- `.planning/ROADMAP.md` - Created, updated with analysis
- `.planning/ISSUES.md` - Created

## Handoff Context

**For Next Session**: Begin Phase 1.1 with full autonomy. Key integration point is adding communitas-core/communitas-mcp as dependencies and wiring GossipContext initialization with auto-demo mode. The MCP tab UI exists but needs real data. Focus on making DM between two TUI instances work first.
