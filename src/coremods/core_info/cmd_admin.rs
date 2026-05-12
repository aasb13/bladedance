/*
 * InspIRCd -- Internet Relay Chat Daemon
 *
 *   Copyright (C) 2026 Sadie Powell <sadie@witchery.services>
 */

use std::ffi::{c_char, c_void};

// Import StdString from stringutils module
use crate::stringutils::StdString;

// RFC 1459 numeric constants
const RPL_ADMINME: u32 = 256;
const RPL_ADMINLOC1: u32 = 257;
const RPL_ADMINLOC2: u32 = 258;
const RPL_ADMINEMAIL: u32 = 259;

/// CommandAdmin structure to hold admin information
pub struct CommandAdmin {
    adminname: String,
    admindesc: String,
    adminemail: String,
}

impl CommandAdmin {
    /// Creates a new CommandAdmin instance
    pub fn new() -> Self {
        CommandAdmin {
            adminname: String::new(),
            admindesc: String::new(),
            adminemail: String::new(),
        }
    }

    /// Sets the admin name from configuration
    pub fn set_admin_name(&mut self, name: &str) {
        self.adminname = name.to_string();
    }

    /// Sets the admin description from configuration  
    pub fn set_admin_description(&mut self, desc: &str) {
        self.admindesc = desc.to_string();
    }

    /// Sets the admin email from configuration
    pub fn set_admin_email(&mut self, email: &str) {
        self.adminemail = email.to_string();
    }

    /// Handles the ADMIN command
    /// 
    /// # Arguments
    /// * `user_level` - The user's access level (0 = normal user, >0 = operator)
    /// * `server_name` - The server name
    /// 
    /// # Returns
    /// A tuple of (success, response_lines) where response_lines contains the admin info
    pub fn handle_admin(&self, user_level: i32, server_name: &str) -> (bool, Vec<String>) {
        let mut response_lines = Vec::new();

        if user_level > 0 {
            // User is an operator, show admin info
            response_lines.push(format!("{} {} :Administrative info", RPL_ADMINME, server_name));
            response_lines.push(format!("{} {} :{}", RPL_ADMINLOC1, server_name, self.adminname));
            response_lines.push(format!("{} {} :Contact via /MSG when online", RPL_ADMINLOC2, server_name));
        } else {
            // User is not an operator, show access denied message
            response_lines.push(format!("{} {} :User level of above 0 is required to execute this command", RPL_ADMINME, server_name));
        }

        (true, response_lines)
    }
}

// C-compatible wrapper functions
#[unsafe(no_mangle)]
pub unsafe extern "C" fn CommandAdmin_Create() -> *mut c_void {
    let admin = Box::new(CommandAdmin::new());
    Box::into_raw(admin) as *mut c_void
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn CommandAdmin_Destroy(ptr: *mut c_void) {
    if ptr.is_null() {
        return;
    }
    unsafe {
        let _ = Box::from_raw(ptr as *mut CommandAdmin);
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn CommandAdmin_SetAdminName(
    ptr: *mut c_void,
    name: *const c_char,
    name_length: usize
) {
    if ptr.is_null() || name.is_null() {
        return;
    }
    
    let admin = unsafe { &mut *(ptr as *mut CommandAdmin) };
    let name_data = unsafe { std::slice::from_raw_parts(name as *const u8, name_length) };
    let name_str = String::from_utf8_lossy(name_data);
    admin.set_admin_name(&name_str);
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn CommandAdmin_SetAdminDescription(
    ptr: *mut c_void,
    desc: *const c_char,
    desc_length: usize
) {
    if ptr.is_null() || desc.is_null() {
        return;
    }
    
    let admin = unsafe { &mut *(ptr as *mut CommandAdmin) };
    let desc_data = unsafe { std::slice::from_raw_parts(desc as *const u8, desc_length) };
    let desc_str = String::from_utf8_lossy(desc_data);
    admin.set_admin_description(&desc_str);
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn CommandAdmin_SetAdminEmail(
    ptr: *mut c_void,
    email: *const c_char,
    email_length: usize
) {
    if ptr.is_null() || email.is_null() {
        return;
    }
    
    let admin = unsafe { &mut *(ptr as *mut CommandAdmin) };
    let email_data = unsafe { std::slice::from_raw_parts(email as *const u8, email_length) };
    let email_str = String::from_utf8_lossy(email_data);
    admin.set_admin_email(&email_str);
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn CommandAdmin_HandleAdmin(
    ptr: *mut c_void,
    user_level: i32,
    server_name: *const c_char,
    server_name_length: usize
) -> StdString {
    if ptr.is_null() || server_name.is_null() {
        return StdString::from_vec("".to_string().into_bytes());
    }
    
    let admin = unsafe { &*(ptr as *mut CommandAdmin) };
    let server_name_data = unsafe { std::slice::from_raw_parts(server_name as *const u8, server_name_length) };
    let server_name_str = String::from_utf8_lossy(server_name_data);
    
    let (_, response_lines) = admin.handle_admin(user_level, &server_name_str);
    
    // Join all response lines with newlines
    let response = response_lines.join("\n");
    StdString::from_vec(response.into_bytes())
}
