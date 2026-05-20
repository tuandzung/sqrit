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
- Transient modes that need to "return to caller" (e.g. Command mode invoked from Normal / Explorer / Results) store their origin on `App` (`command_origin: Option<Mode>`) and restore it on Esc or after execution. This is the lightweight alternative to a parent-chain state machine endorsed in the rationale.

## Addendum — Command mode (2026-05-20)

Vim-style command-line (`:q`, `:quit`, `:q!`, `:quit!`) added as `Mode::Command`. Entered from QueryNormal / Explorer / Results via `:`. Not entered from Picker (already has a direct `q`) or QueryInsert (`:` is a literal char). Reaffirms the flat-enum decision: mode count is now 6, still well below the threshold that would warrant a hierarchical state machine. The origin-tracking pattern above generalises if more transient prompts are added later (e.g. `/` search).

## Addendum — Trait-based dispatch for help overlay (planned for v0.2)

The flat enum stays — it remains the canonical identifier of *which* mode is active and where state lives on `App`. What changes is the dispatch shape: instead of free `handle_key(key, &mut App)` functions, each mode exposes a small trait:

```rust
trait ModeHandler {
    fn dispatch(&self, key: KeyEvent, app: &mut App);
    fn bindings(&self) -> &'static [KeyBinding];
}
```

`KeyBinding { key: &'static str, action: &'static str }` is the data the help overlay (`?`) renders. By declaring bindings on the same type that dispatches them, the help overlay can never drift from the code that handles keys — the only way to add a binding is to add it to both lists in the same impl block, and the compiler's exhaustiveness checks make missing matches visible.

The `Mode` enum still chooses the active handler:

```rust
impl Mode {
    fn handler(&self) -> &dyn ModeHandler { ... }
}
```

So the addendum is purely an internal refinement — external behavior identical, ADR 3's flat-enum decision intact, but the bindings-list-vs-handler drift class is eliminated structurally rather than by lint.
