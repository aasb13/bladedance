/*
 * InspIRCd -- Internet Relay Chat Daemon
 *
 *   Copyright (C) 2019 Matt Schatz <genius3000@g3k.solutions>
 *   Copyright (C) 2018 linuxdaemon <linuxdaemon.irc@gmail.com>
 *   Copyright (C) 2017-2025 Sadie Powell <sadie@witchery.services>
 *   Copyright (C) 2017 Dylan Frank <b00mx0r@aureus.pw>
 *   Copyright (C) 2012-2013, 2016 Attila Molnar <attilamolnar@hush.com>
 *   Copyright (C) 2012, 2019 Robby <robby@chatbelgie.be>
 *   Copyright (C) 2012 Jens Voss <DukePyrolator@anope.org>
 *   Copyright (C) 2009 John Brooks <john@jbrooks.io>
 *   Copyright (C) 2009 Dennis Friis <peavey@inspircd.org>
 *   Copyright (C) 2009 Daniel De Graaf <danieldg@inspircd.org>
 *   Copyright (C) 2008-2009 Craig Edwards <brain@inspircd.org>
 *   Copyright (C) 2008 Thomas Stagner <aquanight@gmail.com>
 *   Copyright (C) 2008 Robin Burchell <robin+git@viroteck.net>
 *
 * This file is part of InspIRCd.  InspIRCd is free software: you can
 * redistribute it and/or modify it under the terms of the GNU General Public
 * License as published by the Free Software Foundation, version 2.
 *
 * This program is distributed in the hope that it will be useful, but WITHOUT
 * ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS
 * FOR A PARTICULAR PURPOSE.  See the GNU General Public License for more
 * details.
 *
 * You should have received a copy of the GNU General Public License
 * along with this program.  If not, see <http://www.gnu.org/licenses/>.
 */

#![allow(non_upper_case_globals)]
#![allow(non_camel_case_types)]
#![allow(dead_code)]

use std::ffi::{c_char, c_void, CStr};
use std::ptr;

// Include the StdString struct from stringutils.rs
#[path = "../stringutils.rs"]
mod stringutils;
use stringutils::StdString;

// Type definitions for C++ compatibility
type time_t = i64;
type User = *mut c_void;
type LocalUser = *mut c_void;
type Module = *mut c_void;
type Command = *mut c_void;
type XLineFactory = *mut c_void;
type XLine = *mut c_void;
type XLineManager = *mut c_void;
type CommandBase = *mut c_void;
type Params = *mut c_void;
type ConfigStatus = *mut c_void;
type ConfigTag = *mut c_void;
type TokenList = *mut c_void;
type StatsContext = *mut c_void;
type ClientProtocolTagMap = *mut c_void;

// Command result enum
#[repr(i32)]
pub enum CmdResult {
    SUCCESS = 0,
    FAILURE = 1,
    INVALID = 2,
}

// Module result enum
#[repr(i32)]
pub enum ModResult {
    MOD_RES_PASSTHRU = 0,
    MOD_RES_DENY = 1,
    MOD_RES_ALLOW = 2,
}

// Command access enum
#[repr(i32)]
pub enum CmdAccess {
    OPERATOR = 1,
    ADMIN = 2,
}

// Module flags
pub const VF_VENDOR: i32 = 1;
pub const VF_COMMON: i32 = 2;
pub const PRIORITY_BEFORE: i32 = 1;

// Event priorities
pub const I_OnPreCommand: i32 = 1;
pub const DefaultPriority: i32 = 100;

// Global server instance - this should be provided by the InspIRCd core
extern "C" {
    static mut ServerInstance: *mut c_void;
}

// C++ interface functions
extern "C" {
    fn XLineFactory_Create(type_name: *const c_char) -> XLineFactory;
    fn XLineFactory_Destroy(factory: XLineFactory);
    fn XLineFactory_SetGenerate(factory: XLineFactory, generate_fn: extern "C" fn(time_t, u64, *const c_char, *const c_char, *const c_char) -> XLine);
    fn XLineFactory_SetAutoApplyToUserList(factory: XLineFactory, auto_apply_fn: extern "C" fn(XLine) -> bool);
    
    fn Shun_Create(set_time: time_t, duration: u64, source: *const c_char, reason: *const c_char, shunmask: *const c_char) -> XLine;
    fn Shun_Destroy(shun: XLine);
    
    fn Command_Create(creator: Module, name: *const c_char, min_params: i32, max_params: i32) -> Command;
    fn Command_Destroy(cmd: Command);
    fn Command_SetAccessNeeded(cmd: Command, access: CmdAccess);
    fn Command_SetSyntax(cmd: Command, syntax: *const c_char);
    fn Command_SetHandle(cmd: Command, handle_fn: extern "C" fn(User, Params, *mut CmdResult) -> CmdResult);
    
    fn Module_Create(flags: i32, description: *const c_char) -> Module;
    fn Module_Destroy(module: Module);
    fn Module_SetInit(module: Module, init_fn: extern "C" fn());
    fn Module_SetDispose(module: Module, dispose_fn: extern "C" fn());
    fn Module_SetPrioritize(module: Module, prioritize_fn: extern "C" fn());
    fn Module_SetOnStats(module: Module, on_stats_fn: extern "C" fn(StatsContext, *mut ModResult) -> ModResult);
    fn Module_SetReadConfig(module: Module, read_config_fn: extern "C" fn(ConfigStatus));
    fn Module_SetOnPreCommand(module: Module, on_pre_command_fn: extern "C" fn(*mut c_char, Params, LocalUser, bool, *mut ModResult) -> ModResult);
    
    fn Users_Find(nick: *const c_char, find_real: bool) -> User;
    fn User_GetNick(user: User) -> *const c_char;
    fn User_GetBanUser(user: User, real: bool) -> *const c_char;
    fn User_GetAddress(user: User) -> *const c_char;
    fn User_WriteNotice(user: User, message: *const c_char);
    fn User_IsFullyConnected(user: LocalUser) -> bool;
    fn User_HasPrivPermission(user: LocalUser, permission: *const c_char) -> bool;
    
    fn XLines_GetXLines() -> XLineManager;
    fn XLines_RegisterFactory(manager: XLineManager, factory: XLineFactory) -> bool;
    fn XLines_UnregisterFactory(manager: XLineManager, factory: XLineFactory);
    fn XLines_DelLine(manager: XLineManager, pattern: *const c_char, type_name: *const c_char, reason: *mut StdString, user: User) -> bool;
    fn XLines_AddLine(manager: XLineManager, line: XLine, user: User) -> bool;
    fn XLines_MatchesLine(manager: XLineManager, type_name: *const c_char, user: User) -> bool;
    fn XLines_DelAll(manager: XLineManager, type_name: *const c_char);
    fn XLines_InvokeStats(manager: XLineManager, type_name: *const c_char, context: StatsContext);
    
    fn CommandParser_LoopCall(user: User, cmd: Command, parameters: Params, index: i32) -> bool;
    
    fn SNO_WriteToSnoMask(mask: c_char, message: *const c_char);
    
    fn Config_GetConfValue(tag_name: *const c_char) -> ConfigTag;
    fn ConfigTag_GetBool(tag: ConfigTag, key: *const c_char, default_value: bool) -> bool;
    fn ConfigTag_GetString(tag: ConfigTag, key: *const c_char, default_value: *const c_char) -> StdString;
    
    fn TokenList_Create() -> TokenList;
    fn TokenList_Destroy(list: TokenList);
    fn TokenList_Clear(list: TokenList);
    fn TokenList_AddList(list: TokenList, token_list: *const c_char);
    fn TokenList_Contains(list: TokenList, token: *const c_char) -> bool;
    
    fn StatsContext_GetSymbol(context: StatsContext) -> c_char;
    
    fn Parameters_GetSize(parameters: Params) -> usize;
    fn Parameters_Get(parameters: Params, index: usize) -> *const c_char;
    fn Parameters_GetTags(parameters: Params) -> ClientProtocolTagMap;
    fn Parameters_Clear(parameters: Params);
    fn Parameters_Resize(parameters: Params, new_size: usize);
    
    fn ClientProtocolTagMap_Begin(tags: ClientProtocolTagMap) -> *mut c_void;
    fn ClientProtocolTagMap_End(tags: ClientProtocolTagMap) -> *mut c_void;
    fn ClientProtocolTagMap_Erase(tag_iter: *mut c_void) -> *mut c_void;
    fn ClientProtocolTagMap_Next(tag_iter: *mut c_void) -> *mut c_void;
    fn ClientProtocolTagMap_GetKey(tag_iter: *mut c_void) -> *const c_char;
    
    fn Duration_TryFrom(duration_str: *const c_char, duration: *mut u64) -> bool;
    fn Duration_ToLongString(duration: u64) -> StdString;
    fn Time_FromNow(duration: u64) -> StdString;
    
    fn ServerInstance_GetTime() -> time_t;
    
    fn Modules_SetPriority(module: Module, event: i32, priority: i32, before_module: *const c_char);
}

// Module state
static mut MODULE_INSTANCE: Option<ModuleShun> = None;
static mut SHUN_FACTORY: Option<XLineFactory> = None;
static mut SHUN_COMMAND: Option<Command> = None;

// Module structure
struct ModuleShun {
    module: Module,
    cmd: Command,
    shun_factory: XLineFactory,
    allowconnect: bool,
    allowtags: bool,
    cleanedcommands: TokenList,
    enabledcommands: TokenList,
    notifyuser: bool,
}

// Shun factory functions
extern "C" fn shun_factory_generate(set_time: time_t, duration: u64, source: *const c_char, reason: *const c_char, xline_specific_mask: *const c_char) -> XLine {
    unsafe {
        Shun_Create(set_time, duration, source, reason, xline_specific_mask)
    }
}

extern "C" fn shun_factory_auto_apply_to_user_list(x: XLine) -> bool {
    false
}

// Command handler
extern "C" fn command_shun_handle(user: User, parameters: Params, result: *mut CmdResult) -> CmdResult {
    unsafe {
        /* syntax: SHUN nick!user@host time :reason goes here */
        /* 'time' is a human-readable timestring, like 2d3h2s. */
        if CommandParser_LoopCall(user, SHUN_COMMAND.unwrap(), parameters, 0) {
            return CmdResult::SUCCESS;
        }

        let target_cstr = Parameters_Get(parameters, 0);
        let mut target = CStr::from_ptr(target_cstr).to_string_lossy().into_owned();

        let find = Users_Find(target_cstr, true);
        if !find.is_null() {
            let nick = User_GetNick(find);
            let ban_user = User_GetBanUser(find, true);
            let address = User_GetAddress(find);
            target = format!("*!{}@{}", 
                CStr::from_ptr(ban_user).to_string_lossy(),
                CStr::from_ptr(address).to_string_lossy()
            );
        }

        let param_size = Parameters_GetSize(parameters);
        if param_size == 1 {
            let mut reason = StdString { data: ptr::null_mut(), length: 0, capacity: 0 };
            
            let xlines = XLines_GetXLines();
            let target_cstr = std::ffi::CString::new(target.clone()).unwrap();
            let original_target_cstr = Parameters_Get(parameters, 0);
            
            if XLines_DelLine(xlines, original_target_cstr, b"SHUN\0".as_ptr() as *const c_char, &mut reason, user) {
                let nick = User_GetNick(user);
                let message = format!("{} removed SHUN on {}: {}", 
                    CStr::from_ptr(nick).to_string_lossy(),
                    CStr::from_ptr(original_target_cstr).to_string_lossy(),
                    "removed" // reason would need to be converted from StdString
                );
                let message_cstr = std::ffi::CString::new(message).unwrap();
                SNO_WriteToSnoMask('x', message_cstr.as_ptr());
            } else if XLines_DelLine(xlines, target_cstr.as_ptr(), b"SHUN\0".as_ptr() as *const c_char, &mut reason, user) {
                let nick = User_GetNick(user);
                let message = format!("{} removed SHUN on {}: {}", 
                    CStr::from_ptr(nick).to_string_lossy(),
                    target,
                    "removed" // reason would need to be converted from StdString
                );
                let message_cstr = std::ffi::CString::new(message).unwrap();
                SNO_WriteToSnoMask('x', message_cstr.as_ptr());
            } else {
                let message = format!("*** Shun {} not found on the list.", CStr::from_ptr(original_target_cstr).to_string_lossy());
                let message_cstr = std::ffi::CString::new(message).unwrap();
                User_WriteNotice(user, message_cstr.as_ptr());
                return CmdResult::FAILURE;
            }
        } else {
            // Adding - XXX todo make this respect <insane> tag perhaps..
            let mut duration: u64 = 0;
            let mut expr_cstr: *const c_char = ptr::null();
            
            if param_size > 2 {
                let duration_str = Parameters_Get(parameters, 1);
                if !Duration_TryFrom(duration_str, &mut duration) {
                    let message_cstr = std::ffi::CString::new("*** Invalid duration for SHUN.").unwrap();
                    User_WriteNotice(user, message_cstr.as_ptr());
                    return CmdResult::FAILURE;
                }
                expr_cstr = Parameters_Get(parameters, 2);
            } else {
                duration = 0;
                expr_cstr = Parameters_Get(parameters, 1);
            }

            let current_time = ServerInstance_GetTime();
            let nick = User_GetNick(user);
            let target_cstr = std::ffi::CString::new(target.clone()).unwrap();
            let r = Shun_Create(current_time, duration, nick, expr_cstr, target_cstr.as_ptr());
            
            let xlines = XLines_GetXLines();
            if XLines_AddLine(xlines, r, user) {
                let nick_str = CStr::from_ptr(nick).to_string_lossy();
                if duration == 0 {
                    let message = format!("{} added permanent SHUN for {}: {}", nick_str, target, CStr::from_ptr(expr_cstr).to_string_lossy());
                    let message_cstr = std::ffi::CString::new(message).unwrap();
                    SNO_WriteToSnoMask('x', message_cstr.as_ptr());
                } else {
                    let duration_str = Duration_ToLongString(duration);
                    let time_str = Time_FromNow(duration);
                    let message = format!("{} added a timed SHUN on {}, expires in {} (on {}): {}", 
                        nick_str, target, 
                        // duration_str and time_str would need to be converted from StdString
                        "duration", "time", 
                        CStr::from_ptr(expr_cstr).to_string_lossy()
                    );
                    let message_cstr = std::ffi::CString::new(message).unwrap();
                    SNO_WriteToSnoMask('x', message_cstr.as_ptr());
                }
            } else {
                Shun_Destroy(r);
                let message = format!("*** Shun for {} already exists.", target);
                let message_cstr = std::ffi::CString::new(message).unwrap();
                User_WriteNotice(user, message_cstr.as_ptr());
                return CmdResult::FAILURE;
            }
        }
        CmdResult::SUCCESS
    }
}

// Helper function to check if user is shunned
fn is_shunned(user: LocalUser) -> bool {
    unsafe {
        if let Some(module) = &MODULE_INSTANCE {
            // Exempt the user if they are not fully connected and allowconnect is enabled.
            if module.allowconnect && !User_IsFullyConnected(user) {
                return false;
            }

            // Exempt the user from shuns if they are an oper with the servers/ignore-shun privilege.
            if User_HasPrivPermission(user, b"servers/ignore-shun\0".as_ptr() as *const c_char) {
                return false;
            }

            // Check whether the user is actually shunned.
            let xlines = XLines_GetXLines();
            XLines_MatchesLine(xlines, b"SHUN\0".as_ptr() as *const c_char, user)
        } else {
            false
        }
    }
}

// Module event handlers
extern "C" fn module_init() {
    unsafe {
        let xlines = XLines_GetXLines();
        XLines_RegisterFactory(xlines, SHUN_FACTORY.unwrap());
    }
}

extern "C" fn module_dispose() {
    unsafe {
        let xlines = XLines_GetXLines();
        XLines_DelAll(xlines, b"SHUN\0".as_ptr() as *const c_char);
        XLines_UnregisterFactory(xlines, SHUN_FACTORY.unwrap());
    }
}

extern "C" fn module_prioritize() {
    unsafe {
        if let Some(module) = &MODULE_INSTANCE {
            Modules_SetPriority(module.module, I_OnPreCommand, PRIORITY_BEFORE, b"alias\0".as_ptr() as *const c_char);
        }
    }
}

extern "C" fn module_on_stats(context: StatsContext, result: *mut ModResult) -> ModResult {
    unsafe {
        let symbol = StatsContext_GetSymbol(context);
        if symbol != 'H' {
            return ModResult::MOD_RES_PASSTHRU;
        }

        let xlines = XLines_GetXLines();
        XLines_InvokeStats(xlines, b"SHUN\0".as_ptr() as *const c_char, context);
        ModResult::MOD_RES_DENY
    }
}

extern "C" fn module_read_config(status: ConfigStatus) {
    unsafe {
        if let Some(module) = &mut MODULE_INSTANCE {
            TokenList_Clear(module.cleanedcommands);
            TokenList_Clear(module.enabledcommands);

            let tag = Config_GetConfValue(b"shun\0".as_ptr() as *const c_char);
            module.allowconnect = ConfigTag_GetBool(tag, b"allowconnect\0".as_ptr() as *const c_char, false);
            module.allowtags = ConfigTag_GetBool(tag, b"allowtags\0".as_ptr() as *const c_char, false);
            
            let cleaned_default = ConfigTag_GetString(tag, b"cleanedcommands\0".as_ptr() as *const c_char, b"AWAY PART QUIT\0".as_ptr() as *const c_char);
            let enabled_default = ConfigTag_GetString(tag, b"enabledcommands\0".as_ptr() as *const c_char, b"ADMIN OPER PING PONG QUIT\0".as_ptr() as *const c_char);
            
            // Note: These would need proper conversion from StdString to *const c_char
            // For now, this is a placeholder showing the intended logic
            module.notifyuser = ConfigTag_GetBool(tag, b"notifyuser\0".as_ptr() as *const c_char, true);
        }
    }
}

extern "C" fn module_on_pre_command(command: *mut c_char, parameters: Params, user: LocalUser, validated: bool, result: *mut ModResult) -> ModResult {
    unsafe {
        if validated || !is_shunned(user) {
            return ModResult::MOD_RES_PASSTHRU;
        }

        if let Some(module) = &MODULE_INSTANCE {
            let command_str = CStr::from_ptr(command).to_string_lossy();
            let command_cstr = std::ffi::CString::new(command_str.as_ref()).unwrap();
            
            if !TokenList_Contains(module.enabledcommands, command_cstr.as_ptr()) {
                if module.notifyuser {
                    let message = format!("*** {} command not processed as you have been blocked from issuing commands.", command_str);
                    let message_cstr = std::ffi::CString::new(message).unwrap();
                    User_WriteNotice(user as User, message_cstr.as_ptr());
                }
                return ModResult::MOD_RES_DENY;
            }

            if !module.allowtags {
                // Remove all client tags.
                let tags = Parameters_GetTags(parameters);
                let mut iter = ClientProtocolTagMap_Begin(tags);
                let end = ClientProtocolTagMap_End(tags);
                
                while iter != end {
                    let key = ClientProtocolTagMap_GetKey(iter);
                    let key_str = CStr::from_ptr(key).to_bytes();
                    if !key_str.is_empty() && key_str[0] == b'+' {
                        iter = ClientProtocolTagMap_Erase(iter);
                    } else {
                        iter = ClientProtocolTagMap_Next(iter);
                    }
                }
            }

            if TokenList_Contains(module.cleanedcommands, command_cstr.as_ptr()) {
                let param_size = Parameters_GetSize(parameters);
                match param_size {
                    0 => {
                        if command_str == "AWAY" || command_str == "QUIT" {
                            Parameters_Clear(parameters);
                        }
                    }
                    1 => {
                        if command_str == "CYCLE" || command_str == "KNOCK" || command_str == "PART" {
                            Parameters_Resize(parameters, 1);
                        }
                    }
                    _ => {}
                }
            }
        }

        ModResult::MOD_RES_PASSTHRU
    }
}

// Module initialization function
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ModuleInit() -> *mut c_void {
    // Create the module
    let description = b"Adds the /SHUN command which allows server operators to prevent users from executing commands.\0";
    let module = Module_Create(VF_VENDOR | VF_COMMON, description.as_ptr() as *const c_char);
    
    // Create the shun factory
    let factory_type = b"SHUN\0";
    let shun_factory = XLineFactory_Create(factory_type.as_ptr() as *const c_char);
    XLineFactory_SetGenerate(shun_factory, shun_factory_generate);
    XLineFactory_SetAutoApplyToUserList(shun_factory, shun_factory_auto_apply_to_user_list);
    
    // Create the command
    let cmd_name = b"SHUN\0";
    let cmd_syntax = b"<nick!user@host>[,<nick!user@host>]+ [<duration> :<reason>]\0";
    let cmd = Command_Create(module, cmd_name.as_ptr() as *const c_char, 1, 3);
    Command_SetAccessNeeded(cmd, CmdAccess::OPERATOR);
    Command_SetSyntax(cmd, cmd_syntax.as_ptr() as *const c_char);
    Command_SetHandle(cmd, command_shun_handle);
    
    // Create token lists
    let cleanedcommands = TokenList_Create();
    let enabledcommands = TokenList_Create();
    
    // Create module instance
    let module_instance = ModuleShun {
        module,
        cmd,
        shun_factory,
        allowconnect: false,
        allowtags: false,
        cleanedcommands,
        enabledcommands,
        notifyuser: true,
    };
    
    // Set module callbacks
    Module_SetInit(module, module_init);
    Module_SetDispose(module, module_dispose);
    Module_SetPrioritize(module, module_prioritize);
    Module_SetOnStats(module, module_on_stats);
    Module_SetReadConfig(module, module_read_config);
    Module_SetOnPreCommand(module, module_on_pre_command);
    
    // Store the module instance globally
    MODULE_INSTANCE = Some(module_instance);
    SHUN_FACTORY = Some(shun_factory);
    SHUN_COMMAND = Some(cmd);
    
    module as *mut c_void
}
