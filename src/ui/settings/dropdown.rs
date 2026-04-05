//! A themed dropdown select component.
//!
//! ```ignore
//! let state = cx.new(|cx| DropdownState::new(items, selected, cx));
//! cx.subscribe(&state, |this, _, ev: &DropdownEvent, cx| { ... }).detach();
//! // In render:
//! self.dropdown.clone()
//! ```

use gpui::*;
use gpui::prelude::FluentBuilder as _;
use crate::ui::theme as t;

// ── Item ─────────────────────────────────────────────────────────

#[derive(Clone)]
pub struct DropdownItem {
    pub id: i64,
    pub label: String,
    pub icon: Option<String>,
}

// ── Event ────────────────────────────────────────────────────────

pub enum DropdownEvent {
    Change(Option<i64>),
}

impl EventEmitter<DropdownEvent> for DropdownState {}

// ── State ────────────────────────────────────────────────────────

pub struct DropdownState {
    items: Vec<DropdownItem>,
    selected: Option<i64>,
    open: bool,
    highlight: Option<usize>,
    /// Focus handle for the trigger (tab-focusable).
    trigger_focus: FocusHandle,
    /// Focus handle for the open menu (receives arrow/enter/escape).
    menu_focus: FocusHandle,
}

impl DropdownState {
    pub fn new(items: Vec<DropdownItem>, selected: Option<i64>, cx: &mut Context<Self>) -> Self {
        Self {
            items,
            selected,
            open: false,
            highlight: None,
            trigger_focus: cx.focus_handle().tab_stop(true),
            menu_focus: cx.focus_handle().tab_stop(false),
        }
    }

    fn open(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.open = true;
        self.highlight = self
            .selected
            .and_then(|id| self.items.iter().position(|i| i.id == id));
        self.menu_focus.focus(window);
        cx.notify();
    }

    fn dismiss(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.open = false;
        self.highlight = None;
        self.trigger_focus.focus(window);
        cx.notify();
    }

    fn confirm(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if let Some(i) = self.highlight {
            self.selected = Some(self.items[i].id);
            cx.emit(DropdownEvent::Change(self.selected));
        }
        self.dismiss(window, cx);
    }

    fn move_highlight(&mut self, delta: isize, cx: &mut Context<Self>) {
        let count = self.items.len();
        if count == 0 {
            return;
        }
        let cur = self.highlight.unwrap_or(0) as isize;
        self.highlight = Some((cur + delta).rem_euclid(count as isize) as usize);
        cx.notify();
    }

    fn selected_item(&self) -> Option<&DropdownItem> {
        self.selected
            .and_then(|id| self.items.iter().find(|i| i.id == id))
    }

    fn render_trigger(&self, focused: bool) -> Div {
        let label = self
            .selected_item()
            .map(|i| i.label.clone())
            .unwrap_or_else(|| "Select...".into());
        let icon = self.selected_item().and_then(|i| i.icon.clone());

        div()
            .px_3()
            .py(px(6.0))
            .rounded(px(6.0))
            .cursor_pointer()
            .border_1()
            .border_color(if self.open || focused {
                t::border_strong()
            } else {
                t::border_subtle()
            })
            .bg(t::bg_input())
            .hover(|s: StyleRefinement| s.border_color(t::border_strong()))
            .flex()
            .items_center()
            .gap(px(6.0))
            .min_w(px(160.0))
            .children(icon.map(|path| {
                svg()
                    .path(SharedString::from(path))
                    .size(px(14.0))
                    .text_color(t::text_dim())
            }))
            .child(
                div()
                    .text_xs()
                    .text_color(t::text_secondary())
                    .flex_grow()
                    .child(label),
            )
            .child(
                div()
                    .text_xs()
                    .text_color(t::text_faint())
                    .child(if self.open { "\u{25B4}" } else { "\u{25BE}" }),
            )
    }

    fn render_option(
        item: &DropdownItem,
        is_selected: bool,
        is_highlighted: bool,
    ) -> Stateful<Div> {
        let label = item.label.clone();
        let icon = item.icon.clone();
        let text_color = if is_selected {
            t::text_secondary()
        } else {
            t::text_dim()
        };

        div()
            .id(SharedString::from(format!("dd-opt-{}", item.id)))
            .mx_1()
            .px(px(8.0))
            .py(px(6.0))
            .rounded(px(5.0))
            .cursor_pointer()
            .flex()
            .items_center()
            .gap(px(8.0))
            .when(is_highlighted, |el: Stateful<Div>| el.bg(t::bg_active()))
            .when(is_selected && !is_highlighted, |el: Stateful<Div>| {
                el.bg(t::bg_selected())
            })
            .hover(|s: StyleRefinement| s.bg(t::bg_active()))
            .children(icon.map(|path| {
                svg()
                    .path(SharedString::from(path))
                    .size(px(14.0))
                    .text_color(text_color)
            }))
            .child(div().text_xs().text_color(text_color).child(label))
    }
}

// ── Render ────────────────────────────────────────────────────────

impl Render for DropdownState {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let open = self.open;
        let highlight = self.highlight;
        let selected = self.selected;
        let trigger_focused = self.trigger_focus.is_focused(window);

        let mut root = div().relative().child(
            div()
                .id("dd-trigger")
                .track_focus(&self.trigger_focus)
                .child(self.render_trigger(trigger_focused))
                .on_click(cx.listener(|this, _, window, cx| {
                    if this.open {
                        this.dismiss(window, cx);
                    } else {
                        this.open(window, cx);
                    }
                }))
                .on_key_down(cx.listener(|this, event: &KeyDownEvent, window, cx| {
                    match event.keystroke.key.as_str() {
                        "enter" | "space" | "down" => {
                            if !this.open {
                                this.open(window, cx);
                            }
                        }
                        _ => {}
                    }
                })),
        );

        if open {
            let options = self.items.iter().enumerate().map(|(i, item)| {
                Self::render_option(item, selected == Some(item.id), highlight == Some(i))
                    .on_click(cx.listener(move |this, _, window, cx| {
                        this.highlight = Some(i);
                        this.confirm(window, cx);
                    }))
                    .on_mouse_move(cx.listener(move |this, _, _, cx| {
                        if this.highlight != Some(i) {
                            this.highlight = Some(i);
                            cx.notify();
                        }
                    }))
            });

            root = root.child(
                div()
                    .id("dd-menu")
                    .track_focus(&self.menu_focus)
                    .absolute()
                    .top(px(40.0))
                    .right_0()
                    .min_w(px(200.0))
                    .py_1()
                    .rounded(px(8.0))
                    .bg(rgb(0x222222))
                    .border_1()
                    .border_color(t::border())
                    .shadow_md()
                    .on_mouse_down_out(cx.listener(|this, _, window, cx| {
                        this.dismiss(window, cx);
                    }))
                    .on_key_down(cx.listener(|this, event: &KeyDownEvent, window, cx| {
                        match event.keystroke.key.as_str() {
                            "down" => this.move_highlight(1, cx),
                            "up" => this.move_highlight(-1, cx),
                            "enter" => this.confirm(window, cx),
                            "escape" => this.dismiss(window, cx),
                            _ => {}
                        }
                    }))
                    .children(options),
            );
        }

        root
    }
}
