#![allow(non_upper_case_globals)]
#![allow(non_camel_case_types)]
#![allow(non_snake_case)]
#![allow(dead_code)]

use std::ffi::{c_char, c_int, CStr, CString};
use std::path::Path;
use clap::{Parser, CommandFactory};

// External C++ functions
unsafe extern "C" {
    fn inspircd_ffi_exit(code: c_int);
    fn inspircd_ffi_println(msg: *const c_char);
    fn inspircd_ffi_eprintln(msg: *const c_char);
    fn inspircd_ffi_isatty(fd: c_int) -> c_int;
    fn inspircd_ffi_sleep(seconds: c_int);
}

/// Parsed command line options
#[derive(Parser, Debug)]
#[command(name = "inspircd")]
#[command(about = "InspIRCd - Internet Relay Chat Daemon", long_about = None)]
struct InspircdArgs {
    /// The location of the main config file
    #[arg(short = 'c', long = "config", value_name = "FILE")]
    config: Option<String>,

    /// Start in debug mode
    #[arg(short = 'd', long = "debug")]
    debug: bool,

    /// Disable forking into the background
    #[arg(short = 'F', long = "nofork")]
    nofork: bool,

    /// Show help and exit
    #[arg(short = 'h', long = "help")]
    help: bool,

    /// Disable writing logs to disk
    #[arg(short = 'L', long = "nolog")]
    nolog: bool,

    /// Start in protocol debug mode
    #[arg(short = 'p', long = "protocoldebug")]
    protocoldebug: bool,

    /// Disable writing the pid file
    #[arg(short = 'P', long = "nopid")]
    nopid: bool,

    /// Allow starting as root (not recommended)
    #[arg(short = 'r', long = "runasroot")]
    runasroot: bool,

    /// Show version and exit
    #[arg(short = 'v', long = "version")]
    version: bool,
}

/// Result of parsing command line options
#[repr(C)]
pub struct ParseOptionsResult {
    pub config_path: *mut c_char,
    pub debug: bool,
    pub nofork: bool,
    pub writelog: bool,
    pub writepid: bool,
    pub runasroot: bool,
    pub forceprotodebug: bool,
    pub should_exit: bool,
    pub exit_code: c_int,
}

/// Expands a path to its absolute form
#[unsafe(no_mangle)]
pub extern "C" fn inspircd_expand_path(path: *const c_char) -> *mut c_char {
    if path.is_null() {
        return std::ptr::null_mut();
    }

    let path_str = unsafe { CStr::from_ptr(path) }.to_string_lossy();
    let path = Path::new(&path_str as &str);

    let expanded = if path.is_absolute() {
        path.to_path_buf()
    } else {
        std::fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf())
    };

    CString::new(expanded.to_string_lossy().as_ref())
        .map(|s| s.into_raw())
        .unwrap_or(std::ptr::null_mut())
}

/// Checks if running as root and warns/exits accordingly
#[unsafe(no_mangle)]
pub extern "C" fn inspircd_check_root() {
    #[cfg(unix)]
    {
        use nix::unistd::{Uid, Gid};

        let euid = Uid::effective();
        let egid = Gid::effective();

        // Check if running as root (uid or gid is 0)
        if euid.as_raw() == 0 || egid.as_raw() == 0 {
            let warning = CString::new(
                "Warning! You have started as root. Running as root is generally not required\n\
                 and may allow an attacker to gain access to your system if they find a way to\n\
                 exploit your IRC server.\n"
            ).unwrap();

            unsafe { inspircd_ffi_println(warning.as_ptr()); }

            let is_tty = unsafe { inspircd_ffi_isatty(1) } != 0;

            if is_tty {
                let msg = CString::new(
                    "InspIRCd will start in 30 seconds. If you are sure that you need to run as root\n\
                     then you can pass the --runasroot option to disable this wait."
                ).unwrap();
                unsafe { inspircd_ffi_println(msg.as_ptr()); }
                unsafe { inspircd_ffi_sleep(30); }
            } else {
                let msg = CString::new(
                    "If you are sure that you need to run as root then you can pass the --runasroot\n\
                     option to disable this error."
                ).unwrap();
                unsafe { inspircd_ffi_println(msg.as_ptr()); }
                unsafe { inspircd_ffi_exit(1); }
            }
        }
    }
}

/// Signal handler that exits with success
#[unsafe(no_mangle)]
pub extern "C" fn inspircd_void_signal_handler() {
    unsafe { inspircd_ffi_exit(0); }
}

/// Parses command line options
#[unsafe(no_mangle)]
pub extern "C" fn inspircd_parse_options(
    argc: c_int,
    argv: *const *const c_char,
    default_config: *const c_char,
) -> ParseOptionsResult {
    let mut result = ParseOptionsResult {
        config_path: std::ptr::null_mut(),
        debug: false,
        nofork: false,
        writelog: true,
        writepid: true,
        runasroot: false,
        forceprotodebug: false,
        should_exit: false,
        exit_code: 0,
    };

    if argv.is_null() || argc < 1 {
        result.should_exit = true;
        result.exit_code = 1;
        return result;
    }

    // Convert argv to Vec<String>
    let args: Vec<String> = unsafe {
        (0..argc)
            .map(|i| {
                let ptr = *argv.add(i as usize);
                if ptr.is_null() {
                    String::new()
                } else {
                    CStr::from_ptr(ptr).to_string_lossy().into_owned()
                }
            })
            .collect()
    };

    // Parse with clap
    match InspircdArgs::try_parse_from(&args) {
        Ok(parsed) => {
            if parsed.help {
                let help = CString::new(format!("{}", InspircdArgs::command())).unwrap();
                unsafe { inspircd_ffi_println(help.as_ptr()); }
                result.should_exit = true;
                result.exit_code = 0;
                return result;
            }

            if parsed.version {
                let version = env!("CARGO_PKG_VERSION");
                let version_str = CString::new(format!("InspIRCd {}", version)).unwrap();
                unsafe { inspircd_ffi_println(version_str.as_ptr()); }
                result.should_exit = true;
                result.exit_code = 0;
                return result;
            }

            // Set config path
            let config = parsed.config.unwrap_or_else(|| {
                unsafe {
                    if default_config.is_null() {
                        "inspircd.conf".to_string()
                    } else {
                        CStr::from_ptr(default_config).to_string_lossy().into_owned()
                    }
                }
            });

            result.config_path = inspircd_expand_path(
                CString::new(config.as_str()).unwrap().as_ptr()
            );

            result.debug = parsed.debug || parsed.protocoldebug;
            result.nofork = result.debug || parsed.nofork;
            result.writelog = !parsed.nolog;
            result.writepid = !parsed.nopid;
            result.runasroot = parsed.runasroot;
            result.forceprotodebug = parsed.protocoldebug;
        }
        Err(e) => {
            let error_msg = CString::new(format!("Error: {}", e)).unwrap();
            unsafe { inspircd_ffi_eprintln(error_msg.as_ptr()); }
            result.should_exit = true;
            result.exit_code = 1;
        }
    }

    result
}

/// Frees a string allocated by Rust
#[unsafe(no_mangle)]
pub extern "C" fn inspircd_free_string(ptr: *mut c_char) {
    if !ptr.is_null() {
        unsafe {
            let _ = CString::from_raw(ptr);
        }
    }
}
