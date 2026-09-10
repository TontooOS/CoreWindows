use serde::{Deserialize, Serialize};

/// Toolkit classification of an open window.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WindowType {
  /// Built with UIKit (`TONTOO_TOOLKIT=UIKit` or `toolkit: uikit`).
  #[serde(rename = "uikit")]
  UIKit,
  /// Built with TontooUI, i.e. a native TontooOS (TOS) app.
  #[serde(rename = "tontoui")]
  TontooUi,
  /// Any other Linux app (GTK, Qt, Electron, Flatpak, X11, ...).
  #[serde(rename = "linux")]
  Linux,
  /// No identifying information available.
  #[serde(rename = "unknown")]
  Unknown,
}

impl WindowType {
  /// Display name of the variant (`UIKit`, `TontooUI`, `Linux`, `Unknown`).
  pub fn as_str(self) -> &'static str {
    match self {
      Self::UIKit => "UIKit",
      Self::TontooUi => "TontooUI",
      Self::Linux => "Linux",
      Self::Unknown => "Unknown",
    }
  }
}

impl std::fmt::Display for WindowType {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    f.write_str(self.as_str())
  }
}

/// One raw window row as reported by the window daemon.
///
/// This mirrors the daemon reply 1:1; use [`crate::WindowInfo`] for the
/// enriched form with toolkit type and app icon.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RawWindow {
  /// Daemon-side window id (stable while the window is mapped).
  pub id: u64,
  /// XDG app id (`set_app_id`), if the client set one.
  #[serde(default)]
  pub app_id: Option<String>,
  /// XDG title (`set_title`), if the client set one.
  #[serde(default)]
  pub title: Option<String>,
  /// Owning client pid, if the daemon knows it.
  #[serde(default)]
  pub pid: Option<i32>,
  /// Whether the window is currently minimized to the dock.
  #[serde(default)]
  pub minimized: bool,
}

/// A fully classified open window: raw daemon data plus toolkit type,
/// bundle identity and the resolved app icon.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WindowInfo {
  /// Daemon-side window id (stable while the window is mapped).
  pub id: u64,
  /// XDG app id (`set_app_id`), if the client set one.
  #[serde(default)]
  pub app_id: Option<String>,
  /// XDG title (`set_title`), if the client set one.
  #[serde(default)]
  pub title: Option<String>,
  /// Owning client pid, if the daemon knows it.
  #[serde(default)]
  pub pid: Option<i32>,
  /// Whether the window is currently minimized to the dock.
  #[serde(default)]
  pub minimized: bool,
  /// Toolkit classification (see [`WindowType`]).
  pub window_type: WindowType,
  /// Bundle id from the owning `.app` `Info.tontoo`, if any.
  #[serde(default)]
  pub bundle_id: Option<String>,
  /// Display name (localized bundle name, bundle dir name, title or
  /// app id, whichever is available first).
  #[serde(default)]
  pub app_name: Option<String>,
  /// Resolved app icon for TontooOS bundles, if any.
  #[serde(default)]
  pub icon: Option<crate::icon::AppIcon>,
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn window_type_names() {
    assert_eq!(WindowType::UIKit.as_str(), "UIKit");
    assert_eq!(WindowType::TontooUi.as_str(), "TontooUI");
    assert_eq!(WindowType::Linux.as_str(), "Linux");
    assert_eq!(WindowType::Unknown.as_str(), "Unknown");
  }

  #[test]
  fn raw_window_sparse_json() {
    let raw: RawWindow = serde_json::from_str(r#"{"id":7}"#).unwrap();
    assert_eq!(raw.id, 7);
    assert!(raw.app_id.is_none());
    assert!(raw.title.is_none());
    assert!(raw.pid.is_none());
    assert!(!raw.minimized);
  }

  #[test]
  fn raw_window_minimized_flag() {
    let raw: RawWindow =
      serde_json::from_str(r#"{"id":3,"minimized":true}"#).unwrap();
    assert!(raw.minimized);
  }
}
