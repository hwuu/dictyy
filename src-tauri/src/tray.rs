//! System tray implementation for Dictyy
//!
//! Provides tray icon with context menu for window control.

use tauri::{
    menu::{Menu, MenuItem},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    AppHandle, Emitter, Manager, Runtime,
};

/// Tray menu item IDs
const MENU_ID_SHOW: &str = "tray-show";
const MENU_ID_QUERY: &str = "tray-query";
const MENU_ID_EXIT: &str = "tray-exit";

/// Initialize the system tray
pub fn init_tray<R: Runtime>(app: &AppHandle<R>) -> Result<(), Box<dyn std::error::Error>> {
    // Build menu items
    let show_item = MenuItem::with_id(app, MENU_ID_SHOW, "Show", true, None::<&str>)?;
    let query_item = MenuItem::with_id(app, MENU_ID_QUERY, "New Query", true, None::<&str>)?;
    let exit_item = MenuItem::with_id(app, MENU_ID_EXIT, "Exit", true, None::<&str>)?;

    // Build menu
    let menu = Menu::with_items(app, &[&show_item, &query_item, &exit_item])?;

    // Build tray icon
    let _tray = TrayIconBuilder::new()
        .menu(&menu)
        .tooltip("Dictyy - Dictionary")
        .icon(app.default_window_icon().unwrap().clone())
        .on_menu_event(|app, event| match event.id.as_ref() {
            MENU_ID_SHOW => {
                show_window(app);
            }
            MENU_ID_QUERY => {
                show_window(app);
                let _ = app.emit("new-query", ());
            }
            MENU_ID_EXIT => {
                app.exit(0);
            }
            _ => {}
        })
        .on_tray_icon_event(|tray, event| {
            if let TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                ..
            } = event
            {
                let app = tray.app_handle();
                show_window(app);
            }
        })
        .build(app)?;

    Ok(())
}

/// Show and focus the main window (three-step recovery)
fn show_window<R: Runtime>(app: &AppHandle<R>) {
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.show();
        let _ = window.unminimize();
        let _ = window.set_focus();
    }
}
