use std::collections::HashMap;
use std::io::Read as _;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::classify::localized_name;
use crate::icon::{icon_field, resolve_icon, AppIcon};

/// System-wide applications directory (OS root).
pub const SYSTEM_APPLICATIONS_DIR: &str = "/Applications";

/// Per-user applications directory name (resolved against `HOME`).
pub const USER_APPLICATIONS_DIR: &str = "Applications";

/// Where an app bundle was found.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
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
}

impl std::fmt::Display for AppSource {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    f.write_str(self.as_str())
  }
}

/// One installed TontooOS application (`.app` bundle).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AppEntry {
  /// Bundle id from `Info.tontoo` (`bundle_id`).
  pub bundle_id: String,
  /// All known names: every locale of the `Info.tontoo` `name` map.
  /// A plain string `name` becomes a single `"default"` entry.
  pub names: HashMap<String, String>,
  /// Display name for the current locale (locale, `en_us`, first entry,
  /// then bundle dir stem fallback).
  pub display_name: String,
  /// Absolute bundle directory (`....app`).
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
/// `Info.tontoo` carrying a `bundle_id`, and zipped `.app` files (TBuild
/// output) with an `Info.tontoo` entry carrying a `bundle_id`. Anything else
/// (plain files, bundles without info, invalid JSON, missing `bundle_id`,
/// non-zip `.app` files) is skipped silently. Results are sorted by display
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
/// zipped `.app` files, see [`list_programs`]).
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
  // Installed bundles are directories; TBuild ships them as zipped `.app`
  // files (opened by `tapp` without extraction). Anything else is ignored.
  path.is_dir() || path.is_file()
}

fn read_app(bundle: &Path, source: AppSource) -> Option<AppEntry> {
  if bundle.is_dir() {
    read_dir_bundle(bundle, source)
  } else {
    read_zip_bundle(bundle, source)
  }
}

fn read_dir_bundle(bundle_dir: &Path, source: AppSource) -> Option<AppEntry> {
  let info_text = std::fs::read_to_string(bundle_dir.join("Info.tontoo")).ok()?;
  let info: serde_json::Value = serde_json::from_str(&info_text).ok()?;
  let bundle_id = info
    .get("bundle_id")
    .and_then(|v| v.as_str())
    .filter(|s| !s.is_empty())?;
  let names = all_names(&info);
  let display_name = localized_name(&info).or_else(|| {
    bundle_dir
      .file_stem()
      .and_then(|s| s.to_str())
      .map(str::to_owned)
  })?;
  let icon = resolve_icon(bundle_dir, bundle_id, &display_name, Some(&info));
  Some(AppEntry {
    bundle_id: bundle_id.to_owned(),
    names,
    display_name,
    bundle_path: bundle_dir.to_owned(),
    source,
    icon,
  })
}

/// Read a zipped `.app` file (TBuild output): the `Info.tontoo` entry may sit
/// at the archive root or under a single top-level `<Name>.app/` directory.
/// `bundle_path` points at the `.app` file itself, which `tapp` opens without
/// prior extraction, so launching works unchanged.
fn read_zip_bundle(zip_path: &Path, source: AppSource) -> Option<AppEntry> {
  let file = std::fs::File::open(zip_path).ok()?;
  let mut archive = zip::ZipArchive::new(file).ok()?;
  let info_name = (0..archive.len())
    .filter_map(|i| {
      archive
        .by_index(i)
        .ok()
        .map(|entry| entry.name().to_owned())
    })
    .find(|name| name == "Info.tontoo" || name.ends_with("/Info.tontoo"))?;
  let prefix = info_name
    .rsplit_once("Info.tontoo")
    .map(|(prefix, _)| prefix)
    .unwrap_or("");
  let mut info_text = String::new();
  archive
    .by_name(&info_name)
    .ok()?
    .read_to_string(&mut info_text)
    .ok()?;
  let info: serde_json::Value = serde_json::from_str(&info_text).ok()?;
  let bundle_id = info
    .get("bundle_id")
    .and_then(|v| v.as_str())
    .filter(|s| !s.is_empty())?;
  let names = all_names(&info);
  let display_name = localized_name(&info).or_else(|| {
    zip_path
      .file_stem()
      .and_then(|s| s.to_str())
      .map(str::to_owned)
  })?;
  let icon = resolve_zip_icon(&mut archive, prefix, zip_path, bundle_id, &display_name, &info);
  Some(AppEntry {
    bundle_id: bundle_id.to_owned(),
    names,
    display_name,
    bundle_path: zip_path.to_owned(),
    source,
    icon,
  })
}

/// Resolve the icon of a zipped bundle: the `icon` field first, then
/// [`crate::icon::ICON_PROBE_FILES`], extracted once into the temp icon
/// cache. `icon_path` is `None` when the archive holds no usable file.
fn resolve_zip_icon(
  archive: &mut zip::ZipArchive<std::fs::File>,
  prefix: &str,
  zip_path: &Path,
  bundle_id: &str,
  app_name: &str,
  info: &serde_json::Value,
) -> AppIcon {
  let mut candidates = Vec::new();
  if let Some(rel) = icon_field(info) {
    candidates.push(rel);
  }
  candidates.extend(
    crate::icon::ICON_PROBE_FILES
      .iter()
      .map(|s| s.to_string()),
  );
  let bundle_path = zip_path.to_owned();
  for rel in candidates {
    let in_zip = format!("{prefix}{rel}");
    let present = archive
      .by_name(&in_zip)
      .map(|entry| entry.is_file())
      .unwrap_or(false);
    if !present {
      continue;
    }
    if let Some(out) = extract_zip_icon(archive, &in_zip, prefix, zip_path, bundle_id) {
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

/// Extract one icon entry into the temp icon cache
/// (`$TMPDIR/tontoo-corewindows-icons/<bundle-id>/<path>`). Cached files are
/// reused; a bundle newer than its cache is extracted again.
fn extract_zip_icon(
  archive: &mut zip::ZipArchive<std::fs::File>,
  in_zip: &str,
  prefix: &str,
  zip_path: &Path,
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
  let rel = in_zip.strip_prefix(prefix).unwrap_or(in_zip);
  let out = std::env::temp_dir()
    .join("tontoo-corewindows-icons")
    .join(safe_id)
    .join(rel);
  if out.is_file() && !zip_newer_than(zip_path, &out) {
    return Some(out);
  }
  let parent = out.parent()?;
  std::fs::create_dir_all(parent).ok()?;
  let mut entry = archive.by_name(in_zip).ok()?;
  let mut out_file = std::fs::File::create(&out).ok()?;
  std::io::copy(&mut entry, &mut out_file).ok()?;
  Some(out)
}

/// Whether the `.app` file was modified after its cached icon extract.
fn zip_newer_than(zip_path: &Path, cached: &Path) -> bool {
  match (
    std::fs::metadata(zip_path).and_then(|m| m.modified()),
    std::fs::metadata(cached).and_then(|m| m.modified()),
  ) {
    (Ok(zip_time), Ok(cache_time)) => zip_time > cache_time,
    _ => true,
  }
}

/// Every known name of a bundle: the full `name` locale map, or a single
/// `"default"` entry for a plain string `name`. Empty when `name` is
/// missing or unusable.
pub fn all_names(info: &serde_json::Value) -> HashMap<String, String> {
  let mut names = HashMap::new();
  match info.get("name") {
    Some(value) if value.is_string() => {
      if let Some(text) = value.as_str() {
        names.insert("default".to_owned(), text.to_owned());
      }
    }
    Some(value) if value.is_object() => {
      for (locale, text) in value.as_object().expect("checked is_object") {
        if let Some(text) = text.as_str() {
          names.insert(locale.clone(), text.to_owned());
        }
      }
    }
    _ => {}
  }
  names
}

#[cfg(test)]
mod tests {
  use super::*;
  use std::io::Write as _;

  fn write_bundle(dir: &Path, name: &str, info: &str, icon: bool) -> PathBuf {
    let bundle = dir.join(name);
    std::fs::create_dir_all(&bundle).unwrap();
    std::fs::File::create(bundle.join("Info.tontoo"))
      .unwrap()
      .write_all(info.as_bytes())
      .unwrap();
    if icon {
      std::fs::File::create(bundle.join("Icon.png"))
        .unwrap()
        .write_all(b"png")
        .unwrap();
    }
    bundle
  }

  #[test]
  fn scan_dir_reads_names_and_icon() {
    let root = tempfile::tempdir().unwrap();
    write_bundle(
      root.path(),
      "Finder.app",
      r#"{"bundle_id":"com.tontoo.finder","name":{"en_us":"Finder","de_de":"FinderDE"},"icon":"Icon.png"}"#,
      true,
    );
    write_bundle(root.path(), "Broken.app", "not json", false);
    write_bundle(root.path(), "NoId.app", r#"{"name":"NoId"}"#, false);
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
    let info: serde_json::Value = serde_json::from_str(r#"{"name":"Terminal"}"#).unwrap();
    let names = all_names(&info);
    assert_eq!(names.len(), 1);
    assert_eq!(names.get("default").map(String::as_str), Some("Terminal"));
  }

  #[test]
  fn missing_name_is_empty() {
    let info: serde_json::Value = serde_json::from_str(r#"{"bundle_id":"x"}"#).unwrap();
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
      r#"{"bundle_id":"com.tontoo.notes","name":"Notes"}"#,
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

  fn write_zip(dir: &Path, name: &str, files: &[(&str, &[u8])]) -> PathBuf {
    let path = dir.join(name);
    let file = std::fs::File::create(&path).unwrap();
    let mut zip = zip::ZipWriter::new(file);
    for (entry, data) in files {
      zip
        .start_file(*entry, zip::write::SimpleFileOptions::default())
        .unwrap();
      zip.write_all(data).unwrap();
    }
    zip.finish().unwrap();
    path
  }

  #[test]
  fn scan_dir_lists_zipped_app_with_icon() {
    let root = tempfile::tempdir().unwrap();
    let zip_path = write_zip(
      root.path(),
      "Demo.app",
      &[
        (
          "Demo.app/Info.tontoo",
          br#"{"bundle_id":"com.tontoo.demo","name":{"en_us":"Demo","de_de":"DemoDE"}}"#,
        ),
        ("Demo.app/Resources/icon.png", b"png"),
      ],
    );

    let entries = scan_dir(root.path(), AppSource::System);
    assert_eq!(entries.len(), 1);
    let app = &entries[0];
    assert_eq!(app.bundle_id, "com.tontoo.demo");
    assert!(app.display_name == "Demo" || app.display_name == "DemoDE");
    assert_eq!(app.bundle_path, zip_path);
    assert_eq!(app.source, AppSource::System);
    let icon = app.icon.icon_path.as_deref().expect("zip icon extracted");
    assert!(icon.is_file());
    assert_eq!(std::fs::read(icon).unwrap(), b"png");
  }

  #[test]
  fn scan_dir_lists_flat_zip_without_icon() {
    let root = tempfile::tempdir().unwrap();
    write_zip(
      root.path(),
      "Flat.app",
      &[("Info.tontoo", br#"{"bundle_id":"com.tontoo.flat","name":"Flat"}"#)],
    );

    let entries = scan_dir(root.path(), AppSource::System);
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].display_name, "Flat");
    assert!(entries[0].icon.icon_path.is_none());
  }

  #[test]
  fn scan_dir_skips_broken_app_files() {
    let root = tempfile::tempdir().unwrap();
    write_zip(root.path(), "NoInfo.app", &[("readme.txt", b"hi")]);
    write_zip(
      root.path(),
      "NoId.app",
      &[("NoId.app/Info.tontoo", br#"{"name":"NoId"}"#)],
    );
    std::fs::write(root.path().join("Binary.app"), b"not a zip").unwrap();
    std::fs::write(root.path().join("notes.txt"), b"plain").unwrap();

    assert!(scan_dir(root.path(), AppSource::System).is_empty());
  }
}
