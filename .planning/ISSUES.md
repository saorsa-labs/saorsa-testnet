# GSD-Hybrid Issues - Communitas MCP Integration

> Deferred work backlog organized by priority

## Priority Definitions

- **P0**: Blockers - Must fix before continuing current task
- **P1**: Next Phase - Required for next phase to begin
- **P2**: This Milestone - Must complete before milestone done
- **P3**: Future - Nice to have, backlog items

---

## P0: Blockers

*None currently*

---

## P1: Next Phase Dependencies

### ISSUE-001: communitas-mcp Binary Location
**Status**: Open
**Context**: Need to determine how quic-test will access communitas-mcp
**Options**:
1. Spawn as subprocess (simplest)
2. Add as library dependency (tighter integration)
3. Connect to external running instance (most flexible)

**Decision Needed**: Which approach for M1 Phase 1.1?

### ISSUE-002: Demo User Identity Display
**Status**: Open
**Context**: When running in demo mode, should display four-word identity prominently
**Location**: TUI header or dedicated status area
**Blocks**: M1 Phase 1.3

---

## P2: This Milestone (M1)

### ISSUE-003: MCP Event Type Definition
**Status**: Open
**Context**: Need to define TuiEvent variants for MCP updates
**Current**: `UpdateMcpState(McpState)` exists but may need expansion
**Consider**: Separate events for connection, tools, invocation results

### ISSUE-004: Error Handling Strategy
**Status**: Open
**Context**: How to handle MCP tool invocation errors in TUI
**Options**:
1. Toast notifications
2. Error panel in MCP tab
3. Both with configurable verbosity

### ISSUE-005: Tool Parameter Input
**Status**: Open
**Context**: Some MCP tools have complex nested parameters
**Challenge**: TUI-based JSON input is difficult
**Consider**: Simplified forms for common operations, raw JSON for advanced

---

## P3: Future / Backlog

### ISSUE-006: WebRTC Call Integration
**Status**: Deferred
**Context**: start_voice_call, join_call tools require WebRTC
**Challenge**: TUI cannot handle audio/video
**Consider**: Launch external handler or document limitation

### ISSUE-007: File Upload Progress
**Status**: Deferred
**Context**: Large file uploads need progress indication
**Location**: File operations MCP sub-page

### ISSUE-008: Kanban Board Visualization
**Status**: Deferred
**Context**: Full kanban board in TUI would be complex
**Consider**: Simplified list view vs actual columns

### ISSUE-009: Test Coverage Metrics
**Status**: Deferred
**Context**: Track which of 133 tools have automated tests
**Location**: M4 test infrastructure

### ISSUE-010: Performance Benchmarks
**Status**: Deferred
**Context**: Measure MCP tool invocation latency
**Location**: M5 testnet validation

---

## Closed Issues

*None yet*

---

## Issue Template

```markdown
### ISSUE-XXX: Title
**Status**: Open | In Progress | Resolved | Deferred
**Priority**: P0 | P1 | P2 | P3
**Context**: Why this matters
**Options**: (if decision needed)
1. Option A
2. Option B
**Decision**: (when resolved)
**Blocks**: (what phases/tasks this blocks)
**Resolved By**: (commit/PR reference)
```
