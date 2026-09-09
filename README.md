# Tontoo CoreWindows

List currently open windows on TontooOS: each window with its toolkit type
(`UIKit`, `TontooUI`, `Linux`) and, for TontooOS `.app` bundles, the resolved
app icon from `Info.tontoo`.

## Made for TontooOS

Explore more at https://github.com/TontooOS/Libs

## Adding to Your Project

Via the SDK (recommended):

```toml
[dependencies]
sdk = { path = "/Library/System/sdk", features = ["CoreWindows"] }
```

## License

TCL v26.1