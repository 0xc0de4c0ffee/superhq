mod agents;
mod assets;
mod db;
mod oauth;
mod sandbox;
mod ui;

use anyhow::Result;
use db::Database;
use gpui::*;
use gpui::prelude::FluentBuilder as _;
use gpui_component::resizable::{h_resizable, resizable_panel};
use gpui_component::{Root, Theme, ThemeMode};
use std::sync::Arc;
use ui::dialogs::forward_port::ForwardPortDialog;
use ui::dialogs::new_workspace::NewWorkspaceDialog;

actions!(superhq, [NewWorkspaceAction, OpenSettingsAction]);
use ui::components::Toast;
use ui::review::SidePanel;
use ui::settings::SettingsPanel;
use ui::sidebar::workspace_list::WorkspaceListView;
use ui::terminal::TerminalPanel;

/// Root application view: 3-panel layout (sidebar | terminal | review).
struct AppView {
    db: Arc<Database>,
    sidebar: Entity<WorkspaceListView>,
    terminal: Entity<TerminalPanel>,
    review: Entity<SidePanel>,
    toast: Entity<Toast>,
    dialog: Option<Entity<NewWorkspaceDialog>>,
    port_dialog: Option<Entity<ForwardPortDialog>>,
    settings: Option<Entity<SettingsPanel>>,
}

impl AppView {
    fn new(db: Arc<Database>, cx: &mut Context<Self>) -> Self {
        let this_for_settings = cx.entity().downgrade();
        let this_for_ports = cx.entity().downgrade();
        let terminal = cx.new(|_cx| {
            let mut panel = TerminalPanel::new(db.clone());
            let app = this_for_settings.clone();
            panel.set_on_open_settings(move |window, cx| {
                let _ = app.update(cx, |this: &mut Self, cx| {
                    this.open_settings(window, cx);
                });
            });
            panel.set_on_open_port_dialog(move |ws_id, window, cx| {
                let _ = this_for_ports.update(cx, |this: &mut Self, cx| {
                    this.open_forward_port_dialog(ws_id, window, cx);
                });
            });
            panel
        });
        let review = cx.new(|_| SidePanel::new());
        // Wire review panel into terminal so it gets sandbox-ready notifications
        terminal.update(cx, |panel, _| {
            panel.set_side_panel(review.clone());
        });
        let this = cx.entity().downgrade();
        let this2 = cx.entity().downgrade();
        let sidebar = cx.new(|cx| {
            WorkspaceListView::new(
                db.clone(),
                terminal.clone(),
                review.clone(),
                move |window, cx| {
                    this.update(cx, |app, cx| {
                        app.open_new_workspace_dialog(window, cx);
                    }).ok();
                },
                move |cx| {
                    this2.update(cx, |app, cx| {
                        app.settings = None;
                        cx.notify();
                    }).ok();
                },
                cx,
            )
        });
        let toast = cx.new(|_| Toast::new());
        Self {
            db,
            sidebar,
            terminal,
            review,
            toast,
            dialog: None,
            port_dialog: None,
            settings: None,
        }
    }

    fn open_settings(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.settings.is_some() {
            return;
        }
        self.sidebar.update(cx, |view, cx| view.clear_active(cx));
        let this = cx.entity().downgrade();
        let terminal = self.terminal.clone();
        let db = self.db.clone();
        let toast = self.toast.clone();
        let view = cx.new(|cx| {
            SettingsPanel::new(
                db,
                toast,
                move |_window, cx| {
                    this.update(cx, |app, cx| {
                        app.settings = None;
                        cx.notify();
                    })
                    .ok();
                    // Notify terminal panel to re-check missing secrets
                    let _ = terminal.update(cx, |panel, cx| {
                        panel.on_settings_closed(cx);
                    });
                },
                window,
                cx,
            )
        });
        self.settings = Some(view);
        cx.notify();
    }

    fn open_forward_port_dialog(&mut self, ws_id: i64, window: &mut Window, cx: &mut Context<Self>) {
        let db = self.db.clone();
        let this = cx.entity().downgrade();

        let view = cx.new(|cx| {
            ForwardPortDialog::new(
                db,
                ws_id,
                move |_window, cx| {
                    this.update(cx, |app, cx| {
                        app.port_dialog = None;
                        cx.notify();
                    }).ok();
                },
                window,
                cx,
            )
        });
        self.port_dialog = Some(view);
        cx.notify();
    }

    fn open_new_workspace_dialog(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let db = self.db.clone();
        let sidebar = self.sidebar.clone();
        let this = cx.entity().downgrade();
        let this2 = cx.entity().downgrade();

        let view = cx.new(|cx| {
            NewWorkspaceDialog::new(
                db,
                move |_window, cx| {
                    sidebar.update(cx, |view: &mut WorkspaceListView, cx| view.refresh(cx));
                    this.update(cx, |app, cx| {
                        app.dialog = None;
                        cx.notify();
                    }).ok();
                },
                move |_window, cx| {
                    this2.update(cx, |app, cx| {
                        app.dialog = None;
                        cx.notify();
                    }).ok();
                },
                window,
                cx,
            )
        });
        self.dialog = Some(view);
        cx.notify();
    }
}

impl Render for AppView {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        use ui::theme as t;
        let show_review = self.review.read(cx).visible;
        let show_settings = self.settings.is_some();

        div()
            .id("app-root")
            .size_full()
            .bg(t::bg_base())
            .on_action(cx.listener(|this, _: &NewWorkspaceAction, window, cx| {
                this.open_new_workspace_dialog(window, cx);
            }))
            .on_action(cx.listener(|this, _: &OpenSettingsAction, window, cx| {
                this.open_settings(window, cx);
            }))
            .child(
                h_resizable("outer-layout")
                    .child(
                        resizable_panel()
                            .size(px(240.0))
                            .size_range(px(180.0)..px(400.0))
                            .child(
                                div()
                                    .id("sidebar-container")
                                    .size_full()
                                    .bg(t::bg_surface())
                                    .border_r_1()
                                    .border_color(t::border_strong())
                                    .flex()
                                    .flex_col()
                                    .child(
                                        div().flex_grow().child(self.sidebar.clone()),
                                    )
                                    // Gear button at bottom of sidebar
                                    .child(
                                        div()
                                            .border_t_1()
                                            .border_color(t::border_subtle())
                                            .child(
                                                div()
                                                    .id("settings-btn")
                                                    .px_2p5()
                                                    .py_2()
                                                    .cursor_pointer()
                                                    .text_xs()
                                                    .text_color(if show_settings {
                                                        t::text_tertiary()
                                                    } else {
                                                        t::text_dim()
                                                    })
                                                    .when(show_settings, |el: Stateful<Div>| {
                                                        el.bg(t::bg_selected())
                                                    })
                                                    .hover(|s: StyleRefinement| {
                                                        s.bg(t::border_subtle())
                                                            .text_color(t::text_tertiary())
                                                    })
                                                    .on_click(
                                                        cx.listener(|this, _, window, cx| {
                                                            if this.settings.is_some() {
                                                                this.settings = None;
                                                                cx.notify();
                                                            } else {
                                                                this.open_settings(window, cx);
                                                            }
                                                        }),
                                                    )
                                                    .child("Settings"),
                                            ),
                                    ),
                            ),
                    )
                    .child(
                        h_resizable("inner-layout")
                            .child(
                                resizable_panel().child(
                                    div()
                                        .size_full()
                                        .bg(t::bg_base())
                                        .child(self.terminal.clone()),
                                ),
                            )
                            .child(
                                resizable_panel()
                                    .visible(show_review)
                                    .size(px(340.0))
                                    .size_range(px(260.0)..px(500.0))
                                    .child(
                                        div()
                                            .size_full()
                                            .bg(t::bg_surface())
                                            .border_l_1()
                                            .border_color(t::border_strong())
                                            .child(self.review.clone()),
                                    ),
                            ),
                    ),
            )
            .children(self.settings.as_ref().map(|s| s.clone()))
            .children(self.dialog.as_ref().map(|d| d.clone()))
            .children(self.port_dialog.as_ref().map(|d| d.clone()))
            .children(Root::render_dialog_layer(window, cx))
            .children(Root::render_sheet_layer(window, cx))
            .child(self.toast.clone())
    }
}

fn main() -> Result<()> {
    let db = Arc::new(Database::open()?);

    let app = Application::new().with_assets(assets::Assets);

    app.run(move |cx| {
        gpui_component::init(cx);
        Theme::change(ThemeMode::Dark, None, cx);

        // Override gpui_component theme colors to match our dark palette
        {
            let theme = cx.global_mut::<Theme>();
            // Popover/menu colors — match our bg_surface (#1a1a1a) instead of #0a0a0a
            theme.popover = gpui::hsla(0.0, 0.0, 0.11, 1.0);         // ~#1c1c1c
            theme.popover_foreground = gpui::hsla(0.0, 0.0, 0.78, 1.0); // ~#c7c7c7
            theme.border = gpui::hsla(0.0, 0.0, 0.16, 1.0);          // ~#292929
            theme.accent_foreground = gpui::hsla(0.0, 0.0, 0.78, 1.0);
        }

        // Disable Tab focus-cycling only inside the terminal, so Tab reaches
        // on_key_down for shell tab-completion. Root's Tab still works in dialogs/menus.
        cx.bind_keys([
            KeyBinding::new("tab", NoAction, Some("Terminal")),
            KeyBinding::new("shift-tab", NoAction, Some("Terminal")),
        ]);

        // Our shortcuts
        cx.bind_keys([
            KeyBinding::new("cmd-n", NewWorkspaceAction, None),
            KeyBinding::new("cmd-,", OpenSettingsAction, None),
            // Tab navigation
            KeyBinding::new("cmd-w", ui::terminal::CloseActiveTab, Some("Terminal")),
            KeyBinding::new("cmd-shift-]", ui::terminal::NextTab, Some("Terminal")),
            KeyBinding::new("cmd-shift-[", ui::terminal::PrevTab, Some("Terminal")),
        ]);


        let db = db.clone();
        cx.open_window(
            WindowOptions {
                window_bounds: Some(WindowBounds::Windowed(Bounds::centered(
                    None,
                    size(px(1400.0), px(900.0)),
                    cx,
                ))),
                titlebar: Some(TitlebarOptions {
                    title: Some("superhq".into()),
                    ..Default::default()
                }),
                ..Default::default()
            },
            |window, cx| {
                let view = cx.new(|cx| AppView::new(db, cx));
                cx.new(|cx| Root::new(view, window, cx))
            },
        )
        .expect("Failed to open window");
    });

    Ok(())
}
