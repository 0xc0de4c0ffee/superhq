use gpui::*;
use gpui::prelude::FluentBuilder as _;
use crate::ui::theme as t;

#[derive(Clone, Copy, PartialEq)]
#[allow(dead_code)]
pub enum PathPickerMode {
    File,
    Directory,
}

#[derive(Clone, Debug)]
#[allow(dead_code)]
pub enum PathPickerEvent {
    Changed(Option<String>),
}

pub struct PathPicker {
    value: Option<String>,
    placeholder: String,
    mode: PathPickerMode,
    focus_handle: FocusHandle,
}

impl EventEmitter<PathPickerEvent> for PathPicker {}

impl PathPicker {
    pub fn new(mode: PathPickerMode, cx: &mut Context<Self>) -> Self {
        Self {
            value: None,
            placeholder: match mode {
                PathPickerMode::File => "No file selected".into(),
                PathPickerMode::Directory => "No folder selected".into(),
            },
            mode,
            focus_handle: cx.focus_handle().tab_stop(true),
        }
    }

    #[allow(dead_code)]
    pub fn placeholder(mut self, text: impl Into<String>) -> Self {
        self.placeholder = text.into();
        self
    }

    pub fn value(&self) -> Option<&str> {
        self.value.as_deref()
    }

    #[allow(dead_code)]
    pub fn set_value(&mut self, path: Option<String>, cx: &mut Context<Self>) {
        self.value = path.clone();
        cx.emit(PathPickerEvent::Changed(path));
        cx.notify();
    }

    fn browse(&mut self, cx: &mut Context<Self>) {
        let (files, directories) = match self.mode {
            PathPickerMode::File => (true, false),
            PathPickerMode::Directory => (false, true),
        };
        let prompt_text = match self.mode {
            PathPickerMode::File => "Select file",
            PathPickerMode::Directory => "Select folder",
        };
        let receiver = cx.prompt_for_paths(PathPromptOptions {
            files,
            directories,
            multiple: false,
            prompt: Some(prompt_text.into()),
        });
        cx.spawn(async move |this, cx| {
            if let Ok(Ok(Some(paths))) = receiver.await {
                if let Some(path) = paths.first() {
                    let path_str = path.to_string_lossy().to_string();
                    cx.update(|cx| {
                        this.update(cx, |this, cx| {
                            this.value = Some(path_str.clone());
                            cx.emit(PathPickerEvent::Changed(Some(path_str)));
                            cx.notify();
                        }).ok();
                    }).ok();
                }
            }
        }).detach();
    }

    fn clear(&mut self, cx: &mut Context<Self>) {
        self.value = None;
        cx.emit(PathPickerEvent::Changed(None));
        cx.notify();
    }

    fn display_text(&self) -> &str {
        self.value
            .as_ref()
            .and_then(|p| p.rsplit('/').next())
            .unwrap_or(&self.placeholder)
    }
}

impl Focusable for PathPicker {
    fn focus_handle(&self, _: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl Render for PathPicker {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let has_value = self.value.is_some();
        let focused = self.focus_handle.is_focused(window);
        let display = self.display_text().to_string();

        div()
            .id("path-picker")
            .track_focus(&self.focus_handle)
            .px_2()
            .py(px(5.0))
            .rounded(px(6.0))
            .border_1()
            .border_color(if focused { t::border_focus() } else { t::border() })
            .bg(t::bg_base())
            .hover(|s| s.border_color(t::border_strong()))
            .cursor_pointer()
            .flex()
            .items_center()
            .gap_2()
            .on_click(cx.listener(|this, _, _, cx| {
                this.browse(cx);
            }))
            .child(
                div()
                    .flex_grow()
                    .text_xs()
                    .min_w_0()
                    .overflow_hidden()
                    .text_ellipsis()
                    .text_color(if has_value { t::text_secondary() } else { t::text_ghost() })
                    .child(display),
            )
            .child(
                div()
                    .id("browse-btn")
                    .px_2()
                    .py(px(2.0))
                    .rounded(px(4.0))
                    .text_xs()
                    .text_color(t::text_dim())
                    .hover(|s| s.bg(t::bg_hover()).text_color(t::text_tertiary()))
                    .child("Browse"),
            )
            .when(has_value, |el| {
                el.child(
                    div()
                        .id("clear-btn")
                        .px_2()
                        .py(px(2.0))
                        .rounded(px(4.0))
                        .text_xs()
                        .text_color(t::text_ghost())
                        .hover(|s| s.bg(t::bg_hover()).text_color(t::text_dim()))
                        .on_click(cx.listener(|this, _, _, cx| {
                            cx.stop_propagation();
                            this.clear(cx);
                        }))
                        .child("Clear"),
                )
            })
    }
}
