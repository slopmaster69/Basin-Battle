//! Windows with the GNU toolchain only: mingw-w64's msvcrt import library can lack
//! `_get_timezone` (it's ucrt's), which the `libc` crate calls for benilla's login code, and the
//! server then fails to link. This reads the runtime's own `_timezone` instead, as
//! `scripts/mingw-tz-shim.c` does for the client. Where the library has it, ours is simply used.

use std::os::raw::{c_int, c_long};

#[link(name = "msvcrt", kind = "dylib")]
extern "C" {
    static _timezone: c_long;
}

/// `_get_timezone(&mut seconds)`: the zone's offset west of UTC in seconds; `EINVAL` for null.
#[no_mangle]
pub unsafe extern "C" fn _get_timezone(seconds: *mut c_long) -> c_int {
    const EINVAL: c_int = 22;
    if seconds.is_null() {
        return EINVAL;
    }
    // SAFETY: `_timezone` is msvcrt's own global, set by `_tzset`; `seconds` was checked.
    unsafe { *seconds = _timezone };
    0
}
