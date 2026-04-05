use super::diff_engine::{DiffStats, FileDiff};
use super::diff_view::{self, DiffScrollState};
use crate::ui::theme as t;
use gpui::*;
use gpui::prelude::FluentBuilder as _;
use gpui_component::scroll::ScrollableElement as _;
use shuru_sdk::AsyncSandbox;
use std::cell::Cell;
use std::collections::{HashMap, HashSet};
use std::rc::Rc;
use std::sync::Arc;

pub struct ChangesTab {
    pub changed_files: Vec<ChangedFile>,
    file_diffs: HashMap<String, FileDiff>,
    scroll_states: HashMap<String, DiffScrollState>,
    expanded: HashMap<String, Rc<Cell<bool>>>,
    highlight_cache: HashMap<String, diff_view::HighlightCache>,
    /// Files with pending discard/keep — filtered from bridge results until confirmed gone.
    suppressed: HashSet<String>,
}

#[derive(Clone)]
pub struct ChangedFile {
    pub path: String,
    pub status: FileStatus,
    pub diff_stats: Option<DiffStats>,
}

#[derive(Clone, Copy, PartialEq)]
pub enum FileStatus {
    Modified,
    Added,
    Deleted,
}

impl FileStatus {
    pub fn label(&self) -> &'static str {
        match self {
            Self::Modified => "M",
            Self::Added => "A",
            Self::Deleted => "D",
        }
    }

    pub fn color(&self) -> Rgba {
        match self {
            Self::Modified => t::status_modified(),
            Self::Added => t::status_added(),
            Self::Deleted => t::status_deleted(),
        }
    }
}

impl ChangesTab {
    pub fn new() -> Self {
        Self {
            changed_files: Vec::new(),
            file_diffs: HashMap::new(),
            scroll_states: HashMap::new(),
            expanded: HashMap::new(),
            highlight_cache: HashMap::new(),
            suppressed: HashSet::new(),
        }
    }

    pub fn clear(&mut self) {
        self.changed_files.clear();
        self.file_diffs.clear();
        self.scroll_states.clear();
        self.expanded.clear();
        self.highlight_cache.clear();
        self.suppressed.clear();
    }

    pub fn snapshot(&self) -> ChangesSnapshot {
        ChangesSnapshot { changed_files: self.changed_files.clone() }
    }

    pub fn restore(&mut self, snap: ChangesSnapshot) {
        self.changed_files = snap.changed_files;
    }

    pub fn apply_results(
        &mut self,
        files: Vec<ChangedFile>,
        diffs: HashMap<String, FileDiff>,
        dirty_paths: &HashSet<String>,
    ) {
        // Lift suppression for any path the bridge has now reported on.
        // Success: file no longer in `files` → gone from UI.
        // Failure: file still in `files` → reappears (operation didn't take effect).
        for path in dirty_paths {
            self.suppressed.remove(path);
        }

        self.changed_files = files.into_iter()
            .filter(|f| !self.suppressed.contains(&f.path))
            .collect();
        self.file_diffs = diffs.into_iter()
            .filter(|(k, _)| !self.suppressed.contains(k))
            .collect();

        for path in dirty_paths {
            self.highlight_cache.remove(path);
        }
    }

    fn suppress_file(&mut self, path: &str) {
        self.suppressed.insert(path.to_string());
        self.changed_files.retain(|f| f.path != path);
        self.file_diffs.remove(path);
        self.highlight_cache.remove(path);
    }

    fn suppress_all(&mut self) {
        self.suppressed.extend(self.changed_files.iter().map(|f| f.path.clone()));
        self.changed_files.clear();
        self.file_diffs.clear();
        self.highlight_cache.clear();
    }

    pub fn render(&mut self, cx: &mut Context<super::SidePanel>) -> AnyElement {
        let has_changes = !self.changed_files.is_empty();
        let mut content = div().size_full().flex().flex_col();

        if !has_changes {
            return content
                .child(div().px_3().py_4().text_xs().text_color(t::text_faint()).child("No changes"))
                .into_any_element();
        }

        let total_add: usize = self.changed_files.iter()
            .filter_map(|f| f.diff_stats.as_ref()).map(|s| s.additions).sum();
        let total_del: usize = self.changed_files.iter()
            .filter_map(|f| f.diff_stats.as_ref()).map(|s| s.deletions).sum();
        let file_count = self.changed_files.len();

        content = content.child(
            div().flex_shrink_0().px_3().py_1p5()
                .flex().items_center().justify_between()
                .border_b_1().border_color(t::border())
                .child(
                    div().flex().items_center().gap_1p5()
                        .child(div().text_xs().text_color(t::text_dim()).child(format!(
                            "{} file{}", file_count, if file_count == 1 { "" } else { "s" }
                        )))
                        .when(total_add > 0, |el: Div| el.child(
                            div().text_xs().text_color(t::diff_add_text()).child(format!("+{}", total_add))
                        ))
                        .when(total_del > 0, |el: Div| el.child(
                            div().text_xs().text_color(t::diff_del_text()).child(format!("-{}", total_del))
                        )),
                )
                .child(
                    div().flex().items_center().gap_1()
                        .child(
                            div().id("discard-all-btn").px_2().py(px(3.0)).rounded(px(4.0))
                                .text_xs().font_weight(FontWeight::MEDIUM)
                                .text_color(t::text_dim()).bg(t::bg_elevated())
                                .cursor_pointer()
                                .hover(|s: StyleRefinement| s.bg(t::bg_hover()).text_color(t::diff_del_text()))
                                .on_click(cx.listener(|panel, _: &ClickEvent, _window, cx| {
                                    if let (Some(sb), Some(handle)) = (&panel.sandbox, &panel.tokio_handle) {
                                        let paths: Vec<String> = panel.changes_tab.changed_files.iter()
                                            .map(|f| f.path.clone()).collect();
                                        panel.changes_tab.suppress_all();
                                        cx.notify();
                                        let sb = sb.clone();
                                        handle.spawn(async move {
                                            for p in &paths {
                                                let full = format!("/workspace/{}", p);
                                                let _ = sb.discard_overlay(&full).await;
                                            }
                                        });
                                    }
                                }))
                                .child("Discard All"),
                        )
                        .child(
                            div().id("keep-all-btn").px_2().py(px(3.0)).rounded(px(4.0))
                                .text_xs().font_weight(FontWeight::MEDIUM)
                                .text_color(t::text_secondary()).bg(t::bg_active())
                                .cursor_pointer()
                                .hover(|s: StyleRefinement| s.bg(t::bg_selected()))
                                .on_click(cx.listener(|panel, _: &ClickEvent, _window, cx| {
                                    if let (Some(sb), Some(handle), Some(host)) =
                                        (&panel.sandbox, &panel.tokio_handle, &panel.host_mount_path)
                                    {
                                        let items: Vec<(String, FileStatus)> = panel.changes_tab.changed_files.iter()
                                            .map(|f| (f.path.clone(), f.status)).collect();
                                        panel.changes_tab.suppress_all();
                                        cx.notify();
                                        let sb = sb.clone();
                                        let host = host.clone();
                                        handle.spawn(async move {
                                            for (path, status) in &items {
                                                keep_one(path, *status, &host, &sb).await;
                                            }
                                        });
                                    }
                                }))
                                .child("Keep All"),
                        ),
                ),
        );

        if !self.file_diffs.is_empty() {
            let mut scroll = div().flex_grow().flex().flex_col().overflow_y_scrollbar().pt_1();

            for file in &self.changed_files {
                if !self.highlight_cache.contains_key(&file.path) {
                    if let Some(diff) = self.file_diffs.get(&file.path) {
                        let runs = diff_view::compute_highlights(&file.path, &diff.hunks);
                        self.highlight_cache.insert(file.path.clone(), runs);
                    }
                }
            }

            for file in &self.changed_files {
                let diff = self.file_diffs.get(&file.path);
                let ss = self.scroll_states.entry(file.path.clone()).or_insert_with(DiffScrollState::new);
                let expanded = self.expanded.entry(file.path.clone()).or_insert_with(|| Rc::new(Cell::new(false)));
                let highlights = self.highlight_cache.get(&file.path);
                let path = file.path.clone();
                let status = file.status;

                let on_keep: Option<Box<dyn Fn(&ClickEvent, &mut Window, &mut App) + 'static>> = {
                    let path = path.clone();
                    Some(Box::new(cx.listener(move |panel, _: &ClickEvent, _window, cx| {
                        if let (Some(sb), Some(handle), Some(host)) =
                            (&panel.sandbox, &panel.tokio_handle, &panel.host_mount_path)
                        {
                            panel.changes_tab.suppress_file(&path);
                            cx.notify();
                            let sb = sb.clone();
                            let host = host.clone();
                            let p = path.clone();
                            handle.spawn(async move { keep_one(&p, status, &host, &sb).await });
                        }
                    })))
                };

                let on_discard: Option<Box<dyn Fn(&ClickEvent, &mut Window, &mut App) + 'static>> = {
                    let path = path.clone();
                    Some(Box::new(cx.listener(move |panel, _: &ClickEvent, _window, cx| {
                        if let (Some(sb), Some(handle)) = (&panel.sandbox, &panel.tokio_handle) {
                            panel.changes_tab.suppress_file(&path);
                            cx.notify();
                            let sb = sb.clone();
                            let p = path.clone();
                            handle.spawn(async move {
                                let full = format!("/workspace/{}", p);
                                let _ = sb.discard_overlay(&full).await;
                            });
                        }
                    })))
                };

                scroll = scroll.child(diff_view::render_file_section(
                    &file.path, file.status,
                    &file.diff_stats.clone().unwrap_or_default(),
                    diff, ss, expanded, highlights, on_keep, on_discard,
                ));
            }

            content = content.child(scroll);
        }

        content.into_any_element()
    }
}

async fn keep_one(path: &str, status: FileStatus, host: &str, sandbox: &Arc<AsyncSandbox>) {
    if status == FileStatus::Deleted {
        let hp = format!("{}/{}", host, path);
        let _ = tokio::fs::remove_file(&hp).await;
    } else {
        let _ = super::diff_engine::copy_to_host(path, host, sandbox).await;
    }
}

pub struct ChangesSnapshot {
    pub changed_files: Vec<ChangedFile>,
}
