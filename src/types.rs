use foundation::serialization::JsonDocument;

/// Toolkit classification of an open window.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WindowType {
  /// Built with UIKit (`TONTOO_TOOLKIT=UIKit` or `toolkit: uikit`).
  UIKit,
  /// Built with TontooUI, i.e. a native TontooOS (TOS) app.
  TontooUi,
  /// Any other Linux app (GTK, Qt, Electron, Flatpak, X11, ...).
  Linux,
  /// No identifying information available.
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

  /// Snake-case wire spelling (`uikit`, `tontoui`, `linux`, `unknown`),
  /// matching the previous serde `rename_all` output.
  pub fn snake_str(self) -> &'static str {
    match self {
      Self::UIKit => "uikit",
      Self::TontooUi => "tontoui",
      Self::Linux => "linux",
      Self::Unknown => "unknown",
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
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RawWindow {
  /// Daemon-side window id (stable while the window is mapped).
  pub id: u64,
  /// XDG app id (`set_app_id`), if the client set one.
  pub app_id: Option<String>,
  /// XDG title (`set_title`), if the client set one.
  pub title: Option<String>,
  /// Owning client pid, if the daemon knows it.
  pub pid: Option<i32>,
  /// Whether the window is currently minimized to the dock.
  pub minimized: bool,
}

impl RawWindow {
  /// Parse one daemon window row. Missing optional fields default exactly
  /// like the previous serde implementation (`None` / `false`); a missing
  /// or unusable `id` or an out-of-range `pid` is an error.
  pub fn from_document(doc: &JsonDocument) -> Result<Self, String> {
    Ok(Self {
      id: doc
        .u64_field("id")
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "window row has no id".to_string())?,
      app_id: doc.str_field("app_id").map_err(|e| e.to_string())?,
      title: doc.str_field("title").map_err(|e| e.to_string())?,
      pid: doc
        .i64_field("pid")
        .map_err(|e| e.to_string())?
        .map(|v| {
          i32::try_from(v).map_err(|_| format!("window row pid out of range: {v}"))
        })
        .transpose()?,
      minimized: doc
        .bool_field("minimized")
        .map_err(|e| e.to_string())?
        .unwrap_or(false),
    })
  }
}

/// A fully classified open window: raw daemon data plus toolkit type,
/// bundle identity and the resolved app icon.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WindowInfo {
  /// Daemon-side window id (stable while the window is mapped).
  pub id: u64,
  /// XDG app id (`set_app_id`), if the client set one.
  pub app_id: Option<String>,
  /// XDG title (`set_title`), if the client set one.
  pub title: Option<String>,
  /// Owning client pid, if the daemon knows it.
  pub pid: Option<i32>,
  /// Whether the window is currently minimized to the dock.
  pub minimized: bool,
  /// Toolkit classification (see [`WindowType`]).
  pub window_type: WindowType,
  /// Bundle id from the owning `.app` `Info.tontoo`, if any.
  pub bundle_id: Option<String>,
  /// Display name (localized bundle name, bundle dir name, title or
  /// app id, whichever is available first).
  pub app_name: Option<String>,
  /// Resolved app icon for TontooOS bundles, if any.
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
    let doc = JsonDocument::parse(r#"{"id":7}"#).unwrap();
    let raw = RawWindow::from_document(&doc).unwrap();
    assert_eq!(raw.id, 7);
    assert!(raw.app_id.is_none());
    assert!(raw.title.is_none());
    assert!(raw.pid.is_none());
    assert!(!raw.minimized);
  }

  #[test]
  fn raw_window_minimized_flag() {
    let doc = JsonDocument::parse(r#"{"id":3,"minimized":true}"#).unwrap();
    let raw = RawWindow::from_document(&doc).unwrap();
    assert!(raw.minimized);
  }
}
