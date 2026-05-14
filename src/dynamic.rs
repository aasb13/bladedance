/*
 * InspIRCd -- Internet Relay Chat Daemon
 *
 *   Copyright (C) 2020 Matt Schatz <genius3000@g3k.solutions>
 *   Copyright (C) 2017-2020, 2022-2023 Sadie Powell <sadie@witchery.services>
 *   Copyright (C) 2012 Robby <robby@chatbelgie.be>
 *   Copyright (C) 2012 ChrisTX <xpipe@hotmail.de>
 *   Copyright (C) 2009-2010 Daniel De Graaf <danieldg@inspircd.org>
 *   Copyright (C) 2009 Uli Schlachter <psychon@znc.in>
 *   Copyright (C) 2007 Robin Burchell <robin+git@viroteck.net>
 *   Copyright (C) 2003, 2006 Craig Edwards <brain@inspircd.org>
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

#![allow(non_upper_case_globals)]
#![allow(non_camel_case_types)]
#![allow(non_snake_case)]
#![allow(dead_code)]

use std::ffi::{c_char, c_void, CString, CStr};
use std::ptr;

// Constants for module ABI
const MODULE_ABI: u64 = 4010;
const RUST_MODULE_ABI: u64 = 4011;

// Symbol names
const MODULE_SYM_ABI: &str = "inspircd_module_abi";
const MODULE_SYM_INIT: &str = "inspircd_module_init";
const MODULE_SYM_VERSION: &str = "inspircd_module_version";
const RUST_MODULE_VTABLE: &str = "rust_module_vtable";

// Platform-specific DLL extension
#[cfg(target_os = "macos")]
const DLL_EXTENSION: &str = ".dylib";
#[cfg(target_os = "windows")]
const DLL_EXTENSION: &str = ".dll";
#[cfg(not(any(target_os = "macos", target_os = "windows")))]
const DLL_EXTENSION: &str = ".so";

// External C++ functions for module creation and error handling
unsafe extern "C" {
    fn dynamic_ffi_create_rust_module_wrapper(
        vtable: *const c_void,
        rust_handle: *mut c_void,
        libname: *const c_char,
        libname_length: usize,
    ) -> *mut c_void;
    
    fn dynamic_ffi_get_last_error() -> *mut c_char;
    fn dynamic_ffi_free_error_string(ptr: *mut c_char);
    
    fn dynamic_ffi_format_version_error(
        libname: *const c_char,
        libname_length: usize,
        version: *const c_char,
        abi: u64,
        module_abi: u64,
    ) -> *mut c_char;
    
    fn dynamic_ffi_set_error_string(ptr: *mut c_char);
}

/// Represents a loaded dynamic library
pub struct DLLManager {
    libname: String,
    handle: Option<libloading::Library>,
    error: String,
}

impl DLLManager {
    /// Creates a new DLLManager and attempts to load the specified library
    pub fn new(name: &str) -> Self {
        let mut manager = DLLManager {
            libname: name.to_string(),
            handle: None,
            error: String::new(),
        };

        // Check if the name has the correct extension
        if !name.ends_with(DLL_EXTENSION) {
            manager.error = format!("{} is not a module (no {} extension)", name, DLL_EXTENSION);
            return manager;
        }

        // Preload MongoDB libraries on Unix systems (required for some modules)
        #[cfg(unix)]
        {
            let _ = unsafe { libloading::Library::new("libmongoc-1.0.so") };
            let _ = unsafe { libloading::Library::new("libbson-1.0.so") };
        }

        // Load the library
        match unsafe { libloading::Library::new(name) } {
            Ok(lib) => manager.handle = Some(lib),
            Err(_) => manager.retrieve_last_error(),
        }

        manager
    }

    /// Attempts to create a new module instance from this shared library
    pub fn call_init(&mut self) -> Option<*mut c_void> {
        if self.handle.is_none() {
            return None;
        }

        let lib = self.handle.as_ref().unwrap();

        // Get the ABI symbol
        let abi: *const u64 = match unsafe { lib.get::<*const u64>(MODULE_SYM_ABI.as_bytes()) } {
            Ok(symbol) => *symbol,
            Err(_) => {
                self.error = format!("{} is not a module (no ABI symbol)", self.libname);
                return None;
            }
        };

        // Dereference the ABI value
        let abi_value = unsafe { *abi };

        // Check if it's a Rust module
        if abi_value == RUST_MODULE_ABI {
            return self.init_rust_module();
        }

        // Check if it's a C++ module with compatible ABI
        if abi_value != MODULE_ABI {
            return self.init_abi_mismatch(abi_value);
        }

        // It's a C++ module - call the init function
        self.init_cpp_module()
    }

    /// Initializes a Rust module
    fn init_rust_module(&mut self) -> Option<*mut c_void> {
        let lib = self.handle.as_ref().unwrap();

        // Get the vtable symbol
        let vtable: *const c_void = match unsafe { lib.get::<*const c_void>(RUST_MODULE_VTABLE.as_bytes()) } {
            Ok(symbol) => *symbol,
            Err(_) => {
                self.error = format!("{} is not a Rust module (no vtable symbol)", self.libname);
                return None;
            }
        };

        // Get the init function
        let init_fn: libloading::Symbol<unsafe extern "C" fn() -> *mut c_void> =
            match unsafe { lib.get::<unsafe extern "C" fn() -> *mut c_void>(MODULE_SYM_INIT.as_bytes()) } {
                Ok(symbol) => symbol,
                Err(_) => {
                    self.error = format!("{} is not a Rust module (no init symbol)", self.libname);
                    return None;
                }
            };

        // Call the init function to get the rust handle
        let rust_handle = unsafe { init_fn() };

        // Create the RustModuleWrapper via C++ FFI
        let libname_cstr = match CString::new(&*self.libname) {
            Ok(s) => s,
            Err(_) => {
                self.error = format!("Failed to convert library name to C string");
                return None;
            }
        };

        let module_ptr = unsafe {
            dynamic_ffi_create_rust_module_wrapper(
                vtable,
                rust_handle,
                libname_cstr.as_ptr(),
                self.libname.len(),
            )
        };

        if module_ptr.is_null() {
            self.retrieve_last_error();
        }

        Some(module_ptr)
    }

    /// Handles ABI mismatch
    fn init_abi_mismatch(&mut self, abi: u64) -> Option<*mut c_void> {
        let lib = self.handle.as_ref().unwrap();

        // Get the version string
        let version: *const c_char = match unsafe { lib.get::<*const c_char>(MODULE_SYM_VERSION.as_bytes()) } {
            Ok(symbol) => *symbol,
            Err(_) => ptr::null(),
        };

        let libname_cstr = match CString::new(&*self.libname) {
            Ok(s) => s,
            Err(_) => {
                self.error = format!("Failed to convert library name to C string");
                return None;
            }
        };

        let error_ptr = unsafe {
            dynamic_ffi_format_version_error(
                libname_cstr.as_ptr(),
                self.libname.len(),
                version,
                abi,
                MODULE_ABI,
            )
        };

        if !error_ptr.is_null() {
            unsafe {
                dynamic_ffi_set_error_string(error_ptr);
                dynamic_ffi_free_error_string(error_ptr);
            }
        }

        None
    }

    /// Initializes a C++ module
    fn init_cpp_module(&mut self) -> Option<*mut c_void> {
        let lib = self.handle.as_ref().unwrap();

        let init_fn: libloading::Symbol<unsafe extern "C" fn() -> *mut c_void> =
            match unsafe { lib.get::<unsafe extern "C" fn() -> *mut c_void>(MODULE_SYM_INIT.as_bytes()) } {
                Ok(symbol) => symbol,
                Err(_) => {
                    self.error = format!("{} is not a module (no init symbol)", self.libname);
                    return None;
                }
            };

        let module_ptr = unsafe { init_fn() };

        if module_ptr.is_null() {
            self.error = format!("Failed to initialize module {}", self.libname);
        }

        Some(module_ptr)
    }

    /// Retrieves the value of the specified symbol
    pub fn get_symbol(&self, name: &str) -> Option<*const c_void> {
        let lib = self.handle.as_ref()?;
        unsafe { lib.get::<*const c_void>(name.as_bytes()).ok().map(|s| *s) }
    }

    /// Retrieves the last error from the OS
    fn retrieve_last_error(&mut self) {
        unsafe {
            let error_ptr = dynamic_ffi_get_last_error();
            if !error_ptr.is_null() {
                let error_cstr = CStr::from_ptr(error_ptr);
                self.error = error_cstr.to_string_lossy().into_owned();
                
                // Clean up newlines in error message
                self.error = self.error.replace('\r', " ").replace('\n', " ");
                
                dynamic_ffi_free_error_string(error_ptr);
            } else {
                self.error = "Unknown error".to_string();
            }
        }
    }

    /// Returns the last error message
    pub fn last_error(&self) -> &str {
        &self.error
    }

    /// Returns the library name
    pub fn library_name(&self) -> &str {
        &self.libname
    }
}

impl Drop for DLLManager {
    fn drop(&mut self) {
        // The Library handle will be automatically closed when dropped
        self.handle = None;
    }
}

// FFI exports for C++

#[unsafe(no_mangle)]
pub unsafe extern "C" fn DLLManager_Create(name: *const c_char, name_length: usize) -> *mut c_void {
    if name.is_null() {
        return ptr::null_mut();
    }

    let name_data = unsafe { std::slice::from_raw_parts(name as *const u8, name_length) };
    let name_str = String::from_utf8_lossy(name_data);
    
    let manager = Box::new(DLLManager::new(&name_str));
    Box::into_raw(manager) as *mut c_void
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn DLLManager_Destroy(ptr: *mut c_void) {
    if ptr.is_null() {
        return;
    }
    let _ = unsafe { Box::from_raw(ptr as *mut DLLManager) };
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn DLLManager_CallInit(ptr: *mut c_void) -> *mut c_void {
    if ptr.is_null() {
        return ptr::null_mut();
    }

    let manager = unsafe { &mut *(ptr as *mut DLLManager) };
    manager.call_init().unwrap_or(ptr::null_mut())
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn DLLManager_GetSymbol(
    ptr: *mut c_void,
    name: *const c_char,
) -> *const c_void {
    if ptr.is_null() || name.is_null() {
        return ptr::null();
    }

    let manager = unsafe { &*(ptr as *mut DLLManager) };
    let name_str = unsafe { CStr::from_ptr(name) }.to_str().unwrap_or("");
    
    manager.get_symbol(name_str).unwrap_or(ptr::null())
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn DLLManager_LastError(ptr: *mut c_void) -> *mut c_char {
    if ptr.is_null() {
        return ptr::null_mut();
    }

    let manager = unsafe { &*(ptr as *mut DLLManager) };
    let error = manager.last_error();
    
    match CString::new(error) {
        Ok(s) => s.into_raw(),
        Err(_) => ptr::null_mut(),
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn DLLManager_LibraryName(ptr: *mut c_void) -> *mut c_char {
    if ptr.is_null() {
        return ptr::null_mut();
    }

    let manager = unsafe { &*(ptr as *mut DLLManager) };
    let name = manager.library_name();
    
    match CString::new(name) {
        Ok(s) => s.into_raw(),
        Err(_) => ptr::null_mut(),
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn DLLManager_FreeString(ptr: *mut c_char) {
    if !ptr.is_null() {
        unsafe {
            let _ = CString::from_raw(ptr);
        }
    }
}
