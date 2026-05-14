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
pub mod configreader;
pub mod dynamic;
pub mod hashcomp;
pub mod inspircd;
pub mod helperfuncs;

#[path = "coremods/core_info/cmd_admin.rs"] 
mod cmd_admin;

use std::ffi::c_void;
use std::collections::HashMap;
use tokio::runtime::{Runtime, Handle};
use std::sync::{LazyLock, Mutex};

#[derive(Default)]
struct ModuleData {
    config: HashMap<String, String>,
    initialized: bool,
}

static MODULE_DATA: LazyLock<Mutex<HashMap<usize, ModuleData>>> = LazyLock::new(|| Mutex::new(HashMap::new()));

unsafe fn handle_to_key(handle: *mut c_void) -> usize {
    handle as usize
}

static RUNTIME: LazyLock<Runtime> = LazyLock::new(|| {
    tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .expect("tokio runtime creation failed")
});

static HANDLE: LazyLock<Handle> = LazyLock::new(|| RUNTIME.handle().clone());

pub fn init_async_runtime() {
    let _ = &*HANDLE; // force init
}

#[unsafe(no_mangle)]
pub extern "C" fn inspircd_async_init() {
    init_async_runtime();
}

#[unsafe(no_mangle)]
pub extern "C" fn inspircd_async_get_handle() -> *const std::ffi::c_void {
    (&*HANDLE) as *const Handle as *const std::ffi::c_void
}