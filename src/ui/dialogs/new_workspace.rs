use gpui::*;
use crate::ui::components::actions::{Cancel, Confirm};
use crate::ui::components::button::{Button, ButtonVariant};
use crate::ui::components::path_picker::{PathPicker, PathPickerEvent, PathPickerMode};

use crate::db::{CreateWorkspaceParams, Database};
use crate::ui::components::TextInput;
use crate::ui::theme as t;
use std::sync::Arc;

pub struct NewWorkspaceDialog {
    db: Arc<Database>,
    name_input: Entity<TextInput>,
    path_picker: Entity<PathPicker>,
    cancel_btn: Entity<Button>,
    create_btn: Entity<Button>,
    on_created: Box<dyn Fn(&mut Window, &mut App) + 'static>,
    on_dismiss: Box<dyn Fn(&mut Window, &mut App) + 'static>,
    focus_handle: FocusHandle,
}

impl NewWorkspaceDialog {
    pub fn new(
        db: Arc<Database>,
        on_created: impl Fn(&mut Window, &mut App) + 'static,
        on_dismiss: impl Fn(&mut Window, &mut App) + 'static,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        let name_input = cx.new(|cx| {
            let mut input = TextInput::new(cx);
            input.set_placeholder("Workspace name");
            input
        });
        name_input.read(cx).focus(window);

        let path_picker = cx.new(|cx| PathPicker::new(PathPickerMode::Directory, cx));
        cx.subscribe(&path_picker, |_this, _, _event: &PathPickerEvent, cx| {
            cx.notify();
        }).detach();

        let this_weak = cx.entity().downgrade();
        let this_weak2 = this_weak.clone();

        let cancel_btn = cx.new(|cx| {
            Button::new("Cancel", cx)
                .on_click(move |window, cx| {
                    let _ = this_weak.update(cx, |this, cx| {
                        (this.on_dismiss)(window, cx);
                    });
                })
        });
        let create_btn = cx.new(|cx| {
            Button::new("Create", cx)
                .variant(ButtonVariant::Primary)
                .on_click(move |window, cx| {
                    let _ = this_weak2.update(cx, |this, cx| {
                        this.submit(window, cx);
                    });
                })
        });

        let focus_handle = cx.focus_handle();
        focus_handle.focus(window);
        Self {
            db,
            name_input,
            path_picker,
            cancel_btn,
            create_btn,
            on_created: Box::new(on_created),
            on_dismiss: Box::new(on_dismiss),
            focus_handle,
        }
    }

    fn submit(&self, window: &mut Window, cx: &mut App) {
        let name = self.name_input.read(cx).value().to_string();
        if name.is_empty() {
            return;
        }

        let mount_path = self.path_picker.read(cx).value().map(|s| s.to_string());
        let is_git_repo = mount_path
            .as_ref()
            .map_or(false, |p| std::path::Path::new(p).join(".git").exists());

        let settings = self.db.get_settings().ok();
        let _id = self.db.create_workspace(CreateWorkspaceParams {
            name,
            mount_path,
            mount_read_only: true,
            is_git_repo,
            branch_name: None,
            base_branch: None,
            initial_prompt: None,
            sandbox_cpus: settings.as_ref().map(|s| s.sandbox_cpus).unwrap_or(2),
            sandbox_memory_mb: settings.as_ref().map(|s| s.sandbox_memory_mb).unwrap_or(8192),
            sandbox_disk_mb: settings.as_ref().map(|s| s.sandbox_disk_mb).unwrap_or(16384),
            allowed_hosts: None,
            secrets_config: None,
            cloned_from_id: None,
        });

        (self.on_created)(window, cx);
    }

    fn dismiss(&self, window: &mut Window, cx: &mut App) {
        (self.on_dismiss)(window, cx);
    }
}

impl Render for NewWorkspaceDialog {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .id("dialog-backdrop")
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
                    .id("dialog-card")
                    .key_context("Dialog")
                    .track_focus(&self.focus_handle)
                    .tab_group()
                    .on_action(cx.listener(|this, _: &Cancel, window, cx| {
                        this.dismiss(window, cx);
                    }))
                    .on_action(cx.listener(|this, _: &Confirm, window, cx| {
                        this.submit(window, cx);
                    }))
                    .on_key_down(|event, window, cx| {
                        use crate::ui::components::actions::KEY_TAB;
                        if event.keystroke.key.as_str() == KEY_TAB {
                            if event.keystroke.modifiers.shift {
                                window.focus_prev();
                            } else {
                                window.focus_next();
                            }
                            cx.stop_propagation();
                        }
                    })
                    .w(px(380.0))
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
                                    .child("New Workspace"),
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
                            // Name field
                            .child(
                                div()
                                    .flex()
                                    .flex_col()
                                    .gap_1()
                                    .child(
                                        div()
                                            .text_xs()
                                            .font_weight(FontWeight::MEDIUM)
                                            .text_color(t::text_dim())
                                            .child("Name"),
                                    )
                                    .child(self.name_input.clone()),
                            )
                            // Mount folder field
                            .child(
                                div()
                                    .flex()
                                    .flex_col()
                                    .gap_1()
                                    .child(
                                        div()
                                            .text_xs()
                                            .font_weight(FontWeight::MEDIUM)
                                            .text_color(t::text_dim())
                                            .child("Mount folder"),
                                    )
                                    .child(self.path_picker.clone())
                                    .child(
                                        div()
                                            .text_xs()
                                            .text_color(t::text_faint())
                                            .pt(px(2.0))
                                            .child("Leave empty for a scratch sandbox"),
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
                            .child(self.cancel_btn.clone())
                            .child(self.create_btn.clone()),
                    ),
            )
    }
}
