use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

/// Resolved app icon for a TontooOS `.app` bundle.
///
/// The icon file is located through the `icon` field of the bundle
/// `Info.tontoo` (relative to the bundle root). Rendering the file is left
/// to the caller (e.g. via CoreIcon); this struct only carries paths.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AppIcon {
  /// Bundle id from `Info.tontoo` (`bundle_id`).
  pub bundle_id: String,
  /// Absolute bundle directory (`....app`).
  pub bundle_path: PathBuf,
  /// Absolute icon file, when the `icon` field resolves to an existing file.
  #[serde(default)]
  pub icon_path: Option<PathBuf>,
  /// Display name of the app.
  pub app_name: String,
}

/// Fallback icon file names probed inside the bundle when `Info.tontoo`
/// carries no usable `icon` field (relative to the bundle root).
pub const ICON_PROBE_FILES: &[&str] = &[
  "Icon.png",
  "icon.png",
  "Icon.jpg",
  "icon.jpg",
  "Resources/Icon.png",
  "Resources/icon.png",
  "Contents/Resources/Icon.png",
];

/// Resolve the app icon for a bundle directory.
///
/// `info` is the parsed `Info.tontoo` (may be `None` for bundles without
/// one). The `icon` field accepts either a plain relative path string or
/// an object with a `path` string. When the field is missing or points at
/// a non-existent file, [`ICON_PROBE_FILES`] is probed; `icon_path` is
/// `None` when nothing exists.
pub fn resolve_icon(
  bundle_dir: &Path,
  bundle_id: &str,
  app_name: &str,
  info: Option<&serde_json::Value>,
) -> AppIcon {
  let mut icon_path: Option<PathBuf> = None;

  if let Some(info) = info {
    if let Some(rel) = icon_field(info) {
      let candidate = bundle_dir.join(&rel);
      if candidate.is_file() {
        icon_path = Some(candidate);
      }
    }
  }

  if icon_path.is_none() {
    icon_path = ICON_PROBE_FILES
      .iter()
      .map(|rel| bundle_dir.join(rel))
      .find(|p| p.is_file());
  }

  AppIcon {
    bundle_id: bundle_id.to_owned(),
    bundle_path: bundle_dir.to_owned(),
    icon_path,
    app_name: app_name.to_owned(),
  }
}

fn icon_field(info: &serde_json::Value) -> Option<String> {
  let icon = info.get("icon")?;
  if let Some(path) = icon.as_str() {
    return Some(path.to_owned());
  }
  icon
    .get("path")
    .and_then(|v| v.as_str())
    .map(str::to_owned)
}

#[cfg(test)]
mod tests {
  use super::*;
  use std::io::Write;

  #[test]
  fn icon_string_field_resolves() {
    let dir = tempfile::tempdir().unwrap();
    let icon = dir.path().join("MyIcon.png");
    std::fs::File::create(&icon)
      .unwrap()
      .write_all(b"png")
      .unwrap();
    let info: serde_json::Value =
      serde_json::from_str(r#"{"icon":"MyIcon.png"}"#).unwrap();
    let resolved = resolve_icon(dir.path(), "com.tontoo.demo", "Demo", Some(&info));
    assert_eq!(resolved.icon_path.as_deref(), Some(icon.as_path()));
  }

  #[test]
  fn icon_object_field_resolves() {
    let dir = tempfile::tempdir().unwrap();
    let icon = dir.path().join("Resources").join("Icon.png");
    std::fs::create_dir_all(icon.parent().unwrap()).unwrap();
    std::fs::File::create(&icon)
      .unwrap()
      .write_all(b"png")
      .unwrap();
    let info: serde_json::Value =
      serde_json::from_str(r#"{"icon":{"path":"Resources/Icon.png"}}"#).unwrap();
    let resolved = resolve_icon(dir.path(), "com.tontoo.demo", "Demo", Some(&info));
    assert_eq!(resolved.icon_path.as_deref(), Some(icon.as_path()));
  }

  #[test]
  fn icon_probe_fallback() {
    let dir = tempfile::tempdir().unwrap();
    let icon = dir.path().join("Icon.png");
    std::fs::File::create(&icon)
      .unwrap()
      .write_all(b"png")
      .unwrap();
    let resolved = resolve_icon(dir.path(), "com.tontoo.demo", "Demo", None);
    assert_eq!(resolved.icon_path.as_deref(), Some(icon.as_path()));
  }

  #[test]
  fn icon_missing_is_none() {
    let dir = tempfile::tempdir().unwrap();
    let resolved = resolve_icon(dir.path(), "com.tontoo.demo", "Demo", None);
    assert!(resolved.icon_path.is_none());
    assert_eq!(resolved.bundle_id, "com.tontoo.demo");
  }
}
