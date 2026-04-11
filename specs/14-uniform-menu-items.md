# 14 — Uniform menu items across all popovers

## Problem

Five different menu surfaces render items with different padding, font
sizes, icon sizes, hover states, and text colors. They look like they
belong to different apps.

## Reference: new tab menu

The new tab menu is the cleanest design. All menus should match it:
- Item: `px_2.5 py(5) rounded(4) text_xs`
- Icon: `14px`, agent color
- Text: `text_secondary` (enabled), `text_ghost` (disabled)
- Hover: `bg_hover`
- Active/selected: `bg_active`
- Section separators: thin `border` line with `mx_2 my_1`
- Shortcut text: `text_ghost text_xs` right-aligned

## Shared components

### `t::menu_item()` — styled item container

Returns a pre-styled Div for a single menu row:
```rust
pub fn menu_item() -> Div {
    div()
        .px_2p5()
        .py(px(5.0))
        .rounded(px(4.0))
        .text_xs()
        .cursor_pointer()
        .text_color(text_secondary())
        .hover(|s| s.bg(bg_hover()))
        .flex()
        .items_center()
        .gap(px(6.0))
}
```

### `t::menu_separator()` — thin line

```rust
pub fn menu_separator() -> Div {
    div()
        .mx_2()
        .my_1()
        .h(px(1.0))
        .bg(border())
}
```

## Menus to update

1. Context menu (components/context_menu.rs) — per-item render
2. Select (components/select.rs) — option render
3. New tab menu (terminal/mod.rs) — render_menu_item
4. Workspace context menu (sidebar/workspace_item.rs) — inline items
5. Terminal context menu (gpui-terminal crate) — uses gpui-component's
   menu, may need separate handling

## Implementation

1. Add `menu_item()` and `menu_separator()` to theme.rs
2. Update each menu surface to use them
3. Each menu still adds its own click/action handlers — the shared
   helpers only provide consistent visual styling
