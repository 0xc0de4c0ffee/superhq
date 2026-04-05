//! A button that requires holding to activate — the hold IS the confirmation.
//!
//! As the user holds, a fill animation progresses from left to right.
//! Releasing early cancels. Completing the hold triggers the callback.

use gpui::*;
use gpui::prelude::FluentBuilder as _;
use std::time::Duration;

use crate::ui::theme as t;

const HOLD_DURATION_MS: u64 = 1200;
const FRAME_INTERVAL_MS: u64 = 16; // ~60fps

pub struct HoldButton {
    id: ElementId,
    label: SharedString,
    icon_path: Option<SharedString>,
    fill_color: Rgba,
    text_color: Rgba,
    progress: f32,
    hold_task: Option<Task<()>>,
    on_confirm: std::rc::Rc<dyn Fn(&mut App) + 'static>,
}

impl HoldButton {
    pub fn new(
        id: impl Into<ElementId>,
        label: impl Into<SharedString>,
        fill_color: Rgba,
        text_color: Rgba,
        on_confirm: impl Fn(&mut App) + 'static,
    ) -> Self {
        Self {
            id: id.into(),
            label: label.into(),
            icon_path: None,
            fill_color,
            text_color,
            progress: 0.0,
            hold_task: None,
            on_confirm: std::rc::Rc::new(on_confirm),
        }
    }

    pub fn icon(mut self, path: impl Into<SharedString>) -> Self {
        self.icon_path = Some(path.into());
        self
    }

    fn start_hold(&mut self, cx: &mut Context<Self>) {
        self.progress = 0.0;
        let this = cx.entity().downgrade();
        let on_confirm = self.on_confirm.clone();

        self.hold_task = Some(cx.spawn(async move |_, cx| {
            let steps = HOLD_DURATION_MS / FRAME_INTERVAL_MS;
            let increment = 1.0 / steps as f32;

            for _ in 0..steps {
                gpui::Timer::after(Duration::from_millis(FRAME_INTERVAL_MS)).await;

                let should_continue = cx.update(|cx| {
                    this.update(cx, |view, cx| {
                        view.progress += increment;
                        cx.notify();
                        view.hold_task.is_some() // still holding?
                    }).unwrap_or(false)
                }).unwrap_or(false);

                if !should_continue { return; }
            }

            // Completed — fire callback
            cx.update(|cx| {
                this.update(cx, |view, cx| {
                    view.progress = 1.0;
                    view.hold_task = None;
                    (on_confirm)(cx);
                }).ok();
            }).ok();
        }));

        cx.notify();
    }

    fn cancel_hold(&mut self, cx: &mut Context<Self>) {
        self.hold_task = None;
        self.progress = 0.0;
        cx.notify();
    }
}

impl Render for HoldButton {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let progress = self.progress;
        let fill_color = self.fill_color;
        let text_color = self.text_color;
        let is_holding = self.hold_task.is_some();

        div()
            .id(self.id.clone())
            .relative()
            .px_2p5()
            .py(px(5.0))
            .rounded(px(4.0))
            .cursor_pointer()
            .overflow_hidden()
            .text_xs()
            .text_color(text_color)
            .bg(t::bg_hover())
            .when(!is_holding, |s| s.hover(|s| s.bg(t::bg_selected())))
            .on_mouse_down(MouseButton::Left, cx.listener(|this, _, _, cx| {
                this.start_hold(cx);
            }))
            .on_mouse_up(MouseButton::Left, cx.listener(|this, _, _, cx| {
                if this.progress < 1.0 {
                    this.cancel_hold(cx);
                }
            }))
            // Fill bar (absolute, behind text)
            .when(progress > 0.0, |el| {
                el.child(
                    div()
                        .absolute()
                        .top_0()
                        .left_0()
                        .h_full()
                        .w(relative(progress))
                        .bg(fill_color)
                        .rounded(px(4.0)),
                )
            })
            // Content (above fill)
            .child(
                div()
                    .relative()
                    .flex()
                    .items_center()
                    .gap_1p5()
                    .children(self.icon_path.as_ref().map(|path| {
                        svg()
                            .path(path.clone())
                            .size(px(14.0))
                            .text_color(text_color)
                    }))
                    .child(if is_holding {
                        SharedString::from("Hold to delete")
                    } else {
                        self.label.clone()
                    }),
            )
    }
}
