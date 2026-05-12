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

/// Generates a server ID (SID) from servername and serverdesc
/// 
/// # Arguments
/// * `servername` - The server name
/// * `serverdesc` - The server description
/// 
/// # Returns
/// A 3-digit string representing the server ID
pub fn generate_sid(servername: &str, serverdesc: &str) -> String {
    let mut sid: u32 = 0;

    // Process servername: sid = 5 * sid + chr for each character
    for chr in servername.bytes() {
        sid = 5 * sid + chr as u32;
    }

    // Process serverdesc: sid = 5 * sid + chr for each character
    for chr in serverdesc.bytes() {
        sid = 5 * sid + chr as u32;
    }

    // Take modulo 1000 and convert to string
    let sid_mod = sid % 1000;
    let mut sidstr = sid_mod.to_string();

    // Pad with leading zeros to make it 3 digits
    while sidstr.len() < 3 {
        sidstr.insert(0, '0');
    }

    sidstr
}

// C-compatible wrapper for generate_sid
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
