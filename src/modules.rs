/*
 * InspIRCd -- Internet Relay Chat Daemon
 *
 *   Copyright (C) 2021 Dominic Hamon
 *   Copyright (C) 2017, 2021-2024 Sadie Powell <sadie@witchery.services>
 *   Copyright (C) 2014 Attila Molnar <attilamolnar@hush.com>
 *   Copyright (C) 2012 Robby <robby@chatbelgie.be>
 *   Copyright (C) 2010 Craig Edwards <brain@inspircd.org>
 *   Copyright (C) 2009-2010 Daniel De Graaf <danieldg@inspircd.org>
 *   Copyright (C) 2009 Uli Schlachter <psychon@znc.in>
 *   Copyright (C) 2007 Robin Burchell <robin+git@viroteck.net>
 *   Copyright (C) 2007 Dennis Friis <peavey@inspircd.org>
 *
 * This file is part of InspIRCd.  InspIRCd is free software: you can
 * redistribute it and/or modify it under the terms of the GNU General Public
 * License as published by the Free Software Foundation, version 2.
 *
 * This program is distributed in the hope that it will be useful, but WITHOUT
 * ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS
 * FOR A PARTICULAR PURPOSE.  See the GNU General Public License for more
 * details.
 *
 * You should have received a copy of the GNU General Public License
 * along with this program.  If not, see <http://www.gnu.org/licenses/>.
 */

use std::ffi::{c_char, CString};
use crate::stringutils::StdString;

// Service type constants matching C++ enum
pub const SERVICE_COMMAND: u32 = 0;
pub const SERVICE_MODE: u32 = 1;
pub const SERVICE_METADATA: u32 = 2;
pub const SERVICE_IOHOOK: u32 = 3;
pub const SERVICE_DATA: u32 = 4;
pub const SERVICE_CUSTOM: u32 = 5;

// Module property flags
pub const VF_CORE: u32 = 1;
pub const VF_VENDOR: u32 = 2;
pub const VF_COMMON: u32 = 4;
pub const VF_OPTCOMMON: u32 = 8;
pub const VF_DEPRECATED: u32 = 16;
pub const VF_LAST: u32 = VF_DEPRECATED;

/// Returns the string representation of a service type.
/// @param service_type The service type.
/// @return The string representation of the service type.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn service_provider_get_type_string(service_type: u32) -> *const c_char {
    let type_str = match service_type {
        SERVICE_COMMAND => "command",
        SERVICE_MODE => "mode",
        SERVICE_METADATA => "metadata",
        SERVICE_IOHOOK => "iohook",
        SERVICE_DATA => "data service",
        SERVICE_CUSTOM => "module service",
        _ => "unknown service",
    };
    CString::new(type_str).unwrap().into_raw()
}

/// Returns the property string for a module based on its property flags.
/// @param properties The module property flags.
/// @return The property string.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn module_get_property_string(properties: u32) -> StdString {
    // R = VF_CORE ("required")
    // V = VF_VENDOR
    // C = VF_COMMON
    // O = VF_OPTCOMMON
    // D = VF_DEPRECATED
    let mut propstr = Vec::from("RVCOD".as_bytes());
    
    let mut pos = 0usize;
    let mut mult = VF_CORE;
    while mult <= VF_LAST {
        if (properties & mult) == 0 {
            propstr[pos] = b'-';
        }
        pos += 1;
        mult *= 2;
    }
    
    StdString::from_vec(propstr)
}

/// Shrinks a module name by removing the "m_" prefix and ".so" extension.
/// @param modname The module name to shrink.
/// @param modname_length The length of the module name.
/// @return The shrunk module name.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn module_manager_shrink_mod_name(modname: *const c_char, modname_length: usize) -> StdString {
    if modname.is_null() || modname_length == 0 {
        return StdString::from_vec(Vec::new());
    }
    
    let modname_slice = unsafe { std::slice::from_raw_parts(modname as *const u8, modname_length) };
    let modname_str = String::from_utf8_lossy(modname_slice);
    
    // Remove "m_" prefix if present
    let start_pos = if modname_str.starts_with("m_") { 2 } else { 0 };
    
    // Remove ".so" extension if present (assuming DLL_EXTENSION is ".so")
    let end_pos = if modname_str.ends_with(".so") { 3 } else { 0 };
    
    let result = modname_str[start_pos..modname_str.len() - end_pos].to_string();
    StdString::from_vec(result.into_bytes())
}