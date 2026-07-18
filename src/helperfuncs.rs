// This file is a Rust port of a corresponding InspIRCd module.
// Original work Copyright (C) the InspIRCd contributors.
// Licensed under GPLv2. See LICENSE for details.

use std::collections::HashMap;
use std::ffi::{c_char, CStr, CString};
use std::os::raw::c_int;

const SECONDS_PER_MINUTE: u64 = 60;
const SECONDS_PER_HOUR: u64 = SECONDS_PER_MINUTE * 60;
const SECONDS_PER_DAY: u64 = SECONDS_PER_HOUR * 24;
const SECONDS_PER_WEEK: u64 = SECONDS_PER_DAY * 7;
const SECONDS_PER_YEAR: u64 = SECONDS_PER_DAY * 365;
const SECONDS_PER_AVG_YEAR: u64 = SECONDS_PER_YEAR + (SECONDS_PER_HOUR * 6);

/// A lookup table of values for multiplier characters used by
/// Duration::{try_from,from}(). In this lookup table, the indexes for
/// the ascii values 'm' and 'M' have the value '60', the indexes
/// for the ascii values 'D' and 'd' have a value of '86400', etc.
static DURATION_MULTI: [u64; 256] = {
    let mut table = [0u64; 256];
    table[b'D' as usize] = SECONDS_PER_DAY;
    table[b'H' as usize] = SECONDS_PER_HOUR;
    table[b'M' as usize] = SECONDS_PER_MINUTE;
    table[b'S' as usize] = 1;
    table[b'W' as usize] = SECONDS_PER_WEEK;
    table[b'Y' as usize] = SECONDS_PER_AVG_YEAR;
    table[b'd' as usize] = SECONDS_PER_DAY;
    table[b'h' as usize] = SECONDS_PER_HOUR;
    table[b'm' as usize] = SECONDS_PER_MINUTE;
    table[b's' as usize] = 1;
    table[b'w' as usize] = SECONDS_PER_WEEK;
    table[b'y' as usize] = SECONDS_PER_AVG_YEAR;
    table
};

pub struct Duration;

impl Duration {
    /// Attempts to parse a duration string into seconds.
    /// Returns Ok(seconds) on success, Err on failure.
    pub fn try_from_str(str: &str) -> Result<u64, ()> {
        let mut total: u64 = 0;
        let mut subtotal: u64 = 0;

        for chr in str.bytes() {
            if chr >= b'0' && chr <= b'9' {
                subtotal = subtotal * 10 + (chr - b'0') as u64;
            } else {
                let multiplier = DURATION_MULTI[chr as usize];
                if multiplier == 0 {
                    return Err(());
                }
                total += subtotal * multiplier;
                subtotal = 0;
            }
        }
        
        // Any trailing values built up are treated as raw seconds
        Ok(total + subtotal)
    }

    /// Parses a duration string into seconds.
    /// Returns 0 on failure.
    pub fn from_str(str: &str) -> u64 {
        Self::try_from_str(str).unwrap_or(0)
    }

    /// Checks if a duration string is valid.
    pub fn is_valid(duration: &str) -> bool {
        for c in duration.bytes() {
            if c >= b'0' && c <= b'9' {
                continue;
            }
            if DURATION_MULTI[c as usize] == 0 {
                return false;
            }
        }
        true
    }

    /// Converts seconds to a short duration string (e.g., "1y2w3d").
    pub fn to_string(mut duration: u64) -> String {
        if duration == 0 {
            return "0s".to_string();
        }

        let mut ret = String::new();

        let years = duration / SECONDS_PER_YEAR;
        if years > 0 {
            ret.push_str(&format!("{}y", years));
            duration -= years * SECONDS_PER_YEAR;
        }

        let weeks = duration / SECONDS_PER_WEEK;
        if weeks > 0 {
            ret.push_str(&format!("{}w", weeks));
            duration -= weeks * SECONDS_PER_WEEK;
        }

        let days = duration / SECONDS_PER_DAY;
        if days > 0 {
            ret.push_str(&format!("{}d", days));
            duration -= days * SECONDS_PER_DAY;
        }

        let hours = duration / SECONDS_PER_HOUR;
        if hours > 0 {
            ret.push_str(&format!("{}h", hours));
            duration -= hours * SECONDS_PER_HOUR;
        }

        let minutes = duration / SECONDS_PER_MINUTE;
        if minutes > 0 {
            ret.push_str(&format!("{}m", minutes));
            duration -= minutes * SECONDS_PER_MINUTE;
        }

        if duration > 0 {
            ret.push_str(&format!("{}s", duration));
        }

        ret
    }

    /// Converts seconds to a long human-readable duration string (e.g., "1 year, 2 weeks and 3 days").
    pub fn to_long_string(mut duration: u64, brief: bool) -> String {
        if duration == 0 {
            return "0 seconds".to_string();
        }

        if brief {
            // In order to get a shorter result we round to the nearest period.
            if duration >= SECONDS_PER_YEAR {
                duration = Self::nearest(duration, SECONDS_PER_DAY);
            } else if duration >= SECONDS_PER_DAY {
                duration = Self::nearest(duration, SECONDS_PER_HOUR);
            } else if duration >= SECONDS_PER_HOUR {
                duration = Self::nearest(duration, SECONDS_PER_MINUTE);
            }
        }

        let mut ret = String::new();

        let years = duration / SECONDS_PER_YEAR;
        if years > 0 {
            ret.push_str(&format!("{} {}", years, if years == 1 { "year" } else { "years" }));
            duration -= years * SECONDS_PER_YEAR;
        }

        let weeks = duration / SECONDS_PER_WEEK;
        if weeks > 0 {
            if !ret.is_empty() {
                ret.push_str(", ");
            }
            ret.push_str(&format!("{} {}", weeks, if weeks == 1 { "week" } else { "weeks" }));
            duration -= weeks * SECONDS_PER_WEEK;
        }

        let days = duration / SECONDS_PER_DAY;
        if days > 0 {
            if !ret.is_empty() {
                ret.push_str(", ");
            }
            ret.push_str(&format!("{} {}", days, if days == 1 { "day" } else { "days" }));
            duration -= days * SECONDS_PER_DAY;
        }

        let hours = duration / SECONDS_PER_HOUR;
        if hours > 0 {
            if !ret.is_empty() {
                ret.push_str(", ");
            }
            ret.push_str(&format!("{} {}", hours, if hours == 1 { "hour" } else { "hours" }));
            duration -= hours * SECONDS_PER_HOUR;
        }

        let minutes = duration / SECONDS_PER_MINUTE;
        if minutes > 0 {
            if !ret.is_empty() {
                ret.push_str(", ");
            }
            ret.push_str(&format!("{} {}", minutes, if minutes == 1 { "minute" } else { "minutes" }));
            duration -= minutes * SECONDS_PER_MINUTE;
        }

        if duration > 0 {
            if !ret.is_empty() {
                ret.push_str(", ");
            }
            ret.push_str(&format!("{} {}", duration, if duration == 1 { "second" } else { "seconds" }));
        }

        // Replace last comma with "and" if there are multiple parts
        if let Some(last_comma) = ret.rfind(',') {
            let first_comma = ret.find(',');
            if first_comma == Some(last_comma) {
                // BEFORE: 1 minute, 2 seconds
                // AFTER:  1 minute and 2 seconds
                ret.replace_range(last_comma..=last_comma, " and");
            } else {
                // BEFORE: 1 hour, 2 minutes, 3 seconds
                // AFTER:  1 hour, 2 minutes, and 3 seconds
                ret.insert_str(last_comma + 1, " and");
            }
        }

        ret
    }

    fn nearest(seconds: u64, roundto: u64) -> u64 {
        if (seconds % roundto) <= (roundto / 2) {
            seconds - (seconds % roundto)
        } else {
            seconds - (seconds % roundto) + roundto
        }
    }
}

pub struct Time;

impl Time {
    /// Converts a timestamp to a string using the given format.
    /// If format is None, uses a default format.
    pub fn to_string(curtime: i64, format: Option<&str>, utc: bool) -> String {
        use chrono::{DateTime, Utc};
        
        let dt = if utc {
            DateTime::<Utc>::from_timestamp(curtime, 0)
                .unwrap_or_else(|| DateTime::<Utc>::from_timestamp(0, 0).unwrap())
        } else {
            // For local time, we'd need to use Local timezone
            // For now, use UTC as fallback
            DateTime::<Utc>::from_timestamp(curtime, 0)
                .unwrap_or_else(|| DateTime::<Utc>::from_timestamp(0, 0).unwrap())
        };

        let format_str = format.unwrap_or("%a %b %d %H:%M:%S %Y");
        dt.format(format_str).to_string()
    }
}

/// Strips IRC color codes and formatting from a string.
pub fn strip_color(line: &mut String) {
    let mut idx = 0;
    let bytes = line.as_bytes();
    let mut result = Vec::new();

    while idx < bytes.len() {
        match bytes[idx] {
            b'\x02' | // Bold
            b'\x1D' | // Italic
            b'\x11' | // Monospace
            b'\x16' | // Reverse
            b'\x1E' | // Strikethrough
            b'\x1F' | // Underline
            b'\x0F'   // Reset
            => {
                idx += 1;
            }
            b'\x03' => {
                // Color code
                let start = idx;
                idx += 1;
                while idx < bytes.len() && idx - start < 6 {
                    let chr = bytes[idx];
                    if chr != b',' && (chr < b'0' || chr > b'9') {
                        break;
                    }
                    idx += 1;
                }
            }
            b'\x04' => {
                // Hex color code
                let start = idx;
                idx += 1;
                while idx < bytes.len() && idx - start < 14 {
                    let chr = bytes[idx];
                    let is_hex = (chr >= b'0' && chr <= b'9') ||
                                 (chr >= b'A' && chr <= b'F') ||
                                 (chr >= b'a' && chr <= b'f');
                    if chr != b',' && !is_hex {
                        break;
                    }
                    idx += 1;
                }
            }
            _ => {
                result.push(bytes[idx]);
                idx += 1;
            }
        }
    }

    *line = String::from_utf8(result).unwrap_or_else(|_| line.clone());
}

/// Checks if a string is a valid Server ID (SID).
/// A valid SID is exactly 3 characters long, starts with a digit,
/// and the other two characters are uppercase letters (A-Z) or digits.
pub fn is_sid(sid: &str) -> bool {
    sid.len() == 3 &&
    sid.chars().next().map_or(false, |c| c >= '0' && c <= '9') &&
    sid.chars().nth(1).map_or(false, |c| (c >= 'A' && c <= 'Z') || (c >= '0' && c <= '9')) &&
    sid.chars().nth(2).map_or(false, |c| (c >= 'A' && c <= 'Z') || (c >= '0' && c <= '9'))
}

/// Checks if a character is a word character (alphanumeric, -, ., _).
/// This is used by the config parser.
pub fn is_wordchar(ch: u8) -> bool {
    ch.is_ascii_alphanumeric() || ch == b'-' || ch == b'.' || ch == b'_'
}

/// Default random number generator - fills output buffer with pseudo-random bytes.
/// This uses a simple Xorshift algorithm for demonstration.
pub fn default_gen_random(output: &mut [u8]) {
    use std::time::{SystemTime, UNIX_EPOCH};
    
    // Use current timestamp as seed
    let duration = SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default();
    let mut seed: u64 = duration.as_nanos() as u64;
    
    // Simple Xorshift64* algorithm
    for byte in output.iter_mut() {
        seed ^= seed >> 12;
        seed ^= seed << 25;
        seed ^= seed >> 27;
        *byte = (seed >> 33) as u8;
    }
}

/// Generates a random integer in the range [0, max).
/// Uses a simple hash-based approach for demonstration.
pub fn gen_random_int(max: u64) -> u64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    
    if max <= 1 {
        return 0;
    }
    
    let duration = SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default();
    let hash = duration.as_nanos() as u64;
    
    // Simple pseudo-random number in range [0, max)
    ((hash.wrapping_mul(6364136223846793005).wrapping_add(1)) % max)
}

/// Generates a random alphanumeric string of the specified length.
/// Uses characters: a-z, A-Z, 0-9
pub fn gen_random_str(length: usize) -> String {
    static CHARS: &[u8] = b"abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789";
    
    let mut result = String::with_capacity(length);
    for _ in 0..length {
        let idx = gen_random_int(CHARS.len() as u64) as usize;
        result.push(CHARS[idx] as char);
    }
    result
}

/// Checks if a string is a valid IRC nickname.
/// Valid nicknames:
/// - Must not be empty and must not exceed max_len
/// - First character must be in range 'A' to '}' (ASCII 65-125)
/// - Subsequent characters can be 'A'-'}', '0'-'9', or '-'
pub fn is_nick(nick: &str, max_len: usize) -> bool {
    if nick.is_empty() || nick.len() > max_len {
        return false;
    }
    
    let mut chars = nick.chars();
    
    // First character: must be in A-} (ASCII 65-125)
    if let Some(first) = chars.next() {
        if first < 'A' || first > '}' {
            return false;
        }
    } else {
        return false; // empty string
    }
    
    // Subsequent characters: can be A-}, 0-9, or -
    for chr in chars {
        if !((chr >= 'A' && chr <= '}') || (chr >= '0' && chr <= '9') || chr == '-') {
            return false;
        }
    }
    
    true
}

/// Checks if a string is a valid IRC username.
/// Valid usernames:
/// - Must not be empty and must not exceed max_len
/// - All characters must be in range 'A'-'}' or be '0'-'9', '-', or '.'
pub fn is_user(user: &str, max_len: usize) -> bool {
    if user.is_empty() || user.len() > max_len {
        return false;
    }
    
    for chr in user.chars() {
        if (chr >= 'A' && chr <= '}') || (chr >= '0' && chr <= '9') || chr == '-' || chr == '.' {
            continue;
        }
        return false;
    }
    
    true
}

/// Checks if a string is a valid IRC mask (nick!user@host format).
/// Valid masks:
/// - Must contain exactly one '!' and one '@'
/// - All characters must be printable ASCII (32-126)
/// - Must not exceed max_len
pub fn is_valid_mask(mask: &str, max_len: usize) -> bool {
    if mask.is_empty() || mask.len() > max_len {
        return false;
    }
    
    let mut exclamation = 0;
    let mut atsign = 0;
    
    for chr in mask.chars() {
        // Check for out of range character
        if chr < '\x20' || chr > '\x7e' {
            return false;
        }
        
        match chr {
            '!' => exclamation += 1,
            '@' => atsign += 1,
            _ => {}
        }
    }
    
    // Valid masks must have exactly one '!' and one '@'
    exclamation == 1 && atsign == 1
}

/// Checks if a string is a valid hostname.
/// Valid hostnames:
/// - Must not be empty and must not exceed max_len
/// - Must contain at least one dot (unless allowsimple is true)
/// - Labels separated by dots can contain alphanumeric chars and dashes
/// - Dashes cannot be at start/end of labels or consecutive
/// - Dots cannot be at start/end or consecutive
pub fn is_host(host: &str, max_len: usize, allowsimple: bool) -> bool {
    if host.is_empty() || host.len() > max_len {
        return false;
    }
    
    let bytes = host.as_bytes();
    let mut numdashes = 0;
    let mut numdots = 0;
    let mut seendot = false;
    let hostend = bytes.len() - 1;
    
    for (idx, &chr) in bytes.iter().enumerate() {
        // If the current character is a label separator (dot)
        if chr == b'.' {
            numdots += 1;
            
            // Consecutive separators are not allowed and dashes can not exist at the start or end
            // of labels and separators must only exist between labels.
            if seendot || numdashes > 0 || idx == 0 || idx == hostend {
                return false;
            }
            
            seendot = true;
            continue;
        }
        
        // If this point is reached then the character is not a dot.
        seendot = false;
        
        // If the current character is a dash
        if chr == b'-' {
            // Consecutive separators are not allowed and dashes can not exist at the start or end
            // of labels and separators must only exist between labels.
            if seendot || numdashes >= 2 || idx == 0 || idx == hostend {
                return false;
            }
            
            numdashes += 1;
            continue;
        }
        
        // If this point is reached then the character is not a dash.
        numdashes = 0;
        
        // Alphanumeric characters are allowed at any position.
        if !(chr.is_ascii_alphanumeric()) {
            return false;
        }
    }
    
    // Whilst simple hostnames (e.g. localhost) are valid we do not allow the server to use
    // them to prevent issues with clients that differentiate between short client and server
    // prefixes by checking whether the nickname contains a dot.
    numdots > 0 || allowsimple
}

/// Processes color escape sequences in a string.
pub fn process_colors(line: &mut String) {
    let formats: HashMap<char, &str> = [
        ('\\', "\\"),
        ('{', "{"),
        ('}', "}"),
        ('b', "\x02"),  // Bold
        ('c', "\x03"),  // Color
        ('h', "\x04"),  // Hex Color
        ('i', "\x1D"),  // Italic
        ('m', "\x11"),  // Monospace
        ('r', "\x16"),  // Reverse
        ('s', "\x1E"),  // Strikethrough
        ('u', "\x1F"),  // Underline
        ('x', "\x0F"),  // Reset
    ].iter().cloned().collect();

    let colors: HashMap<&str, u8> = [
        ("white", 0),
        ("black", 1),
        ("blue", 2),
        ("green", 3),
        ("red", 4),
        ("brown", 5),
        ("magenta", 6),
        ("orange", 7),
        ("yellow", 8),
        ("light green", 9),
        ("cyan", 10),
        ("light cyan", 11),
        ("light blue", 12),
        ("pink", 13),
        ("gray", 14),
        ("grey", 14),
        ("light gray", 15),
        ("light grey", 15),
        ("default", 99),
    ].iter().cloned().collect();

    let mut idx = 0;
    let bytes = line.as_bytes();
    let mut result = Vec::new();

    while idx < bytes.len() {
        if bytes[idx] != b'\\' {
            result.push(bytes[idx]);
            idx += 1;
            continue;
        }

        let start = idx;
        idx += 1;
        if idx >= bytes.len() {
            // Stray \ at the end of the string; skip
            continue;
        }

        let chr = bytes[idx] as char;
        if let Some(&replacement) = formats.get(&chr) {
            result.extend_from_slice(replacement.as_bytes());
            idx += 1;

            if chr != 'c' {
                continue;
            }

            // Only colors can have values
            if idx >= bytes.len() || bytes[idx] != b'{' {
                continue;
            }

            let fg_start = idx + 1;
            let fgend = match bytes[fg_start..].iter().position(|&c| c == b',' || c == b'}') {
                Some(pos) => fg_start + pos,
                None => {
                    // Malformed color value, strip
                    result.truncate(start);
                    break;
                }
            };

            let mut bgend = None;
            if bytes[fgend] == b',' {
                let bg_start = fgend + 1;
                if let Some(pos) = bytes[bg_start..].iter().position(|&c| c == b'}') {
                    bgend = Some(bg_start + pos);
                } else {
                    // Malformed color value, strip
                    result.truncate(start);
                    break;
                }
            }

            let fg_str = std::str::from_utf8(&bytes[fg_start..fgend]).unwrap_or("");
            let fg = *colors.get(fg_str).unwrap_or(&99);
            result.extend_from_slice(fg.to_string().as_bytes());

            if let Some(bg_pos) = bgend {
                result.push(b',');
                let bg_str = std::str::from_utf8(&bytes[fgend + 1..bg_pos]).unwrap_or("");
                let bg = *colors.get(bg_str).unwrap_or(&99);
                result.extend_from_slice(bg.to_string().as_bytes());
                idx = bg_pos + 1;
            } else {
                idx = fgend + 1;
            }
        } else {
            // Unknown escape, skip
            idx += 1;
        }
    }

    *line = String::from_utf8(result).unwrap_or_else(|_| line.clone());
}

// FFI exports for C++ interop

#[unsafe(no_mangle)]
pub extern "C" fn helperfuncs_duration_try_from(str: *const c_char, duration: *mut u64) -> c_int {
    if str.is_null() || duration.is_null() {
        return 0;
    }
    
    let c_str = unsafe { CStr::from_ptr(str) };
    let str_slice = match c_str.to_str() {
        Ok(s) => s,
        Err(_) => return 0,
    };
    
    match Duration::try_from_str(str_slice) {
        Ok(seconds) => {
            unsafe { *duration = seconds };
            1
        }
        Err(_) => 0,
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn helperfuncs_duration_from(str: *const c_char) -> u64 {
    if str.is_null() {
        return 0;
    }
    
    let c_str = unsafe { CStr::from_ptr(str) };
    let str_slice = match c_str.to_str() {
        Ok(s) => s,
        Err(_) => return 0,
    };
    
    Duration::from_str(str_slice)
}

#[unsafe(no_mangle)]
pub extern "C" fn helperfuncs_duration_is_valid(duration: *const c_char) -> c_int {
    if duration.is_null() {
        return 0;
    }
    
    let c_str = unsafe { CStr::from_ptr(duration) };
    let str_slice = match c_str.to_str() {
        Ok(s) => s,
        Err(_) => return 0,
    };
    
    if Duration::is_valid(str_slice) { 1 } else { 0 }
}

#[unsafe(no_mangle)]
pub extern "C" fn helperfuncs_duration_to_string(duration: u64) -> *mut c_char {
    let result = Duration::to_string(duration);
    CString::new(result).map(|s| s.into_raw()).unwrap_or(std::ptr::null_mut())
}

#[unsafe(no_mangle)]
pub extern "C" fn helperfuncs_duration_to_long_string(duration: u64, brief: c_int) -> *mut c_char {
    let result = Duration::to_long_string(duration, brief != 0);
    CString::new(result).map(|s| s.into_raw()).unwrap_or(std::ptr::null_mut())
}

#[unsafe(no_mangle)]
pub extern "C" fn helperfuncs_time_to_string(curtime: i64, format: *const c_char, utc: c_int) -> *mut c_char {
    let format_str = if format.is_null() {
        None
    } else {
        let c_str = unsafe { CStr::from_ptr(format) };
        match c_str.to_str() {
            Ok(s) => Some(s),
            Err(_) => None,
        }
    };
    
    let result = Time::to_string(curtime, format_str, utc != 0);
    CString::new(result).map(|s| s.into_raw()).unwrap_or(std::ptr::null_mut())
}

#[unsafe(no_mangle)]
pub extern "C" fn helperfuncs_strip_color(line: *mut c_char) {
    if line.is_null() {
        return;
    }
    
    let c_str = unsafe { CStr::from_ptr(line) };
    let mut string = c_str.to_string_lossy().into_owned();
    strip_color(&mut string);
    
    // Copy back to the C string
    let bytes = string.as_bytes();
    unsafe {
        std::ptr::copy_nonoverlapping(bytes.as_ptr(), line as *mut u8, bytes.len());
        *(line.add(bytes.len())) = 0;
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn helperfuncs_process_colors(line: *mut c_char) {
    if line.is_null() {
        return;
    }
    
    let c_str = unsafe { CStr::from_ptr(line) };
    let mut string = c_str.to_string_lossy().into_owned();
    process_colors(&mut string);
    
    // Copy back to the C string
    let bytes = string.as_bytes();
    unsafe {
        std::ptr::copy_nonoverlapping(bytes.as_ptr(), line as *mut u8, bytes.len());
        *(line.add(bytes.len())) = 0;
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn helperfuncs_free_string(ptr: *mut c_char) {
    if !ptr.is_null() {
        unsafe {
            let _ = CString::from_raw(ptr);
        }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn helperfuncs_is_sid(sid: *const c_char) -> c_int {
    if sid.is_null() {
        return 0;
    }
    
    let c_str = unsafe { CStr::from_ptr(sid) };
    let str_slice = match c_str.to_str() {
        Ok(s) => s,
        Err(_) => return 0,
    };
    
    if is_sid(str_slice) { 1 } else { 0 }
}

#[unsafe(no_mangle)]
pub extern "C" fn helperfuncs_is_nick(nick: *const c_char, max_len: usize) -> c_int {
    if nick.is_null() {
        return 0;
    }
    
    let c_str = unsafe { CStr::from_ptr(nick) };
    let str_slice = match c_str.to_str() {
        Ok(s) => s,
        Err(_) => return 0,
    };
    
    if is_nick(str_slice, max_len) { 1 } else { 0 }
}

#[unsafe(no_mangle)]
pub extern "C" fn helperfuncs_is_user(user: *const c_char, max_len: usize) -> c_int {
    if user.is_null() {
        return 0;
    }
    
    let c_str = unsafe { CStr::from_ptr(user) };
    let str_slice = match c_str.to_str() {
        Ok(s) => s,
        Err(_) => return 0,
    };
    
    if is_user(str_slice, max_len) { 1 } else { 0 }
}

#[unsafe(no_mangle)]
pub extern "C" fn helperfuncs_is_valid_mask(mask: *const c_char, max_len: usize) -> c_int {
    if mask.is_null() {
        return 0;
    }
    
    let c_str = unsafe { CStr::from_ptr(mask) };
    let str_slice = match c_str.to_str() {
        Ok(s) => s,
        Err(_) => return 0,
    };
    
    if is_valid_mask(str_slice, max_len) { 1 } else { 0 }
}

#[unsafe(no_mangle)]
pub extern "C" fn helperfuncs_is_host(host: *const c_char, max_len: usize, allowsimple: c_int) -> c_int {
    if host.is_null() {
        return 0;
    }
    
    let c_str = unsafe { CStr::from_ptr(host) };
    let str_slice = match c_str.to_str() {
        Ok(s) => s,
        Err(_) => return 0,
    };
    
    if is_host(str_slice, max_len, allowsimple != 0) { 1 } else { 0 }
}

#[unsafe(no_mangle)]
pub extern "C" fn helperfuncs_is_wordchar(ch: c_int) -> c_int {
    if ch < 0 || ch > 255 {
        return 0;
    }
    if is_wordchar(ch as u8) { 1 } else { 0 }
}

#[unsafe(no_mangle)]
pub extern "C" fn helperfuncs_gen_random_int(max: u64) -> u64 {
    gen_random_int(max)
}

#[unsafe(no_mangle)]
pub extern "C" fn helperfuncs_gen_random_str(length: usize) -> *mut c_char {
    let result = gen_random_str(length);
    CString::new(result).map(|s| s.into_raw()).unwrap_or(std::ptr::null_mut())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_duration_from_str() {
        assert_eq!(Duration::from_str("60"), 60);
        assert_eq!(Duration::from_str("1m"), 60);
        assert_eq!(Duration::from_str("1h"), 3600);
        assert_eq!(Duration::from_str("1d"), 86400);
        assert_eq!(Duration::from_str("1w"), 604800);
        assert_eq!(Duration::from_str("1y"), 31557600);
    }

    #[test]
    fn test_duration_is_valid() {
        assert!(Duration::is_valid("60"));
        assert!(Duration::is_valid("1m"));
        assert!(Duration::is_valid("1h30m"));
        assert!(!Duration::is_valid("1x"));
    }

    #[test]
    fn test_duration_to_string() {
        assert_eq!(Duration::to_string(0), "0s");
        assert_eq!(Duration::to_string(60), "1m");
        assert_eq!(Duration::to_string(3661), "1h1m1s");
    }

    #[test]
    fn test_strip_color() {
        let mut s = String::from("\x02bold\x0F");
        strip_color(&mut s);
        assert_eq!(s, "bold");
    }

    #[test]
    fn test_process_colors() {
        let mut s = String::from(r"\bbold");
        process_colors(&mut s);
        assert_eq!(s, "\x02bold");
    }

    #[test]
    fn test_is_sid() {
        // Valid SIDs: 3 chars, first is digit, rest are uppercase or digits
        assert!(is_sid("0AA"));
        assert!(is_sid("1AB"));
        assert!(is_sid("9ZZ"));
        assert!(is_sid("0A0"));
        assert!(is_sid("123"));
        assert!(is_sid("9Z9"));
        
        // Invalid SIDs: wrong length
        assert!(!is_sid(""));
        assert!(!is_sid("A"));
        assert!(!is_sid("AB"));
        assert!(!is_sid("ABCD"));
        
        // Invalid SIDs: first char not a digit
        assert!(!is_sid("AAA"));
        assert!(!is_sid("BAA"));
        assert!(!is_sid("ZAA"));
        
        // Invalid SIDs: contains lowercase
        assert!(!is_sid("0aA"));
        assert!(!is_sid("0Ab"));
        
        // Invalid SIDs: contains special characters
        assert!(!is_sid("0A-"));
        assert!(!is_sid("0A_"));
    }

    #[test]
    fn test_is_nick() {
        // Valid nicknames with max_len = 30
        // First char must be A-}, subsequent can be A-}, 0-9, or -
        assert!(is_nick("TestUser", 30));
        assert!(is_nick("user123", 30)); // 'u' is in A-} range
        assert!(is_nick("Test-User", 30));
        assert!(is_nick("a", 30)); // 'a' is in A-} range (97)
        assert!(is_nick("A", 30));
        assert!(is_nick("Test_", 30)); // '_' is in A-} range (95)
        assert!(is_nick("Test}", 30)); // '}' is the upper bound (125)
        assert!(is_nick("t", 30)); // lowercase 't' (116) is in A-} range
        
        // Invalid nicknames: empty
        assert!(!is_nick("", 30));
        
        // Invalid nicknames: too long
        assert!(!is_nick("ThisNicknameIsWayTooLongForTheLimit", 30));
        
        // Invalid nicknames: first char out of range
        assert!(!is_nick(" test", 30)); // space (32) is before A (65)
        assert!(!is_nick("~test", 30)); // ~ (126) is after } (125)
        assert!(!is_nick("\x1Ftest", 30)); // control char before A
        
        // Invalid nicknames: first char is digit (digits only allowed after first char)
        assert!(!is_nick("1test", 30));
        assert!(!is_nick("9user", 30));
        
        // Invalid nicknames: first char is dash (dashes only allowed after first char)
        assert!(!is_nick("-test", 30));
        
        // Invalid nicknames: special chars not in allowed ranges
        assert!(!is_nick("Test@User", 30)); // @ (64) is before A (65)
        assert!(!is_nick("Test User", 30)); // space (32) not allowed
        assert!(!is_nick("Test(User", 30)); // ( (40) not allowed
    }

    #[test]
    fn test_is_user() {
        // Valid usernames with max_len = 30
        assert!(is_user("testuser", 30));
        assert!(is_user("user123", 30));
        assert!(is_user("user-name", 30));
        assert!(is_user("user.name", 30));
        assert!(is_user("a", 30));
        
        // Invalid usernames: empty
        assert!(!is_user("", 30));
        
        // Invalid usernames: too long
        assert!(!is_user("ThisUsernameIsWayTooLongForTheLimit", 30));
        
        // Invalid usernames: contains disallowed characters
        assert!(!is_user("user@domain", 30)); // @ not allowed
        assert!(!is_user("user name", 30)); // space not allowed
        assert!(!is_user("user!name", 30)); // ! not allowed
    }

    #[test]
    fn test_is_valid_mask() {
        // Valid masks with max_len = 100
        assert!(is_valid_mask("nick!user@host", 100));
        assert!(is_valid_mask("test!testuser@testhost", 100));
        
        // Invalid masks: empty
        assert!(!is_valid_mask("", 100));
        
        // Invalid masks: too long
        assert!(!is_valid_mask("a!b@" + &"c".repeat(100), 10));
        
        // Invalid masks: no exclamation mark
        assert!(!is_valid_mask("nickuser@host", 100));
        
        // Invalid masks: no at sign
        assert!(!is_valid_mask("nick!userhost", 100));
        
        // Invalid masks: multiple exclamation marks
        assert!(!is_valid_mask("nick!!user@host", 100));
        
        // Invalid masks: multiple at signs
        assert!(!is_valid_mask("nick!user@host@domain", 100));
        
        // Invalid masks: contains control characters
        assert!(!is_valid_mask("nick\x01user@host", 100)); // SOH character
        assert!(!is_valid_mask("nick!user\x7fhost", 100)); // DEL character
    }

    #[test]
    fn test_is_host() {
        // Valid hostnames with allowsimple = false
        assert!(is_host("example.com", 100, false));
        assert!(is_host("sub.example.com", 100, false));
        assert!(is_host("test-host.example.com", 100, false));
        assert!(is_host("localhost", 100, true)); // allowsimple = true
        
        // Valid hostnames with dashes
        assert!(is_host("my-host.example.com", 100, false));
        
        // Invalid hostnames: empty
        assert!(!is_host("", 100, false));
        
        // Invalid hostnames: too long
        assert!(!is_host(&"a".repeat(101), 100, false));
        
        // Invalid hostnames: no dot and allowsimple = false
        assert!(!is_host("localhost", 100, false));
        
        // Invalid hostnames: starts with dot
        assert!(!is_host(".example.com", 100, false));
        
        // Invalid hostnames: ends with dot
        assert!(!is_host("example.com.", 100, false));
        
        // Invalid hostnames: consecutive dots
        assert!(!is_host("example..com", 100, false));
        
        // Invalid hostnames: starts with dash
        assert!(!is_host("-example.com", 100, false));
        
        // Invalid hostnames: ends with dash
        assert!(!is_host("example-.com", 100, false));
        
        // Invalid hostnames: consecutive dashes
        assert!(!is_host("example--host.com", 100, false));
        
        // Invalid hostnames: special characters not allowed
        assert!(!is_host("example_host.com", 100, false)); // underscore
        assert!(!is_host("example@host.com", 100, false)); // at sign
    }

    #[test]
    fn test_is_wordchar() {
        // Alphanumeric
        assert!(is_wordchar(b'a'));
        assert!(is_wordchar(b'Z'));
        assert!(is_wordchar(b'0'));
        assert!(is_wordchar(b'9'));
        
        // Special allowed chars
        assert!(is_wordchar(b'-'));
        assert!(is_wordchar(b'.'));
        assert!(is_wordchar(b'_'));
        
        // Not allowed
        assert!(!is_wordchar(b' '));
        assert!(!is_wordchar(b'@'));
        assert!(!is_wordchar(b'!'));
    }

    #[test]
    fn test_gen_random_int() {
        // Test that it returns a value in the correct range
        let result = gen_random_int(100);
        assert!(result < 100);
        
        // Test edge cases
        assert_eq!(gen_random_int(0), 0);
        assert_eq!(gen_random_int(1), 0);
    }

    #[test]
    fn test_gen_random_str() {
        // Test that it returns a string of the correct length
        let result = gen_random_str(10);
        assert_eq!(result.len(), 10);
        
        // Test that all characters are alphanumeric
        for ch in result.chars() {
            assert!(ch.is_ascii_alphanumeric());
        }
        
        // Test edge case
        assert_eq!(gen_random_str(0).len(), 0);
    }
}
