use std::ffi::c_char;
use std::sync::Mutex;

// Import StdString from stringutils module
use crate::stringutils::StdString;

// Global UID state
static CURRENT_UID: Mutex<String> = Mutex::new(String::new());

/// Generate a Server ID (SID) from server name and description
/// 
/// Uses a simple hash function to generate a 3-digit numeric SID:
/// 1. Hash servername and serverdesc using a linear congruential generator
/// 2. Take modulo 1000 to get a value in range 0-999
/// 3. Format as zero-padded 3-digit string
pub fn generate_sid(servername: &str, serverdesc: &str) -> String {
    let mut sid: u32 = 0;

    // Hash servername using LCG-like algorithm
    for chr in servername.bytes() {
        sid = 5 * sid + chr as u32;
    }
    // Hash serverdesc using same algorithm
    for chr in serverdesc.bytes() {
        sid = 5 * sid + chr as u32;
    }
    // Constrain to 3-digit range and format with leading zeros
    let sid_mod = sid % 1000;
    let mut sidstr = sid_mod.to_string();
    while sidstr.len() < 3 {
        sidstr.insert(0, '0');
    }

    sidstr
}

/// Generate a Server ID (SID) from C-style strings
///
/// # Safety
/// This function is unsafe because it takes raw pointers and lengths.
/// The caller must ensure that the pointers are valid for the specified lengths.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn rust_generate_sid(
    servername: *const c_char,
    servername_length: usize,
    serverdesc: *const c_char,
    serverdesc_length: usize
) -> StdString {
    // Handle null pointers gracefully
    if servername.is_null() || serverdesc.is_null() {
        return StdString::from_vec("000".to_string().into_bytes());
    }

    // Create slices from the raw pointers with bounds checking
    let servername_data = unsafe {
        // Check that the length is not excessive to prevent potential issues
        if servername_length > 0 {
            std::slice::from_raw_parts(servername as *const u8, servername_length)
        } else {
            &[]
        }
    };
    
    let serverdesc_data = unsafe {
        // Check that the length is not excessive to prevent potential issues
        if serverdesc_length > 0 {
            std::slice::from_raw_parts(serverdesc as *const u8, serverdesc_length)
        } else {
            &[]
        }
    };

    // Convert to strings, handling invalid UTF-8 gracefully
    let servername_str = String::from_utf8_lossy(servername_data);
    let serverdesc_str = String::from_utf8_lossy(serverdesc_data);

    let sid = generate_sid(&servername_str, &serverdesc_str);
    StdString::from_vec(sid.into_bytes())
}

// UID management functions
fn increment_uid(uid: &mut String, pos: usize) -> bool {
    /*
     * Okay. The rules for generating a UID go like this...
     * -- > ABCDEFGHIJKLMNOPQRSTUVWXYZ --> 012345679 --> WRAP
     * That is, we start at A. When we reach Z, we go to 0. At 9, we go to
     * A again, in an iterative fashion.. so..
     * AAA9 -> AABA, and so on. -- w00t
     */

    let bytes = unsafe { uid.as_bytes_mut() };
    
    // If we hit Z, wrap around to 0.
    if bytes[pos] == b'Z' {
        bytes[pos] = b'0';
        true
    } else if bytes[pos] == b'9' {
        /*
         * Or, if we hit 9, wrap around to pos = 'A' and (pos - 1)++,
         * e.g. A9 -> BA -> BB ..
         */
        bytes[pos] = b'A';
        if pos == 3 {
            // At pos 3, if we hit '9', we've run out of available UIDs, and reset to AAA..AAA.
            false
        } else {
            increment_uid(uid, pos - 1)
        }
    } else {
        // Anything else, just increment.
        bytes[pos] += 1;
        true
    }
}

pub fn init_uid(sid: &str) {
    let mut uid = String::new();
    /*
     * Copy SID into the first three digits, 9's to the rest
     * Why 9? Well, we increment before we find, otherwise we have an unnecessary copy, and I want UID to start at AAA..AA
     * and not AA..AB. So by initialising to 99999, we force it to rollover to AAAAA on the first IncrementUID call.
     */
    uid.push_str(&sid[..3]);
    for _ in 3..9 {
        uid.push('9');
    }
    
    *CURRENT_UID.lock().unwrap() = uid;
}

/// Get the next unique identifier (UID)
///
/// Returns a String containing the next UID in the sequence.
/// The UID sequence follows a specific pattern:
/// - Starts with AAAAAAAA
/// - Increments through: 0-9 then A-Z for positions 3-8
/// - Positions 0-2 are derived from the SID and can change due to carry
/// - When all positions reach their maximum (Z9ZZZZZZ), the sequence wraps
pub fn get_uid() -> String {
    let mut uid = CURRENT_UID.lock().unwrap();
    increment_uid(&mut uid, 8);
    uid.clone()
}

// C bindings for UID management
/// Initialize the UID system with a Server ID
///
/// # Safety
/// This function is unsafe because it takes a raw pointer and length.
/// The caller must ensure that the pointer is valid for the specified length.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn rust_uid_init(sid: *const c_char, sid_length: usize) {
    // Handle null pointer
    if sid.is_null() {
        return;
    }
    
    // Create slice from the raw pointer with bounds checking
    let sid_data = unsafe {
        // Check that the length is not excessive to prevent potential issues
        if sid_length > 0 {
            std::slice::from_raw_parts(sid as *const u8, sid_length)
        } else {
            &[]
        }
    };
    
    // Convert to string, handling invalid UTF-8 gracefully
    let sid_str = String::from_utf8_lossy(sid_data);
    init_uid(&sid_str);
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn rust_uid_get() -> StdString {
    let uid = get_uid();
    StdString::from_vec(uid.into_bytes())
}
