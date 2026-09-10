//! C FFI exports for CoreWindows.

use std::ffi::{CStr, CString};
use std::os::raw::c_char;

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
    Ok(windows) => match serde_json::to_string(&windows) {
      Ok(json) => CString::new(json).unwrap_or_default().into_raw(),
      Err(_) => std::ptr::null_mut(),
    },
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
  match serde_json::to_string(&crate::list_programs()) {
    Ok(json) => CString::new(json).unwrap_or_default().into_raw(),
    Err(_) => std::ptr::null_mut(),
  }
}
