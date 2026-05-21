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

## Addendum — Command mode removed (2026-05-21, supersedes 2026-05-20 addendum)

After the v0.2 space command palette (T2) landed, the `:` command mode handled exactly one input — `q`/`quit`/`q!`/`quit!` → quit — which is now fully covered by `<space>q`. `Mode::Command` was kept open as an extension surface for future multi-arg commands (`:w`, `:e <path>`, `:set …`), but the v0.2/v0.3 roadmap has no such commands: settings live in `~/.sqrit/config.toml`, exports go through results-pane shortcuts, and the only outstanding "prompt" surfaces (`/` filter in T6) get their own dedicated transient mode rather than reusing the command line.

Decision: remove `Mode::Command`, the `command_buffer` / `command_origin` fields on `App`, the `:` arms in `QueryNormal` / `Explorer` / `Results`, and the `src/mode/command.rs` handler. `:` becomes an unbound no-op outside `QueryInsert` (where it remains a literal character). Mode count drops back to 5, well within the flat-enum threshold.

The origin-tracking pattern from the 2026-05-20 addendum is still useful and lives on in `ThemePickerState::origin`; it just isn't worth a dedicated mode for a single one-shot command.

If a multi-arg command surface is ever required (e.g. ad-hoc scripting, `:source`), reintroducing `Mode::Command` is mechanical — but until then, the palette + `?` help overlay (T3) carries the discoverability load without the maintenance cost of a parallel input system.

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
