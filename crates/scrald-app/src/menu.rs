//! The macOS menu bar (DESIGN.md §11.1). Tauri gives a Mac app a default
//! menu (the app menu, File, Edit, View, Window, Help) whose items do the
//! native thing. That's right for all of them but Quit: it ends the app at
//! once, without the close request each window needs to ask about unsaved
//! changes (DESIGN.md §9.3). So Scrald uses the default menu with its own
//! Quit item, which closes every window the way its close button does.

use tauri::menu::{Menu, MenuEvent, MenuItem, MenuItemKind};
use tauri::{AppHandle, Manager, Wry};

/// The id of Scrald's Quit item, to recognize it in menu events.
const QUIT_ID: &str = "scrald-quit";

/// Tauri's default menu, with the app menu's Quit replaced by Scrald's.
// Rust note: `Menu<Wry>` names the menu type for Tauri's one webview
// runtime (Wry); Tauri's types are generic over the runtime, and this is
// the concrete one the app uses.
pub fn build(app: &AppHandle) -> tauri::Result<Menu<Wry>> {
    let menu = Menu::default(app)?;
    // The first submenu is the app menu: About, Services, Hide, Quit.
    let Some(MenuItemKind::Submenu(app_menu)) = menu.items()?.into_iter().next() else {
        tracing::warn!("no app menu; keeping the default Quit");
        return Ok(menu);
    };
    let items = app_menu.items()?;
    let quit_at = items.iter().position(|item| match item {
        MenuItemKind::Predefined(predefined) => {
            predefined.text().is_ok_and(|text| text.starts_with("Quit"))
        }
        _ => false,
    });
    let Some(quit_at) = quit_at else {
        tracing::warn!("no Quit item in the app menu; keeping the default menu");
        return Ok(menu);
    };
    app_menu.remove_at(quit_at)?;
    let quit = MenuItem::with_id(app, QUIT_ID, "Quit Scrald", true, Some("Cmd+Q"))?;
    app_menu.insert(&quit, quit_at)?;
    Ok(menu)
}

/// Handles a menu item Scrald added. The predefined items need nothing from
/// Scrald.
pub fn on_menu_event(app: &AppHandle, event: MenuEvent) {
    if event.id() == QUIT_ID {
        quit(app);
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
