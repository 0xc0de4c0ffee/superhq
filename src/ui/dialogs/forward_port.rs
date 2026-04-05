use gpui::*;
use gpui::prelude::FluentBuilder as _;
use gpui_component::input::{Input, InputState};
use gpui_component::Sizable as _;

use crate::db::{Database, PortMapping};
use crate::ui::theme as t;
use std::sync::Arc;

enum PortDialogView {
    List,
    Add,
}

pub struct ForwardPortDialog {
    db: Arc<Database>,
    workspace_id: i64,
    view: PortDialogView,
    guest_input: Entity<InputState>,
    host_input: Entity<InputState>,
    on_dismiss: Box<dyn Fn(&mut Window, &mut App) + 'static>,
    focus_handle: FocusHandle,
}

impl ForwardPortDialog {
    pub fn new(
        db: Arc<Database>,
        workspace_id: i64,
        on_dismiss: impl Fn(&mut Window, &mut App) + 'static,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        let guest_input = cx.new(|cx| {
            let mut s = InputState::new(window, cx);
            s.set_placeholder("e.g. 3000", window, cx);
            s
        });
        let host_input = cx.new(|cx| {
            let mut s = InputState::new(window, cx);
            s.set_placeholder("Defaults to same port", window, cx);
            s
        });
        let focus_handle = cx.focus_handle();
        focus_handle.focus(window);
        Self {
            db,
            workspace_id,
            view: PortDialogView::List,
            guest_input,
            host_input,
            on_dismiss: Box::new(on_dismiss),
            focus_handle,
        }
    }

    fn show_add_view(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.view = PortDialogView::Add;
        self.guest_input.update(cx, |s, cx| {
            s.set_value("", window, cx);
            s.focus(window, cx);
        });
        self.host_input.update(cx, |s, cx| s.set_value("", window, cx));
        cx.notify();
    }

    fn submit_add(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let guest_str = self.guest_input.read(cx).value().to_string();
        let host_str = self.host_input.read(cx).value().to_string();

        let guest_port: u16 = match guest_str.trim().parse() {
            Ok(p) if p > 0 => p,
            _ => return,
        };
        let host_port: u16 = if host_str.trim().is_empty() {
            guest_port
        } else {
            match host_str.trim().parse() {
                Ok(p) if p > 0 => p,
                _ => return,
            }
        };

        let mapping = PortMapping { guest_port, host_port };
        if let Err(e) = self.db.add_port_mapping(self.workspace_id, &mapping) {
            eprintln!("[ports] failed to add mapping: {e}");
            return;
        }

        self.view = PortDialogView::List;
        self.guest_input.update(cx, |s, cx| s.set_value("", window, cx));
        self.host_input.update(cx, |s, cx| s.set_value("", window, cx));
        self.focus_handle.focus(window);
        cx.notify();
    }

    fn cancel_add(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.view = PortDialogView::List;
        self.focus_handle.focus(window);
        cx.notify();
    }

    fn remove_mapping(&self, guest_port: u16, cx: &mut Context<Self>) {
        if let Err(e) = self.db.remove_port_mapping(self.workspace_id, guest_port) {
            eprintln!("[ports] failed to remove mapping: {e}");
        }
        cx.notify();
    }

    fn dismiss(&self, window: &mut Window, cx: &mut App) {
        (self.on_dismiss)(window, cx);
    }

    fn render_list(&self, cx: &mut Context<Self>) -> AnyElement {
        let mappings = self.db.get_port_mappings(self.workspace_id).unwrap_or_default();
        let has_mappings = !mappings.is_empty();

        let mut port_rows = div().flex().flex_col();
        for pm in &mappings {
            let gp = pm.guest_port;
            port_rows = port_rows.child(
                div()
                    .px_4()
                    .py_2()
                    .flex()
                    .items_center()
                    .border_b_1()
                    .border_color(t::border_subtle())
                    .child(
                        div()
                            .flex_grow()
                            .text_xs()
                            .text_color(t::text_secondary())
                            .child(format!(":{} \u{2192} localhost:{}", pm.guest_port, pm.host_port)),
                    )
                    .child(
                        div()
                            .id(SharedString::from(format!("port-rm-{gp}")))
                            .px_1p5()
                            .py(px(1.0))
                            .rounded(px(3.0))
                            .cursor_pointer()
                            .text_xs()
                            .text_color(t::text_ghost())
                            .hover(|s| s.text_color(t::text_muted()).bg(t::bg_hover()))
                            .on_click(cx.listener(move |this, _, _, cx| {
                                this.remove_mapping(gp, cx);
                            }))
                            .child("Remove"),
                    ),
            );
        }

        div()
            .flex()
            .flex_col()
            // Header
            .child(
                div()
                    .px_4()
                    .py_3()
                    .border_b_1()
                    .border_color(t::border_subtle())
                    .flex()
                    .items_center()
                    .justify_between()
                    .child(
                        div()
                            .text_sm()
                            .font_weight(FontWeight::MEDIUM)
                            .text_color(t::text_secondary())
                            .child("Forwarded Ports"),
                    )
                    .child(
                        div()
                            .id("port-close-btn")
                            .px_2()
                            .py_1()
                            .rounded(px(4.0))
                            .cursor_pointer()
                            .text_xs()
                            .text_color(t::text_ghost())
                            .hover(|s| s.bg(t::bg_hover()).text_color(t::text_dim()))
                            .on_click(cx.listener(|this, _, window, cx| {
                                this.dismiss(window, cx);
                            }))
                            .child("Close"),
                    ),
            )
            // Port rows
            .when(has_mappings, |el| el.child(port_rows))
            // Empty state
            .when(!has_mappings, |el| {
                el.child(
                    div()
                        .px_4()
                        .py_6()
                        .flex()
                        .items_center()
                        .justify_center()
                        .child(
                            div()
                                .text_xs()
                                .text_color(t::text_faint())
                                .child("No ports forwarded"),
                        ),
                )
            })
            // Footer
            .child(
                div()
                    .px_4()
                    .py_3()
                    .border_t_1()
                    .border_color(t::border_subtle())
                    .flex()
                    .justify_end()
                    .child(
                        div()
                            .id("port-add-btn")
                            .px_3()
                            .py_1()
                            .rounded(px(6.0))
                            .cursor_pointer()
                            .text_xs()
                            .bg(t::bg_selected())
                            .text_color(t::text_secondary())
                            .hover(|s| s.bg(t::bg_hover()).text_color(t::text_primary()))
                            .on_click(cx.listener(|this, _, window, cx| {
                                this.show_add_view(window, cx);
                            }))
                            .child("Forward Port"),
                    ),
            )
            .into_any_element()
    }

    fn render_add(&self, cx: &mut Context<Self>) -> AnyElement {
        div()
            .flex()
            .flex_col()
            // Header
            .child(
                div()
                    .px_4()
                    .py_3()
                    .border_b_1()
                    .border_color(t::border_subtle())
                    .child(
                        div()
                            .text_sm()
                            .font_weight(FontWeight::MEDIUM)
                            .text_color(t::text_secondary())
                            .child("Forward Port"),
                    ),
            )
            // Body
            .child(
                div()
                    .px_4()
                    .py_3()
                    .flex()
                    .flex_col()
                    .gap_3()
                    .child(
                        div()
                            .flex()
                            .flex_col()
                            .gap_1()
                            .child(
                                div().text_xs().text_color(t::text_dim()).child("Port"),
                            )
                            .child(Input::new(&self.guest_input).small()),
                    )
                    .child(
                        div()
                            .flex()
                            .flex_col()
                            .gap_1()
                            .child(
                                div().text_xs().text_color(t::text_dim()).child("Forward to"),
                            )
                            .child(Input::new(&self.host_input).small())
                            .child(
                                div()
                                    .text_xs()
                                    .text_color(t::text_faint())
                                    .child("Leave empty to use the same port on host"),
                            ),
                    ),
            )
            // Footer
            .child(
                div()
                    .px_4()
                    .py_3()
                    .border_t_1()
                    .border_color(t::border_subtle())
                    .flex()
                    .justify_end()
                    .gap_2()
                    .child(
                        div()
                            .id("port-back-btn")
                            .px_3()
                            .py_1()
                            .rounded(px(6.0))
                            .cursor_pointer()
                            .text_xs()
                            .text_color(t::text_dim())
                            .hover(|s| s.bg(t::bg_hover()).text_color(t::text_tertiary()))
                            .on_click(cx.listener(|this, _, window, cx| {
                                this.cancel_add(window, cx);
                            }))
                            .child("Cancel"),
                    )
                    .child(
                        div()
                            .id("port-submit-btn")
                            .px_3()
                            .py_1()
                            .rounded(px(6.0))
                            .cursor_pointer()
                            .text_xs()
                            .bg(t::bg_selected())
                            .text_color(t::text_secondary())
                            .hover(|s| s.bg(t::bg_hover()).text_color(t::text_primary()))
                            .on_click(cx.listener(|this, _, window, cx| {
                                this.submit_add(window, cx);
                            }))
                            .child("Forward"),
                    ),
            )
            .into_any_element()
    }
}

impl Render for ForwardPortDialog {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let body = match self.view {
            PortDialogView::List => self.render_list(cx),
            PortDialogView::Add => self.render_add(cx),
        };

        div()
            .id("port-dialog-backdrop")
            .absolute()
            .top_0()
            .left_0()
            .size_full()
            .flex()
            .items_center()
            .justify_center()
            .bg(rgba(0x00000088))
            .occlude()
            .on_mouse_down(MouseButton::Left, cx.listener(|this, _, window, cx| {
                this.dismiss(window, cx);
            }))
            .child(
                div()
                    .id("port-dialog-card")
                    .track_focus(&self.focus_handle)
                    .on_key_down(cx.listener(|this, event: &KeyDownEvent, window, cx| {
                        if event.keystroke.key == "escape" {
                            match this.view {
                                PortDialogView::Add => this.cancel_add(window, cx),
                                PortDialogView::List => this.dismiss(window, cx),
                            }
                        }
                    }))
                    .w(px(360.0))
                    .bg(t::bg_surface())
                    .border_1()
                    .border_color(t::border())
                    .rounded(px(10.0))
                    .shadow_lg()
                    .flex()
                    .flex_col()
                    .overflow_hidden()
                    .on_mouse_down(MouseButton::Left, |_, _, cx| {
                        cx.stop_propagation();
                    })
                    .child(body),
            )
    }
}
