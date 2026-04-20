//! `PtyBus` — the shared runtime handle to a single terminal's PTY that
//! bridges the local `TerminalView` and any remote clients.
//!
//! Lives outside GPUI entities (in a plain `Arc<RwLock<HashMap>>`) so both
//! the GPUI render thread and async tokio tasks can access it safely.
//!
//! ## Dimensions aggregation
//!
//! The PTY has a single size. Multiple clients (local desktop + zero or
//! more remote attaches) can be reading the same stream and each has its
//! own xterm sized to its viewport. If each client's resize hit the PTY
//! directly the size would thrash between them on every keystroke-driven
//! refresh, and full-screen TUIs redraw at the wrong coordinates.
//!
//! The bus aggregates client-advertised sizes and resizes the PTY to the
//! minimum across all attached clients. Every client's xterm then shows
//! the full PTY contents; clients with bigger viewports just see empty
//! margin. One physical resize per effective-min change.

use std::collections::HashMap;
use std::sync::{Arc, Mutex, RwLock};

use bytes::Bytes;
use shuru_sdk::ShellWriter;
use superhq_remote_proto::types::{TabId, WorkspaceId};

/// Abstract input side of a PTY: send keystrokes, resize.
/// Implementations bridge to whatever concrete writer the tab uses —
/// `ShellWriter` for sandboxed agent / guest-shell tabs, a
/// `portable_pty` master for the host shell.
pub trait PtyInput: Send + Sync {
    fn send_input(&self, data: &[u8]) -> Result<(), String>;
    /// Resize using (rows, cols) — matches the shuru-sdk convention.
    fn resize(&self, rows: u16, cols: u16) -> Result<(), String>;
}

impl PtyInput for ShellWriter {
    fn send_input(&self, data: &[u8]) -> Result<(), String> {
        self.send_input(data).map_err(|e| e.to_string())
    }
    fn resize(&self, rows: u16, cols: u16) -> Result<(), String> {
        self.resize(rows, cols).map_err(|e| e.to_string())
    }
}

/// Distinguishes clients in the per-client size map.
///
/// `Local` is the desktop app's `TerminalView`. `Remote(String)` is a
/// keyed remote attach (keyed by device id so two remotes from the
/// same device don't double-count). The keying also means a remote's
/// repeat `pty.attach` cleanly overwrites its previous entry instead
/// of leaking.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum ClientId {
    Local,
    Remote(String),
}

/// Runtime handle for one tab's PTY.
///
/// The `scrollback` mutex **also guards** the broadcast ordering: the
/// PTY-reader thread pushes to the ring and sends to the broadcast under
/// the same lock, so an attach handler holding the lock can snapshot +
/// subscribe atomically. See `snapshot_and_subscribe`.
#[derive(Clone)]
pub struct PtyBus {
    pub writer: Arc<dyn PtyInput>,
    pub output: tokio::sync::broadcast::Sender<Bytes>,
    pub scrollback: Arc<Mutex<crate::sandbox::pty_adapter::ScrollbackRing>>,
    sizes: Arc<Mutex<SizeAggregator>>,
}

struct SizeAggregator {
    /// Per-client advertised `(cols, rows)`.
    per_client: HashMap<ClientId, (u16, u16)>,
    /// Size we last pushed to the writer. The PTY's current effective
    /// dimensions. Seeded at 80x24 so an unattached tab reports
    /// something sane; real values land as soon as a client joins.
    effective: (u16, u16),
}

impl PtyBus {
    pub fn new(
        writer: impl PtyInput + 'static,
        output: tokio::sync::broadcast::Sender<Bytes>,
        scrollback: Arc<Mutex<crate::sandbox::pty_adapter::ScrollbackRing>>,
    ) -> Self {
        Self {
            writer: Arc::new(writer),
            output,
            scrollback,
            sizes: Arc::new(Mutex::new(SizeAggregator {
                per_client: HashMap::new(),
                effective: (80, 24),
            })),
        }
    }

    /// Atomically capture the current scrollback bytes AND subscribe to
    /// future live output. Any byte the PTY emits after this call goes
    /// to the returned receiver; everything before is in the snapshot.
    /// No duplicates, no gaps.
    pub fn snapshot_and_subscribe(
        &self,
    ) -> (Vec<u8>, tokio::sync::broadcast::Receiver<Bytes>) {
        let sb = self.scrollback.lock();
        let bytes = sb
            .as_ref()
            .map(|s| s.snapshot())
            .unwrap_or_default();
        let sub = self.output.subscribe();
        drop(sb);
        (bytes, sub)
    }

    /// Record the most recent size advertised by `client` and, if the
    /// effective min across all attached clients changed, push a single
    /// resize down to the PTY. Returns the effective size (useful for
    /// `pty.attach` responses so the caller can letterbox its xterm).
    pub fn report_client_size(
        &self,
        client: ClientId,
        cols: u16,
        rows: u16,
    ) -> (u16, u16) {
        self.update_with(|agg| {
            agg.per_client.insert(client, (cols, rows));
        })
    }

    /// Remove `client` from the aggregator. Used when a remote detaches
    /// or its data stream closes. If no clients remain the effective
    /// size is kept (don't downsize a live PTY to 0) so scrollback
    /// rendering stays consistent for the next attacher.
    pub fn release_client(&self, client: &ClientId) -> (u16, u16) {
        self.update_with(|agg| {
            agg.per_client.remove(client);
        })
    }

    fn update_with<F: FnOnce(&mut SizeAggregator)>(&self, f: F) -> (u16, u16) {
        let mut agg = match self.sizes.lock() {
            Ok(g) => g,
            Err(_) => return (80, 24),
        };
        f(&mut agg);
        let new_effective = min_of(&agg.per_client).unwrap_or(agg.effective);
        if new_effective != agg.effective {
            agg.effective = new_effective;
            let (cols, rows) = new_effective;
            let _ = self.writer.resize(rows, cols);
        }
        agg.effective
    }

    pub fn current_dimensions(&self) -> (u16, u16) {
        self.sizes.lock().map(|g| g.effective).unwrap_or((80, 24))
    }
}

fn min_of(m: &HashMap<ClientId, (u16, u16)>) -> Option<(u16, u16)> {
    let mut iter = m.values();
    let first = *iter.next()?;
    Some(iter.fold(first, |(c, r), &(cc, rr)| (c.min(cc), r.min(rr))))
}

/// Shared map keyed by (workspace_id, tab_id).
pub type PtyMap = Arc<RwLock<HashMap<(WorkspaceId, TabId), PtyBus>>>;

pub fn new_pty_map() -> PtyMap {
    Arc::new(RwLock::new(HashMap::new()))
}
