# Spike: Supervisor fd-passing and portable-pty investigation

**Date:** 2026-04-17  
**Issue:** #8 — Phase 2.3: Supervisor PTYs survive backend restart

---

## portable-pty `MasterPty` and `AsRawFd`

**Finding:** `portable-pty`'s `MasterPty` trait does **not** expose `AsRawFd`. The trait only declares:
- `resize(&self, size: PtySize) -> Result<(), Error>`
- `try_clone_reader(&self) -> Result<Box<dyn Read + Send>, Error>`
- `take_writer(&self) -> Result<Box<dyn Write + Send>, Error>`

The concrete Linux implementation (`UnixMasterPty`) does implement `AsRawFd`, but this is not accessible through the trait object `Box<dyn MasterPty>`. Downcasting via `as_any()` is not available since `MasterPty` doesn't extend `Any`.

**Decision:** The supervisor uses `nix::pty::openpty()` directly instead of `portable-pty` for PTY creation. This gives concrete `OwnedFd` types for both master and slave, allowing direct SCM_RIGHTS fd passing without any trait gymnastics.

`portable-pty` remains available for the `hangard` backend's `PtyHandle::Direct` fallback (sessions spawned when supervisor is unreachable), where fd-passing is not required.

---

## `try_clone_reader()` path

`try_clone_reader()` returns `Box<dyn Read + Send>`. On Linux the underlying type is a `File`, but the trait object hides this. Using `std::io::Read::by_ref()` or other adapters doesn't recover `AsRawFd`. This path is a dead end for SCM_RIGHTS.

---

## PTY kernel buffer behavior

The Linux PTY master has a kernel-side ring buffer (~4 KB default, configurable via `TIOCOUTQ`). Behavior:

- Bytes written to the slave (child output) accumulate in the master's read buffer.
- If the master buffer fills before a reader drains it, **the child blocks on write**. This is expected and documented.
- Practical implication: if the backend crashes and nobody is reading the master, the child will block once ~4 KB of output accumulates. This is acceptable for interactive sessions (child is waiting for user input).
- After backend reconnects and calls `AttachFd`, it receives a dup of the master fd and can resume reading. Buffered bytes are not lost.

---

## Crash mid-write behavior

If the backend crashes mid-write to the PTY:

- The write call returns an error (EPIPE or similar) — handled by the caller, no hang.
- Partial bytes already written to the PTY slave may have been consumed by the child. This is unavoidable at the kernel level.
- Application-level framing (which shells and Claude Code both use) handles partial input via command boundaries and readline semantics.

---

## Supervisor runtime: tokio vs sync

**Decision:** Supervisor uses `tokio` with `current_thread` scheduler (via `#[tokio::main]`).

Rationale:
- Needs concurrent client handling (multiple hangard connections)
- Needs signal processing (`SIGCHLD`, `SIGTERM`)
- `tokio::signal::unix::signal(SignalKind::child())` integrates cleanly with the async loop
- `current_thread` runtime is sufficient — expected load is <10 concurrent clients

A sync supervisor was considered but rejected because signal handling + multiple concurrent sockets on a single thread requires either epoll manually or a framework. Tokio provides this cleanly.

---

## fd_pass_demo results

The `fd_pass_demo` binary (build with `--features dev-tools`) validates:

1. `nix::sys::socket::sendmsg` with `ControlMessage::ScmRights(&[fd])` correctly passes an fd across a `socketpair`.
2. `nix::sys::socket::recvmsg` with `nix::cmsg_space!(RawFd)` receives the fd as `ControlMessageOwned::ScmRights`.
3. The received fd is a **new** fd number in the receiver's address space (distinct integer, same underlying file).
4. `set_cloexec()` immediately sets `FD_CLOEXEC` on the received fd.
5. After `send_fd`, the sender's original fd remains valid (SCM_RIGHTS dups, not transfers).

Key nix 0.29 API notes:
- No `pty` feature flag needed; `nix::pty::openpty` is unconditionally compiled on Unix.
- `msg.cmsgs()` returns `Result<CmsgIterator>` — use `?` to unwrap.
- `nix::cmsg_space!(RawFd)` returns `Vec<u8>` sized correctly with alignment padding.

---

## Both-down (supervisor + backend) scenario

If both supervisor and backend die:
- Child processes are reparented to PID 1 (init/systemd) since the supervisor (subreaper) is gone.
- PTY master fds are closed — no process holds them.
- On restart, supervisor scans sidecar files. Finds orphan PIDs. Checks `kill(pid, 0)` — PIDs are alive (now under init).
- No `RegisterFd` arrives (backend lost its dup'd fds too). After 30s timeout, supervisor sends SIGTERM then SIGKILL.
- Sessions are lost. This is documented and accepted for MVP.

---

## SCM_RIGHTS fd lifetime rule

After `send_fd(socket, fd, payload)`:
- The kernel dups `fd` into the receiver's fd table during `sendmsg`.
- The sender retains its copy of `fd`. No need to close it after sending.
- The receiver owns the new fd; wrapping in `OwnedFd` ensures it's closed on drop.

When the supervisor sends a master fd via `AttachFd`:
- Supervisor calls `send_fd(fd_channel, master_fd, nonce_payload)`
- Supervisor keeps `master_fd` in `MasterHandle::OwnedFd` (needed for future `Write`/`Resize`)
- Backend receives a new fd number pointing to the same PTY master

When backend reconnects and calls `RegisterFd`:
- Backend calls `send_fd(fd_channel, held_master_fd, nonce_payload)`
- Backend keeps its copy (still needed for direct reads)
- Supervisor receives a new fd, wraps in `MasterHandle::OwnedFd`

Both sides legitimately holding fds to the same PTY master is intentional.
