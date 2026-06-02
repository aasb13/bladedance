// This file is a Rust port of a corresponding InspIRCd module.
// Original work Copyright (C) the InspIRCd contributors.
// Licensed under GPLv2. See LICENSE for details.

use std::ffi::{c_char, CStr, CString};

/// Represents a user/host pair for FFI
#[repr(C)]
pub struct RustUserHostPair {
    pub user: *mut c_char,
    pub host: *mut c_char,
}

/// Splits a user@host string into user and host parts.
/// Returns a RustUserHostPair with allocated strings that must be freed by the caller.
pub fn split_user_host(user_and_host: &str) -> RustUserHostPair {
    let mut user = "*".to_string();
    let mut host = "*".to_string();

    if let Some(x) = user_and_host.find('@') {
        if x > 0 {
            user = user_and_host[..x].to_string();
        }
        if x + 1 < user_and_host.len() {
            host = user_and_host[x + 1..].to_string();
        }
        
        if user.is_empty() {
            user = "*".to_string();
        }
        if host.is_empty() {
            host = "*".to_string();
        }
    } else {
        user.clear();
        host = user_and_host.to_string();
    }

    RustUserHostPair {
        user: CString::new(user).unwrap().into_raw(),
        host: CString::new(host).unwrap().into_raw(),
    }
}

// FFI exports for C++ interop

#[unsafe(no_mangle)]
pub extern "C" fn xline_split_user_host(user_and_host: *const c_char) -> RustUserHostPair {
    if user_and_host.is_null() {
        return RustUserHostPair {
            user: CString::new("*").unwrap().into_raw(),
            host: CString::new("*").unwrap().into_raw(),
        };
    }

    let c_str = unsafe { CStr::from_ptr(user_and_host) };
    let str_slice = c_str.to_str().unwrap_or("");
    split_user_host(str_slice)
}

#[unsafe(no_mangle)]
pub extern "C" fn xline_free_user_host(pair: RustUserHostPair) {
    if !pair.user.is_null() {
        unsafe {
            let _ = CString::from_raw(pair.user);
        }
    }
    if !pair.host.is_null() {
        unsafe {
            let _ = CString::from_raw(pair.host);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_split_user_host_full() {
        let pair = split_user_host("user@host.com");
        let user = unsafe { CStr::from_ptr(pair.user) }.to_str().unwrap();
        let host = unsafe { CStr::from_ptr(pair.host) }.to_str().unwrap();
        assert_eq!(user, "user");
        assert_eq!(host, "host.com");
        xline_free_user_host(pair);
    }

    #[test]
    fn test_split_user_host_only_host() {
        let pair = split_user_host("host.com");
        let user = unsafe { CStr::from_ptr(pair.user) }.to_str().unwrap();
        let host = unsafe { CStr::from_ptr(pair.host) }.to_str().unwrap();
        assert_eq!(user, "");
        assert_eq!(host, "host.com");
        xline_free_user_host(pair);
    }

    #[test]
    fn test_split_user_host_empty_parts() {
        let pair = split_user_host("@");
        let user = unsafe { CStr::from_ptr(pair.user) }.to_str().unwrap();
        let host = unsafe { CStr::from_ptr(pair.host) }.to_str().unwrap();
        assert_eq!(user, "*");
        assert_eq!(host, "*");
        xline_free_user_host(pair);
    }

    #[test]
    fn test_split_user_host_no_at() {
        let pair = split_user_host("justhost");
        let user = unsafe { CStr::from_ptr(pair.user) }.to_str().unwrap();
        let host = unsafe { CStr::from_ptr(pair.host) }.to_str().unwrap();
        assert_eq!(user, "");
        assert_eq!(host, "justhost");
        xline_free_user_host(pair);
    }
}
