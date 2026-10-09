//! Window geometry: remembering it, and restoring it onto a monitor that is
//! actually connected (DESIGN.md §11).

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

use tauri::{PhysicalPosition, PhysicalSize, Window};

use crate::state::WindowGeometry;

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
    let wanted = Rect {
        x: geometry.x,
        y: geometry.y,
        width: geometry.width,
        height: geometry.height,
    };
    let rect = clamp_to_monitors(wanted, &monitors);
    window.set_size(PhysicalSize::new(rect.width, rect.height))?;
    window.set_position(PhysicalPosition::new(rect.x, rect.y))?;
    if geometry.maximized {
        window.maximize()?;
    }
    Ok(())
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
    fn no_monitors_means_no_change() {
        assert_eq!(
            clamp_to_monitors(window(-9999, -9999), &[]),
            window(-9999, -9999)
        );
    }
}
