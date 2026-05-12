/*
 * InspIRCd -- Internet Relay Chat Daemon
 *
 *   Copyright (C) 2019 linuxdaemon <linuxdaemon.irc@gmail.com>
 *   Copyright (C) 2018 systocrat <systocrat@outlook.com>
 *   Copyright (C) 2018 Dylan Frank <b00mx0r@aureus.pw>
 *   Copyright (C) 2013, 2017-2026 Sadie Powell <sadie@witchery.services>
 */

#![allow(non_snake_case)]
#![allow(unsafe_op_in_unsafe_fn)]

use std::ffi::{c_char, CString};
use std::os::raw::c_uchar;
use std::ptr;
use std::slice;

type User = core::ffi::c_void;

const MAX_USERMODE_HANDLERS: usize = 512;
const MODE_PARAM_BUF: usize = 8192;

unsafe extern "C" {
    fn user_ffi_invalidate_cache(u: *mut User);

    fn user_ffi_user_read_real_user(u: *const User, out: *mut *const u8, len: *mut usize);
    fn user_ffi_user_read_cached_address(u: *mut User, out: *mut *const u8, len: *mut usize);
    fn user_ffi_user_read_displayed_user(u: *const User, out: *mut *const u8, len: *mut usize);
    fn user_ffi_user_read_displayed_host(u: *const User, out: *mut *const u8, len: *mut usize);
    fn user_ffi_user_read_real_host(u: *const User, out: *mut *const u8, len: *mut usize);
    fn user_ffi_user_read_nick(u: *const User, out: *mut *const u8, len: *mut usize);

    fn user_ffi_user_set_cached_useraddress(u: *mut User, data: *const u8, len: usize);
    fn user_ffi_user_set_cached_userhost(u: *mut User, data: *const u8, len: usize);
    fn user_ffi_user_set_cached_realuserhost(u: *mut User, data: *const u8, len: usize);
    fn user_ffi_user_set_cached_mask(u: *mut User, data: *const u8, len: usize);
    fn user_ffi_user_set_cached_realmask(u: *mut User, data: *const u8, len: usize);

    fn user_ffi_find_user_mode_char(m: c_uchar) -> *mut core::ffi::c_void;
    fn user_ffi_user_mode_id_is_set(u: *const User, id: u32) -> bool;
    fn user_ffi_usermode_handlers_fill(out: *mut *mut core::ffi::c_void, max_out: usize) -> usize;
    fn user_ffi_modehandler_id(mh: *mut core::ffi::c_void) -> u32;
    fn user_ffi_modehandler_char(mh: *mut core::ffi::c_void) -> c_char;
    fn user_ffi_modehandler_needs_param_on_list(mh: *mut core::ffi::c_void) -> bool;
    fn user_ffi_modehandler_user_parameter_copy(
        u: *mut User,
        mh: *mut core::ffi::c_void,
        buf: *mut u8,
        cap: usize,
    ) -> usize;

    fn user_ffi_user_notice_mask_bit(u: *const User, sm: c_uchar) -> bool;
    fn user_ffi_user_shares_channel_with(u: *const User, other: *mut User) -> bool;
}

#[inline]
fn snomask_char_is_valid(c: u8) -> bool {
    matches!(c, b'a'..=b'z' | b'A'..=b'Z')
}

fn fmt_user_at_host(user: &[u8], host: &[u8]) -> Vec<u8> {
    let mut v = Vec::with_capacity(user.len() + 1 + host.len());
    v.extend_from_slice(user);
    v.push(b'@');
    v.extend_from_slice(host);
    v
}

fn fmt_nick_user_host(nick: &[u8], user: &[u8], host: &[u8]) -> Vec<u8> {
    let mut v = Vec::with_capacity(nick.len() + 1 + user.len() + 1 + host.len());
    v.extend_from_slice(nick);
    v.push(b'!');
    v.extend_from_slice(user);
    v.push(b'@');
    v.extend_from_slice(host);
    v
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn rust_user_invalidate_cache(u: *mut User) {
    user_ffi_invalidate_cache(u);
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn rust_user_is_notice_mask_set(u: *const User, sm: c_uchar) -> bool {
    if !snomask_char_is_valid(sm) {
        return false;
    }
    user_ffi_user_notice_mask_bit(u, sm)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn rust_user_is_mode_set(u: *mut User, m: c_uchar) -> bool {
    let mh = user_ffi_find_user_mode_char(m);
    if mh.is_null() {
        return false;
    }
    let id = user_ffi_modehandler_id(mh);
    user_ffi_user_mode_id_is_set(u, id)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn rust_user_get_mode_letters(u: *mut User, includeparams: bool) -> *mut c_char {
    let mut letters = vec![b'+'];
    let mut params = Vec::<u8>::new();

    let mut handlers = vec![ptr::null_mut::<core::ffi::c_void>(); MAX_USERMODE_HANDLERS];
    let n = user_ffi_usermode_handlers_fill(handlers.as_mut_ptr(), handlers.len());
    let mut param_buf = vec![0u8; MODE_PARAM_BUF];

    for mh in handlers.iter().take(n).copied() {
        if mh.is_null() {
            continue;
        }
        let id = user_ffi_modehandler_id(mh);
        if !user_ffi_user_mode_id_is_set(u, id) {
            continue;
        }
        let ch = user_ffi_modehandler_char(mh);
        if ch == 0 {
            continue;
        }
        letters.push(ch as u8);
        if includeparams && user_ffi_modehandler_needs_param_on_list(mh) {
            let plen = user_ffi_modehandler_user_parameter_copy(
                u,
                mh,
                param_buf.as_mut_ptr(),
                param_buf.len(),
            );
            if plen > 0 {
                params.push(b' ');
                params.extend_from_slice(&param_buf[..plen]);
            }
        }
    }

    letters.extend_from_slice(&params);
    CString::new(letters)
        .map(|c| c.into_raw())
        .unwrap_or_else(|_| ptr::null_mut())
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn rust_users_free_c_string(p: *mut c_char) {
    if !p.is_null() {
        drop(CString::from_raw(p));
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn rust_user_fill_cached_user_address(u: *mut User) {
    let mut pr = ptr::null();
    let mut lr = 0usize;
    user_ffi_user_read_real_user(u as *const User, &mut pr, &mut lr);
    let mut pa = ptr::null();
    let mut la = 0usize;
    user_ffi_user_read_cached_address(u, &mut pa, &mut la);
    let user = slice::from_raw_parts(pr, lr);
    let addr = slice::from_raw_parts(pa, la);
    let out = fmt_user_at_host(user, addr);
    user_ffi_user_set_cached_useraddress(u, out.as_ptr(), out.len());
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn rust_user_fill_cached_user_host(u: *mut User) {
    let uc = u as *const User;
    let mut pu = ptr::null();
    let mut lu = 0usize;
    user_ffi_user_read_displayed_user(uc, &mut pu, &mut lu);
    let mut ph = ptr::null();
    let mut lh = 0usize;
    user_ffi_user_read_displayed_host(uc, &mut ph, &mut lh);
    let user = slice::from_raw_parts(pu, lu);
    let host = slice::from_raw_parts(ph, lh);
    let out = fmt_user_at_host(user, host);
    user_ffi_user_set_cached_userhost(u, out.as_ptr(), out.len());
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn rust_user_fill_cached_real_user_host(u: *mut User) {
    let uc = u as *const User;
    let mut pu = ptr::null();
    let mut lu = 0usize;
    user_ffi_user_read_real_user(uc, &mut pu, &mut lu);
    let mut ph = ptr::null();
    let mut lh = 0usize;
    user_ffi_user_read_real_host(uc, &mut ph, &mut lh);
    let user = slice::from_raw_parts(pu, lu);
    let host = slice::from_raw_parts(ph, lh);
    let out = fmt_user_at_host(user, host);
    user_ffi_user_set_cached_realuserhost(u, out.as_ptr(), out.len());
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn rust_user_fill_cached_mask(u: *mut User) {
    let uc = u as *const User;
    let mut pn = ptr::null();
    let mut ln = 0usize;
    user_ffi_user_read_nick(uc, &mut pn, &mut ln);
    let mut pu = ptr::null();
    let mut lu = 0usize;
    user_ffi_user_read_displayed_user(uc, &mut pu, &mut lu);
    let mut ph = ptr::null();
    let mut lh = 0usize;
    user_ffi_user_read_displayed_host(uc, &mut ph, &mut lh);
    let nick = slice::from_raw_parts(pn, ln);
    let user = slice::from_raw_parts(pu, lu);
    let host = slice::from_raw_parts(ph, lh);
    let out = fmt_nick_user_host(nick, user, host);
    user_ffi_user_set_cached_mask(u, out.as_ptr(), out.len());
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn rust_user_fill_cached_real_mask(u: *mut User) {
    let uc = u as *const User;
    let mut pn = ptr::null();
    let mut ln = 0usize;
    user_ffi_user_read_nick(uc, &mut pn, &mut ln);
    let mut pu = ptr::null();
    let mut lu = 0usize;
    user_ffi_user_read_real_user(uc, &mut pu, &mut lu);
    let mut ph = ptr::null();
    let mut lh = 0usize;
    user_ffi_user_read_real_host(uc, &mut ph, &mut lh);
    let nick = slice::from_raw_parts(pn, ln);
    let user = slice::from_raw_parts(pu, lu);
    let host = slice::from_raw_parts(ph, lh);
    let out = fmt_nick_user_host(nick, user, host);
    user_ffi_user_set_cached_realmask(u, out.as_ptr(), out.len());
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn rust_user_shares_channel_with(u: *const User, other: *mut User) -> bool {
    user_ffi_user_shares_channel_with(u, other)
}

/** Expand IPv4-style addresses that start with ':' so IRC wire format stays consistent (matches legacy User::GetAddress). */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn rust_users_normalize_addr_display(
    inp: *const u8,
    in_len: usize,
    out: *mut u8,
    out_cap: usize,
) -> usize {
    if in_len == 0 || inp.is_null() || out.is_null() {
        return 0;
    }
    let s = slice::from_raw_parts(inp, in_len);
    if s[0] == b':' {
        let need = in_len + 1;
        if out_cap < need {
            return 0;
        }
        *out = b'0';
        ptr::copy_nonoverlapping(s.as_ptr(), out.add(1), in_len);
        need
    } else if out_cap < in_len {
        0
    } else {
        ptr::copy_nonoverlapping(s.as_ptr(), out, in_len);
        in_len
    }
}

/** Strip CR, replace NUL with space — same rules as UserIOHandler::OnDataReady line assembly. */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn rust_users_filter_irc_line(
    inp: *const u8,
    in_len: usize,
    out: *mut u8,
    out_cap: usize,
) -> usize {
    if inp.is_null() || out.is_null() {
        return 0;
    }
    let mut j = 0usize;
    for i in 0..in_len {
        let c = *inp.add(i);
        if c == b'\r' {
            continue;
        }
        if j >= out_cap {
            return 0;
        }
        *out.add(j) = if c == 0 { b' ' } else { c };
        j += 1;
    }
    j
}
