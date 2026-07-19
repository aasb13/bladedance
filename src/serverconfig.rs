// This file is a Rust port of a corresponding InspIRCd module.
// Original work Copyright (C) the InspIRCd contributors.
// Licensed under GPLv2. See LICENSE for details.

use std::ffi::{c_char, c_int, CStr, CString};
use std::collections::HashMap;
use std::fs::File;
use std::io::{BufRead, BufReader, Read};
use tracing::{error, warn, info};

/// Banned user treatment options
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BannedUserTreatment {
    /// Don't treat a banned user any different to normal.
    Normal = 0,
    /// Restrict the actions of a banned user.
    RestrictSilent = 1,
    /// Restrict the actions of a banned user and notify them of their treatment.
    RestrictNotify = 2,
}

impl Default for BannedUserTreatment {
    fn default() -> Self {
        BannedUserTreatment::Normal
    }
}

/// Server limits configuration
#[repr(C)]
#[derive(Debug, Clone, Default)]
pub struct ServerLimits {
    pub max_line: usize,
    pub max_nick: usize,
    pub max_channel: usize,
    pub max_modes: usize,
    pub max_user: usize,
    pub max_quit: usize,
    pub max_topic: usize,
    pub max_kick: usize,
    pub max_real: usize,
    pub max_away: usize,
    pub max_host: usize,
    pub max_key: usize,
}

/// Server paths configuration
#[repr(C)]
#[derive(Debug, Clone, Default)]
pub struct ServerPaths {
    pub config: String,
    pub data: String,
    pub log: String,
    pub module: String,
    pub runtime: String,
}

/// Command-line options
#[repr(C)]
#[derive(Debug, Clone, Default)]
pub struct CommandLineOptions {
    pub config: Option<String>,
    pub debug: bool,
    pub nofork: bool,
    pub nolog: bool,
    pub nopid: bool,
    pub protocoldebug: bool,
    pub runasroot: bool,
    pub version: bool,
    pub help: bool,
    pub test: bool,
}

/// Main server configuration
#[repr(C)]
#[derive(Debug, Default)]
pub struct ServerConfig {
    // Server identity
    pub server_name: String,
    pub server_id: String,
    pub server_desc: String,
    pub network: String,
    
    // Default modes
    pub default_modes: String,
    
    // Mask settings
    pub mask_in_list: bool,
    pub mask_in_topic: bool,
    pub no_snotice_stack: bool,
    pub syntax_hints: bool,
    
    // X-line messages
    pub xline_message: String,
    pub xline_quit: String,
    pub xline_quit_public: String,
    
    // Banned user treatment
    pub restrict_banned_users: BannedUserTreatment,
    
    // Wildcard IPv6
    pub wildcard_ipv6: bool,
    
    // Performance settings
    pub max_conn: i32,
    pub net_buffer_size: usize,
    pub soft_limit: usize,
    pub time_skip_warn: u64,
    
    // Security settings
    pub custom_version: String,
    pub hide_server: String,
    pub max_targets: usize,
    
    // CIDR settings
    pub ipv4_range: u8,
    pub ipv6_range: u8,
    
    // Limits
    pub limits: ServerLimits,
    
    // Paths
    pub paths: ServerPaths,
    
    // Command line options
    pub command_line: CommandLineOptions,
    
    // Config file path
    pub config_file_name: String,
    
    // Whether config is valid
    pub valid: bool,
}

impl ServerConfig {
    pub fn new() -> Self {
        Self::default()
    }
    
    /// Initialize from command-line options
    pub fn from_command_line(config_path: Option<String>, args: CommandLineOptions) -> Self {
        let mut config = Self::new();
        config.command_line = args;
        config.config_file_name = config_path.unwrap_or_else(|| "inspircd.conf".to_string());
        config
    }
    
    /// Fill in default values
    pub fn fill_defaults(&mut self) {
        // Set default limits
        self.limits = ServerLimits {
            max_line: 512,
            max_nick: 30,
            max_channel: 60,
            max_modes: 20,
            max_user: 10,
            max_quit: 300,
            max_topic: 330,
            max_kick: 300,
            max_real: 130,
            max_away: 200,
            max_host: 64,
            max_key: 32,
        };
        
        // Set default paths
        self.paths = ServerPaths {
            config: "/etc/inspircd".to_string(),
            data: "/var/lib/inspircd".to_string(),
            log: "/var/log/inspircd".to_string(),
            module: "/usr/lib/inspircd/modules".to_string(),
            runtime: "/var/run/inspircd".to_string(),
        };
        
        // Default values
        self.default_modes = "not".to_string();
        self.xline_message = "You're banned!".to_string();
        self.xline_quit = "%fulltype%: %reason%".to_string();
        self.max_targets = 5;
        self.ipv4_range = 32;
        self.ipv6_range = 128;
    }
}

/// FFI function to create a new ServerConfig in Rust
#[unsafe(no_mangle)]
pub extern "C" fn serverconfig_new() -> *mut ServerConfig {
    Box::into_raw(Box::new(ServerConfig::new()))
}

/// FFI function to free a ServerConfig allocated in Rust
#[unsafe(no_mangle)]
pub extern "C" fn serverconfig_free(ptr: *mut ServerConfig) {
    if !ptr.is_null() {
        unsafe { Box::from_raw(ptr); }
    }
}

/// FFI function to fill defaults in a ServerConfig
#[unsafe(no_mangle)]
pub extern "C" fn serverconfig_fill_defaults(ptr: *mut ServerConfig) {
    if !ptr.is_null() {
        unsafe { (*ptr).fill_defaults(); }
    }
}

/// FFI function to set a string value in ServerConfig
#[unsafe(no_mangle)]
pub extern "C" fn serverconfig_set_string(
    ptr: *mut ServerConfig,
    field_name: *const c_char,
    value: *const c_char,
) {
    if ptr.is_null() || field_name.is_null() || value.is_null() {
        return;
    }
    
    let config = unsafe { &mut *ptr };
    let field = unsafe { CStr::from_ptr(field_name) }.to_string_lossy();
    let val = unsafe { CStr::from_ptr(value) }.to_string_lossy().into_owned();
    
    match field.as_ref() {
        "server_name" => config.server_name = val,
        "server_id" => config.server_id = val,
        "server_desc" => config.server_desc = val,
        "network" => config.network = val,
        "default_modes" => config.default_modes = val,
        "xline_message" => config.xline_message = val,
        "xline_quit" => config.xline_quit = val,
        "xline_quit_public" => config.xline_quit_public = val,
        "custom_version" => config.custom_version = val,
        "hide_server" => config.hide_server = val,
        _ => warn!("Unknown ServerConfig field: {}", field),
    }
}

/// FFI function to get a string value from ServerConfig
#[unsafe(no_mangle)]
pub extern "C" fn serverconfig_get_string(
    ptr: *const ServerConfig,
    field_name: *const c_char,
) -> *mut c_char {
    if ptr.is_null() || field_name.is_null() {
        return std::ptr::null_mut();
    }
    
    let config = unsafe { &*ptr };
    let field = unsafe { CStr::from_ptr(field_name) }.to_string_lossy();
    
    let value: &str = match field.as_ref() {
        "server_name" => config.server_name.as_str(),
        "server_id" => config.server_id.as_str(),
        "server_desc" => config.server_desc.as_str(),
        "network" => config.network.as_str(),
        "default_modes" => config.default_modes.as_str(),
        "xline_message" => config.xline_message.as_str(),
        "xline_quit" => config.xline_quit.as_str(),
        "xline_quit_public" => config.xline_quit_public.as_str(),
        "custom_version" => config.custom_version.as_str(),
        "hide_server" => config.hide_server.as_str(),
        "config_file_name" => config.config_file_name.as_str(),
        _ => {
            warn!("Unknown ServerConfig field: {}", field);
            return std::ptr::null_mut();
        }
    };
    
    CString::new(value).map(|s| s.into_raw()).unwrap_or(std::ptr::null_mut())
}

/// Represents a position in a config file
#[repr(C)]
#[derive(Debug, Clone, Default)]
pub struct FilePosition {
    pub name: String,
    pub line: u64,
    pub column: u64,
}

/// Represents a config tag with its attributes
#[repr(C)]
#[derive(Debug, Clone, Default)]
pub struct ConfigTag {
    pub name: String,
    pub source: FilePosition,
    pub attributes: HashMap<String, String>,
}

impl ConfigTag {
    pub fn new(name: String, source: FilePosition) -> Self {
        Self {
            name,
            source,
            attributes: HashMap::new(),
        }
    }
    
    pub fn get_string(&self, key: &str, default: &str) -> String {
        self.attributes.get(key).cloned().unwrap_or_else(|| default.to_string())
    }
    
    pub fn get_bool(&self, key: &str, default: bool) -> bool {
        self.attributes.get(key)
            .map(|v| crate::configreader::parse_bool(v, default))
            .unwrap_or(default)
    }
    
    pub fn get_int<T: std::str::FromStr>(&self, key: &str, default: T) -> T
    where T::Err: std::fmt::Debug {
        self.attributes.get(key)
            .and_then(|v| v.parse().ok())
            .unwrap_or(default)
    }
}

/// Parses a simple XML-like config file and returns a vector of tags
/// Handles multi-line tags like:
/// <tag
///     key1="value1"
///     key2="value2"
/// >
pub fn parse_config_file(path: &str) -> Result<Vec<ConfigTag>, String> {
    let file = File::open(path).map_err(|e| format!("Failed to open {}: {}", path, e))?;
    let reader = BufReader::new(file);
    
    let mut tags = Vec::new();
    let mut current_line = 0u64;
    let mut current_tag: Option<ConfigTag> = None;
    
    for line_result in reader.lines() {
        current_line += 1;
        let line = line_result.map_err(|e| format!("Failed to read line {}: {}", current_line, e))?;
        
        // Skip empty lines and comments
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }
        
        // Check if we're inside a multi-line tag
        if let Some(ref mut tag) = current_tag {
            // Check for closing tag (standalone >)
            if trimmed == ">" {
                tags.push(tag.clone());
                current_tag = None;
                continue;
            }
            
            // Check for self-closing tag like free="no"/>
            if trimmed.ends_with("/>") {
                // Parse attributes before the />
                let content = &trimmed[..trimmed.len()-2];
                let parts: Vec<&str> = content.split_whitespace().collect();
                for part in parts {
                    if let Some((key, val)) = parse_attribute(part) {
                        tag.attributes.insert(key, val);
                    }
                }
                tags.push(tag.clone());
                current_tag = None;
                continue;
            }
            
            // Check if line ends with > (closing the tag)
            if trimmed.ends_with('>') {
                // Parse attributes before the >
                let content = &trimmed[..trimmed.len()-1];
                let parts: Vec<&str> = content.split_whitespace().collect();
                for part in parts {
                    if let Some((key, val)) = parse_attribute(part) {
                        tag.attributes.insert(key, val);
                    }
                }
                tags.push(tag.clone());
                current_tag = None;
                continue;
            }
            
            // Parse attributes from this line
            let parts: Vec<&str> = trimmed.split_whitespace().collect();
            for part in parts {
                if let Some((key, val)) = parse_attribute(part) {
                    tag.attributes.insert(key, val);
                }
            }
            continue;
        }
        
        // Try to parse a new opening tag
        if trimmed.starts_with('<') {
            let content = &trimmed[1..];
            
            // Check if it's a closing tag
            if content.starts_with('/') {
                continue;
            }
            
            // Check for self-closing tag like <tag ... />
            if content.ends_with("/>") {
                let inner = &content[..content.len()-2];
                let parts: Vec<&str> = inner.split_whitespace().collect();
                if parts.is_empty() {
                    continue;
                }
                
                let tag_name = parts[0].to_string();
                let mut attributes = HashMap::new();
                
                for part in &parts[1..] {
                    if let Some((key, val)) = parse_attribute(part) {
                        attributes.insert(key, val);
                    }
                }
                
                tags.push(ConfigTag {
                    name: tag_name,
                    source: FilePosition {
                        name: path.to_string(),
                        line: current_line,
                        column: 0,
                    },
                    attributes,
                });
                continue;
            }
            
            // It's an opening tag that might span multiple lines
            // Remove the '>' if present at the end
            let inner = if content.ends_with('>') {
                &content[..content.len()-1]
            } else {
                content
            };
            
            let parts: Vec<&str> = inner.split_whitespace().collect();
            if parts.is_empty() {
                continue;
            }
            
            let tag_name = parts[0].to_string();
            let mut attributes = HashMap::new();
            
            for part in &parts[1..] {
                if let Some((key, val)) = parse_attribute(part) {
                    attributes.insert(key, val);
                }
            }
            
            current_tag = Some(ConfigTag {
                name: tag_name,
                source: FilePosition {
                    name: path.to_string(),
                    line: current_line,
                    column: 0,
                },
                attributes,
            });
        }
    }
    
    // Push the last tag if any
    if let Some(tag) = current_tag {
        tags.push(tag);
    }
    
    Ok(tags)
}

/// Parses a config file and fills a ServerConfig struct
pub fn fill_server_config_from_file(config: &mut ServerConfig, path: &str) -> Result<(), String> {
    let tags = parse_config_file(path)?;
    
    // Find and process <server> tag
    for tag in &tags {
        match tag.name.as_str() {
            "server" => {
                if let Some(name) = tag.attributes.get("name") {
                    config.server_name = name.clone();
                }
                if let Some(id) = tag.attributes.get("id") {
                    config.server_id = id.clone();
                }
                if let Some(desc) = tag.attributes.get("description") {
                    config.server_desc = desc.clone();
                }
                if let Some(network) = tag.attributes.get("network") {
                    config.network = network.clone();
                }
            }
            "options" => {
                if let Some(default_modes) = tag.attributes.get("defaultmodes") {
                    config.default_modes = default_modes.clone();
                }
                config.mask_in_list = tag.get_bool("maskinlist", false);
                config.mask_in_topic = tag.get_bool("maskintopic", tag.get_bool("hostintopic", false));
                config.no_snotice_stack = tag.get_bool("nosnoticestack", false);
                config.syntax_hints = tag.get_bool("syntaxhints", false);
                if let Some(xline_msg) = tag.attributes.get("xlinemessage") {
                    config.xline_message = xline_msg.clone();
                }
                if let Some(xline_quit) = tag.attributes.get("xlinequit") {
                    config.xline_quit = xline_quit.clone();
                }
                // Parse restrictbannedusers enum
                if let Some(reflect) = tag.attributes.get("restrictbannedusers") {
                    config.restrict_banned_users = match reflect.to_lowercase().as_str() {
                        "no" => BannedUserTreatment::Normal,
                        "silent" => BannedUserTreatment::RestrictSilent,
                        "yes" => BannedUserTreatment::RestrictNotify,
                        _ => BannedUserTreatment::RestrictNotify,
                    };
                }
                // Parse defaultbind enum - for now use true as default
                config.wildcard_ipv6 = tag.get_bool("defaultbind", true);
            }
            "performance" => {
                config.max_conn = tag.get_int("somaxconn", 128);
                config.net_buffer_size = tag.get_int("netbuffersize", 10240);
                config.soft_limit = tag.get_int("softlimit", 1024);
                if let Some(warn_str) = tag.attributes.get("timeskipwarn") {
                    if let Ok(warn) = warn_str.parse() {
                        config.time_skip_warn = warn;
                    }
                }
            }
            "security" => {
                if let Some(custom_version) = tag.attributes.get("customversion") {
                    config.custom_version = custom_version.clone();
                }
                if let Some(hide_server) = tag.attributes.get("hideserver") {
                    config.hide_server = hide_server.clone();
                }
                config.max_targets = tag.get_int("maxtargets", 5);
                if let Some(public_quit) = tag.attributes.get("publicxlinequit") {
                    config.xline_quit_public = public_quit.clone();
                }
            }
            "cidr" => {
                config.ipv4_range = tag.get_int("ipv4clone", 32);
                config.ipv6_range = tag.get_int("ipv6clone", 128);
            }
            "limits" => {
                config.limits.max_line = tag.get_int("maxline", 512);
                config.limits.max_nick = tag.get_int("maxnick", 30);
                config.limits.max_channel = tag.get_int("maxchan", 60);
                config.limits.max_modes = tag.get_int("maxmodes", 20);
                config.limits.max_user = tag.get_int("maxuser", 10);
                config.limits.max_quit = tag.get_int("maxquit", 300);
                config.limits.max_topic = tag.get_int("maxtopic", 330);
                config.limits.max_kick = tag.get_int("maxkick", 300);
                config.limits.max_real = tag.get_int("maxreal", 130);
                config.limits.max_away = tag.get_int("maxaway", 200);
                config.limits.max_host = tag.get_int("maxhost", 64);
                config.limits.max_key = tag.get_int("maxkey", 32);
            }
            "path" => {
                if let Some(config_dir) = tag.attributes.get("configdir") {
                    config.paths.config = config_dir.clone();
                }
                if let Some(data_dir) = tag.attributes.get("datadir") {
                    config.paths.data = data_dir.clone();
                }
                if let Some(log_dir) = tag.attributes.get("logdir") {
                    config.paths.log = log_dir.clone();
                }
                if let Some(module_dir) = tag.attributes.get("moduledir") {
                    config.paths.module = module_dir.clone();
                }
                if let Some(runtime_dir) = tag.attributes.get("runtimedir") {
                    config.paths.runtime = runtime_dir.clone();
                }
            }
            _ => {}
        }
    }
    
    config.valid = true;
    Ok(())
}

/// Parses an attribute like key="value" or key='value'
fn parse_attribute(attr: &str) -> Option<(String, String)> {
    let parts: Vec<&str> = attr.splitn(2, '=').collect();
    if parts.len() != 2 {
        return None;
    }
    
    let key = parts[0].trim().to_string();
    let value_str = parts[1].trim();
    
    // Remove quotes
    let value = if value_str.starts_with('"') && value_str.ends_with('"') {
        value_str[1..value_str.len()-1].to_string()
    } else if value_str.starts_with('\'') && value_str.ends_with('\'') {
        value_str[1..value_str.len()-1].to_string()
    } else {
        value_str.to_string()
    };
    
    Some((key, value))
}

/// FFI function to read and parse a config file
#[unsafe(no_mangle)]
pub extern "C" fn serverconfig_read_config_file(
    path: *const c_char,
) -> *mut Vec<ConfigTag> {
    if path.is_null() {
        return std::ptr::null_mut();
    }
    
    let path_str = unsafe { CStr::from_ptr(path) }.to_string_lossy();
    
    match parse_config_file(&path_str) {
        Ok(tags) => Box::into_raw(Box::new(tags)),
        Err(e) => {
            error!("Failed to parse config file: {}", e);
            std::ptr::null_mut()
        }
    }
}

/// FFI function to free a vector of ConfigTags
#[unsafe(no_mangle)]
pub extern "C" fn serverconfig_free_tags(ptr: *mut Vec<ConfigTag>) {
    if !ptr.is_null() {
        unsafe { Box::from_raw(ptr); }
    }
}

/// FFI function to free a string allocated by Rust
#[unsafe(no_mangle)]
pub extern "C" fn serverconfig_free_string(ptr: *mut c_char) {
    if !ptr.is_null() {
        unsafe { CString::from_raw(ptr); }
    }
}

/// FFI function to parse a config file and return a new ServerConfig
/// Returns a pointer to a newly allocated ServerConfig filled from the file
#[unsafe(no_mangle)]
pub extern "C" fn serverconfig_parse_file(
    path: *const c_char,
) -> *mut ServerConfig {
    if path.is_null() {
        return std::ptr::null_mut();
    }
    
    let path_str = unsafe { CStr::from_ptr(path) }.to_string_lossy();
    
    match parse_config_file(&path_str) {
        Ok(tags) => {
            info!("Rust config parser: successfully parsed {} tags from {}", tags.len(), path_str);
            // Fill a Rust config with the parsed data
            let mut config = ServerConfig::default();
            if let Err(e) = fill_server_config_from_file(&mut config, &path_str) {
                error!("Failed to fill config: {}", e);
                return std::ptr::null_mut();
            }
            // Log some values to show it works
            info!("Rust config parser: server_name={}, server_id={}, network={}", 
                  config.server_name, config.server_id, config.network);
            Box::into_raw(Box::new(config))
        }
        Err(e) => {
            error!("Failed to parse config file: {}", e);
            std::ptr::null_mut()
        }
    }
}

/// FFI function to validate a server ID
#[unsafe(no_mangle)]
pub extern "C" fn serverconfig_is_sid(sid: *const c_char) -> c_int {
    if sid.is_null() {
        return 0;
    }
    
    let sid_str = unsafe { CStr::from_ptr(sid) }.to_string_lossy();
    // A server ID must be 3 characters long, with the first character a digit 
    // and the next two characters a digit or letter
    if sid_str.len() == 3 {
        let chars: Vec<char> = sid_str.chars().collect();
        if chars[0].is_ascii_digit() 
            && chars[1].is_ascii_alphanumeric() 
            && chars[2].is_ascii_alphanumeric() {
            return 1;
        }
    }
    0
}

/// FFI function to get hostname
#[unsafe(no_mangle)]
pub extern "C" fn serverconfig_get_hostname() -> *mut c_char {
    let hostname = if let Ok(listener) = std::net::TcpListener::bind("0.0.0.0:0") {
        if let Ok(addr) = listener.local_addr() {
            addr.ip().to_string()
        } else {
            "irc.example.com".to_string()
        }
    } else {
        "irc.example.com".to_string()
    };
    
    CString::new(hostname).unwrap().into_raw()
}
