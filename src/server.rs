/*
 * InspIRCd -- Internet Relay Chat Daemon
 *
 *   Copyright (C) 2026 Sadie Powell <sadie@witchery.services>
 */

#![allow(unsafe_op_in_unsafe_fn)]
#![allow(non_upper_case_globals)]
#![allow(non_camel_case_types)]
#![allow(non_snake_case)]
#![allow(dead_code)]

use std::ffi::c_char;

// Import StdString from stringutils module
use crate::stringutils::StdString;

pub fn generate_sid(servername: &str, serverdesc: &str) -> String {
    let mut sid: u32 = 0;

    for chr in servername.bytes() {
        sid = 5 * sid + chr as u32;
    }
    for chr in serverdesc.bytes() {
        sid = 5 * sid + chr as u32;
    }
    let sid_mod = sid % 1000;
    let mut sidstr = sid_mod.to_string();
    while sidstr.len() < 3 {
        sidstr.insert(0, '0');
    }

    sidstr
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn rust_generate_sid(
    servername: *const c_char,
    servername_length: usize,
    serverdesc: *const c_char,
    serverdesc_length: usize
) -> StdString {
    if servername.is_null() || serverdesc.is_null() {
        return StdString::from_vec("000".to_string().into_bytes());
    }

    let servername_data = unsafe { std::slice::from_raw_parts(servername as *const u8, servername_length) };
    let serverdesc_data = unsafe { std::slice::from_raw_parts(serverdesc as *const u8, serverdesc_length) };

    let servername_str = String::from_utf8_lossy(servername_data);
    let serverdesc_str = String::from_utf8_lossy(serverdesc_data);

    let sid = generate_sid(&servername_str, &serverdesc_str);
    StdString::from_vec(sid.into_bytes())
}
