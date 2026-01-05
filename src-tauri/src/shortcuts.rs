//! Global shortcut implementation for Dictyy
//!
//! Handles Ctrl+` shortcut to toggle window visibility.

use tauri::{AppHandle, Emitter, Manager, Runtime};
use tauri_plugin_global_shortcut::{GlobalShortcutExt, Shortcut, ShortcutState};

/// Default shortcut key
pub const DEFAULT_SHORTCUT: &str = "Ctrl+`";

/// Setup global shortcuts
///
/// # Arguments
/// * `app` - Tauri app handle
/// * `shortcut_str` - Shortcut string (e.g., "Ctrl+`")
/// * `enabled` - Whether to enable the shortcut
#[tauri::command]
pub async fn setup_shortcuts<R: Runtime>(
    app: AppHandle<R>,
    shortcut_str: String,
    enabled: bool,
) -> Result<(), String> {
    let shortcuts = app.global_shortcut();

    // Unregister all existing shortcuts first
    shortcuts
        .unregister_all()
        .map_err(|e| format!("Failed to unregister shortcuts: {}", e))?;

    if !enabled {
        return Ok(());
    }

    // Parse shortcut
    let shortcut: Shortcut = shortcut_str
        .parse()
        .map_err(|e| format!("Failed to parse shortcut '{}': {}", shortcut_str, e))?;

    // Register shortcut
    let app_handle = app.clone();
    shortcuts
        .on_shortcut(shortcut, move |_app, _shortcut, event| {
            if event.state == ShortcutState::Pressed {
                toggle_window(&app_handle);
            }
        })
        .map_err(|e| format!("Failed to register shortcut: {}", e))?;

    Ok(())
}

/// Toggle window visibility
fn toggle_window<R: Runtime>(app: &AppHandle<R>) {
    if let Some(window) = app.get_webview_window("main") {
        if window.is_visible().unwrap_or(false) {
            let _ = window.hide();
        } else {
            let _ = window.show();
            let _ = window.unminimize();
            let _ = window.set_focus();
            // Notify frontend to focus input
            let _ = app.emit("new-query", ());
        }
    }
}

/// Initialize default shortcut on app startup
pub fn init_shortcuts<R: Runtime>(app: &AppHandle<R>) -> Result<(), String> {
    let shortcuts = app.global_shortcut();

    let shortcut: Shortcut = DEFAULT_SHORTCUT
        .parse()
        .map_err(|e| format!("Failed to parse default shortcut: {}", e))?;

    let app_handle = app.clone();
    shortcuts
        .on_shortcut(shortcut, move |_app, _shortcut, event| {
            if event.state == ShortcutState::Pressed {
                toggle_window(&app_handle);
            }
        })
        .map_err(|e| format!("Failed to register default shortcut: {}", e))?;

    Ok(())
}
