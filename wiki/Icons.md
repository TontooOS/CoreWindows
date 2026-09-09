# Icons

`resolve_icon` locates the app icon file of a TontooOS `.app` bundle
through the `icon` field of its `Info.tontoo`. Rendering the file is left
to the caller (e.g. via CoreIcon); this module only resolves paths.

## Info.tontoo icon field

The `icon` field accepts two shapes (both relative to the bundle root):

```json
{"bundle_id": "com.tontoo.finder", "name": "Finder", "icon": "Icon.png"}
{"bundle_id": "com.tontoo.finder", "name": "Finder", "icon": {"path": "Resources/Icon.png"}}
```

When the field is missing or points at a non-existent file, these
candidates are probed in order (`ICON_PROBE_FILES`):

| Candidate | Note |
|---|---|
| `Icon.png` / `icon.png` | Bundle root |
| `Icon.jpg` / `icon.jpg` | Bundle root |
| `Resources/Icon.png` / `Resources/icon.png` | Resources dir |
| `Contents/Resources/Icon.png` | macOS-style layout |

## AppIcon

```rust
pub struct AppIcon {
  pub bundle_id: String,
  pub bundle_path: PathBuf,
  pub icon_path: Option<PathBuf>,
  pub app_name: String,
}
```

| Field | Type | Description |
|---|---|---|
| `bundle_id` | `String` | Bundle id from `Info.tontoo` |
| `bundle_path` | `PathBuf` | Absolute bundle directory (`....app`) |
| `icon_path` | `Option<PathBuf>` | Absolute icon file, `None` when nothing exists |
| `app_name` | `String` | Display name of the app |

## resolve_icon

```rust
pub fn resolve_icon(
  bundle_dir: &Path,
  bundle_id: &str,
  app_name: &str,
  info: Option<&serde_json::Value>,
) -> AppIcon
```

Resolves the icon for a bundle directory. `info` is the parsed
`Info.tontoo` (may be `None`). Never fails: without any existing file
the result carries `icon_path: None`.

```rust
use corewindows::icon::resolve_icon;
use std::path::Path;

let icon = resolve_icon(Path::new("/Applications/Finder.app"), "com.tontoo.finder", "Finder", None);
println!("{:?}", icon.icon_path);
```

## Usage / Example

End to end from a classified window (this is what
`WindowsProvider::windows` does internally):

```rust
use corewindows::{classify, icon::resolve_icon, RawWindow};

let raw = RawWindow { id: 1, app_id: None, title: None, pid: Some(1234) };
let c = classify(&raw);
let icon = match (&c.bundle_dir, &c.bundle_id, &c.app_name) {
  (Some(dir), Some(id), Some(name)) => Some(resolve_icon(dir, id, name, c.bundle_info.as_ref())),
  _ => None,
};
```

## Cross References

- [Classification.md](Classification.md) – where `bundle_dir` and `bundle_info` come from
- [Types.md](Types.md) – `WindowInfo.icon`
