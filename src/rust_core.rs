/*
 * InspIRCd -- Internet Relay Chat Daemon
 *
 *   Crate root for the core staticlib. Meson passes only a single .rs file to
 *   rustc; additional modules are wired via `mod` here.
 */

mod stringutils;
mod bancache;
mod usermanager;
mod cidr;
mod channelmanager;
mod snomasks;
mod wildcard;
mod logging;
mod modulemanager;
mod users;
mod server;