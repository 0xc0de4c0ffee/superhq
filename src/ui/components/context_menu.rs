use gpui::*;
use gpui::prelude::FluentBuilder as _;
use crate::ui::theme as t;
use super::actions::{Cancel, Confirm, SelectUp, SelectDown};

pub struct MenuItem {
    pub label: SharedString,
    pub shortcut: Option<SharedString>,
    pub action: Box<dyn Fn(&mut Window, &mut App)>,
    pub disabled: bool,
}

impl MenuItem {
    pub fn new(label: impl Into<SharedString>, action: impl Fn(&mut Window, &mut App) + 'static) -> Self {
        Self {
            label: label.into(),
            shortcut: None,
            action: Box::new(action),
            disabled: false,
        }
    }

    #[allow(dead_code)]
    pub fn shortcut(mut self, shortcut: impl Into<SharedString>) -> Self {
        self.shortcut = Some(shortcut.into());
        self
    }

    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }
}

pub enum MenuEntry {
    Item(MenuItem),
    Separator,
}

#[derive(Clone)]
pub enum ContextMenuEvent {
    Dismiss,
}

pub struct ContextMenu {
    entries: Vec<MenuEntry>,
    pub position: Point<Pixels>,
    pub focus_handle: FocusHandle,
    highlight: Option<usize>,
}

impl EventEmitter<ContextMenuEvent> for ContextMenu {}

impl ContextMenu {
    pub fn new(position: Point<Pixels>, entries: Vec<MenuEntry>, cx: &mut Context<Self>) -> Self {
        let focus_handle = cx.focus_handle();
        Self { entries, position, focus_handle, highlight: None }
    }

    fn dismiss(&mut self, _: &Cancel, _window: &mut Window, cx: &mut Context<Self>) {
        cx.emit(ContextMenuEvent::Dismiss);
    }

    fn confirm(&mut self, _: &Confirm, window: &mut Window, cx: &mut Context<Self>) {
        if let Some(idx) = self.highlight {
            if let MenuEntry::Item(item) = &self.entries[idx] {
                if !item.disabled {
                    (item.action)(window, cx);
                }
            }
        }
        cx.emit(ContextMenuEvent::Dismiss);
    }

    fn select_up(&mut self, _: &SelectUp, _window: &mut Window, cx: &mut Context<Self>) {
        self.move_highlight(-1);
        cx.notify();
    }

    fn select_down(&mut self, _: &SelectDown, _window: &mut Window, cx: &mut Context<Self>) {
        self.move_highlight(1);
        cx.notify();
    }

    fn move_highlight(&mut self, delta: isize) {
        let item_indices: Vec<usize> = self.entries.iter().enumerate()
            .filter_map(|(i, e)| match e {
                MenuEntry::Item(item) if !item.disabled => Some(i),
                _ => None,
            })
            .collect();

        if item_indices.is_empty() { return; }

        let current_pos = self.highlight
            .and_then(|h| item_indices.iter().position(|&i| i == h))
            .unwrap_or(if delta > 0 { item_indices.len() - 1 } else { 0 });

        let new_pos = (current_pos as isize + delta).rem_euclid(item_indices.len() as isize) as usize;
        self.highlight = Some(item_indices[new_pos]);
    }
}

impl Focusable for ContextMenu {
    fn focus_handle(&self, _: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl Render for ContextMenu {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let mut menu = div().min_w(px(160.0));

        for (i, entry) in self.entries.iter().enumerate() {
            match entry {
                MenuEntry::Separator => {
                    menu = menu.child(t::menu_separator());
                }
                MenuEntry::Item(item) => {
                    let disabled = item.disabled;
                    let label = item.label.clone();
                    let shortcut = item.shortcut.clone();
                    let highlighted = self.highlight == Some(i);
                    let item_idx = i;

                    menu = menu.child(
                        t::menu_item()
                            .id(SharedString::from(format!("ctx-{i}")))
                            .hover(|s| s.bg(t::bg_hover()))
                            .justify_between()
                            .when(highlighted, |el| el.bg(t::bg_active()))
                            .when(!disabled, |el| {
                                el.on_click(cx.listener(move |menu, _, window, cx| {
                                    if let MenuEntry::Item(item) = &menu.entries[item_idx] {
                                        (item.action)(window, cx);
                                    }
                                    cx.emit(ContextMenuEvent::Dismiss);
                                }))
                            })
                            .when(disabled, |el| {
                                el.text_color(t::text_ghost())
                                    .cursor_default()
                            })
                            .child(label)
                            .when_some(shortcut, |el, sc| {
                                el.child(
                                    div()
                                        .text_color(t::text_ghost())
                                        .text_xs()
                                        .child(sc),
                                )
                            }),
                    );
                }
            }
        }

        t::popover()
            .id("context-menu")
            .key_context("ContextMenu")
            .track_focus(&self.focus_handle)
            .on_action(cx.listener(Self::dismiss))
            .on_action(cx.listener(Self::confirm))
            .on_action(cx.listener(Self::select_up))
            .on_action(cx.listener(Self::select_down))
            .on_mouse_move(cx.listener(|this, _, _, cx| {
                if this.highlight.is_some() {
                    this.highlight = None;
                    cx.notify();
                }
            }))
            .on_mouse_down(MouseButton::Left, cx.listener(|_, _: &MouseDownEvent, _, cx| {
                cx.stop_propagation();
            }))
            .on_mouse_down_out(cx.listener(|_this, _: &MouseDownEvent, _, cx| {
                cx.emit(ContextMenuEvent::Dismiss);
            }))
            .child(menu)
    }
}
