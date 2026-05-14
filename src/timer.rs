/*
 * InspIRCd -- Internet Relay Chat Daemon
 *
 *   Copyright (C) 2021-2023 Sadie Powell <sadie@witchery.services>
 *   Copyright (C) 2021 Dominic Hamon
 *   Copyright (C) 2013-2014 Attila Molnar <attilamolnar@hush.com>
 *   Copyright (C) 2012 Robby <robby@chatbelgie.be>
 *   Copyright (C) 2009 Uli Schlachter <psychon@znc.in>
 *   Copyright (C) 2009 Daniel De Graaf <danieldg@inspircd.org>
 *   Copyright (C) 2008 Robin Burchell <robin+git@viroteck.net>
 *   Copyright (C) 2007 Dennis Friis <peavey@inspircd.org>
 *   Copyright (C) 2007 Craig Edwards <brain@inspircd.org>
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

use std::collections::BTreeMap;
use std::ffi::c_void;

// Type definitions for C++ compatibility
type time_t = i64;

/// Rust Timer struct that mirrors the C++ Timer class
#[repr(C)]
pub struct Timer {
    /// The triggering time
    pub trigger: time_t,
    
    /// Number of seconds between triggers
    pub secs: u64,
    
    /// True if this is a repeating timer
    pub repeat: bool,
    
    /// Opaque pointer to C++ timer object for virtual method calls
    pub cpp_timer: *mut c_void,
}

impl Timer {
    /// Creates a new Timer instance
    pub fn new(secs_from_now: u64, repeating: bool, cpp_timer: *mut c_void) -> Self {
        Timer {
            trigger: 0,  // Will be set by AddTimer like in original
            secs: secs_from_now,
            repeat: repeating,
            cpp_timer,
        }
    }
    
    /// Gets the trigger time
    pub fn get_trigger(&self) -> time_t {
        self.trigger
    }
    
    /// Sets the trigger time
    pub fn set_trigger(&mut self, nexttrigger: time_t) {
        self.trigger = nexttrigger;
    }
    
    /// Gets the interval
    pub fn get_interval(&self) -> u64 {
        self.secs
    }
    
    /// Sets the interval and optionally restarts the timer
    pub fn set_interval(&mut self, newinterval: u64, restart: bool) {
        self.secs = newinterval;
        if restart {
            unsafe {
                timer_ffi_del_timer(self.cpp_timer);
                self.trigger = timer_ffi_server_time() + newinterval as i64;
                timer_ffi_add_timer(self.cpp_timer);
            }
        }
    }
    
    /// Gets whether this timer repeats
    pub fn get_repeat(&self) -> bool {
        self.repeat
    }
    
    /// Cancels repeat state
    pub fn cancel_repeat(&mut self) {
        self.repeat = false;
    }
}

/// TimerManager for managing collections of timers
pub struct TimerManager {
    /// Map of trigger time to timer pointers
    timers: BTreeMap<time_t, *mut Timer>,
}

impl TimerManager {
    /// Creates a new TimerManager
    pub fn new() -> Self {
        TimerManager {
            timers: std::collections::BTreeMap::new(),
        }
    }
    
    /// Ticks all pending timers
    pub fn tick_timers(&mut self) {
        let now = unsafe { timer_ffi_server_time() };
        
        // Collect timers to process first (to avoid iterator invalidation)
        let mut timers_to_process = Vec::new();
        for (&trigger_time, &timer_ptr) in self.timers.iter() {
            if trigger_time > now {
                break;
            }
            timers_to_process.push((trigger_time, timer_ptr));
        }
        
        // Process timers
        for (trigger_time, timer_ptr) in timers_to_process {
            // Remove from map first (like original's erase(i++))
            self.timers.remove(&trigger_time);
            
            let timer = unsafe { &mut *timer_ptr };
            
            // Call the C++ Tick method
            let should_continue = unsafe { timer_ffi_timer_tick(timer.cpp_timer) };
            
            if !should_continue {
                continue;
            }
            
            if timer.repeat {
                timer.trigger = now + timer.secs as i64;
                // Add back for next cycle (like original's AddTimer)
                self.timers.insert(timer.trigger, timer_ptr);
            }
        }
    }
    
    /// Adds a timer to the manager
    pub fn add_timer(&mut self, timer: *mut Timer) {
        let trigger = unsafe { timer_ffi_server_time() } + unsafe { (*timer).secs as i64 };
        unsafe { (*timer).trigger = trigger };
        self.timers.insert(trigger, timer);
    }
    
    /// Removes a timer from the manager
    pub fn del_timer(&mut self, timer: *mut c_void) {
        let rust_timer = unsafe { timer_ffi_get_rust_timer(timer) as *mut Timer };
        if rust_timer.is_null() {
            return;
        }
        
        let trigger_time = unsafe { (*rust_timer).get_trigger() };
        if trigger_time == 0 {
            return;
        }
        
        // Find all timers with this trigger time (equal_range equivalent)
        let matching_timers: Vec<(time_t, *mut Timer)> = self.timers
            .range(trigger_time..=trigger_time)
            .map(|(&time, &timer)| (time, timer))
            .collect();
        
        for (time, timer_ptr) in matching_timers {
            if std::ptr::eq(rust_timer, timer_ptr) {
                unsafe { (*timer_ptr).trigger = 0 };
                self.timers.remove(&time);
                break;
            }
        }
    }
}

/// Global timer manager instance
static mut GLOBAL_TIMER_MANAGER: TimerManager = TimerManager {
    timers: std::collections::BTreeMap::new(),
};
static mut TIMER_MANAGER_INIT: bool = false;

/// Gets the global timer manager instance
pub fn get_timer_manager() -> *mut TimerManager {
    unsafe {
        if !TIMER_MANAGER_INIT {
            GLOBAL_TIMER_MANAGER = TimerManager::new();
            TIMER_MANAGER_INIT = true;
        }
        &raw mut GLOBAL_TIMER_MANAGER
    }
}

// FFI function declarations
unsafe extern "C" {
    fn timer_ffi_server_time() -> time_t;
    fn timer_ffi_add_timer(timer: *mut c_void);
    fn timer_ffi_del_timer(timer: *mut c_void);
    fn timer_ffi_timer_tick(timer: *mut c_void) -> bool;
    fn timer_ffi_get_rust_timer(timer: *mut c_void) -> *mut c_void;
}


// Extern "C" functions for C++ to call
#[unsafe(no_mangle)]
pub extern "C" fn timer_rust_create_timer(secs_from_now: u64, repeating: bool, cpp_timer: *mut c_void) -> *mut Timer {
    let timer = Timer::new(secs_from_now, repeating, cpp_timer);
    Box::into_raw(Box::new(timer))
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn timer_rust_destroy_timer(timer: *mut Timer) {
    if !timer.is_null() {
        let _ = unsafe { Box::from_raw(timer) };
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn timer_rust_get_trigger(timer: *const Timer) -> time_t {
    unsafe { (*timer).get_trigger() }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn timer_rust_set_trigger(timer: *mut Timer, nexttrigger: time_t) {
    unsafe { (*timer).set_trigger(nexttrigger) };
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn timer_rust_get_interval(timer: *const Timer) -> u64 {
    unsafe { (*timer).get_interval() }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn timer_rust_set_interval(timer: *mut Timer, newinterval: u64, restart: bool) {
    unsafe { (*timer).set_interval(newinterval, restart) };
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn timer_rust_get_repeat(timer: *const Timer) -> bool {
    unsafe { (*timer).get_repeat() }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn timer_rust_cancel_repeat(timer: *mut Timer) {
    unsafe { (*timer).cancel_repeat() };
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn timer_rust_tick_timers() {
    let manager_ptr = get_timer_manager();
    if !manager_ptr.is_null() {
        unsafe { (*manager_ptr).tick_timers() };
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn timer_rust_add_timer(timer: *mut Timer) {
    let manager_ptr = get_timer_manager();
    if !manager_ptr.is_null() {
        unsafe { (*manager_ptr).add_timer(timer) };
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn timer_rust_del_timer(cpp_timer: *mut c_void) {
    let manager_ptr = get_timer_manager();
    if !manager_ptr.is_null() {
        unsafe { (*manager_ptr).del_timer(cpp_timer) };
    }
}
