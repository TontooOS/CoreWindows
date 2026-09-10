# Windows

`WindowsProvider` fetches the currently open windows from the window daemon
over its unix socket and enriches each row into a `WindowInfo`.

## Socket

| Item | Value |
|---|---|
| Default path | `/run/tontoo-windows.sock` (`DEFAULT_SOCKET_PATH`) |
| Override | `WINDOWS_SOCKET` environment variable (`WindowsProvider::from_env`) |
| Framing | One JSON object per line, one JSON reply line |

When the socket path does not exist, every call returns
`Err(WindowsError::SocketMissing(path))`.

## Daemon protocol

The daemon side (TontooCompositor) implements these ops. Requests:

```json
{"id": 1, "op": "ping"}
{"id": 1, "op": "list_windows"}
{"id": 1, "op": "minimize_window", "window": 5}
{"id": 1, "op": "restore_window", "window": 5}
{"id": 1, "op": "set_fullscreen", "window": 5, "fullscreen": true}
{"id": 1, "op": "close_window", "window": 5}
```

| Op | Effect (daemon side) |
|---|---|
| `ping` | Answer `{"pong": true}` |
| `list_windows` | Answer `{"windows": [...]}` (see below; minimized rows carry `"minimized": true`) |
| `minimize_window` | Iconify the window, app keeps running |
| `restore_window` | Re-map a minimized window and drop its temp dock icon |
| `set_fullscreen` | Fullscreen (`true`) or unfullscreen (`false`) the window |
| `close_window` | Ask the client to close (`xdg_toplevel.close` / `WM_DELETE_WINDOW`) |

Force quit needs no daemon op: the lib sends `SIGKILL` to the window
pid directly (see [Actions.md](Actions.md)).

Replies use `{"ok": true, "result": ...}` or
`{"ok": false, "error": "..."}`:

```json
{"ok": true, "result": {"pong": true}}
{"ok": true, "result": {"windows": [{"id": 1, "app_id": "org.tontoo.uikit", "title": "Finder", "pid": 1234}]}}
```

All window fields but `id` are optional; `app_id` is the XDG app id,
`title` the XDG title and `pid` the owning client pid.

## Constructors

### WindowsProvider::new

```rust
pub fn new() -> Self
```

Uses the default socket path. Never fails; I/O errors surface on use.

### WindowsProvider::with_socket

```rust
pub fn with_socket(path: impl Into<PathBuf>) -> Self
```

Uses an explicit socket path (tests, custom daemon locations).

### WindowsProvider::from_env

```rust
pub fn from_env() -> Self
```

Builds from `WINDOWS_SOCKET`, falls back to the default path.

## Functions

### WindowsProvider::ping

```rust
pub fn ping(&self) -> Result<bool>
```

Returns `true` on a valid pong reply, `false` on a well-formed reply
without one. Returns `Err` when the socket is missing, unreachable or
the reply is malformed.

### WindowsProvider::list_raw

```rust
pub fn list_raw(&self) -> Result<Vec<RawWindow>>
```

Raw window rows from the daemon, no classification or icon lookup.
Returns `Err` when the reply has no `windows` array (`Parse`) or a row
fails to deserialize (`Parse` with the row error).

### WindowsProvider::windows

```rust
pub fn windows(&self) -> Result<Vec<WindowInfo>>
```

Fully classified windows: each raw row enriched with toolkit type,
bundle identity (see [Classification.md](Classification.md)) and the
resolved app icon (see [Icons.md](Icons.md)). Rows without a bundle get
`icon: None`.

### Crate-level helpers

```rust
pub fn ping() -> Result<bool>
pub fn list_raw() -> Result<Vec<RawWindow>>
pub fn windows() -> Result<Vec<WindowInfo>>
```

Shortcuts using `WindowsProvider::new()` (default socket).

## Usage / Example

```rust
use corewindows::WindowsProvider;

let provider = WindowsProvider::from_env();
assert!(provider.ping().unwrap());
for w in provider.windows().unwrap() {
  println!("#{} [{}] {}", w.id, w.window_type, w.app_name.unwrap_or_default());
}
```

Run the bundled example (needs a running daemon):

```bash
cargo run --example list_windows
```

## Cross References

- [Types.md](Types.md) – `RawWindow` and `WindowInfo` shapes
- [Actions.md](Actions.md) – minimize, fullscreen, close, force quit
- [Classification.md](Classification.md) – type and bundle enrichment
- [Icons.md](Icons.md) – icon enrichment
- [Ffi.md](Ffi.md) – C access to the same calls
