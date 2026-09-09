#ifndef TONTOO_COREWINDOWS_H
#define TONTOO_COREWINDOWS_H

/* CoreWindows C API: list currently open windows with toolkit type and
 * app icons, plus window actions (minimize, fullscreen, close, force quit).
 * All strings returned by this library must be released with
 * tontoo_corewindows_string_free().
 */

#ifdef __cplusplus
extern "C" {
#endif

#include <stdint.h>

#define TONTOO_COREWINDOWS_VERSION "26.1.0"
#define TONTOO_COREWINDOWS_DEFAULT_SOCKET "/run/tontoo-windows.sock"

/* Framework version as a static string (do not free). */
const char *tontoo_corewindows_version(void);

/* Ping the window daemon: 1 on pong, 0 otherwise.
 * socket_path may be NULL (uses WINDOWS_SOCKET or the default socket). */
int tontoo_corewindows_ping(const char *socket_path);

/* Currently open windows as a JSON array of WindowInfo objects
 * (see wiki/Types.md). NULL when the daemon is unreachable. */
char *tontoo_corewindows_list_windows(const char *socket_path);

/* Minimize a window (iconify). Returns 0 on success, -1 on error. */
int tontoo_corewindows_minimize_window(const char *socket_path, uint64_t id);

/* Set fullscreen state (nonzero = fullscreen like the green UIKit traffic
 * light, 0 = windowed). Returns 0 on success, -1 on error. */
int tontoo_corewindows_set_fullscreen(const char *socket_path, uint64_t id, int fullscreen);

/* Gracefully close a window: the app is asked to close and may show a save
 * dialog (e.g. LibreOffice). Nothing is killed. Returns 0 on success,
 * -1 on error. */
int tontoo_corewindows_close_window(const char *socket_path, uint64_t id);

/* Force quit the owner of a window (SIGKILL, no save dialog).
 * Returns 0 on success, -1 on daemon error, -2 when the window id is
 * unknown or carries no pid. */
int tontoo_corewindows_force_quit_window(const char *socket_path, uint64_t id);

/* Force quit a process id (SIGKILL, no save dialog).
 * Returns 0 on success, -1 on error. */
int tontoo_corewindows_force_quit_pid(int pid);

/* Installed programs (~/Applications and /Applications) as a JSON array
 * of AppEntry objects (see wiki/Programs.md). Never NULL on success
 * (empty array when nothing is found). */
char *tontoo_corewindows_list_programs(void);

/* Release a string returned by this library (NULL is ignored). */
void tontoo_corewindows_string_free(char *s);

#ifdef __cplusplus
}
#endif

#endif
