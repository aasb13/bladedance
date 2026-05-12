/*
 * InspIRCd -- Internet Relay Chat Daemon
 *
 *   Copyright (C) 2020-2023 Sadie Powell <sadie@witchery.services>
 *
 * This file is part of InspIRCd.  InspIRCd is free software: you can
 * redistribute it and/or modify it under the terms of the GNU General Public
 * License as published by the Free Software Foundation, version 2.
 */

#![allow(unsafe_op_in_unsafe_fn)]

use std::os::raw::{c_char, c_uchar};

unsafe extern "C" {
    fn channelmgr_ffi_max_channel_len() -> usize;
    fn channelmgr_ffi_channels_is_prefix(prefix: c_uchar) -> bool;
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn rust_channelmanager_default_is_channel(data: *const c_char, len: usize) -> bool {
    let channel = std::slice::from_raw_parts(data.cast::<u8>(), len);
    default_is_channel(channel)
}

fn default_is_channel(channel: &[u8]) -> bool {
    if channel.is_empty() || channel.len() > unsafe { channelmgr_ffi_max_channel_len() } {
        return false;
    }

    if !unsafe { channelmgr_ffi_channels_is_prefix(channel[0]) } {
        return false;
    }

    for chr in channel.iter().skip(1).copied() {
        match chr {
            0x07 | // BELL
            0x20 | // SPACE
            0x2C => // COMMA
                return false,
            _ => {}
        }
    }

    true
}
