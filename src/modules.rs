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