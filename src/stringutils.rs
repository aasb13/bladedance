// This file is a Rust port of a corresponding InspIRCd module.
// Original work Copyright (C) the InspIRCd contributors.
// Licensed under GPLv2. See LICENSE for details.

use std::collections::HashSet;
use std::ffi::{c_char, c_void, CStr};
use std::slice;

pub const BASE64_TABLE: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
pub const HEX_TABLE_LOWER: &[u8] = b"0123456789abcdef";
pub const HEX_TABLE_UPPER: &[u8] = b"0123456789ABCDEF";
pub const PERCENT_TABLE: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789-_.~";

#[repr(C)]
pub struct StdString {
    pub data: *mut u8,        // 8 bytes - pointer to heap data or SSO buffer
    pub length: usize,         // 8 bytes - current string length
    pub sso_union: SsoUnion, // 16 bytes - union for SSO or capacity
}

#[repr(C)]
pub union SsoUnion {
    sso_buffer: [u8; 16], // 15 chars + null terminator for small strings
    capacity: usize,        // capacity for heap-allocated strings
}

impl StdString {
    pub(crate) fn from_vec(mut vec: Vec<u8>) -> Self {
        let length = vec.len();
        let capacity = vec.capacity();
        let data = vec.as_mut_ptr();
        std::mem::forget(vec);
        
        StdString { data, length, sso_union: SsoUnion { capacity } }
    }

    pub(crate) fn is_empty(&self) -> bool {
        self.length == 0
    }

    pub(crate) fn as_bytes(&self) -> &[u8] {
        if self.data.is_null() || self.length == 0 {
            return &[];
        }
        unsafe { std::slice::from_raw_parts(self.data, self.length) }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn StdString_Destroy(str: *mut StdString) {
    if str.is_null() {
        return;
    }
    unsafe {
        let s = &*str;
        if !s.data.is_null() && s.length >= 16 {
            let _ = Vec::from_raw_parts(s.data, s.length, s.sso_union.capacity);
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn Percent_Encode(
    data: *const c_void,
    length: usize,
    table: *const c_char,
    upper: bool,
) -> StdString {
    let table = if table.is_null() {
        PERCENT_TABLE
    } else {
        unsafe {
            let c_str = CStr::from_ptr(table);
            c_str.to_bytes()
        }
    };

    let udata = unsafe { slice::from_raw_parts(data as *const u8, length) };

    // Preallocate the output buffer to avoid constant reallocations.
    let mut buffer: Vec<u8> = Vec::with_capacity(length * 3);

    let hex_table = if upper { HEX_TABLE_UPPER } else { HEX_TABLE_LOWER };

    for &chr in udata {
        if table.contains(&chr) {
            buffer.push(chr);
        } else {
            buffer.push(b'%');
            buffer.push(hex_table[(chr >> 4) as usize]);
            buffer.push(hex_table[(chr & 15) as usize]);
        }
    }

    StdString::from_vec(buffer)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn Percent_Decode(data: *const c_void, length: usize) -> StdString {
    // Preallocate the output buffer to avoid constant reallocations.
    let mut buffer: Vec<u8> = Vec::with_capacity(length);

    let cdata = unsafe { slice::from_raw_parts(data as *const u8, length) };

    let mut idx = 0;
    while idx < length {
        if cdata[idx] == b'%' {
            idx += 1;
            let octet1 = if idx < length {
                (cdata[idx] as char).to_ascii_uppercase() as u8
            } else {
                0
            };
            idx += 1;
            let octet2 = if idx < length {
                (cdata[idx] as char).to_ascii_uppercase() as u8
            } else {
                0
            };

            let table1 = HEX_TABLE_UPPER.iter().position(|&c| c == octet1);
            let table2 = HEX_TABLE_UPPER.iter().position(|&c| c == octet2);

            let pair = ((table1.unwrap_or(0) as u8) << 4) + (table2.unwrap_or(0) as u8);
            buffer.push(pair);
        } else {
            buffer.push(cdata[idx]);
        }
        idx += 1;
    }

    StdString::from_vec(buffer)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn Hex_Encode(
    data: *const c_void,
    length: usize,
    table: *const c_char,
    separator: c_char,
) -> StdString {
    let table = if table.is_null() {
        HEX_TABLE_LOWER
    } else {
        unsafe {
            let c_str = CStr::from_ptr(table);
            c_str.to_bytes()
        }
    };

    // Preallocate the output buffer to avoid constant reallocations.
    let sep_len = if separator != 0 { length } else { 0 };
    let mut buffer: Vec<u8> = Vec::with_capacity((length * 2) + sep_len);

    let udata = unsafe { slice::from_raw_parts(data as *const u8, length) };

    for (idx, &chr) in udata.iter().enumerate() {
        if idx != 0 && separator != 0 {
            buffer.push(separator as u8);
        }
        buffer.push(table[(chr >> 4) as usize]);
        buffer.push(table[(chr & 15) as usize]);
    }

    StdString::from_vec(buffer)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn Hex_Decode(
    data: *const c_void,
    length: usize,
    table: *const c_char,
    separator: c_char,
) -> StdString {
    let table = if table.is_null() {
        HEX_TABLE_LOWER
    } else {
        unsafe {
            let c_str = CStr::from_ptr(table);
            c_str.to_bytes()
        }
    };

    let segment = if separator != 0 { 3 } else { 2 };

    // Preallocate the output buffer to avoid constant reallocations.
    let mut buffer: Vec<u8> = Vec::with_capacity(length / segment);

    let cdata = unsafe { slice::from_raw_parts(data as *const u8, length) };

    let mut idx = 0;
    while idx + 1 < length {
        let table1 = table.iter().position(|&c| c == cdata[idx]);
        let table2 = table.iter().position(|&c| c == cdata[idx + 1]);

        let pair = ((table1.unwrap_or(0) as u8) << 4) + (table2.unwrap_or(0) as u8);
        buffer.push(pair);

        idx += segment;
    }

    StdString::from_vec(buffer)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn Base64_Encode(
    data: *const c_void,
    length: usize,
    table: *const c_char,
    padding: c_char,
) -> StdString {
    // Use the default table if one is not specified.
    let table = if table.is_null() {
        BASE64_TABLE
    } else {
        unsafe {
            let c_str = CStr::from_ptr(table);
            c_str.to_bytes()
        }
    };

    // Preallocate the output buffer to avoid constant reallocations.
    let mut buffer: Vec<u8> = Vec::with_capacity(4 * ((length + 2) / 3));

    let udata = unsafe { slice::from_raw_parts(data as *const u8, length) };

    let mut idx = 0;
    while idx < length {
        let octet1 = if idx < length { udata[idx] } else { 0 };
        idx += 1;
        let octet2 = if idx < length { udata[idx] } else { 0 };
        idx += 1;
        let octet3 = if idx < length { udata[idx] } else { 0 };
        idx += 1;

        let triple = ((octet1 as u32) << 16) + ((octet2 as u32) << 8) + (octet3 as u32);

        buffer.push(table[((triple >> 18) & 63) as usize]);
        buffer.push(table[((triple >> 12) & 63) as usize]);
        buffer.push(table[((triple >> 6) & 63) as usize]);
        buffer.push(table[(triple & 63) as usize]);
    }

    let padding_count: [usize; 3] = [0, 2, 1];
    if padding != 0 {
        for i in 0..padding_count[length % 3] {
            let pos = buffer.len() - 1 - i;
            buffer[pos] = padding as u8;
        }
    } else {
        let remove_count = padding_count[length % 3];
        let new_len = buffer.len() - remove_count;
        buffer.truncate(new_len);
    }

    StdString::from_vec(buffer)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn Base64_Decode(
    data: *const c_void,
    length: usize,
    table: *const c_char,
) -> StdString {
    // Use the default table if one is not specified.
    let table = if table.is_null() {
        BASE64_TABLE
    } else {
        unsafe {
            let c_str = CStr::from_ptr(table);
            c_str.to_bytes()
        }
    };

    // Preallocate the output buffer to avoid constant reallocations.
    let mut buffer: Vec<u8> = Vec::with_capacity((length / 4) * 3);

    let mut current_bits: u32 = 0;
    let mut seen_bits: usize = 0;

    let cdata = unsafe { slice::from_raw_parts(data as *const u8, length) };

    for &chr in cdata {
        if let Some(pos) = table.iter().position(|&c| c == chr) {
            current_bits = (current_bits << 6) | (pos as u32);
            seen_bits += 6;

            if seen_bits >= 8 {
                seen_bits -= 8;
                buffer.push(((current_bits >> seen_bits) & 0xFF) as u8);
            }
        }
    }

    StdString::from_vec(buffer)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn Template_Replace(
    str: *const c_char,
    str_length: usize,
    vars_data: *const *const c_char,
    vars_values: *const *const c_char,
    vars_count: usize,
) -> StdString {
    let str_data = unsafe { slice::from_raw_parts(str as *const u8, str_length) };
    let mut out: Vec<u8> = Vec::with_capacity(str_length);

    let mut idx = 0;
    while idx < str_data.len() {
        if str_data[idx] != b'%' {
            out.push(str_data[idx]);
            idx += 1;
            continue;
        }

        let mut endidx = idx + 1;
        let mut found_end = false;
        while endidx < str_data.len() {
            if str_data[endidx] == b'%' {
                found_end = true;
                break;
            }
            endidx += 1;
        }

        if !found_end {
            out.push(str_data[idx]);
            idx += 1;
            continue;
        }

        if endidx - idx == 1 {
            out.push(b'%');
            idx = endidx + 1;
            continue;
        }

        let var_name = &str_data[idx + 1..endidx];

        for i in 0..vars_count {
            let name_ptr = unsafe { *vars_data.add(i) };
            let value_ptr = unsafe { *vars_values.add(i) };

            if name_ptr.is_null() || value_ptr.is_null() {
                continue;
            }

            let name_cstr = unsafe { CStr::from_ptr(name_ptr) };
            let name_bytes = name_cstr.to_bytes();

            if name_bytes == var_name {
                let value_cstr = unsafe { CStr::from_ptr(value_ptr) };
                out.extend_from_slice(value_cstr.to_bytes());
                break;
            }
        }

        idx = endidx + 1;
    }

    StdString::from_vec(out)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn InspIRCd_TimingSafeCompare(
    one: *const c_char,
    one_length: usize,
    two: *const c_char,
    two_length: usize,
) -> bool {
    if one_length != two_length {
        return false;
    }

    let one_data = unsafe { slice::from_raw_parts(one as *const u8, one_length) };
    let two_data = unsafe { slice::from_raw_parts(two as *const u8, two_length) };

    let mut diff: u8 = 0;
    for i in 0..one_length {
        diff |= one_data[i] ^ two_data[i];
    }

    diff == 0
}

#[repr(C)]
pub struct TokenList {
    permissive: bool,
    tokens: HashSet<String>,
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn TokenList_New(tokenlist: *const c_char, tokenlist_length: usize) -> *mut TokenList {
    let mut list = Box::new(TokenList {
        permissive: false,
        tokens: HashSet::new(),
    });

    if !tokenlist.is_null() && tokenlist_length > 0 {
        unsafe {
            TokenList_AddList(&mut *list, tokenlist, tokenlist_length);
        }
    }

    Box::into_raw(list)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn TokenList_Destroy(list: *mut TokenList) {
    if !list.is_null() {
        unsafe {
            let _ = Box::from_raw(list);
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn TokenList_AddList(list: *mut TokenList, tokenlist: *const c_char, tokenlist_length: usize) {
    if list.is_null() || tokenlist.is_null() {
        return;
    }
    let list = unsafe { &mut *list };
    let data = unsafe { slice::from_raw_parts(tokenlist as *const u8, tokenlist_length) };

    let mut pos = 0;
    while pos < data.len() {
        while pos < data.len() && (data[pos] == b' ' || data[pos] == b'\t') {
            pos += 1;
        }

        if pos >= data.len() {
            break;
        }

        let start = pos;
        while pos < data.len() && data[pos] != b' ' && data[pos] != b'\t' {
            pos += 1;
        }

        let token = String::from_utf8_lossy(&data[start..pos]);
        if !token.is_empty() {
            if token.starts_with('-') {
                let to_remove = &token[1..];
                unsafe {
                    TokenList_Remove(list, to_remove.as_ptr() as *const c_char, to_remove.len());
                }
            } else {
                unsafe {
                    TokenList_Add(list, token.as_ptr() as *const c_char, token.len());
                }
            }
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn TokenList_Add(list: *mut TokenList, token: *const c_char, token_length: usize) {
    if list.is_null() || token.is_null() {
        return;
    }
    let list = unsafe { &mut *list };
    let data = unsafe { slice::from_raw_parts(token as *const u8, token_length) };
    let token_str = String::from_utf8_lossy(data);

    // If the token is empty or contains just whitespace it is invalid.
    if token_str.is_empty() || token_str.trim().is_empty() {
        return;
    }

    if token_str == "*" {
        list.permissive = true;
        list.tokens.clear();
        return;
    }

    // Store token in lowercase for case-insensitive comparison (matches irc::insensitive_swo).
    let token_lower = token_str.to_lowercase();

    if list.permissive {
        list.tokens.remove(&token_lower);
    } else {
        list.tokens.insert(token_lower);
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn TokenList_Clear(list: *mut TokenList) {
    if list.is_null() {
        return;
    }
    let list = unsafe { &mut *list };
    list.permissive = false;
    list.tokens.clear();
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn TokenList_Contains(list: *const TokenList, token: *const c_char, token_length: usize) -> bool {
    if list.is_null() || token.is_null() {
        return false;
    }
    let list = unsafe { &*list };
    let data = unsafe { slice::from_raw_parts(token as *const u8, token_length) };
    let token_str = String::from_utf8_lossy(data);
    let token_lower = token_str.to_lowercase();

    if list.permissive && list.tokens.contains(&token_lower) {
        return false;
    }

    if !list.permissive && !list.tokens.contains(&token_lower) {
        return false;
    }

    true
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn TokenList_Remove(list: *mut TokenList, token: *const c_char, token_length: usize) {
    if list.is_null() || token.is_null() {
        return;
    }
    let list = unsafe { &mut *list };
    let data = unsafe { slice::from_raw_parts(token as *const u8, token_length) };
    let token_str = String::from_utf8_lossy(data);

    // If the token is empty or contains just whitespace it is invalid.
    if token_str.is_empty() || token_str.trim().is_empty() {
        return;
    }

    if token_str == "*" {
        list.permissive = false;
        list.tokens.clear();
        return;
    }

    // Store token in lowercase for case-insensitive comparison (matches irc::insensitive_swo).
    let token_lower = token_str.to_lowercase();

    if list.permissive {
        list.tokens.insert(token_lower);
    } else {
        list.tokens.remove(&token_lower);
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn TokenList_ToString(list: *const TokenList) -> StdString {
    if list.is_null() {
        return StdString::from_vec(Vec::new());
    }
    let list = unsafe { &*list };

    if list.permissive {
        let mut buffer = String::from("*");
        for token in &list.tokens {
            buffer.push_str(" -");
            buffer.push_str(token);
        }
        StdString::from_vec(buffer.into_bytes())
    } else {
        let mut buffer = String::new();
        let mut first = true;
        for token in &list.tokens {
            if !first {
                buffer.push(' ');
            }
            first = false;
            buffer.push_str(token);
        }
        StdString::from_vec(buffer.into_bytes())
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn TokenList_Equals(one: *const TokenList, two: *const TokenList) -> bool {
    if one.is_null() || two.is_null() {
        return one == two;
    }
    let one = unsafe { &*one };
    let two = unsafe { &*two };

    if one.permissive != two.permissive {
        return false;
    }

    if one.tokens.len() != two.tokens.len() {
        return false;
    }

    for token in &one.tokens {
        if !two.tokens.contains(token) {
            return false;
        }
    }

    true
}
