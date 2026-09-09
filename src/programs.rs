use std::collections::HashMap;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::classify::localized_name;
use crate::icon::{resolve_icon, AppIcon};

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
/// Each `*.app` directory with a readable `Info.tontoo` carrying a
/// `bundle_id` becomes one [`AppEntry`]; anything else (plain files,
/// bundles without info, invalid JSON, missing `bundle_id`) is skipped
/// silently. Results are sorted by display name (case-insensitive).
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

/// Scan one applications directory for `.app` bundles.
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
  path.is_dir()
    && path
      .file_name()
      .map(|n| n.to_string_lossy().ends_with(".app"))
      .unwrap_or(false)
}

fn read_app(bundle_dir: &Path, source: AppSource) -> Option<AppEntry> {
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
}
