// This file is a Rust port of a corresponding InspIRCd module.
// Original work Copyright (C) the InspIRCd contributors.
// Licensed under GPLv2. See LICENSE for details.

#[unsafe(no_mangle)]
pub extern "C" fn rust_CanCreateSCTPSocket() -> bool {
    #[cfg(unix)]
    {
        unsafe {
            let fd = libc::socket(
                libc::AF_INET,
                libc::SOCK_STREAM,
                libc::IPPROTO_SCTP,
            );
            
            if fd >= 0 {
                libc::close(fd);
                true
            } else {
                false
            }
        }
    }
    
    #[cfg(not(unix))]
    {
        false
    }
}