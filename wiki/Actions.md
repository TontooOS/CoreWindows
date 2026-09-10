# Actions

Window actions change window state through the daemon (minimize,
restore, fullscreen, graceful close) or directly (force quit). All
actions use the daemon window `id` from [Types.md](Types.md).

## Daemon actions

These send one request each to the window daemon
(see [Windows.md](Windows.md) for the protocol). `Err` on missing or
unreachable daemon, malformed replies and daemon-side `ok: false`.

### minimize_window

```rust
pub fn minimize_window(&self, id: u64) -> Result<()>
```

Minimizes (iconifies) a window. The daemon hides the window, the app
keeps running. Request: `{"id":1,"op":"minimize_window","window":5}`.

### restore_window

```rust
pub fn restore_window(&self, id: u64) -> Result<()>
```

Restores a window minimized to the dock: the daemon re-maps it and drops
its temporary dock icon (same path as clicking the dock icon).
Request: `{"id":1,"op":"restore_window","window":5}`. Returns `Err` when
the id is not a minimized window or its client is gone.

### set_fullscreen

```rust
pub fn set_fullscreen(&self, id: u64, fullscreen: bool) -> Result<()>
```

Sets fullscreen state: `true` is fullscreen like the green UIKit
traffic light / F11, `false` returns to windowed mode.
Request: `{"id":1,"op":"set_fullscreen","window":5,"fullscreen":true}`.

Shortcuts on the provider:

```rust
pub fn fullscreen_window(&self, id: u64) -> Result<()>
pub fn unfullscreen_window(&self, id: u64) -> Result<()>
```

### close_window

```rust
pub fn close_window(&self, id: u64) -> Result<()>
```

Gracefully closes a window: the daemon asks the client to close
(`xdg_toplevel.close` on Wayland, `WM_DELETE_WINDOW` on X11).
The app itself decides what happens next, e.g. LibreOffice shows its
save dialog for unsaved documents. Nothing is killed.
Request: `{"id":1,"op":"close_window","window":5}`.

## Force quit

### force_quit_window

```rust
pub fn force_quit_window(&self, id: u64) -> Result<()>
```

Force quits the owner of a window: resolves the window id to its pid
via `list_raw` and sends `SIGKILL` immediately (macOS Force Quit
equivalent). No save dialog can appear, unsaved work is lost.
Returns `Err(Parse)` when the window id is unknown or carries no pid.

### force_quit_pid

```rust
pub fn force_quit_pid(pid: i32) -> Result<()>
```

Force quits a process id directly (`SIGKILL`). Returns `Err` when the
signal fails (unknown pid or insufficient permission). Also exported
at the crate root and over FFI / SDK.

## Crate-level helpers

```rust
pub fn minimize_window(id: u64) -> Result<()>
pub fn restore_window(id: u64) -> Result<()>
pub fn set_fullscreen(id: u64, fullscreen: bool) -> Result<()>
pub fn close_window(id: u64) -> Result<()>
pub fn force_quit_window(id: u64) -> Result<()>
```

Shortcuts using `WindowsProvider::new()` (default socket).

## Usage / Example

```rust
use corewindows::WindowsProvider;

let provider = WindowsProvider::from_env();
// Minimize everything except the focused editor, then close window 7 politely.
for w in provider.windows().unwrap() {
  if w.id != 7 {
    provider.minimize_window(w.id).unwrap();
  }
}
provider.close_window(7).unwrap();
```

## Cross References

- [Windows.md](Windows.md) – daemon protocol frames for each action
- [Types.md](Types.md) – window ids used by the actions
- [Ffi.md](Ffi.md) – C access with numeric return codes
