# ADR 7 — Hint bar layout

**Status:** Accepted
**Date:** 2026-07-15

## Context

Since v0.2, `?` opens a help overlay that lists the active mode's keybindings. The on-demand, modal overlay blocks the rest of the UI while open. Mode keys remain hard to discover: a newcomer in Results does not know that `v` opens the cell viewer or `/` opens the filter until they press `?`. lazygit addresses this problem with an always-visible hint row at the bottom of the screen.

## Decision

Add one reserved row directly above the status bar for mode-aware keybinding hints.

- **Layout (Section B from brainstorming):** Add a row above the status bar instead of replacing it. The status bar keeps its full width for error messages.
- **Content (Section C from brainstorming):** Left-justify mode bindings from `ModeHandler::bindings()`. Right-justify the constant palette suffix `<sp> cmd  ? help`, with a separator pipe between the two groups.
- **Toggleable:** Add `[hint_bar].enabled` to `~/.sqrit/config.toml` with a default of `true`, plus `auto_hide_narrow` for very small terminals.
- **Theme integration:** Add optional `hint_bar_*` keys to each theme TOML `[colors]` table. Missing fields fall back individually: `hint_bar_bg` to `bg`, `hint_bar_fg` to `fg`, `hint_bar_key` to `border_focused`, and `hint_bar_separator` to `border_unfocused`.

## Alternatives considered

- **Replace status bar (Section A).** Rejected: error messages lose width unpredictably; rotating mode hints beside error text creates visual noise and risks overwriting status during long error sequences.
- **`<space>`-armed popup only (Section C of original).** Rejected: it conflicts with the always-visible goal; the help overlay already provides on-demand keybinding help.

## Consequences

- **Invariant V10:** Hint bar bindings come only from `ModeHandler::bindings()`, never inline strings. The help overlay uses the same source, so the two cannot drift.
- Every new mode handler must order `bindings()` from most to least important; PR review catches violations.
- The hint bar reserves one terminal row, or zero when disabled or auto-hidden on narrow terminals.
- The theme schema remains forward-additive: pre-v0.3.1 user TOMLs without the `hint_bar_*` keys render correctly through per-field fallback.
