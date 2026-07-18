// This file is a Rust port of a corresponding InspIRCd module.
// Original work Copyright (C) the InspIRCd contributors.
// Licensed under GPLv2. See LICENSE for details.

use std::ffi::{c_char, c_int, c_longlong, c_ulonglong, c_uchar, CStr};

// Duration constants
const SECONDS_PER_MINUTE: u64 = 60;
const SECONDS_PER_HOUR: u64 = SECONDS_PER_MINUTE * 60;
const SECONDS_PER_DAY: u64 = SECONDS_PER_HOUR * 24;
const SECONDS_PER_WEEK: u64 = SECONDS_PER_DAY * 7;
const SECONDS_PER_YEAR: u64 = SECONDS_PER_DAY * 365;
const SECONDS_PER_AVG_YEAR: u64 = SECONDS_PER_YEAR + (SECONDS_PER_HOUR * 6);

/// A lookup table for duration multiplier characters
static DURATION_MULTIPLIERS: [u64; 256] = {
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

/// Checks if a string represents a boolean "yes" value
/// Valid yes values: "yes", "true", "on" (case-insensitive)
pub fn is_yes(value: &str) -> bool {
    match value.to_lowercase().as_str() {
        "yes" | "true" | "on" => true,
        _ => false,
    }
}

/// Checks if a string represents a boolean "no" value
/// Valid no values: "no", "false", "off" (case-insensitive)
pub fn is_no(value: &str) -> bool {
    match value.to_lowercase().as_str() {
        "no" | "false" | "off" => true,
        _ => false,
    }
}

/// Parses a boolean from a string, returning the default if not valid
pub fn parse_bool(value: &str, default: bool) -> bool {
    if value.is_empty() {
        return default;
    }
    if is_yes(value) {
        return true;
    }
    if is_no(value) {
        return false;
    }
    // Not a valid boolean, return default
    default
}

/// Magnitude multipliers for numeric parsing
/// K/k = kilo (1024), M/m = mega (1024*1024), G/g = giga (1024*1024*1024)
const MAGNITUDE_MULTIPLIERS: [u64; 256] = {
    let mut table = [0u64; 256];
    table[b'K' as usize] = 1024;
    table[b'M' as usize] = 1024 * 1024;
    table[b'G' as usize] = 1024 * 1024 * 1024;
    table[b'k' as usize] = 1024;
    table[b'm' as usize] = 1024 * 1024;
    table[b'g' as usize] = 1024 * 1024 * 1024;
    table
};

/// Parses a signed integer from a string with optional magnitude suffix
/// Returns (value, tail_position) where tail_position points to the character after the number
pub fn parse_sint_with_tail(str: &str) -> (i64, usize) {
    let bytes = str.as_bytes();
    let mut idx = 0;
    let mut negative = false;
    let mut value: i64 = 0;

    // Skip whitespace
    while idx < bytes.len() && bytes[idx].is_ascii_whitespace() {
        idx += 1;
    }

    // Handle sign
    if idx < bytes.len() {
        match bytes[idx] {
            b'+' => { idx += 1; }
            b'-' => { negative = true; idx += 1; }
            _ => {}
        }
    }

    // Parse digits
    let start_idx = idx;
    while idx < bytes.len() && bytes[idx].is_ascii_digit() {
        let digit = (bytes[idx] - b'0') as i64;
        // Check for overflow
        if value > (i64::MAX - digit) / 10 {
            // Overflow - return what we have
            break;
        }
        value = value * 10 + digit;
        idx += 1;
    }

    if negative {
        value = -value;
    }

    // Apply magnitude if present
    if idx < bytes.len() {
        let multiplier = MAGNITUDE_MULTIPLIERS[bytes[idx] as usize];
        if multiplier > 0 {
            // Check for overflow before multiplying
            if value > 0 && value > (i64::MAX / multiplier as i64) {
                // Would overflow
                return (value, idx);
            }
            if value < 0 && value < (i64::MIN / multiplier as i64) {
                // Would underflow
                return (value, idx);
            }
            value = value.wrapping_mul(multiplier as i64);
            idx += 1;
        }
    }

    (value, idx)
}

/// Parses an unsigned integer from a string with optional magnitude suffix
/// Returns (value, tail_position) where tail_position points to the character after the number
pub fn parse_uint_with_tail(str: &str) -> (u64, usize) {
    let bytes = str.as_bytes();
    let mut idx = 0;
    let mut value: u64 = 0;

    // Skip whitespace
    while idx < bytes.len() && bytes[idx].is_ascii_whitespace() {
        idx += 1;
    }

    // Parse digits
    let start_idx = idx;
    while idx < bytes.len() && bytes[idx].is_ascii_digit() {
        let digit = (bytes[idx] - b'0') as u64;
        // Check for overflow
        if value > (u64::MAX - digit) / 10 {
            // Overflow - return what we have
            break;
        }
        value = value * 10 + digit;
        idx += 1;
    }

    // Apply magnitude if present
    if idx < bytes.len() {
        let multiplier = MAGNITUDE_MULTIPLIERS[bytes[idx] as usize];
        if multiplier > 0 {
            // Check for overflow before multiplying
            if value > (u64::MAX / multiplier) {
                // Would overflow
                return (value, idx);
            }
            value = value.wrapping_mul(multiplier);
            idx += 1;
        }
    }

    (value, idx)
}

/// Parses a float from a string
/// Returns the value and the tail position
pub fn parse_float_with_tail(str: &str) -> (f64, usize) {
    let bytes = str.as_bytes();
    let mut idx = 0;
    let mut negative = false;
    let mut value: f64 = 0.0;
    let mut has_digits = false;
    let mut in_fraction = false;
    let mut fraction_divisor: f64 = 1.0;

    // Skip whitespace
    while idx < bytes.len() && bytes[idx].is_ascii_whitespace() {
        idx += 1;
    }

    // Handle sign
    if idx < bytes.len() {
        match bytes[idx] {
            b'+' => { idx += 1; }
            b'-' => { negative = true; idx += 1; }
            _ => {}
        }
    }

    // Parse integer part
    while idx < bytes.len() && bytes[idx].is_ascii_digit() {
        let digit = (bytes[idx] - b'0') as f64;
        value = value * 10.0 + digit;
        has_digits = true;
        idx += 1;
    }

    // Parse fraction part
    if idx < bytes.len() && bytes[idx] == b'.' {
        idx += 1;
        in_fraction = true;
        fraction_divisor = 10.0;
        while idx < bytes.len() && bytes[idx].is_ascii_digit() {
            let digit = (bytes[idx] - b'0') as f64;
            value = value + digit / fraction_divisor;
            has_digits = true;
            fraction_divisor *= 10.0;
            idx += 1;
        }
    }

    // Parse exponent
    if idx < bytes.len() && (bytes[idx] == b'e' || bytes[idx] == b'E') {
        idx += 1;
        let mut exponent_negative = false;
        if idx < bytes.len() && bytes[idx] == b'-' {
            exponent_negative = true;
            idx += 1;
        } else if idx < bytes.len() && bytes[idx] == b'+' {
            idx += 1;
        }

        let mut exponent: i32 = 0;
        while idx < bytes.len() && bytes[idx].is_ascii_digit() {
            let digit = (bytes[idx] - b'0') as i32;
            exponent = exponent * 10 + digit;
            idx += 1;
        }

        if exponent_negative {
            exponent = -exponent;
        }

        value *= 10_f64.powi(exponent);
    }

    // Apply magnitude if present (K, M, G)
    if idx < bytes.len() {
        let multiplier = MAGNITUDE_MULTIPLIERS[bytes[idx] as usize];
        if multiplier > 0 {
            value *= multiplier as f64;
            idx += 1;
        }
    }

    if negative {
        value = -value;
    }

    (value, idx)
}

/// Parses a character from a string, returns default if string length != 1
pub fn parse_character(value: &str, default: u8, empty_nul: bool) -> u8 {
    if value.is_empty() {
        return if empty_nul { 0 } else { default };
    }
    if value.len() == 1 {
        value.as_bytes()[0]
    } else {
        default
    }
}

/// Clamps a value to a range
pub fn clamp_sint(value: i64, min: i64, max: i64, default: i64) -> i64 {
    if value < min || value > max {
        default
    } else {
        value
    }
}

/// Clamps an unsigned value to a range
pub fn clamp_uint(value: u64, min: u64, max: u64, default: u64) -> u64 {
    if value < min || value > max {
        default
    } else {
        value
    }
}

/// Clamps a float value to a range
pub fn clamp_float(value: f64, min: f64, max: f64, default: f64) -> f64 {
    if value < min || value > max || !value.is_finite() {
        default
    } else {
        value
    }
}

/// Validates that a string length is within bounds
pub fn validate_string_length(value: &str, minlen: usize, maxlen: usize) -> bool {
    let len = value.len();
    len >= minlen && len <= maxlen
}

/// Parses a duration string (e.g., "1y2w3d4h5m6s") into seconds
/// Returns the duration in seconds, or None if parsing failed
pub fn parse_duration(str: &str) -> Option<u64> {
    let mut total: u64 = 0;
    let mut subtotal: u64 = 0;

    for chr in str.bytes() {
        if chr >= b'0' && chr <= b'9' {
            // Check for overflow
            let digit = (chr - b'0') as u64;
            if subtotal > (u64::MAX - digit) / 10 {
                return None;
            }
            subtotal = subtotal * 10 + digit;
        } else {
            let multiplier = DURATION_MULTIPLIERS[chr as usize];
            if multiplier == 0 {
                return None;
            }
            // Check for overflow before multiplying
            if subtotal > 0 && multiplier > 0 && subtotal > (u64::MAX / multiplier) {
                return None;
            }
            total = total.wrapping_add(subtotal * multiplier);
            subtotal = 0;
        }
    }
    
    // Add any trailing numeric value as seconds
    if subtotal > 0 {
        if total > (u64::MAX - subtotal) {
            return None;
        }
        total = total.wrapping_add(subtotal);
    }
    
    Some(total)
}

/// Clamps a duration value to a range
pub fn clamp_duration(value: u64, min: u64, max: u64, default: u64) -> u64 {
    if value < min || value > max {
        default
    } else {
        value
    }
}

// FFI exports for C++ interop

#[unsafe(no_mangle)]
pub extern "C" fn configreader_parse_bool(value: *const c_char, default: c_int, result: *mut c_int) -> c_int {
    if value.is_null() || result.is_null() {
        return 0;
    }

    let c_str = unsafe { CStr::from_ptr(value) };
    let str_slice = match c_str.to_str() {
        Ok(s) => s,
        Err(_) => {
            unsafe { *result = default; }
            return 0;
        }
    };

    let parsed = parse_bool(str_slice, default != 0);
    unsafe { *result = parsed as c_int; }
    
    // Return 1 if valid boolean, 0 if using default
    if str_slice.is_empty() {
        1  // Empty string is valid (returns default)
    } else if is_yes(str_slice) || is_no(str_slice) {
        1  // Valid boolean
    } else {
        0  // Invalid boolean, using default
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn configreader_parse_sint(
    value: *const c_char,
    def: c_longlong,
    min: c_longlong,
    max: c_longlong,
    result: *mut c_longlong,
    tail: *mut usize,
) -> c_int {
    if value.is_null() || result.is_null() {
        return 0;
    }

    let c_str = unsafe { CStr::from_ptr(value) };
    let str_slice = match c_str.to_str() {
        Ok(s) => s,
        Err(_) => {
            unsafe { *result = def; }
            return 0;
        }
    };

    let (parsed, idx) = parse_sint_with_tail(str_slice);

    // Check if we parsed anything
    if str_slice.is_empty() || (str_slice.bytes().all(|b| b.is_ascii_whitespace()) && def != 0) {
        unsafe { *result = def; }
        return 0;
    }

    // Clamp to range
    let final_value = clamp_sint(parsed, min, max, def);

    unsafe {
        *result = final_value;
        if !tail.is_null() {
            *tail = idx;
        }
    }

    1
}

#[unsafe(no_mangle)]
pub extern "C" fn configreader_parse_uint(
    value: *const c_char,
    def: c_ulonglong,
    min: c_ulonglong,
    max: c_ulonglong,
    result: *mut c_ulonglong,
    tail: *mut usize,
) -> c_int {
    if value.is_null() || result.is_null() {
        return 0;
    }

    let c_str = unsafe { CStr::from_ptr(value) };
    let str_slice = match c_str.to_str() {
        Ok(s) => s,
        Err(_) => {
            unsafe { *result = def; }
            return 0;
        }
    };

    let (parsed, idx) = parse_uint_with_tail(str_slice);

    // Check if we parsed anything
    if str_slice.is_empty() || (str_slice.bytes().all(|b| b.is_ascii_whitespace()) && def != 0) {
        unsafe { *result = def; }
        return 0;
    }

    // Clamp to range
    let final_value = clamp_uint(parsed, min, max, def);

    unsafe {
        *result = final_value;
        if !tail.is_null() {
            *tail = idx;
        }
    }

    1
}

#[unsafe(no_mangle)]
pub extern "C" fn configreader_parse_float(
    value: *const c_char,
    def: c_longlong,
    min: c_longlong,
    max: c_longlong,
    result: *mut c_longlong,
    tail: *mut usize,
) -> c_int {
    if value.is_null() || result.is_null() {
        return 0;
    }

    let c_str = unsafe { CStr::from_ptr(value) };
    let str_slice = match c_str.to_str() {
        Ok(s) => s,
        Err(_) => {
            unsafe { *result = def; }
            return 0;
        }
    };

    let (parsed, idx) = parse_float_with_tail(str_slice);

    // Check if we parsed anything
    if str_slice.is_empty() || (str_slice.bytes().all(|b| b.is_ascii_whitespace()) && def != 0) {
        unsafe { *result = def; }
        return 0;
    }

    // Convert to integer representation (long double -> int64_t)
    // This matches the C++ behavior which stores as long double but casts to int64_t
    let int_value = parsed as i64;

    // Clamp to range
    let final_value = clamp_sint(int_value, min, max, def);

    unsafe {
        *result = final_value;
        if !tail.is_null() {
            *tail = idx;
        }
    }

    1
}

#[unsafe(no_mangle)]
pub extern "C" fn configreader_parse_character(
    value: *const c_char,
    def: c_uchar,
    empty_nul: c_int,
) -> c_uchar {
    if value.is_null() {
        return def;
    }

    let c_str = unsafe { CStr::from_ptr(value) };
    let str_slice = match c_str.to_str() {
        Ok(s) => s,
        Err(_) => return def,
    };

    parse_character(str_slice, def, empty_nul != 0)
}

#[unsafe(no_mangle)]
pub extern "C" fn configreader_validate_string_length(
    value: *const c_char,
    minlen: usize,
    maxlen: usize,
) -> c_int {
    if value.is_null() {
        return 0;
    }

    let c_str = unsafe { CStr::from_ptr(value) };
    let str_slice = match c_str.to_str() {
        Ok(s) => s,
        Err(_) => return 0,
    };

    validate_string_length(str_slice, minlen, maxlen) as c_int
}

#[unsafe(no_mangle)]
pub extern "C" fn configreader_parse_duration(
    value: *const c_char,
    def: c_ulonglong,
    min: c_ulonglong,
    max: c_ulonglong,
    result: *mut c_ulonglong,
) -> c_int {
    if value.is_null() || result.is_null() {
        return 0;
    }

    let c_str = unsafe { CStr::from_ptr(value) };
    let str_slice = match c_str.to_str() {
        Ok(s) => s,
        Err(_) => {
            unsafe { *result = def; }
            return 0;
        }
    };

    // Check if string is empty
    if str_slice.is_empty() {
        unsafe { *result = def; }
        return 0;
    }

    // Parse the duration
    match parse_duration(str_slice) {
        Some(parsed) => {
            // Clamp to range
            let final_value = clamp_duration(parsed, min, max, def);
            unsafe { *result = final_value; }
            1
        }
        None => {
            unsafe { *result = def; }
            0
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_bool() {
        assert!(parse_bool("yes", false));
        assert!(parse_bool("YES", false));
        assert!(parse_bool("true", false));
        assert!(parse_bool("True", false));
        assert!(parse_bool("on", false));
        assert!(parse_bool("ON", false));

        assert!(!parse_bool("no", true));
        assert!(!parse_bool("NO", true));
        assert!(!parse_bool("false", true));
        assert!(!parse_bool("False", true));
        assert!(!parse_bool("off", true));
        assert!(!parse_bool("OFF", true));

        // Invalid values return default
        assert!(parse_bool("invalid", true));
        assert!(!parse_bool("invalid", false));
        assert!(parse_bool("", true));
        assert!(!parse_bool("", false));
    }

    #[test]
    fn test_parse_sint_basic() {
        let (val, _) = parse_sint_with_tail("42");
        assert_eq!(val, 42);

        let (val, _) = parse_sint_with_tail("-42");
        assert_eq!(val, -42);

        let (val, _) = parse_sint_with_tail("0");
        assert_eq!(val, 0);

        let (val, _) = parse_sint_with_tail("  42  ");
        assert_eq!(val, 42);
    }

    #[test]
    fn test_parse_sint_with_magnitude() {
        let (val, idx) = parse_sint_with_tail("1K");
        assert_eq!(val, 1024);
        assert_eq!(idx, 2);

        let (val, idx) = parse_sint_with_tail("1M");
        assert_eq!(val, 1024 * 1024);
        assert_eq!(idx, 2);

        let (val, idx) = parse_sint_with_tail("1G");
        assert_eq!(val, 1024 * 1024 * 1024);
        assert_eq!(idx, 2);
    }

    #[test]
    fn test_parse_uint_basic() {
        let (val, _) = parse_uint_with_tail("42");
        assert_eq!(val, 42);

        let (val, _) = parse_uint_with_tail("0");
        assert_eq!(val, 0);
    }

    #[test]
    fn test_parse_uint_with_magnitude() {
        let (val, idx) = parse_uint_with_tail("1K");
        assert_eq!(val, 1024);
        assert_eq!(idx, 2);

        let (val, idx) = parse_uint_with_tail("2M");
        assert_eq!(val, 2 * 1024 * 1024);
        assert_eq!(idx, 2);
    }

    #[test]
    fn test_parse_float_basic() {
        let (val, _) = parse_float_with_tail("42.5");
        assert!((val - 42.5).abs() < 1e-10);

        let (val, _) = parse_float_with_tail("-42.5");
        assert!((val - (-42.5)).abs() < 1e-10);

        let (val, _) = parse_float_with_tail("1e3");
        assert!((val - 1000.0).abs() < 1e-10);

        let (val, _) = parse_float_with_tail("1.5e2");
        assert!((val - 150.0).abs() < 1e-10);
    }

    #[test]
    fn test_parse_character() {
        assert_eq!(parse_character("a", b'x', false), b'a');
        assert_eq!(parse_character("a", b'x', false), b'a');
        assert_eq!(parse_character("", b'x', false), b'x');
        assert_eq!(parse_character("", b'x', true), 0);
        assert_eq!(parse_character("ab", b'x', false), b'x');
    }

    #[test]
    fn test_clamp_sint() {
        assert_eq!(clamp_sint(50, 0, 100, 0), 50);
        assert_eq!(clamp_sint(-5, 0, 100, 0), 0);
        assert_eq!(clamp_sint(150, 0, 100, 0), 0);
    }

    #[test]
    fn test_clamp_uint() {
        assert_eq!(clamp_uint(50, 0, 100, 0), 50);
        assert_eq!(clamp_uint(150, 0, 100, 0), 0);
    }

    #[test]
    fn test_validate_string_length() {
        assert!(validate_string_length("hello", 0, 10));
        assert!(validate_string_length("hello", 5, 10));
        assert!(!validate_string_length("hello", 0, 4));
        assert!(!validate_string_length("hello", 6, 10));
    }

    #[test]
    fn test_parse_duration_basic() {
        assert_eq!(parse_duration("60"), Some(60));
        assert_eq!(parse_duration("60s"), Some(60));
        assert_eq!(parse_duration("60S"), Some(60));
    }

    #[test]
    fn test_parse_duration_minutes() {
        assert_eq!(parse_duration("1m"), Some(60));
        assert_eq!(parse_duration("1M"), Some(60));
        assert_eq!(parse_duration("5m"), Some(300));
    }

    #[test]
    fn test_parse_duration_hours() {
        assert_eq!(parse_duration("1h"), Some(3600));
        assert_eq!(parse_duration("1H"), Some(3600));
        assert_eq!(parse_duration("2h"), Some(7200));
    }

    #[test]
    fn test_parse_duration_days() {
        assert_eq!(parse_duration("1d"), Some(86400));
        assert_eq!(parse_duration("1D"), Some(86400));
    }

    #[test]
    fn test_parse_duration_weeks() {
        assert_eq!(parse_duration("1w"), Some(604800));
        assert_eq!(parse_duration("1W"), Some(604800));
    }

    #[test]
    fn test_parse_duration_years() {
        assert_eq!(parse_duration("1y"), Some(31557600));
        assert_eq!(parse_duration("1Y"), Some(31557600));
    }

    #[test]
    fn test_parse_duration_combined() {
        assert_eq!(parse_duration("1h30m"), Some(5400));
        assert_eq!(parse_duration("1d2h"), Some(93600));
        assert_eq!(parse_duration("1y2w3d4h5m6s"), Some(31557600 + 1209600 + 259200 + 14400 + 300 + 6));
    }

    #[test]
    fn test_parse_duration_invalid() {
        assert_eq!(parse_duration(""), None);
        assert_eq!(parse_duration("invalid"), None);
        assert_eq!(parse_duration("1x"), None);
    }

    #[test]
    fn test_clamp_duration() {
        assert_eq!(clamp_duration(50, 0, 100, 0), 50);
        assert_eq!(clamp_duration(150, 0, 100, 0), 0);
        assert_eq!(clamp_duration(150, 0, 100, 10), 10);
    }
}


