use std::io::{BufRead, BufReader, Write};
use std::os::unix::net::UnixStream;
use std::path::{Path, PathBuf};
use std::time::Duration;

use crate::classify::classify;
use crate::error::{Result, WindowsError};
use crate::icon::resolve_icon;
use crate::types::{RawWindow, WindowInfo};

/// Default window daemon socket path (mirrors the daemon default).
pub const DEFAULT_SOCKET_PATH: &str = "/run/tontoo-windows.sock";

/// Socket read timeout per request.
const REQUEST_TIMEOUT: Duration = Duration::from_secs(10);

/// Client handle for the window daemon. All window rows come from the
/// daemon over its unix socket, a missing daemon surfaces as an error.
///
/// The daemon side (TontooCompositor) answers `{"id":1,"op":"list_windows"}`
/// with `{"ok":true,"result":{"windows":[...]}}`, one object per mapped
/// window: `{"id":1,"app_id":"...","title":"...","pid":1234}` (all fields
/// but `id` optional).
#[derive(Debug, Clone)]
pub struct WindowsProvider {
  socket_path: PathBuf,
}

impl Default for WindowsProvider {
  fn default() -> Self {
    Self::new()
  }
}

impl WindowsProvider {
  pub fn new() -> Self {
    Self {
      socket_path: PathBuf::from(DEFAULT_SOCKET_PATH),
    }
  }

  pub fn with_socket(path: impl Into<PathBuf>) -> Self {
    Self {
      socket_path: path.into(),
    }
  }

  /// Build from `WINDOWS_SOCKET`, falls back to the default path.
  pub fn from_env() -> Self {
    let socket_path = std::env::var("WINDOWS_SOCKET")
      .ok()
      .map(PathBuf::from)
      .unwrap_or_else(|| PathBuf::from(DEFAULT_SOCKET_PATH));
    Self { socket_path }
  }

  pub fn socket_path(&self) -> &Path {
    &self.socket_path
  }

  /// Ping the daemon. Returns `true` on a valid pong reply.
  pub fn ping(&self) -> Result<bool> {
    let result = self.request("ping")?;
    Ok(result.get("pong").and_then(|v| v.as_bool()).unwrap_or(false))
  }

  /// Raw window rows from the daemon, no classification or icon lookup.
  pub fn list_raw(&self) -> Result<Vec<RawWindow>> {
    let result = self.request("list_windows")?;
    let windows = result
      .get("windows")
      .and_then(|v| v.as_array())
      .ok_or_else(|| WindowsError::Parse("reply has no windows array".to_owned()))?;
    windows
      .iter()
      .map(|w| {
        serde_json::from_value(w.clone())
          .map_err(|e| WindowsError::Parse(format!("bad window row: {e}")))
      })
      .collect()
  }

  /// Fully classified windows: daemon rows enriched with toolkit type,
  /// bundle identity and the resolved app icon.
  pub fn windows(&self) -> Result<Vec<WindowInfo>> {
    Ok(
      self
        .list_raw()?
        .iter()
        .map(|raw| {
          let c = classify(raw);
          let icon = match (&c.bundle_dir, &c.bundle_id, &c.app_name) {
            (Some(dir), Some(bundle_id), Some(app_name)) => Some(resolve_icon(
              dir,
              bundle_id,
              app_name,
              c.bundle_info.as_ref(),
            )),
            _ => None,
          };
          WindowInfo {
            id: raw.id,
            app_id: raw.app_id.clone(),
            title: raw.title.clone(),
            pid: raw.pid,
            minimized: raw.minimized,
            window_type: c.window_type,
            bundle_id: c.bundle_id,
            app_name: c.app_name,
            icon,
          }
        })
        .collect(),
    )
  }

  fn request(&self, op: &str) -> Result<serde_json::Value> {
    self.request_with(op, serde_json::json!({}))
  }

  /// Minimize a window (iconify). The daemon hides the window, the app
  /// keeps running.
  pub fn minimize_window(&self, id: u64) -> Result<()> {
    self.request_with("minimize_window", serde_json::json!({"window": id}))?;
    Ok(())
  }

  /// Restore a window minimized to the dock. The daemon re-maps the window
  /// and drops its temporary dock icon. Returns `Err` when the id is not a
  /// minimized window or its client is gone.
  pub fn restore_window(&self, id: u64) -> Result<()> {
    self.request_with("restore_window", serde_json::json!({"window": id}))?;
    Ok(())
  }

  /// Set fullscreen state of a window (`true` = fullscreen like the green
  /// UIKit traffic light / F11, `false` = back to windowed).
  pub fn set_fullscreen(&self, id: u64, fullscreen: bool) -> Result<()> {
    self.request_with(
      "set_fullscreen",
      serde_json::json!({"window": id, "fullscreen": fullscreen}),
    )?;
    Ok(())
  }

  /// Fullscreen a window (shortcut for `set_fullscreen(id, true)`).
  pub fn fullscreen_window(&self, id: u64) -> Result<()> {
    self.set_fullscreen(id, true)
  }

  /// Unfullscreen a window (shortcut for `set_fullscreen(id, false)`).
  pub fn unfullscreen_window(&self, id: u64) -> Result<()> {
    self.set_fullscreen(id, false)
  }

  /// Gracefully close a window: the daemon asks the client to close
  /// (`xdg_toplevel.close` on Wayland, `WM_DELETE_WINDOW` on X11).
  /// The app itself decides what happens next, e.g. LibreOffice shows
  /// its save dialog for unsaved documents. Nothing is killed.
  pub fn close_window(&self, id: u64) -> Result<()> {
    self.request_with("close_window", serde_json::json!({"window": id}))?;
    Ok(())
  }

  /// Force quit the owner of a window: resolves the window id to its pid
  /// via `list_raw` and sends `SIGKILL` immediately. No save dialog can
  /// appear, unsaved work is lost. Returns `Err` when the window id is
  /// unknown or carries no pid.
  pub fn force_quit_window(&self, id: u64) -> Result<()> {
    let pid = self
      .list_raw()?
      .iter()
      .find(|w| w.id == id)
      .and_then(|w| w.pid)
      .ok_or_else(|| {
        WindowsError::Parse(format!("no killable process for window {id}"))
      })?;
    force_quit_pid(pid)
  }

  fn request_with(&self, op: &str, params: serde_json::Value) -> Result<serde_json::Value> {
    if !self.socket_path.exists() {
      return Err(WindowsError::SocketMissing(
        self.socket_path.to_string_lossy().into_owned(),
      ));
    }
    let stream = UnixStream::connect(&self.socket_path)
      .map_err(|e| WindowsError::Connection(e.to_string()))?;
    stream
      .set_read_timeout(Some(REQUEST_TIMEOUT))
      .map_err(|e| WindowsError::Connection(e.to_string()))?;
    let mut writer = stream
      .try_clone()
      .map_err(|e| WindowsError::Connection(e.to_string()))?;
    let mut reader = BufReader::new(stream);

    let mut frame = serde_json::json!({"id": 1, "op": op});
    if let (Some(map), Some(extra)) = (frame.as_object_mut(), params.as_object()) {
      for (k, v) in extra {
        map.insert(k.clone(), v.clone());
      }
    }
    let line = frame.to_string() + "\n";
    writer
      .write_all(line.as_bytes())
      .and_then(|_| writer.flush())
      .map_err(|e| WindowsError::Connection(e.to_string()))?;

    let mut reply = String::new();
    reader
      .read_line(&mut reply)
      .map_err(|e| WindowsError::Connection(e.to_string()))?;
    let frame: serde_json::Value =
      serde_json::from_str(&reply).map_err(|e| WindowsError::Protocol(e.to_string()))?;
    if frame.get("ok").and_then(|v| v.as_bool()).unwrap_or(false) {
      Ok(frame.get("result").cloned().unwrap_or(serde_json::Value::Null))
    } else {
      Err(WindowsError::Server(
        frame
          .get("error")
          .and_then(|v| v.as_str())
          .unwrap_or("unknown error")
          .to_string(),
      ))
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn missing_socket_errors() {
    let provider = WindowsProvider::with_socket("/nonexistent/tontoo-windows-test.sock");
    let err = provider.ping().unwrap_err();
    assert!(matches!(err, WindowsError::SocketMissing(_)));
    assert!(err.to_string().contains("/nonexistent/tontoo-windows-test.sock"));
  }

  #[test]
  fn from_env_default() {
    std::env::remove_var("WINDOWS_SOCKET");
    assert_eq!(
      WindowsProvider::from_env().socket_path(),
      Path::new(DEFAULT_SOCKET_PATH)
    );
  }

  /// Fake daemon: answers `expect` requests with `{"ok":true,...}` and
  /// returns the raw request lines. `windows` controls the
  /// `list_windows` payload.
  fn fake_daemon(
    name: &str,
    expect: usize,
    windows: serde_json::Value,
  ) -> (PathBuf, std::thread::JoinHandle<Vec<String>>) {
    let dir = std::env::temp_dir().join(format!("corewindows-test-{}", std::process::id()));
    let _ = std::fs::create_dir_all(&dir);
    let path = dir.join(format!("{name}.sock"));
    let _ = std::fs::remove_file(&path);
    let listener = std::os::unix::net::UnixListener::bind(&path).unwrap();
    let handle = std::thread::spawn(move || {
      let mut seen = Vec::new();
      for stream in listener.incoming().take(expect) {
        let Ok(stream) = stream else { break };
        stream
          .set_read_timeout(Some(Duration::from_millis(500)))
          .unwrap();
        let mut reader = BufReader::new(stream.try_clone().unwrap());
        let mut line = String::new();
        if reader.read_line(&mut line).is_err() || line.is_empty() {
          break;
        }
        seen.push(line.trim().to_owned());
        let req: serde_json::Value = serde_json::from_str(seen.last().unwrap()).unwrap();
        let result = match req.get("op").and_then(|v| v.as_str()) {
          Some("ping") => serde_json::json!({"pong": true}),
          Some("list_windows") => serde_json::json!({"windows": windows}),
          _ => serde_json::Value::Null,
        };
        let reply = serde_json::json!({"ok": true, "result": result}).to_string() + "\n";
        use std::io::Write as _;
        let _ = (&stream).write_all(reply.as_bytes());
      }
      seen
    });
    (path, handle)
  }

  fn frame(lines: &[String], i: usize) -> serde_json::Value {
    serde_json::from_str(&lines[i]).unwrap()
  }

  #[test]
  fn actions_send_expected_frames() {
    let (path, server) = fake_daemon("actions", 5, serde_json::json!([]));
    let provider = WindowsProvider::with_socket(&path);
    provider.minimize_window(5).unwrap();
    provider.restore_window(5).unwrap();
    provider.set_fullscreen(5, true).unwrap();
    provider.unfullscreen_window(5).unwrap();
    provider.close_window(5).unwrap();
    drop(provider);
    let seen = server.join().unwrap();
    assert_eq!(seen.len(), 5);
    assert_eq!(frame(&seen, 0)["op"], serde_json::json!("minimize_window"));
    assert_eq!(frame(&seen, 0)["window"], serde_json::json!(5));
    assert_eq!(frame(&seen, 1)["op"], serde_json::json!("restore_window"));
    assert_eq!(frame(&seen, 1)["window"], serde_json::json!(5));
    assert_eq!(frame(&seen, 2)["op"], serde_json::json!("set_fullscreen"));
    assert_eq!(frame(&seen, 2)["fullscreen"], serde_json::json!(true));
    assert_eq!(frame(&seen, 3)["fullscreen"], serde_json::json!(false));
    assert_eq!(frame(&seen, 4)["op"], serde_json::json!("close_window"));
    let _ = std::fs::remove_file(&path);
  }

  #[test]
  fn force_quit_unknown_window_errors() {
    let (path, server) =
      fake_daemon("forcequit", 2, serde_json::json!([{"id": 1, "app_id": "x", "pid": null}]));
    let provider = WindowsProvider::with_socket(&path);
    let err = provider.force_quit_window(99).unwrap_err();
    assert!(matches!(err, WindowsError::Parse(_)));
    let err = provider.force_quit_window(1).unwrap_err();
    assert!(matches!(err, WindowsError::Parse(_)));
    drop(provider);
    let _ = server.join().unwrap();
    let _ = std::fs::remove_file(&path);
  }

  #[test]
  fn force_quit_missing_socket_errors() {
    let provider = WindowsProvider::with_socket("/nonexistent/tontoo-windows-test.sock");
    let err = provider.minimize_window(1).unwrap_err();
    assert!(matches!(err, WindowsError::SocketMissing(_)));
  }
}

/// Force quit a process id: sends `SIGKILL` immediately.
///
/// No save dialog can appear, unsaved work is lost. This is the macOS
/// Force Quit equivalent. Returns `Err` when the signal fails (e.g.
/// unknown pid or insufficient permission).
pub fn force_quit_pid(pid: i32) -> Result<()> {
  let rc = unsafe { libc::kill(pid, libc::SIGKILL) };
  if rc == 0 {
    Ok(())
  } else {
    Err(WindowsError::Connection(
      std::io::Error::last_os_error().to_string(),
    ))
  }
}
