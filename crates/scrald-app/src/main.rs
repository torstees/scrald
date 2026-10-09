// Hide the console window in release builds on Windows. Debug builds keep it
// so `tracing` output is visible while developing.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod commands;

use std::path::PathBuf;

use anyhow::Context;
use tauri::Manager;
use tracing_subscriber::EnvFilter;

use commands::LaunchInfo;

/// Default log filter when `RUST_LOG` isn't set: info everywhere, debug for our crates.
const DEFAULT_LOG_FILTER: &str = "info,scrald=debug,scrald_core=debug";

// Rust note: `anyhow::Result<()>` is `Result<(), anyhow::Error>`, a "success
// or any error" type. Returning `Err` from `main` prints it and exits non-zero.
fn main() -> anyhow::Result<()> {
    init_logging();

    let launch = LaunchInfo::from_arg(std::env::args_os().nth(1).map(PathBuf::from));
    tracing::info!(path = ?launch.path, exists = launch.exists, "starting Scrald");

    let window_title = format!("{} \u{2014} Scrald", launch.title);

    tauri::Builder::default()
        .manage(launch)
        .invoke_handler(tauri::generate_handler![commands::launch_info])
        .setup(move |app| {
            if let Some(window) = app.get_webview_window("main") {
                window.set_title(&window_title)?;
            }
            Ok(())
        })
        .run(tauri::generate_context!())
        // Rust note: `.context(...)` (from anyhow) wraps an error with a
        // message; together with `?` it propagates the error out of `main`,
        // like rethrowing with extra detail.
        .context("error while running the Tauri application")?;

    Ok(())
}

fn init_logging() {
    let filter =
        EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new(DEFAULT_LOG_FILTER));
    tracing_subscriber::fmt().with_env_filter(filter).init();
}
