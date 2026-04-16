//! Host-side `.git` directory watcher used by the sidebar to flip workspace
//! rows between "folder" and "git repo" states in response to `git init`,
//! clones, or repo deletions, without polling.

use gpui::{Context, Task, WeakEntity};
use notify::{EventKind, RecommendedWatcher, RecursiveMode, Watcher};
use std::path::PathBuf;
use std::time::Duration;

/// One mount path plus whether it currently contains a `.git` directory.
/// Used as the rebuild key: flipping `has_git` means we need a new watcher
/// on the newly-created (or newly-removed) `.git` dir.
pub type WatchEntry = (PathBuf, bool);

/// Owns a `notify` watcher plus the async drainer that coalesces bursts and
/// invokes a caller-provided "git state changed" handler on the UI thread.
pub struct GitDirWatcher<V: 'static> {
    _watcher: RecommendedWatcher,
    _task: Task<()>,
    entries: Vec<WatchEntry>,
    _marker: std::marker::PhantomData<V>,
}

impl<V: 'static> GitDirWatcher<V> {
    pub fn entries(&self) -> &[WatchEntry] {
        &self.entries
    }

    /// Install watchers on each mount path (NonRecursive — for `.git`
    /// appear/disappear), on each existing `.git` dir (NonRecursive — for
    /// `config` / `HEAD` / `refs/*` edits), and on the avatar cache dir.
    /// Runs `on_change` when any relevant event fires, debounced 80ms.
    pub fn new<F>(
        entries: Vec<WatchEntry>,
        entity: WeakEntity<V>,
        on_change: F,
        cx: &mut Context<V>,
    ) -> Option<Self>
    where
        F: Fn(&mut V, &mut Context<V>) + 'static,
    {
        if entries.is_empty() {
            return None;
        }

        let avatar_dir = crate::avatar_cache::cache_dir();
        std::fs::create_dir_all(&avatar_dir).ok();
        let avatar_dir_for_filter = avatar_dir.clone();

        let (tx, rx) = flume::unbounded::<()>();
        let mut watcher = match RecommendedWatcher::new(
            move |res: notify::Result<notify::Event>| {
                let Ok(event) = res else { return; };
                let relevant = matches!(
                    event.kind,
                    EventKind::Create(_) | EventKind::Remove(_) | EventKind::Modify(_),
                ) && event.paths.iter().any(|p| {
                    let in_git = p.components().any(|c| c.as_os_str() == crate::git::GIT_DIR);
                    let in_avatars = p.starts_with(&avatar_dir_for_filter);
                    in_git || in_avatars
                });
                if relevant {
                    let _ = tx.send(());
                }
            },
            notify::Config::default(),
        ) {
            Ok(w) => w,
            Err(e) => {
                eprintln!("git watcher init failed: {e}");
                return None;
            }
        };

        for (path, has_git) in &entries {
            if let Err(e) = watcher.watch(path, RecursiveMode::NonRecursive) {
                eprintln!("git watcher watch failed for {}: {e}", path.display());
            }
            if *has_git {
                let git_dir = path.join(crate::git::GIT_DIR);
                if let Err(e) = watcher.watch(&git_dir, RecursiveMode::NonRecursive) {
                    eprintln!("git watcher watch failed for {}: {e}", git_dir.display());
                }
            }
        }
        if let Err(e) = watcher.watch(&avatar_dir, RecursiveMode::NonRecursive) {
            eprintln!("avatar cache watch failed: {e}");
        }

        let task = cx.spawn(async move |_, cx| {
            while let Ok(()) = rx.recv_async().await {
                // One create → multiple FS events on macOS; settle before refreshing.
                gpui::Timer::after(Duration::from_millis(80)).await;
                while rx.try_recv().is_ok() {}
                let _ = cx.update(|cx| {
                    entity.update(cx, |view, cx| on_change(view, cx)).ok();
                });
            }
        });

        Some(Self {
            _watcher: watcher,
            _task: task,
            entries,
            _marker: std::marker::PhantomData,
        })
    }
}
