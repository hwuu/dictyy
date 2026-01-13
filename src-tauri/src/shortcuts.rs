//! Global shortcut implementation for Dictyy
//!
//! Handles Ctrl+` shortcut to toggle window visibility.

use tauri::{AppHandle, Emitter, Manager, Runtime};
use tauri_plugin_global_shortcut::{GlobalShortcutExt, Shortcut, ShortcutState};

#[cfg(windows)]
use crate::screen_capture;

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
        let is_visible = window.is_visible().unwrap_or(false);

        // 尝试获取当前选中的文本
        #[cfg(windows)]
        let selected_text = screen_capture::get_current_selected_text();
        #[cfg(not(windows))]
        let selected_text: Option<String> = None;

        if is_visible {
            // 窗口已可见
            if let Some(word) = selected_text {
                // 有选中文本 → 查询新单词（不隐藏窗口）
                #[derive(serde::Serialize, Clone)]
                struct ShowWordDetail {
                    word: String,
                }
                let _ = window.set_focus();
                let _ = app.emit("show-word-detail", ShowWordDetail { word });
            } else {
                // 没有选中文本 → 隐藏窗口
                let _ = window.hide();
            }
        } else {
            // 窗口不可见 → 显示窗口
            let _ = window.show();
            let _ = window.unminimize();
            let _ = window.set_focus();

            // 如果有选中的文本，发送 show-word-detail 事件进行查询
            // 否则发送 new-query 事件聚焦输入框
            if let Some(word) = selected_text {
                #[derive(serde::Serialize, Clone)]
                struct ShowWordDetail {
                    word: String,
                }
                let _ = app.emit("show-word-detail", ShowWordDetail { word });
            } else {
                let _ = app.emit("new-query", ());
            }
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
