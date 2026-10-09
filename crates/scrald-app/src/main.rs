// Hide the console window in release builds on Windows. Debug builds keep it
// so `tracing` output is visible while developing.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod commands;
mod protocol;

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
        .manage(protocol::AssetRegistry::default())
        // Rust note: the closure gets a context (for app state) and the
        // request; `responder` lets us answer later, from another thread, so
        // file reads never block the webview's main thread.
        .register_asynchronous_uri_scheme_protocol(protocol::SCHEME, |ctx, request, responder| {
            let app = ctx.app_handle().clone();
            std::thread::spawn(move || {
                let registry = app.state::<protocol::AssetRegistry>();
                responder.respond(protocol::respond(&registry, &request));
            });
        })
        .on_window_event(|window, event| {
            if let tauri::WindowEvent::Destroyed = event {
                window
                    .state::<protocol::AssetRegistry>()
                    .remove_window(window.label());
            }
        })
        .invoke_handler(tauri::generate_handler![
            commands::launch_info,
            commands::open_document,
            commands::report_timing,
        ])
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

/// Logs to stderr, or to the file named by `SCRALD_LOG_FILE` if set. Release
/// builds have no console on Windows, so the file is how to see their logs
/// (for example when measuring performance).
fn init_logging() {
    let filter =
        EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new(DEFAULT_LOG_FILTER));
    let builder = tracing_subscriber::fmt().with_env_filter(filter);
    match std::env::var_os("SCRALD_LOG_FILE").map(std::fs::File::create) {
        // Rust note: `Mutex<File>` lets several threads share one file handle
        // safely; tracing locks it for each log line.
        Some(Ok(file)) => builder
            .with_ansi(false)
            .with_writer(std::sync::Mutex::new(file))
            .init(),
        _ => builder.init(),
    }
}
