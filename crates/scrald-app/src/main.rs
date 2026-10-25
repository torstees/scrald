// Hide the console window in release builds on Windows. Debug builds keep it
// so `tracing` output is visible while developing.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod commands;
mod fonts;
mod newdoc;
mod protocol;
mod session;
mod state;
mod themes;
mod vaults;
mod watcher;
mod window;

use std::path::{Path, PathBuf};

use anyhow::Context;
use tauri::Manager;
use tracing_subscriber::EnvFilter;

use commands::{LaunchInfo, Launches};

/// Default log filter when `RUST_LOG` isn't set: info everywhere, debug for our crates.
const DEFAULT_LOG_FILTER: &str = "info,scrald=debug,scrald_core=debug";

// Rust note: `anyhow::Result<()>` is `Result<(), anyhow::Error>`, a "success
// or any error" type. Returning `Err` from `main` prints it and exits non-zero.
fn main() -> anyhow::Result<()> {
    init_logging();

    let launch = LaunchInfo::from_arg(std::env::args_os().nth(1).map(PathBuf::from));
    tracing::info!(path = ?launch.path, exists = launch.exists, "starting Scrald");

    let first_launch = launch.clone();
    let launches = Launches::default();
    launches.set("main", launch);

    tauri::Builder::default()
        // Registered first, so a second launch hands its file to this process
        // and exits before anything else starts (DESIGN.md §11).
        .plugin(tauri_plugin_single_instance::init(|app, args, cwd| {
            // `args[0]` is the program; a file comes after it.
            let launch = LaunchInfo::from_arg_in(args.get(1).map(PathBuf::from), Path::new(&cwd));
            tracing::info!(path = ?launch.path, "launch handed over by a new instance");
            let app = app.clone();
            // Rust note: `move` hands `app` and `launch` to the thread, which
            // owns them from then on (a C++ lambda capturing by value).
            // Windows are built off this callback's thread, the event loop's.
            std::thread::spawn(move || {
                if let Err(error) = window::open_window(&app, launch) {
                    tracing::warn!(error = format!("{error:#}"), "could not open a window");
                }
            });
        }))
        .manage(launches)
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
                // Downloaded Google Fonts live under their own first segment.
                let path = scrald_core::assets::percent_decode(request.uri().path());
                let font_prefix = format!("/{}/", fonts::URL_PREFIX);
                if let Some(relative) = path.strip_prefix(&font_prefix)
                    && let Some(cache) = app.try_state::<fonts::FontCache>()
                {
                    responder.respond(cache.respond(relative));
                    return;
                }
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
                window.state::<Launches>().forget(label);
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
            commands::search_document,
            commands::edit_front_matter,
            commands::save_document,
            commands::block_source,
            commands::document_text,
            commands::set_document_text,
            commands::edit_block,
            commands::step_history,
            commands::check_disk,
            commands::disk_text,
            commands::file_text,
            commands::keep_local_version,
            commands::load_disk_version,
            commands::recovered_changes,
            commands::restore_recovered,
            commands::discard_recovered,
            commands::recovered_text,
            commands::pick_assets_folder,
            commands::save_reading_position,
            commands::set_remote_images,
            commands::recent_documents,
            commands::set_document_pinned,
            commands::forget_document,
            commands::pick_document,
            commands::new_document_options,
            commands::pick_folder,
            commands::create_document,
            commands::remember_new_document_folder,
            commands::forget_new_document_folder,
            commands::remember_new_folders,
            commands::set_remember_new_folders,
            commands::open_templates_folder,
            commands::start_theme,
            commands::list_themes,
            commands::theme_style,
            commands::theme_details,
            commands::theme_contrast,
            commands::preview_theme_edit,
            commands::save_theme_edit,
            commands::set_document_theme,
            commands::set_default_theme,
            commands::default_theme,
            commands::contents_depth,
            commands::set_contents_depth,
            commands::duplicate_theme,
            commands::pick_terminal_theme,
            commands::import_terminal_theme,
            commands::user_theme_folder,
            commands::system_fonts,
            commands::set_document_typography,
            commands::set_document_flavor,
            commands::properties_open,
            commands::set_properties_open,
            commands::front_settings,
            commands::set_front_settings,
            commands::properties_placement,
            commands::set_properties_placement,
            commands::autosave,
            commands::set_autosave,
            commands::typography_defaults,
            commands::set_typography_defaults,
            commands::resolve_link,
            commands::open_external,
            commands::print_window,
            commands::report_timing,
        ])
        .setup(move |app| {
            app.manage(open_state_store(app.handle()));
            // Windows start hidden (tauri.conf.json) so they can be moved to
            // their remembered place before anyone sees them.
            if let Some(webview_window) = app.get_webview_window("main") {
                // Geometry and window events work with the plain `Window`.
                let window = webview_window.as_ref().window();
                window::show_placed(&window, &first_launch, &[])?;
            }

            let theme_service = themes::ThemeService::new(user_theme_dir(app.handle()));
            if let Err(error) = theme_service.watch(app.handle()) {
                tracing::warn!(
                    error = format!("{error:#}"),
                    "user theme hot reload unavailable"
                );
            }
            app.manage(theme_service);
            app.manage(fonts::FontCache::new(font_cache_dir(app.handle())));

            let templates = newdoc::Templates::new(templates_dir(app.handle()));
            if let Err(error) = templates.ensure_sample(&app.state::<state::StateStore>()) {
                tracing::warn!(error = format!("{error:#}"), "templates folder unavailable");
            }
            app.manage(templates);

            start_crash_recovery(app.handle());
            Ok(())
        })
        .run(app_context())
        // Rust note: `.context(...)` (from anyhow) wraps an error with a
        // message; together with `?` it propagates the error out of `main`,
        // like rethrowing with extra detail.
        .context("error while running the Tauri application")?;

    Ok(())
}

/// The app's configuration. With `SCRALD_DATA_DIR` set (automated and manual
/// test runs), the identifier gets a suffix for that folder, because the
/// single-instance plugin finds the running Scrald by its identifier: a test
/// run then never hands its file to the user's own Scrald, which would open
/// it with their real state database, and test runs on one folder still find
/// each other.
fn app_context() -> tauri::Context {
    let mut context = tauri::generate_context!();
    if let Some(dir) = std::env::var_os("SCRALD_DATA_DIR") {
        let suffix = scrald_core::hash::fnv1a(dir.as_encoded_bytes());
        let config = context.config_mut();
        config.identifier = format!("{}.test-{suffix:016x}", config.identifier);
        tracing::info!(identifier = config.identifier, "test identifier");
    }
    context
}

/// Opens the state database in the app data directory. If that fails, Scrald
/// still runs, with an in-memory store that forgets everything on exit.
fn open_state_store(app: &tauri::AppHandle) -> state::StateStore {
    let opened = data_dir(app).and_then(|dir| {
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

/// The app data folder, or `SCRALD_DATA_DIR` if set, so automated test runs
/// keep their state out of the user's real app data.
fn data_dir(app: &tauri::AppHandle) -> anyhow::Result<PathBuf> {
    match std::env::var_os("SCRALD_DATA_DIR") {
        Some(dir) => Ok(PathBuf::from(dir)),
        None => app.path().app_data_dir().context("no app data directory"),
    }
}

/// Copies unsaved changes to recovery files in `recovery/` in the app data
/// folder every few seconds (DESIGN.md §9.3), on a thread of its own.
fn start_crash_recovery(app: &tauri::AppHandle) {
    let dir = match data_dir(app) {
        Ok(dir) => dir.join("recovery"),
        Err(error) => {
            tracing::warn!(error = format!("{error:#}"), "crash recovery unavailable");
            return;
        }
    };
    tracing::info!(path = %dir.display(), "recovery files");
    app.state::<session::Sessions>().set_recovery_dir(dir);
    let app = app.clone();
    std::thread::spawn(move || {
        loop {
            std::thread::sleep(session::RECOVERY_INTERVAL);
            app.state::<session::Sessions>().write_recovery();
        }
    });
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

/// `templates/` in the app config folder (or under `SCRALD_DATA_DIR`, for
/// tests): the user's templates for new documents (DESIGN.md §11).
fn templates_dir(app: &tauri::AppHandle) -> Option<PathBuf> {
    match std::env::var_os("SCRALD_DATA_DIR") {
        Some(dir) => Some(PathBuf::from(dir).join("templates")),
        None => app
            .path()
            .app_config_dir()
            .ok()
            .map(|dir| dir.join("templates")),
    }
}

/// Where downloaded Google Fonts go: `fonts/` in the app cache folder (or
/// under `SCRALD_DATA_DIR`, for tests).
fn font_cache_dir(app: &tauri::AppHandle) -> Option<PathBuf> {
    match std::env::var_os("SCRALD_DATA_DIR") {
        Some(dir) => Some(PathBuf::from(dir).join("cache").join("fonts")),
        None => app.path().app_cache_dir().ok().map(|dir| dir.join("fonts")),
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
