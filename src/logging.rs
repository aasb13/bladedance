use std::collections::HashMap;
use std::ffi::{CStr, CString};
use std::fs::OpenOptions;
use std::io::Write;
use std::os::raw::{c_char, c_int};
use std::ptr;
use std::slice;
use std::sync::Mutex;
use tracing::info;

use crate::stringutils::StdString;

type time_t = i64;

#[repr(C)]
struct tm {
    tm_sec: c_int,
    tm_min: c_int,
    tm_hour: c_int,
    tm_mday: c_int,
    tm_mon: c_int,
    tm_year: c_int,
    tm_wday: c_int,
    tm_yday: c_int,
    tm_isdst: c_int,
}

unsafe extern "C" {
    fn localtime(timer: *const time_t) -> *mut tm;
    fn strftime(s: *mut c_char, maxsize: usize, format: *const c_char, timeptr: *const tm) -> usize;
}

#[repr(C)]
pub enum FileMethodTarget {
    FILE = 0,
    STDOUT = 1,
    STDERR = 2,
}

pub struct FileMethodHandle {
    file: Option<std::fs::File>,
    target: FileMethodTarget,
    name: String,
    flush: u64,
    lines: u64,
    prevtime: i64,
    timestr: String,
}

lazy_static::lazy_static! {
    static ref FILE_HANDLES: Mutex<HashMap<usize, FileMethodHandle>> = Mutex::new(HashMap::new());
    static ref NEXT_HANDLE_ID: Mutex<usize> = Mutex::new(1);
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
        ""
    } else {
        unsafe { std::str::from_utf8_unchecked(std::slice::from_raw_parts(ty, ty_len)) }
    };

    let msg_s = if msg.is_null() || msg_len == 0 {
        ""
    } else {
        unsafe { std::str::from_utf8_unchecked(std::slice::from_raw_parts(msg, msg_len)) }
    };

    info!(level = level, ty = ty_s, "{}", msg_s);
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
    flush: u64,
    kind: u8,
) -> *mut std::os::raw::c_void {
    if target.is_null() || target_length == 0 {
        return ptr::null_mut();
    }

    let target_str = unsafe {
        std::str::from_utf8_unchecked(slice::from_raw_parts(target as *const u8, target_length))
    };
    let name = target_str.to_string();

    let file_target = match kind {
        1 => FileMethodTarget::STDOUT,
        2 => FileMethodTarget::STDERR,
        _ => FileMethodTarget::FILE,
    };

    let file = match file_target {
        FileMethodTarget::FILE => {
            match OpenOptions::new().create(true).append(true).open(&name) {
                Ok(f) => Some(f),
                Err(_) => return ptr::null_mut(),
            }
        }
        FileMethodTarget::STDOUT | FileMethodTarget::STDERR => None,
    };

    let handle = FileMethodHandle {
        file,
        target: file_target,
        name,
        flush,
        lines: 0,
        prevtime: 0,
        timestr: String::new(),
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
    time: i64,
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
    let mut handles = FILE_HANDLES.lock().unwrap();
    
    if let Some(method) = handles.get_mut(&id) {
        if method.prevtime != time {
            method.prevtime = time;
            
            let format_str = CString::new("%d %b %H:%M:%S").unwrap();
            let mut buffer = [0u8; 64];
            
            unsafe {
                let tm_ptr = localtime(&time);
                if !tm_ptr.is_null() {
                    strftime(buffer.as_mut_ptr() as *mut c_char, buffer.len(), format_str.as_ptr(), tm_ptr);
                    let c_str = CStr::from_ptr(buffer.as_ptr() as *const c_char);
                    method.timestr = c_str.to_string_lossy().to_string();
                } else {
                    method.timestr = format!("{}", time);
                }
            }
        }

        let type_s = if type_str.is_null() || type_length == 0 {
            ""
        } else {
            unsafe { std::str::from_utf8_unchecked(slice::from_raw_parts(type_str as *const u8, type_length)) }
        };

        let message_s = if message.is_null() || message_length == 0 {
            ""
        } else {
            unsafe { std::str::from_utf8_unchecked(slice::from_raw_parts(message as *const u8, message_length)) }
        };

        let log_line = format!("{} {}: {}\n", method.timestr, type_s, message_s);

        let result = match method.target {
            FileMethodTarget::FILE => {
                if let Some(ref mut file) = method.file {
                    match file.write_all(log_line.as_bytes()) {
                        Ok(_) => {
                            method.lines += 1;
                            if method.flush > 0 && method.lines % method.flush == 0 {
                                let _ = file.flush();
                            }
                            StdString::from_vec(Vec::new())
                        }
                        Err(e) => StdString::from_vec(format!("Unable to write to {}: {}", method.name, e).into_bytes()),
                    }
                } else {
                    StdString::from_vec("File handle is null".to_string().into_bytes())
                }
            }
            FileMethodTarget::STDOUT => {
                print!("{}", log_line);
                method.lines += 1;
                if method.flush > 0 && method.lines % method.flush == 0 {
                    let _ = std::io::stdout().flush();
                }
                StdString::from_vec(Vec::new())
            }
            FileMethodTarget::STDERR => {
                eprint!("{}", log_line);
                method.lines += 1;
                if method.flush > 0 && method.lines % method.flush == 0 {
                    let _ = std::io::stderr().flush();
                }
                StdString::from_vec(Vec::new())
            }
        };

        result
    } else {
        StdString::from_vec("Handle not found".to_string().into_bytes())
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn rust_log_filemethod_tick(handle: *mut std::os::raw::c_void) {
    if handle.is_null() {
        return;
    }

    let id = unsafe { *(handle as *mut usize) };
    let mut handles = FILE_HANDLES.lock().unwrap();
    
    if let Some(method) = handles.get_mut(&id) {
        match method.target {
            FileMethodTarget::FILE => {
                if let Some(ref mut file) = method.file {
                    let _ = file.flush();
                }
            }
            FileMethodTarget::STDOUT => {
                let _ = std::io::stdout().flush();
            }
            FileMethodTarget::STDERR => {
                let _ = std::io::stderr().flush();
            }
        }
    }
}

