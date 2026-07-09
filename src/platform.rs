//! Windows-specific helpers for privileged operation. Everything here has a
//! non-Windows stub so the crate still builds on other platforms.

/// Is this process running with an elevated (administrator) token?
///
/// Used to decide whether the raw-volume MFT fast scan is available, without
/// actually attempting to open a volume.
#[cfg(windows)]
pub fn is_elevated() -> bool {
    use std::mem;
    use windows_sys::Win32::Foundation::{CloseHandle, HANDLE};
    use windows_sys::Win32::Security::{
        GetTokenInformation, TokenElevation, TOKEN_ELEVATION, TOKEN_QUERY,
    };
    use windows_sys::Win32::System::Threading::{GetCurrentProcess, OpenProcessToken};

    // SAFETY: standard Win32 token-query sequence; `token` is closed before
    // return, and `elevation`/`ret_len` are live for the duration of the call.
    unsafe {
        let mut token: HANDLE = std::ptr::null_mut();
        if OpenProcessToken(GetCurrentProcess(), TOKEN_QUERY, &mut token) == 0 {
            return false;
        }
        let mut elevation = TOKEN_ELEVATION { TokenIsElevated: 0 };
        let mut ret_len: u32 = 0;
        let ok = GetTokenInformation(
            token,
            TokenElevation,
            &mut elevation as *mut _ as *mut core::ffi::c_void,
            mem::size_of::<TOKEN_ELEVATION>() as u32,
            &mut ret_len,
        );
        CloseHandle(token);
        ok != 0 && elevation.TokenIsElevated != 0
    }
}

/// Relaunch this same executable elevated via the UAC "runas" verb. Returns
/// `true` if the elevated process was launched (the caller should then exit so
/// only the elevated instance remains); `false` if the user declined the UAC
/// prompt or the launch failed.
#[cfg(windows)]
pub fn relaunch_as_admin() -> bool {
    use std::os::windows::ffi::OsStrExt;
    use windows_sys::Win32::UI::Shell::ShellExecuteW;
    use windows_sys::Win32::UI::WindowsAndMessaging::SW_SHOWNORMAL;

    let Ok(exe) = std::env::current_exe() else {
        return false;
    };
    let wide_exe: Vec<u16> = exe
        .as_os_str()
        .encode_wide()
        .chain(std::iter::once(0))
        .collect();
    let verb: Vec<u16> = "runas".encode_utf16().chain(std::iter::once(0)).collect();

    // SAFETY: both pointers reference NUL-terminated UTF-16 buffers that live
    // past the call; the remaining args are null (no parameters/dir/parent).
    let result = unsafe {
        ShellExecuteW(
            std::ptr::null_mut(),
            verb.as_ptr(),
            wide_exe.as_ptr(),
            std::ptr::null(),
            std::ptr::null(),
            SW_SHOWNORMAL,
        )
    };
    // ShellExecuteW returns a value greater than 32 on success.
    (result as isize) > 32
}

#[cfg(not(windows))]
pub fn is_elevated() -> bool {
    false
}

#[cfg(not(windows))]
pub fn relaunch_as_admin() -> bool {
    false
}
