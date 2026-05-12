/*
 * InspIRCd -- Internet Relay Chat Daemon
 *
 *   Copyright (C) 2021-2025 Sadie Powell <sadie@witchery.services>
 *   Copyright (C) 2013-2014 Attila Molnar <attilamolnar@hush.com>
 *   Copyright (C) 2013 Adam <Adam@anope.org>
 *   Copyright (C) 2012 Robby <robby@chatbelgie.be>
 *   Copyright (C) 2009-2010 Daniel De Graaf <danieldg@inspircd.org>
 *   Copyright (C) 2008 Robin Burchell <robin+git@viroteck.net>
 *   Copyright (C) 2007 Dennis Friis <peavey@inspircd.org>
 *   Copyright (C) 2006 Craig Edwards <brain@inspircd.org>
 */

#![allow(non_snake_case)]
#![allow(unsafe_op_in_unsafe_fn)]

use std::ffi::{c_char, CString};
use std::os::raw::c_void;

type SnomaskManager = c_void;
type Snomask = c_void;

unsafe extern "C" {
    fn snomask_ffi_description_set(mgr: *mut SnomaskManager, slot: usize, text: *const c_char);
    fn snomask_ffi_mask(mgr: *mut SnomaskManager, slot: usize) -> *mut Snomask;
    fn snomask_ffi_description_cstr(s: *mut Snomask) -> *const c_char;
    fn snomask_ffi_last_message_assign(s: *mut Snomask, v: *const c_char);
    fn snomask_ffi_last_message_clear(s: *mut Snomask);
    fn snomask_ffi_last_message_cstr(s: *mut Snomask) -> *const c_char;
    fn snomask_ffi_last_letter_set(s: *mut Snomask, c: c_char);
    fn snomask_ffi_last_letter_get(s: *mut Snomask) -> c_char;
    fn snomask_ffi_count_get(s: *mut Snomask) -> u32;
    fn snomask_ffi_count_set(s: *mut Snomask, v: u32);
    fn snomask_ffi_no_snotice_stack() -> bool;
    fn snomask_ffi_first_mod_on_send_snotice(
        letter: c_char,
        desc: *const c_char,
        msg: *const c_char,
    ) -> bool;
    fn snomask_ffi_foreach_mod_on_send_snotice(
        letter: c_char,
        desc: *const c_char,
        msg: *const c_char,
    );
    fn snomask_ffi_send_impl(letter: c_char, desc: *const c_char, msg: *const c_char);
    fn snomask_ffi_send_global_notice(letter: c_char, text: *const c_char);
}

fn c_str_to_string(p: *const c_char) -> String {
    if p.is_null() {
        return String::new();
    }
    unsafe { std::ffi::CStr::from_ptr(p) }
        .to_string_lossy()
        .into_owned()
}

fn slot_any_case(letter: u8) -> Option<usize> {
    if (b'a'..=b'z').contains(&letter) {
        Some((letter - b'a') as usize)
    } else if (b'A'..=b'Z').contains(&letter) {
        Some((letter - b'A') as usize)
    } else {
        None
    }
}

/** Returns the description of this snomask */
fn snomask_get_description(sn: *mut Snomask, letter: u8) -> String {
    let mut ret = String::new();
    if letter.is_ascii_uppercase() {
        ret.push_str("REMOTE");
    }
    let desc = unsafe { c_str_to_string(snomask_ffi_description_cstr(sn)) };
    if !desc.is_empty() {
        ret.push_str(&desc);
    } else {
        ret.push_str(&format!(
            "SNO-{}",
            (letter as char).to_ascii_lowercase()
        ));
    }
    ret
}

unsafe fn send_via_ffi(letter: u8, desc: &str, msg: &str) {
    let dc = CString::new(desc).unwrap_or_else(|_| CString::new("").unwrap());
    let mc = CString::new(msg).unwrap_or_else(|_| CString::new("").unwrap());
    snomask_ffi_send_impl(letter as c_char, dc.as_ptr(), mc.as_ptr());
}

unsafe fn snomask_flush_one(sn: *mut Snomask) {
    let cnt = snomask_ffi_count_get(sn);
    if cnt > 1 {
        let last_letter = snomask_ffi_last_letter_get(sn) as u8;
        let desc = snomask_get_description(sn, last_letter);
        let msg = format!("(last message repeated {} times)", cnt);
        let desc_c = CString::new(desc.as_str()).unwrap_or_else(|_| CString::new("").unwrap());
        let msg_c = CString::new(msg.as_str()).unwrap_or_else(|_| CString::new("").unwrap());
        snomask_ffi_foreach_mod_on_send_snotice(
            last_letter as c_char,
            desc_c.as_ptr(),
            msg_c.as_ptr(),
        );
        send_via_ffi(last_letter, &desc, &msg);
    }

    snomask_ffi_last_message_clear(sn);
    snomask_ffi_count_set(sn, 0);
}

unsafe fn snomask_send_message(sn: *mut Snomask, message: &str, letter: u8) {
    let stack_ok = !snomask_ffi_no_snotice_stack();
    let same_line = message == c_str_to_string(snomask_ffi_last_message_cstr(sn));
    let same_letter = letter == snomask_ffi_last_letter_get(sn) as u8;
    if stack_ok && same_line && same_letter {
        snomask_ffi_count_set(sn, snomask_ffi_count_get(sn).saturating_add(1));
        return;
    }

    snomask_flush_one(sn);

    let desc = snomask_get_description(sn, letter);
    let desc_c = CString::new(desc.as_str()).unwrap_or_else(|_| CString::new("").unwrap());
    let msg_c = CString::new(message).unwrap_or_else(|_| CString::new("").unwrap());
    if snomask_ffi_first_mod_on_send_snotice(
        letter as c_char,
        desc_c.as_ptr(),
        msg_c.as_ptr(),
    ) {
        return;
    }

    send_via_ffi(letter, &desc, message);

    snomask_ffi_last_message_assign(sn, msg_c.as_ptr());
    snomask_ffi_last_letter_set(sn, letter as c_char);
    snomask_ffi_count_set(sn, snomask_ffi_count_get(sn).saturating_add(1));
}

unsafe fn write_to_sno_mask(mgr: *mut SnomaskManager, letter: u8, text: &str) {
    if (b'a'..=b'z').contains(&letter) {
        let sn = snomask_ffi_mask(mgr, (letter - b'a') as usize);
        if !sn.is_null() {
            snomask_send_message(sn, text, letter);
        }
    } else if (b'A'..=b'Z').contains(&letter) {
        let sn = snomask_ffi_mask(mgr, (letter - b'A') as usize);
        if !sn.is_null() {
            snomask_send_message(sn, text, letter);
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn rust_snomask_flush_snotices(mgr: *mut SnomaskManager) {
    for i in 0..26usize {
        let sn = snomask_ffi_mask(mgr, i);
        if !sn.is_null() {
            snomask_flush_one(sn);
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn rust_snomask_enable(
    mgr: *mut SnomaskManager,
    letter: c_char,
    desc: *const c_char,
) {
    let lc = letter as u8;
    if (b'a'..=b'z').contains(&lc) {
        let slot = (lc - b'a') as usize;
        snomask_ffi_description_set(mgr, slot, desc);
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn rust_snomask_write_to_mask(
    mgr: *mut SnomaskManager,
    letter: c_char,
    text: *const c_char,
) {
    let s = c_str_to_string(text);
    write_to_sno_mask(mgr, letter as u8, &s);
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn rust_snomask_write_global_sno(
    mgr: *mut SnomaskManager,
    letter: c_char,
    text: *const c_char,
) {
    let s = c_str_to_string(text);
    write_to_sno_mask(mgr, letter as u8, &s);
    let tc = CString::new(s.as_str()).unwrap_or_else(|_| CString::new("").unwrap());
    snomask_ffi_send_global_notice(letter, tc.as_ptr());
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn rust_snomask_manager_ctor_init(mgr: *mut SnomaskManager) {
    let pairs = [
        (b'a' as c_char, c"ANNOUNCEMENT".as_ptr()),
        (b'c' as c_char, c"CONNECT".as_ptr()),
        (b'k' as c_char, c"KILL".as_ptr()),
        (b'o' as c_char, c"OPER".as_ptr()),
        (b'q' as c_char, c"QUIT".as_ptr()),
        (b'r' as c_char, c"REHASH".as_ptr()),
    ];
    for (ch, ptr) in pairs {
        rust_snomask_enable(mgr, ch, ptr);
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn rust_snomask_is_snomask(ch: c_char) -> bool {
    let c = ch as u8;
    (b'a'..=b'z').contains(&c) || (b'A'..=b'Z').contains(&c)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn rust_snomask_is_usable(mgr: *mut SnomaskManager, ch: c_char) -> bool {
    if !rust_snomask_is_snomask(ch) {
        return false;
    }
    let Some(slot) = slot_any_case(ch as u8) else {
        return false;
    };
    let sn = snomask_ffi_mask(mgr, slot);
    if sn.is_null() {
        return false;
    }
    !c_str_to_string(snomask_ffi_description_cstr(sn)).is_empty()
}
