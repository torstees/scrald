//! Windows: opening one per document, and remembering their geometry and
//! restoring it onto a monitor that is actually connected (DESIGN.md §11).

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::sync::atomic::{AtomicU32, Ordering};

use anyhow::Context;
use tauri::{AppHandle, Manager, PhysicalPosition, PhysicalSize, WebviewWindowBuilder, Window};

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
        .filter_map(|w| w.outer_position().ok())
        .map(|p| (p.x, p.y))
        .collect();
    tracing::info!(window = config.label, path = ?launch.path, "opening a new window");
    show_placed(&window, &launch, &open)
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

/// A rectangle in physical pixels.
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
/// `monitors`; otherwise moves it (shrinking if needed) to the center of the
/// first monitor, which callers pass as the primary one. With no monitors
/// known, leaves it alone.
pub fn clamp_to_monitors(window: Rect, monitors: &[Rect]) -> Rect {
    let reachable = monitors.iter().any(|m| {
        let overlap_x = window.right().min(m.right()) - i64::from(window.x.max(m.x));
        let title_top = i64::from(window.y);
        overlap_x >= MIN_VISIBLE_WIDTH
            && title_top >= i64::from(m.y)
            && title_top + MIN_TITLE_BAR <= m.bottom()
    });
    if reachable {
        return window;
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
    let position = window.outer_position().ok()?;
    let size = window.inner_size().ok()?;
    Some(Rect {
        x: position.x,
        y: position.y,
        width: size.width,
        height: size.height,
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
    window.set_size(PhysicalSize::new(rect.width, rect.height))?;
    window.set_position(PhysicalPosition::new(rect.x, rect.y))?;
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
    let area = monitor.work_area();
    Rect {
        x: area.position.x,
        y: area.position.y,
        width: area.size.width,
        height: area.size.height,
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
    fn no_monitors_means_no_change() {
        assert_eq!(
            clamp_to_monitors(window(-9999, -9999), &[]),
            window(-9999, -9999)
        );
    }
}
