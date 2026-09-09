# Localization

Error messages are localized through `lang/en_us.json` and
`lang/de_de.json`, compiled into the binary. Only these two locales
exist; the system locale selects between them.

## Locale detection

`lang::current_locale()` reads `LC_ALL`, then `LC_MESSAGES`, then
`LANG`. Values starting with `de` (case-insensitive) select `de_de`,
everything else (including unset) selects `en_us`.

## Message keys

| Key | `en_us` | `de_de` |
|---|---|---|
| `socket_missing` | Window daemon socket not found | Fenster-Daemon-Socket nicht gefunden |
| `connection_failed` | Failed to connect to the window daemon | Verbindung zum Fenster-Daemon fehlgeschlagen |
| `protocol_error` | Window daemon protocol error | Fenster-Daemon-Protokollfehler |
| `server_error` | Window daemon reported an error | Fenster-Daemon meldet einen Fehler |
| `parse_error` | Failed to parse the window daemon reply | Antwort des Fenster-Daemon konnte nicht gelesen werden |

## t

```rust
pub fn t(key: &str) -> String
```

Translates `key` in the current locale, falls back to the key itself
when missing. Used by `WindowsError::fmt`; never fails.

## Cross References

- [Windows.md](Windows.md) – where the errors surface
- [MAIN.md](MAIN.md) – supported locales
