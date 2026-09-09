use std::sync::OnceLock;

const EN_US: &str = include_str!("../lang/en_us.json");
const DE_DE: &str = include_str!("../lang/de_de.json");

static MESSAGES: OnceLock<serde_json::Value> = OnceLock::new();

pub fn current_locale() -> &'static str {
  let lang = std::env::var("LC_ALL")
    .or_else(|_| std::env::var("LC_MESSAGES"))
    .or_else(|_| std::env::var("LANG"))
    .unwrap_or_default();

  if lang.to_lowercase().starts_with("de") {
    "de_de"
  } else {
    "en_us"
  }
}

fn messages() -> &'static serde_json::Value {
  MESSAGES.get_or_init(|| {
    let raw = match current_locale() {
      "de_de" => DE_DE,
      _ => EN_US,
    };
    serde_json::from_str(raw).expect("built-in language file is invalid")
  })
}

/// Translate `key`, falls back to the key itself when missing.
pub fn t(key: &str) -> String {
  messages()
    .get(key)
    .and_then(|v| v.as_str())
    .unwrap_or(key)
    .to_string()
}
