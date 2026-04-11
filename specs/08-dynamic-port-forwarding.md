# 08 — Dynamic port forwarding + bind retry on fork

## Problem 1: Port forwarding on running sandbox

Port forwards are configured at boot via `SandboxConfig.ports`. When the
user adds a mapping via the ports dialog to a running sandbox, it saves to
DB but the running VM never picks it up. There's no API to add port
forwards dynamically.

### Current flow

```
SandboxConfig.ports → boot_vm() → sandbox.start_port_forwarding()
                                   ↑ called once, at boot
```

### Fix

Expose `start_port_forwarding` through AsyncSandbox so running sandboxes
can add forwards:

1. Add `SandboxCmd::AddPortForward { mapping, reply }` to the SDK
2. Add `AsyncSandbox::add_port_forward(mapping) -> Result<()>` 
3. In `run_vm_loop`, handle the command by calling
   `sandbox.start_port_forwarding(&[mapping])` and storing the handle
4. In SuperHQ, after the user adds a port mapping in the dialog,
   call `sandbox.add_port_forward(mapping)` on the running sandbox

## Problem 2: Port bind failure on fork

When checkpointing + forking, the old sandbox's port listener threads
may still hold the host port when the new sandbox tries to bind it.

`TcpListener::bind("127.0.0.1:PORT")` fails with EADDRINUSE.

### Fix

Two layers:

**A. SDK: SO_REUSEADDR on port forward listeners**

In `start_port_forwarding` (sandbox.rs:792), use `socket2` to set
`SO_REUSEADDR` before binding. This allows the new listener to bind
even if the old one hasn't fully closed.

```rust
use socket2::{Socket, Domain, Type, Protocol};
let socket = Socket::new(Domain::IPV4, Type::STREAM, Some(Protocol::TCP))?;
socket.set_reuse_address(true)?;
socket.bind(&addr.parse::<std::net::SocketAddr>()?.into())?;
socket.listen(128)?;
let tcp_listener: TcpListener = socket.into();
```

**B. SuperHQ: retry with backoff on bind failure**

If `add_port_forward` fails, retry up to 3 times with 500ms delay.
This handles the window where the old listener is shutting down.

## Changes

| Crate | File | Change |
|-------|------|--------|
| shuru-sdk | lib.rs | Add `SandboxCmd::AddPortForward`, `AsyncSandbox::add_port_forward()` |
| shuru-sdk | lib.rs | Store additional `PortForwardHandle`s in `run_vm_loop` |
| shuru-vm | sandbox.rs | Add `SO_REUSEADDR` to port forward listeners |
| superhq | ports dialog | Call `sandbox.add_port_forward()` after saving to DB |
| superhq | terminal/mod.rs | Retry on bind failure during fork boot |
