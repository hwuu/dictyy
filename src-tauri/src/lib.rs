use log::{error, info};
use tauri::{Manager, WindowEvent};
use std::path::PathBuf;

mod dictionary;
mod llm;
mod shortcuts;
mod tray;
#[cfg(windows)]
mod screen_capture;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    // 构建日志目录路径：%LOCALAPPDATA%\Dictyy\logs
    let log_dir = dirs::data_local_dir()
        .map(|d| d.join("Dictyy").join("logs"))
        .unwrap_or_else(|| PathBuf::from("logs"));

    tauri::Builder::default()
        // 日志插件需要最先初始化
        .plugin(
            tauri_plugin_log::Builder::new()
                .target(tauri_plugin_log::Target::new(
                    tauri_plugin_log::TargetKind::Folder {
                        path: log_dir,
                        file_name: Some("Dictyy".to_string()),
                    },
                ))
                .level(if cfg!(debug_assertions) {
                    log::LevelFilter::Debug  // 开发模式显示 DEBUG
                } else {
                    log::LevelFilter::Info   // 生产模式只显示 INFO+
                })
                .build(),
        )
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
            dictionary::lookup_collins,
            dictionary::lookup_etyma,
            dictionary::lookup_gpt4,
            dictionary::lookup_abstract,
            llm::llm_query,
            llm::get_llm_config,
            #[cfg(windows)]
            screen_capture::set_screen_capture_enabled,
            #[cfg(windows)]
            screen_capture::get_screen_capture_enabled
        ])
        .setup(|app| {
            info!("Application setup starting...");
            let handle = app.handle();

            // Initialize dictionary
            info!("Initializing dictionary module...");
            if let Err(e) = dictionary::init_dictionary(handle) {
                error!("Failed to initialize dictionary: {}", e);
            } else {
                info!("Dictionary initialized successfully");
            }

            // Initialize LLM
            info!("Initializing LLM module...");
            if let Err(e) = llm::init_llm(handle) {
                error!("Failed to initialize LLM: {}", e);
            } else {
                info!("LLM initialized successfully");
            }

            // Initialize system tray
            info!("Initializing system tray...");
            if let Err(e) = tray::init_tray(handle) {
                error!("Failed to initialize tray: {}", e);
            } else {
                info!("System tray initialized successfully");
            }

            // Initialize default shortcuts
            info!("Initializing shortcuts...");
            if let Err(e) = shortcuts::init_shortcuts(handle) {
                error!("Failed to initialize shortcuts: {}", e);
            } else {
                info!("Shortcuts initialized successfully");
            }

            // Initialize screen capture (Windows only)
            #[cfg(windows)]
            {
                info!("Initializing screen capture...");
                if let Err(e) = screen_capture::init_screen_capture(handle) {
                    error!("Failed to initialize screen capture: {}", e);
                } else {
                    info!("Screen capture initialized successfully");
                }
            }

            info!("Application setup completed successfully");

            // Setup window close interception - hide instead of close
            if let Some(window) = app.get_webview_window("main") {
                // Set window size: 2/3 of screen width, 3/4 of screen height
                if let Some(monitor) = window.current_monitor().ok().flatten() {
                    let screen_size = monitor.size();
                    let width = (screen_size.width as f64 * 2.0 / 3.0) as u32;
                    let height = (screen_size.height as f64 * 3.0 / 4.0) as u32;
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
