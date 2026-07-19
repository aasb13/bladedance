// This file is a Rust implementation of the poll socket engine backend.
// Original work Copyright (C) the InspIRCd contributors.
// Licensed under GPLv2. See LICENSE for details.

use std::ffi::c_int;
use std::os::raw::{c_void, c_short};
use std::sync::Mutex;
use lazy_static::lazy_static;

#[cfg(not(target_os = "windows"))]
use libc::{poll, POLLIN, POLLOUT, POLLERR, POLLHUP, pollfd};

// Event mask constants from socketengine.h - same as in socketengine_epoll.rs
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

/// Maximum number of file descriptors to poll
const MAX_POLL_FDS: usize = 1024;

/// Poll engine state managed by Rust
struct PollEngine {
    /// Array of pollfd structures
    events: Vec<pollfd>,
    /// Maps fd to index in events array
    fd_mappings: Vec<c_int>,
}

impl PollEngine {
    pub fn new() -> Self {
        PollEngine {
            events: Vec::with_capacity(MAX_POLL_FDS),
            fd_mappings: vec![-1; MAX_POLL_FDS],
        }
    }

    /// Convert event mask to poll() events
    fn mask_to_poll(event_mask: EventMask) -> c_int {
        let mut rv = 0;
        if event_mask & (FD_WANT_POLL_READ | FD_WANT_FAST_READ) != 0 {
            rv |= POLLIN as c_int;
        }
        if event_mask & (FD_WANT_POLL_WRITE | FD_WANT_FAST_WRITE | FD_WANT_SINGLE_WRITE) != 0 {
            rv |= POLLOUT as c_int;
        }
        rv
    }

    /// Add a file descriptor to the poll set
    pub fn add_fd(&mut self, fd: c_int, event_mask: EventMask, _eh_ptr: *mut c_void) -> bool {
        if fd < 0 || fd as usize >= self.fd_mappings.len() {
            return false;
        }

        // Check for duplicate
        if (fd as usize) < self.fd_mappings.len() && self.fd_mappings[fd as usize] != -1 {
            return false;
        }

        let index = self.events.len();

        // Resize mappings if needed
        if fd as usize >= self.fd_mappings.len() {
            self.fd_mappings.resize(fd as usize * 2 + 1, -1);
        }
        self.fd_mappings[fd as usize] = index as c_int;

        // Add to events
        self.events.push(pollfd {
            fd,
            events: Self::mask_to_poll(event_mask) as c_short,
            revents: 0,
        });

        true
    }

    /// Modify an existing file descriptor's event mask
    pub fn mod_fd(&mut self, fd: c_int, event_mask: EventMask, _eh_ptr: *mut c_void) -> bool {
        if fd < 0 || fd as usize >= self.fd_mappings.len() {
            return false;
        }

        let index = self.fd_mappings[fd as usize];
        if index == -1 {
            return false;
        }

        self.events[index as usize].events = Self::mask_to_poll(event_mask) as c_short;
        true
    }

    /// Remove a file descriptor from the poll set
    pub fn del_fd(&mut self, fd: c_int) -> bool {
        if fd < 0 || fd as usize >= self.fd_mappings.len() {
            return false;
        }

        let index = self.fd_mappings[fd as usize];
        if index == -1 {
            return false;
        }

        let last_index = self.events.len() - 1;

        if index as usize != last_index {
            // Move last element to fill the gap
            let last_fd = self.events[last_index].fd;
            self.events[index as usize] = self.events[last_index];
            self.fd_mappings[last_fd as usize] = index;
        }

        // Remove last element
        self.events.pop();
        self.fd_mappings[fd as usize] = -1;

        true
    }

    /// Wait for events using poll()
    /// Returns the number of events, or -1 on error
    pub fn wait(&mut self, timeout_ms: c_int) -> c_int {
        if self.events.is_empty() {
            // Use a small timeout to avoid blocking indefinitely
            return unsafe { poll(std::ptr::null_mut(), 0, timeout_ms) };
        }

        unsafe { poll(self.events.as_mut_ptr(), self.events.len() as u64, timeout_ms) }
    }

    /// Get the number of file descriptors currently being polled
    pub fn get_fd_count(&self) -> usize {
        self.events.len()
    }

    /// Get the pollfd at a specific index
    pub fn get_pollfd(&self, index: usize) -> Option<pollfd> {
        self.events.get(index).cloned()
    }
}

/// Global poll engine state
lazy_static! {
    static ref POLL_ENGINE: Mutex<PollEngine> = Mutex::new(PollEngine::new());
}

/// Initialize the poll socket engine
#[unsafe(no_mangle)]
pub unsafe extern "C" fn rust_socketengine_poll_init() -> bool {
    let mut guard = POLL_ENGINE.lock().unwrap();
    // Poll doesn't need special initialization, just reset state
    *guard = PollEngine::new();
    true
}

/// Deinitialize the poll socket engine
#[unsafe(no_mangle)]
pub unsafe extern "C" fn rust_socketengine_poll_deinit() {
    let mut guard = POLL_ENGINE.lock().unwrap();
    *guard = PollEngine::new();
}

/// Recover from fork (poll doesn't need special handling)
#[unsafe(no_mangle)]
pub unsafe extern "C" fn rust_socketengine_poll_recover_from_fork() -> bool {
    true
}

/// Add a file descriptor to the poll engine
#[unsafe(no_mangle)]
pub unsafe extern "C" fn rust_socketengine_poll_add_fd(
    fd: c_int,
    event_mask: EventMask,
    eh_ptr: *mut c_void,
) -> bool {
    POLL_ENGINE.lock().unwrap().add_fd(fd, event_mask, eh_ptr)
}

/// Modify an existing file descriptor's event mask
#[unsafe(no_mangle)]
pub unsafe extern "C" fn rust_socketengine_poll_mod_fd(
    fd: c_int,
    event_mask: EventMask,
    eh_ptr: *mut c_void,
) -> bool {
    POLL_ENGINE.lock().unwrap().mod_fd(fd, event_mask, eh_ptr)
}

/// Remove a file descriptor from the poll engine
#[unsafe(no_mangle)]
pub unsafe extern "C" fn rust_socketengine_poll_del_fd(fd: c_int) -> bool {
    POLL_ENGINE.lock().unwrap().del_fd(fd)
}

/// Wait for events using poll()
/// Fills the provided events array and returns the number of events, or -1 on error
#[unsafe(no_mangle)]
pub unsafe extern "C" fn rust_socketengine_poll_wait(
    events_ptr: *mut pollfd,
    max_events: c_int,
    timeout_ms: c_int,
) -> c_int {
    let mut guard = POLL_ENGINE.lock().unwrap();
    
    let result = guard.wait(timeout_ms);
    
    if result > 0 && !events_ptr.is_null() {
        // Copy events to the provided buffer
        let count = result.min(max_events) as usize;
        unsafe {
            std::ptr::copy_nonoverlapping(
                guard.events.as_ptr(),
                events_ptr,
                count,
            );
        }
    }
    
    result
}

/// Get the number of file descriptors currently being polled
#[unsafe(no_mangle)]
pub unsafe extern "C" fn rust_socketengine_poll_get_fd_count() -> c_int {
    POLL_ENGINE.lock().unwrap().get_fd_count() as c_int
}
