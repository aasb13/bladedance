#![allow(unsafe_op_in_unsafe_fn)]

use std::ffi::c_int;

#[cfg(target_os = "windows")]
use winapi::um::winsock2::{closesocket, ioctlsocket, FIONBIO, connect, bind, shutdown, listen, accept, SOCKET_ERROR, WSAEWOULDBLOCK};
#[cfg(target_os = "windows")]
use winapi::um::winsock2::SOCKADDR as sockaddr;
#[cfg(target_os = "windows")]
use winapi::shared::ws2def::SOCKADDR as sockaddr_alias;
use std::os::raw::c_void;

#[cfg(not(target_os = "windows"))]
use libc::{close, fcntl, F_GETFL, F_SETFL, O_NONBLOCK, connect, bind, shutdown, listen, accept, socklen_t};

type ssize_t = libc::ssize_t;
type uint64_t = libc::uint64_t;

#[cfg(target_os = "windows")]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn rust_socketengine_close(fd: c_int) -> c_int {
    closesocket(fd)
}

#[cfg(not(target_os = "windows"))]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn rust_socketengine_close(fd: c_int) -> c_int {
    close(fd)
}

#[cfg(target_os = "windows")]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn rust_socketengine_blocking(fd: c_int) -> c_int {
    let mut opt: u32 = 0;
    ioctlsocket(fd, FIONBIO, &mut opt)
}

#[cfg(not(target_os = "windows"))]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn rust_socketengine_blocking(fd: c_int) -> c_int {
    let flags = fcntl(fd, F_GETFL, 0);
    if flags == -1 {
        return -1;
    }
    fcntl(fd, F_SETFL, flags & !O_NONBLOCK)
}

#[cfg(target_os = "windows")]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn rust_socketengine_nonblocking(fd: c_int) -> c_int {
    let mut opt: u32 = 1;
    ioctlsocket(fd, FIONBIO, &mut opt)
}

#[cfg(not(target_os = "windows"))]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn rust_socketengine_nonblocking(fd: c_int) -> c_int {
    let flags = fcntl(fd, F_GETFL, 0);
    if flags == -1 {
        return -1;
    }
    fcntl(fd, F_SETFL, flags | O_NONBLOCK)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn rust_socketengine_stats_update_read_counters(
    len_in: ssize_t,
    read_events: *mut uint64_t,
    indata: *mut usize,
    error_events: *mut uint64_t,
) {
    if len_in > 0 {
        *read_events += 1;
        *indata += len_in as usize;
    } else if len_in < 0 {
        *read_events += 1;
        *error_events += 1;
    } else {
        *read_events += 1;
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn rust_socketengine_stats_update_write_counters(
    len_out: ssize_t,
    write_events: *mut uint64_t,
    outdata: *mut usize,
    error_events: *mut uint64_t,
) {
    if len_out > 0 {
        *write_events += 1;
        *outdata += len_out as usize;
    } else if len_out < 0 {
        *write_events += 1;
        *error_events += 1;
    } else {
        *write_events += 1;
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn rust_socketengine_stats_get_bandwidth(
    indata: usize,
    outdata: usize,
    kbitpersec_in: *mut f32,
    kbitpersec_out: *mut f32,
    kbitpersec_total: *mut f32,
) {
    let in_kbit = indata as f32 * 8.0;
    let out_kbit = outdata as f32 * 8.0;
    *kbitpersec_total = (in_kbit + out_kbit) / 1024.0;
    *kbitpersec_in = in_kbit / 1024.0;
    *kbitpersec_out = out_kbit / 1024.0;
}

#[cfg(target_os = "windows")]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn rust_socketengine_connect(
    fd: c_int,
    addr: *const sockaddr,
    addrlen: libc::c_int,
) -> c_int {
    let ret = connect(fd, addr as *const _, addrlen);
    if ret == SOCKET_ERROR as i32 {
        let err = winapi::um::winsock2::WSAGetLastError();
        if err == WSAEWOULDBLOCK as i32 {
            libc::set_errno(libc::EINPROGRESS);
        }
    }
    ret
}

#[cfg(not(target_os = "windows"))]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn rust_socketengine_connect(
    fd: c_int,
    addr: *const libc::sockaddr,
    addrlen: socklen_t,
) -> c_int {
    connect(fd, addr, addrlen)
}

#[cfg(target_os = "windows")]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn rust_socketengine_bind(
    fd: c_int,
    addr: *const sockaddr,
    addrlen: libc::c_int,
) -> c_int {
    bind(fd, addr as *const _, addrlen)
}

#[cfg(not(target_os = "windows"))]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn rust_socketengine_bind(
    fd: c_int,
    addr: *const libc::sockaddr,
    addrlen: socklen_t,
) -> c_int {
    bind(fd, addr, addrlen)
}

#[cfg(target_os = "windows")]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn rust_socketengine_shutdown(fd: c_int, how: c_int) -> c_int {
    shutdown(fd, how)
}

#[cfg(not(target_os = "windows"))]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn rust_socketengine_shutdown(fd: c_int, how: c_int) -> c_int {
    shutdown(fd, how)
}

#[cfg(target_os = "windows")]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn rust_socketengine_listen(fd: c_int, backlog: c_int) -> c_int {
    listen(fd, backlog)
}

#[cfg(not(target_os = "windows"))]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn rust_socketengine_listen(fd: c_int, backlog: c_int) -> c_int {
    listen(fd, backlog)
}

#[cfg(target_os = "windows")]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn rust_socketengine_accept(
    fd: c_int,
    addr: *mut sockaddr,
    addrlen: *mut libc::c_int,
) -> c_int {
    accept(fd, addr as *mut _, addrlen)
}

#[cfg(not(target_os = "windows"))]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn rust_socketengine_accept(
    fd: c_int,
    addr: *mut libc::sockaddr,
    addrlen: *mut socklen_t,
) -> c_int {
    accept(fd, addr, addrlen)
}

#[cfg(target_os = "windows")]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn rust_socketengine_recvfrom(
    fd: c_int,
    buf: *mut u8,
    len: usize,
    flags: c_int,
    from: *mut sockaddr,
    fromlen: *mut libc::c_int,
) -> ssize_t {
    winapi::um::winsock2::recvfrom(fd, buf as *mut _, len as i32, flags, from as *mut _, fromlen)
}

#[cfg(not(target_os = "windows"))]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn rust_socketengine_recvfrom(
    fd: c_int,
    buf: *mut u8,
    len: usize,
    flags: c_int,
    from: *mut libc::sockaddr,
    fromlen: *mut socklen_t,
) -> ssize_t {
    libc::recvfrom(fd, buf as *mut _, len, flags, from, fromlen)
}

#[cfg(target_os = "windows")]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn rust_socketengine_send(
    fd: c_int,
    buf: *const u8,
    len: usize,
    flags: c_int,
) -> ssize_t {
    winapi::um::winsock2::send(fd, buf as *const _, len as i32, flags)
}

#[cfg(not(target_os = "windows"))]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn rust_socketengine_send(
    fd: c_int,
    buf: *const u8,
    len: usize,
    flags: c_int,
) -> ssize_t {
    libc::send(fd, buf as *const _, len, flags)
}

#[cfg(target_os = "windows")]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn rust_socketengine_recv(
    fd: c_int,
    buf: *mut u8,
    len: usize,
    flags: c_int,
) -> ssize_t {
    winapi::um::winsock2::recv(fd, buf as *mut _, len as i32, flags)
}

#[cfg(not(target_os = "windows"))]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn rust_socketengine_recv(
    fd: c_int,
    buf: *mut u8,
    len: usize,
    flags: c_int,
) -> ssize_t {
    libc::recv(fd, buf as *mut _, len, flags)
}

#[cfg(target_os = "windows")]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn rust_socketengine_sendto(
    fd: c_int,
    buf: *const u8,
    len: usize,
    flags: c_int,
    to: *const sockaddr,
    tolen: libc::c_int,
) -> ssize_t {
    winapi::um::winsock2::sendto(fd, buf as *const _, len as i32, flags, to as *const _, tolen)
}

#[cfg(not(target_os = "windows"))]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn rust_socketengine_sendto(
    fd: c_int,
    buf: *const u8,
    len: usize,
    flags: c_int,
    to: *const libc::sockaddr,
    tolen: socklen_t,
) -> ssize_t {
    libc::sendto(fd, buf as *const _, len, flags, to, tolen)
}
