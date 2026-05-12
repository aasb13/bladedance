use tracing::info;

#[unsafe(no_mangle)]
pub unsafe extern "C" fn LogManager_Log(
    level: u8,
    ty: *const u8,
    ty_len: usize,
    msg: *const u8,
    msg_len: usize,
) {
    let ty_s = if ty.is_null() || ty_len == 0 {
        ""
    } else {
        unsafe { std::str::from_utf8_unchecked(std::slice::from_raw_parts(ty, ty_len)) }
    };

    let msg_s = if msg.is_null() || msg_len == 0 {
        ""
    } else {
        unsafe { std::str::from_utf8_unchecked(std::slice::from_raw_parts(msg, msg_len)) }
    };

    info!(level = level, ty = ty_s, "{}", msg_s);
}

