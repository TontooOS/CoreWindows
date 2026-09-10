# FFI

C API of the library, declared in `Headers/corewindows.h`. The Rust
side lives in `src/ffi.rs`.

## Functions

### tontoo_corewindows_version

```c
const char *tontoo_corewindows_version(void);
```

Framework version as a static string. Do not free the pointer.

| Return | Meaning |
|---|---|
| non-null | Version string, e.g. `"26.1.0"` |

### tontoo_corewindows_ping

```c
int tontoo_corewindows_ping(const char *socket_path);
```

Pings the window daemon. `socket_path` may be `NULL` (uses
`WINDOWS_SOCKET` or the default socket).

| Return | Meaning |
|---|---|
| `1` | Pong received |
| `0` | No pong (daemon missing, unreachable or bad reply) |

### tontoo_corewindows_list_windows

```c
char *tontoo_corewindows_list_windows(const char *socket_path);
```

Currently open windows as a JSON array of `WindowInfo` objects
(see [Types.md](Types.md)). `socket_path` may be `NULL` (uses
`WINDOWS_SOCKET` or the default socket).

| Return | Meaning |
|---|---|
| non-null | JSON array, release with `tontoo_corewindows_string_free` |
| null | Daemon unreachable or reply unusable |

### tontoo_corewindows_string_free

```c
void tontoo_corewindows_string_free(char *s);
```

Releases a string returned by this library. `NULL` is ignored.

## Action functions

All take the daemon window `id` (`uint64_t`) and an optional
`socket_path` (`NULL` uses `WINDOWS_SOCKET` or the default socket).
They return `0` on success.

### tontoo_corewindows_minimize_window

```c
int tontoo_corewindows_minimize_window(const char *socket_path, uint64_t id);
```

Minimizes (iconifies) a window, the app keeps running.

| Return | Meaning |
|---|---|
| `0` | Minimized |
| `-1` | Error (daemon missing, unreachable, or refused) |

### tontoo_corewindows_restore_window

```c
int tontoo_corewindows_restore_window(const char *socket_path, uint64_t id);
```

Restores a window minimized to the dock.

| Return | Meaning |
|---|---|
| `0` | Restored |
| `-1` | Error (unknown id, not minimized, client gone, or daemon refused) |

### tontoo_corewindows_set_fullscreen

```c
int tontoo_corewindows_set_fullscreen(const char *socket_path, uint64_t id, int fullscreen);
```

Nonzero `fullscreen` is fullscreen like the green UIKit traffic light,
`0` returns to windowed mode.

| Return | Meaning |
|---|---|
| `0` | State applied |
| `-1` | Error (daemon missing, unreachable, or refused) |

### tontoo_corewindows_close_window

```c
int tontoo_corewindows_close_window(const char *socket_path, uint64_t id);
```

Graceful close: the app is asked to close and may show a save dialog.
Nothing is killed.

| Return | Meaning |
|---|---|
| `0` | Close requested |
| `-1` | Error (daemon missing, unreachable, or refused) |

### tontoo_corewindows_force_quit_window

```c
int tontoo_corewindows_force_quit_window(const char *socket_path, uint64_t id);
```

Force quits the window owner (`SIGKILL`, no save dialog).

| Return | Meaning |
|---|---|
| `0` | Killed |
| `-1` | Daemon error (window list unavailable) |
| `-2` | Window id unknown or carries no pid |

### tontoo_corewindows_force_quit_pid

```c
int tontoo_corewindows_force_quit_pid(int pid);
```

Force quits a process id (`SIGKILL`, no save dialog).

| Return | Meaning |
|---|---|
| `0` | Killed |
| `-1` | Signal failed (unknown pid or denied) |

### tontoo_corewindows_list_programs

```c
char *tontoo_corewindows_list_programs(void);
```

Installed programs as a JSON array of `AppEntry` objects
(see [Programs.md](Programs.md)). Never null on success: an empty
array when nothing is found. Release with
`tontoo_corewindows_string_free`.

## Memory Rules

| Pointer | Owner | Release |
|---|---|---|
| `tontoo_corewindows_version` result | Library (static) | Never free |
| `tontoo_corewindows_list_windows` result | Caller | `tontoo_corewindows_string_free` |

## Usage / Example

```c
#include "corewindows.h"
#include <stdio.h>

int main(void) {
  char *json = tontoo_corewindows_list_windows(NULL);
  if (json) {
    puts(json);
    tontoo_corewindows_string_free(json);
  }
  return 0;
}
```

## Cross References

- [Windows.md](Windows.md) – the Rust calls behind the C functions
- [Actions.md](Actions.md) – action semantics (graceful close vs force quit)
- [Programs.md](Programs.md) – the JSON shape of `AppEntry`
- [Types.md](Types.md) – the JSON shape of `WindowInfo`
