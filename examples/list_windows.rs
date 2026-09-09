use corewindows::WindowsProvider;

fn main() {
  let provider = WindowsProvider::from_env();
  match provider.windows() {
    Ok(windows) => {
      if windows.is_empty() {
        println!("no open windows");
      }
      for w in &windows {
        let name = w.app_name.as_deref().unwrap_or("?");
        let icon = w
          .icon
          .as_ref()
          .and_then(|i| i.icon_path.as_deref())
          .map(|p| p.to_string_lossy().into_owned())
          .unwrap_or_else(|| "-".to_owned());
        println!(
          "#{} [{}] {} (bundle: {}, icon: {})",
          w.id,
          w.window_type,
          name,
          w.bundle_id.as_deref().unwrap_or("-"),
          icon
        );
      }
    }
    Err(e) => {
      eprintln!("error: {e}");
      std::process::exit(1);
    }
  }
}
