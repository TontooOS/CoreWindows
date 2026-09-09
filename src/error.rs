use crate::lang::t;

/// Error type for all provider operations.
#[derive(Debug)]
pub enum WindowsError {
  /// The window daemon socket path does not exist (daemon not running).
  SocketMissing(String),
  /// Transport failure while talking to the daemon.
  Connection(String),
  /// Malformed frame from the daemon.
  Protocol(String),
  /// The daemon answered with `ok: false`.
  Server(String),
  /// A well-formed reply with unusable content.
  Parse(String),
}

/// Convenience alias used by every provider function.
pub type Result<T> = std::result::Result<T, WindowsError>;

impl std::fmt::Display for WindowsError {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    match self {
      Self::SocketMissing(path) => write!(f, "{}: {}", t("socket_missing"), path),
      Self::Connection(detail) => write!(f, "{}: {}", t("connection_failed"), detail),
      Self::Protocol(detail) => write!(f, "{}: {}", t("protocol_error"), detail),
      Self::Server(detail) => write!(f, "{}: {}", t("server_error"), detail),
      Self::Parse(detail) => write!(f, "{}: {}", t("parse_error"), detail),
    }
  }
}

impl std::error::Error for WindowsError {}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn display_mentions_path() {
    let err = WindowsError::SocketMissing("/run/tontoo-windows.sock".to_string());
    assert!(err.to_string().contains("/run/tontoo-windows.sock"));
  }
}
