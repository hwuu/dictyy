use tauri::{Manager, WindowEvent};
use tauri_plugin_log::{Target, TargetKind};

mod dictionary;
mod llm;
mod shortcuts;
mod tray;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(
            tauri_plugin_log::Builder::new()
                .targets([
                    Target::new(TargetKind::Stdout),
                    Target::new(TargetKind::LogDir { file_name: None }),
                ])
                .build(),
        )
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
            let handle = app.handle();

            // Initialize dictionary
            if let Err(e) = dictionary::init_dictionary(handle) {
                log::error!("Failed to initialize dictionary: {}", e);
            }

            // Initialize LLM
            if let Err(e) = llm::init_llm(handle) {
                log::error!("Failed to initialize LLM: {}", e);
            }

            // Initialize system tray
            if let Err(e) = tray::init_tray(handle) {
                log::error!("Failed to initialize tray: {}", e);
            }

            // Initialize default shortcuts
            if let Err(e) = shortcuts::init_shortcuts(handle) {
                log::error!("Failed to initialize shortcuts: {}", e);
            }

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
