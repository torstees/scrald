//! Linux only: WebKitGTK's DMABUF renderer (DESIGN.md §11.1, #208). In
//! virtual machines and with some GPU drivers it leaves the page blank: the
//! window opens, but nothing is ever drawn. Turning it off with
//! `WEBKIT_DISABLE_DMABUF_RENDERER=1` fixes that everywhere, for a little
//! rendering speed where it worked.
//!
//! The variable must be set before WebKitGTK starts. Setting an environment
//! variable from inside a running Rust program is `unsafe` (other threads
//! could be reading the environment), so instead, when the variable isn't
//! set, Scrald starts itself again with it set, replacing this process. A
//! value the user sets, `0` included, is left alone.

use std::ffi::OsString;
use std::os::unix::process::CommandExt;
use std::process::Command;

/// The variable WebKitGTK reads.
const VAR: &str = "WEBKIT_DISABLE_DMABUF_RENDERER";

/// If the variable isn't set, replaces this process with Scrald run again
/// with it set, so this returns only if it's already set or Scrald couldn't
/// be started again. In the second case it returns the reason, for the log
/// (which isn't set up yet when this runs).
pub fn ensure_dmabuf_off() -> Option<String> {
    if std::env::var_os(VAR).is_some() {
        return None;
    }
    let exe = match std::env::current_exe() {
        Ok(exe) => exe,
        Err(error) => return Some(format!("can't find Scrald's own path: {error}")),
    };
    // Rust note: `args_os` yields each argument as an `OsString`, which can
    // hold file names that aren't valid UTF-8, unlike `String`.
    let mut args = std::env::args_os();
    let program_name = args.next().unwrap_or_else(|| OsString::from("scrald"));
    // `exec` (Unix only, through `CommandExt`) swaps in the new program and
    // only returns if that failed.
    let error = Command::new(exe)
        .arg0(program_name)
        .args(args)
        .env(VAR, "1")
        .exec();
    Some(format!("couldn't restart with {VAR}=1: {error}"))
}
