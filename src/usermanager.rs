use std::ffi::{c_char, c_int, c_void, CStr, CString};
use std::ptr;

type UserManager = c_void;
type User = c_void;
type LocalUser = c_void;
type ListenSocket = c_void;
type UserIOHandler = c_void;
type XLine = c_void;
type BanCacheHit = c_void;

#[allow(non_camel_case_types)]
type time_t = i64;

const CONN_FULL: u32 = 7;
const CONN_NICKUSER: u32 = 3;
const FD_WANT_FAST_READ: c_int = 0x4;
const FD_WANT_EDGE_WRITE: c_int = 0x80;

const CONNECTION_TIMEOUT_MSG: &str = "Connection timeout";
const ERR_INTERNAL_CONN: &str = "Internal error handling connection";
const ERR_SOFT_LIMIT: &str = "No more connections allowed";

unsafe extern "C" {
    fn um_ffi_server_time() -> time_t;
    fn um_ffi_local_user_nextping(lu: *mut LocalUser) -> time_t;
    fn um_ffi_local_user_set_nextping(lu: *mut LocalUser, t: time_t);
    fn um_ffi_local_user_lastping(lu: *mut LocalUser) -> u32;
    fn um_ffi_local_user_set_lastping(lu: *mut LocalUser, v: u32);
    fn um_ffi_local_user_class_pingtime(lu: *mut LocalUser) -> u64;
    fn um_ffi_duration_to_long_string(secs: u64) -> *const c_char;
    fn um_ffi_streamsocket_get_iohook(eh: *mut UserIOHandler) -> *mut c_void;
    fn um_ffi_iohook_ping(hook: *mut c_void) -> bool;
    fn um_ffi_iohook_next_in_chain(hook: *mut c_void) -> *mut c_void;
    fn um_ffi_local_user_send_irc_ping(lu: *mut LocalUser);
    fn um_ffi_local_user_connection_timeout_deadline(lu: *mut LocalUser) -> i64;
    fn um_ffi_mod_on_check_ready_is_passthru(lu: *mut LocalUser) -> bool;
    fn um_ffi_local_user_full_connect(lu: *mut LocalUser);
    fn um_ffi_listen_socket_iohookprov_count(via: *mut ListenSocket) -> usize;
    fn um_ffi_listen_socket_iohookprov_empty_name(via: *mut ListenSocket, idx: usize) -> bool;
    fn um_ffi_listen_socket_iohookprov_valid(via: *mut ListenSocket, idx: usize) -> bool;
    fn um_ffi_log_listen_iohook_nonexistent(via: *mut ListenSocket, idx: usize);
    fn um_ffi_insp_format_misconfigured_iohook(idx: usize) -> *const c_char;
    fn um_ffi_listen_socket_iohookprov_on_accept(
        via: *mut ListenSocket,
        idx: usize,
        eh: *mut UserIOHandler,
        client: *const c_void,
        server: *const c_void,
    );
    fn um_ffi_useriohandler_error_nonempty(eh: *mut UserIOHandler) -> bool;
    fn um_ffi_useriohandler_error_cstr(eh: *mut UserIOHandler) -> *const c_char;

    fn um_ffi_local_user_new(
        socket: c_int,
        client: *const c_void,
        server: *const c_void,
    ) -> *mut LocalUser;
    fn um_ffi_log_users_new_fd(socket: c_int);
    fn um_ffi_local_user_iohandler(lu: *mut LocalUser) -> *mut UserIOHandler;
    fn um_ffi_foreach_mod_on_user_init(lu: *mut LocalUser);
    fn um_ffi_socket_engine_add_fd(eh: *mut UserIOHandler, event_mask: c_int) -> bool;
    fn um_ffi_log_users_internal_error();
    fn um_ffi_log_softlimit_warning();
    fn um_ffi_config_soft_limit() -> usize;
    fn um_ffi_user_manager_rust_access_local_users_size(um: *mut UserManager) -> usize;
    fn um_ffi_local_user_find_connect_class(lu: *mut LocalUser) -> bool;
    fn um_ffi_local_user_set_exempt_from_eline(lu: *mut LocalUser, exempt: bool);
    fn um_ffi_xline_matches_E(lu: *mut LocalUser) -> *mut XLine;
    fn um_ffi_ban_cache_get_hit(addr: *const c_char) -> *mut BanCacheHit;
    fn um_ffi_bancache_hit_type_non_empty(b: *mut BanCacheHit) -> bool;
    fn um_ffi_bancache_hit_reason_cstr(b: *mut BanCacheHit) -> *const c_char;
    fn um_ffi_log_bancache_positive(addr: *const c_char);
    fn um_ffi_log_bancache_negative(addr: *const c_char);
    fn um_ffi_config_xline_message_empty() -> bool;
    fn um_ffi_local_user_write_numeric_banned(lu: *mut LocalUser);
    fn um_ffi_local_user_get_address_cstr(lu: *mut LocalUser) -> *const c_char;
    fn um_ffi_xline_matches_Z(lu: *mut LocalUser) -> *mut XLine;
    fn um_ffi_xline_apply(line: *mut XLine, lu: *mut LocalUser);
    fn um_ffi_config_raw_log() -> bool;
    fn um_ffi_log_notify_raw_io(lu: *mut LocalUser);
    fn um_ffi_foreach_mod_on_change_remote_address(lu: *mut LocalUser);
    fn um_ffi_foreach_mod_on_user_post_init(lu: *mut LocalUser);
    fn um_ffi_user_manager_rust_access_inc_unknown(um: *mut UserManager);
    fn um_ffi_user_manager_rust_access_client_insert(um: *mut UserManager, lu: *mut LocalUser);
    fn um_ffi_user_manager_rust_access_local_push_front(um: *mut UserManager, lu: *mut LocalUser);

    fn um_ffi_user_quitting(u: *mut User) -> bool;
    fn um_ffi_user_is_server(u: *mut User) -> bool;
    fn um_ffi_user_as_local(u: *mut User) -> *mut LocalUser;
    fn um_ffi_log_users_bug_quitting(nick: *const c_char);
    fn um_ffi_log_users_bug_server(nick: *const c_char);
    fn um_ffi_quit_user_run_prequit(
        lu: *mut LocalUser,
        quitmessage: *const c_char,
        operquitmessage_or_null: *const c_char,
    ) -> bool;
    fn um_ffi_quit_user_tls_quit() -> *const c_char;
    fn um_ffi_quit_user_tls_oper() -> *const c_char;
    fn um_ffi_user_set_quitting(u: *mut User);
    fn um_ffi_log_quit_user(uuid: *const c_char, nick: *const c_char, quitmessage: *const c_char);
    fn um_ffi_local_user_send_error_quit(lu: *mut LocalUser, operquitmsg: *const c_char);
    fn um_ffi_global_culls_add_item(u: *mut User);
    fn um_ffi_foreach_mod_on_user_quit(u: *mut User, quitmsg: *const c_char, operquitmsg: *const c_char);
    fn um_ffi_foreach_mod_on_user_disconnect(lu: *mut LocalUser);
    fn um_ffi_local_user_eh_close(lu: *mut LocalUser);
    fn um_ffi_sno_write_client_exiting(
        realmask: *const c_char,
        addr: *const c_char,
        operquitmsg: *const c_char,
    );
    fn um_ffi_user_get_real_mask_cstr(u: *mut User) -> *const c_char;
    fn um_ffi_user_get_address_cstr(u: *mut User) -> *const c_char;
    fn um_ffi_user_manager_rust_access_local_erase(um: *mut UserManager, lu: *mut LocalUser);
    fn um_ffi_local_user_connect_class_dec_use_count(lu: *mut LocalUser);
    fn um_ffi_user_manager_rust_access_client_erase_nick_cstr(um: *mut UserManager, nick: *const c_char) -> bool;
    fn um_ffi_log_users_bug_nick_not_found(nick: *const c_char);
    fn um_ffi_user_manager_rust_access_uuid_erase_cstr(um: *mut UserManager, uuid: *const c_char);
    fn um_ffi_user_purge_empty_channels(u: *mut User);
    fn um_ffi_user_oper_logout(u: *mut User);
    fn um_ffi_user_manager_rust_access_clonemap_clear(um: *mut UserManager);
    fn um_ffi_user_manager_rust_access_clonemap_add(um: *mut UserManager, u: *mut User);
    fn um_ffi_user_manager_rust_access_clonemap_remove(um: *mut UserManager, u: *mut User);
    fn um_ffi_user_manager_rust_access_client_iter_new(um: *mut UserManager) -> *mut c_void;
    fn um_ffi_user_manager_rust_access_client_iter_next(it: *mut c_void) -> *mut User;
    fn um_ffi_user_manager_rust_access_client_iter_free(it: *mut c_void);
    fn um_ffi_user_manager_rust_access_services_swap(um: *mut UserManager, users: *const *mut User, count: usize);
    fn um_ffi_user_manager_rust_access_local_iter_new(um: *mut UserManager) -> *mut c_void;
    fn um_ffi_user_manager_rust_access_local_iter_next(it: *mut c_void) -> *mut LocalUser;
    fn um_ffi_user_manager_rust_access_local_iter_free(it: *mut c_void);
    fn um_ffi_user_manager_rust_access_get_already_sent_id(um: *mut UserManager) -> u64;
    fn um_ffi_user_manager_rust_access_set_already_sent_id(um: *mut UserManager, v: u64);
    fn um_ffi_local_user_set_already_sent(lu: *mut LocalUser, v: u64);
    fn um_ffi_user_manager_rust_access_dec_unknown(um: *mut UserManager);
    fn um_ffi_user_manager_find_nick_impl(um: *mut UserManager, nick: *const c_char, fully: bool) -> *mut User;
    fn um_ffi_user_manager_find_uuid_impl(um: *mut UserManager, uuid: *const c_char, fully: bool) -> *mut User;
    fn um_ffi_user_get_nick_cstr(u: *mut User) -> *const c_char;
    fn um_ffi_user_get_uuid_cstr(u: *mut User) -> *const c_char;
    fn um_ffi_user_is_fully_connected(u: *mut User) -> bool;
    fn um_ffi_user_server_is_service(u: *mut User) -> bool;
    fn um_ffi_local_user_command_flood_penalty(lu: *mut LocalUser) -> u64;
    fn um_ffi_local_user_eh_get_sendq_size(lu: *mut LocalUser) -> usize;
    fn um_ffi_local_user_get_class_commandrate(lu: *mut LocalUser) -> u64;
    fn um_ffi_local_user_set_command_flood_penalty(lu: *mut LocalUser, v: u64);
    fn um_ffi_local_user_eh_on_data_ready(lu: *mut LocalUser);
    fn um_ffi_local_user_connected(lu: *mut LocalUser) -> u32;
    fn um_ffi_write_common_quit(u: *mut User, quitmsg: *const c_char, operquitmsg: *const c_char);
}

/// Copy a NUL-terminated C string from C++ into an owned Rust string (FFI boundary only).
unsafe fn c_str_to_string(p: *const c_char) -> String {
    if p.is_null() {
        return String::new();
    }
    unsafe { CStr::from_ptr(p).to_string_lossy().into_owned() }
}

unsafe fn user_nick_string(user: *mut User) -> String {
    unsafe { c_str_to_string(um_ffi_user_get_nick_cstr(user)) }
}

unsafe fn user_uuid_string(user: *mut User) -> String {
    unsafe { c_str_to_string(um_ffi_user_get_uuid_cstr(user)) }
}

/// Matches C++ `Duration::ToLongString` via one FFI call.
unsafe fn duration_long_string_from_cpp(secs: u64) -> String {
    unsafe { c_str_to_string(um_ffi_duration_to_long_string(secs)) }
}

fn ping_timeout_quit_message(secs_u64: u64) -> String {
    format!(
        "Ping timeout: {}",
        unsafe { duration_long_string_from_cpp(secs_u64) }
    )
}

unsafe fn user_real_mask_string(u: *mut User) -> String {
    unsafe { c_str_to_string(um_ffi_user_get_real_mask_cstr(u)) }
}

unsafe fn user_address_string_user(u: *mut User) -> String {
    unsafe { c_str_to_string(um_ffi_user_get_address_cstr(u)) }
}

unsafe fn local_user_address_string(lu: *mut LocalUser) -> String {
    unsafe { c_str_to_string(um_ffi_local_user_get_address_cstr(lu)) }
}

unsafe fn apply_local_user_flood_and_sendq_decay(curr: *mut LocalUser) {
    if unsafe { um_ffi_local_user_command_flood_penalty(curr) } != 0 || unsafe { um_ffi_local_user_eh_get_sendq_size(curr) } != 0 {
        let rate: u64 = unsafe { um_ffi_local_user_get_class_commandrate(curr) };
        let penalty: u64 = unsafe { um_ffi_local_user_command_flood_penalty(curr) };
        if penalty > rate {
            unsafe { um_ffi_local_user_set_command_flood_penalty(curr, penalty - rate) };
        } else {
            unsafe { um_ffi_local_user_set_command_flood_penalty(curr, 0) };
        }
        unsafe { um_ffi_local_user_eh_on_data_ready(curr) };
    }
}

unsafe fn check_ping_timeout(um: *mut UserManager, user: *mut LocalUser) {
    // Check if it is time to ping the user yet.
    if um_ffi_server_time() < um_ffi_local_user_nextping(user) {
        return;
    }

    // This user didn't answer the last ping, remove them.
    if um_ffi_local_user_lastping(user) == 0 {
        let t = um_ffi_server_time();
        let next = um_ffi_local_user_nextping(user);
        let pingtime = um_ffi_local_user_class_pingtime(user) as time_t;
        let secs_i64 = t - (next - pingtime);
        let secs_u64 = secs_i64.max(0) as u64;
        let msg = ping_timeout_quit_message(secs_u64);
        quit_user_inner(um, user.cast::<User>(), msg.as_str(), None);
        return;
    }

    um_ffi_local_user_set_lastping(user, 0);
    um_ffi_local_user_set_nextping(
        user,
        um_ffi_server_time() + um_ffi_local_user_class_pingtime(user) as time_t,
    );

    // If the user has an I/O hook that can handle pinging use that instead.
    let mut hook = um_ffi_streamsocket_get_iohook(um_ffi_local_user_iohandler(user));
    while !hook.is_null() {
        if um_ffi_iohook_ping(hook) {
            return; // Client has been pinged.
        }
        hook = um_ffi_iohook_next_in_chain(hook);
    }

    // Send a ping to the client using an IRC message.
    um_ffi_local_user_send_irc_ping(user);
}

unsafe fn check_connection_timeout(um: *mut UserManager, user: *mut LocalUser) {
    let dl = um_ffi_local_user_connection_timeout_deadline(user);
    if dl >= 0 && um_ffi_server_time() > dl {
        // Either the user did not send NICK/USER or a module blocked connection in
        // OnCheckReady until the client timed out.
        quit_user_inner(um, user.cast::<User>(), CONNECTION_TIMEOUT_MSG, None);
    }
}

unsafe fn check_modules_ready(um: *mut UserManager, user: *mut LocalUser) {
    if um_ffi_mod_on_check_ready_is_passthru(user) {
        // User has sent NICK/USER and modules are ready.
        um_ffi_local_user_full_connect(user);
        return;
    }

    // If the user has been quit in OnCheckReady then we shouldn't quit
    // them again for having a registration timeout.
    if !um_ffi_user_quitting(user.cast::<User>()) {
        check_connection_timeout(um, user);
    }
}

unsafe fn quit_user_inner(
    um: *mut UserManager,
    user: *mut User,
    quit_message: &str,
    oper_message: Option<&str>,
) {
    if um_ffi_user_quitting(user) {
        let nick = user_nick_string(user);
        let nick_c = CString::new(nick).unwrap_or_else(|_| CString::new("").unwrap());
        um_ffi_log_users_bug_quitting(nick_c.as_ptr());
        return;
    }

    if um_ffi_user_is_server(user) {
        let nick = user_nick_string(user);
        let nick_c = CString::new(nick).unwrap_or_else(|_| CString::new("").unwrap());
        um_ffi_log_users_bug_server(nick_c.as_ptr());
        return;
    }

    let localuser = um_ffi_user_as_local(user);
    let quit_c = CString::new(quit_message).unwrap_or_else(|_| CString::new("").unwrap());
    let oper_storage = oper_message.and_then(|s| CString::new(s).ok());
    let deny = um_ffi_quit_user_run_prequit(
        localuser,
        quit_c.as_ptr(),
        oper_storage
            .as_ref()
            .map(|c| c.as_ptr())
            .unwrap_or(ptr::null()),
    );
    if deny {
        return;
    }

    let quit_final = c_str_to_string(um_ffi_quit_user_tls_quit());
    let oper_final = c_str_to_string(um_ffi_quit_user_tls_oper());
    let quit_final_c = CString::new(quit_final.as_str()).unwrap_or_else(|_| CString::new("").unwrap());
    let oper_final_c = CString::new(oper_final.as_str()).unwrap_or_else(|_| CString::new("").unwrap());

    um_ffi_user_set_quitting(user);
    let uuid = user_uuid_string(user);
    let nick = user_nick_string(user);
    let uuid_c = CString::new(uuid.as_str()).unwrap_or_else(|_| CString::new("").unwrap());
    let nick_c = CString::new(nick.as_str()).unwrap_or_else(|_| CString::new("").unwrap());
    let log_quit_c = CString::new(quit_message).unwrap_or_else(|_| CString::new("").unwrap());
    um_ffi_log_quit_user(uuid_c.as_ptr(), nick_c.as_ptr(), log_quit_c.as_ptr());

    if !localuser.is_null() {
        um_ffi_local_user_send_error_quit(localuser, oper_final_c.as_ptr());
    }

    um_ffi_global_culls_add_item(user);

    if um_ffi_user_is_fully_connected(user) {
        um_ffi_foreach_mod_on_user_quit(user, quit_final_c.as_ptr(), oper_final_c.as_ptr());
        um_ffi_write_common_quit(user, quit_final_c.as_ptr(), oper_final_c.as_ptr());
    } else {
        um_ffi_user_manager_rust_access_dec_unknown(um);
    }

    if !localuser.is_null() {
        um_ffi_foreach_mod_on_user_disconnect(localuser);
        um_ffi_local_user_eh_close(localuser);

        if um_ffi_user_is_fully_connected(user) {
            let mask = user_real_mask_string(user);
            let addr = user_address_string_user(user);
            let mask_c = CString::new(mask.as_str()).unwrap_or_else(|_| CString::new("").unwrap());
            let addr_c = CString::new(addr.as_str()).unwrap_or_else(|_| CString::new("").unwrap());
            um_ffi_sno_write_client_exiting(mask_c.as_ptr(), addr_c.as_ptr(), oper_final_c.as_ptr());
        }
        um_ffi_user_manager_rust_access_local_erase(um, localuser);
        um_ffi_local_user_connect_class_dec_use_count(localuser);
    }

    let nick_erase = CString::new(nick.as_str()).unwrap_or_else(|_| CString::new("").unwrap());
    if !um_ffi_user_manager_rust_access_client_erase_nick_cstr(um, nick_erase.as_ptr()) {
        um_ffi_log_users_bug_nick_not_found(nick_erase.as_ptr());
    }

    let uuid_erase = CString::new(uuid.as_str()).unwrap_or_else(|_| CString::new("").unwrap());
    um_ffi_user_manager_rust_access_uuid_erase_cstr(um, uuid_erase.as_ptr());
    unsafe { um_ffi_user_purge_empty_channels(user); }
    unsafe { um_ffi_user_oper_logout(user); }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn rust_usermanager_quit_user(
    um: *mut UserManager,
    user: *mut User,
    quitmessage: *const c_char,
    operquitmessage: *const c_char,
) {
    let quit = if quitmessage.is_null() {
        String::new()
    } else {
        unsafe { c_str_to_string(quitmessage) }
    };
    let oper = if operquitmessage.is_null() {
        None
    } else {
        Some(c_str_to_string(operquitmessage))
    };
    quit_user_inner(um, user, quit.as_str(), oper.as_deref());
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn rust_usermanager_add_user(
    um: *mut UserManager,
    socket: c_int,
    via: *mut ListenSocket,
    client: *const c_void,
    server: *const c_void,
) {
    // User constructor allocates a new UUID for the user and inserts it into the uuidlist
    let new = um_ffi_local_user_new(socket, client, server);
    let eh = um_ffi_local_user_iohandler(new);

    um_ffi_log_users_new_fd(socket);

    um_ffi_user_manager_rust_access_inc_unknown(um);
    um_ffi_user_manager_rust_access_client_insert(um, new);
    um_ffi_user_manager_rust_access_clonemap_add(um, new.cast::<User>());
    um_ffi_user_manager_rust_access_local_push_front(um, new);
    um_ffi_foreach_mod_on_user_init(new);

    if !um_ffi_socket_engine_add_fd(eh, FD_WANT_FAST_READ | FD_WANT_EDGE_WRITE) {
        um_ffi_log_users_internal_error();
        quit_user_inner(um, new.cast::<User>(), ERR_INTERNAL_CONN, None);
        return;
    }

    // If this listener has an IO hook provider set then tell it about the connection
    let n = um_ffi_listen_socket_iohookprov_count(via);
    for idx in 0..n {
        if !um_ffi_listen_socket_iohookprov_valid(via, idx) {
            if um_ffi_listen_socket_iohookprov_empty_name(via, idx) {
                continue;
            }

            um_ffi_log_listen_iohook_nonexistent(via, idx);
            let msg = c_str_to_string(um_ffi_insp_format_misconfigured_iohook(idx));
            quit_user_inner(um, new.cast::<User>(), msg.as_str(), None);
            return;
        }

        um_ffi_listen_socket_iohookprov_on_accept(via, idx, eh, client, server);

        // IOHook could have encountered a fatal error, e.g. if the TLS ClientHello
        // was already in the queue and there was no common TLS version.
        if um_ffi_useriohandler_error_nonempty(eh) {
            let err = c_str_to_string(um_ffi_useriohandler_error_cstr(eh));
            quit_user_inner(um, new.cast::<User>(), err.as_str(), None);
            return;
        }
    }

    if um_ffi_user_manager_rust_access_local_users_size(um) > um_ffi_config_soft_limit() {
        um_ffi_log_softlimit_warning();
        quit_user_inner(um, new.cast::<User>(), ERR_SOFT_LIMIT, None);
        return;
    }

    if !um_ffi_local_user_find_connect_class(new) {
        return; // User does not match any connect classes.
    }

    /*
     * even with bancache, we still have to keep User::exempt current.
     * besides that, if we get a positive bancache hit, we still won't fuck
     * them over if they are exempt. -- w00t
     */
    let exempt = !um_ffi_xline_matches_E(new).is_null();
    um_ffi_local_user_set_exempt_from_eline(new, exempt);

    let addr = local_user_address_string(new);
    let addr_c = CString::new(addr.as_str()).unwrap_or_else(|_| CString::new("").unwrap());
    let b = um_ffi_ban_cache_get_hit(addr_c.as_ptr());
    if !b.is_null() {
        if um_ffi_bancache_hit_type_non_empty(b) && !exempt {
            /* user banned */
            um_ffi_log_bancache_positive(addr_c.as_ptr());

            if !um_ffi_config_xline_message_empty() {
                um_ffi_local_user_write_numeric_banned(new);
            }

            // IMPORTANT: we don't check XLineQuitPublic here because the only
            // person who might see the ban at this point is the affected user.
            let reason = c_str_to_string(um_ffi_bancache_hit_reason_cstr(b));
            quit_user_inner(um, new.cast::<User>(), reason.as_str(), None);
            return;
        } else {
            um_ffi_log_bancache_negative(addr_c.as_ptr());
        }
    } else if !exempt {
        let r = um_ffi_xline_matches_Z(new);
        if !r.is_null() {
            um_ffi_xline_apply(r, new);
            return;
        }
    }

    if um_ffi_config_raw_log() {
        um_ffi_log_notify_raw_io(new);
    }

    um_ffi_foreach_mod_on_change_remote_address(new);
    if !um_ffi_user_quitting(new.cast::<User>()) {
        um_ffi_foreach_mod_on_user_post_init(new);
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn rust_usermanager_add_clone(um: *mut UserManager, user: *mut User) {
    um_ffi_user_manager_rust_access_clonemap_add(um, user);
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn rust_usermanager_remove_clone_counts(um: *mut UserManager, user: *mut User) {
    um_ffi_user_manager_rust_access_clonemap_remove(um, user);
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn rust_usermanager_rehash_clone_counts(um: *mut UserManager) {
    um_ffi_user_manager_rust_access_clonemap_clear(um);
    let it = um_ffi_user_manager_rust_access_client_iter_new(um);
    loop {
        let u = um_ffi_user_manager_rust_access_client_iter_next(it);
        if u.is_null() {
            break;
        }
        um_ffi_user_manager_rust_access_clonemap_add(um, u);
    }
    um_ffi_user_manager_rust_access_client_iter_free(it);
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn rust_usermanager_rehash_services(um: *mut UserManager) {
    let mut vec: Vec<*mut User> = Vec::new();
    let it = um_ffi_user_manager_rust_access_client_iter_new(um);
    loop {
        let user = um_ffi_user_manager_rust_access_client_iter_next(it);
        if user.is_null() {
            break;
        }
        if um_ffi_user_server_is_service(user) {
            vec.push(user);
        }
    }
    um_ffi_user_manager_rust_access_client_iter_free(it);
    if vec.is_empty() {
        um_ffi_user_manager_rust_access_services_swap(um, ptr::null(), 0);
    } else {
        um_ffi_user_manager_rust_access_services_swap(um, vec.as_ptr(), vec.len());
    }
}

/// This function is called once a second from the mainloop.
/// It is intended to do background checking on all the users, e.g. do
/// ping checks, connection timeouts, etc.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn rust_usermanager_do_background_user_stuff(um: *mut UserManager) {
    let it = um_ffi_user_manager_rust_access_local_iter_new(um);
    loop {
        let curr = um_ffi_user_manager_rust_access_local_iter_next(it);
        if curr.is_null() {
            break;
        }

        // It's possible that we quit the user below due to ping timeout etc. and QuitUser() removes it from the list
        apply_local_user_flood_and_sendq_decay(curr);

        match um_ffi_local_user_connected(curr) {
            CONN_FULL => check_ping_timeout(um, curr),
            CONN_NICKUSER => check_modules_ready(um, curr),
            _ => check_connection_timeout(um, curr),
        }
    }
    um_ffi_user_manager_rust_access_local_iter_free(it);
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn rust_usermanager_next_already_sent_id(um: *mut UserManager) -> u64 {
    let mut id = um_ffi_user_manager_rust_access_get_already_sent_id(um);
    id = id.wrapping_add(1);
    um_ffi_user_manager_rust_access_set_already_sent_id(um, id);
    if id == 0 {
        // Wrapped around, reset the already_sent ids of all users
        unsafe { um_ffi_user_manager_rust_access_set_already_sent_id(um, 1) };
        let lit = unsafe { um_ffi_user_manager_rust_access_local_iter_new(um) };
        loop {
            let user = unsafe { um_ffi_user_manager_rust_access_local_iter_next(lit) };
            if user.is_null() {
                break;
            }
            unsafe { um_ffi_local_user_set_already_sent(user, 0) };
        }
        unsafe { um_ffi_user_manager_rust_access_local_iter_free(lit) };
        return 1;
    }
    id
}

unsafe fn find_user_by_nick_or_uuid_string(
    um: *mut UserManager,
    nickuuid: &str,
    fullyconnected: bool,
) -> *mut User {
    if nickuuid.is_empty() {
        return ptr::null_mut();
    }
    let key = CString::new(nickuuid).unwrap_or_else(|_| CString::new("").unwrap());
    let first = nickuuid.as_bytes()[0];
    if first.is_ascii_digit() {
        unsafe { um_ffi_user_manager_find_uuid_impl(um, key.as_ptr(), fullyconnected) }
    } else {
        unsafe { um_ffi_user_manager_find_nick_impl(um, key.as_ptr(), fullyconnected) }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn rust_usermanager_find(
    um: *mut UserManager,
    nickuuid: *const c_char,
    fullyconnected: bool,
) -> *mut User {
    if nickuuid.is_null() {
        return ptr::null_mut();
    }
    let s = unsafe { c_str_to_string(nickuuid) };
    unsafe { find_user_by_nick_or_uuid_string(um, s.as_str(), fullyconnected) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn rust_usermanager_find_nick(
    um: *mut UserManager,
    nick: *const c_char,
    fullyconnected: bool,
) -> *mut User {
    unsafe { um_ffi_user_manager_find_nick_impl(um, nick, fullyconnected) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn rust_usermanager_find_uuid(
    um: *mut UserManager,
    uuid: *const c_char,
    fullyconnected: bool,
) -> *mut User {
    unsafe { um_ffi_user_manager_find_uuid_impl(um, uuid, fullyconnected) }
}
