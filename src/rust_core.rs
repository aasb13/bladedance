/*
 * InspIRCd -- Internet Relay Chat Daemon
 *
 *   Crate root for the core staticlib. Meson passes only a single .rs file to
 *   rustc; additional modules are wired via `mod` here.
 */

pub mod stringutils;
pub mod bancache;
pub mod usermanager;
pub mod cidr;
pub mod channelmanager;
pub mod snomasks;
pub mod wildcard;
pub mod logging;
pub mod modulemanager;
pub mod modules;
pub mod timer;
pub mod users;
pub mod server;
pub mod traits;
pub mod account;

#[path = "coremods/core_info/cmd_admin.rs"] 
mod cmd_admin;

use std::ffi::c_void;
use std::collections::HashMap;
use std::sync::Mutex;
use std::sync::LazyLock;

#[derive(Default)]
struct ModuleData {
    config: HashMap<String, String>,
    initialized: bool,
}

static MODULE_DATA: LazyLock<Mutex<HashMap<usize, ModuleData>>> = LazyLock::new(|| Mutex::new(HashMap::new()));

unsafe fn handle_to_key(handle: *mut c_void) -> usize {
    handle as usize
}