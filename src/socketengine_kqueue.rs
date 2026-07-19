// This file is a Rust implementation of the kqueue socket engine backend.
// Original work Copyright (C) the InspIRCd contributors.
// Licensed under GPLv2. See LICENSE for details.

#![cfg(any(target_os = "freebsd", target_os = "netbsd", target_os = "macos", target_os = "openbsd"))]

use std::ffi::c_int;
use std::os::raw::c_void;
use std::sync::Mutex;
use lazy_static::lazy_static;

#[cfg(not(target_os = "windows"))]
use libc::{kqueue, kevent, close, EVFILT_READ, EVFILT_WRITE, EV_ADD, EV_DELETE, EV_MOD, EV_ONESHOT, EV_EOF, timespec};

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

/// Maximum number of events to return from kevent
const MAX_EVENTS: usize = 128;

/// Maximum number of pending changes
const MAX_CHANGES: usize = 64;

/// Kqueue engine state managed by Rust
struct KqueueEngine {
    engine_handle: c_int,
    changelist: Vec<libc::kevent>,
    eventlist: Vec<libc::kevent>,
    change_pos: usize,
}

impl KqueueEngine {
    pub fn new() -> Option<Self> {
        let handle = unsafe { kqueue() };
        if handle == -1 {
            return None;
        }

        Some(KqueueEngine {
            engine_handle: handle,
            changelist: Vec::with_capacity(MAX_CHANGES),
            eventlist: Vec::with_capacity(MAX_EVENTS),
            change_pos: 0,
        })
    }

    pub fn deinit(&mut self) {
        if self.engine_handle != -1 {
            unsafe { close(self.engine_handle) };
            self.engine_handle = -1;
        }
    }

    /// Get a reference to the changelist buffer for adding a change
    fn get_change_ke(&mut self) -> *mut libc::kevent {
        if self.change_pos >= self.changelist.capacity() {
            self.changelist.reserve(self.changelist.capacity() * 2);
        }
        // Ensure we have enough space
        while self.change_pos >= self.changelist.len() {
            self.changelist.push(unsafe { std::mem::MaybeUninit::uninit().assume_init() });
        }
        let ptr = &mut self.changelist[self.change_pos] as *mut libc::kevent;
        self.change_pos += 1;
        ptr
    }

    /// Add a file descriptor to kqueue
    pub fn add_fd(&mut self, fd: c_int, event_mask: EventMask, eh_ptr: *mut c_void) -> bool {
        // Always add read filter first
        let ke = self.get_change_ke();
        
        // Build the kevent for read
        unsafe {
            libc::EV_SET(
                ke,
                fd as u32,
                EVFILT_READ,
                EV_ADD,
                0,
                0,
                eh_ptr,
            );
        }

        // We need to also add write filter if requested
        if event_mask & (FD_WANT_POLL_WRITE | FD_WANT_FAST_WRITE | FD_WANT_SINGLE_WRITE) != 0 {
            let ke = self.get_change_ke();
            let flags = if event_mask & (FD_WANT_FAST_WRITE | FD_WANT_SINGLE_WRITE) != 0 {
                EV_ADD | EV_ONESHOT
            } else {
                EV_ADD
            };
            
            unsafe {
                libc::EV_SET(
                    ke,
                    fd as u32,
                    EVFILT_WRITE,
                    flags,
                    0,
                    0,
                    eh_ptr,
                );
            }
        }

        true
    }

    /// Modify an existing file descriptor's event mask
    pub fn mod_fd(&mut self, fd: c_int, old_mask: EventMask, new_mask: EventMask, eh_ptr: *mut c_void) -> bool {
        let diff = old_mask ^ new_mask;

        // Handle poll-style write changes
        if (new_mask & FD_WANT_POLL_WRITE) != 0 && (old_mask & FD_WANT_POLL_WRITE) == 0 {
            // New poll-style write: add write filter
            let ke = self.get_change_ke();
            unsafe {
                libc::EV_SET(
                    ke,
                    fd as u32,
                    EVFILT_WRITE,
                    EV_ADD,
                    0,
                    0,
                    eh_ptr,
                );
            }
        } else if (old_mask & FD_WANT_POLL_WRITE) != 0 && (new_mask & FD_WANT_POLL_WRITE) == 0 {
            // Removing poll-style write: delete write filter
            let ke = self.get_change_ke();
            unsafe {
                libc::EV_SET(
                    ke,
                    fd as u32,
                    EVFILT_WRITE,
                    EV_DELETE,
                    0,
                    0,
                    std::ptr::null_mut(),
                );
            }
        }

        // Handle fast/single write changes
        if (new_mask & (FD_WANT_FAST_WRITE | FD_WANT_SINGLE_WRITE)) != 0 
            && (old_mask & (FD_WANT_FAST_WRITE | FD_WANT_SINGLE_WRITE)) == 0 {
            // New one-shot write: add write filter with EV_ONESHOT
            let ke = self.get_change_ke();
            unsafe {
                libc::EV_SET(
                    ke,
                    fd as u32,
                    EVFILT_WRITE,
                    EV_ADD | EV_ONESHOT,
                    0,
                    0,
                    eh_ptr,
                );
            }
        }

        true
    }

    /// Remove a file descriptor from kqueue
    pub fn del_fd(&mut self, fd: c_int, _eh_ptr: *mut c_void) -> bool {
        // First remove write filter
        let ke = self.get_change_ke();
        unsafe {
            libc::EV_SET(
                ke,
                fd as u32,
                EVFILT_WRITE,
                EV_DELETE,
                0,
                0,
                std::ptr::null_mut(),
            );
        }

        // Then remove read filter
        let ke = self.get_change_ke();
        unsafe {
            libc::EV_SET(
                ke,
                fd as u32,
                EVFILT_READ,
                EV_DELETE,
                0,
                0,
                std::ptr::null_mut(),
            );
        }

        true
    }

    /// Process pending changes and wait for events
    /// Returns the number of events, or -1 on error
    pub fn dispatch_events(&mut self) -> c_int {
        // Resize eventlist if needed
        while self.eventlist.len() < MAX_EVENTS {
            self.eventlist.push(unsafe { std::mem::MaybeUninit::uninit().assume_init() });
        }

        let mut ts = timespec {
            tv_sec: 1,
            tv_nsec: 0,
        };

        // Process changes and wait for events
        let result = unsafe { kevent(
            self.engine_handle,
            self.changelist.as_ptr() as *const libc::kevent,
            self.change_pos as c_int,
            self.eventlist.as_mut_ptr(),
            MAX_EVENTS as c_int,
            &mut ts,
        ) };

        // Reset change position
        self.change_pos = 0;

        if result < 0 {
            return result;
        }

        result
    }

    /// Get the event at a specific index
    pub fn get_event(&self, index: usize) -> Option<libc::kevent> {
        self.eventlist.get(index).cloned()
    }
}

/// Global kqueue engine state
lazy_static! {
    static ref KQUEUE_ENGINE: Mutex<KqueueEngine> = Mutex::new(KqueueEngine::new().unwrap());
}

/// Initialize the kqueue socket engine
#[unsafe(no_mangle)]
pub unsafe extern "C" fn rust_socketengine_kqueue_init() -> bool {
    let mut guard = KQUEUE_ENGINE.lock().unwrap();
    *guard = KqueueEngine::new().unwrap_or_else(|| KqueueEngine {
        engine_handle: -1,
        changelist: Vec::new(),
        eventlist: Vec::new(),
        change_pos: 0,
    });
    guard.engine_handle != -1
}

/// Deinitialize the kqueue socket engine
#[unsafe(no_mangle)]
pub unsafe extern "C" fn rust_socketengine_kqueue_deinit() {
    let mut guard = KQUEUE_ENGINE.lock().unwrap();
    guard.deinit();
}

/// Recover from fork (reinitialize kqueue handle)
#[unsafe(no_mangle)]
pub unsafe extern "C" fn rust_socketengine_kqueue_recover_from_fork() -> bool {
    // kqueue doesn't survive fork, need to reinitialize
    rust_socketengine_kqueue_deinit();
    rust_socketengine_kqueue_init()
}

/// Add a file descriptor to the kqueue engine
#[unsafe(no_mangle)]
pub unsafe extern "C" fn rust_socketengine_kqueue_add_fd(
    fd: c_int,
    event_mask: EventMask,
    eh_ptr: *mut c_void,
) -> bool {
    KQUEUE_ENGINE.lock().unwrap().add_fd(fd, event_mask, eh_ptr)
}

/// Modify an existing file descriptor's event mask
#[unsafe(no_mangle)]
pub unsafe extern "C" fn rust_socketengine_kqueue_mod_fd(
    fd: c_int,
    old_mask: EventMask,
    new_mask: EventMask,
    eh_ptr: *mut c_void,
) -> bool {
    KQUEUE_ENGINE.lock().unwrap().mod_fd(fd, old_mask, new_mask, eh_ptr)
}

/// Remove a file descriptor from the kqueue engine
#[unsafe(no_mangle)]
pub unsafe extern "C" fn rust_socketengine_kqueue_del_fd(
    fd: c_int,
    eh_ptr: *mut c_void,
) -> bool {
    KQUEUE_ENGINE.lock().unwrap().del_fd(fd, eh_ptr)
}

/// Dispatch events (process changes and wait)
/// Returns the number of events, or -1 on error
#[unsafe(no_mangle)]
pub unsafe extern "C" fn rust_socketengine_kqueue_dispatch_events() -> c_int {
    KQUEUE_ENGINE.lock().unwrap().dispatch_events()
}

/// Get an event from the event list
/// Returns 0 on success, -1 on error
#[unsafe(no_mangle)]
pub unsafe extern "C" fn rust_socketengine_kqueue_get_event(
    index: usize,
   kev_out: *mut libc::kevent,
) -> c_int {
    let guard = KQUEUE_ENGINE.lock().unwrap();
    if let Some(kev) = guard.get_event(index) {
        unsafe { *kev_out = kev; }
        0
    } else {
        -1
    }
}
