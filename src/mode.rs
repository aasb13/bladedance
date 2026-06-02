// This file is a Rust port of a corresponding InspIRCd module.
// Original work Copyright (C) the InspIRCd contributors.
// Licensed under GPLv2. See LICENSE for details.

#![allow(non_upper_case_globals)]
#![allow(non_camel_case_types)]
#![allow(non_snake_case)]
#![allow(dead_code)]

use std::ffi::{c_char, c_void};
use std::os::raw::c_ulong;
use crate::stringutils::{StdString, StdString_Destroy};

/// Holds the values for different type of modes that can exist, USER or CHANNEL type.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ModeType {
    /// User mode
    MODETYPE_USER = 0,
    /// Channel mode
    MODETYPE_CHANNEL = 1,
}

/// These fixed values can be used to proportionally compare module-defined prefixes to known values.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PrefixModeValue {
    /// +v
    VOICE_VALUE = 10000,
    /// +h
    HALFOP_VALUE = 20000,
    /// +o
    OP_VALUE = 30000,
}

/// Parameter specification for modes
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ParamSpec {
    /// No parameters
    PARAM_NONE = 0,
    /// Parameter required on mode setting only
    PARAM_SETONLY = 1,
    /// Parameter always required
    PARAM_ALWAYS = 2,
}

/// Mode handler class types
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ModeHandlerClass {
    MC_PREFIX = 0,
    MC_LIST = 1,
    MC_PARAM = 2,
    MC_OTHER = 3,
}

/// The maximum number of modes which can be created.
pub const MODEID_MAX: usize = 64;

/// The maximum length of a mode parameter.
pub const MODE_PARAM_MAX: usize = 250;

/// Determines whether the specified character is a valid mode.
#[unsafe(no_mangle)]
pub extern "C" fn ModeParser_IsModeChar(chr: c_char) -> bool {
    let c = chr as u8;
    (c >= b'0' && c <= b'9') || (c >= b'A' && c <= b'Z') || (c >= b'a' && c <= b'z')
}

/// Retrieves the index of the specified mode char within a mode bitset.
#[unsafe(no_mangle)]
pub extern "C" fn ModeParser_GetModeIndex(chr: c_char) -> usize {
    let c = chr as u8;
    // Bitset layout:
    //   0123456789                 = 10 [0-9]
    //   ABCDEFGHIJKLMNOPQRSTUVWXYZ = 26 [10-35]
    //   abcdefghijklmnopqrstuvwxyz = 26 [36-61]
    if c >= b'0' && c <= b'9' {
        return (c - b'0') as usize;
    }
    if c >= b'A' && c <= b'Z' {
        return (c - b'A' + 10) as usize;
    }
    if c >= b'a' && c <= b'z' {
        return (c - b'a' + 36) as usize;
    }
    MODEID_MAX
}

/// Tidy a banmask. This makes a banmask 'acceptable' if fields are left out.
#[unsafe(no_mangle)]
pub extern "C" fn ModeParser_CleanMask(mask: *mut StdString) {
    if mask.is_null() {
        return;
    }
    
    unsafe {
        let s = &mut *mask;
        let bytes = s.as_bytes().to_vec();
        let mut mask_str = String::from_utf8_lossy(&bytes).to_string();
        
        let pos_of_pling = mask_str.find('!');
        let pos_of_at = pos_of_pling.and_then(|p| mask_str[p..].find('@').map(|x| p + x));
        let pos_of_hostchar = pos_of_at.and_then(|p| mask_str[p..].find(|c| c == ':' || c == '.').map(|x| p + x));
        
        let len = mask_str.len();
        
        if pos_of_pling == Some(len - 1) || pos_of_at == Some(len - 1) {
            // Malformed mask; needs * after the ! or @.
            mask_str.push('*');
        }
        
        if pos_of_pling == Some(0) || pos_of_at == Some(0) {
            // Malformed mask; needs * before the ! or @.
            mask_str.insert(0, '*');
        }
        
        if pos_of_pling.is_none() && pos_of_at.is_none() {
            if pos_of_hostchar.is_none() {
                mask_str.push_str("!*@*"); // The mask looks like "nick".
            } else {
                mask_str.insert_str(0, "*!*@"); // The mask looks like "host".
            }
        } else if pos_of_pling.is_none() && pos_of_at.is_some() {
            // The mask looks like "user@host".
            mask_str.insert_str(0, "*!");
        } else if pos_of_pling.is_some() && pos_of_at.is_none() {
            // The mask looks like "nick!user".
            mask_str.push_str("@*");
        } else if let (Some(pling), Some(at)) = (pos_of_pling, pos_of_at) {
            if at - pling == 1 {
                // The mask looks like "nick!@host".
                mask_str.insert(at, '*');
            }
        }
        
        // Convert back to StdString
        let new_bytes = mask_str.into_bytes();
        // Destroy the old string
        StdString_Destroy(s);
        // Create new one
        *s = StdString::from_vec(new_bytes);
    }
}
