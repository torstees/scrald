//! Windows: opening one per document, and remembering their geometry and
//! restoring it onto a monitor that is actually connected (DESIGN.md §11).

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::sync::atomic::{AtomicU32, Ordering};

use anyhow::Context;
use tauri::{
    AppHandle, LogicalPosition, LogicalSize, Manager, PhysicalPosition, PhysicalSize, Position,
    Size, WebviewWindowBuilder, Window,
};

use crate::commands::{LaunchInfo, Launches};
use crate::state::{StateStore, WindowGeometry, document_key};

/// Numbers the windows opened after the first ("main"): `window-1`, ...
// Rust note: an atomic integer can be changed through a shared reference from
// any thread without a lock, like `std::atomic<uint32_t>` in C++.
static NEXT_WINDOW: AtomicU32 = AtomicU32::new(1);

/// Opens `launch` (a file, or the start screen) in a new window, or, if that
/// file is already open, brings its window to the front instead, so the same
/// document isn't edited in two windows.
pub fn open_window(app: &AppHandle, launch: LaunchInfo) -> anyhow::Result<()> {
    if let Some(path) = launch.path.as_deref().filter(|_| launch.exists)
        && let Some(label) = app.state::<WindowTracker>().window_showing(path)
        && let Some(existing) = app.get_webview_window(&label)
    {
        tracing::info!(window = label, "document already open; focusing its window");
        existing.unminimize()?;
        existing.set_focus()?;
        return Ok(());
    }

    // Same settings as the first window in tauri.conf.json, under a new label.
    let mut config = app
        .config()
        .app
        .windows
        .first()
        .cloned()
        .context("no window in tauri.conf.json")?;
    config.label = format!("window-{}", NEXT_WINDOW.fetch_add(1, Ordering::Relaxed));
    // The page asks for its launch info as soon as it loads.
    app.state::<Launches>().set(&config.label, launch.clone());
    let webview_window = WebviewWindowBuilder::from_config(app, &config)?.build()?;
    let window = webview_window.as_ref().window();
    // Keep it off the windows that are already open.
    let open: Vec<(i32, i32)> = app
        .webview_windows()
        .values()
        .filter(|w| w.label() != config.label)
        .filter_map(|w| {
            let position = w.outer_position().ok()?;
            let scale = w.scale_factor().ok()?;
            Some(screen_point(position, scale))
        })
        .collect();
    tracing::info!(window = config.label, path = ?launch.path, "opening a new window");
    show_placed(&window, &launch, &open)
}

/// Opens files the system hands to the running app: on macOS, a file opened
/// from Finder (or with `open`) arrives as an "opened" event rather than on
/// the command line, both when it starts Scrald and while Scrald is running
/// (DESIGN.md §11.1). Runs on the event loop's thread.
///
/// At startup the first window is still on its way to showing the start
/// screen, so the first file goes there instead of into a second window.
/// Every other file opens like a file from a later launch: in a window of
/// its own, or by focusing the window that already shows it.
// Rust note: `#[cfg(...)]` compiles an item only on matching targets, like
// `#ifdef __APPLE__`; the other platforms never see it, so it can't be an
// unused-code warning there.
#[cfg(target_os = "macos")]
pub fn open_files(app: &AppHandle, paths: Vec<PathBuf>) {
    for path in paths {
        let launch = LaunchInfo::from_arg(Some(path));
        if app
            .state::<Launches>()
            .replace_unread("main", launch.clone())
        {
            tracing::info!(path = ?launch.path, "opened file goes to the first window");
            // Before setup, the window isn't shown yet, and setup places it
            // with this launch; after, it's titled and placed for the file now.
            if let Some(main) = app.get_webview_window("main")
                && main.is_visible().unwrap_or(false)
            {
                let window = main.as_ref().window();
                if let Err(error) = show_placed(&window, &launch, &[]) {
                    tracing::warn!(error = format!("{error:#}"), "could not place the window");
                }
            }
            continue;
        }
        tracing::info!(path = ?launch.path, "file opened by the system");
        let app = app.clone();
        // New windows are built off the event loop's thread, as in main.rs.
        std::thread::spawn(move || {
            if let Err(error) = open_window(&app, launch) {
                tracing::warn!(error = format!("{error:#}"), "could not open a window");
            }
        });
    }
}

/// The local files among the URLs the system asked the app to open; other
/// URLs (Scrald registers no URL schemes) are skipped.
// Rust note: `filter_map` keeps the `Some` results and drops the `None`s;
// `.ok()` turns the `Result` from `to_file_path` into an `Option`.
#[cfg(any(target_os = "macos", test))]
pub fn file_paths(urls: &[tauri::Url]) -> Vec<PathBuf> {
    urls.iter()
        .filter(|url| url.scheme() == "file")
        .filter_map(|url| url.to_file_path().ok())
        .collect()
}

/// Titles a new (hidden) window, moves it to where its document's window was
/// last (or the last window's size and place), and shows it. `open` lists
/// the positions of the other windows, which it steps away from.
pub fn show_placed(
    window: &Window,
    launch: &LaunchInfo,
    open: &[(i32, i32)],
) -> anyhow::Result<()> {
    window.set_title(&launch.window_title())?;
    let path = launch.path.as_deref().filter(|_| launch.exists);
    // The store is managed after the first window is placed, so it may be missing here.
    let remembered = match window.try_state::<StateStore>() {
        Some(store) => store.window_for(path),
        None => Ok(None),
    };
    match remembered {
        Ok(Some(mut geometry)) => {
            let rect = cascade(rect_of(&geometry), open);
            geometry.x = rect.x;
            geometry.y = rect.y;
            apply_geometry(window, geometry)?;
        }
        Ok(None) => {}
        Err(error) => tracing::warn!(error = format!("{error:#}"), "could not read window state"),
    }
    window.show()?;
    window.set_focus()?;
    Ok(())
}

/// How far a new window is moved, right and down, from an open window that
/// is at the same place.
const CASCADE_STEP: i32 = 32;

/// Moves `wanted` a step down and right while its top-left corner is at an
/// open window's (within a step), so a new window never hides an open one
/// exactly. Gives up after a few steps; the monitor clamp still applies.
pub fn cascade(wanted: Rect, open: &[(i32, i32)]) -> Rect {
    let mut rect = wanted;
    for _ in 0..10 {
        let covers = open
            .iter()
            .any(|&(x, y)| (rect.x - x).abs() < CASCADE_STEP && (rect.y - y).abs() < CASCADE_STEP);
        if !covers {
            break;
        }
        rect.x += CASCADE_STEP;
        rect.y += CASCADE_STEP;
    }
    rect
}

/// Whether window and monitor rectangles are kept in points (logical pixels)
/// rather than physical pixels. On macOS, a window's physical size depends
/// on the monitor it's on (a 2x display doubles it), so the same window
/// measured on two monitors gives two sizes, and a size saved on one comes
/// back twice as big or half as big on the other. Points are the same on
/// every monitor there, and the desktop's coordinates are in points. On
/// Windows, physical pixels are the consistent unit (DESIGN.md §11).
const USE_POINTS: bool = cfg!(target_os = "macos");

/// A position in screen units (see `USE_POINTS`), from Tauri's physical one
/// and the scale factor of the monitor it was measured on.
fn screen_point(position: PhysicalPosition<i32>, scale: f64) -> (i32, i32) {
    if USE_POINTS {
        // Rust note: `to_logical::<f64>` picks the number type of the result
        // with a "turbofish" (`::<T>`), as in `static_cast<double>` in C++.
        let logical = position.to_logical::<f64>(scale);
        (logical.x.round() as i32, logical.y.round() as i32)
    } else {
        (position.x, position.y)
    }
}

/// A size in screen units, as `screen_point` for positions.
fn screen_size(size: PhysicalSize<u32>, scale: f64) -> (u32, u32) {
    if USE_POINTS {
        let logical = size.to_logical::<f64>(scale);
        (logical.width.round() as u32, logical.height.round() as u32)
    } else {
        (size.width, size.height)
    }
}

/// A rectangle in screen units: points on macOS, physical pixels elsewhere
/// (see `USE_POINTS`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Rect {
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
}

impl Rect {
    fn right(&self) -> i64 {
        i64::from(self.x) + i64::from(self.width)
    }

    fn bottom(&self) -> i64 {
        i64::from(self.y) + i64::from(self.height)
    }
}

/// How much of a window must be on a monitor, in pixels, for it to count as
/// reachable: enough of the title bar to grab and drag.
const MIN_VISIBLE_WIDTH: i64 = 120;
const MIN_TITLE_BAR: i64 = 40;

/// Returns `window` unchanged if its title bar is reachable on one of
/// `monitors` and it fits that monitor. One that's reachable but bigger is
/// shrunk to the monitor and moved onto it. One that isn't reachable is
/// moved (shrinking if needed) to the center of the first monitor, which
/// callers pass as the primary one. With no monitors known, leaves it alone.
pub fn clamp_to_monitors(window: Rect, monitors: &[Rect]) -> Rect {
    let overlap_x = |m: &Rect| window.right().min(m.right()) - i64::from(window.x.max(m.x));
    // Of the monitors its title bar is reachable on, the one it overlaps most,
    // as macOS chooses a window's screen.
    // Rust note: `filter` then `max_by_key` walks the monitors lazily and
    // keeps the largest key, as an `Option` (`None` if nothing passed).
    let reachable_on = monitors
        .iter()
        .filter(|m| {
            let title_top = i64::from(window.y);
            overlap_x(m) >= MIN_VISIBLE_WIDTH
                && title_top >= i64::from(m.y)
                && title_top + MIN_TITLE_BAR <= m.bottom()
        })
        .max_by_key(|m| overlap_x(m));
    if let Some(m) = reachable_on {
        if window.width <= m.width && window.height <= m.height {
            return window;
        }
        let width = window.width.min(m.width);
        let height = window.height.min(m.height);
        // Keep its place where it still fits; otherwise slide it back onto the monitor.
        let max_x = i64::from(m.x) + i64::from(m.width - width);
        let max_y = i64::from(m.y) + i64::from(m.height - height);
        return Rect {
            x: i64::from(window.x).clamp(i64::from(m.x), max_x) as i32,
            y: i64::from(window.y).clamp(i64::from(m.y), max_y) as i32,
            width,
            height,
        };
    }
    let Some(primary) = monitors.first() else {
        return window;
    };
    let width = window.width.min(primary.width);
    let height = window.height.min(primary.height);
    Rect {
        x: primary.x + ((primary.width - width) / 2) as i32,
        y: primary.y + ((primary.height - height) / 2) as i32,
        width,
        height,
    }
}

/// Per-window facts the app needs when the window closes: which document it
/// shows, and its last normal (not maximized or minimized) bounds, since a
/// maximized window can't report the size it would restore to.
#[derive(Default)]
pub struct WindowTracker {
    inner: Mutex<HashMap<String, Tracked>>,
}

#[derive(Default, Clone)]
struct Tracked {
    document: Option<PathBuf>,
    normal: Option<Rect>,
}

impl WindowTracker {
    pub fn set_document(&self, window: &str, path: &Path) {
        self.lock().entry(window.to_string()).or_default().document = Some(path.to_path_buf());
    }

    /// Records the window's current bounds if it's in its normal state.
    pub fn observe(&self, window: &Window) {
        let maximized = window.is_maximized().unwrap_or(false);
        let minimized = window.is_minimized().unwrap_or(false);
        if maximized || minimized {
            return;
        }
        if let Some(rect) = current_rect(window) {
            self.lock()
                .entry(window.label().to_string())
                .or_default()
                .normal = Some(rect);
        }
    }

    /// The document shown in `window` and the geometry to remember for it.
    pub fn snapshot(&self, window: &Window) -> (Option<PathBuf>, Option<WindowGeometry>) {
        let tracked = self.lock().get(window.label()).cloned().unwrap_or_default();
        let maximized = window.is_maximized().unwrap_or(false);
        let rect = tracked.normal.or_else(|| current_rect(window));
        let geometry = rect.map(|r| WindowGeometry {
            x: r.x,
            y: r.y,
            width: r.width,
            height: r.height,
            maximized,
        });
        (tracked.document, geometry)
    }

    pub fn forget(&self, window: &str) {
        self.lock().remove(window);
    }

    /// The window showing the document at `path`, if any. Paths are compared
    /// in their canonical form, so `doc.md` and `.\doc.md` are the same file.
    pub fn window_showing(&self, path: &Path) -> Option<String> {
        let key = document_key(path);
        // Rust note: `find_map` stops at the first item the closure turns
        // into `Some`, like a loop that returns from the first match;
        // `cond.then(|| x)` is `Some(x)` if `cond` is true, else `None`.
        self.lock().iter().find_map(|(label, tracked)| {
            let shown = tracked.document.as_deref()?;
            (document_key(shown) == key).then(|| label.clone())
        })
    }

    fn lock(&self) -> std::sync::MutexGuard<'_, HashMap<String, Tracked>> {
        self.inner
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
    }
}

/// The window's outer position and inner size: the pair that `set_position`
/// and `set_size` take back.
fn current_rect(window: &Window) -> Option<Rect> {
    let scale = window.scale_factor().ok()?;
    let (x, y) = screen_point(window.outer_position().ok()?, scale);
    let (width, height) = screen_size(window.inner_size().ok()?, scale);
    Some(Rect {
        x,
        y,
        width,
        height,
    })
}

/// Moves and sizes `window` to `geometry`, kept on a connected monitor.
pub fn apply_geometry(window: &Window, geometry: WindowGeometry) -> tauri::Result<()> {
    let mut monitors: Vec<Rect> = Vec::new();
    // The primary monitor first: it's where an off-screen window is moved to.
    if let Ok(Some(primary)) = window.primary_monitor() {
        monitors.push(monitor_rect(&primary));
    }
    for monitor in window.available_monitors()? {
        let rect = monitor_rect(&monitor);
        if !monitors.contains(&rect) {
            monitors.push(rect);
        }
    }
    let rect = clamp_to_monitors(rect_of(&geometry), &monitors);
    // Rust note: `Size` and `Position` are enums holding either kind of unit,
    // so one call takes points or physical pixels.
    let (size, position) = if USE_POINTS {
        (
            Size::Logical(LogicalSize::new(rect.width.into(), rect.height.into())),
            Position::Logical(LogicalPosition::new(rect.x.into(), rect.y.into())),
        )
    } else {
        (
            Size::Physical(PhysicalSize::new(rect.width, rect.height)),
            Position::Physical(PhysicalPosition::new(rect.x, rect.y)),
        )
    };
    window.set_size(size)?;
    window.set_position(position)?;
    if geometry.maximized {
        window.maximize()?;
    }
    Ok(())
}

fn rect_of(geometry: &WindowGeometry) -> Rect {
    Rect {
        x: geometry.x,
        y: geometry.y,
        width: geometry.width,
        height: geometry.height,
    }
}

fn monitor_rect(monitor: &tauri::Monitor) -> Rect {
    // The work area excludes the taskbar, so restored windows don't hide under it.
    // Each monitor's physical numbers are in its own scale, so on macOS
    // they're turned into points with that monitor's factor.
    let area = monitor.work_area();
    let scale = monitor.scale_factor();
    let (x, y) = screen_point(area.position, scale);
    let (width, height) = screen_size(area.size, scale);
    Rect {
        x,
        y,
        width,
        height,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const PRIMARY: Rect = Rect {
        x: 0,
        y: 0,
        width: 1920,
        height: 1040,
    };
    const LEFT: Rect = Rect {
        x: -2560,
        y: 0,
        width: 2560,
        height: 1400,
    };

    fn window(x: i32, y: i32) -> Rect {
        Rect {
            x,
            y,
            width: 1000,
            height: 800,
        }
    }

    #[test]
    fn visible_window_is_unchanged() {
        assert_eq!(
            clamp_to_monitors(window(100, 100), &[PRIMARY]),
            window(100, 100)
        );
        // On a secondary monitor that is still connected.
        assert_eq!(
            clamp_to_monitors(window(-2000, 50), &[PRIMARY, LEFT]),
            window(-2000, 50)
        );
    }

    #[test]
    fn window_on_disconnected_monitor_moves_to_primary_center() {
        let moved = clamp_to_monitors(window(-2000, 50), &[PRIMARY]);
        assert_eq!(
            moved,
            Rect {
                x: 460,
                y: 120,
                width: 1000,
                height: 800
            }
        );
    }

    #[test]
    fn mostly_offscreen_or_unreachable_title_bar_is_moved() {
        // Only 50 px peeking in from the right edge.
        assert_ne!(
            clamp_to_monitors(window(1870, 100), &[PRIMARY]),
            window(1870, 100)
        );
        // Title bar above the top of the screen.
        assert_ne!(
            clamp_to_monitors(window(100, -300), &[PRIMARY]),
            window(100, -300)
        );
        // Title bar below the bottom.
        assert_ne!(
            clamp_to_monitors(window(100, 1020), &[PRIMARY]),
            window(100, 1020)
        );
    }

    #[test]
    fn oversized_window_shrinks_to_fit() {
        let huge = Rect {
            x: 5000,
            y: 5000,
            width: 4000,
            height: 3000,
        };
        assert_eq!(clamp_to_monitors(huge, &[PRIMARY]), PRIMARY);
    }

    #[test]
    fn reachable_window_bigger_than_its_monitor_shrinks_onto_it() {
        // Its title bar is on the left monitor, but it's wider than that monitor.
        let wide = Rect {
            x: -2400,
            y: 100,
            width: 5900,
            height: 1900,
        };
        assert_eq!(clamp_to_monitors(wide, &[PRIMARY, LEFT]), LEFT);
        // Only a little too tall: it keeps its x, shrinks, and moves up to fit.
        let tall = Rect {
            x: 200,
            y: 300,
            width: 1000,
            height: 1100,
        };
        assert_eq!(
            clamp_to_monitors(tall, &[PRIMARY]),
            Rect {
                x: 200,
                y: 0,
                width: 1000,
                height: 1040
            }
        );
    }

    #[test]
    fn points_on_macos_physical_pixels_elsewhere() {
        let position = PhysicalPosition::new(3000, 400);
        let size = PhysicalSize::new(2000, 1600);
        if USE_POINTS {
            assert_eq!(screen_point(position, 2.0), (1500, 200));
            assert_eq!(screen_size(size, 2.0), (1000, 800));
        } else {
            assert_eq!(screen_point(position, 2.0), (3000, 400));
            assert_eq!(screen_size(size, 2.0), (2000, 1600));
        }
    }

    #[test]
    fn new_window_steps_off_open_windows() {
        // Nothing at its place: unchanged.
        assert_eq!(cascade(window(100, 100), &[(500, 500)]), window(100, 100));
        // One window there: a step down and right.
        assert_eq!(cascade(window(100, 100), &[(100, 100)]), window(132, 132));
        // Two, one step apart (a cascade already open): past both.
        assert_eq!(
            cascade(window(100, 100), &[(100, 100), (132, 132)]),
            window(164, 164)
        );
        // Nearly the same place counts too.
        assert_eq!(cascade(window(100, 100), &[(110, 95)]), window(132, 132));
    }

    #[test]
    fn window_showing_finds_the_document_by_its_canonical_path() {
        let tracker = WindowTracker::default();
        let crate_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        tracker.set_document("main", &crate_dir.join("Cargo.toml"));
        assert_eq!(
            tracker.window_showing(&crate_dir.join("src").join("..").join("Cargo.toml")),
            Some("main".to_string())
        );
        assert_eq!(tracker.window_showing(&crate_dir.join("build.rs")), None);
        tracker.forget("main");
        assert_eq!(tracker.window_showing(&crate_dir.join("Cargo.toml")), None);
    }

    #[test]
    fn only_file_urls_become_paths() {
        let file = std::path::absolute(Path::new("notes").join("a b.md")).expect("absolute");
        let file_url = tauri::Url::from_file_path(&file).expect("file URL");
        let web = tauri::Url::parse("https://example.com/a.md").expect("URL");
        assert_eq!(file_paths(&[web, file_url]), vec![file]);
    }

    #[test]
    fn no_monitors_means_no_change() {
        assert_eq!(
            clamp_to_monitors(window(-9999, -9999), &[]),
            window(-9999, -9999)
        );
    }
}
