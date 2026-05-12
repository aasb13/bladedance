/*
 * InspIRCd -- Internet Relay Chat Daemon
 *
 *   Copyright (C) 2022-2025 Sadie Powell <sadie@witchery.services>
 */

#![allow(unsafe_op_in_unsafe_fn)]

use std::ffi::c_char;
use std::ffi::c_void;
use std::io::Write;
use crate::stringutils::StdString;

enum FileMethodKind {
    File(std::fs::File),
    Stdout,
    Stderr,
}

struct FileMethod {
    kind: FileMethodKind,
    flush: u64,
    lines: u64,
    last_time: i64,
    last_time_str: Vec<u8>,
    name: Vec<u8>,
}

impl FileMethod {
    fn new(target: &[u8], flush: u64, kind: u8) -> std::io::Result<Self> {
        let name = target.to_vec();
        let kind = match kind {
            1 => FileMethodKind::Stdout,
            2 => FileMethodKind::Stderr,
            _ => {
                // File path
                let path = std::path::Path::new(std::str::from_utf8(target).unwrap_or_default());
                let file = std::fs::OpenOptions::new().create(true).append(true).open(path)?;
                FileMethodKind::File(file)
            }
        };

        Ok(FileMethod {
            kind,
            flush,
            lines: 0,
            last_time: -1,
            last_time_str: Vec::new(),
            name,
        })
    }

    fn tick(&mut self) {
        match &mut self.kind {
            FileMethodKind::File(f) => {
                let _ = f.flush();
            }
            FileMethodKind::Stdout => {
                let _ = std::io::stdout().flush();
            }
            FileMethodKind::Stderr => {
                let _ = std::io::stderr().flush();
            }
        }
    }

    fn on_log(&mut self, time: i64, type_s: &[u8], message: &[u8]) -> std::io::Result<()> {
        // Cache timestamp string per-second. We don't have access to the C++ Time::ToString
        // here; use a stable unix timestamp for now.
        if self.last_time != time {
            self.last_time = time;
            self.last_time_str.clear();
            self.last_time_str.extend_from_slice(time.to_string().as_bytes());
        }

        let mut line: Vec<u8> = Vec::with_capacity(
            self.last_time_str.len() + 1 + type_s.len() + 2 + message.len() + 1,
        );
        line.extend_from_slice(&self.last_time_str);
        line.push(b' ');
        line.extend_from_slice(type_s);
        line.extend_from_slice(b": ");
        line.extend_from_slice(message);
        line.push(b'\n');

        match &mut self.kind {
            FileMethodKind::File(f) => {
                f.write_all(&line)?;
                self.lines += 1;
                if self.flush > 0 && (self.lines % self.flush == 0) {
                    f.flush()?;
                }
            }
            FileMethodKind::Stdout => {
                let mut out = std::io::stdout().lock();
                out.write_all(&line)?;
                out.flush()?;
            }
            FileMethodKind::Stderr => {
                let mut out = std::io::stderr().lock();
                out.write_all(&line)?;
                out.flush()?;
            }
        }

        Ok(())
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn rust_log_filemethod_create(
    target: *const c_char,
    target_length: usize,
    flush: u64,
    kind: u8,
) -> *mut c_void {
    if target.is_null() {
        return std::ptr::null_mut();
    }
    let target_data = unsafe { std::slice::from_raw_parts(target as *const u8, target_length) };
    match FileMethod::new(target_data, flush, kind) {
        Ok(m) => Box::into_raw(Box::new(m)) as *mut c_void,
        Err(_) => std::ptr::null_mut(),
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn rust_log_filemethod_destroy(handle: *mut c_void) {
    if handle.is_null() {
        return;
    }
    unsafe {
        let _ = Box::from_raw(handle as *mut FileMethod);
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn rust_log_filemethod_tick(handle: *mut c_void) {
    if handle.is_null() {
        return;
    }
    let m = unsafe { &mut *(handle as *mut FileMethod) };
    m.tick();
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn rust_log_filemethod_on_log(
    handle: *mut c_void,
    time: i64,
    _level: u8,
    type_ptr: *const c_char,
    type_length: usize,
    message_ptr: *const c_char,
    message_length: usize,
) -> StdString {
    if handle.is_null() || type_ptr.is_null() || message_ptr.is_null() {
        return StdString::from_vec("invalid parameters".as_bytes().to_vec());
    }

    let type_data = unsafe { std::slice::from_raw_parts(type_ptr as *const u8, type_length) };
    let msg_data = unsafe { std::slice::from_raw_parts(message_ptr as *const u8, message_length) };

    let m = unsafe { &mut *(handle as *mut FileMethod) };
    match m.on_log(time, type_data, msg_data) {
        Ok(()) => StdString::from_vec(Vec::new()),
        Err(e) => StdString::from_vec(e.to_string().into_bytes()),
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn rust_log_level_to_string(level: u8) -> *const c_char {
    match level {
        0 => c"critical".as_ptr(),
        1 => c"warning".as_ptr(),
        2 => c"normal".as_ptr(),
        3 => c"debug".as_ptr(),
        4 => c"rawio".as_ptr(),
        _ => c"unknown".as_ptr(),
    }
}
