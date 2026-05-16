use std::ffi::{c_char, CStr, CString};

/// Escapes special characters in IRC message tags according to IRCv3 tag escaping rules.
/// Replaces: space -> \s, semicolon -> \:, backslash -> \\, newline -> \n, carriage return -> \r
pub fn escape_tag(value: &str) -> String {
    let mut ret = String::with_capacity(value.len());
    for chr in value.chars() {
        match chr {
            ' ' => ret.push_str("\\s"),
            ';' => ret.push_str("\\:"),
            '\\' => ret.push_str("\\\\"),
            '\n' => ret.push_str("\\n"),
            '\r' => ret.push_str("\\r"),
            _ => ret.push(chr),
        }
    }
    ret
}

/// Unescapes special characters in IRC message tags according to IRCv3 tag escaping rules.
/// Replaces: \s -> space, \: -> semicolon, \\ -> backslash, \n -> newline, \r -> carriage return
pub fn unescape_tag(value: &str) -> String {
    let mut ret = String::with_capacity(value.len());
    let mut chars = value.chars().peekable();
    
    while let Some(chr) = chars.next() {
        if chr != '\\' {
            ret.push(chr);
            continue;
        }

        // Found a backslash, get the next character
        if let Some(next_chr) = chars.next() {
            match next_chr {
                's' => ret.push(' '),
                ':' => ret.push(';'),
                '\\' => ret.push('\\'),
                'n' => ret.push('\n'),
                'r' => ret.push('\r'),
                _ => ret.push(next_chr),
            }
        }
    }
    
    ret
}

// FFI exports for C++ interop

#[unsafe(no_mangle)]
pub extern "C" fn clientprotocol_escape_tag(value: *const c_char) -> *mut c_char {
    if value.is_null() {
        return CString::new("").unwrap().into_raw();
    }
    
    let str = unsafe { CStr::from_ptr(value) }.to_str().unwrap_or("");
    CString::new(escape_tag(str)).unwrap().into_raw()
}

#[unsafe(no_mangle)]
pub extern "C" fn clientprotocol_unescape_tag(value: *const c_char) -> *mut c_char {
    if value.is_null() {
        return CString::new("").unwrap().into_raw();
    }
    
    let str = unsafe { CStr::from_ptr(value) }.to_str().unwrap_or("");
    CString::new(unescape_tag(str)).unwrap().into_raw()
}

#[unsafe(no_mangle)]
pub extern "C" fn clientprotocol_free_string(ptr: *mut c_char) {
    if !ptr.is_null() {
        unsafe {
            let _ = CString::from_raw(ptr);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_escape_tag() {
        assert_eq!(escape_tag("hello world"), "hello\\sworld");
        assert_eq!(escape_tag("test;value"), "test\\:value");
        assert_eq!(escape_tag("path\\file"), "path\\\\file");
        assert_eq!(escape_tag("line1\nline2"), "line1\\nline2");
        assert_eq!(escape_tag("carriage\rreturn"), "carriage\\rreturn");
    }

    #[test]
    fn test_unescape_tag() {
        assert_eq!(unescape_tag("hello\\sworld"), "hello world");
        assert_eq!(unescape_tag("test\\:value"), "test;value");
        assert_eq!(unescape_tag("path\\\\file"), "path\\file");
        assert_eq!(unescape_tag("line1\\nline2"), "line1\nline2");
        assert_eq!(unescape_tag("carriage\\rreturn"), "carriage\rreturn");
    }

    #[test]
    fn test_roundtrip() {
        let original = "hello world;test\\path\nline";
        let escaped = escape_tag(original);
        let unescaped = unescape_tag(&escaped);
        assert_eq!(original, unescaped);
    }
}
