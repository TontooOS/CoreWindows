use std::collections::HashMap;
use std::path::{Path, PathBuf};

use archivekit::{AppManifest, AppReader};
use foundation::serialization::JsonDocument;

use crate::classify::localized_name;
use crate::icon::{icon_field, resolve_icon, AppIcon};

/// System-wide applications directory (OS root).
pub const SYSTEM_APPLICATIONS_DIR: &str = "/Applications";

/// Per-user applications directory name (resolved against `HOME`).
pub const USER_APPLICATIONS_DIR: &str = "Applications";

/// `Info.tontoo` file name inside bundles and containers.
pub const INFO_FILE: &str = "Info.tontoo";

/// Where an app bundle was found.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AppSource {
  /// `~/Applications` – installed by the user.
  User,
  /// `/Applications` – installed system-wide.
  System,
}

impl AppSource {
  /// Display name of the variant (`User`, `System`).
  pub fn as_str(self) -> &'static str {
    match self {
      Self::User => "User",
      Self::System => "System",
    }
  }

  /// Snake-case wire spelling (`user`, `system`), matching the previous
  /// serde `rename_all` output.
  pub fn snake_str(self) -> &'static str {
    match self {
      Self::User => "user",
      Self::System => "system",
    }
  }
}

impl std::fmt::Display for AppSource {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    f.write_str(self.as_str())
  }
}

/// One installed TontooOS application (`.app` bundle).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AppEntry {
  /// Bundle id from `Info.tontoo` (`bundle_id`).
  pub bundle_id: String,
  /// All known names: every locale of the `Info.tontoo` `name` map.
  /// A plain string `name` becomes a single `"default"` entry.
  pub names: HashMap<String, String>,
  /// Display name for the current locale (locale, `en_us`, first entry,
  /// then bundle dir stem fallback).
  pub display_name: String,
  /// Absolute bundle directory (`....app`), or the container file itself
  /// for single-file TAPP containers.
  pub bundle_path: PathBuf,
  /// Where the bundle was found.
  pub source: AppSource,
  /// Resolved app icon (paths only, see [`AppIcon`]).
  pub icon: AppIcon,
}

/// Directories scanned by [`list_programs`]: user first, then system.
/// Missing directories are skipped, so the result may be empty.
pub fn program_dirs() -> Vec<(AppSource, PathBuf)> {
  let mut dirs = Vec::with_capacity(2);
  if let Ok(home) = std::env::var("HOME") {
    if !home.is_empty() {
      dirs.push((AppSource::User, PathBuf::from(home).join(USER_APPLICATIONS_DIR)));
    }
  }
  dirs.push((AppSource::System, PathBuf::from(SYSTEM_APPLICATIONS_DIR)));
  dirs
}

/// List all installed programs from `~/Applications` and `/Applications`.
///
/// Each `*.app` entry becomes one [`AppEntry`]: directories with a readable
/// fico `Info.tontoo` carrying a `bundle_id`, and TAPP `.app` containers
/// (TBuild output) with a manifest carrying a `bundle_id`. Only the manifest
/// (plus the icon on demand) is read from containers. Anything else (plain
/// files, bundles without info, invalid manifests, missing `bundle_id`,
/// non-TAPP `.app` files) is skipped silently. Results are sorted by display
/// name (case-insensitive).
pub fn list_programs() -> Vec<AppEntry> {
  let mut entries = Vec::new();
  for (source, dir) in program_dirs() {
    entries.extend(scan_dir(&dir, source));
  }
  entries.sort_by(|a, b| {
    a.display_name
      .to_lowercase()
      .cmp(&b.display_name.to_lowercase())
  });
  entries
}

/// Scan one applications directory for `.app` bundles (directories and
/// TAPP `.app` containers, see [`list_programs`]).
pub fn scan_dir(dir: &Path, source: AppSource) -> Vec<AppEntry> {
  let mut entries = Vec::new();
  let Ok(read_dir) = std::fs::read_dir(dir) else {
    return entries;
  };
  for entry in read_dir.flatten() {
    let path = entry.path();
    if !is_app_bundle(&path) {
      continue;
    }
    if let Some(app) = read_app(&path, source) {
      entries.push(app);
    }
  }
  entries
}

fn is_app_bundle(path: &Path) -> bool {
  if !path
    .file_name()
    .map(|n| n.to_string_lossy().ends_with(".app"))
    .unwrap_or(false)
  {
    return false;
  }
  // Installed bundles are directories; TBuild ships them as single-file
  // TAPP `.app` containers (opened by `tapp` without extraction).
  // Anything else is ignored.
  path.is_dir() || path.is_file()
}

fn read_app(bundle: &Path, source: AppSource) -> Option<AppEntry> {
  if bundle.is_dir() {
    read_dir_bundle(bundle, source)
  } else {
    read_container_bundle(bundle, source)
  }
}

/// Read an installed directory bundle: `<dir>/Info.tontoo` in fico syntax.
fn read_dir_bundle(bundle_dir: &Path, source: AppSource) -> Option<AppEntry> {
  let text = std::fs::read_to_string(bundle_dir.join(INFO_FILE)).ok()?;
  let manifest = AppManifest::from_fico(&text).ok()?;
  let info = manifest_to_json(&manifest)?;
  let bundle_id = manifest.bundle_id.clone();
  let names = all_names(&info);
  let display_name = localized_name(&info).or_else(|| {
    bundle_dir
      .file_stem()
      .and_then(|s| s.to_str())
      .map(str::to_owned)
  })?;
  let icon = resolve_icon(bundle_dir, &bundle_id, &display_name, Some(&info));
  Some(AppEntry {
    bundle_id,
    names,
    display_name,
    bundle_path: bundle_dir.to_owned(),
    source,
    icon,
  })
}

/// Read a TAPP `.app` container (TBuild output) by index: only the footer,
/// the central directory, the manifest and (on demand) the icon entry are
/// touched. `bundle_path` points at the `.app` file itself, which `tapp`
/// opens without prior extraction, so launching works unchanged.
fn read_container_bundle(container: &Path, source: AppSource) -> Option<AppEntry> {
  let mut reader = AppReader::open(container).ok()?;
  let manifest = reader.read_manifest().ok()?;
  let info = manifest_to_json(&manifest)?;
  let bundle_id = manifest.bundle_id.clone();
  let names = all_names(&info);
  let display_name = localized_name(&info).or_else(|| {
    container
      .file_stem()
      .and_then(|s| s.to_str())
      .map(str::to_owned)
  })?;
  let icon = resolve_container_icon(&mut reader, container, &bundle_id, &display_name, &info);
  Some(AppEntry {
    bundle_id,
    names,
    display_name,
    bundle_path: container.to_owned(),
    source,
    icon,
  })
}

/// Resolve the icon of a TAPP container: the manifest `icon` entry first,
/// then [`crate::icon::ICON_PROBE_FILES`] under the container top prefix,
/// read selectively and extracted once into the temp icon cache.
/// `icon_path` is `None` when the container holds no usable file.
fn resolve_container_icon(
  reader: &mut AppReader<std::fs::File>,
  container: &Path,
  bundle_id: &str,
  app_name: &str,
  info: &JsonDocument,
) -> AppIcon {
  let top = reader
    .manifest_name()
    .and_then(|entry| entry.strip_suffix(INFO_FILE))
    .unwrap_or("")
    .to_string();
  let mut candidates = Vec::new();
  if let Some(rel) = icon_field(info) {
    candidates.push(rel);
  }
  candidates.extend(
    crate::icon::ICON_PROBE_FILES
      .iter()
      .map(|s| s.to_string()),
  );
  let bundle_path = container.to_owned();
  for rel in candidates {
    let full = format!("{top}{rel}");
    let present = reader.find(&full).is_some_and(|meta| !meta.is_dir());
    if !present {
      continue;
    }
    if let Some(out) = extract_container_icon(reader, &full, &rel, container, bundle_id) {
      return AppIcon {
        bundle_id: bundle_id.to_owned(),
        bundle_path,
        icon_path: Some(out),
        app_name: app_name.to_owned(),
      };
    }
  }
  AppIcon {
    bundle_id: bundle_id.to_owned(),
    bundle_path,
    icon_path: None,
    app_name: app_name.to_owned(),
  }
}

/// Resolve the icon of a container for window enrichment (see
/// [`crate::provider`]): opens the container, reads the manifest and
/// extracts the icon into the temp icon cache. Used when a running app was
/// launched from a single-file container (`TONTOO_APP_CONTAINER`) instead
/// of a directory bundle.
pub(crate) fn container_icon_cached(
  container: &Path,
  bundle_id: &str,
  app_name: &str,
) -> Option<AppIcon> {
  let mut reader = AppReader::open(container).ok()?;
  let manifest = reader.read_manifest().ok()?;
  let info = manifest_to_json(&manifest)?;
  Some(resolve_container_icon(
    &mut reader,
    container,
    bundle_id,
    app_name,
    &info,
  ))
}

/// Extract one container entry into the temp icon cache
/// (`$TMPDIR/tontoo-corewindows-icons/<bundle-id>/<path>`). Cached files are
/// reused; a container newer than its cache is extracted again.
fn extract_container_icon(
  reader: &mut AppReader<std::fs::File>,
  full: &str,
  rel: &str,
  container: &Path,
  bundle_id: &str,
) -> Option<PathBuf> {
  let safe_id: String = bundle_id
    .chars()
    .map(|c| {
      if c.is_ascii_alphanumeric() {
        c
      } else {
        '-'
      }
    })
    .collect();
  let out = std::env::temp_dir()
    .join("tontoo-corewindows-icons")
    .join(safe_id)
    .join(rel);
  if out.is_file() && !container_newer_than(container, &out) {
    return Some(out);
  }
  let parent = out.parent()?;
  std::fs::create_dir_all(parent).ok()?;
  let bytes = reader.read_file(full).ok()?;
  std::fs::write(&out, &bytes).ok()?;
  Some(out)
}

/// Whether the `.app` container was modified after its cached icon extract.
fn container_newer_than(container: &Path, cached: &Path) -> bool {
  match (
    std::fs::metadata(container).and_then(|m| m.modified()),
    std::fs::metadata(cached).and_then(|m| m.modified()),
  ) {
    (Ok(container_time), Ok(cache_time)) => container_time > cache_time,
    _ => true,
  }
}

/// Minimal JSON string escaping for [`manifest_to_json`].
fn json_escape(raw: &str) -> String {
  let mut out = String::with_capacity(raw.len() + 2);
  for c in raw.chars() {
    match c {
      '"' => out.push_str("\\\""),
      '\\' => out.push_str("\\\\"),
      '\n' => out.push_str("\\n"),
      '\r' => out.push_str("\\r"),
      '\t' => out.push_str("\\t"),
      c if c.is_control() => out.push_str(&format!("\\u{:04x}", c as u32)),
      c => out.push(c),
    }
  }
  out
}

/// Convert a fico [`AppManifest`] into the JSON document the name, icon and
/// classification helpers work on (`bundle_id`, `version`, `name` map,
/// optional `icon`). Keeps those helpers (and the FFI) format-agnostic.
pub(crate) fn manifest_to_json(manifest: &AppManifest) -> Option<JsonDocument> {
  let mut names = String::new();
  for (index, (locale, name)) in manifest.names.iter().enumerate() {
    if index > 0 {
      names.push(',');
    }
    names.push_str(&format!(
      "\"{}\":\"{}\"",
      json_escape(locale),
      json_escape(name)
    ));
  }
  let mut doc = format!(
    "{{\"bundle_id\":\"{}\",\"version\":\"{}\",\"name\":{{{}}}}}",
    json_escape(&manifest.bundle_id),
    json_escape(&manifest.version),
    names
  );
  if let Some(icon) = &manifest.icon {
    doc.pop();
    doc.push_str(&format!(",\"icon\":\"{}\"}}", json_escape(icon)));
  }
  JsonDocument::parse(&doc).ok()
}

/// Every known name of a bundle: the full `name` locale map, or a single
/// `"default"` entry for a plain string `name`. Empty when `name` is
/// missing or unusable.
pub fn all_names(info: &JsonDocument) -> HashMap<String, String> {
  if let Some(text) = info.str_field("name").unwrap_or(None) {
    let mut names = HashMap::with_capacity(1);
    names.insert("default".to_owned(), text);
    return names;
  }
  info.string_map_field("name").unwrap_or_default()
}

#[cfg(test)]
mod tests {
  use super::*;
  use archivekit::{AppBuilder, AppManifest};
  use std::io::Write as _;

  fn write_bundle(dir: &Path, name: &str, info: &str, icon: bool) -> PathBuf {
    let bundle = dir.join(name);
    std::fs::create_dir_all(&bundle).unwrap();
    std::fs::File::create(bundle.join(INFO_FILE))
      .unwrap()
      .write_all(info.as_bytes())
      .unwrap();
    if icon {
      let icon_path = bundle.join("App").join("icon.tico");
      std::fs::create_dir_all(icon_path.parent().unwrap()).unwrap();
      std::fs::File::create(&icon_path)
        .unwrap()
        .write_all(b"tico")
        .unwrap();
    }
    bundle
  }

  const FINDER_INFO: &str = "app {\n  bundle_id: com.tontoo.finder\n  version: \"1.0\"\n  executable: App/finder\n  icon: App/icon.tico\n  name {\n    en_us: \"Finder\"\n    de_de: \"FinderDE\"\n  }\n}\n";

  #[test]
  fn scan_dir_reads_names_and_icon() {
    let root = tempfile::tempdir().unwrap();
    write_bundle(root.path(), "Finder.app", FINDER_INFO, true);
    write_bundle(root.path(), "Broken.app", "not fico {{{", false);
    write_bundle(
      root.path(),
      "NoId.app",
      "app {\n  version: \"1.0\"\n  executable: App/noid\n}\n",
      false,
    );
    std::fs::create_dir_all(root.path().join("Not.flat")).unwrap();

    let mut entries = scan_dir(root.path(), AppSource::User);
    assert_eq!(entries.len(), 1);
    let app = entries.pop().unwrap();
    assert_eq!(app.bundle_id, "com.tontoo.finder");
    assert_eq!(app.names.get("en_us").map(String::as_str), Some("Finder"));
    assert_eq!(app.names.get("de_de").map(String::as_str), Some("FinderDE"));
    assert!(app.display_name == "Finder" || app.display_name == "FinderDE");
    assert!(app.icon.icon_path.is_some());
    assert_eq!(app.source, AppSource::User);
  }

  #[test]
  fn plain_string_name_becomes_default() {
    let info = JsonDocument::parse(r#"{"name":"Terminal"}"#).unwrap();
    let names = all_names(&info);
    assert_eq!(names.len(), 1);
    assert_eq!(names.get("default").map(String::as_str), Some("Terminal"));
  }

  #[test]
  fn missing_name_is_empty() {
    let info = JsonDocument::parse(r#"{"bundle_id":"x"}"#).unwrap();
    assert!(all_names(&info).is_empty());
  }

  #[test]
  fn missing_dir_scans_empty() {
    let entries = scan_dir(
      Path::new("/nonexistent/corewindows-programs-test"),
      AppSource::System,
    );
    assert!(entries.is_empty());
  }

  #[test]
  fn list_programs_uses_home() {
    let home = tempfile::tempdir().unwrap();
    let apps = home.path().join("Applications");
    std::fs::create_dir_all(&apps).unwrap();
    write_bundle(
      &apps,
      "Notes.app",
      "app {\n  bundle_id: com.tontoo.notes\n  version: \"1.0\"\n  executable: App/notes\n  name {\n    en_us: \"Notes\"\n  }\n}\n",
      false,
    );
    let old_home = std::env::var("HOME").ok();
    std::env::set_var("HOME", home.path());
    let entries = list_programs();
    if let Some(old) = old_home {
      std::env::set_var("HOME", old);
    }
    let notes: Vec<_> = entries
      .iter()
      .filter(|a| a.bundle_id == "com.tontoo.notes")
      .collect();
    assert_eq!(notes.len(), 1);
    assert_eq!(notes[0].source, AppSource::User);
    assert_eq!(notes[0].display_name, "Notes");
  }

  /// Valid `.tico` bytes via CoreIcon (dev-dependency), so container
  /// fixtures always pass structural validation.
  fn test_tico() -> Vec<u8> {
    use coreicon::generator::{Background, IconCanvas, Layer, LayerContent};
    use coreicon::{tico::Tico, Color};
    let canvas = IconCanvas::new()
      .background(Background::color(Color::BLACK))
      .layer(Layer::new(LayerContent::circle(512.0)))
      .glass();
    let path = std::env::temp_dir().join("corewindows-test-icon.tico");
    Tico::export(&canvas, "t", &path).expect("tico export");
    std::fs::read(&path).expect("read tico")
  }

  fn write_container(dir: &Path, name: &str, with_icon: bool) -> PathBuf {
    let path = dir.join(format!("{name}.app"));
    let mut manifest = AppManifest::new(
      format!("com.tontoo.{}", name.to_lowercase()),
      "2.0",
      format!("App/{name}"),
    );
    if with_icon {
      manifest.icon = Some("App/icon.tico".to_string());
    }
    manifest
      .names
      .push(("en_us".to_string(), name.to_string()));
    let mut builder = AppBuilder::new(name).expect("builder");
    builder.set_manifest(manifest);
    builder
      .add_executable(&format!("App/{name}"), b"binary".to_vec())
      .unwrap();
    if with_icon {
      builder.add_icon_tico("App/icon.tico", test_tico()).unwrap();
    }
    builder.write_to_file(&path).expect("write container");
    path
  }

  #[test]
  fn scan_dir_lists_container_app_with_icon() {
    let root = tempfile::tempdir().unwrap();
    let container = write_container(root.path(), "Demo", true);

    let entries = scan_dir(root.path(), AppSource::System);
    assert_eq!(entries.len(), 1);
    let app = &entries[0];
    assert_eq!(app.bundle_id, "com.tontoo.demo");
    assert_eq!(app.display_name, "Demo");
    assert_eq!(app.bundle_path, container);
    assert_eq!(app.source, AppSource::System);
    let icon = app.icon.icon_path.as_deref().expect("icon extracted");
    assert!(icon.is_file());
    assert_eq!(std::fs::read(icon).unwrap(), test_tico());
  }

  #[test]
  fn scan_dir_lists_container_without_icon() {
    let root = tempfile::tempdir().unwrap();
    write_container(root.path(), "Flat", false);

    let entries = scan_dir(root.path(), AppSource::System);
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].display_name, "Flat");
    assert!(entries[0].icon.icon_path.is_none());
  }

  #[test]
  fn scan_dir_skips_broken_app_files() {
    let root = tempfile::tempdir().unwrap();
    std::fs::write(root.path().join("NoInfo.app"), b"not a container").unwrap();
    write_bundle(
      root.path(),
      "NoId.app",
      "app {\n  version: \"1.0\"\n  executable: App/noid\n}\n",
      false,
    );
    std::fs::write(root.path().join("notes.txt"), b"plain").unwrap();

    assert!(scan_dir(root.path(), AppSource::System).is_empty());
  }

  #[test]
  fn manifest_converts_to_json() {
    let mut manifest = AppManifest::new("com.tontoo.x", "3.0", "App/x");
    manifest.icon = Some("App/icon.tico".to_string());
    manifest.names.push(("en_us".to_string(), "X\"Y".to_string()));
    let info = manifest_to_json(&manifest).expect("converts");
    assert_eq!(
      info.str_field("bundle_id").unwrap(),
      Some("com.tontoo.x".to_string())
    );
    let names = all_names(&info);
    assert_eq!(names.get("en_us").map(String::as_str), Some("X\"Y"));
    assert_eq!(
      crate::icon::icon_field(&info),
      Some("App/icon.tico".to_string())
    );
  }
}
