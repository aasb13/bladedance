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
use libc::{AF_INET, AF_INET6};
use crate::wildcard;
use crate::hashcomp;

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

fn parse_ip_address(addr: &str) -> Option<(Vec<u8>, i32)> {
    // Try IPv4 first
    if let Ok(ipv4) = addr.parse::<std::net::Ipv4Addr>() {
        return Some((ipv4.octets().to_vec(), AF_INET));
    }
    
    // Try IPv6
    if let Ok(ipv6) = addr.parse::<std::net::Ipv6Addr>() {
        return Some((ipv6.octets().to_vec(), AF_INET6));
    }
    
    None
}

fn parse_cidr_mask(cidr: &str) -> Option<(Vec<u8>, u8, i32)> {
    let slash_pos = cidr.rfind('/');
    
    if let Some(pos) = slash_pos {
        let addr_part = &cidr[..pos];
        let length_part = &cidr[pos + 1..];
        
        let length: u8 = length_part.parse().ok()?;
        
        if let Some((bytes, family)) = parse_ip_address(addr_part) {
            // Normalize the mask by applying the prefix length
            let normalized = normalize_cidr(&bytes, length, family);
            Some((normalized, length, family))
        } else {
            None
        }
    } else {
        // No slash, treat as /128 for IPv6 or /32 for IPv4
        if let Some((bytes, family)) = parse_ip_address(cidr) {
            let length = if family == libc::AF_INET { 32 } else { 128 };
            let normalized = normalize_cidr(&bytes, length, family);
            Some((normalized, length, family))
        } else {
            None
        }
    }
}

fn normalize_cidr(bytes: &[u8], length: u8, family: i32) -> Vec<u8> {
    let mut result = bytes.to_vec();
    let total_bytes = if family == AF_INET { 4 } else { 16 };
    
    let border = (length / 8) as usize;
    let bitmask: u8 = ((0xFF00u16 >> (length & 7)) & 0xFF) as u8;
    
    for i in 0..total_bytes {
        if i < border {
            // Keep the byte as-is
        } else if i == border {
            // Apply the bitmask
            result[i] &= bitmask;
        } else {
            // Zero out remaining bytes
            result[i] = 0;
        }
    }
    
    result
}

fn cidr_match_normalized(address: &str, cidr: &str) -> bool {
    // Parse the address
    let (addr_bytes, addr_family) = match parse_ip_address(address) {
        Some(result) => result,
        None => return false,
    };
    
    // Parse the CIDR mask
    let (mask_bytes, mask_length, mask_family) = match parse_cidr_mask(cidr) {
        Some(result) => result,
        None => return false,
    };
    
    // Families must match
    if addr_family != mask_family {
        return false;
    }
    
    // Normalize the address with the mask length
    let normalized_addr = normalize_cidr(&addr_bytes, mask_length, addr_family);
    
    // Compare the normalized address with the mask
    normalized_addr == mask_bytes
}

unsafe fn ffi_norm_match(address_copy: &str, cidr_copy: &str) -> bool {
    cidr_match_normalized(address_copy, cidr_copy)
}

unsafe fn ffi_wild_match(a: &str, b: &str) -> bool {
    return wildcard::rust_wildcard_match(
		a.as_ptr(),
		b.as_ptr(),
	    &hashcomp::ASCII_CASE_INSENSITIVE_MAP as *const u8)
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

pub(crate) fn rust_match_cidr_str(address: &str, cidr_mask: &str, match_with_username: bool) -> bool {
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
