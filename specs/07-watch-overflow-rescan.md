# 07 — Handle inotify queue overflow with rescan

## Problem

During bulk file operations (git clone, npm install), the inotify kernel
queue overflows (default limit: 16384 events). The `notify` crate emits
`Flag::Rescan` but the guest ignores it. Events are silently lost, so the
review panel shows an incomplete file list.

## How other projects handle this

- **Watchman**: on `IN_Q_OVERFLOW`, schedules a full directory recrawl
- **notify crate**: emits `Event` with `Flag::Rescan`, consumer must handle it
- **VSCode/parcel-watcher**: ignores overflow entirely (known gap)

The standard approach: event-driven watching for the normal case, full
filesystem walk as the recovery path when overflow is detected.

## Current flow

```
Guest (shuru-guest/main.rs):
  notify::recommended_watcher → callback sends events over mpsc
  loop: recv events → filter gitignore → write WatchEvent frames over vsock

SDK (shuru-sdk/lib.rs):
  reader thread reads vsock frames → sends WatchEvent over tokio mpsc

SuperHQ (watcher.rs):
  recv WatchEvent → filter gitignore → check existence → send DiffResult
```

### What's missing

1. Guest doesn't check `event.need_rescan()` from the notify crate
2. No `OVERFLOW` event type in the protocol
3. SuperHQ has no rescan logic

## Changes

### A. Protocol: add OVERFLOW watch kind

```rust
// shuru-proto/src/lib.rs
pub mod watch_kind {
    pub const CREATE: u8 = 0x01;
    pub const MODIFY: u8 = 0x02;
    pub const DELETE: u8 = 0x03;
    pub const OVERFLOW: u8 = 0xFF;
}
```

### B. Guest: detect overflow, send OVERFLOW event

In `handle_watch`, check `event.need_rescan()`:

```rust
// After receiving notify::Event:
if event.need_rescan() {
    let evt = WatchEvent { kind: watch_kind::OVERFLOW, path: String::new() };
    frame::write_frame(&mut writer, frame::WATCH_EVENT, &evt.encode());
}
```

### C. SDK: pass OVERFLOW through

No change needed — WatchEvent already carries `kind: u8`, and the SDK
forwards all events. SuperHQ checks the kind.

### D. SuperHQ watcher: on OVERFLOW, do a full scan

When receiving a WatchEvent with `kind == OVERFLOW`:
1. Run `find /workspace -type f` in sandbox (single shell call)
2. Diff against `cached_files` to find new/removed files
3. Send a DiffResult with the delta

This is the same recovery path Watchman uses.

## Implementation order

1. Add `OVERFLOW` constant to shuru-proto
2. Detect and send overflow in shuru-guest
3. Handle overflow in SuperHQ watcher with a full scan
