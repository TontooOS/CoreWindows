# Classification

`classify` decides the toolkit type of one raw window row and resolves its
bundle identity. It is the shared core behind `WindowsProvider::windows`.

## Detection order

First hit wins for the type:

1. `TONTOO_TOOLKIT` in `/proc/<pid>/environ` (`UIKit`, `TontooUI`).
   UIKit sets it automatically in `App::run` (see
   `uikit::app::mark_toolkit`); TontooUI follows the same convention.
2. The `toolkit` field of the owning `.app` `Info.tontoo`
   (`uikit`, `tontoui` / `tontoo-ui` / `tos`, case-insensitive).
3. A bundle id starting with `com.tontoo.` implies `TontooUI`.
4. The app id `org.tontoo.uikit` (the GTK application id every UIKit
   app uses) implies `UIKit`.
5. Any remaining window with app id, title or pid is a `Linux` app;
   rows without any identifying information are `Unknown`.

## classify

```rust
pub fn classify(raw: &RawWindow) -> Classification
```

Never fails; unknown inputs classify as `Linux` or `Unknown`
(see step 5). Returns `Err` never – the return type is a plain struct.

```rust
use corewindows::{classify, RawWindow, WindowType};

let raw = RawWindow { id: 1, app_id: Some("org.mozilla.firefox".into()), title: None, pid: None };
assert_eq!(classify(&raw).window_type, WindowType::Linux);
```

## Classification

```rust
pub struct Classification {
  pub window_type: WindowType,
  pub bundle_id: Option<String>,
  pub app_name: Option<String>,
  pub bundle_dir: Option<PathBuf>,
  pub bundle_info: Option<serde_json::Value>,
}
```

| Field | Type | Description |
|---|---|---|
| `window_type` | `WindowType` | Toolkit classification |
| `bundle_id` | `Option<String>` | `bundle_id` from `Info.tontoo`, if any |
| `app_name` | `Option<String>` | Localized bundle name, bundle dir stem, title or app id |
| `bundle_dir` | `Option<PathBuf>` | Owning `.app` bundle directory, if any |
| `bundle_info` | `Option<Value>` | Parsed `Info.tontoo`, if any |

## Helpers

### toolkit_of_pid

```rust
pub fn toolkit_of_pid(pid: i32) -> Option<WindowType>
```

Reads the `TONTOO_TOOLKIT` marker from `/proc/<pid>/environ`.
Returns `None` when the process is gone, unreadable or unmarked.

### bundle_of_pid

```rust
pub fn bundle_of_pid(pid: i32) -> Option<(PathBuf, serde_json::Value)>
```

Finds the owning `.app` bundle of a process (nearest ancestor directory
ending in `.app`, or the binary itself when named `*.app`) and parses
its `Info.tontoo`. Returns `None` for plain binaries, missing info
files and invalid JSON.

### localized_name

```rust
pub fn localized_name(info: &serde_json::Value) -> Option<String>
```

Bundle display name: `name` is either a plain string or a locale map
(`{"en_us": ..., "de_de": ...}`), resolved with the current locale
first, then `en_us`, then the first entry. Returns `None` when the
`name` field is missing or unusable.

## Constants

| Constant | Value | Meaning |
|---|---|---|
| `TOOLKIT_ENV_VAR` | `"TONTOO_TOOLKIT"` | Marker variable every toolkit sets |
| `TOOLKIT_UIKIT` | `"UIKit"` | Id reported by UIKit apps |
| `TOOLKIT_TONTOOUI` | `"TontooUI"` | Id reported by TontooUI apps |
| `UIKIT_APP_ID` | `"org.tontoo.uikit"` | GTK application id of UIKit apps |
| `TONTOO_BUNDLE_PREFIX` | `"com.tontoo."` | Bundle prefix implying `TontooUI` |

## Usage / Example

```rust
use corewindows::{classify, RawWindow};

let raw = RawWindow { id: 3, app_id: None, title: Some("Notes".into()), pid: None };
let c = classify(&raw);
println!("type={} name={:?} bundle={:?}", c.window_type, c.app_name, c.bundle_id);
```

## Cross References

- [Types.md](Types.md) – `WindowType` variants
- [Icons.md](Icons.md) – icon lookup from `bundle_dir` / `bundle_info`
- [Windows.md](Windows.md) – provider enrichment pipeline
