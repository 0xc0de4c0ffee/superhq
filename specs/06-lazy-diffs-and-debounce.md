# 06 — Lazy diffs, watcher debounce, and file list cap

## Problem

A `git clone` or `npm install` creates thousands of files. The watcher reads
and diffs every file eagerly, sending massive results to the UI. The app
freezes because:

1. Watcher reads 2× file contents (host + sandbox) per changed path
2. Watcher computes line-level diffs for every file
3. UI rebuilds the entire file list with all diff stats
4. No debounce — events fire as fast as the filesystem produces them

## How VSCode handles this

- `git status` provides file list + status (no content reading)
- Diffs computed lazily when user opens a file, in a web worker
- 1s debounce + 5s cooldown between status runs
- Virtual scrolled list (only ~30 visible DOM nodes)
- Hard cap at 10,000 files — stops auto-refresh beyond that

## Proposed changes

### A. Watcher: paths + status only (no content reading)

The watcher currently does this per dirty path:
```
read_host_file → read_sandbox_file → compare → compute_file_diff
```

Change to:
```
check_host_exists → check_sandbox_exists → determine status (A/M/D)
```

For mounted workspaces, a file that exists on both sides is Modified.
For scratch sandboxes, everything is Added. Deleted = host exists,
sandbox doesn't. This requires no content reading.

New DiffResult shape — no diffs, just file metadata:
```rust
pub struct DiffResult {
    pub dirty_paths: HashSet<String>,
    pub updated_files: HashMap<String, ChangedFile>,
    pub removed_paths: HashSet<String>,
}
```

`ChangedFile` drops `diff_stats` (unknown until expanded):
```rust
pub struct ChangedFile {
    pub path: String,
    pub status: FileStatus,
}
```

### B. Debounce watcher events

After draining `try_recv`, sleep 500ms before processing. This collapses
rapid bursts (clone, install) into fewer batches.

```rust
loop {
    let event = watch.receiver.recv().await; // block for first event
    tokio::time::sleep(Duration::from_millis(500)).await; // debounce
    // drain anything that arrived during the sleep
    while let Ok(ev) = watch.receiver.try_recv() { ... }
    // process batch
}
```

### C. Diff on expand (lazy)

Move diff computation to `ChangesTab::render()`, triggered when the user
expands a file. Run it off the UI thread (same pattern as highlights).

```rust
// In render(), when expanded and no cached diff:
if expanded.get() && !self.file_diffs.contains_key(&file.path) {
    // spawn background diff computation
    spawn_diff(sandbox, host_mount_path, &file.path, cx);
}
// Render without diff stats until ready (show "computing..." or just omit stats)
```

The diff computation needs sandbox + host_mount_path, which live on
SidePanel. Pass them down or store references in ChangesTab.

### D. File list cap

If `changed_files.len() > 500`, show the first 500 with a
"and N more files..." footer. No diff expansion available beyond the cap.

## Changes by file

### watcher.rs
- Remove `read_host_file`, `read_sandbox_file`, `compute_file_diff` calls
- Replace with existence checks (sandbox: `read_file` with size 0 check
  or `exec_in("test -f ...")`, host: `tokio::fs::metadata`)
- Remove `cached_diffs` HashMap entirely
- Remove `last_content` tracking (no content to compare)
- Add 500ms debounce sleep after first event
- DiffResult loses `updated_diffs`
- For detecting Modified vs unchanged in mounted workspaces: compare
  file modification times or sizes (host metadata vs sandbox metadata)
  instead of reading full contents. Fall back to reporting all
  host+sandbox files as Modified — the actual diff on expand will show
  if there are real changes.

### changes_tab.rs
- Remove `file_diffs` from struct (diffs are per-expand, not per-update)
  Actually keep `file_diffs` as a cache, but populate it on expand,
  not from watcher results.
- `apply_results` no longer receives diffs — just file list updates
- `ChangedFile` drops `diff_stats` (or makes it Optional, populated on expand)
- Add `sandbox: Option<Arc<AsyncSandbox>>` and `host_mount_path` refs
  to ChangesTab so it can spawn diff computation
- Diff computation on expand: spawn background thread, insert result
  into `file_diffs` cache, `cx.notify()`
- Add `diffing: HashSet<String>` to track in-flight computations
  (same pattern as `highlighting`)
- Show file count in header instead of +/- stats (until diffs are computed)

### diff_view.rs
- `render_file_section` handles `diff: None` gracefully (show header
  without stats, show "Tap to load diff" or spinner on expand)

### mod.rs
- Pass sandbox + host_mount_path to ChangesTab or make them accessible

## Implementation order

1. Debounce (watcher.rs) — easiest, immediate win for bursts
2. Remove eager diffing from watcher — send paths + status only
3. Lazy diff on expand — move diff computation to UI-triggered background task
4. File list cap — simple UI change

## Not in scope

- Virtual scrolled file list (spec 07 — proper fix for 10k+ files)
- Using git status instead of file watcher (would require git in sandbox)
