/* mingw-w64's msvcrt import library has no _get_timezone, _get_tzname or _get_dstbias (ucrt's);
   the Rust libc crate calls them. Read the runtime's own data instead. */
#include <stddef.h>
#include <string.h>
#include <errno.h>
__declspec(dllimport) extern long _timezone;
__declspec(dllimport) extern long _dstbias;
__declspec(dllimport) extern char *_tzname[2];

int __cdecl _get_timezone(long *seconds) {
    if (!seconds) return EINVAL;
    *seconds = _timezone;
    return 0;
}

int __cdecl _get_dstbias(long *seconds) {
    if (!seconds) return EINVAL;
    *seconds = _dstbias;
    return 0;
}

int __cdecl _get_tzname(size_t *len, char *buf, size_t size, int index) {
    if (!len || index < 0 || index > 1 || (!buf && size)) return EINVAL;
    const char *name = _tzname[index] ? _tzname[index] : "";
    size_t n = strlen(name) + 1;
    *len = n;
    if (!buf) return 0;
    if (size < n) { if (size) buf[0] = 0; return ERANGE; }
    memcpy(buf, name, n);
    return 0;
}
