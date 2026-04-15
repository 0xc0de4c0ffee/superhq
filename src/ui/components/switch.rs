use gpui::*;
use gpui::prelude::FluentBuilder as _;
use crate::ui::theme as t;

#[derive(Clone, Debug)]
pub enum SwitchEvent {
    Change(bool),
}

/// Focusable on/off switch. Emits `SwitchEvent::Change` on toggle.
/// Activates on click, Space, or Enter when focused.
pub struct Switch {
    value: bool,
    disabled: bool,
    focus_handle: FocusHandle,
}

impl EventEmitter<SwitchEvent> for Switch {}

impl Switch {
    pub fn new(value: bool, cx: &mut Context<Self>) -> Self {
        Self {
            value,
            disabled: false,
            focus_handle: cx.focus_handle().tab_stop(true),
        }
    }

    #[allow(dead_code)]
    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    pub fn value(&self) -> bool {
        self.value
    }

    pub fn set_value(&mut self, value: bool, cx: &mut Context<Self>) {
        if self.value != value {
            self.value = value;
            cx.notify();
        }
    }

    fn toggle(&mut self, cx: &mut Context<Self>) {
        if self.disabled { return; }
        self.value = !self.value;
        cx.emit(SwitchEvent::Change(self.value));
        cx.notify();
    }
}

impl Focusable for Switch {
    fn focus_handle(&self, _: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

const TRACK_W: f32 = 26.0;
const TRACK_H: f32 = 14.0;
const THUMB: f32 = 10.0;
const THUMB_INSET: f32 = 2.0;

impl Render for Switch {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let focused = self.focus_handle.is_focused(window);
        let on = self.value;
        let disabled = self.disabled;

        let track_bg = if on { t::accent() } else { t::border() };
        let thumb_x = if on { TRACK_W - THUMB - THUMB_INSET } else { THUMB_INSET };

        // GPUI auto-fires on_click handlers when Enter/Space is released on a
        // focused element with click_listeners — covers both mouse and keyboard.
        div()
            .id("switch")
            .track_focus(&self.focus_handle)
            // Focus ring sits outside the track so the track shape isn't distorted.
            .p(px(2.0))
            .rounded_full()
            .border_1()
            .border_color(t::focus_ring(focused))
            .when(!disabled, |el| {
                el.cursor_pointer()
                    .on_click(cx.listener(|this, _, _, cx| this.toggle(cx)))
            })
            .when(disabled, |el| el.opacity(0.5).cursor_default())
            .child(
                div()
                    .relative()
                    .w(px(TRACK_W))
                    .h(px(TRACK_H))
                    .rounded_full()
                    .bg(track_bg)
                    .child(
                        div()
                            .absolute()
                            .top(px((TRACK_H - THUMB) / 2.0))
                            .left(px(thumb_x))
                            .w(px(THUMB))
                            .h(px(THUMB))
                            .rounded_full()
                            .bg(t::text_primary()),
                    ),
            )
    }
}
