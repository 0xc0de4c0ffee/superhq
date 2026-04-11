# 13 — Action-based key dispatch for focus-aware Escape handling

## Problem

Escape key closes the dialog even when a context menu or focused input
should handle it first. Raw `on_key_down` + `stop_propagation` is fragile
and doesn't work reliably with GPUI's deferred rendering.

## How GPUI handles this

GPUI dispatches key events through the **action system**:

1. Each component sets a `key_context` on its root div (e.g., "ContextMenu")
2. Key bindings are scoped to contexts via `KeyBinding::new("escape", Cancel, Some("ContextMenu"))`
3. When a key is pressed, GPUI walks the focus chain bottom-up, matching
   bindings against the context stack
4. The deepest matching handler fires first and consumes the action
5. If no handler matches at that level, it bubbles up

This means: if ContextMenu has focus and binds `escape` -> `Cancel`,
it fires before Dialog's `escape` -> `Dismiss`.

## Changes

### A. Define shared actions

```rust
// src/ui/components/actions.rs
actions!(ui, [Cancel, Confirm]);
```

`Cancel` = escape/dismiss. `Confirm` = enter/submit. Shared across all
components. Registered once at app startup.

### B. Context menu uses Cancel action

- Set `key_context("ContextMenu")`
- Bind `escape` -> `Cancel` in "ContextMenu" context
- Handle `Cancel` via `on_action(cx.listener(Self::dismiss))`
- `dismiss` emits `ContextMenuEvent::Dismiss` and restores focus

### C. Dialog uses Cancel action

- Set `key_context("Dialog")`
- Bind `escape` -> `Cancel` in "Dialog" context
- Handle `Cancel` via `on_action(cx.listener(Self::dismiss))`
- Remove raw `on_key_down` escape handler

### D. TextInput uses Cancel to clear/blur

- Already has `key_context("TextInput")`
- Bind `escape` -> `Cancel` in "TextInput" context (optional)
- Handler: dismiss context menu if open, otherwise blur

### E. Focus chain priority

When context menu is open and focused:
```
Window
  └── Dialog (key_context: "Dialog")
        └── TextInput (key_context: "TextInput")
              └── ContextMenu (key_context: "ContextMenu")  ← focused
```

Escape -> walks up from ContextMenu -> finds Cancel binding -> fires
ContextMenu::dismiss -> consumed. Dialog never sees it.

After menu closes, focus returns to TextInput:
```
Window
  └── Dialog (key_context: "Dialog")
        └── TextInput (key_context: "TextInput")  ← focused
```

Escape -> walks up from TextInput -> if TextInput handles Cancel, it
fires there. Otherwise bubbles to Dialog -> Dialog::dismiss.

## Implementation

1. Create `src/ui/components/actions.rs` with Cancel/Confirm
2. Register bindings at app startup
3. Update ContextMenu to use action-based dispatch
4. Update Dialog to use action-based dispatch
5. Remove all raw `on_key_down` escape handlers
