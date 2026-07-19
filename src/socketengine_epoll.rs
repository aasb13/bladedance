// This file is a Rust implementation of the epoll socket engine backend.
// Original work Copyright (C) the InspIRCd contributors.
// Licensed under GPLv2. See LICENSE for details.

use std::ffi::c_int;
use std::os::raw::c_void;
use std::sync::Mutex;

#[cfg(not(target_os = "windows"))]
use libc::{epoll_create, epoll_wait, epoll_ctl, close, EPOLLIN, EPOLLOUT, EPOLLET, EPOLL_CTL_ADD, EPOLL_CTL_MOD, EPOLL_CTL_DEL, epoll_event};

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

/// Global epoll engine state
static EPOLL_ENGINE: Mutex<Option<EpollEngine>> = Mutex::new(None);

// Maximum number of events to return from epoll_wait
const MAX_EVENTS: usize = 128;

#[repr(C)]
pub struct EpollEngine {
    engine_handle: c_int,
    max_events: usize,
}

impl EpollEngine {
    pub fn new() -> Option<Self> {
        let handle = unsafe { epoll_create(128) };
        if handle == -1 {
            return None;
        }
        Some(EpollEngine {
            engine_handle: handle,
            max_events: 16,
        })
    }

    pub fn deinit(&mut self) {
        if self.engine_handle != -1 {
            unsafe { close(self.engine_handle) };
            self.engine_handle = -1;
        }
    }

    /// Convert event mask to epoll events
    fn mask_to_epoll(&self, event_mask: EventMask) -> u32 {
        let mut rv = 0u32;
        
        if event_mask & (FD_WANT_POLL_READ | FD_WANT_POLL_WRITE | FD_WANT_SINGLE_WRITE) != 0 {
            // Standard polling
            if event_mask & (FD_WANT_POLL_READ | FD_WANT_FAST_READ) != 0 {
                rv |= EPOLLIN as u32;
            }
            if event_mask & (FD_WANT_POLL_WRITE | FD_WANT_FAST_WRITE | FD_WANT_SINGLE_WRITE) != 0 {
                rv |= EPOLLOUT as u32;
            }
        } else {
            // Edge-triggered polling
            rv |= EPOLLET as u32;
            if event_mask & (FD_WANT_FAST_READ | FD_WANT_EDGE_READ) != 0 {
                rv |= EPOLLIN as u32;
            }
            if event_mask & (FD_WANT_FAST_WRITE | FD_WANT_EDGE_WRITE) != 0 {
                rv |= EPOLLOUT as u32;
            }
        }
        rv
    }

    pub fn add_fd(&self, fd: c_int, event_mask: EventMask, eh_ptr: *mut c_void) -> bool {
        let epoll_events = self.mask_to_epoll(event_mask);
        
        // Create epoll_event with the EventHandler pointer
        // The epoll_event structure has a union for data, we use the ptr field
        let mut ev: epoll_event = unsafe { std::mem::MaybeUninit::uninit().assume_init() };
        ev.events = epoll_events;
        ev.u64 = eh_ptr as u64;  // Store pointer as u64
        
        // SAFETY: This is safe because we're passing a properly initialized epoll_event
        let result = unsafe { epoll_ctl(self.engine_handle, EPOLL_CTL_ADD, fd, &mut ev) };
        result == 0
    }

    pub fn mod_fd(&self, fd: c_int, event_mask: EventMask, eh_ptr: *mut c_void) -> bool {
        let epoll_events = self.mask_to_epoll(event_mask);
        
        let mut ev: epoll_event = unsafe { std::mem::MaybeUninit::uninit().assume_init() };
        ev.events = epoll_events;
        ev.u64 = eh_ptr as u64;  // Store pointer as u64
        
        let result = unsafe { epoll_ctl(self.engine_handle, EPOLL_CTL_MOD, fd, &mut ev) };
        result == 0
    }

    pub fn del_fd(&self, fd: c_int) -> bool {
        // For EPOLL_CTL_DEL, the event parameter is ignored
        let mut ev: epoll_event = unsafe { std::mem::MaybeUninit::uninit().assume_init() };
        let result = unsafe { epoll_ctl(self.engine_handle, EPOLL_CTL_DEL, fd, &mut ev) };
        result == 0
    }
}

/// Initialize the epoll socket engine
#[unsafe(no_mangle)]
pub unsafe extern "C" fn rust_socketengine_epoll_init() -> bool {
    match EpollEngine::new() {
        Some(engine) => {
            let mut guard = EPOLL_ENGINE.lock().unwrap();
            *guard = Some(engine);
            true
        }
        None => false,
    }
}

/// Deinitialize the epoll socket engine
#[unsafe(no_mangle)]
pub unsafe extern "C" fn rust_socketengine_epoll_deinit() {
    let mut guard = EPOLL_ENGINE.lock().unwrap();
    if let Some(engine) = guard.as_mut() {
        engine.deinit();
    }
    *guard = None;
}

/// Recover from fork (reinitialize epoll handle)
#[unsafe(no_mangle)]
pub unsafe extern "C" fn rust_socketengine_epoll_recover_from_fork() -> bool {
    // Close existing handle if any
    unsafe { rust_socketengine_epoll_deinit() };
    unsafe { rust_socketengine_epoll_init() }
}

/// Add a file descriptor to the epoll engine
#[unsafe(no_mangle)]
pub unsafe extern "C" fn rust_socketengine_epoll_add_fd(
    fd: c_int,
    event_mask: EventMask,
    eh_ptr: *mut c_void,
) -> bool {
    let guard = EPOLL_ENGINE.lock().unwrap();
    guard.as_ref().map_or(false, |engine| {
        engine.add_fd(fd, event_mask, eh_ptr)
    })
}

/// Modify an existing file descriptor's event mask
#[unsafe(no_mangle)]
pub unsafe extern "C" fn rust_socketengine_epoll_mod_fd(
    fd: c_int,
    event_mask: EventMask,
    eh_ptr: *mut c_void,
) -> bool {
    let guard = EPOLL_ENGINE.lock().unwrap();
    guard.as_ref().map_or(false, |engine| {
        engine.mod_fd(fd, event_mask, eh_ptr)
    })
}

/// Remove a file descriptor from the epoll engine
#[unsafe(no_mangle)]
pub unsafe extern "C" fn rust_socketengine_epoll_del_fd(fd: c_int) -> bool {
    let guard = EPOLL_ENGINE.lock().unwrap();
    guard.as_ref().map_or(false, |engine| {
        engine.del_fd(fd)
    })
}

/// Wait for events and return the number of events available
/// This function uses the Rust-managed epoll handle
#[unsafe(no_mangle)]
pub unsafe extern "C" fn rust_socketengine_epoll_wait(
    events_ptr: *mut libc::epoll_event,
    max_events: c_int,
    timeout_ms: c_int,
) -> c_int {
    let guard = EPOLL_ENGINE.lock().unwrap();
    guard.as_ref().map_or(0, |engine| {
        // Create a buffer for events
        let mut temp_events = vec![std::mem::MaybeUninit::<libc::epoll_event>::uninit(); MAX_EVENTS];
        
        let result = unsafe { epoll_wait(
            engine.engine_handle,
            temp_events.as_mut_ptr() as *mut libc::epoll_event,
            max_events.min(MAX_EVENTS as c_int),
            timeout_ms,
        ) };
        
        if result > 0 && !events_ptr.is_null() {
            // Copy the events to the provided buffer
            unsafe {
                std::ptr::copy_nonoverlapping(
                    temp_events.as_ptr() as *const libc::epoll_event,
                    events_ptr,
                    result as usize,
                );
            }
        }
        
        result
    })
}

/// Get the Rust-managed epoll file descriptor
#[unsafe(no_mangle)]
pub unsafe extern "C" fn rust_socketengine_epoll_get_handle() -> c_int {
    let guard = EPOLL_ENGINE.lock().unwrap();
    guard.as_ref().map_or(-1, |engine| engine.engine_handle)
}


