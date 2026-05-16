use std::sync::{Arc, Mutex as StdMutex};
use tokio::sync::Mutex;
use mongodb::{Client, Collection, Database, options::{ClientOptions, FindOneOptions, UpdateOptions, InsertOneOptions}};
use mongodb::bson::{doc, Document};
use chrono::{DateTime, Utc};
use std::ffi::c_void;
use tracing::{error, info};

use rust_core::traits::{Module, Command, CmdResult, Params};
use rust_core::users::User;
use rust_core::account::AccountAPI;

unsafe extern "C" {
    fn inspircd_async_get_handle() -> *const std::ffi::c_void;
    fn rust_log_manager_write(level: i32, module: *const i8, message: *const i8);
}

#[inline]
fn core_handle() -> Option<tokio::runtime::Handle> {
    unsafe {
        let p = inspircd_async_get_handle();
        if p.is_null() { return None; }
        Some((*(p as *const tokio::runtime::Handle)).clone())
    }
}

// VTable structure for C++ wrapper
#[repr(C)]
struct RustModuleVtable {
    init: extern "C" fn(*mut c_void),
    read_config: extern "C" fn(*mut c_void),
    destroy: extern "C" fn(*mut c_void),
}

// Static vtable for C++ wrapper
static VTABLE: RustModuleVtable = RustModuleVtable {
    init: rust_module_init,
    read_config: rust_module_read_config,
    destroy: rust_module_destroy,
};

#[derive(Debug, Clone)]
struct BladedanceConfig {
    mongo_uri: String,
}

impl Default for BladedanceConfig {
    fn default() -> Self {
        Self {
            mongo_uri: "mongodb://127.0.0.1:27017".to_string(),
        }
    }
}

#[derive(Clone)]
pub struct ModuleBladedance {
    config: Arc<StdMutex<BladedanceConfig>>,
    client: Option<Arc<Client>>,
    database: Option<Arc<Database>>,
    users_collection: Option<Arc<Collection<Document>>>,
    channels_collection: Option<Arc<Collection<Document>>>,
    account_api: AccountAPI,
    db_initialized: bool,
    db_initializing: bool,
}

impl ModuleBladedance {
    pub fn new() -> Self {
        Self {
            config: Arc::new(StdMutex::new(BladedanceConfig::default())),
            client: None,
            database: None,
            users_collection: None,
            channels_collection: None,
            account_api: AccountAPI::new(),
            db_initialized: false,
            db_initializing: false,
        }
    }

    async fn initialize_database(&mut self, uri: &str) -> Result<(), Box<dyn std::error::Error>> {
        self.db_initializing = true;

        let client_options = ClientOptions::parse(uri).await?;
        let client = Client::with_options(client_options)?;
        
        let database = client.database("irc");
        let users_collection = database.collection("users");
        let channels_collection = database.collection("channels");

        self.client = Some(Arc::new(client));
        self.database = Some(Arc::new(database));
        self.users_collection = Some(Arc::new(users_collection));
        self.channels_collection = Some(Arc::new(channels_collection));

        self.db_initialized = true;
        self.db_initializing = false;

        Ok(())
    }

    async fn get_user_level(&self, user: &User) -> i32 {
        if let (Some(users_collection), Some(_client)) = (&self.users_collection, &self.client) {
            if user.is_oper() {
                return 4;
            }

            let account_name = match self.account_api.get_account_name(user) {
                Some(name) if !name.is_empty() => name,
                _ => return 0,
            };

            let filter = doc! { "account_name": &account_name };
            let projection = doc! { "userlevel": 1 };
            
            let options = FindOneOptions::builder().projection(projection).build();
            
            if let Ok(Some(document)) = users_collection.find_one(filter).with_options(options).await {
                if let Ok(level) = document.get_i32("userlevel") {
                    return level;
                }
            }
        }
        0
    }

    async fn set_user_level(&self, account_name: &str, level: i32) -> Result<(), Box<dyn std::error::Error>> {
        if let Some(users_collection) = &self.users_collection {
            let filter = doc! { "account_name": account_name };
            let update = doc! { "$set": { "userlevel": level } };
            
            let options = UpdateOptions::builder().upsert(true).build();
            users_collection.update_one(filter, update).with_options(options).await?;
        }
        Ok(())
    }

    async fn add_user(&self, account_name: &str, level: i32) -> Result<(), Box<dyn std::error::Error>> {
        if let Some(users_collection) = &self.users_collection {
            let now: DateTime<Utc> = Utc::now();
            let document = doc! {
                "account_name": account_name,
                "userlevel": level,
                "created_at": now.to_rfc3339()
            };
            
            let options = InsertOneOptions::default();
            users_collection.insert_one(document).with_options(options).await?;
        }
        Ok(())
    }
}

#[async_trait::async_trait]
impl Module for ModuleBladedance {
    fn read_config(&mut self, config: &str) -> Result<(), Box<dyn std::error::Error>> {
        // Parse configuration - for now just extract mongo URI
        let mut config_guard = self.config.lock().unwrap();
        
        // Simple parsing for mongouri=... format
        if let Some(uri_start) = config.find("mongouri=") {
            let uri_part = &config[uri_start + 9..];
            if let Some(uri_end) = uri_part.find(|c| c == '\n' || c == ' ') {
                config_guard.mongo_uri = uri_part[..uri_end].to_string();
            } else {
                config_guard.mongo_uri = uri_part.to_string();
            }
        }

        if !self.db_initialized && !self.db_initializing {
            let uri = config_guard.mongo_uri.clone();
            drop(config_guard);
            
            if let Some(h) = core_handle() {
                h.block_on(self.initialize_database(&uri))?;
            } else {
                return Err("Core async runtime not initialized".into());
            }
        }

        Ok(())
    }

    fn init(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        // Module initialization
        Ok(())
    }
}

pub struct CommandWhoami {
    module: Arc<Mutex<ModuleBladedance>>,
}

impl CommandWhoami {
    pub fn new(module: Arc<Mutex<ModuleBladedance>>) -> Self {
        Self { module }
    }
}

#[async_trait::async_trait]
impl Command for CommandWhoami {
    async fn handle(&self, user: &User, _params: &Params) -> CmdResult {
        let module = self.module.clone();
        
        let level = {
            let module_guard = module.lock().await;
            module_guard.get_user_level(user).await
        };
        
        let account_name = {
            let module_guard = module.lock().await;
            module_guard.account_api.get_account_name(user)
                .unwrap_or_else(|| "none".to_string())
        };
        
        let message = format!("Account: {} | Level: {}", account_name, level);
        user.write_remote_notice(&message);
        
        CmdResult::Success
    }
}

// ABI version for Rust modules
#[unsafe(no_mangle)]
pub static inspircd_module_abi: u64 = 4011u64;

// Module version
#[unsafe(no_mangle)]
pub static inspircd_module_version: [u8; 7] = *b"4.10.1\0";

#[unsafe(no_mangle)]
pub extern "C" fn rust_module_init(handle: *mut std::ffi::c_void) {
    if handle.is_null() {
        error!(module = "m_bladedance", "rust_module_init called with null handle");
        return;
    }
    let module = unsafe { &mut *(handle as *mut ModuleBladedance) };
    let uri = module.config.lock().unwrap().mongo_uri.clone();

    match core_handle() {
        Some(h) => match h.block_on(module.initialize_database(&uri)) {
            Ok(_) => info!(module = "m_bladedance", "MongoDB connected to {}", uri),
            Err(e) => error!(module = "m_bladedance", "MongoDB connection failed: {}", e),
        },
        None => error!(module = "m_bladedance", "Core async runtime not available"),
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn rust_module_read_config(_handle: *mut std::ffi::c_void) {
    // Config re-read if needed
}

extern "C" fn rust_module_destroy(handle: *mut std::ffi::c_void) {
    if !handle.is_null() {
        // Convert back to Box and let it drop
        unsafe { drop(Box::from_raw(handle as *mut ModuleBladedance)); }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn inspircd_module_init() -> *mut std::ffi::c_void {
    let module = ModuleBladedance::new();
    Box::into_raw(Box::new(module)) as *mut std::ffi::c_void
}

#[unsafe(no_mangle)]
pub static rust_module_vtable: RustModuleVtable = RustModuleVtable {
    init: rust_module_init,
    read_config: rust_module_read_config,
    destroy: rust_module_destroy,
};