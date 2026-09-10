# Tontoo CoreWindows – Wiki

Tontoo CoreWindows lists the currently open windows on TontooOS: each window
with its toolkit type (`UIKit`, `TontooUI`, `Linux`) and, for TontooOS `.app`
bundles, the resolved app icon from `Info.tontoo`.

- Repository: https://github.com/TontooOS/Libs
- License: TCL
- Version: 26.1.0

## Feature Index

| Feature | File | Description |
|---|---|---|
| Main index | [MAIN.md](MAIN.md) | This page |
| Rules | [RULE.md](RULE.md) | Development and usage rules |
| Types | [Types.md](Types.md) | `WindowType`, `RawWindow`, `WindowInfo` |
| Windows | [Windows.md](Windows.md) | `WindowsProvider`, daemon socket protocol |
| Actions | [Actions.md](Actions.md) | Minimize, fullscreen, close, force quit |
| Programs | [Programs.md](Programs.md) | Installed `.app` bundles from both applications dirs |
| Classification | [Classification.md](Classification.md) | Toolkit detection rules and helpers |
| Icons | [Icons.md](Icons.md) | `AppIcon`, `Info.tontoo` icon resolution |
| Localization | [Localization.md](Localization.md) | `lang/` files and error messages |
| FFI | [Ffi.md](Ffi.md) | C API in `Headers/corewindows.h` |

## Quick Start

Add the dependency to your `Cargo.toml`:

```toml
[dependencies]
sdk = { path = "/Library/System/sdk", features = ["CoreWindows"] }
```

Then at the crate root:

```rust
sdk::preinclude!();
use CoreWindows::WindowsProvider;

fn main() {
  let windows = WindowsProvider::from_env().windows().unwrap();
  for w in &windows {
    println!("[{}] {}", w.window_type, w.app_name.as_deref().unwrap_or("?"));
  }
}
```

Without the SDK, depend on the crate directly:

```toml
[dependencies]
corewindows = { path = "/Library/System/corewindows.library" }
```

See [Windows.md](Windows.md) and [Types.md](Types.md) for details.

## Changelog

- 2026-09-10: Window restore (`restore_window` on provider, crate root,
  FFI and SDK) plus `minimized` flag on `RawWindow`/`WindowInfo` and
  daemon rows.
- 2026-09-10: Zipped `.app` files (TBuild output) are listed too
  (`Info.tontoo` at root or under `<Name>.app/`); icons are extracted
  once into `$TMPDIR/tontoo-corewindows-icons/<bundle-id>/`.
- 2026-09-09: Program listing (`list_programs` over `~/Applications`
  and `/Applications` with all names, bundle path, source and icon;
  FFI, SDK, example).
- 2026-09-09: Window actions (minimize, fullscreen, graceful close,
  force quit via `SIGKILL`) on provider, crate root, FFI and SDK.
- 2026-09-08: Initial wiki and crate (window listing, classification,
  icon resolution, FFI, SDK shim).
