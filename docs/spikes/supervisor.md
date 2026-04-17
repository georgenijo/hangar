# Spike: Supervisor PTY fd-passing

## What we validated

`fd_passing_spike.rs` proves the end-to-end flow:
1. Open PTY with `nix::pty::openpty()`
2. Fork+exec bash with slave as stdin/stdout/stderr via `pre_exec` + `dup2`
3. Pass master fd to a second process/thread via `SCM_RIGHTS`
4. Write command to received fd, read back output

## SCM_RIGHTS syscall sequence

```
supervisor                          backend
─────────────────────────────────────────────
openpty() → (master_fd, slave_fd)
fork+exec bash (slave as tty)
drop(slave_fd)
listen on Unix socket

accept()  ←─────────── connect()
                  ←─── send b"\x00"   (at least 1 byte required)
sendmsg(SCM_RIGHTS=[master_fd.dup()])
          ──────────────────────────→
                        recvmsg() → received_fd (NEW fd number)
                        write(received_fd, b"echo hello\n")
                        read(received_fd) → output
```

**Key gotcha:** fd numbers change across the socket transfer. If supervisor
sends fd=7, backend receives some other number (e.g. fd=9). The kernel
allocates the next free fd in the receiving process. Code must never assume
fd numbers are preserved.

**Required 1-byte payload:** Linux requires at least 1 byte of regular data
alongside SCM_RIGHTS ancillary data. Sending ancillary-only causes EINVAL.

## `openpty` vs `portable_pty`

`portable_pty`'s `MasterPty` trait does not expose a raw `RawFd` for
`SCM_RIGHTS` passing. The supervisor uses `nix::pty::openpty()` directly,
which returns `OwnedFd` values — ideal for `try_clone()` and SCM_RIGHTS.

The backend reconstructs PTY I/O using `RawFdMaster` (a thin struct
implementing `Read`/`Write` over `OwnedFd`). No need to reconstruct a full
`portable_pty` object since we only need I/O and resize on the master side.

## `prctl(PR_SET_CHILD_SUBREAPER)` semantics

When a process dies, its orphaned children are normally reparented to PID 1.
With `prctl(PR_SET_CHILD_SUBREAPER, 1)`, the closest ancestor subreaper
absorbs those orphans instead. This means:

- Supervisor is subreaper for all its descendants
- If a PTY child (bash) spawns subprocesses and then dies, those grandchildren
  are reparented to the supervisor, not init
- Supervisor can `waitpid` for them and update session state

Implemented via `libc::prctl(libc::PR_SET_CHILD_SUBREAPER, 1, ...)` early in
supervisor `main()`. Linux-only; no-op guard for other platforms.

## Spawning without `nix::fork`

Calling `fork()` inside a multithreaded tokio process is dangerous: the child
inherits all thread pool state (mutexes, async runtime), which can deadlock.

Solution: `std::process::Command::spawn()` with `unsafe { cmd.pre_exec(...) }`.
The `pre_exec` closure runs in the child after fork but before exec. At that
point the child is single-threaded and safe to call `dup2`/`setsid`/`ioctl`.
`Command::spawn()` handles the threading concerns internally.

## Race conditions

**Backend connects before supervisor is ready:** `SupervisorClient::connect`
retries 3× with 500 ms backoff. If supervisor isn't up yet, backend falls back
to `portable_pty` local mode and marks all sessions as Exited.

**Supervisor crashes while backend holds received fd:** The fd is valid in the
backend's process — it's a kernel object. The backend keeps reading/writing
normally. When the PTY child eventually exits, `read()` returns EOF.

**Child exits before backend attaches fd:** Supervisor marks `alive = false`
via SIGCHLD handler. Backend calls `attach_fd`, gets the fd (supervisor keeps
master open), starts reader thread which immediately gets EOF, cleanup task
marks session Exited. Correct behavior, no special handling needed.

**Supervisor crash mid-send (between write_frame and send_fd):** Backend reads
the JSON response, then blocks on `recv_fd`. If the socket is closed before
the fd arrives, `recvmsg` returns an error. `attach_fd` propagates the error,
session is left as Idle in DB (will be marked Exited on next restart).

## fd table limits

Each reattached session duplicates the master fd once per connection. With many
sessions, fd usage grows. Default Linux limit is 1024 (soft) / 1048576 (hard).
For typical usage (< 100 sessions) this is not a concern.
