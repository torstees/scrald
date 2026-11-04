//! The macOS menu bar (DESIGN.md §11.1). Tauri gives a Mac app a default
//! menu (the app menu, File, Edit, View, Window, Help) whose items do the
//! native thing. Scrald keeps it, with two items of its own in the app menu:
//!
//! - **About Scrald** opens Scrald's About dialog (DESIGN.md §10) rather
//!   than the system's About panel.
//! - **Quit Scrald**: the default Quit ends the app at once, without the
//!   close request each window needs to ask about unsaved changes (DESIGN.md
//!   §9.3), so Scrald's closes every window the way its close button does.

use tauri::menu::{Menu, MenuEvent, MenuItem, MenuItemKind, Submenu};
use tauri::{AppHandle, Emitter, EventTarget, Manager, Wry};

/// The ids of Scrald's own items, to recognize them in menu events.
const ABOUT_ID: &str = "scrald-about";
const QUIT_ID: &str = "scrald-quit";

/// The event that asks a window's page to show the About dialog.
const SHOW_ABOUT: &str = "show-about";

/// Tauri's default menu, with the app menu's About and Quit replaced by
/// Scrald's.
// Rust note: `Menu<Wry>` names the menu type for Tauri's one webview
// runtime (Wry); Tauri's types are generic over the runtime, and this is
// the concrete one the app uses.
pub fn build(app: &AppHandle) -> tauri::Result<Menu<Wry>> {
    let menu = Menu::default(app)?;
    // The first submenu is the app menu: About, Services, Hide, Quit.
    let Some(MenuItemKind::Submenu(app_menu)) = menu.items()?.into_iter().next() else {
        tracing::warn!("no app menu; keeping the default menu");
        return Ok(menu);
    };
    let about = MenuItem::with_id(app, ABOUT_ID, "About Scrald", true, None::<&str>)?;
    replace_predefined(&app_menu, "About", &about)?;
    let quit = MenuItem::with_id(app, QUIT_ID, "Quit Scrald", true, Some("Cmd+Q"))?;
    replace_predefined(&app_menu, "Quit", &quit)?;
    Ok(menu)
}

/// Puts `item` in place of the predefined item whose title starts with
/// `title`. If there's no such item, leaves the menu as it is and logs it.
fn replace_predefined(
    submenu: &Submenu<Wry>,
    title: &str,
    item: &MenuItem<Wry>,
) -> tauri::Result<()> {
    let position = submenu.items()?.iter().position(|existing| match existing {
        MenuItemKind::Predefined(predefined) => {
            predefined.text().is_ok_and(|text| text.starts_with(title))
        }
        _ => false,
    });
    match position {
        Some(position) => {
            submenu.remove_at(position)?;
            submenu.insert(item, position)?;
        }
        None => tracing::warn!(title, "no such item in the app menu; keeping the default"),
    }
    Ok(())
}

/// Handles the menu items Scrald added. The predefined items need nothing
/// from Scrald.
pub fn on_menu_event(app: &AppHandle, event: MenuEvent) {
    if event.id() == ABOUT_ID {
        show_about(app);
    } else if event.id() == QUIT_ID {
        quit(app);
    }
}

/// Asks the window in front to show the About dialog. The menu bar belongs
/// to the frontmost app, so a Scrald window is normally focused; if none
/// reports focus, any window will do. There's always one, since Scrald ends
/// with its last window.
fn show_about(app: &AppHandle) {
    let windows = app.webview_windows();
    let focused = windows
        .values()
        .find(|window| window.is_focused().unwrap_or(false))
        .or_else(|| windows.values().next());
    let Some(window) = focused else { return };
    let target = EventTarget::webview_window(window.label());
    if let Err(error) = app.emit_to(target, SHOW_ABOUT, ()) {
        tracing::warn!(%error, "could not ask a window to show About");
    }
}

/// Asks every window to close, as its close button would. A window without
/// unsaved changes closes; one with them asks "Save / Don't save / Cancel"
/// (or saves, with autosave on). The app ends when the last window closes,
/// so cancelling in one window keeps Scrald running with that window open.
fn quit(app: &AppHandle) {
    tracing::info!("quit requested from the menu");
    for (label, window) in app.webview_windows() {
        if let Err(error) = window.close() {
            tracing::warn!(window = label, error = %error, "could not ask a window to close");
        }
    }
}
