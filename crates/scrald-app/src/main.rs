// Hide the console window in release builds on Windows. Debug builds keep it
// so `tracing` output is visible while developing.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod commands;
mod protocol;
mod session;
mod state;
mod themes;
mod vaults;
mod watcher;
mod window;

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
    let launch_path = launch.path.clone();

    tauri::Builder::default()
        .manage(launch)
        .manage(protocol::AssetRegistry::default())
        .manage(watcher::DocumentWatchers::default())
        .manage(session::Sessions::default())
        .manage(vaults::VaultCache::default())
        .plugin(tauri_plugin_opener::init())
        // Used only from Rust (the assets folder picker); the frontend has
        // no dialog permission.
        .plugin(tauri_plugin_dialog::init())
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
        .register_asynchronous_uri_scheme_protocol(themes::SCHEME, |ctx, request, responder| {
            let app = ctx.app_handle().clone();
            std::thread::spawn(move || {
                let response = match app.try_state::<themes::ThemeService>() {
                    Some(service) => service.respond(&request),
                    None => tauri::http::Response::builder()
                        .status(503)
                        .body(Vec::new())
                        .unwrap_or_default(),
                };
                responder.respond(response);
            });
        })
        .manage(window::WindowTracker::default())
        .on_window_event(|window, event| match event {
            tauri::WindowEvent::Moved(_) | tauri::WindowEvent::Resized(_) => {
                window.state::<window::WindowTracker>().observe(window);
            }
            tauri::WindowEvent::CloseRequested { .. } => remember_window(window),
            tauri::WindowEvent::Destroyed => {
                let label = window.label();
                window
                    .state::<protocol::AssetRegistry>()
                    .remove_window(label);
                window.state::<watcher::DocumentWatchers>().unwatch(label);
                window.state::<window::WindowTracker>().forget(label);
                window.state::<session::Sessions>().forget(label);
            }
            _ => {}
        })
        .invoke_handler(tauri::generate_handler![
            commands::launch_info,
            commands::open_document,
            commands::reparse_document,
            commands::edit_front_matter,
            commands::save_document,
            commands::block_source,
            commands::edit_block,
            commands::step_history,
            commands::check_disk,
            commands::pick_assets_folder,
            commands::save_reading_position,
            commands::set_remote_images,
            commands::recent_documents,
            commands::list_themes,
            commands::theme_style,
            commands::set_document_theme,
            commands::set_default_theme,
            commands::duplicate_theme,
            commands::user_theme_folder,
            commands::system_fonts,
            commands::set_document_typography,
            commands::set_document_flavor,
            commands::properties_open,
            commands::set_properties_open,
            commands::typography_defaults,
            commands::set_typography_defaults,
            commands::resolve_link,
            commands::open_external,
            commands::report_timing,
        ])
        .setup(move |app| {
            let store = open_state_store(app.handle());
            // The main window starts hidden (tauri.conf.json) so it can be
            // moved to its remembered place before anyone sees it.
            if let Some(webview_window) = app.get_webview_window("main") {
                // Geometry and window events work with the plain `Window`.
                let window = webview_window.as_ref().window();
                window.set_title(&window_title)?;
                match store.window_for(launch_path.as_deref()) {
                    Ok(Some(geometry)) => window::apply_geometry(&window, geometry)?,
                    Ok(None) => {}
                    Err(error) => {
                        tracing::warn!(error = format!("{error:#}"), "could not read window state")
                    }
                }
                window.show()?;
            }
            app.manage(store);

            let theme_service = themes::ThemeService::new(user_theme_dir(app.handle()));
            if let Err(error) = theme_service.watch(app.handle()) {
                tracing::warn!(
                    error = format!("{error:#}"),
                    "user theme hot reload unavailable"
                );
            }
            app.manage(theme_service);
            Ok(())
        })
        .run(tauri::generate_context!())
        // Rust note: `.context(...)` (from anyhow) wraps an error with a
        // message; together with `?` it propagates the error out of `main`,
        // like rethrowing with extra detail.
        .context("error while running the Tauri application")?;

    Ok(())
}

/// Opens the state database in the app data directory. If that fails, Scrald
/// still runs, with an in-memory store that forgets everything on exit.
///
/// `SCRALD_DATA_DIR` overrides the directory, so automated test runs keep
/// their state out of the user's real app data.
fn open_state_store(app: &tauri::AppHandle) -> state::StateStore {
    let dir = match std::env::var_os("SCRALD_DATA_DIR") {
        Some(dir) => Ok(PathBuf::from(dir)),
        None => app.path().app_data_dir().context("no app data directory"),
    };
    let opened = dir.and_then(|dir| {
        let db = dir.join("scrald.db");
        tracing::info!(path = %db.display(), "state database");
        state::StateStore::open(&db)
    });
    match opened {
        Ok(store) => store,
        Err(error) => {
            tracing::warn!(
                error = format!("{error:#}"),
                "state database unavailable; nothing will be remembered"
            );
            // An in-memory SQLite database can only fail to open if SQLite
            // itself is broken, in which case nothing else would work either.
            state::StateStore::in_memory().expect("in-memory SQLite database")
        }
    }
}

/// `themes/` in the app config folder (or under `SCRALD_DATA_DIR`, for tests).
fn user_theme_dir(app: &tauri::AppHandle) -> Option<PathBuf> {
    match std::env::var_os("SCRALD_DATA_DIR") {
        Some(dir) => Some(PathBuf::from(dir).join("themes")),
        None => app
            .path()
            .app_config_dir()
            .ok()
            .map(|dir| dir.join("themes")),
    }
}

/// Saves a closing window's geometry for its document and as the last-used size.
fn remember_window(window: &tauri::Window) {
    let (document, geometry) = window.state::<window::WindowTracker>().snapshot(window);
    let Some(geometry) = geometry else { return };
    if let Some(store) = window.try_state::<state::StateStore>()
        && let Err(error) = store.save_window(document.as_deref(), geometry)
    {
        tracing::warn!(error = format!("{error:#}"), "could not save window state");
    }
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
