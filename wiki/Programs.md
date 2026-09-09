# Programs

`list_programs` scans the two applications directories for installed
TontooOS `.app` bundles. Pure filesystem scan, no daemon needed.

## Directories

| Source | Path | Meaning |
|---|---|---|
| `User` | `~/Applications` (`HOME` + `Applications`) | Installed by the user |
| `System` | `/Applications` | Installed system-wide |

`program_dirs()` returns both in scan order (user first); missing
directories are skipped silently.

## AppEntry

```rust
pub struct AppEntry {
  pub bundle_id: String,
  pub names: HashMap<String, String>,
  pub display_name: String,
  pub bundle_path: PathBuf,
  pub source: AppSource,
  pub icon: AppIcon,
}
```

| Field | Type | Description |
|---|---|---|
| `bundle_id` | `String` | Bundle id from `Info.tontoo` |
| `names` | `HashMap<String, String>` | **All** names: every locale of the `name` map; a plain string `name` becomes one `"default"` entry |
| `display_name` | `String` | Name for the current locale (locale, `en_us`, first entry, bundle dir stem fallback) |
| `bundle_path` | `PathBuf` | Absolute bundle directory (`....app`) |
| `source` | `AppSource` | `User` or `System` |
| `icon` | `AppIcon` | Resolved app icon (see [Icons.md](Icons.md)) |

## Functions

### list_programs

```rust
pub fn list_programs() -> Vec<AppEntry>
```

Lists every bundle with a readable `Info.tontoo` carrying a
`bundle_id`. Plain files, bundles without info, invalid JSON and
missing `bundle_id` values are skipped silently. Never fails;
returns an empty vec when nothing is found. Results are sorted by
display name (case-insensitive).

```rust
use corewindows::list_programs;

for app in list_programs() {
  println!("[{}] {} ({})", app.source, app.display_name, app.bundle_id);
}
```

### scan_dir

```rust
pub fn scan_dir(dir: &Path, source: AppSource) -> Vec<AppEntry>
```

Scans one directory (unsorted). Returns an empty vec for missing or
unreadable directories.

### program_dirs

```rust
pub fn program_dirs() -> Vec<(AppSource, PathBuf)>
```

The scanned directories: user first, then system. `HOME` unset or
empty drops the user entry.

### all_names

```rust
pub fn all_names(info: &serde_json::Value) -> HashMap<String, String>
```

Every known name of a parsed `Info.tontoo`: the full `name` locale
map, or one `"default"` entry for a plain string. Empty when `name`
is missing or unusable.

## Usage / Example

Run the bundled example:

```bash
cargo run --example list_programs
```

## Cross References

- [Icons.md](Icons.md) – how `icon` is resolved
- [Classification.md](Classification.md) – `localized_name` used for `display_name`
- [Ffi.md](Ffi.md) – `tontoo_corewindows_list_programs`
