use gpui::*;
use gpui::prelude::FluentBuilder as _;
use crate::ui::theme as t;
use super::actions::{Cancel, Confirm, SelectUp, SelectDown};

#[derive(Clone)]
pub struct SelectItem {
    pub id: i64,
    pub label: String,
    pub icon: Option<String>,
}

pub enum SelectEvent {
    Change(Option<i64>),
}

impl EventEmitter<SelectEvent> for Select {}

pub struct Select {
    items: Vec<SelectItem>,
    selected: Option<i64>,
    open: bool,
    highlight: Option<usize>,
    trigger_focus: FocusHandle,
    menu_focus: FocusHandle,
    _focus_out_sub: Option<gpui::Subscription>,
}

impl Select {
    pub fn new(items: Vec<SelectItem>, selected: Option<i64>, cx: &mut Context<Self>) -> Self {
        Self {
            items,
            selected,
            open: false,
            highlight: None,
            trigger_focus: cx.focus_handle().tab_stop(true),
            menu_focus: cx.focus_handle().tab_stop(false),
            _focus_out_sub: None,
        }
    }

    fn open(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.open = true;
        self.highlight = self
            .selected
            .and_then(|id| self.items.iter().position(|i| i.id == id));
        self.menu_focus.focus(window);
        self._focus_out_sub = Some(cx.on_focus_out(&self.menu_focus, window, |this: &mut Self, _, _window: &mut Window, cx: &mut Context<Self>| {
            if this.open {
                this.open = false;
                this.highlight = None;
                cx.notify();
            }
        }));
        cx.notify();
    }

    fn dismiss(&mut self, _: &Cancel, window: &mut Window, cx: &mut Context<Self>) {
        self.open = false;
        self.highlight = None;
        self.trigger_focus.focus(window);
        cx.notify();
    }

    fn confirm(&mut self, _: &Confirm, window: &mut Window, cx: &mut Context<Self>) {
        if let Some(i) = self.highlight {
            self.selected = Some(self.items[i].id);
            cx.emit(SelectEvent::Change(self.selected));
        }
        self.dismiss(&Cancel, window, cx);
    }

    fn select_up(&mut self, _: &SelectUp, _window: &mut Window, cx: &mut Context<Self>) {
        self.move_highlight(-1, cx);
    }

    fn select_down(&mut self, _: &SelectDown, _window: &mut Window, cx: &mut Context<Self>) {
        self.move_highlight(1, cx);
    }

    fn move_highlight(&mut self, delta: isize, cx: &mut Context<Self>) {
        let count = self.items.len();
        if count == 0 { return; }
        let cur = self.highlight.unwrap_or(0) as isize;
        self.highlight = Some((cur + delta).rem_euclid(count as isize) as usize);
        cx.notify();
    }

    fn selected_item(&self) -> Option<&SelectItem> {
        self.selected
            .and_then(|id| self.items.iter().find(|i| i.id == id))
    }
}

impl Render for Select {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let open = self.open;
        let highlight = self.highlight;
        let selected = self.selected;
        let trigger_focused = self.trigger_focus.is_focused(window);

        let label = self
            .selected_item()
            .map(|i| i.label.clone())
            .unwrap_or_else(|| "Select...".into());
        let icon = self.selected_item().and_then(|i| i.icon.clone());

        let mut root = div().relative().child(
            div()
                .id("select-trigger")
                .track_focus(&self.trigger_focus)
                .px_3()
                .py(px(6.0))
                .rounded(px(6.0))
                .cursor_pointer()
                .border_1()
                .border_color(if open || trigger_focused {
                    t::border_focus()
                } else {
                    t::border()
                })
                .bg(t::bg_surface())
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
                        .child(if open { "\u{25B4}" } else { "\u{25BE}" }),
                )
                .on_click(cx.listener(|this, _, window, cx| {
                    if this.open {
                        this.dismiss(&Cancel, window, cx);
                    } else {
                        this.open(window, cx);
                    }
                }))
                .on_action(cx.listener(|this, _: &SelectDown, window, cx| {
                    if !this.open { this.open(window, cx); }
                })),
        );

        if open {
            let options: Vec<_> = self.items.iter().enumerate().map(|(i, item)| {
                let is_selected = selected == Some(item.id);
                let is_highlighted = highlight == Some(i);
                let label = item.label.clone();
                let icon = item.icon.clone();

                t::menu_item()
                    .id(SharedString::from(format!("sel-opt-{}", item.id)))
                    .hover(|s: StyleRefinement| s.bg(t::bg_hover()))
                    .when(is_highlighted, |el| el.bg(t::bg_active()))
                    .when(is_selected && !is_highlighted, |el| el.bg(t::bg_selected()))
                    .when(!is_selected, |el| el.text_color(t::text_dim()))
                    .children(icon.map(|path| {
                        svg()
                            .path(SharedString::from(path))
                            .size(px(14.0))
                            .text_color(if is_selected { t::text_secondary() } else { t::text_dim() })
                    }))
                    .child(label)
                    .on_click(cx.listener(move |this, _, window, cx| {
                        this.highlight = Some(i);
                        this.confirm(&Confirm, window, cx);
                    }))
            }).collect();

            root = root.child(
                gpui::deferred(
                    gpui::anchored()
                        .snap_to_window()
                        .child(
                            t::popover()
                                .id("select-menu")
                                .key_context("Select")
                                .track_focus(&self.menu_focus)
                                .on_action(cx.listener(Self::dismiss))
                                .on_action(cx.listener(Self::confirm))
                                .on_action(cx.listener(Self::select_up))
                                .on_action(cx.listener(Self::select_down))
                                .on_mouse_down_out(cx.listener(|this, _, window, cx| {
                                    this.dismiss(&Cancel, window, cx);
                                }))
                                .on_mouse_move(cx.listener(|this, _, _, cx| {
                                    if this.highlight.is_some() {
                                        this.highlight = None;
                                        cx.notify();
                                    }
                                }))
                                .min_w(px(200.0))
                                .children(options),
                        ),
                ),
            );
        }

        root
    }
}
