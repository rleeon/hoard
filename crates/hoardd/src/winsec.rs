//! The named pipe's ACL on Windows: this user only (and SYSTEM).
//!
//! With no explicit security descriptor, a named pipe's default DACL grants full
//! control to the creator, to administrators and to LocalSystem, **and read access
//! to `Everyone`**. Game names, local paths and the sync's commands travel over
//! that pipe, so "readable by everybody" will not do: ADR 0021 asks for
//! user-only permissions (0600 on unix, an ACL here).
//!
//! The descriptor is built from SDDL with the **current user's SID**, not with the
//! `OW` (CREATOR OWNER) alias. `OW` would work in most cases, but resolving it
//! depends on the object's owner being whoever created it; an explicit SID depends
//! on nothing. `OW` stays as the fallback for the rare case of not being able to
//! read the token, and then it is said in the log: silently degrading to an open
//! pipe is exactly what must not happen.

use std::ffi::c_void;
use std::ptr;

use anyhow::{bail, Result};
use windows_sys::core::{PCWSTR, PWSTR};
use windows_sys::Win32::Foundation::{CloseHandle, LocalFree, ERROR_SUCCESS, HANDLE, HLOCAL};
use windows_sys::Win32::Security::Authorization::{
    ConvertSecurityDescriptorToStringSecurityDescriptorW, ConvertSidToStringSidW,
    ConvertStringSecurityDescriptorToSecurityDescriptorW, GetSecurityInfo, SDDL_REVISION_1,
    SE_KERNEL_OBJECT,
};
use windows_sys::Win32::Security::{
    GetTokenInformation, TokenUser, ACL, DACL_SECURITY_INFORMATION, PSECURITY_DESCRIPTOR,
    SECURITY_ATTRIBUTES, TOKEN_QUERY, TOKEN_USER,
};
use windows_sys::Win32::System::Threading::{GetCurrentProcess, OpenProcessToken};

/// A "me and SYSTEM only" security descriptor, ready to hand to
/// `CreateNamedPipe`. It owns its memory (freed with `LocalFree`).
pub struct SecurityDescriptor {
    psd: PSECURITY_DESCRIPTOR,
}

// SAFETY: once built, the descriptor is read-only as far as we are concerned; it
// is only handed to `CreateNamedPipe`, which copies it into the object. Nobody
// mutates it, so sharing it across threads (the `Listener` lives in a tokio task)
// is safe.
unsafe impl Send for SecurityDescriptor {}
unsafe impl Sync for SecurityDescriptor {}

impl SecurityDescriptor {
    /// `D:P(A;;GA;;;<sid>)(A;;GA;;;SY)`: a protected DACL (`P`, no inheritance),
    /// full access (`GA`) for the current user and for LocalSystem, and **nothing**
    /// for anybody else. `Everyone` being absent is the whole point.
    pub fn user_only() -> Result<Self> {
        let sddl = match current_user_sid() {
            Ok(sid) => format!("D:P(A;;GA;;;{sid})(A;;GA;;;SY)"),
            Err(err) => {
                tracing::warn!(
                    error = %err,
                    "hoardd: couldn't read this process's user SID; falling back to the \
                     CREATOR OWNER ACE for the pipe ACL"
                );
                "D:P(A;;GA;;;OW)(A;;GA;;;SY)".to_string()
            }
        };
        let wide: Vec<u16> = sddl.encode_utf16().chain(std::iter::once(0)).collect();
        let mut psd: PSECURITY_DESCRIPTOR = ptr::null_mut();
        // SAFETY: `wide` is a NUL-terminated UTF-16 string that lives for the whole
        // call, and `psd` is a valid out pointer.
        let ok = unsafe {
            ConvertStringSecurityDescriptorToSecurityDescriptorW(
                wide.as_ptr() as PCWSTR,
                SDDL_REVISION_1,
                &mut psd,
                ptr::null_mut(),
            )
        };
        if ok == 0 {
            let err = std::io::Error::last_os_error();
            bail!("building the pipe security descriptor from SDDL failed: {err}");
        }
        Ok(Self { psd })
    }

    /// `SECURITY_ATTRIBUTES` apuntando a este descriptor. El llamante la pasa
    /// por puntero a `CreateNamedPipe` y no debe sobrevivir a `self`.
    pub fn attributes(&self) -> SECURITY_ATTRIBUTES {
        SECURITY_ATTRIBUTES {
            nLength: std::mem::size_of::<SECURITY_ATTRIBUTES>() as u32,
            lpSecurityDescriptor: self.psd,
            // The pipe is not inherited: a child process (an installer, a game
            // launched from the app) has no business inheriting the handle to the
            // sync's control channel.
            bInheritHandle: 0,
        }
    }
}

/// An kernel object's effective DACL, in SDDL.
///
/// It is read **from the object as created**, not from the descriptor we handed
/// it: the ADR asks for user-only permissions, and that used to be verified at the
/// type level (it compiled) rather than at run time. This is what makes it
/// assertable, both in a test and in the startup log, where what ACL this machine's
/// pipe really has ends up written down.
///
/// # Safety
/// `handle` must be a valid handle to a live kernel object.
pub(crate) unsafe fn dacl_sddl(handle: HANDLE) -> Result<String> {
    let mut dacl: *mut ACL = ptr::null_mut();
    let mut psd: PSECURITY_DESCRIPTOR = ptr::null_mut();
    let rc = GetSecurityInfo(
        handle,
        SE_KERNEL_OBJECT,
        DACL_SECURITY_INFORMATION,
        ptr::null_mut(),
        ptr::null_mut(),
        &mut dacl,
        ptr::null_mut(),
        &mut psd,
    );
    if rc != ERROR_SUCCESS {
        bail!("GetSecurityInfo failed with {rc}");
    }
    let mut text: PWSTR = ptr::null_mut();
    let mut len: u32 = 0;
    let ok = ConvertSecurityDescriptorToStringSecurityDescriptorW(
        psd,
        SDDL_REVISION_1,
        DACL_SECURITY_INFORMATION,
        &mut text,
        &mut len,
    );
    if ok == 0 {
        let err = std::io::Error::last_os_error();
        LocalFree(psd as HLOCAL);
        bail!("rendering the security descriptor as SDDL failed: {err}");
    }
    let out = wide_to_string(text);
    LocalFree(text as HLOCAL);
    // `psd` was allocated by `GetSecurityInfo`, which documents `LocalFree`. `dacl`
    // points *inside* that block, so it is not freed separately.
    LocalFree(psd as HLOCAL);
    Ok(out)
}

impl Drop for SecurityDescriptor {
    fn drop(&mut self) {
        if !self.psd.is_null() {
            // SAFETY: `psd` was allocated by
            // `ConvertStringSecurityDescriptorToSecurityDescriptorW`, which documents
            // `LocalFree` as its deallocator.
            unsafe { LocalFree(self.psd as HLOCAL) };
        }
    }
}

/// Handle del token, cerrado al salir del scope.
struct TokenHandle(HANDLE);

impl Drop for TokenHandle {
    fn drop(&mut self) {
        if !self.0.is_null() {
            // SAFETY: a valid handle obtained from `OpenProcessToken`.
            unsafe { CloseHandle(self.0) };
        }
    }
}

/// SID del usuario de este proceso, en forma de cadena (`S-1-5-21-…`).
fn current_user_sid() -> Result<String> {
    // SAFETY: every call is checked before its output is used, and the buffers
    // passed in outlive the call.
    unsafe {
        let mut raw: HANDLE = ptr::null_mut();
        if OpenProcessToken(GetCurrentProcess(), TOKEN_QUERY, &mut raw) == 0 {
            bail!(
                "OpenProcessToken failed: {}",
                std::io::Error::last_os_error()
            );
        }
        let token = TokenHandle(raw);

        // The two-call pattern: the first one only returns the size.
        let mut needed: u32 = 0;
        GetTokenInformation(token.0, TokenUser, ptr::null_mut(), 0, &mut needed);
        if needed == 0 {
            bail!(
                "GetTokenInformation couldn't size the token user: {}",
                std::io::Error::last_os_error()
            );
        }
        let mut buf = vec![0u8; needed as usize];
        if GetTokenInformation(
            token.0,
            TokenUser,
            buf.as_mut_ptr() as *mut c_void,
            needed,
            &mut needed,
        ) == 0
        {
            bail!(
                "GetTokenInformation failed: {}",
                std::io::Error::last_os_error()
            );
        }

        // `buf` is a `Vec<u8>` (aligned to 1) and `TOKEN_USER` needs pointer
        // alignment, so it is read unaligned on purpose. The SID itself still lives
        // inside `buf`, which is alive to the end.
        let user: TOKEN_USER = ptr::read_unaligned(buf.as_ptr() as *const TOKEN_USER);
        let mut sid_str: PWSTR = ptr::null_mut();
        if ConvertSidToStringSidW(user.User.Sid, &mut sid_str) == 0 {
            bail!(
                "ConvertSidToStringSidW failed: {}",
                std::io::Error::last_os_error()
            );
        }
        let text = wide_to_string(sid_str);
        LocalFree(sid_str as HLOCAL);
        Ok(text)
    }
}

/// A NUL-terminated UTF-16 string turned into a `String`.
///
/// # Safety
/// `ptr` must point at a valid NUL-terminated UTF-16 string.
pub(crate) unsafe fn wide_to_string(ptr: PWSTR) -> String {
    let mut len = 0usize;
    while *ptr.add(len) != 0 {
        len += 1;
    }
    String::from_utf16_lossy(std::slice::from_raw_parts(ptr, len))
}
