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

use crate::stringutils::StdString;

// Assumes: StdString defined elsewhere (must be 32 bytes on this system:
// data ptr (8), size (8), capacity (8), plus 8 bytes for SSO/small string optimization).
// vtable pointer is 8 bytes.

#[repr(C)]
pub struct User {
    // Base class Extensible: vtable pointer at offset 0.
    pub vtable: *const c_void,   // 8 bytes

    // Private fields (from users.h, in order)
    pub cached_address: StdString,       // offset 8,  32 bytes
    pub cached_useraddress: StdString,   // offset 40, 32 bytes
    pub cached_userhost: StdString,      // offset 72, 32 bytes
    pub cached_realuserhost: StdString,  // offset 104, 32 bytes
    pub cached_mask: StdString,          // offset 136, 32 bytes
    pub cached_realmask: StdString,      // offset 168, 32 bytes
    pub displayhost: StdString,          // offset 200, 32 bytes
    pub realhost: StdString,             // offset 232, 32 bytes
    pub realname: StdString,             // offset 264, 32 bytes
    pub displayuser: StdString,          // offset 296, 32 bytes
    pub realuser: StdString,             // offset 328, 32 bytes

    // modes: ModeParser::ModeStatus (std::bitset<64>)
    // Represented as a u64, same as snomasks. Alignment 8.
    pub modes: u64,                      // offset 360, 8 bytes

    // nickchanged: time_t (i64)
    pub nickchanged: i64,                // offset 368, 8 bytes

    // signon: time_t (i64)
    pub signon: i64,                     // offset 376, 8 bytes

    // client_sa: irc::sockets::sockaddrs (112 bytes, verified with C++ test)
    pub client_sa: [u8; 112],            // offset 384, 112 bytes

    // nick: std::string (32 bytes)
    pub nick: StdString,                 // offset 496, 32 bytes

    // uuid: const std::string (32 bytes)
    pub uuid: StdString,                 // offset 528, 32 bytes

    // snomasks: std::bitset<64> (u64)
    pub snomasks: u64,                   // offset 560, 8 bytes

    // chans: ChanList (intrusive list)
    // InspIRCd intrusive list stores a pointer to the first member/head.
    // That's 8 bytes.
    pub chans: *mut c_void,              // offset 568, 8 bytes

    // server: Server* (8 bytes)
    pub server: *mut c_void,             // offset 576, 8 bytes

    // away: std::optional<AwayState>
    // std::optional<T> is typically { bool has_value; union { T value; }; }
    // AwayState contains two fields: std::string message (32 bytes) and time_t time (8 bytes).
    // That's 40 bytes, plus the bool flag and padding to align to 8 bytes = 48 bytes total.
    pub away: AwayOptional,              // custom struct below

    // oper: std::shared_ptr<OperAccount>
    // Shared ptr is two pointers: object ptr (8) and control block ptr (8).
    pub oper_obj: *mut c_void,
    pub oper_ctrl: *mut c_void,

    // Bitfield members: connected:3, quitting:1, uniqueusername:1 (bool), usertype:2 (const)
    // All packed into a single unsigned int (4 bytes). Use u32 and access via bit ops.
    pub bitfield: u32,
}

/// Manual std::optional<AwayState> layout placeholder.
/// std::optional<T> is typically { bool has_value; union { T value; }; }
/// AwayState contains std::string message (32 bytes) and time_t time (8 bytes) = 40 bytes.
/// With bool flag (1 byte) + padding (7 bytes) to align to 8 bytes = 48 bytes total.
#[repr(C)]
pub struct AwayOptional {
    pub has_value: u8,      // bool
    // padding to align the AwayState (align 8)
    pub _pad: [u8; 7],
    pub value: AwayState,   // the actual away data
}

#[repr(C)]
pub struct AwayState {
    pub message: StdString, // 32 bytes
    pub time: i64,          // 8 bytes
}

fn get_connected(&self) -> u8 { (self.bitfield & 0b111) as u8 }
fn set_connected(&mut self, v: u8) { self.bitfield = (self.bitfield & !0b111) | (v as u32); }
fn get_quitting(&self) -> bool { ((self.bitfield >> 3) & 1) != 0 }
fn set_quitting(&mut self, v: bool) {
    if v {
        self.bitfield |= 1 << 3;
    } else {
        self.bitfield &= !(1 << 3);
    }
}
fn get_uniqueusername(&self) -> bool { ((self.bitfield >> 4) & 1) != 0 }
fn get_usertype(&self) -> u8 { ((self.bitfield >> 5) & 0b11) as u8 }
// usertype is const; you wouldn't have a setter in Rust, but it's set in the constructor.

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
