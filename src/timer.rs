// This file is a Rust port of a corresponding InspIRCd module.
// Original work Copyright (C) the InspIRCd contributors.
// Licensed under GPLv2. See LICENSE for details.

use std::collections::BTreeMap;
use std::ffi::c_void;
use std::ptr;
use std::cell::RefCell;

// Type definitions for C++ compatibility
type TimeT = i64;

thread_local! {
    // Deferred destruction list – only used on the main event loop thread.
    static DEFERRED_FREE_LIST: RefCell<Vec<*mut Timer>> = RefCell::new(Vec::new());
}

#[repr(C)]
pub struct Timer {
    pub trigger: TimeT,
    pub secs: u64,
    pub repeat: bool,
    pub cpp_timer: *mut c_void,
    pub cpp_timer_valid: bool,
    pub rust_callback: Option<unsafe extern "C" fn(*mut c_void)>,
    pub rust_callback_data: *mut c_void,
    // NEW: prevents freeing while this timer is in the processing batch
    pub being_processed: bool,
}

impl Timer {
    pub fn new(secs_from_now: u64, repeating: bool, cpp_timer: *mut c_void) -> Self {
        Timer {
            trigger: 0,
            secs: secs_from_now,
            repeat: repeating,
            cpp_timer,
            cpp_timer_valid: true,
            rust_callback: None,
            rust_callback_data: ptr::null_mut(),
            being_processed: false,
        }
    }

    pub fn new_with_rust_callback(
        secs_from_now: u64,
        repeating: bool,
        callback: unsafe extern "C" fn(*mut c_void),
        callback_data: *mut c_void,
    ) -> Self {
        Timer {
            trigger: 0,
            secs: secs_from_now,
            repeat: repeating,
            cpp_timer: ptr::null_mut(),
            cpp_timer_valid: false,
            rust_callback: Some(callback),
            rust_callback_data: callback_data,
            being_processed: false,
        }
    }
    pub fn get_trigger(&self) -> TimeT {
        self.trigger
    }
    pub fn set_trigger(&mut self, nexttrigger: TimeT) {
        self.trigger = nexttrigger;
    }
    pub fn get_interval(&self) -> u64 {
        self.secs
    }
    pub fn set_interval(&mut self, newinterval: u64, restart: bool) {
        self.secs = newinterval;
        if restart {
            unsafe {
                timer_rust_del_timer(self.cpp_timer);
                self.trigger = timer_ffi_server_time() + newinterval as i64;
                timer_rust_add_timer(self as *const _ as *mut _);
            }
        }
    }
    pub fn get_repeat(&self) -> bool {
        self.repeat
    }
    pub fn cancel_repeat(&mut self) {
        self.repeat = false;
    }
}

pub struct TimerManager {
    timers: BTreeMap<TimeT, *mut Timer>,
}

impl TimerManager {
    pub fn new() -> Self {
        TimerManager {
            timers: std::collections::BTreeMap::new(),
        }
    }
    pub fn tick_timers(&mut self) {
        let now = unsafe { timer_ffi_server_time() };

        // 1) Collect ALL due timers and immediately clear the map.
        //    This prevents any re-entrant map modification from causing issues.
        let due: Vec<(TimeT, *mut Timer)> = self
            .timers
            .range(..=now)
            .map(|(&t, &ptr)| (t, ptr))
            .collect();

        // Remove every collected entry from the map.
        for &(trigger, _) in &due {
            self.timers.remove(&trigger);
        }

        // 2) Mark every collected wrapper as being processed.
        for &(_, timer_ptr) in &due {
            unsafe {
                (*timer_ptr).being_processed = true;
            }
        }

        // 3) Process each timer in the batch.
        for (_, timer_ptr) in due.iter() {
            // Safety: pointer is valid because we prevent freeing via `being_processed`.
            let timer = unsafe { &mut **timer_ptr };

            if !timer.cpp_timer_valid || timer.cpp_timer.is_null() {
                continue; // already invalidated or was deleted by another timer's Tick()
            }

            let cpp_timer = (*timer).cpp_timer;
            let should_continue = unsafe { timer_ffi_timer_tick(cpp_timer) };

            // After Tick(), the wrapper may have been invalidated (deferred destroy).
            if !(*timer).cpp_timer_valid {
                continue;
            }

            if should_continue && (*timer).repeat {
                (*timer).trigger = now + (*timer).secs as i64;
                self.timers.insert((*timer).trigger, *timer_ptr);
            } else {
                // Timer should not continue – request destruction.
                // Since `being_processed` is true, this will only invalidate
                // and defer actual freeing.
                unsafe { timer_rust_destroy_timer(*timer_ptr) };
            }
        }

        // 4) Clear the processing flag on ALL collected wrappers.
        for &(_, timer_ptr) in &due {
            let timer = unsafe { &mut *timer_ptr };
            timer.being_processed = false;
        }

        // 5) Free all wrappers whose destruction was deferred.
        let deferred = DEFERRED_FREE_LIST.with(|list| list.replace(Vec::new()));
        for ptr in deferred {
            if !ptr.is_null() {
                // Safety: pointer was deferred because `being_processed` was true;
                // now it is false and the wrapper is no longer in the map.
                let _ = unsafe { Box::from_raw(ptr) };
            }
        }
    }
    pub fn add_timer(&mut self, timer: *mut Timer) {
        let trigger = unsafe { timer_ffi_server_time() } + unsafe { (*timer).secs as i64 };
        unsafe { (*timer).trigger = trigger };
        self.timers.insert(trigger, timer);
    }
    pub fn del_timer(&mut self, timer: *mut c_void) {
        let rust_timer = unsafe { timer_ffi_get_rust_timer(timer) as *mut Timer };
        if rust_timer.is_null() {
            return;
        }
        
        let trigger_time = unsafe { (*rust_timer).get_trigger() };
        if trigger_time == 0 {
            return;
        }
        let matching_timers: Vec<(TimeT, *mut Timer)> = self.timers
            .range(trigger_time..=trigger_time)
            .map(|(&time, &timer)| (time, timer))
            .collect();
        
        for (time, timer_ptr) in matching_timers {
            if std::ptr::eq(rust_timer, timer_ptr) {
                unsafe { (*timer_ptr).trigger = 0 };
                self.timers.remove(&time);
                // Clean up Rust wrapper to prevent memory leak
                unsafe { timer_rust_destroy_timer(timer_ptr) };
                break;
            }
        }
    }
}

static mut GLOBAL_TIMER_MANAGER: TimerManager = TimerManager {
    timers: std::collections::BTreeMap::new(),
};
static mut TIMER_MANAGER_INIT: bool = false;

pub fn get_timer_manager() -> *mut TimerManager {
    unsafe {
        if !TIMER_MANAGER_INIT {
            GLOBAL_TIMER_MANAGER = TimerManager::new();
            TIMER_MANAGER_INIT = true;
        }
        &raw mut GLOBAL_TIMER_MANAGER
    }
}

unsafe extern "C" {
    fn timer_ffi_server_time() -> TimeT;
    fn timer_ffi_timer_tick(timer: *mut c_void) -> bool;
    fn timer_ffi_get_rust_timer(timer: *mut c_void) -> *mut c_void;
}


#[unsafe(no_mangle)]
pub extern "C" fn timer_rust_create_timer(secs_from_now: u64, repeating: bool, cpp_timer: *mut c_void) -> *mut Timer {
    let timer = Timer::new(secs_from_now, repeating, cpp_timer);
    Box::into_raw(Box::new(timer))
}

#[unsafe(no_mangle)]
pub extern "C" fn timer_rust_create_timer_with_callback(
    secs_from_now: u64,
    repeating: bool,
    callback: unsafe extern "C" fn(*mut c_void),
    callback_data: *mut c_void,
) -> *mut Timer {
    let timer = Timer::new_with_rust_callback(secs_from_now, repeating, callback, callback_data);
    Box::into_raw(Box::new(timer))
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn timer_rust_destroy_timer(timer: *mut Timer) {
    if timer.is_null() {
        return;
    }

    let t = unsafe { &mut *timer };

    // Already invalidated? Nothing to do.
    if !t.cpp_timer_valid {
        return;
    }

    // Mark as destroyed.
    t.cpp_timer_valid = false;
    t.cpp_timer = ptr::null_mut();
    t.rust_callback = None;

    if t.being_processed {
        // Defer actual freeing until the current batch finishes.
        DEFERRED_FREE_LIST.with(|list| {
            list.borrow_mut().push(timer);
        });
    } else {
        // Safe to free immediately.
        let _ = unsafe { Box::from_raw(timer) };
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn timer_rust_get_trigger(timer: *const Timer) -> TimeT {
    unsafe { (*timer).get_trigger() }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn timer_rust_set_trigger(timer: *mut Timer, nexttrigger: TimeT) {
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
pub unsafe extern "C" fn timer_rust_invalidate_cpp_timer(timer: *mut Timer) {
    if !timer.is_null() {
        unsafe { (*timer).cpp_timer_valid = false };
    }
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
    if manager_ptr.is_null() {
        return;
    }
    
    let rust_timer = unsafe { timer_ffi_get_rust_timer(cpp_timer) as *mut Timer };
    if rust_timer.is_null() {
        return;
    }
    
    let trigger_time = unsafe { (*rust_timer).get_trigger() };
    if trigger_time == 0 {
        return;
    }
    
    let matching_timers: Vec<(TimeT, *mut Timer)> = unsafe {
        (*manager_ptr).timers
            .range(trigger_time..=trigger_time)
            .map(|(&time, &timer)| (time, timer))
            .collect()
    };
    
    for (time, timer_ptr) in matching_timers {
        if std::ptr::eq(rust_timer, timer_ptr) {
            // Mark trigger as 0 first
            unsafe { (*timer_ptr).trigger = 0 };
            // Remove from map
            unsafe { (*manager_ptr).timers.remove(&time) };
            
            // IMPORTANT: Invalidate the C++ timer reference first
            unsafe { timer_rust_invalidate_cpp_timer(timer_ptr) };
            
            // DO NOT destroy the Rust wrapper here!
            // The C++ destructor will call timer_rust_destroy_timer
            // Destroying it here would cause a double-free
            break;
        }
    }
}
