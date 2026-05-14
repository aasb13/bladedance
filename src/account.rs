/*
 * InspIRCd -- Internet Relay Chat Daemon
 *
 *   Account API for Rust modules
 *
 * This file is part of InspIRCd.  InspIRCd is free software: you can
 * redistribute it and/or modify it under terms of the GNU General Public
 * License as published by Free Software Foundation, version 2.
 *
 * This program is distributed in hope that it will be useful, but WITHOUT
 * ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS
 * FOR A PARTICULAR PURPOSE.  See the GNU General Public License for more
 * details.
 *
 * You should have received a copy of the GNU General Public License
 * along with this program.  If not, see <http://www.gnu.org/licenses/>.
 */

use std::ffi::CStr;
use std::os::raw::c_char;
use crate::users::User;

#[derive(Debug, Clone)]
pub struct AccountAPI;

impl AccountAPI {
    pub fn new() -> Self {
        Self
    }

    pub fn get_account_name(&self, user: &User) -> Option<String> {
        unsafe {
            let account_ptr = account_ffi_get_account_name(user as *const User as *mut User);
            if account_ptr.is_null() {
                return None;
            }
            
            let account_cstr = CStr::from_ptr(account_ptr);
            let account_str = account_cstr.to_string_lossy().into_owned();
            
            // Free the C string allocated by C++ code
            account_ffi_free_string(account_ptr);
            
            if account_str.is_empty() {
                None
            } else {
                Some(account_str)
            }
        }
    }

    /// Check if a user is logged into an account
    pub fn is_logged_in(&self, user: &User) -> bool {
        self.get_account_name(user).is_some()
    }

    /// Get account details for a user
    pub fn get_account_details(&self, user: &User) -> Option<AccountDetails> {
        if let Some(account_name) = self.get_account_name(user) {
            Some(AccountDetails {
                name: account_name,
            })
        } else {
            None
        }
    }
}

#[derive(Debug, Clone)]
pub struct AccountDetails {
    pub name: String,
}

impl Default for AccountAPI {
    fn default() -> Self {
        Self::new()
    }
}

// External C++ functions
unsafe extern "C" {
    fn account_ffi_get_account_name(user: *mut User) -> *mut c_char;
    fn account_ffi_free_string(ptr: *mut c_char);
}
