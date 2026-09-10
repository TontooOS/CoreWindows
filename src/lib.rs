pub mod classify;
pub mod error;
pub mod icon;
pub mod lang;
pub mod programs;
pub mod provider;
pub mod types;

pub use classify::{classify, Classification};
pub use error::{Result, WindowsError};
pub use icon::{resolve_icon, AppIcon};
pub use programs::{
  all_names, list_programs, program_dirs, scan_dir, AppEntry, AppSource,
  SYSTEM_APPLICATIONS_DIR, USER_APPLICATIONS_DIR,
};
pub use provider::{force_quit_pid, WindowsProvider, DEFAULT_SOCKET_PATH};
pub use types::{RawWindow, WindowInfo, WindowType};

/// Library version: (major, minor, patch).
pub const COREWINDOWS_VERSION: (u32, u32, u32) = (26, 1, 0);

/// Ping the window daemon at the default socket.
pub fn ping() -> Result<bool> {
  WindowsProvider::new().ping()
}

/// Raw window rows from the daemon at the default socket, no enrichment.
pub fn list_raw() -> Result<Vec<RawWindow>> {
  WindowsProvider::new().list_raw()
}

/// Fully classified windows (type + bundle + icon) at the default socket.
pub fn windows() -> Result<Vec<WindowInfo>> {
  WindowsProvider::new().windows()
}

/// Minimize a window (iconify) at the default socket.
pub fn minimize_window(id: u64) -> Result<()> {
  WindowsProvider::new().minimize_window(id)
}

/// Restore a window minimized to the dock at the default socket.
pub fn restore_window(id: u64) -> Result<()> {
  WindowsProvider::new().restore_window(id)
}

/// Set fullscreen state of a window at the default socket.
pub fn set_fullscreen(id: u64, fullscreen: bool) -> Result<()> {
  WindowsProvider::new().set_fullscreen(id, fullscreen)
}

/// Gracefully close a window (the app may ask to save) at the default socket.
pub fn close_window(id: u64) -> Result<()> {
  WindowsProvider::new().close_window(id)
}

/// Force quit the owner of a window (`SIGKILL`) at the default socket.
pub fn force_quit_window(id: u64) -> Result<()> {
  WindowsProvider::new().force_quit_window(id)
}

/// Convenience prelude that re-exports the most commonly used types.
pub mod prelude {
  pub use crate::classify::{classify, Classification};
  pub use crate::error::{Result, WindowsError};
  pub use crate::icon::{resolve_icon, AppIcon};
  pub use crate::programs::{
    all_names, list_programs, program_dirs, scan_dir, AppEntry, AppSource,
    SYSTEM_APPLICATIONS_DIR, USER_APPLICATIONS_DIR,
  };
  pub use crate::provider::{force_quit_pid, WindowsProvider, DEFAULT_SOCKET_PATH};
  pub use crate::types::{RawWindow, WindowInfo, WindowType};
  pub use crate::{
    close_window, force_quit_window, list_raw, minimize_window, ping, restore_window,
    set_fullscreen, windows, COREWINDOWS_VERSION,
  };
}

mod ffi;
