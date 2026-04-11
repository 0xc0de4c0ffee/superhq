use gpui::*;
use gpui::prelude::FluentBuilder as _;
use crate::ui::theme as t;
use super::actions::Confirm;

#[derive(Clone, Copy, PartialEq, Default)]
#[allow(dead_code)]
pub enum ButtonVariant {
    #[default]
    Default,
    Primary,
    Danger,
    Ghost,
}

/// Focusable button entity. Use for dialog footers and primary actions
/// where Tab focus and Enter/Space activation matter.
///
/// For simple inline action buttons that don't need Tab focus,
/// use `t::button()` / `t::button_primary()` / `t::button_danger()`.
pub struct Button {
    label: SharedString,
    variant: ButtonVariant,
    icon: Option<SharedString>,
    disabled: bool,
    focus_handle: FocusHandle,
    on_click: Option<Box<dyn Fn(&mut Window, &mut App) + 'static>>,
}

impl Button {
    pub fn new(label: impl Into<SharedString>, cx: &mut Context<Self>) -> Self {
        Self {
            label: label.into(),
            variant: ButtonVariant::Default,
            icon: None,
            disabled: false,
            focus_handle: cx.focus_handle().tab_stop(true),
            on_click: None,
        }
    }

    pub fn variant(mut self, variant: ButtonVariant) -> Self {
        self.variant = variant;
        self
    }

    #[allow(dead_code)]
    pub fn icon(mut self, path: impl Into<SharedString>) -> Self {
        self.icon = Some(path.into());
        self
    }

    #[allow(dead_code)]
    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    pub fn on_click(mut self, handler: impl Fn(&mut Window, &mut App) + 'static) -> Self {
        self.on_click = Some(Box::new(handler));
        self
    }

    fn activate(&mut self, _: &Confirm, window: &mut Window, cx: &mut Context<Self>) {
        if self.disabled { return; }
        if let Some(ref handler) = self.on_click {
            handler(window, cx);
        }
    }
}

impl Focusable for Button {
    fn focus_handle(&self, _: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl Render for Button {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let focused = self.focus_handle.is_focused(window);
        let variant = self.variant;
        let disabled = self.disabled;

        let (text_color, bg, hover_text, hover_bg) = match variant {
            ButtonVariant::Default => (
                t::text_dim(),
                None,
                Some(t::text_tertiary()),
                Some(t::bg_hover()),
            ),
            ButtonVariant::Primary => (
                t::text_secondary(),
                Some(t::bg_selected()),
                Some(t::text_primary()),
                Some(t::bg_active()),
            ),
            ButtonVariant::Danger => (
                t::error_text(),
                None,
                None,
                Some(t::error_bg()),
            ),
            ButtonVariant::Ghost => (
                t::text_dim(),
                None,
                Some(t::text_secondary()),
                None,
            ),
        };

        let mut el = div()
            .id("button")
            .key_context("Button")
            .track_focus(&self.focus_handle)
            .on_action(cx.listener(Self::activate))
            .px_3()
            .py(px(5.0))
            .rounded(px(6.0))
            .text_xs()
            .font_weight(FontWeight::MEDIUM)
            .flex()
            .items_center()
            .gap(px(6.0))
            .text_color(text_color)
            .border_1()
            .border_color(if focused { t::border_focus() } else { t::transparent() });

        if let Some(bg) = bg {
            el = el.bg(bg);
        }

        el = el
            .when(!disabled, |el| {
                el.cursor_pointer()
                    .hover(move |s| {
                        let s = if let Some(c) = hover_text { s.text_color(c) } else { s };
                        if let Some(c) = hover_bg { s.bg(c) } else { s }
                    })
                    .on_click(cx.listener(|this, _, window, cx| {
                        if let Some(ref handler) = this.on_click {
                            handler(window, cx);
                        }
                    }))
            })
            .when(disabled, |el| {
                el.opacity(0.5).cursor_default()
            });

        if let Some(ref icon) = self.icon {
            el = el.child(
                svg()
                    .path(icon.clone())
                    .size(px(14.0))
                    .text_color(text_color),
            );
        }

        el.child(self.label.clone())
    }
}
