use gpui::*;
use gpui::prelude::FluentBuilder as _;
use gpui_component::input::{Input, InputState};
use gpui_component::WindowExt as _;
use gpui_component::{v_flex, Sizable as _};

use crate::db::Database;
use crate::ui::theme as t;
use std::sync::Arc;

actions!(superhq, [ToggleCommandPalette]);

#[derive(Clone)]
#[allow(dead_code)]
struct PaletteItem {
    label: String,
    detail: String,
    action: PaletteAction,
}

#[derive(Clone)]
#[allow(dead_code)]
enum PaletteAction {
    SwitchWorkspace(i64),
    NewWorkspace,
}

#[allow(dead_code)]
pub(crate) struct CommandPalette {
    db: Arc<Database>,
    query_input: Entity<InputState>,
    items: Vec<PaletteItem>,
    filtered_items: Vec<PaletteItem>,
    selected_index: usize,
    on_switch_workspace: Box<dyn Fn(i64, &mut Window, &mut App) + 'static>,
    on_new_workspace: Box<dyn Fn(&mut Window, &mut App) + 'static>,
}

impl CommandPalette {
    pub fn open(
        db: Arc<Database>,
        on_switch_workspace: impl Fn(i64, &mut Window, &mut App) + 'static,
        on_new_workspace: impl Fn(&mut Window, &mut App) + 'static,
        window: &mut Window,
        cx: &mut App,
    ) {
        let on_switch = std::rc::Rc::new(on_switch_workspace);
        let on_new = std::rc::Rc::new(on_new_workspace);

        let view = cx.new(|cx| {
            let query_input = cx.new(|cx| {
                let mut state = InputState::new(window, cx);
                state.set_placeholder("Search workspaces or type a command...", window, cx);
                state.focus(window, cx);
                state
            });

            let mut items = Vec::new();

            // Add workspace items
            if let Ok(workspaces) = db.list_workspaces() {
                for ws in workspaces {
                    let detail = match (&ws.mount_path, ws.is_git_repo) {
                        (Some(path), true) => {
                            let name = path.split('/').last().unwrap_or(path);
                            name.to_string()
                        }
                        (Some(path), false) => format!("{} · no git", path.split('/').last().unwrap_or(path)),
                        (None, _) => "scratch sandbox".to_string(),
                    };
                    items.push(PaletteItem {
                        label: ws.name.clone(),
                        detail,
                        action: PaletteAction::SwitchWorkspace(ws.id),
                    });
                }
            }

            // Add command items
            items.push(PaletteItem {
                label: "New Workspace".to_string(),
                detail: "Create a new workspace".to_string(),
                action: PaletteAction::NewWorkspace,
            });

            let filtered_items = items.clone();

            CommandPalette {
                db,
                query_input,
                items,
                filtered_items,
                selected_index: 0,
                on_switch_workspace: Box::new({
                    let on_switch = on_switch.clone();
                    move |id, w, cx| on_switch(id, w, cx)
                }),
                on_new_workspace: Box::new({
                    let on_new = on_new.clone();
                    move |w, cx| on_new(w, cx)
                }),
            }
        });

        let view_for_child = view.clone();

        window.open_dialog(cx, move |dialog, _window, _cx| {
            dialog
                .title("Command Palette")
                .width(px(500.))
                .close_button(true)
                .child(view_for_child.clone())
        });
    }

    fn filter_items(&mut self, cx: &App) {
        let query = self.query_input.read(cx).value().to_string().to_lowercase();
        if query.is_empty() {
            self.filtered_items = self.items.clone();
        } else {
            self.filtered_items = self
                .items
                .iter()
                .filter(|item| {
                    item.label.to_lowercase().contains(&query)
                        || item.detail.to_lowercase().contains(&query)
                })
                .cloned()
                .collect();
        }
        self.selected_index = 0;
    }

    #[allow(dead_code)]
    fn execute_selected(&self, window: &mut Window, cx: &mut App) {
        if let Some(item) = self.filtered_items.get(self.selected_index) {
            match &item.action {
                PaletteAction::SwitchWorkspace(id) => {
                    (self.on_switch_workspace)(*id, window, cx);
                }
                PaletteAction::NewWorkspace => {
                    (self.on_new_workspace)(window, cx);
                }
            }
            window.close_dialog(cx);
        }
    }
}

impl Render for CommandPalette {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        // Re-filter on each render (picks up input changes)
        self.filter_items(cx);

        v_flex()
            .gap_1()
            .child(Input::new(&self.query_input).small())
            .child(
                div().flex().flex_col().max_h(px(300.0)).children(
                    self.filtered_items
                        .iter()
                        .enumerate()
                        .map(|(i, item)| {
                            let is_selected = i == self.selected_index;
                            let label = item.label.clone();
                            let detail = item.detail.clone();

                            div()
                                .px_2()
                                .py_1()
                                .rounded_md()
                                .flex()
                                .items_center()
                                .justify_between()
                                .when(is_selected, |s| s.bg(t::bg_selected()))
                                .hover(|s| s.bg(t::bg_active()))
                                .child(
                                    div()
                                        .text_sm()
                                        .text_color(if is_selected {
                                            t::text_primary()
                                        } else {
                                            t::text_tertiary()
                                        })
                                        .child(label),
                                )
                                .child(
                                    div()
                                        .text_xs()
                                        .text_color(t::text_ghost())
                                        .child(detail),
                                )
                        }),
                ),
            )
    }
}
