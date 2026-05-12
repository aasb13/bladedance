#![allow(unsafe_op_in_unsafe_fn)]
#![allow(non_upper_case_globals)]
#![allow(non_camel_case_types)]
#![allow(non_snake_case)]
#![allow(dead_code)]

#[path = "../stringutils.rs"]
mod stringutils;

#[path = "../bancache.rs"]
mod bancache;

#[path = "../usermanager.rs"]
mod usermanager;

#[path = "../cidr.rs"]
mod cidr;

#[path = "../channelmanager.rs"]
mod channelmanager;

#[path = "../snomasks.rs"]
mod snomasks;

#[path = "../wildcard.rs"]
mod wildcard;

#[path = "../modulemanager.rs"]
mod modulemanager;

#[path = "../users.rs"]
mod users;

#[path = "../server.rs"]
mod server;

#[path = "../logging.rs"]
mod logging_ffi;

mod logging;

#[path = "../coremods/core_info/cmd_admin.rs"]
mod cmd_admin;

