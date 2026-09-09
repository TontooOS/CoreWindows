use corewindows::list_programs;

fn main() {
  let programs = list_programs();
  if programs.is_empty() {
    println!("no programs installed");
  }
  for app in &programs {
    let icon = app
      .icon
      .icon_path
      .as_deref()
      .map(|p| p.to_string_lossy().into_owned())
      .unwrap_or_else(|| "-".to_owned());
    println!(
      "[{}] {} ({}, {}, icon: {})",
      app.source,
      app.display_name,
      app.bundle_id,
      app.bundle_path.display(),
      icon
    );
  }
}
