// This file is a Rust port of a corresponding InspIRCd module.
// Original work Copyright (C) the InspIRCd contributors.
// Licensed under GPLv2. See LICENSE for details.

use std::ffi::{c_char, c_int, CStr, CString};
use std::fs;
use std::collections::HashMap;
use serde::{Deserialize, Serialize};
use toml;
use tracing::{error, warn, info, debug};
use crate::serverconfig::{ConfigTag, FilePosition};

/// Represents a bind configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BindConfig {
    pub address: Option<String>,
    pub port: Option<String>,
    #[serde(rename = "type")]
    pub bind_type: Option<String>,
    pub sslprofile: Option<String>,
    pub defer: Option<String>,
    pub free: Option<String>,
    pub path: Option<String>,
    pub permissions: Option<String>,
    pub replace: Option<String>,
    pub hook: Option<String>,
}

/// Represents a connect class configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConnectClassConfig {
    pub name: Option<String>,
    pub parent: Option<String>,
    pub allow: Option<String>,
    pub deny: Option<String>,
    pub reason: Option<String>,
    pub hash: Option<String>,
    pub password: Option<String>,
    pub maxchans: Option<String>,
    pub timeout: Option<String>,
    pub localmax: Option<String>,
    pub globalmax: Option<String>,
    pub maxconnwarn: Option<String>,
    pub resolvehostnames: Option<String>,
    pub useconnectban: Option<String>,
    pub useconnflood: Option<String>,
}

/// Represents server limits configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LimitsConfig {
    pub maxline: Option<usize>,
    pub maxnick: Option<usize>,
    pub maxchan: Option<usize>,
    pub maxmodes: Option<usize>,
    pub maxuser: Option<usize>,
    pub maxquit: Option<usize>,
    pub maxtopic: Option<usize>,
    pub maxkick: Option<usize>,
    pub maxreal: Option<usize>,
    pub maxaway: Option<usize>,
    pub maxhost: Option<usize>,
    pub maxkey: Option<usize>,
}

/// Represents server paths configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PathsConfig {
    pub configdir: Option<String>,
    pub datadir: Option<String>,
    pub logdir: Option<String>,
    pub moduledir: Option<String>,
    pub runtimedir: Option<String>,
}

/// Represents performance configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceConfig {
    pub somaxconn: Option<i32>,
    pub netbuffersize: Option<usize>,
    pub softlimit: Option<usize>,
    pub timeskipwarn: Option<u64>,
}

/// Represents security configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityConfig {
    pub customversion: Option<String>,
    pub hideserver: Option<String>,
    pub maxtargets: Option<usize>,
    pub publicxlinequit: Option<String>,
    pub hidebans: Option<bool>,
}

/// Represents CIDR configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CidrConfig {
    pub ipv4clone: Option<u8>,
    pub ipv6clone: Option<u8>,
}

/// Represents options configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OptionsConfig {
    pub defaultmodes: Option<String>,
    pub maskinlist: Option<bool>,
    pub maskintopic: Option<bool>,
    pub hostintopic: Option<bool>,
    pub nosnoticestack: Option<bool>,
    pub syntaxhints: Option<bool>,
    pub xlinemessage: Option<String>,
    pub xlinequit: Option<String>,
    pub restrictbannedusers: Option<String>,
    pub defaultbind: Option<String>,
}

/// Represents admin configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdminConfig {
    pub name: Option<String>,
    pub description: Option<String>,
    pub email: Option<String>,
}

/// Represents SSL profile configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SslProfileConfig {
    pub name: Option<String>,
    #[serde(rename = "type")]
    pub profile_type: Option<String>,
    pub certfile: Option<String>,
    pub keyfile: Option<String>,
    pub cafile: Option<String>,
    pub capath: Option<String>,
    pub dhfile: Option<String>,
    pub cipher: Option<String>,
    pub protocol: Option<String>,
}

/// Represents module configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModuleConfig {
    pub name: Option<String>,
    pub args: Option<String>,
}

/// Main configuration structure that matches TOML format
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TomlServerConfig {
    pub server: Option<ServerConfig>,
    pub admin: Option<AdminConfig>,
    pub binds: Option<Vec<BindConfig>>,
    pub connect: Option<Vec<ConnectClassConfig>>,
    pub limits: Option<LimitsConfig>,
    pub paths: Option<PathsConfig>,
    pub performance: Option<PerformanceConfig>,
    pub security: Option<SecurityConfig>,
    pub cidr: Option<CidrConfig>,
    pub options: Option<OptionsConfig>,
    pub modules: Option<Vec<ModuleConfig>>,
    pub sslprofiles: Option<Vec<SslProfileConfig>>,
    pub defines: Option<std::collections::HashMap<String, String>>,
    pub dns: Option<DnsConfig>,
    pub whowas: Option<WhowasConfig>,
    pub files: Option<FilesConfig>,
    pub insane: Option<InsaneConfig>,
    pub badips: Option<Vec<BadIpConfig>>,
    pub badnicks: Option<Vec<BadNickConfig>>,
    pub badhosts: Option<Vec<BadHostConfig>>,
    pub exceptions: Option<Vec<ExceptionConfig>>,
    pub maxlists: Option<Vec<MaxListConfig>>,
}

/// DNS configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DnsConfig {
    pub timeout: Option<u64>,
}

/// Whowas configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WhowasConfig {
    pub groupsize: Option<usize>,
    pub maxgroups: Option<usize>,
    pub maxkeep: Option<String>,
    pub nickupdate: Option<bool>,
}

/// Files configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FilesConfig {
    pub motd: Option<String>,
}

/// Insane configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InsaneConfig {
    pub hostmasks: Option<bool>,
    pub ipmasks: Option<bool>,
    pub nickmasks: Option<bool>,
    pub trigger: Option<String>,
}

/// Bad IP configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BadIpConfig {
    pub ipmask: Option<String>,
    pub reason: Option<String>,
}

/// Bad nick configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BadNickConfig {
    pub nick: Option<String>,
    pub reason: Option<String>,
}

/// Bad host configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BadHostConfig {
    pub host: Option<String>,
    pub reason: Option<String>,
}

/// Exception configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExceptionConfig {
    pub host: Option<String>,
    pub reason: Option<String>,
}

/// MaxList configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MaxListConfig {
    pub mode: Option<String>,
    pub chan: Option<String>,
    pub limit: Option<usize>,
}

/// Server configuration from TOML
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerConfig {
    pub name: Option<String>,
    pub id: Option<String>,
    pub description: Option<String>,
    pub network: Option<String>,
}

/// Main configuration structure for FFI
#[repr(C)]
#[derive(Debug)]
pub struct ParsedServerConfig {
    // Server identity
    pub server_name: String,
    pub server_id: String,
    pub server_desc: String,
    pub network: String,
    
    // Admin info
    pub admin_name: String,
    pub admin_desc: String,
    pub admin_email: String,
    
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
    
    // Banned user treatment (as string for now, will be converted in C++)
    pub restrict_banned_users: String,
    
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
    
    // Paths
    pub config_path: String,
    pub data_path: String,
    pub log_path: String,
    pub module_path: String,
    pub runtime_path: String,
    
    // Bind configurations
    pub binds: Vec<BindConfig>,
    
    // Connect classes
    pub connect_classes: Vec<ConnectClassConfig>,
    
    // Modules
    pub modules: Vec<String>,
    
    // Defines (variables)
    pub defines: std::collections::HashMap<String, String>,
    
    // Whether config is valid
    pub valid: bool,
    
    // Config file path
    pub config_file_name: String,
}

impl Default for ParsedServerConfig {
    fn default() -> Self {
        Self {
            server_name: String::new(),
            server_id: String::new(),
            server_desc: String::new(),
            network: String::new(),
            admin_name: String::new(),
            admin_desc: String::new(),
            admin_email: String::new(),
            default_modes: "not".to_string(),
            mask_in_list: false,
            mask_in_topic: false,
            no_snotice_stack: false,
            syntax_hints: false,
            xline_message: "You're banned!".to_string(),
            xline_quit: "%fulltype%: %reason%".to_string(),
            xline_quit_public: String::new(),
            restrict_banned_users: "yes".to_string(),
            wildcard_ipv6: true,
            max_conn: 128,
            net_buffer_size: 10240,
            soft_limit: 1024,
            time_skip_warn: 2,
            custom_version: String::new(),
            hide_server: String::new(),
            max_targets: 5,
            ipv4_range: 32,
            ipv6_range: 128,
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
            config_path: "./run/conf".to_string(),
            data_path: "./run/data".to_string(),
            log_path: "./run/logs".to_string(),
            module_path: "./run/modules".to_string(),
            runtime_path: "./run/data".to_string(),
            binds: Vec::new(),
            connect_classes: Vec::new(),
            modules: Vec::new(),
            defines: std::collections::HashMap::new(),
            valid: false,
            config_file_name: String::new(),
        }
    }
}

impl ParsedServerConfig {
    pub fn new() -> Self {
        Self::default()
    }
    
    /// Fill in default values
    pub fn fill_defaults(&mut self) {
        // Defaults are already set in the Default implementation
    }
    
    /// Parse TOML config file and fill the struct
    pub fn parse_toml_file(path: &str) -> Result<Self, String> {
        let content = fs::read_to_string(path)
            .map_err(|e| format!("Failed to read config file {}: {}", path, e))?;
        
        let toml_config: TomlServerConfig = toml::from_str(&content)
            .map_err(|e| format!("Failed to parse TOML config from {}: {}", path, e))?;
        
        let mut config = Self::default();
        config.config_file_name = path.to_string();
        
        // Process defines (variables)
        if let Some(defines) = toml_config.defines {
            config.defines = defines;
        }
        
        // Process server section
        if let Some(server) = toml_config.server {
            if let Some(name) = server.name {
                // Apply variable substitution
                config.server_name = apply_variable_substitution(&name, &config.defines);
            }
            if let Some(id) = server.id {
                config.server_id = id;
            }
            if let Some(desc) = server.description {
                config.server_desc = apply_variable_substitution(&desc, &config.defines);
            }
            if let Some(network) = server.network {
                config.network = apply_variable_substitution(&network, &config.defines);
            }
        }
        
        // Process admin section
        if let Some(admin) = toml_config.admin {
            if let Some(name) = admin.name {
                config.admin_name = apply_variable_substitution(&name, &config.defines);
            }
            if let Some(desc) = admin.description {
                config.admin_desc = apply_variable_substitution(&desc, &config.defines);
            }
            if let Some(email) = admin.email {
                config.admin_email = apply_variable_substitution(&email, &config.defines);
            }
        }
        
        // Process options section
        if let Some(options) = toml_config.options {
            if let Some(defaultmodes) = options.defaultmodes {
                config.default_modes = apply_variable_substitution(&defaultmodes, &config.defines);
            }
            config.mask_in_list = options.maskinlist.unwrap_or(false);
            config.mask_in_topic = options.maskintopic.unwrap_or(options.hostintopic.unwrap_or(false));
            config.no_snotice_stack = options.nosnoticestack.unwrap_or(false);
            config.syntax_hints = options.syntaxhints.unwrap_or(false);
            if let Some(xlinemessage) = options.xlinemessage {
                config.xline_message = apply_variable_substitution(&xlinemessage, &config.defines);
            }
            if let Some(xlinequit) = options.xlinequit {
                config.xline_quit = apply_variable_substitution(&xlinequit, &config.defines);
            }
            if let Some(restrictbannedusers) = options.restrictbannedusers {
                config.restrict_banned_users = restrictbannedusers;
            }
            if let Some(defaultbind) = options.defaultbind {
                config.wildcard_ipv6 = parse_bool(&defaultbind, true);
            }
        }
        
        // Process limits section
        if let Some(limits) = toml_config.limits {
            config.max_line = limits.maxline.unwrap_or(512);
            config.max_nick = limits.maxnick.unwrap_or(30);
            config.max_channel = limits.maxchan.unwrap_or(60);
            config.max_modes = limits.maxmodes.unwrap_or(20);
            config.max_user = limits.maxuser.unwrap_or(10);
            config.max_quit = limits.maxquit.unwrap_or(300);
            config.max_topic = limits.maxtopic.unwrap_or(330);
            config.max_kick = limits.maxkick.unwrap_or(300);
            config.max_real = limits.maxreal.unwrap_or(130);
            config.max_away = limits.maxaway.unwrap_or(200);
            config.max_host = limits.maxhost.unwrap_or(64);
            config.max_key = limits.maxkey.unwrap_or(32);
        }
        
        // Process paths section
        if let Some(paths) = toml_config.paths {
            if let Some(configdir) = paths.configdir {
                config.config_path = apply_variable_substitution(&configdir, &config.defines);
            }
            if let Some(datadir) = paths.datadir {
                config.data_path = apply_variable_substitution(&datadir, &config.defines);
            }
            if let Some(logdir) = paths.logdir {
                config.log_path = apply_variable_substitution(&logdir, &config.defines);
            }
            if let Some(moduledir) = paths.moduledir {
                config.module_path = apply_variable_substitution(&moduledir, &config.defines);
            }
            if let Some(runtimedir) = paths.runtimedir {
                config.runtime_path = apply_variable_substitution(&runtimedir, &config.defines);
            }
        }
        
        // Process performance section
        if let Some(performance) = toml_config.performance {
            config.max_conn = performance.somaxconn.unwrap_or(128);
            config.net_buffer_size = performance.netbuffersize.unwrap_or(10240);
            config.soft_limit = performance.softlimit.unwrap_or(1024);
            config.time_skip_warn = performance.timeskipwarn.unwrap_or(2);
        }
        
        // Process security section
        if let Some(security) = toml_config.security {
            if let Some(customversion) = security.customversion {
                config.custom_version = apply_variable_substitution(&customversion, &config.defines);
            }
            if let Some(hideserver) = security.hideserver {
                config.hide_server = apply_variable_substitution(&hideserver, &config.defines);
            }
            config.max_targets = security.maxtargets.unwrap_or(5);
            if let Some(publicxlinequit) = security.publicxlinequit {
                config.xline_quit_public = apply_variable_substitution(&publicxlinequit, &config.defines);
            }
            if let Some(hidebans) = security.hidebans {
                if hidebans {
                    config.xline_quit_public = "%fulltype%".to_string();
                }
            }
        }
        
        // Process CIDR section
        if let Some(cidr) = toml_config.cidr {
            config.ipv4_range = cidr.ipv4clone.unwrap_or(32);
            config.ipv6_range = cidr.ipv6clone.unwrap_or(128);
        }
        
        // Process bind configurations
        if let Some(binds) = toml_config.binds {
            for bind in binds {
                let mut processed_bind = bind;
                // Apply variable substitution to bind fields
                if let Some(address) = &mut processed_bind.address {
                    *address = apply_variable_substitution(address, &config.defines);
                }
                if let Some(port) = &mut processed_bind.port {
                    *port = apply_variable_substitution(port, &config.defines);
                }
                if let Some(sslprofile) = &mut processed_bind.sslprofile {
                    *sslprofile = apply_variable_substitution(sslprofile, &config.defines);
                }
                config.binds.push(processed_bind);
            }
        }
        
        // Process connect classes
        if let Some(connect_classes) = toml_config.connect {
            for connect_class in connect_classes {
                let mut processed_connect = connect_class;
                // Apply variable substitution to connect class fields
                if let Some(allow) = &mut processed_connect.allow {
                    *allow = apply_variable_substitution(allow, &config.defines);
                }
                if let Some(deny) = &mut processed_connect.deny {
                    *deny = apply_variable_substitution(deny, &config.defines);
                }
                if let Some(reason) = &mut processed_connect.reason {
                    *reason = apply_variable_substitution(reason, &config.defines);
                }
                config.connect_classes.push(processed_connect);
            }
        }
        
        // Process modules
        if let Some(module_configs) = toml_config.modules {
            for module_config in module_configs {
                if let Some(name) = module_config.name {
                    let name = apply_variable_substitution(&name, &config.defines);
                    config.modules.push(name);
                }
            }
        }
        
        config.valid = true;
        
        info!("TOML config parser: successfully parsed config from {}", path);
        debug!("Server: name={}, id={}, network={}", 
               config.server_name, config.server_id, config.network);
        
        Ok(config)
    }
}

/// Apply variable substitution to a string using the defines map
/// Supports &varname; syntax
fn apply_variable_substitution(s: &str, defines: &std::collections::HashMap<String, String>) -> String {
    let mut result = s.to_string();
    let mut changed = true;
    
    // Keep applying substitutions until no more changes
    while changed {
        changed = false;
        let mut new_result = String::new();
        let mut chars = result.chars().peekable();
        
        while let Some(c) = chars.next() {
            if c == '&' {
                // Look for variable name
                let mut var_name = String::new();
                let mut found_semicolon = false;
                
                while let Some(&next_c) = chars.peek() {
                    if next_c == ';' {
                        chars.next(); // consume the semicolon
                        found_semicolon = true;
                        break;
                    } else if next_c == '&' {
                        // Another & before ; - this is literal &
                        break;
                    } else {
                        var_name.push(next_c);
                        chars.next();
                    }
                }
                
                if found_semicolon && !var_name.is_empty() {
                    // Handle predefined variables
                    let value = match var_name.as_str() {
                        "dir.config" => "./run/conf".to_string(),
                        "dir.example" => "./run/conf/examples".to_string(),
                        "dir.data" => "./run/data".to_string(),
                        "dir.log" => "./run/logs".to_string(),
                        "dir.module" => "./run/modules".to_string(),
                        "dir.runtime" => "./run/data".to_string(),
                        "networkDomain" => defines.get("networkDomain").cloned().unwrap_or_else(|| "example.com".to_string()),
                        "networkName" => defines.get("networkName").cloned().unwrap_or_else(|| "ExampleNet".to_string()),
                        _ => defines.get(&var_name).cloned().unwrap_or_else(|| "&".to_string() + &var_name + ";"),
                    };
                    new_result.push_str(&value);
                    changed = true;
                } else {
                    // Not a valid variable reference, keep the & and any consumed chars
                    new_result.push('&');
                    new_result.push_str(&var_name);
                    if found_semicolon {
                        new_result.push(';');
                    }
                }
            } else {
                new_result.push(c);
            }
        }
        
        result = new_result;
    }
    
    result
}

/// Parse boolean from string
fn parse_bool(value: &str, default: bool) -> bool {
    if value.is_empty() {
        return default;
    }
    match value.to_lowercase().as_str() {
        "yes" | "true" | "on" => true,
        "no" | "false" | "off" => false,
        _ => default,
    }
}

// FFI functions for C++ interop

/// FFI function to parse a TOML config file and return a new ParsedServerConfig
#[unsafe(no_mangle)]
pub extern "C" fn configtoml_parse_file(path: *const c_char) -> *mut ParsedServerConfig {
    if path.is_null() {
        return std::ptr::null_mut();
    }
    
    let path_str = unsafe { CStr::from_ptr(path) }.to_string_lossy();
    
    match ParsedServerConfig::parse_toml_file(&path_str) {
        Ok(config) => Box::into_raw(Box::new(config)),
        Err(e) => {
            error!("TOML config parser: failed to parse {}: {}", path_str, e);
            std::ptr::null_mut()
        }
    }
}

/// FFI function to free a ParsedServerConfig allocated in Rust
#[unsafe(no_mangle)]
pub extern "C" fn configtoml_free(ptr: *mut ParsedServerConfig) {
    if !ptr.is_null() {
        unsafe { Box::from_raw(ptr); }
    }
}

/// FFI function to get a string value from ParsedServerConfig
#[unsafe(no_mangle)]
pub extern "C" fn configtoml_get_string(
    ptr: *const ParsedServerConfig,
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
        "admin_name" => config.admin_name.as_str(),
        "admin_desc" => config.admin_desc.as_str(),
        "admin_email" => config.admin_email.as_str(),
        "default_modes" => config.default_modes.as_str(),
        "xline_message" => config.xline_message.as_str(),
        "xline_quit" => config.xline_quit.as_str(),
        "xline_quit_public" => config.xline_quit_public.as_str(),
        "custom_version" => config.custom_version.as_str(),
        "hide_server" => config.hide_server.as_str(),
        "restrict_banned_users" => config.restrict_banned_users.as_str(),
        "config_path" => config.config_path.as_str(),
        "data_path" => config.data_path.as_str(),
        "log_path" => config.log_path.as_str(),
        "module_path" => config.module_path.as_str(),
        "runtime_path" => config.runtime_path.as_str(),
        "config_file_name" => config.config_file_name.as_str(),
        _ => {
            warn!("Unknown ParsedServerConfig field: {}", field);
            return std::ptr::null_mut();
        }
    };
    
    CString::new(value).map(|s| s.into_raw()).unwrap_or(std::ptr::null_mut())
}

/// FFI function to get a boolean value from ParsedServerConfig
#[unsafe(no_mangle)]
pub extern "C" fn configtoml_get_bool(
    ptr: *const ParsedServerConfig,
    field_name: *const c_char,
) -> c_int {
    if ptr.is_null() || field_name.is_null() {
        return 0;
    }
    
    let config = unsafe { &*ptr };
    let field = unsafe { CStr::from_ptr(field_name) }.to_string_lossy();
    
    match field.as_ref() {
        "mask_in_list" => config.mask_in_list as c_int,
        "mask_in_topic" => config.mask_in_topic as c_int,
        "no_snotice_stack" => config.no_snotice_stack as c_int,
        "syntax_hints" => config.syntax_hints as c_int,
        "wildcard_ipv6" => config.wildcard_ipv6 as c_int,
        "valid" => config.valid as c_int,
        _ => {
            warn!("Unknown boolean field: {}", field);
            0
        }
    }
}

/// FFI function to get an integer value from ParsedServerConfig
#[unsafe(no_mangle)]
pub extern "C" fn configtoml_get_int(
    ptr: *const ParsedServerConfig,
    field_name: *const c_char,
) -> c_int {
    if ptr.is_null() || field_name.is_null() {
        return 0;
    }
    
    let config = unsafe { &*ptr };
    let field = unsafe { CStr::from_ptr(field_name) }.to_string_lossy();
    
    match field.as_ref() {
        "max_conn" => config.max_conn,
        "soft_limit" => config.soft_limit as c_int,
        "max_targets" => config.max_targets as c_int,
        "max_line" => config.max_line as c_int,
        "max_nick" => config.max_nick as c_int,
        "max_channel" => config.max_channel as c_int,
        "max_modes" => config.max_modes as c_int,
        "max_user" => config.max_user as c_int,
        "max_quit" => config.max_quit as c_int,
        "max_topic" => config.max_topic as c_int,
        "max_kick" => config.max_kick as c_int,
        "max_real" => config.max_real as c_int,
        "max_away" => config.max_away as c_int,
        "max_host" => config.max_host as c_int,
        "max_key" => config.max_key as c_int,
        "net_buffer_size" => config.net_buffer_size as c_int,
        "ipv4_range" => config.ipv4_range as c_int,
        "ipv6_range" => config.ipv6_range as c_int,
        _ => {
            warn!("Unknown integer field: {}", field);
            0
        }
    }
}

/// FFI function to get a u64 value from ParsedServerConfig
#[unsafe(no_mangle)]
pub extern "C" fn configtoml_get_u64(
    ptr: *const ParsedServerConfig,
    field_name: *const c_char,
) -> u64 {
    if ptr.is_null() || field_name.is_null() {
        return 0;
    }
    
    let config = unsafe { &*ptr };
    let field = unsafe { CStr::from_ptr(field_name) }.to_string_lossy();
    
    match field.as_ref() {
        "time_skip_warn" => config.time_skip_warn,
        "soft_limit" => config.soft_limit as u64,
        "net_buffer_size" => config.net_buffer_size as u64,
        _ => {
            warn!("Unknown u64 field: {}", field);
            0
        }
    }
}

/// FFI function to check if TOML config file exists and is valid
#[unsafe(no_mangle)]
pub extern "C" fn configtoml_file_exists(path: *const c_char) -> c_int {
    if path.is_null() {
        return 0;
    }
    
    let path_str = unsafe { CStr::from_ptr(path) }.to_string_lossy();
    let path_str_str = &*path_str; // Convert Cow<'_, str> to &str
    
    // First check if file exists
    if !std::path::Path::new(path_str_str).exists() {
        return 0;
    }
    
    // Try to parse it as TOML
    let content = match fs::read_to_string(path_str_str) {
        Ok(content) => content,
        Err(_) => return 0,
    };
    
    match toml::from_str::<TomlServerConfig>(&content) {
        Ok(_) => 1,
        Err(_) => 0,
    }
}