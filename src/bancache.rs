/*
 * InspIRCd -- Internet Relay Chat Daemon
 *
 *   Copyright (C) 2021-2022, 2025 Sadie Powell <sadie@witchery.services>
 *   Copyright (C) 2012-2014 Attila Molnar <attilamolnar@hush.com>
 *   Copyright (C) 2012 Robby <robby@chatbelgie.be>
 *   Copyright (C) 2009 Daniel De Graaf <danieldg@inspircd.org>
 *   Copyright (C) 2007-2008 Robin Burchell <robin+git@viroteck.net>
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

use std::collections::HashMap;
use std::ffi::{c_char, c_void};
use std::ptr;

use crate::stringutils::{StdString, StdString_Destroy};

// Type definitions for C++ compatibility
type time_t = i64;

unsafe extern "C" {
    fn um_ffi_server_time() -> time_t;
    fn rust_log_manager_write(level: u8,
        type_str: *const c_char, type_length: usize, 
        message: *const c_char, message_length: usize);
}

/// Layout must match `BanCacheHit` in include/bancache.h (StdString, StdString, time_t).
#[repr(C)]
pub struct BanCacheHit {
    pub Type: StdString,
    pub Reason: StdString,
    pub Expiry: time_t,
}

impl Drop for BanCacheHit {
    fn drop(&mut self) {
        unsafe {
            StdString_Destroy(&mut self.Type);
            StdString_Destroy(&mut self.Reason);
        }
    }
}

impl BanCacheHit {
    pub fn new(type_str: &str, reason: &str, seconds: time_t) -> Self {
        let expiry = unsafe { um_ffi_server_time() } + if seconds != 0 { seconds } else { 86400 };
        BanCacheHit {
            Type: StdString::from_vec(type_str.as_bytes().to_vec()),
            Reason: StdString::from_vec(reason.as_bytes().to_vec()),
            Expiry: expiry,
        }
    }

    pub fn IsPositive(&self) -> bool {
        !self.Reason.is_empty()
    }
}

fn std_string_bytes_eq(s: &StdString, t: &str) -> bool {
    s.as_bytes() == t.as_bytes()
}

const TAG: &[u8] = b"BANCACHE\0";

/// A manager for ban cache, which allocates and deallocates and checks cached bans.
pub struct BanCacheManager {
    BanHash: HashMap<String, *mut BanCacheHit>,
}

fn log_bancache_line(message: &str) {
    let _tag = b"BANCACHE";
    let msg = message.as_bytes();
    unsafe {
        rust_log_manager_write(3, // debug level
            TAG.as_ptr() as *const c_char, TAG.len() - 1, // strip trailing \0 for length
            msg.as_ptr() as *const c_char, msg.len());
    }
}

impl BanCacheManager {
    pub fn new() -> Self {
        BanCacheManager {
            BanHash: HashMap::new(),
        }
    }

    pub fn AddHit(&mut self, ip: &str, type_str: &str, reason: &str, seconds: time_t) -> *mut BanCacheHit {
        if self.BanHash.contains_key(ip) {
            return ptr::null_mut();
        }

        let hit = Box::new(BanCacheHit::new(type_str, reason, seconds));
        let hit_ptr = Box::into_raw(hit);
        self.BanHash.insert(ip.to_string(), hit_ptr);
        hit_ptr
    }

    pub fn GetHit(&mut self, ip: &str) -> *mut BanCacheHit {
        if !self.BanHash.contains_key(ip) {
            return ptr::null_mut();
        }
        if self.RemoveIfExpiredByIp(ip) {
            return ptr::null_mut();
        }
        *self.BanHash.get(ip).unwrap()
    }

    pub fn RemoveEntries(&mut self, type_str: &str, positive: bool) {
        if positive {
            let line = format!(
                "BanCacheManager::RemoveEntries(): Removing positive hits for {type_str}"
            );
            log_bancache_line(&line);
        } else {
            log_bancache_line("BanCacheManager::RemoveEntries(): Removing all negative hits");
        }

        let mut ips_to_remove = Vec::new();

        for (ip, &hit_ptr) in &self.BanHash {
            if self.IsExpiredByPtr(hit_ptr) {
                ips_to_remove.push(ip.clone());
                continue;
            }

            let hit = unsafe { &*hit_ptr };
            let remove = if positive {
                hit.IsPositive() && std_string_bytes_eq(&hit.Type, type_str)
            } else {
                !hit.IsPositive()
            };

            if remove {
                let line = format!("BanCacheManager::RemoveEntries(): Removing a hit on {ip}");
                log_bancache_line(&line);
                ips_to_remove.push(ip.clone());
            }
        }

        for ip in ips_to_remove {
            if let Some(hit_ptr) = self.BanHash.remove(&ip) {
                unsafe {
                    let _ = Box::from_raw(hit_ptr);
                }
            }
        }
    }

    fn IsExpiredByPtr(&self, hit_ptr: *mut BanCacheHit) -> bool {
        let hit = unsafe { &*hit_ptr };
        let current_time = unsafe { um_ffi_server_time() };
        current_time >= hit.Expiry
    }

    fn RemoveIfExpiredByIp(&mut self, ip: &str) -> bool {
        let Some(&hit_ptr) = self.BanHash.get(ip) else {
            return false;
        };
        if !self.IsExpiredByPtr(hit_ptr) {
            return false;
        }
        let msg = format!("Hit on {ip} is out of date, removing!");
        log_bancache_line(&msg);
        self.BanHash.remove(ip);
        unsafe {
            let _ = Box::from_raw(hit_ptr);
        }
        true
    }
}

impl Drop for BanCacheManager {
    fn drop(&mut self) {
        for (_, hit_ptr) in std::mem::take(&mut self.BanHash) {
            unsafe {
                let _ = Box::from_raw(hit_ptr);
            }
        }
    }
}

pub struct BanCacheModule {
    manager: BanCacheManager,
}

impl BanCacheModule {
    pub fn new() -> Self {
        BanCacheModule {
            manager: BanCacheManager::new(),
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn BanCacheManager_Create() -> *mut c_void {
    let module = Box::new(BanCacheModule::new());
    Box::into_raw(module) as *mut c_void
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn BanCacheManager_Destroy(ptr: *mut c_void) {
    if ptr.is_null() {
        return;
    }
    unsafe {
        let _ = Box::from_raw(ptr as *mut BanCacheModule);
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn BanCacheManager_AddHit(
    module_ptr: *mut c_void,
    ip: *const c_char,
    type_str: *const c_char,
    reason: *const c_char,
    seconds: time_t,
) -> *mut c_void {
    if module_ptr.is_null() || ip.is_null() || type_str.is_null() || reason.is_null() {
        return ptr::null_mut();
    }

    let module = unsafe { &mut *(module_ptr as *mut BanCacheModule) };

    let ip_s = unsafe { std::ffi::CStr::from_ptr(ip) }.to_str().unwrap_or("");
    let type_s = unsafe { std::ffi::CStr::from_ptr(type_str) }
        .to_str()
        .unwrap_or("");
    let reason_s = unsafe { std::ffi::CStr::from_ptr(reason) }
        .to_str()
        .unwrap_or("");

    module.manager.AddHit(ip_s, type_s, reason_s, seconds) as *mut c_void
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn BanCacheManager_GetHit(module_ptr: *mut c_void, ip: *const c_char) -> *mut c_void {
    if module_ptr.is_null() || ip.is_null() {
        return ptr::null_mut();
    }

    let module = unsafe { &mut *(module_ptr as *mut BanCacheModule) };

    let ip_s = unsafe { std::ffi::CStr::from_ptr(ip) }.to_str().unwrap_or("");
    module.manager.GetHit(ip_s) as *mut c_void
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn BanCacheManager_RemoveEntries(
    module_ptr: *mut c_void,
    type_str: *const c_char,
    positive: bool,
) {
    if module_ptr.is_null() || type_str.is_null() {
        return;
    }

    let module = unsafe { &mut *(module_ptr as *mut BanCacheModule) };

    let type_s = unsafe { std::ffi::CStr::from_ptr(type_str) }
        .to_str()
        .unwrap_or("");
    module.manager.RemoveEntries(type_s, positive);
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn BanCacheHit_IsPositive(hit_ptr: *mut c_void) -> bool {
    if hit_ptr.is_null() {
        return false;
    }

    let hit = unsafe { &*(hit_ptr as *mut BanCacheHit) };
    hit.IsPositive()
}

fn clone_std_string_bytes(s: &StdString) -> Vec<u8> {
    s.as_bytes().to_vec()
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn BanCacheHit_GetType(hit_ptr: *mut c_void) -> StdString {
    if hit_ptr.is_null() {
        return StdString::from_vec(Vec::new());
    }

    let hit = unsafe { &*(hit_ptr as *mut BanCacheHit) };
    StdString::from_vec(clone_std_string_bytes(&hit.Type))
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn BanCacheHit_GetReason(hit_ptr: *mut c_void) -> StdString {
    if hit_ptr.is_null() {
        return StdString::from_vec(Vec::new());
    }

    let hit = unsafe { &*(hit_ptr as *mut BanCacheHit) };
    StdString::from_vec(clone_std_string_bytes(&hit.Reason))
}
