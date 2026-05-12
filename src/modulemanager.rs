/*
 * InspIRCd -- Internet Relay Chat Daemon
 *
 *   Copyright (C) 2013, 2015, 2019-2024, 2026 Sadie Powell <sadie@witchery.services>
 */

#![allow(unsafe_op_in_unsafe_fn)]
#![allow(non_upper_case_globals)]
#![allow(non_camel_case_types)]
#![allow(non_snake_case)]
#![allow(dead_code)]

use std::ffi::{c_char, c_void};

// Import StdString from stringutils module
use crate::stringutils::StdString;

// Simple module manager that delegates to C++ for complex operations
// This demonstrates the pattern of moving logic to Rust while keeping the glue thin

pub struct ModuleManager {
    // For now, this is a placeholder that will hold state
    // The actual logic will be moved from C++ gradually
}

impl ModuleManager {
    pub fn new() -> Self {
        ModuleManager {}
    }
    
    // This function contains the logic that was previously in C++
    // It's a simplified version that demonstrates the approach
    pub fn module_name_contains_path(&self, name: &str) -> bool {
        // Simple Rust implementation of the path validation logic
        name.bytes().any(|b| b == b'\\' || b == b'/')
    }
    
    /// Expands a module name by adding "m_" prefix and ".so" extension if needed
    /// 
    /// # Arguments
    /// * `modname` - The module name to expand
    /// 
    /// # Returns
    /// A String containing the expanded module name with "m_" prefix and ".so" extension
    pub fn expand_mod_name(&self, modname: &str) -> String {
        const DLL_EXTENSION: &str = ".so";
        let mut fullname = String::new();
        
        // Add "m_" prefix if it doesn't start with "core_" or "m_"
        if !modname.starts_with("core_") && !modname.starts_with("m_") {
            fullname.push_str("m_");
        }
        
        fullname.push_str(modname);
        
        // Add ".so" extension if it doesn't already have it
        if !modname.ends_with(DLL_EXTENSION) {
            fullname.push_str(DLL_EXTENSION);
        }
        
        fullname
    }
    
    /// Validates that a module file exists and is a regular file
    /// 
    /// # Arguments
    /// * `module_file_path` - The full path to the module file (PrependModule result)
    /// * `filename` - The module filename for error messages
    /// 
    /// # Returns
    /// Result<(), String> - Ok(()) if file exists and is regular, Err(error_message) otherwise
    pub fn validate_module_file(&self, module_file_path: &str, filename: &str) -> Result<(), String> {
        use std::path::Path;
        
        // Check if the file exists and is a regular file (replicates std::filesystem::is_regular_file)
        let path = Path::new(module_file_path);
        if !path.exists() || !path.is_file() {
            return Err(format!("Module file could not be found: {}", filename));
        }
        
        Ok(())
    }
    
    // This would contain the Load logic from C++ in a full implementation
    // For now, it's a placeholder to show the structure
    pub fn load_module(&self, modname: &str) -> bool {
        // Validate module name (this is the part we can move to Rust)
        if self.module_name_contains_path(modname) {
            return false;
        }
        
        // The rest of the loading logic would be moved here gradually
        // For now, we return true to indicate success
        true
    }
}

impl Drop for ModuleManager {
    fn drop(&mut self) {
        // Cleanup if needed
    }
}

// The existing function that was already in Rust
#[unsafe(no_mangle)]
pub unsafe extern "C" fn rust_module_name_contains_path(name: *const c_char) -> bool {
    if name.is_null() {
        return false;
    }
    let b = std::ffi::CStr::from_ptr(name).to_bytes();
    b.iter().any(|&x| x == b'\\' || x == b'/')
}

// C-compatible wrapper for ExpandModName
#[unsafe(no_mangle)]
pub unsafe extern "C" fn rust_expand_mod_name(modname: *const c_char, modname_length: usize) -> StdString {
    if modname.is_null() {
        return StdString::from_vec(Vec::new());
    }
    
    let modname_data = unsafe { std::slice::from_raw_parts(modname as *const u8, modname_length) };
    let modname_str = String::from_utf8_lossy(modname_data);
    let manager = ModuleManager::new();
    let expanded = manager.expand_mod_name(&modname_str);
    
    StdString::from_vec(expanded.into_bytes())
}

// C-compatible wrapper for validate_module_file
#[unsafe(no_mangle)]
pub unsafe extern "C" fn rust_validate_module_file(
    module_file_path: *const c_char, 
    module_file_path_length: usize,
    filename: *const c_char,
    filename_length: usize
) -> StdString {
    if module_file_path.is_null() || filename.is_null() {
        return StdString::from_vec("Invalid parameters".to_string().into_bytes());
    }
    
    let module_file_path_data = unsafe { std::slice::from_raw_parts(module_file_path as *const u8, module_file_path_length) };
    let filename_data = unsafe { std::slice::from_raw_parts(filename as *const u8, filename_length) };
    
    let module_file_path_str = String::from_utf8_lossy(module_file_path_data);
    let filename_str = String::from_utf8_lossy(filename_data);
    
    let manager = ModuleManager::new();
    match manager.validate_module_file(&module_file_path_str, &filename_str) {
        Ok(()) => StdString::from_vec(Vec::new()), // Empty string for success
        Err(error) => StdString::from_vec(error.into_bytes()),
    }
}

// C-compatible wrapper functions for the new Rust logic
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ModuleManager_Create() -> *mut c_void {
    let manager = Box::new(ModuleManager::new());
    Box::into_raw(manager) as *mut c_void
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ModuleManager_Destroy(ptr: *mut c_void) {
    if ptr.is_null() {
        return;
    }
    unsafe {
        let _ = Box::from_raw(ptr as *mut ModuleManager);
    }
}

// Example of how we would move the Load logic to Rust
// This is a simplified version to demonstrate the pattern
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ModuleManager_ValidateModuleName(ptr: *mut c_void, modname: *const c_char) -> bool {
    if ptr.is_null() || modname.is_null() {
        return false;
    }

    let manager = unsafe { &*(ptr as *mut ModuleManager) };
    let modname_str = unsafe { std::ffi::CStr::from_ptr(modname) }.to_str().unwrap_or("");
    
    manager.module_name_contains_path(modname_str)
}
