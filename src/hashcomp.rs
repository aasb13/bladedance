use std::ffi::{c_char, CStr, CString};
use std::os::raw::c_int;

/// ASCII case-insensitive map for IRC string comparisons
pub static ASCII_CASE_INSENSITIVE_MAP: [u8; 256] = [
    0,   1,   2,   3,   4,   5,   6,   7,   8,   9,   // 0-9
    10,  11,  12,  13,  14,  15,  16,  17,  18,  19,  // 10-19
    20,  21,  22,  23,  24,  25,  26,  27,  28,  29,  // 20-29
    30,  31,  32,  33,  34,  35,  36,  37,  38,  39,  // 30-39
    40,  41,  42,  43,  44,  45,  46,  47,  48,  49,  // 40-49
    50,  51,  52,  53,  54,  55,  56,  57,  58,  59,  // 50-59
    60,  61,  62,  63,  64,  97,  98,  99,  100, 101, // 60-69
    102, 103, 104, 105, 106, 107, 108, 109, 110, 111, // 70-79
    112, 113, 114, 115, 116, 117, 118, 119, 120, 121, // 80-89
    122, 91,  92,  93,  94,  95,  96,  97,  98,  99,  // 90-99
    100, 101, 102, 103, 104, 105, 106, 107, 108, 109, // 100-109
    110, 111, 112, 113, 114, 115, 116, 117, 118, 119, // 110-119
    120, 121, 122, 123, 124, 125, 126, 127, 128, 129, // 120-129
    130, 131, 132, 133, 134, 135, 136, 137, 138, 139, // 130-139
    140, 141, 142, 143, 144, 145, 146, 147, 148, 149, // 140-149
    150, 151, 152, 153, 154, 155, 156, 157, 158, 159, // 150-159
    160, 161, 162, 163, 164, 165, 166, 167, 168, 169, // 160-169
    170, 171, 172, 173, 174, 175, 176, 177, 178, 179, // 170-179
    180, 181, 182, 183, 184, 185, 186, 187, 188, 189, // 180-189
    190, 191, 192, 193, 194, 195, 196, 197, 198, 199, // 190-199
    200, 201, 202, 203, 204, 205, 206, 207, 208, 209, // 200-209
    210, 211, 212, 213, 214, 215, 216, 217, 218, 219, // 210-219
    220, 221, 222, 223, 224, 225, 226, 227, 228, 229, // 220-229
    230, 231, 232, 233, 234, 235, 236, 237, 238, 239, // 230-249
    240, 241, 242, 243, 244, 245, 246, 247, 248, 249, // 240-249
    250, 251, 252, 253, 254, 255,                     // 250-255
];

/// Case-insensitive string comparison using the national case insensitive map
pub fn equals(s1: &str, s2: &str) -> bool {
    if s1.len() != s2.len() {
        return false;
    }

    for (c1, c2) in s1.bytes().zip(s2.bytes()) {
        if ASCII_CASE_INSENSITIVE_MAP[c1 as usize] != ASCII_CASE_INSENSITIVE_MAP[c2 as usize] {
            return false;
        }
    }

    true
}

/// Case-insensitive substring search
pub fn find(haystack: &str, needle: &str) -> Option<usize> {
    if needle.len() > haystack.len() {
        return None;
    }

    let haystack_last = haystack.len() - needle.len();
    for hpos in 0..=haystack_last {
        let mut found = true;
        for (npos, nc) in needle.bytes().enumerate() {
            let hc = haystack.as_bytes()[hpos + npos];
            if ASCII_CASE_INSENSITIVE_MAP[nc as usize] != ASCII_CASE_INSENSITIVE_MAP[hc as usize] {
                found = false;
                break;
            }
        }
        if found {
            return Some(hpos);
        }
    }

    None
}

/// Case-insensitive string comparison for sorting (strict weak ordering)
pub fn insensitive_swo_compare(a: &str, b: &str) -> bool {
    let maxsize = a.len().min(b.len());

    for i in 0..maxsize {
        let a_char = ASCII_CASE_INSENSITIVE_MAP[a.as_bytes()[i] as usize];
        let b_char = ASCII_CASE_INSENSITIVE_MAP[b.as_bytes()[i] as usize];
        if a_char > b_char {
            return false;
        } else if a_char < b_char {
            return true;
        }
    }
    a.len() < b.len()
}

/// Hash function for case-insensitive strings
pub fn insensitive_hash(s: &str) -> usize {
    let mut t: usize = 0;
    for c in s.bytes() {
        t = 5 * t + ASCII_CASE_INSENSITIVE_MAP[c as usize] as usize;
    }
    t
}

/// Separator-based token stream
#[repr(C)]
pub struct RustSepStream {
    tokens: *mut c_char,
    sep: u8,
    allow_empty: bool,
    pos: usize,
}

impl RustSepStream {
    pub fn new(source: &str, separator: char, allowempty: bool) -> Self {
        RustSepStream {
            tokens: CString::new(source).unwrap().into_raw(),
            sep: separator as u8,
            allow_empty: allowempty,
            pos: 0,
        }
    }

    pub fn get_token(&mut self) -> Option<String> {
        if self.stream_end() {
            return None;
        }

        if !self.allow_empty {
            let tokens_str = unsafe { CStr::from_ptr(self.tokens) }.to_str().unwrap_or("");
            if let Some(new_pos) = tokens_str[self.pos..].chars().position(|c| c != self.sep as char) {
                self.pos += new_pos;
            } else {
                self.pos = tokens_str.len() + 1;
                return None;
            }
        }

        let tokens_str = unsafe { CStr::from_ptr(self.tokens) }.to_str().unwrap_or("");
        let p = tokens_str[self.pos..].find(self.sep as char);
        let p = match p {
            Some(pos) => self.pos + pos,
            None => tokens_str.len(),
        };

        let token = tokens_str[self.pos..p].to_string();
        self.pos = p + 1;
        Some(token)
    }

    pub fn get_remaining(&self) -> String {
        if self.stream_end() {
            return String::new();
        }
        let tokens_str = unsafe { CStr::from_ptr(self.tokens) }.to_str().unwrap_or("");
        tokens_str[self.pos..].to_string()
    }

    pub fn stream_end(&self) -> bool {
        let tokens_str = unsafe { CStr::from_ptr(self.tokens) }.to_str().unwrap_or("");
        self.pos > tokens_str.len()
    }
}

impl Drop for RustSepStream {
    fn drop(&mut self) {
        if !self.tokens.is_null() {
            unsafe {
                let _ = CString::from_raw(self.tokens);
            }
        }
    }
}

// FFI exports for C++ interop

#[unsafe(no_mangle)]
pub extern "C" fn hashcomp_equals(s1: *const c_char, s2: *const c_char) -> c_int {
    if s1.is_null() || s2.is_null() {
        return 0;
    }
    
    let str1 = unsafe { CStr::from_ptr(s1) }.to_str().unwrap_or("");
    let str2 = unsafe { CStr::from_ptr(s2) }.to_str().unwrap_or("");
    
    if equals(str1, str2) { 1 } else { 0 }
}

#[unsafe(no_mangle)]
pub extern "C" fn hashcomp_find(haystack: *const c_char, needle: *const c_char) -> isize {
    if haystack.is_null() || needle.is_null() {
        return -1;
    }
    
    let hay = unsafe { CStr::from_ptr(haystack) }.to_str().unwrap_or("");
    let need = unsafe { CStr::from_ptr(needle) }.to_str().unwrap_or("");
    
    match find(hay, need) {
        Some(pos) => pos as isize,
        None => -1,
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn hashcomp_insensitive_swo_compare(a: *const c_char, b: *const c_char) -> c_int {
    if a.is_null() || b.is_null() {
        return 0;
    }
    
    let str_a = unsafe { CStr::from_ptr(a) }.to_str().unwrap_or("");
    let str_b = unsafe { CStr::from_ptr(b) }.to_str().unwrap_or("");
    
    if insensitive_swo_compare(str_a, str_b) { 1 } else { 0 }
}

#[unsafe(no_mangle)]
pub extern "C" fn hashcomp_insensitive_hash(s: *const c_char) -> usize {
    if s.is_null() {
        return 0;
    }
    
    let str = unsafe { CStr::from_ptr(s) }.to_str().unwrap_or("");
    insensitive_hash(str)
}

// SepStream FFI exports

#[unsafe(no_mangle)]
pub extern "C" fn hashcomp_sepstream_new(source: *const c_char, separator: u8, allowempty: bool) -> *mut RustSepStream {
    if source.is_null() {
        return std::ptr::null_mut();
    }
    
    let source_str = unsafe { CStr::from_ptr(source) }.to_str().unwrap_or("");
    Box::into_raw(Box::new(RustSepStream::new(source_str, separator as char, allowempty)))
}

#[unsafe(no_mangle)]
pub extern "C" fn hashcomp_sepstream_get_token(stream: *mut RustSepStream, token: *mut *mut c_char) -> bool {
    if stream.is_null() || token.is_null() {
        return false;
    }
    
    let stream_ref = unsafe { &mut *stream };
    match stream_ref.get_token() {
        Some(t) => {
            unsafe {
                *token = CString::new(t).unwrap().into_raw();
            }
            true
        }
        None => false,
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn hashcomp_sepstream_get_remaining(stream: *mut RustSepStream) -> *mut c_char {
    if stream.is_null() {
        return std::ptr::null_mut();
    }
    
    let stream_ref = unsafe { &*stream };
    CString::new(stream_ref.get_remaining()).unwrap().into_raw()
}

#[unsafe(no_mangle)]
pub extern "C" fn hashcomp_sepstream_stream_end(stream: *mut RustSepStream) -> bool {
    if stream.is_null() {
        return true;
    }
    
    let stream_ref = unsafe { &*stream };
    stream_ref.stream_end()
}

#[unsafe(no_mangle)]
pub extern "C" fn hashcomp_sepstream_free(stream: *mut RustSepStream) {
    if !stream.is_null() {
        unsafe {
            let _ = Box::from_raw(stream);
        }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn hashcomp_free_string(ptr: *mut c_char) {
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
    fn test_equals() {
        assert!(equals("hello", "HELLO"));
        assert!(equals("world", "WORLD"));
        assert!(!equals("hello", "world"));
        assert!(equals("", ""));
    }

    #[test]
    fn test_find() {
        assert_eq!(find("hello world", "WORLD"), Some(6));
        assert_eq!(find("HELLO", "lo"), Some(3));
        assert_eq!(find("test", "xyz"), None);
    }

    #[test]
    fn test_insensitive_hash() {
        let h1 = insensitive_hash("hello");
        let h2 = insensitive_hash("HELLO");
        assert_eq!(h1, h2);
    }

    #[test]
    fn test_insensitive_swo_compare() {
        assert!(insensitive_swo_compare("aaa", "bbb"));
        assert!(!insensitive_swo_compare("bbb", "aaa"));
        assert!(insensitive_swo_compare("aa", "aaa"));
    }

    #[test]
    fn test_sepstream() {
        let mut stream = RustSepStream::new("a,b,c", ',', false);
        assert_eq!(stream.get_token(), Some("a".to_string()));
        assert_eq!(stream.get_token(), Some("b".to_string()));
        assert_eq!(stream.get_token(), Some("c".to_string()));
        assert_eq!(stream.get_token(), None);
    }

    #[test]
    fn test_sepstream_empty() {
        let mut stream = RustSepStream::new("a,,c", ',', false);
        assert_eq!(stream.get_token(), Some("a".to_string()));
        assert_eq!(stream.get_token(), Some("c".to_string()));
    }

    #[test]
    fn test_sepstream_allow_empty() {
        let mut stream = RustSepStream::new("a,,c", ',', true);
        assert_eq!(stream.get_token(), Some("a".to_string()));
        assert_eq!(stream.get_token(), Some("".to_string()));
        assert_eq!(stream.get_token(), Some("c".to_string()));
    }
}
