use tauri::{Manager, WindowEvent};
use std::fs::OpenOptions;
use std::io::Write;

mod dictionary;
mod llm;
mod shortcuts;
mod tray;

/// 写调试日志到用户目录
pub fn debug_log(msg: &str) {
    if let Some(local_dir) = dirs::data_local_dir() {
        let log_dir = local_dir.join("Dictyy");
        let _ = std::fs::create_dir_all(&log_dir);
        let log_file = log_dir.join("debug.log");
        if let Ok(mut file) = OpenOptions::new().create(true).append(true).open(&log_file) {
            let timestamp = chrono::Local::now().format("%Y-%m-%d %H:%M:%S");
            let _ = writeln!(file, "[{}] {}", timestamp, msg);
        }
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    debug_log("=== Application starting ===");
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_global_shortcut::Builder::new().build())
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
            // Focus existing window when another instance is launched
            if let Some(window) = app.get_webview_window("main") {
                let _ = window.show();
                let _ = window.unminimize();
                let _ = window.set_focus();
            }
        }))
        .plugin(tauri_plugin_store::Builder::new().build())
        .plugin(tauri_plugin_clipboard_manager::init())
        .manage(dictionary::DictionaryState::new())
        .manage(llm::LlmState::new())
        .invoke_handler(tauri::generate_handler![
            shortcuts::setup_shortcuts,
            dictionary::lookup_word,
            dictionary::search_words,
            llm::llm_query,
            llm::get_llm_config
        ])
        .setup(|app| {
            debug_log("Setup starting...");
            let handle = app.handle();

            // Initialize dictionary
            debug_log("Initializing dictionary...");
            if let Err(e) = dictionary::init_dictionary(handle) {
                debug_log(&format!("ERROR: Failed to initialize dictionary: {}", e));
            } else {
                debug_log("Dictionary initialized successfully");
            }

            // Initialize LLM
            debug_log("Initializing LLM...");
            if let Err(e) = llm::init_llm(handle) {
                debug_log(&format!("ERROR: Failed to initialize LLM: {}", e));
            } else {
                debug_log("LLM initialized successfully");
            }

            // Initialize system tray
            debug_log("Initializing tray...");
            if let Err(e) = tray::init_tray(handle) {
                debug_log(&format!("ERROR: Failed to initialize tray: {}", e));
            } else {
                debug_log("Tray initialized successfully");
            }

            // Initialize default shortcuts
            debug_log("Initializing shortcuts...");
            if let Err(e) = shortcuts::init_shortcuts(handle) {
                debug_log(&format!("ERROR: Failed to initialize shortcuts: {}", e));
            } else {
                debug_log("Shortcuts initialized successfully");
            }

            debug_log("Setup completed");

            // Setup window close interception - hide instead of close
            if let Some(window) = app.get_webview_window("main") {
                // Set window size to 2/3 of screen and center it
                if let Some(monitor) = window.current_monitor().ok().flatten() {
                    let screen_size = monitor.size();
                    let width = (screen_size.width as f64 * 2.0 / 3.0) as u32;
                    let height = (screen_size.height as f64 * 2.0 / 3.0) as u32;
                    let x = ((screen_size.width - width) / 2) as i32;
                    let y = ((screen_size.height - height) / 2) as i32;

                    let _ = window.set_size(tauri::Size::Physical(tauri::PhysicalSize { width, height }));
                    let _ = window.set_position(tauri::Position::Physical(tauri::PhysicalPosition { x, y }));
                }

                let window_clone = window.clone();
                window.on_window_event(move |event| {
                    if let WindowEvent::CloseRequested { api, .. } = event {
                        api.prevent_close();
                        let _ = window_clone.hide();
                    }
                });

                // Show window on startup
                let _ = window.show();
            }

            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
