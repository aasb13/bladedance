// This file is a Rust port of a corresponding InspIRCd module.
// Original work Copyright (C) the InspIRCd contributors.
// Licensed under GPLv2. See LICENSE for details.

use std::collections::HashMap;
use std::ffi::CString;
use std::os::raw::c_char;
use std::ptr;
use std::sync::Mutex;
use tracing::{debug, error, info, warn, trace};

use crate::stringutils::StdString;

#[repr(C)]
pub enum FileMethodTarget {
    FILE = 0,
    STDOUT = 1,
    STDERR = 2,
}

pub struct FileMethodHandle {
    target: FileMethodTarget,
    name: String,
}

// Implement Send and Sync for FileMethodHandle since we only use it in a single-threaded context
unsafe impl Send for FileMethodHandle {}
unsafe impl Sync for FileMethodHandle {}

lazy_static::lazy_static! {
    static ref FILE_HANDLES: Mutex<HashMap<usize, FileMethodHandle>> = Mutex::new(HashMap::new());
    static ref NEXT_HANDLE_ID: Mutex<usize> = Mutex::new(1);
}

/// Helper function to safely convert raw bytes to a String, handling invalid UTF-8
unsafe fn from_raw_parts_lossy(ptr: *const u8, len: usize) -> String {
    unsafe { String::from_utf8_lossy(std::slice::from_raw_parts(ptr, len)).into_owned() }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn LogManager_Log(
    level: u8,
    ty: *const u8,
    ty_len: usize,
    msg: *const u8,
    msg_len: usize,
) {
    let ty_s = if ty.is_null() || ty_len == 0 {
        String::new()
    } else {
        unsafe { from_raw_parts_lossy(ty, ty_len) }
    };

    let msg_s = if msg.is_null() || msg_len == 0 {
        String::new()
    } else {
        unsafe { from_raw_parts_lossy(msg, msg_len) }
    };

    // Map InspIRCd log levels to tracing levels
    match level {
        0 => error!(ty = %ty_s, "{}", msg_s), // CRITICAL
        1 => warn!(ty = %ty_s, "{}", msg_s),  // WARNING
        2 => info!(ty = %ty_s, "{}", msg_s),  // NORMAL
        3 => debug!(ty = %ty_s, "{}", msg_s), // DEBUG
        4 => debug!(ty = %ty_s, "{}", msg_s), // RAWIO
        _ => info!(ty = %ty_s, "{}", msg_s),  // UNKNOWN
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn rust_log_level_to_string(level: u8) -> *const c_char {
    let str = match level {
        0 => "critical",
        1 => "warning",
        2 => "normal",
        3 => "debug",
        4 => "rawio",
        _ => "unknown",
    };
    str.as_ptr() as *const c_char
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn rust_log_filemethod_create(
    target: *const c_char,
    target_length: usize,
    _flush: u64,
    kind: u8,
) -> *mut std::os::raw::c_void {
    if target.is_null() || target_length == 0 {
        return ptr::null_mut();
    }

    let target_str = unsafe {
        from_raw_parts_lossy(target as *const u8, target_length)
    };
    let name = target_str.to_string();

    let file_target = match kind {
        1 => FileMethodTarget::STDOUT,
        2 => FileMethodTarget::STDERR,
        _ => FileMethodTarget::FILE,
    };

    let handle = FileMethodHandle {
        target: file_target,
        name,
    };

    let mut handles = FILE_HANDLES.lock().unwrap();
    let mut next_id = NEXT_HANDLE_ID.lock().unwrap();
    let id = *next_id;
    *next_id += 1;
    handles.insert(id, handle);

    Box::into_raw(Box::new(id)) as *mut std::os::raw::c_void
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn rust_log_filemethod_destroy(handle: *mut std::os::raw::c_void) {
    if handle.is_null() {
        return;
    }

    let id = unsafe { Box::from_raw(handle as *mut usize) };
    let mut handles = FILE_HANDLES.lock().unwrap();
    handles.remove(&*id);
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn rust_log_filemethod_on_log(
    handle: *mut std::os::raw::c_void,
    _time: i64,
    level: u8,
    type_str: *const c_char,
    type_length: usize,
    message: *const c_char,
    message_length: usize,
) -> StdString {
    if handle.is_null() {
        return StdString::from_vec("Invalid handle".to_string().into_bytes());
    }

    let id = unsafe { *(handle as *mut usize) };
    let handles = FILE_HANDLES.lock().unwrap();
    
    if let Some(_method) = handles.get(&id) {
        let type_s = if type_str.is_null() || type_length == 0 {
            String::new()
        } else {
            unsafe { from_raw_parts_lossy(type_str as *const u8, type_length) }
        };

        let message_s = if message.is_null() || message_length == 0 {
            String::new()
        } else {
            unsafe { from_raw_parts_lossy(message as *const u8, message_length) }
        };

        match level {
            0 => error!(log_type = %type_s, "{}", message_s),
            1 => warn!(log_type = %type_s, "{}", message_s),
            2 => info!(log_type = %type_s, "{}", message_s),
            3 => debug!(log_type = %type_s, "{}", message_s),
            4 => trace!(log_type = %type_s, "{}", message_s),
            _ => info!(log_type = %type_s, "{}", message_s),
        }

        StdString::from_vec(Vec::new())
    } else {
        StdString::from_vec("Handle not found".to_string().into_bytes())
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn rust_log_debug_method_on_log(
    _time: i64,
    level: u8,
    type_str: *const c_char,
    type_length: usize,
    message: *const c_char,
    message_length: usize,
) {
    let type_s = if type_str.is_null() || type_length == 0 {
        String::new()
    } else {
        unsafe { from_raw_parts_lossy(type_str as *const u8, type_length) }
    };

    let message_s = if message.is_null() || message_length == 0 {
        String::new()
    } else {
        unsafe { from_raw_parts_lossy(message as *const u8, message_length) }
    };

    match level {
        0 => error!(log_type = %type_s, "{}", message_s),
        1 => warn!(log_type = %type_s, "{}", message_s),
        2 => info!(log_type = %type_s, "{}", message_s),
        3 => debug!(log_type = %type_s, "{}", message_s),
        4 => trace!(log_type = %type_s, "{}", message_s),
        _ => info!(log_type = %type_s, "{}", message_s),
    }
}

// Manager state and functions
#[repr(C)]
pub struct LoggerInfo {
    level: u8,
    method_handle: *mut std::os::raw::c_void,
    config: bool,
    dead: bool,
}

// Implement Send and Sync for LoggerInfo since we only use it in a single-threaded context
unsafe impl Send for LoggerInfo {}
unsafe impl Sync for LoggerInfo {}

lazy_static::lazy_static! {
    static ref LOG_MANAGER_STATE: Mutex<LogManagerState> = Mutex::new(LogManagerState::new());
}

pub struct LogManagerState {
    loggers: Vec<LoggerInfo>,
    maxlevel: u8,
    logging: bool,
    caching: bool,
    cache: Vec<CachedMessage>,
}

impl LogManagerState {
    fn new() -> Self {
        Self {
            loggers: Vec::new(),
            maxlevel: 0, // Level::LOWEST
            logging: false,
            caching: true,
            cache: Vec::new(),
        }
    }
}

#[repr(C)]
pub struct CachedMessage {
    time: i64,
    level: u8,
    type_str: *mut c_char,
    type_length: usize,
    message: *mut c_char,
    message_length: usize,
}

// Implement Send and Sync for CachedMessage since we only use it in a single-threaded context
unsafe impl Send for CachedMessage {}
unsafe impl Sync for CachedMessage {}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn rust_log_manager_write(
    level: u8,
    type_str: *const c_char,
    type_length: usize,
    message: *const c_char,
    message_length: usize,
) {
    let type_s = if type_str.is_null() || type_length == 0 {
        String::new()
    } else {
        unsafe { from_raw_parts_lossy(type_str as *const u8, type_length) }
    };

    let message_s = if message.is_null() || message_length == 0 {
        String::new()
    } else {
        unsafe { from_raw_parts_lossy(message as *const u8, message_length) }
    };

    let mut state = LOG_MANAGER_STATE.lock().unwrap();

    if state.logging {
        return; // Avoid log loops
    }

    if state.maxlevel < level && !state.caching {
        return; // No loggers care about this message
    }

    state.logging = true;

    // Log to tracing - this is the only logging we need now
    match level {
        0 => error!(log_type = %type_s, "{}", message_s), // CRITICAL
        1 => warn!(log_type = %type_s, "{}", message_s),  // WARNING
        2 => info!(log_type = %type_s, "{}", message_s),  // NORMAL
        3 => debug!(log_type = %type_s, "{}", message_s), // DEBUG
        4 => trace!(log_type = %type_s, "{}", message_s), // RAWIO
        _ => info!(log_type = %type_s, "{}", message_s),  // UNKNOWN
    }

    if state.caching {
        // Cache the message
        let type_len = type_s.len();
        let message_len = message_s.len();
        let type_copy = CString::new(type_s).unwrap();
        let message_copy = CString::new(message_s).unwrap();
        
        let cached = CachedMessage {
            time: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs() as i64,
            level,
            type_str: type_copy.into_raw(),
            type_length: type_len,
            message: message_copy.into_raw(),
            message_length: message_len,
        };
        state.cache.push(cached);
    }
    
    state.logging = false;
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn rust_log_manager_enable_debug_mode(forceprotodebug: bool) {
    let mut state = LOG_MANAGER_STATE.lock().unwrap();
    
    // Create a debug logger handle
    let debug_handle = std::ptr::null_mut(); // Special handle for debug logging
    let level = if forceprotodebug { 4 } else { 3 }; // RAWIO or DEBUG
    
    let logger = LoggerInfo {
        level,
        method_handle: debug_handle,
        config: false,
        dead: false,
    };
    
    state.loggers.push(logger);
    state.maxlevel = level;
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn rust_log_manager_check_level() -> u8 {
    let mut state = LOG_MANAGER_STATE.lock().unwrap();
    
    let mut newmaxlevel = 0; // Level::LOWEST
    for logger in state.loggers.iter() {
        if !logger.dead && logger.level > newmaxlevel {
            newmaxlevel = logger.level;
        }
    }
    
    state.maxlevel = newmaxlevel;
    newmaxlevel
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn rust_log_manager_add_logger(
    level: u8,
    method_handle: *mut std::os::raw::c_void,
    config: bool,
) {
    let mut state = LOG_MANAGER_STATE.lock().unwrap();
    
    let logger = LoggerInfo {
        level,
        method_handle,
        config,
        dead: false,
    };
    
    state.loggers.push(logger);
    
    if level > state.maxlevel {
        state.maxlevel = level;
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn rust_log_manager_open_logs(requiremethods: bool) {
    let mut state = LOG_MANAGER_STATE.lock().unwrap();
    
    if requiremethods && state.caching {
        state.logging = true;
        
        // Collect cached messages first to avoid borrowing issues
        let cached_messages: Vec<CachedMessage> = state.cache.drain(..).collect();
        
        for cached in cached_messages {
            let type_s = unsafe { from_raw_parts_lossy(cached.type_str as *const u8, cached.type_length) };
            let message_s = unsafe { from_raw_parts_lossy(cached.message as *const u8, cached.message_length) };

            // Emit tracing event for cached message
            match cached.level {
                0 => error!(log_type = %type_s, "{}", message_s), // CRITICAL
                1 => warn!(log_type = %type_s, "{}", message_s),  // WARNING
                2 => info!(log_type = %type_s, "{}", message_s),  // NORMAL
                3 => debug!(log_type = %type_s, "{}", message_s), // DEBUG
                4 => trace!(log_type = %type_s, "{}", message_s), // RAWIO
                _ => info!(log_type = %type_s, "{}", message_s),  // UNKNOWN
            }

            // Free cached strings
            unsafe {
                let _ = CString::from_raw(cached.type_str);
                let _ = CString::from_raw(cached.message);
            }
        }
        
        state.caching = false;
        state.logging = false;
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn rust_log_manager_close_logs() {
    let mut state = LOG_MANAGER_STATE.lock().unwrap();
    
    state.logging = true; // Prevent writing to dying loggers
    state.loggers.retain(|logger| !logger.config);
    state.logging = false;
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn rust_log_manager_get_maxlevel() -> u8 {
    let state = LOG_MANAGER_STATE.lock().unwrap();
    state.maxlevel
}

#[deprecated(since = "0.1.0", note = "Use tracing macros (error!, warn!, info!, debug!, trace!) directly")]
pub fn log(level: u8, log_type: &str, message: &str) {
    match level {
        0 => error!(log_type, "{}", message), // CRITICAL
        1 => warn!(log_type, "{}", message),  // WARNING
        2 => info!(log_type, "{}", message),  // NORMAL
        3 => debug!(log_type, "{}", message), // DEBUG
        4 => trace!(log_type, "{}", message), // RAWIO
        _ => info!(log_type, "{}", message),  // UNKNOWN
    }
}
