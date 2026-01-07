//! 屏幕取词模块
//!
//! 使用 UI Automation API 轮询获取选中文本。
//! 当选中文本稳定 500ms 后显示气泡，文本变化或清空时关闭气泡。

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;
use std::thread;
use std::time::{Duration, Instant};
use tauri::{AppHandle, Emitter, Manager, WebviewWindowBuilder, WebviewUrl};
use windows::core::Interface;
use windows::Win32::System::Com::{CoInitializeEx, CoUninitialize, COINIT_APARTMENTTHREADED};
use windows::Win32::System::Ole::{
    SafeArrayAccessData, SafeArrayGetLBound, SafeArrayGetUBound, SafeArrayUnaccessData,
};
use windows::Win32::UI::Accessibility::{
    CUIAutomation, IUIAutomation, IUIAutomationTextPattern, UIA_TextPatternId,
};

use crate::debug_log;

/// 选中文本的位置信息
struct TextBounds {
    left: i32,
    top: i32,
    right: i32,
    bottom: i32,
}

/// 全局状态：屏幕取词是否启用
static SCREEN_CAPTURE_ENABLED: AtomicBool = AtomicBool::new(true);

/// 全局 AppHandle
static APP_HANDLE: Mutex<Option<AppHandle>> = Mutex::new(None);

/// 当前显示的气泡单词
static CURRENT_BUBBLE_WORD: Mutex<Option<String>> = Mutex::new(None);

/// 启用/禁用屏幕取词
pub fn set_enabled(enabled: bool) {
    SCREEN_CAPTURE_ENABLED.store(enabled, Ordering::SeqCst);
    debug_log(&format!("Screen capture enabled: {}", enabled));
}

/// 获取屏幕取词状态
pub fn is_enabled() -> bool {
    SCREEN_CAPTURE_ENABLED.load(Ordering::SeqCst)
}

/// 初始化屏幕取词
pub fn init_screen_capture(app: &AppHandle) -> Result<(), String> {
    debug_log("Initializing screen capture with polling...");

    // 保存 AppHandle
    {
        let mut handle = APP_HANDLE.lock().unwrap();
        *handle = Some(app.clone());
    }

    // 启动轮询线程
    thread::spawn(|| {
        if let Err(e) = start_polling() {
            debug_log(&format!("Polling thread error: {}", e));
        }
    });

    Ok(())
}

/// 启动轮询
fn start_polling() -> Result<(), String> {
    debug_log("Starting selection polling...");

    unsafe {
        // 初始化 COM
        CoInitializeEx(None, COINIT_APARTMENTTHREADED)
            .ok()
            .map_err(|e| format!("CoInitializeEx failed: {:?}", e))?;
    }

    // 创建 UI Automation 实例（复用，避免每次轮询都创建）
    let automation: IUIAutomation = unsafe {
        windows::Win32::System::Com::CoCreateInstance(
            &CUIAutomation,
            None,
            windows::Win32::System::Com::CLSCTX_INPROC_SERVER,
        )
        .map_err(|e| format!("Failed to create IUIAutomation: {:?}", e))?
    };

    // 上次检测到的文本和时间
    let mut last_text: Option<String> = None;
    let mut last_text_time: Option<Instant> = None;
    let mut bubble_shown_for: Option<String> = None;

    loop {
        thread::sleep(Duration::from_millis(200)); // 200ms 轮询间隔

        if !SCREEN_CAPTURE_ENABLED.load(Ordering::SeqCst) {
            continue;
        }

        // 获取当前选中文本
        let current = get_selected_text_with_automation(&automation);

        match current {
            Ok(Some((text, bounds))) => {
                let text = text.trim().to_string();

                if !is_valid_word(&text) {
                    // 无效文本，重置状态
                    if last_text.is_some() {
                        last_text = None;
                        last_text_time = None;
                    }
                    // 关闭气泡
                    if bubble_shown_for.is_some() {
                        close_bubble();
                        bubble_shown_for = None;
                    }
                    continue;
                }

                // 检查文本是否变化
                let text_changed = last_text.as_ref() != Some(&text);

                if text_changed {
                    // 文本变化，重置计时
                    last_text = Some(text.clone());
                    last_text_time = Some(Instant::now());

                    // 如果气泡显示的是不同的词，关闭它
                    if bubble_shown_for.as_ref() != Some(&text) && bubble_shown_for.is_some() {
                        close_bubble();
                        bubble_shown_for = None;
                    }
                } else if let Some(start_time) = last_text_time {
                    // 文本没变，检查是否稳定了 500ms
                    if start_time.elapsed() >= Duration::from_millis(500) {
                        // 稳定了，显示气泡（如果还没显示）
                        if bubble_shown_for.as_ref() != Some(&text) {
                            show_bubble(&text, bounds);
                            bubble_shown_for = Some(text.clone());
                        }
                    }
                }
            }
            Ok(None) | Err(_) => {
                // 没有选中文本或获取失败
                if last_text.is_some() {
                    last_text = None;
                    last_text_time = None;
                }
                // 关闭气泡
                if bubble_shown_for.is_some() {
                    close_bubble();
                    bubble_shown_for = None;
                }
            }
        }
    }
}

/// 关闭气泡窗口
fn close_bubble() {
    let app = {
        let handle = APP_HANDLE.lock().unwrap();
        handle.clone()
    };

    if let Some(app) = app {
        let app_clone = app.clone();
        let _ = app.run_on_main_thread(move || {
            if let Some(bubble) = app_clone.get_webview_window("bubble") {
                let _ = bubble.hide();
            }
        });
    }

    // 清除当前气泡单词
    let mut current = CURRENT_BUBBLE_WORD.lock().unwrap();
    *current = None;
}

/// 使用 UI Automation 获取选中文本及其位置（复用 automation 实例）
fn get_selected_text_with_automation(
    automation: &IUIAutomation,
) -> Result<Option<(String, Option<TextBounds>)>, String> {
    unsafe {
        // 获取焦点元素
        let focused = automation
            .GetFocusedElement()
            .map_err(|e| format!("GetFocusedElement failed: {:?}", e))?;

        // 尝试获取 TextPattern
        let pattern_obj = focused
            .GetCurrentPattern(UIA_TextPatternId)
            .map_err(|e| format!("GetCurrentPattern failed: {:?}", e))?;

        let text_pattern: IUIAutomationTextPattern = pattern_obj
            .cast()
            .map_err(|e| format!("Cast to TextPattern failed: {:?}", e))?;

        // 获取选中的文本范围
        let selection = text_pattern
            .GetSelection()
            .map_err(|e| format!("GetSelection failed: {:?}", e))?;

        let count = selection
            .Length()
            .map_err(|e| format!("Get selection length failed: {:?}", e))?;

        if count == 0 {
            return Ok(None);
        }

        // 获取第一个选中范围
        let range = selection
            .GetElement(0)
            .map_err(|e| format!("GetElement failed: {:?}", e))?;

        // 获取文本
        let text_bstr = range
            .GetText(-1)
            .map_err(|e| format!("GetText failed: {:?}", e))?;

        let text = text_bstr.to_string();

        if text.is_empty() {
            return Ok(None);
        }

        // 获取边界矩形
        let bounds = get_text_bounds(&range);

        Ok(Some((text, bounds)))
    }
}

/// 从 IUIAutomationTextRange 获取边界矩形
fn get_text_bounds(
    range: &windows::Win32::UI::Accessibility::IUIAutomationTextRange,
) -> Option<TextBounds> {
    unsafe {
        let sa_ptr = range.GetBoundingRectangles().ok()?;

        if sa_ptr.is_null() {
            return None;
        }

        // 获取数组边界
        let lower_bound = SafeArrayGetLBound(sa_ptr, 1).ok()?;
        let upper_bound = SafeArrayGetUBound(sa_ptr, 1).ok()?;

        let count = (upper_bound - lower_bound + 1) as usize;

        // 至少需要 4 个元素 (left, top, width, height)
        if count < 4 {
            return None;
        }

        // 访问数组数据
        let mut data_ptr: *mut std::ffi::c_void = std::ptr::null_mut();
        if SafeArrayAccessData(sa_ptr, &mut data_ptr).is_err() {
            return None;
        }

        let doubles = std::slice::from_raw_parts(data_ptr as *const f64, count);

        let left = doubles[0] as i32;
        let top = doubles[1] as i32;
        let width = doubles[2] as i32;
        let height = doubles[3] as i32;

        let _ = SafeArrayUnaccessData(sa_ptr);

        Some(TextBounds {
            left,
            top,
            right: left + width,
            bottom: top + height,
        })
    }
}

/// 验证是否为有效单词
fn is_valid_word(text: &str) -> bool {
    // 长度限制：1-50 字符
    if text.is_empty() || text.len() > 50 {
        return false;
    }

    // 只包含英文字母、连字符、撇号、空格
    let valid = text
        .chars()
        .all(|c| c.is_ascii_alphabetic() || c == '-' || c == '\'' || c == ' ');

    if !valid {
        return false;
    }

    // 至少包含一个字母
    text.chars().any(|c| c.is_ascii_alphabetic())
}

/// 显示气泡窗口
fn show_bubble(word: &str, bounds: Option<TextBounds>) {
    let app = {
        let handle = APP_HANDLE.lock().unwrap();
        handle.clone()
    };

    let Some(app) = app else {
        return;
    };

    // 检查是否已经显示了相同的单词
    {
        let current = CURRENT_BUBBLE_WORD.lock().unwrap();
        if current.as_ref() == Some(&word.to_string()) {
            return;
        }
    }

    // 更新当前气泡单词
    {
        let mut current = CURRENT_BUBBLE_WORD.lock().unwrap();
        *current = Some(word.to_string());
    }

    let word = word.to_string();
    let bounds_data = bounds.map(|b| (b.left, b.bottom));
    let app_clone = app.clone();

    let _ = app.run_on_main_thread(move || {
        // 获取主窗口用于获取显示器信息
        let main_window = match app_clone.get_webview_window("main") {
            Some(w) => w,
            None => return,
        };

        // 获取 DPI 缩放因子
        let scale_factor = main_window.scale_factor().unwrap_or(1.0);

        // 气泡尺寸
        let bubble_width = 320.0;
        let bubble_height = 150.0;

        // 计算气泡位置
        let (text_x, text_y) = if let Some((left, bottom)) = bounds_data {
            (
                (left as f64 / scale_factor) as i32,
                ((bottom + 5) as f64 / scale_factor) as i32,
            )
        } else {
            (100, 100) // 默认位置
        };

        // 获取屏幕尺寸（逻辑像素）
        let (screen_width, screen_height) = main_window
            .current_monitor()
            .ok()
            .flatten()
            .map(|m| {
                let size = m.size();
                (
                    (size.width as f64 / scale_factor) as i32,
                    (size.height as f64 / scale_factor) as i32,
                )
            })
            .unwrap_or((1920, 1080));

        // 计算气泡位置，默认在文本下方 10px
        let mut bubble_x = text_x;
        let mut bubble_y = text_y + 10;

        // 检查边界
        if bubble_x + bubble_width as i32 > screen_width {
            bubble_x = screen_width - bubble_width as i32 - 10;
        }
        if bubble_x < 10 {
            bubble_x = 10;
        }
        if bubble_y + bubble_height as i32 > screen_height {
            bubble_y = text_y - bubble_height as i32 - 30;
        }
        if bubble_y < 10 {
            bubble_y = 10;
        }

        let url = format!("/bubble?word={}", urlencoding::encode(&word));

        // 尝试复用已有的气泡窗口
        if let Some(bubble) = app_clone.get_webview_window("bubble") {
            // 更新位置
            let _ = bubble.set_position(tauri::Position::Logical(tauri::LogicalPosition {
                x: bubble_x as f64,
                y: bubble_y as f64,
            }));
            // 发送事件更新单词
            let _ = bubble.emit("update-word", &word);
            return;
        }

        // 创建新的气泡窗口
        let _ = WebviewWindowBuilder::new(&app_clone, "bubble", WebviewUrl::App(url.into()))
            .title("Dictyy Bubble")
            .inner_size(bubble_width, bubble_height)
            .position(bubble_x as f64, bubble_y as f64)
            .decorations(false)
            .transparent(true)
            .always_on_top(true)
            .skip_taskbar(true)
            .focused(false)
            .visible(false)
            .build();
    });
}

/// Tauri 命令：设置屏幕取词状态
#[tauri::command]
pub fn set_screen_capture_enabled(enabled: bool) {
    set_enabled(enabled);
}

/// Tauri 命令：获取屏幕取词状态
#[tauri::command]
pub fn get_screen_capture_enabled() -> bool {
    is_enabled()
}
