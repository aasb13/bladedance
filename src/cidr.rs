/*
 * InspIRCd -- Internet Relay Chat Daemon
 *
 *   Copyright (C) 2019, 2022-2023 Sadie Powell <sadie@witchery.services>
 *   Copyright (C) 2014-2015 Attila Molnar <attilamolnar@hush.com>
 *   Copyright (C) 2012 Robby <robby@chatbelgie.be>
 *   Copyright (C) 2009-2010 Daniel De Graaf <danieldg@inspircd.org>
 *   Copyright (C) 2008 Robin Burchell <robin+git@viroteck.net>
 *   Copyright (C) 2008 Craig Edwards <brain@inspircd.org>
 *
 * This file is part of InspIRCd.  InspIRCd is free software: you can
 * redistribute it and/or modify it under the terms of the GNU General Public
 * License as published by the Free Software Foundation, version 2.
 */

#![allow(unsafe_op_in_unsafe_fn)]

use std::ffi::{c_char, CString};

unsafe extern "C" {
    fn cidr_ffi_match_wildcard_ascii(a: *const c_char, b: *const c_char) -> bool;
    fn cidr_ffi_match_normalized(addr: *const c_char, cidr: *const c_char) -> bool;
}

fn is_invalid_cidr_mask_format(cidr_copy: &str) -> bool {
    let Some(per_pos) = cidr_copy.rfind('/') else {
        return false;
    };
    if per_pos == cidr_copy.len() - 1 {
        return true;
    }
    let tail = &cidr_copy[per_pos + 1..];
    if tail.chars().any(|c| !matches!(c, '0'..='9')) {
        return true;
    }
    let head = &cidr_copy[..per_pos];
    if head
        .chars()
        .any(|c| !matches!(c, '0'..='9' | 'a'..='f' | 'A'..='F' | '.' | ':'))
    {
        return true;
    }
    false
}

unsafe fn ffi_norm_match(address_copy: &str, cidr_copy: &str) -> bool {
    let a = CString::new(address_copy).unwrap_or_else(|_| CString::new("").unwrap());
    let c = CString::new(cidr_copy).unwrap_or_else(|_| CString::new("").unwrap());
    cidr_ffi_match_normalized(a.as_ptr(), c.as_ptr())
}

unsafe fn ffi_wild_match(a: &str, b: &str) -> bool {
    let ca = CString::new(a).unwrap_or_else(|_| CString::new("").unwrap());
    let cb = CString::new(b).unwrap_or_else(|_| CString::new("").unwrap());
    cidr_ffi_match_wildcard_ascii(ca.as_ptr(), cb.as_ptr())
}

/// Match CIDR strings, e.g. 127.0.0.1 to 127.0.0.0/8 or 3ffe:1:5:6::8 to 3ffe:1::0/32
///
/// This will also attempt to match any leading usernames or nicknames on the mask, using
/// match(), when match_with_username is true.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn rust_match_cidr(
    address: *const c_char,
    cidr_mask: *const c_char,
    match_with_username: bool,
) -> bool {
    let address = std::ffi::CStr::from_ptr(address)
        .to_string_lossy()
        .into_owned();
    let cidr_mask = std::ffi::CStr::from_ptr(cidr_mask)
        .to_string_lossy()
        .into_owned();
    rust_match_cidr_str(&address, &cidr_mask, match_with_username)
}

fn rust_match_cidr_str(address: &str, cidr_mask: &str, match_with_username: bool) -> bool {
    let (address_copy, cidr_copy) = if match_with_username {
        /* The caller is trying to match username@<mask>/bits.
         * Chop off the username@ portion, use match() on it
         * separately.
         */
        let username_mask_pos = cidr_mask.rfind('@');
        let username_addr_pos = address.rfind('@');

        /* Both strings have an @ symbol in them */
        if let (Some(pm), Some(pa)) = (username_mask_pos, username_addr_pos) {
            /* Try and match() the strings before the @
             * symbols, and recursively call MatchCIDR without
             * username matching enabled to match the host part.
             */
            return unsafe {
                ffi_wild_match(&address[..pa], &cidr_mask[..pm])
                    && rust_match_cidr_str(&address[pa + 1..], &cidr_mask[pm + 1..], false)
            };
        }

        (slice_after_last_at(address), slice_after_last_at(cidr_mask))
    } else {
        (address.to_string(), cidr_mask.to_string())
    };

    if is_invalid_cidr_mask_format(&cidr_copy) {
        // The CIDR mask is invalid
        return false;
    }

    unsafe { ffi_norm_match(&address_copy, &cidr_copy) }
}

fn slice_after_last_at(s: &str) -> String {
    let start = s.rfind('@').map(|i| i + 1).unwrap_or(0);
    s[start..].to_string()
}
