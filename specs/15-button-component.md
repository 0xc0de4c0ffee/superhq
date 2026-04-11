# 15 — Button component

## Problem

Buttons across the app are hand-rolled divs with inconsistent padding,
colors, hover states, and no keyboard support. The "Cancel" / "Create"
buttons in dialogs, "Save" in settings, "Browse" in path picker, tab
bar actions — all slightly different.

## Reference: current button patterns

| Location | Style | Keyboard | Focus |
|----------|-------|----------|-------|
| Dialog Cancel | `px_3 py(5) rounded(6) text_xs text_dim` | no | no |
| Dialog Create | `px_3 py(5) rounded(6) text_xs bg_selected` | no | no |
| Settings Save | `px_3 py(5) rounded(6) text_xs bg_selected` | no | no |
| Path Browse | `px_2 py(2) rounded(4) text_xs text_dim` | no | no |
| Hold Delete | custom hold-to-confirm | no | no |

None have focus rings, tab stops, or Enter/Space activation.

## Proposed: Button component

### Variants

```rust
pub enum ButtonVariant {
    Default,    // subtle: no bg, text_dim, hover: bg_hover
    Primary,    // emphasis: bg_selected, text_secondary, hover: bg_active
    Danger,     // destructive: text error color, hover: error_bg
    Ghost,      // minimal: just text, no border/bg
}
```

### Props

```rust
pub struct Button {
    label: SharedString,
    variant: ButtonVariant,
    icon: Option<SharedString>,      // optional leading SVG icon
    disabled: bool,
    focus_handle: FocusHandle,       // tab_stop(true)
    on_click: Option<Box<dyn Fn(...)>>,
}
```

### Behavior

- Tab-focusable (`tab_stop(true)`)
- Focus border: `border_focus()` when focused
- Enter/Space activates when focused (via `Confirm` action in `Button` context)
- Hover: variant-specific bg change
- Disabled: `opacity(0.5)`, no cursor, no click/keyboard

### Styling (matching existing "Create" button as Primary reference)

All variants share:
- `px_3 py(px(5.0)) rounded(px(6.0)) text_xs font_weight(MEDIUM)`
- `cursor_pointer` (unless disabled)
- `flex items_center gap(px(6.0))` (for icon + label)

Variant differences:
- **Default**: `text_dim`, hover `bg_hover text_tertiary`
- **Primary**: `bg_selected text_secondary`, hover `bg_active text_primary`
- **Danger**: `text error_text`, hover `error_bg`
- **Ghost**: `text_dim`, hover `text_secondary`

### Usage

```rust
Button::new("Cancel", cx)
    .on_click(cx.listener(|this, _, window, cx| { ... }))

Button::new("Create", cx)
    .variant(ButtonVariant::Primary)
    .on_click(cx.listener(|this, _, window, cx| { ... }))

Button::new("Delete", cx)
    .variant(ButtonVariant::Danger)
    .icon("icons/trash.svg")
    .on_click(...)
```

### Key bindings

```rust
KeyBinding::new("enter", Confirm, Some("Button")),
KeyBinding::new("space", Confirm, Some("Button")),
```

Only fires when the button itself has focus — scoped to "Button" context.

## Implementation

1. Create `src/ui/components/button.rs`
2. Register key bindings in `actions.rs`
3. Wire into new_workspace dialog (Cancel + Create)
4. Wire into ports dialog
5. Wire into settings (Save, Reset, Remove)
