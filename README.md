# Tontoo CoreWindows

List currently open windows on TontooOS: each window with its toolkit type
(`UIKit`, `TontooUI`, `Linux`) and, for TontooOS `.app` bundles, the resolved
app icon from `Info.tontoo`.

Wiki: [wiki/MAIN.md](wiki/MAIN.md)

## Made for TontooOS

Explore more at https://github.com/TontooOS/Libs

## Adding to Your Project

Via the SDK (recommended):

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

Or directly:

```toml
[dependencies]
corewindows = { path = "/Library/System/corewindows.library" }
```

```rust
fn main() {
    for w in corewindows::windows().unwrap() {
        println!("#{} [{}]", w.id, w.window_type);
    }
    // Minimize window 1, fullscreen window 2, politely close window 3.
    corewindows::minimize_window(1).unwrap();
    corewindows::set_fullscreen(2, true).unwrap();
    corewindows::close_window(3).unwrap();
}
```

## License

TCL v26.1