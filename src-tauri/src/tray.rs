//! System tray implementation for Dictyy
//!
//! Provides tray icon with context menu for window control.

use tauri::{
    menu::{CheckMenuItem, Menu, MenuItem, PredefinedMenuItem},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    AppHandle, Emitter, Manager, Runtime,
};

#[cfg(windows)]
use crate::screen_capture;

/// Tray menu item IDs
const MENU_ID_SHOW: &str = "tray-show";
const MENU_ID_SCREEN_CAPTURE: &str = "tray-screen-capture";
const MENU_ID_EXIT: &str = "tray-exit";

/// Initialize the system tray
pub fn init_tray<R: Runtime>(app: &AppHandle<R>) -> Result<(), Box<dyn std::error::Error>> {
    // Build menu items
    let show_item = MenuItem::with_id(app, MENU_ID_SHOW, "Show", true, None::<&str>)?;

    // Screen capture toggle (default enabled)
    #[cfg(windows)]
    let screen_capture_item = CheckMenuItem::with_id(
        app,
        MENU_ID_SCREEN_CAPTURE,
        "屏幕取词",
        true,
        screen_capture::is_enabled(),
        None::<&str>,
    )?;

    let separator = PredefinedMenuItem::separator(app)?;
    let exit_item = MenuItem::with_id(app, MENU_ID_EXIT, "Exit", true, None::<&str>)?;

    // Build menu
    #[cfg(windows)]
    let menu = Menu::with_items(app, &[&show_item, &screen_capture_item, &separator, &exit_item])?;

    #[cfg(not(windows))]
    let menu = Menu::with_items(app, &[&show_item, &separator, &exit_item])?;

    // Build tray icon
    let _tray = TrayIconBuilder::new()
        .menu(&menu)
        .tooltip("Dictyy - Dictionary")
        .icon(app.default_window_icon().unwrap().clone())
        .on_menu_event(|app, event| match event.id.as_ref() {
            MENU_ID_SHOW => {
                show_and_focus(app);
            }
            #[cfg(windows)]
            MENU_ID_SCREEN_CAPTURE => {
                // Toggle screen capture
                let new_state = !screen_capture::is_enabled();
                screen_capture::set_enabled(new_state);
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
                show_and_focus(app);
            }
        })
        .build(app)?;

    Ok(())
}

/// Show window and focus input
fn show_and_focus<R: Runtime>(app: &AppHandle<R>) {
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.show();
        let _ = window.unminimize();
        let _ = window.set_focus();
        let _ = app.emit("new-query", ());
    }
}
