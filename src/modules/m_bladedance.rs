use std::sync::{Arc, Mutex as StdMutex};
use tokio::sync::Mutex;
use mongodb::{Client, Collection, Database, options::{ClientOptions, FindOneOptions, UpdateOptions, InsertOneOptions}};
use mongodb::bson::{doc, Document};
use chrono::{DateTime, Utc};
use std::ffi::c_void;

use rust_core::traits::{Module, Command, CmdResult, Params};
use rust_core::users::User;
use rust_core::account::AccountAPI;
use rust_core::logging::log;

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
            
            // Initialize database in a blocking context
            let rt = tokio::runtime::Runtime::new()?;
            rt.block_on(self.initialize_database(&uri))?;
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
        log(1, "m_bladedance", "rust_module_init called with null handle");
        return;
    }
    let module = unsafe { &mut *(handle as *mut ModuleBladedance) };
    let uri = module.config.lock().unwrap().mongo_uri.clone();

    let rt = match tokio::runtime::Runtime::new() {
        Ok(rt) => rt,
        Err(e) => {
            log(1, "m_bladedance", &format!("tokio runtime error: {}", e));
            return;
        }
    };

    match rt.block_on(module.initialize_database(&uri)) {
        Ok(_) => {
            log(2, "m_bladedance", &format!("MongoDB connected to {}", uri));
        },
        Err(e) => {
            log(1, "m_bladedance", &format!("MongoDB connection failed: {}", e));
        }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn rust_module_read_config(handle: *mut std::ffi::c_void) {
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