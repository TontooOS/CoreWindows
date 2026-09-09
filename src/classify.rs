use std::path::{Path, PathBuf};

use crate::types::{RawWindow, WindowType};

/// Environment variable every TontooOS UI toolkit sets with its own id
/// (`UIKit`, `TontooUI`). UIKit sets it in `App::run` (see
/// `uikit::app::mark_toolkit`).
pub const TOOLKIT_ENV_VAR: &str = "TONTOO_TOOLKIT";

/// Toolkit id reported by UIKit apps.
pub const TOOLKIT_UIKIT: &str = "UIKit";

/// Toolkit id reported by TontooUI apps.
pub const TOOLKIT_TONTOOUI: &str = "TontooUI";

/// GTK application id used by every UIKit app (`App::run`).
pub const UIKIT_APP_ID: &str = "org.tontoo.uikit";

/// Bundle id prefix reserved for native TontooOS apps.
pub const TONTOO_BUNDLE_PREFIX: &str = "com.tontoo.";

fn normalize_toolkit(value: &str) -> Option<WindowType> {
  match value.trim().to_lowercase().as_str() {
    "uikit" => Some(WindowType::UIKit),
    "tontooui" | "tontoo-ui" | "tontoo_ui" | "tos" => Some(WindowType::TontooUi),
    _ => None,
  }
}

/// Classification result for one raw window row.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Classification {
  /// Toolkit classification (see [`WindowType`]).
  pub window_type: WindowType,
  /// Bundle id from the owning `.app` `Info.tontoo`, if any.
  pub bundle_id: Option<String>,
  /// Display name (localized bundle name, bundle dir name, title or
  /// app id, whichever is available first).
  pub app_name: Option<String>,
  /// Owning `.app` bundle directory, if the process runs from one.
  pub bundle_dir: Option<PathBuf>,
  /// Parsed `Info.tontoo` of the owning bundle, if any.
  pub bundle_info: Option<serde_json::Value>,
}

/// Classify one raw window row.
///
/// Detection order (first hit wins for the type):
///
/// 1. `TONTOO_TOOLKIT` in `/proc/<pid>/environ` (`UIKit` / `TontooUI`).
/// 2. `toolkit` field of the owning `.app` `Info.tontoo`.
/// 3. Bundle id starting with `com.tontoo.` implies `TontooUI`.
/// 4. App id `org.tontoo.uikit` implies `UIKit`.
/// 5. Any remaining window with app id, title or pid is a `Linux` app,
///    rows without any identifying information are `Unknown`.
pub fn classify(raw: &RawWindow) -> Classification {
  let mut window_type: Option<WindowType> = None;
  let mut bundle_dir: Option<PathBuf> = None;
  let mut bundle_info: Option<serde_json::Value> = None;

  if let Some(pid) = raw.pid {
    if window_type.is_none() {
      window_type = toolkit_of_pid(pid);
    }
    if let Some((dir, info)) = bundle_of_pid(pid) {
      bundle_dir = Some(dir);
      bundle_info = Some(info);
    }
  }

  if window_type.is_none() {
    if let Some(info) = bundle_info.as_ref() {
      window_type = info
        .get("toolkit")
        .and_then(|v| v.as_str())
        .and_then(normalize_toolkit);
    }
  }

  let bundle_id = bundle_info
    .as_ref()
    .and_then(|info| info.get("bundle_id"))
    .and_then(|v| v.as_str())
    .map(str::to_owned);

  if window_type.is_none() {
    if let Some(id) = bundle_id.as_deref() {
      if id.starts_with(TONTOO_BUNDLE_PREFIX) {
        window_type = Some(WindowType::TontooUi);
      }
    }
  }

  if window_type.is_none() {
    if raw.app_id.as_deref() == Some(UIKIT_APP_ID) {
      window_type = Some(WindowType::UIKit);
    }
  }

  let app_name = app_name_for(raw, bundle_dir.as_deref(), bundle_info.as_ref());

  let window_type = window_type.unwrap_or_else(|| {
    if raw.app_id.is_some() || raw.title.is_some() || raw.pid.is_some() {
      WindowType::Linux
    } else {
      WindowType::Unknown
    }
  });

  Classification {
    window_type,
    bundle_id,
    app_name,
    bundle_dir,
    bundle_info,
  }
}

fn app_name_for(
  raw: &RawWindow,
  bundle_dir: Option<&Path>,
  info: Option<&serde_json::Value>,
) -> Option<String> {
  if let Some(info) = info {
    if let Some(name) = localized_name(info) {
      return Some(name);
    }
  }
  if let Some(dir) = bundle_dir {
    if let Some(stem) = dir.file_stem().and_then(|s| s.to_str()) {
      return Some(stem.to_owned());
    }
  }
  if let Some(title) = raw.title.clone() {
    return Some(title);
  }
  raw.app_id.clone()
}

/// Localized bundle display name: `name` is either a plain string or a
/// locale map (`{"en_us": ..., "de_de": ...}`), resolved with the same
/// fallback chain as FishPerms (`current locale`, `en_us`, first entry).
pub fn localized_name(info: &serde_json::Value) -> Option<String> {
  let name = info.get("name")?;
  if let Some(text) = name.as_str() {
    return Some(text.to_owned());
  }
  let names = name.as_object()?;
  let locale = crate::lang::current_locale();
  names
    .get(locale)
    .or_else(|| names.get("en_us"))
    .or_else(|| names.values().next())
    .and_then(|v| v.as_str())
    .map(str::to_owned)
}

/// Toolkit marker of a process from `/proc/<pid>/environ`.
/// Returns `None` when the process is gone, unreadable or unmarked.
pub fn toolkit_of_pid(pid: i32) -> Option<WindowType> {
  let raw = std::fs::read(format!("/proc/{pid}/environ")).ok()?;
  let prefix = format!("{TOOLKIT_ENV_VAR}=");
  raw
    .split(|b| *b == 0)
    .filter_map(|entry| std::str::from_utf8(entry).ok())
    .find_map(|entry| entry.strip_prefix(&prefix))
    .and_then(normalize_toolkit)
}

/// Owning `.app` bundle of a process: nearest ancestor directory ending in
/// `.app` (or the binary itself when named `*.app`), plus its parsed
/// `Info.tontoo`. Returns `None` for plain binaries and unreadable info.
pub fn bundle_of_pid(pid: i32) -> Option<(PathBuf, serde_json::Value)> {
  let exe = std::fs::read_link(format!("/proc/{pid}/exe")).ok()?;
  let exe_path = exe.canonicalize().ok()?;
  let bundle_dir = find_bundle_dir(&exe_path)?;
  let info_text = std::fs::read_to_string(bundle_dir.join("Info.tontoo")).ok()?;
  let info: serde_json::Value = serde_json::from_str(&info_text).ok()?;
  Some((bundle_dir, info))
}

fn find_bundle_dir(exe_path: &Path) -> Option<PathBuf> {
  if exe_path
    .file_name()
    .map(|n| n.to_string_lossy().ends_with(".app"))
    .unwrap_or(false)
  {
    return exe_path.parent().map(Path::to_path_buf);
  }
  let mut current: Option<&Path> = exe_path.parent();
  while let Some(dir) = current {
    if dir
      .file_name()
      .map(|n| n.to_string_lossy().ends_with(".app"))
      .unwrap_or(false)
    {
      return Some(dir.to_path_buf());
    }
    current = dir.parent();
  }
  None
}

#[cfg(test)]
mod tests {
  use super::*;

  fn raw(app_id: Option<&str>, title: Option<&str>, pid: Option<i32>) -> RawWindow {
    RawWindow {
      id: 1,
      app_id: app_id.map(str::to_owned),
      title: title.map(str::to_owned),
      pid,
    }
  }

  #[test]
  fn toolkit_markers_map() {
    assert_eq!(normalize_toolkit("UIKit"), Some(WindowType::UIKit));
    assert_eq!(normalize_toolkit("uikit"), Some(WindowType::UIKit));
    assert_eq!(normalize_toolkit("TontooUI"), Some(WindowType::TontooUi));
    assert_eq!(normalize_toolkit("tos"), Some(WindowType::TontooUi));
    assert_eq!(normalize_toolkit("gtk"), None);
  }

  #[test]
  fn uikit_app_id_fallback() {
    let c = classify(&raw(Some(UIKIT_APP_ID), Some("Demo"), None));
    assert_eq!(c.window_type, WindowType::UIKit);
    assert_eq!(c.app_name.as_deref(), Some("Demo"));
  }

  #[test]
  fn foreign_app_is_linux() {
    let c = classify(&raw(Some("org.mozilla.firefox"), Some("Firefox"), Some(4242)));
    assert_eq!(c.window_type, WindowType::Linux);
    assert_eq!(c.app_name.as_deref(), Some("Firefox"));
    assert!(c.bundle_id.is_none());
  }

  #[test]
  fn empty_row_is_unknown() {
    let c = classify(&raw(None, None, None));
    assert_eq!(c.window_type, WindowType::Unknown);
    assert!(c.app_name.is_none());
  }

  #[test]
  fn localized_name_prefers_current_locale() {
    let info: serde_json::Value =
      serde_json::from_str(r#"{"name":{"en_us":"Finder","de_de":"FinderDE"}}"#).unwrap();
    let name = localized_name(&info).unwrap();
    assert!(name == "Finder" || name == "FinderDE");
  }

  #[test]
  fn localized_name_plain_string() {
    let info: serde_json::Value = serde_json::from_str(r#"{"name":"Terminal"}"#).unwrap();
    assert_eq!(localized_name(&info).as_deref(), Some("Terminal"));
  }

  #[test]
  fn missing_pid_is_not_a_bundle() {
    assert!(bundle_of_pid(i32::MAX).is_none());
    assert!(toolkit_of_pid(i32::MAX).is_none());
  }
}
