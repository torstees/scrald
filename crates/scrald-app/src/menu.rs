//! The macOS menu bar (DESIGN.md §11.1). Tauri gives a Mac app a default
//! menu (the app menu, File, Edit, View, Window, Help) whose items do the
//! native thing. Scrald keeps it, with three items of its own in the app menu:
//!
//! - **About Scrald** opens Scrald's About dialog (DESIGN.md §10) rather
//!   than the system's About panel.
//! - **Settings…** (`Cmd+,`), where Mac apps keep it, opens Scrald's
//!   settings dialog. The default menu has no such item.
//! - **Quit Scrald**: the default Quit ends the app at once, without the
//!   close request each window needs to ask about unsaved changes (DESIGN.md
//!   §9.3), so Scrald's closes every window the way its close button does.

use tauri::menu::{Menu, MenuEvent, MenuItem, MenuItemKind, PredefinedMenuItem, Submenu};
use tauri::{AppHandle, Emitter, EventTarget, Manager, Wry};

/// The ids of Scrald's own items, to recognize them in menu events.
const ABOUT_ID: &str = "scrald-about";
const SETTINGS_ID: &str = "scrald-settings";
const QUIT_ID: &str = "scrald-quit";

/// The events that ask a window's page to show the About or settings dialog.
const SHOW_ABOUT: &str = "show-about";
const SHOW_SETTINGS: &str = "show-settings";

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
    add_settings(app, &app_menu)?;
    let quit = MenuItem::with_id(app, QUIT_ID, "Quit Scrald", true, Some("Cmd+Q"))?;
    replace_predefined(&app_menu, "Quit", &quit)?;
    Ok(menu)
}

/// Adds **Settings…** after About and the separator below it, in a group of
/// its own, as Mac apps lay out their app menu.
fn add_settings(app: &AppHandle, app_menu: &Submenu<Wry>) -> tauri::Result<()> {
    let items = app_menu.items()?;
    let Some(about_at) = items.iter().position(|item| item.id() == ABOUT_ID) else {
        tracing::warn!("no About item; Settings… left out of the app menu");
        return Ok(());
    };
    let settings = MenuItem::with_id(app, SETTINGS_ID, "Settings…", true, Some("Cmd+,"))?;
    // Rust note: `min` keeps the index in range if the menu is shorter than
    // expected (`insert` past the end would be an error).
    let at = (about_at + 2).min(items.len());
    app_menu.insert(&settings, at)?;
    app_menu.insert(&PredefinedMenuItem::separator(app)?, at + 1)?;
    Ok(())
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
        show_in_front(app, SHOW_ABOUT);
    } else if event.id() == SETTINGS_ID {
        show_in_front(app, SHOW_SETTINGS);
    } else if event.id() == QUIT_ID {
        quit(app);
    }
}

/// Asks the window in front to show a dialog (`event`). The menu bar belongs
/// to the frontmost app, so a Scrald window is normally focused; if none
/// reports focus, any window will do. There's always one, since Scrald ends
/// with its last window.
fn show_in_front(app: &AppHandle, event: &str) {
    let windows = app.webview_windows();
    let focused = windows
        .values()
        .find(|window| window.is_focused().unwrap_or(false))
        .or_else(|| windows.values().next());
    let Some(window) = focused else { return };
    let target = EventTarget::webview_window(window.label());
    if let Err(error) = app.emit_to(target, event, ()) {
        tracing::warn!(%error, event, "could not ask a window to show a dialog");
    }
}

/// Asks every window to close, as its close button would. A window without
/// unsaved changes closes; one with them asks "Save / Don't save / Cancel"
/// (or saves, with autosave on). The app ends when the last window closes,
/// so cancelling in one window keeps Scrald running with that window open.
pub fn quit(app: &AppHandle) {
    tracing::info!("quit requested from the menu");
    for (label, window) in app.webview_windows() {
        if let Err(error) = window.close() {
            tracing::warn!(window = label, error = %error, "could not ask a window to close");
        }
    }
}
