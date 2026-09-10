# Types

Core data types of the library: the toolkit enum, the raw daemon row and the
enriched window record.

## WindowType

```rust
pub enum WindowType {
  UIKit,
  TontooUi,
  Linux,
  Unknown,
}
```

Toolkit classification of an open window.

| Variant | JSON | Meaning |
|---|---|---|
| `UIKit` | `"uikit"` | Built with UIKit (`TONTOO_TOOLKIT=UIKit` or `toolkit: uikit`) |
| `TontooUi` | `"tontoui"` | Built with TontooUI, i.e. a native TontooOS (TOS) app |
| `Linux` | `"linux"` | Any other Linux app (GTK, Qt, Electron, Flatpak, X11, ...) |
| `Unknown` | `"unknown"` | No identifying information available |

### WindowType::as_str

```rust
pub fn as_str(self) -> &'static str
```

Returns the display name (`UIKit`, `TontooUI`, `Linux`, `Unknown`).
Never fails. `Display` prints the same string.

```rust
assert_eq!(WindowType::TontooUi.as_str(), "TontooUI");
```

## RawWindow

```rust
pub struct RawWindow {
  pub id: u64,
  pub app_id: Option<String>,
  pub title: Option<String>,
  pub pid: Option<i32>,
  pub minimized: bool,
}
```

One raw window row as reported by the window daemon. Mirrors the daemon
reply 1:1; all fields but `id` are optional and default to `None` when
absent, so sparse rows like `{"id":7}` deserialize without error.
`minimized` defaults to `false` when absent.

| Field | Type | Description |
|---|---|---|
| `id` | `u64` | Daemon-side window id, stable while the window is mapped |
| `app_id` | `Option<String>` | XDG app id (`set_app_id`), if the client set one |
| `title` | `Option<String>` | XDG title (`set_title`), if the client set one |
| `pid` | `Option<i32>` | Owning client pid, if the daemon knows it |
| `minimized` | `bool` | Whether the window is currently minimized to the dock |

## WindowInfo

```rust
pub struct WindowInfo {
  pub id: u64,
  pub app_id: Option<String>,
  pub title: Option<String>,
  pub pid: Option<i32>,
  pub minimized: bool,
  pub window_type: WindowType,
  pub bundle_id: Option<String>,
  pub app_name: Option<String>,
  pub icon: Option<AppIcon>,
}
```

A fully classified open window: raw daemon data plus toolkit type, bundle
identity and the resolved app icon. Built by `WindowsProvider::windows`;
see [Windows.md](Windows.md).

| Field | Type | Description |
|---|---|---|
| `minimized` | `bool` | Whether the window is currently minimized to the dock |
| `window_type` | `WindowType` | Toolkit classification (see [Classification.md](Classification.md)) |
| `bundle_id` | `Option<String>` | Bundle id from the owning `.app` `Info.tontoo`, if any |
| `app_name` | `Option<String>` | Display name (localized bundle name, bundle dir name, title or app id) |
| `icon` | `Option<AppIcon>` | Resolved app icon for TontooOS bundles (see [Icons.md](Icons.md)) |

## Usage / Example

```rust
use corewindows::{WindowType, WindowsProvider};

let windows = WindowsProvider::from_env().windows().unwrap();
let uikit: Vec<_> = windows.iter()
  .filter(|w| w.window_type == WindowType::UIKit)
  .collect();
println!("{} UIKit windows open", uikit.len());
```

## Cross References

- [Windows.md](Windows.md) – how `WindowInfo` rows are fetched and enriched
- [Classification.md](Classification.md) – how `window_type` is decided
- [Icons.md](Icons.md) – how `icon` is resolved
