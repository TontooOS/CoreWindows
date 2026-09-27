//! C FFI exports for CoreWindows.

use std::ffi::{CStr, CString};
use std::os::raw::c_char;

use foundation::serialization::{JsonObject, JSONSerialization};

fn opt_str(obj: &mut JsonObject, key: &str, value: Option<&str>) {
  match value {
    Some(text) => {
      obj.field_str(key, text);
    }
    None => {
      obj.field_null(key);
    }
  }
}

fn icon_json(icon: &crate::icon::AppIcon) -> String {
  let mut obj = JsonObject::new();
  obj.field_str("bundle_id", &icon.bundle_id);
  obj.field_str("bundle_path", &icon.bundle_path.to_string_lossy());
  match &icon.icon_path {
    Some(path) => obj.field_str("icon_path", &path.to_string_lossy()),
    None => obj.field_null("icon_path"),
  };
  obj.field_str("app_name", &icon.app_name);
  obj.build(false).unwrap_or_else(|_| "{}".to_string())
}

fn window_json(window: &crate::types::WindowInfo) -> String {
  let mut obj = JsonObject::new();
  obj.field_u64("id", window.id);
  opt_str(&mut obj, "app_id", window.app_id.as_deref());
  opt_str(&mut obj, "title", window.title.as_deref());
  match window.pid {
    Some(pid) => {
      obj.field_i64("pid", pid as i64);
    }
    None => {
      obj.field_null("pid");
    }
  }
  obj.field_bool("minimized", window.minimized);
  obj.field_str("window_type", window.window_type.snake_str());
  opt_str(&mut obj, "bundle_id", window.bundle_id.as_deref());
  opt_str(&mut obj, "app_name", window.app_name.as_deref());
  match &window.icon {
    Some(icon) => {
      let _ = obj.field_raw("icon", &icon_json(icon));
    }
    None => {
      obj.field_null("icon");
    }
  }
  obj.build(false).unwrap_or_else(|_| "{}".to_string())
}

fn app_entry_json(entry: &crate::programs::AppEntry) -> String {
  let mut obj = JsonObject::new();
  obj.field_str("bundle_id", &entry.bundle_id);
  let names_json =
    JSONSerialization::stringify_string_map(&entry.names, false).unwrap_or_else(|_| "{}".to_string());
  let _ = obj.field_raw("names", &names_json);
  obj.field_str("display_name", &entry.display_name);
  obj.field_str("bundle_path", &entry.bundle_path.to_string_lossy());
  obj.field_str("source", entry.source.snake_str());
  let _ = obj.field_raw("icon", &icon_json(&entry.icon));
  obj.build(false).unwrap_or_else(|_| "{}".to_string())
}

fn json_array(items: &[String]) -> String {
  format!("[{}]", items.join(","))
}

unsafe fn read_str(ptr: *const c_char) -> Option<String> {
  if ptr.is_null() {
    return None;
  }
  CStr::from_ptr(ptr).to_str().ok().map(str::to_owned)
}

/// The framework version as a static C string.
#[no_mangle]
pub extern "C" fn tontoo_corewindows_version() -> *const c_char {
  concat!(env!("CARGO_PKG_VERSION"), "\0").as_ptr() as *const c_char
}

/// Ping the window daemon: 1 on pong, 0 otherwise.
///
/// When `socket_path` is null or empty the default socket is used.
///
/// # Safety
///
/// `socket_path` must be NUL-terminated or null.
#[no_mangle]
pub unsafe extern "C" fn tontoo_corewindows_ping(socket_path: *const c_char) -> i32 {
  let provider = match read_str(socket_path) {
    Some(path) if !path.is_empty() => crate::WindowsProvider::with_socket(path),
    _ => crate::WindowsProvider::from_env(),
  };
  provider.ping().unwrap_or(false) as i32
}

/// List currently open windows as a JSON array of [`crate::WindowInfo`].
///
/// Returns null when the daemon is unreachable. Free the string with
/// [`tontoo_corewindows_string_free`]. When `socket_path` is null or empty
/// the default socket (`WINDOWS_SOCKET` or `/run/tontoo-windows.sock`) is
/// used.
///
/// # Safety
///
/// `socket_path` must be NUL-terminated or null.
#[no_mangle]
pub unsafe extern "C" fn tontoo_corewindows_list_windows(
  socket_path: *const c_char,
) -> *mut c_char {
  let provider = match read_str(socket_path) {
    Some(path) if !path.is_empty() => crate::WindowsProvider::with_socket(path),
    _ => crate::WindowsProvider::from_env(),
  };
  match provider.windows() {
    Ok(windows) => {
      let items: Vec<String> = windows.iter().map(window_json).collect();
      CString::new(json_array(&items)).unwrap_or_default().into_raw()
    }
    Err(_) => std::ptr::null_mut(),
  }
}

/// Free a string returned by this library.
///
/// # Safety
///
/// `s` must be a pointer returned by this API or null.
#[no_mangle]
pub unsafe extern "C" fn tontoo_corewindows_string_free(s: *mut c_char) {
  if !s.is_null() {
    drop(CString::from_raw(s));
  }
}

unsafe fn provider_for(socket_path: *const c_char) -> crate::WindowsProvider {
  match read_str(socket_path) {
    Some(path) if !path.is_empty() => crate::WindowsProvider::with_socket(path),
    _ => crate::WindowsProvider::from_env(),
  }
}

fn action_code(result: crate::Result<()>) -> i32 {
  match result {
    Ok(()) => 0,
    Err(_) => -1,
  }
}

/// Minimize a window (iconify). Returns 0 on success, -1 on error.
///
/// # Safety
///
/// `socket_path` must be NUL-terminated or null (default socket).
#[no_mangle]
pub unsafe extern "C" fn tontoo_corewindows_minimize_window(
  socket_path: *const c_char,
  id: u64,
) -> i32 {
  action_code(provider_for(socket_path).minimize_window(id))
}

/// Restore a window minimized to the dock. Returns 0 on success, -1 on
/// error (unknown id, not minimized, or client gone).
///
/// # Safety
///
/// `socket_path` must be NUL-terminated or null (default socket).
#[no_mangle]
pub unsafe extern "C" fn tontoo_corewindows_restore_window(
  socket_path: *const c_char,
  id: u64,
) -> i32 {
  action_code(provider_for(socket_path).restore_window(id))
}

/// Set fullscreen state of a window (`fullscreen` != 0 = fullscreen like
/// the green UIKit traffic light, 0 = windowed). Returns 0 on success,
/// -1 on error.
///
/// # Safety
///
/// `socket_path` must be NUL-terminated or null (default socket).
#[no_mangle]
pub unsafe extern "C" fn tontoo_corewindows_set_fullscreen(
  socket_path: *const c_char,
  id: u64,
  fullscreen: i32,
) -> i32 {
  action_code(provider_for(socket_path).set_fullscreen(id, fullscreen != 0))
}

/// Gracefully close a window: the daemon asks the client to close, the app
/// itself may show a save dialog (e.g. LibreOffice). Nothing is killed.
/// Returns 0 on success, -1 on error.
///
/// # Safety
///
/// `socket_path` must be NUL-terminated or null (default socket).
#[no_mangle]
pub unsafe extern "C" fn tontoo_corewindows_close_window(
  socket_path: *const c_char,
  id: u64,
) -> i32 {
  action_code(provider_for(socket_path).close_window(id))
}

/// Force quit the owner of a window (`SIGKILL`, no save dialog).
/// Returns 0 on success, -1 on daemon error, -2 when the window id is
/// unknown or carries no pid.
///
/// # Safety
///
/// `socket_path` must be NUL-terminated or null (default socket).
#[no_mangle]
pub unsafe extern "C" fn tontoo_corewindows_force_quit_window(
  socket_path: *const c_char,
  id: u64,
) -> i32 {
  match provider_for(socket_path).force_quit_window(id) {
    Ok(()) => 0,
    Err(crate::WindowsError::Parse(_)) => -2,
    Err(_) => -1,
  }
}

/// Force quit a process id (`SIGKILL`, no save dialog).
/// Returns 0 on success, -1 on error (unknown pid or denied).
#[no_mangle]
pub extern "C" fn tontoo_corewindows_force_quit_pid(pid: i32) -> i32 {
  action_code(crate::force_quit_pid(pid))
}

/// List installed programs (`~/Applications` and `/Applications`) as a
/// JSON array of [`crate::AppEntry`] (bundle id, all names, display name,
/// bundle path, source, icon). Never null: an empty array when nothing is
/// found. Free the string with [`tontoo_corewindows_string_free`].
#[no_mangle]
pub extern "C" fn tontoo_corewindows_list_programs() -> *mut c_char {
  let programs = crate::list_programs();
  let items: Vec<String> = programs.iter().map(app_entry_json).collect();
  CString::new(json_array(&items)).unwrap_or_default().into_raw()
}

#[cfg(test)]
mod tests {
  use super::*;
  use foundation::serialization::JsonDocument;

  fn sample_window() -> crate::types::WindowInfo {
    crate::types::WindowInfo {
      id: 7,
      app_id: Some("org.test.app".to_owned()),
      title: Some("Test \"quoted\"".to_owned()),
      pid: Some(4242),
      minimized: true,
      window_type: crate::types::WindowType::TontooUi,
      bundle_id: None,
      app_name: Some("TestApp".to_owned()),
      icon: Some(crate::icon::AppIcon {
        bundle_id: "com.tontoo.test".to_owned(),
        bundle_path: std::path::PathBuf::from("/Applications/Test.app"),
        icon_path: None,
        app_name: "TestApp".to_owned(),
      }),
    }
  }

  #[test]
  fn window_json_roundtrip_matches_serde_shape() {
    let json = window_json(&sample_window());
    let doc = JsonDocument::parse(&json).unwrap();
    assert_eq!(doc.u64_field("id").unwrap(), Some(7));
    assert_eq!(
      doc.str_field("app_id").unwrap().as_deref(),
      Some("org.test.app")
    );
    assert_eq!(
      doc.str_field("title").unwrap().as_deref(),
      Some("Test \"quoted\"")
    );
    assert_eq!(
      doc.str_field("window_type").unwrap().as_deref(),
      Some("tontoui")
    );
    assert_eq!(doc.bool_field("minimized").unwrap(), Some(true));
    assert!(doc.nested("bundle_id").unwrap().is_none());
    let icon = doc.nested("icon").unwrap().expect("icon object");
    assert_eq!(
      icon.str_field("bundle_id").unwrap().as_deref(),
      Some("com.tontoo.test")
    );
    assert!(icon.nested("icon_path").unwrap().is_none());
  }

  #[test]
  fn app_entry_json_roundtrip() {
    let entry = crate::programs::AppEntry {
      bundle_id: "com.tontoo.demo".to_owned(),
      names: [("en_us".to_owned(), "Demo".to_owned())].into_iter().collect(),
      display_name: "Demo".to_owned(),
      bundle_path: std::path::PathBuf::from("/Applications/Demo.app"),
      source: crate::programs::AppSource::System,
      icon: crate::icon::AppIcon {
        bundle_id: "com.tontoo.demo".to_owned(),
        bundle_path: std::path::PathBuf::from("/Applications/Demo.app"),
        icon_path: Some(std::path::PathBuf::from("/Applications/Demo.app/Icon.png")),
        app_name: "Demo".to_owned(),
      },
    };
    let json = app_entry_json(&entry);
    let doc = JsonDocument::parse(&json).unwrap();
    assert_eq!(
      doc.str_field("bundle_id").unwrap().as_deref(),
      Some("com.tontoo.demo")
    );
    assert_eq!(
      doc.str_field("source").unwrap().as_deref(),
      Some("system")
    );
    let names = doc.string_map_field("names").unwrap();
    assert_eq!(names.get("en_us").map(String::as_str), Some("Demo"));
    let icon = doc.nested("icon").unwrap().expect("icon object");
    assert_eq!(
      icon.str_field("icon_path").unwrap().as_deref(),
      Some("/Applications/Demo.app/Icon.png")
    );
  }
}
