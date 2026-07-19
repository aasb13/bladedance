// This file is a Rust implementation of the select socket engine backend.
// Original work Copyright (C) the InspIRCd contributors.
// Licensed under GPLv2. See LICENSE for details.

use std::ffi::c_int;
use std::os::raw::c_void;
use std::sync::Mutex;
use lazy_static::lazy_static;

#[cfg(not(target_os = "windows"))]
use libc::{select, fd_set, FD_ZERO, FD_SET, FD_CLR, FD_ISSET, timeval, time_t, suseconds_t};

// Event mask constants from socketengine.h
type EventMask = c_int;

const FD_WANT_NO_READ: EventMask = 0x1;
const FD_WANT_POLL_READ: EventMask = 0x2;
const FD_WANT_FAST_READ: EventMask = 0x4;
const FD_WANT_EDGE_READ: EventMask = 0x8;
const FD_WANT_READ_MASK: EventMask = 0x0F;

const FD_WANT_NO_WRITE: EventMask = 0x10;
const FD_WANT_POLL_WRITE: EventMask = 0x20;
const FD_WANT_FAST_WRITE: EventMask = 0x40;
const FD_WANT_EDGE_WRITE: EventMask = 0x80;
const FD_WANT_SINGLE_WRITE: EventMask = 0x100;
const FD_WANT_WRITE_MASK: EventMask = 0x1F0;

const FD_ADD_TRIAL_READ: EventMask = 0x1000;
const FD_READ_WILL_BLOCK: EventMask = 0x2000;
const FD_ADD_TRIAL_WRITE: EventMask = 0x4000;
const FD_WRITE_WILL_BLOCK: EventMask = 0x8000;

/// Maximum file descriptor for select
const FD_SETSIZE: usize = 1024;

/// Select engine state managed by Rust
struct SelectEngine {
    read_set: fd_set,
    write_set: fd_set,
    error_set: fd_set,
    max_fd: c_int,
}

impl SelectEngine {
    pub fn new() -> Self {
        // In Rust, we can't directly create fd_set, so we'll use libc's FD_ZERO
        // For now, we'll manage this through FFI
        SelectEngine {
            read_set: unsafe { std::mem::MaybeUninit::uninit().assume_init() },
            write_set: unsafe { std::mem::MaybeUninit::uninit().assume_init() },
            error_set: unsafe { std::mem::MaybeUninit::uninit().assume_init() },
            max_fd: 0,
        }
    }

    /// Initialize the fd_sets
    pub fn init(&mut self) {
        unsafe {
            FD_ZERO(&mut self.read_set);
            FD_ZERO(&mut self.write_set);
            FD_ZERO(&mut self.error_set);
        }
        self.max_fd = 0;
    }

    /// Add a file descriptor to the select sets
    pub fn add_fd(&mut self, fd: c_int, event_mask: EventMask, _eh_ptr: *mut c_void) -> bool {
        if fd < 0 || fd >= FD_SETSIZE as c_int {
            return false;
        }

        unsafe {
            // Always add to error set
            FD_SET(fd, &mut self.error_set);

            if event_mask & (FD_WANT_POLL_READ | FD_WANT_FAST_READ) != 0 {
                FD_SET(fd, &mut self.read_set);
            }
            if event_mask & (FD_WANT_POLL_WRITE | FD_WANT_FAST_WRITE | FD_WANT_SINGLE_WRITE) != 0 {
                FD_SET(fd, &mut self.write_set);
            }
        }

        if fd > self.max_fd {
            self.max_fd = fd;
        }

        true
    }

    /// Modify an existing file descriptor's event mask
    pub fn mod_fd(&mut self, fd: c_int, old_mask: EventMask, new_mask: EventMask, _eh_ptr: *mut c_void) -> bool {
        if fd < 0 || fd >= FD_SETSIZE as c_int {
            return false;
        }

        let diff = old_mask ^ new_mask;

        unsafe {
            if diff & (FD_WANT_POLL_READ | FD_WANT_FAST_READ) != 0 {
                if new_mask & (FD_WANT_POLL_READ | FD_WANT_FAST_READ) != 0 {
                    FD_SET(fd, &mut self.read_set);
                } else {
                    FD_CLR(fd, &mut self.read_set);
                }
            }
            if diff & (FD_WANT_POLL_WRITE | FD_WANT_FAST_WRITE | FD_WANT_SINGLE_WRITE) != 0 {
                if new_mask & (FD_WANT_POLL_WRITE | FD_WANT_FAST_WRITE | FD_WANT_SINGLE_WRITE) != 0 {
                    FD_SET(fd, &mut self.write_set);
                } else {
                    FD_CLR(fd, &mut self.write_set);
                }
            }
        }

        true
    }

    /// Remove a file descriptor from the select sets
    pub fn del_fd(&mut self, fd: c_int) -> bool {
        if fd < 0 || fd >= FD_SETSIZE as c_int {
            return false;
        }

        unsafe {
            FD_CLR(fd, &mut self.read_set);
            FD_CLR(fd, &mut self.write_set);
            FD_CLR(fd, &mut self.error_set);
        }

        if fd == self.max_fd {
            self.max_fd -= 1;
        }

        true
    }

    /// Wait for events using select()
    /// Returns the number of file descriptors with events, or -1 on error
    pub fn wait(&self, timeout_sec: c_int, timeout_usec: c_int) -> c_int {
        let mut rfdset = self.read_set;
        let mut wfdset = self.write_set;
        let mut errfdset = self.error_set;

        let mut tval = timeval {
            tv_sec: timeout_sec as time_t,
            tv_usec: timeout_usec as suseconds_t,
        };

        unsafe { select(self.max_fd + 1, &mut rfdset, &mut wfdset, &mut errfdset, &mut tval) }
    }

    /// Check if a file descriptor has events
    pub fn has_events(&self, fd: c_int, rfdset: &fd_set, wfdset: &fd_set, errfdset: &fd_set) -> (bool, bool, bool) {
        unsafe {
            (
                FD_ISSET(fd, rfdset) as c_int != 0,
                FD_ISSET(fd, wfdset) as c_int != 0,
                FD_ISSET(fd, errfdset) as c_int != 0,
            )
        }
    }

    /// Get current max_fd
    pub fn get_max_fd(&self) -> c_int {
        self.max_fd
    }

    /// Get references to the fd_sets for iteration
    pub fn get_sets(&self) -> (&fd_set, &fd_set, &fd_set) {
        (&self.read_set, &self.write_set, &self.error_set)
    }
}

/// Global select engine state
lazy_static! {
    static ref SELECT_ENGINE: Mutex<SelectEngine> = Mutex::new(SelectEngine::new());
}

/// Initialize the select socket engine
#[unsafe(no_mangle)]
pub unsafe extern "C" fn rust_socketengine_select_init() -> bool {
    let mut guard = SELECT_ENGINE.lock().unwrap();
    guard.init();
    true
}

/// Deinitialize the select socket engine
#[unsafe(no_mangle)]
pub unsafe extern "C" fn rust_socketengine_select_deinit() {
    let mut guard = SELECT_ENGINE.lock().unwrap();
    guard.init(); // Just reset
}

/// Recover from fork (select doesn't need special handling)
#[unsafe(no_mangle)]
pub unsafe extern "C" fn rust_socketengine_select_recover_from_fork() -> bool {
    true
}

/// Add a file descriptor to the select engine
#[unsafe(no_mangle)]
pub unsafe extern "C" fn rust_socketengine_select_add_fd(
    fd: c_int,
    event_mask: EventMask,
    eh_ptr: *mut c_void,
) -> bool {
    SELECT_ENGINE.lock().unwrap().add_fd(fd, event_mask, eh_ptr)
}

/// Modify an existing file descriptor's event mask
#[unsafe(no_mangle)]
pub unsafe extern "C" fn rust_socketengine_select_mod_fd(
    fd: c_int,
    old_mask: EventMask,
    new_mask: EventMask,
    eh_ptr: *mut c_void,
) -> bool {
    SELECT_ENGINE.lock().unwrap().mod_fd(fd, old_mask, new_mask, eh_ptr)
}

/// Remove a file descriptor from the select engine
#[unsafe(no_mangle)]
pub unsafe extern "C" fn rust_socketengine_select_del_fd(fd: c_int) -> bool {
    SELECT_ENGINE.lock().unwrap().del_fd(fd)
}

/// Wait for events using select()
/// Returns the number of file descriptors with events, or -1 on error
#[unsafe(no_mangle)]
pub unsafe extern "C" fn rust_socketengine_select_wait(
    timeout_sec: c_int,
    timeout_usec: c_int,
) -> c_int {
    SELECT_ENGINE.lock().unwrap().wait(timeout_sec, timeout_usec)
}

/// Get the maximum file descriptor currently being tracked
#[unsafe(no_mangle)]
pub unsafe extern "C" fn rust_socketengine_select_get_max_fd() -> c_int {
    SELECT_ENGINE.lock().unwrap().get_max_fd()
}

/// Check if a file descriptor has read, write, or error events
/// Returns a bitmask: bit 0 = read, bit 1 = write, bit 2 = error
#[unsafe(no_mangle)]
pub unsafe extern "C" fn rust_socketengine_select_has_events(
    fd: c_int,
    rfdset: *const fd_set,
    wfdset: *const fd_set,
    errfdset: *const fd_set,
) -> c_int {
    if fd < 0 || fd >= FD_SETSIZE as c_int {
        return 0;
    }

    let mut result = 0;
    
    unsafe {
        if FD_ISSET(fd, rfdset) as c_int != 0 {
            result |= 1;
        }
        if FD_ISSET(fd, wfdset) as c_int != 0 {
            result |= 2;
        }
        if FD_ISSET(fd, errfdset) as c_int != 0 {
            result |= 4;
        }
    }
    
    result
}
