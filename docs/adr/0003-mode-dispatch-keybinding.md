# ADR 3: Mode-based key handling (not hierarchical state machine)

**Status:** Accepted

## Context

sqrit needs modal key handling: Normal/Insert for the query editor, focus states for Explorer/Query/Results panes, popup states for autocomplete and connection picker. `sqlit` uses a full hierarchical state machine with parent chains and allow/forbid action lists.

## Decision

Use a simple `Mode` enum. Each variant has its own `handle_key()` method. Main event loop dispatches to active mode. No formal state machine.

## Rationale

- Baseline has ~5-6 modes. Not enough to justify a 20+ state hierarchy like `sqlit`.
- Mode enum + handler per mode is the sweet spot: each mode owns its key handling, no god-function match.
- If needed later, a pending-state mechanism (e.g. `gg` requiring two `g` presses) can be added locally within Normal mode's handler.
- The state machine pattern can be introduced in v0.2 if mode count grows beyond what a flat enum handles cleanly.

## Consequences

- Cross-mode transitions (Normal → Insert, Explorer → Query) are explicit `match` arms in each handler.
- No centralized "allowed actions" list — keybinding display in status bar must be derived from current mode.
- Adding new modes requires extending the enum and adding a handler — straightforward but linear growth.
