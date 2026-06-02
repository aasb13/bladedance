// This file is a Rust port of a corresponding InspIRCd module.
// Original work Copyright (C) the InspIRCd contributors.
// Licensed under GPLv2. See LICENSE for details.

#![allow(unsafe_op_in_unsafe_fn)]

use std::ffi::c_char;
use std::slice;

use crate::cidr::rust_match_cidr_str;

fn match_internal(string: &[u8], wild: &[u8], map: &[u8; 256]) -> bool {
    let mut s_idx = 0usize;
    let mut w_idx = 0usize;
    let mut mp = 0usize;
    let mut cp = 0usize;
    let mut mp_set = false;
    let mut cp_set = false;

    while s_idx < string.len() {
        let wc = wild.get(w_idx).copied().unwrap_or(0);
        if wc == b'*' {
            break;
        }
        let sc = string[s_idx];
        if map[wc as usize] != map[sc as usize] && wc != b'?' {
            return false;
        }
        w_idx += 1;
        s_idx += 1;
    }

    while s_idx < string.len() {
        if wild.get(w_idx) == Some(&b'*') {
            w_idx += 1;
            if w_idx >= wild.len() {
                return true;
            }
            mp = w_idx;
            cp = s_idx + 1;
            mp_set = true;
            cp_set = true;
        } else {
            let wc = wild[w_idx];
            let sc = string[s_idx];
            if map[wc as usize] == map[sc as usize] || wc == b'?' {
                w_idx += 1;
                s_idx += 1;
            } else if mp_set && cp_set {
                w_idx = mp;
                s_idx = cp;
                cp += 1;
            } else {
                return false;
            }
        }
    }

    while wild.get(w_idx) == Some(&b'*') {
        w_idx += 1;
    }

    w_idx >= wild.len()
}

unsafe fn c_strlen(p: *const u8) -> usize {
    let mut i = 0usize;
    while *p.add(i) != 0 {
        i += 1;
    }
    i
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn rust_wildcard_match(
    str: *const u8,
    wild: *const u8,
    map: *const u8,
) -> bool {
    let map_sl = slice::from_raw_parts(map, 256);
    let Ok(map_arr): Result<&[u8; 256], _> = map_sl.try_into() else {
        return false;
    };
    let slen = c_strlen(str);
    let wlen = c_strlen(wild);
    let s = slice::from_raw_parts(str, slen);
    let w = slice::from_raw_parts(wild, wlen);
    match_internal(s, w, map_arr)
}

fn match_cidr_style(str_: &str, mask: &str, map: &[u8; 256]) -> bool {
    rust_match_cidr_str(str_, mask, true)
        || match_internal(str_.as_bytes(), mask.as_bytes(), map)
}

fn sepstream_space_tokens(tokens: &str) -> Vec<&str> {
    let mut pos = 0usize;
    let source = tokens;
    let mut out = Vec::new();
    loop {
        while pos < source.len() && source.as_bytes()[pos] == b' ' {
            pos += 1;
        }
        if pos >= source.len() {
            break;
        }
        let p = source[pos..]
            .find(' ')
            .map(|i| pos + i)
            .unwrap_or(source.len());
        out.push(&source[pos..p]);
        pos = p + 1;
    }
    out
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn rust_inspircd_match_mask(
    masks: *const c_char,
    hostname: *const c_char,
    ipaddr: *const c_char,
    ascii_map: *const u8,
) -> bool {
    let map_sl = slice::from_raw_parts(ascii_map, 256);
    let Ok(map_arr): Result<&[u8; 256], _> = map_sl.try_into() else {
        return false;
    };
    let masks = std::ffi::CStr::from_ptr(masks)
        .to_string_lossy()
        .into_owned();
    let hostname = std::ffi::CStr::from_ptr(hostname)
        .to_string_lossy()
        .into_owned();
    let ipaddr = std::ffi::CStr::from_ptr(ipaddr)
        .to_string_lossy()
        .into_owned();

    for mask in sepstream_space_tokens(&masks) {
        if match_internal(hostname.as_bytes(), mask.as_bytes(), map_arr)
            || match_cidr_style(&ipaddr, mask, map_arr)
        {
            return true;
        }
    }
    false
}
