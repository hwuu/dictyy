//! 屏幕取词模块
//!
//! 使用 UI Automation API 轮询获取选中文本。
//! 当 UIA 不支持时，回退到 Ctrl+Insert 方案。
//! 当选中文本稳定 500ms 后显示气泡，文本变化或清空时关闭气泡。

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;
use std::thread;
use std::time::{Duration, Instant};
use tauri::{AppHandle, Manager, WebviewWindowBuilder, WebviewUrl};
use windows::core::Interface;
use windows::Win32::System::Com::{CoInitializeEx, COINIT_APARTMENTTHREADED};
use windows::Win32::System::Ole::{
    SafeArrayAccessData, SafeArrayGetLBound, SafeArrayGetUBound, SafeArrayUnaccessData,
};
use windows::Win32::UI::Accessibility::{
    CUIAutomation, IUIAutomation, IUIAutomationTextPattern, UIA_TextPatternId,
};
use windows::Win32::UI::Input::KeyboardAndMouse::{
    SendInput, INPUT, INPUT_KEYBOARD, KEYBDINPUT, KEYBD_EVENT_FLAGS, KEYEVENTF_KEYUP, VK_CONTROL, VK_INSERT,
};
use windows::Win32::UI::WindowsAndMessaging::GetCursorPos;
use windows::Win32::Foundation::POINT;
use windows::Win32::System::Threading::{OpenProcess, QueryFullProcessImageNameW, PROCESS_QUERY_LIMITED_INFORMATION};

use log::{debug, info, warn};

/// 选中文本的位置信息
#[allow(dead_code)]
#[derive(Clone)]
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
    info!("Screen capture enabled: {}", enabled);
}

/// 获取屏幕取词状态
pub fn is_enabled() -> bool {
    SCREEN_CAPTURE_ENABLED.load(Ordering::SeqCst)
}

/// 初始化屏幕取词
pub fn init_screen_capture(app: &AppHandle) -> Result<(), String> {
    info!("Initializing screen capture with polling...");

    // 保存 AppHandle
    {
        let mut handle = APP_HANDLE.lock().unwrap();
        *handle = Some(app.clone());
    }

    // 启动轮询线程
    thread::spawn(|| {
        if let Err(e) = start_polling() {
            warn!("Polling thread error: {}", e);
        }
    });

    Ok(())
}

/// 获取进程名称（通过 PID）
fn get_process_name(pid: u32) -> Option<String> {
    use windows::core::PWSTR;
    use windows::Win32::System::Threading::PROCESS_NAME_WIN32;

    unsafe {
        let handle = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, pid).ok()?;
        let mut buffer = [0u16; 1024];
        let mut size = buffer.len() as u32;

        if QueryFullProcessImageNameW(handle, PROCESS_NAME_WIN32, PWSTR::from_raw(buffer.as_mut_ptr()), &mut size).is_ok() {
            let path = String::from_utf16_lossy(&buffer[..size as usize]);
            path.split('\\').last().map(|s| s.to_lowercase())
        } else {
            None
        }
    }
}

/// 检测是否为 Edge PDF（通过进程名和元素名）
fn is_edge_pdf(element_name: &str, pid: u32) -> bool {
    let name_lower = element_name.to_lowercase();

    if let Some(process_name) = get_process_name(pid) {
        let is_edge = process_name.contains("msedge") || process_name.contains("edge");
        let is_pdf_page = name_lower.contains("页") ||
                          name_lower.starts_with("page ") ||
                          name_lower.is_empty();
        return is_edge && is_pdf_page;
    }
    false
}

/// 启动轮询
fn start_polling() -> Result<(), String> {
    info!("Starting text selection polling...");

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

    // 上次检测到的文本、位置和时间
    let mut last_text: Option<(String, Option<TextBounds>)> = None;
    let mut last_text_time: Option<Instant> = None;
    let mut bubble_shown_for: Option<String> = None;
    // DEBUG: 上次焦点信息，用于减少重复日志
    let mut last_focus_info: Option<(String, u32)> = None;
    // Ctrl+Insert 回退相关
    let mut last_clipboard_fallback: Option<Instant> = None;
    let mut cached_clipboard_text: Option<(String, Option<TextBounds>)> = None;
    // 记录上次 clipboard 返回空的时间（用于避免频繁重试）
    let mut last_clipboard_empty_time: Option<Instant> = None;

    loop {
        thread::sleep(Duration::from_millis(200)); // 200ms 轮询间隔

        if !SCREEN_CAPTURE_ENABLED.load(Ordering::SeqCst) {
            continue;
        }

        // 获取当前选中文本（优先 UIA，失败则回退到 Ctrl+Insert）
        let (current, focus_changed): (Option<(String, Option<TextBounds>)>, bool) = match get_selected_text_with_automation(&automation, &mut last_focus_info) {
            Ok(result) => {
                // UIA 成功，清除 Ctrl+Insert 缓存和空结果时间
                cached_clipboard_text = None;
                last_clipboard_empty_time = None;
                (result.text, result.focus_changed)
            }
            Err(uia_error) => {
                // UIA 失败，只对 Edge PDF 使用 Ctrl+Insert fallback
                // 其他不支持 TextPattern 的应用不轮询检测（只在快捷键触发时处理）

                if is_edge_pdf(&uia_error.element_name, uia_error.pid) {
                    // Edge PDF：使用 Ctrl+Insert fallback 轮询
                    if uia_error.focus_changed {
                        // 焦点刚变化，不触发 Ctrl+Insert，清除缓存和空结果时间
                        cached_clipboard_text = None;
                        last_clipboard_empty_time = None;
                        (None, true)
                    } else {
                        // 焦点稳定，检查是否最近刚尝试过且返回空
                        let recently_empty = last_clipboard_empty_time
                            .map(|t| t.elapsed() < Duration::from_secs(5))
                            .unwrap_or(false);

                        if recently_empty {
                            // 最近（5秒内）尝试过且返回空，不再重复尝试
                            (None, false)
                        } else {
                            // 可以尝试，检查冷却时间
                            let can_fallback = last_clipboard_fallback
                                .map(|t| t.elapsed() >= Duration::from_secs(1))
                                .unwrap_or(true);

                            if can_fallback {
                                debug!("Using clipboard fallback for Edge PDF");
                                last_clipboard_fallback = Some(Instant::now());
                                let result = get_selected_text_with_clipboard();

                                // 如果返回空，记录时间
                                if result.is_none() {
                                    last_clipboard_empty_time = Some(Instant::now());
                                } else {
                                    // 有结果，清除空结果时间
                                    last_clipboard_empty_time = None;
                                }

                                // 缓存结果
                                cached_clipboard_text = result.clone();
                                (result, false)
                            } else {
                                // 冷却中，返回缓存的结果（保持状态）
                                (cached_clipboard_text.clone(), false)
                            }
                        }
                    }
                } else {
                    // 其他不支持 TextPattern 的应用：不使用 fallback 轮询
                    // 清除缓存和状态
                    cached_clipboard_text = None;
                    last_clipboard_empty_time = None;
                    (None, uia_error.focus_changed)
                }
            }
        };

        // 焦点变化时，关闭气泡并重置状态
        if focus_changed {
            if bubble_shown_for.is_some() {
                close_bubble();
                bubble_shown_for = None;
            }
            last_text = None;
            last_text_time = None;
        }

        match current {
            Some((text, bounds)) => {
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
                let text_changed = last_text.as_ref().map(|(t, _)| t) != Some(&text);

                if text_changed {
                    // 文本变化
                    if focus_changed {
                        // 焦点刚变化，只记录文本但不开始计时（忽略窗口切换时已选中的文本）
                        debug!("Text detected after focus change (ignored): '{}'", text);
                        last_text = Some((text.clone(), bounds));
                        last_text_time = None;
                    } else {
                        // 焦点稳定，正常的文本变化，开始计时
                        debug!("Text selection changed to: '{}'", text);
                        last_text = Some((text.clone(), bounds));
                        last_text_time = Some(Instant::now());

                        // 如果气泡显示的是不同的词，关闭它
                        if bubble_shown_for.as_ref() != Some(&text) && bubble_shown_for.is_some() {
                            close_bubble();
                            bubble_shown_for = None;
                        }
                    }
                } else if let Some(start_time) = last_text_time {
                    // 文本没变，检查是否稳定了 200ms
                    if start_time.elapsed() >= Duration::from_millis(200) {
                        // 稳定了，显示气泡（如果还没显示）
                        if bubble_shown_for.as_ref() != Some(&text) {
                            // 使用首次检测时保存的 bounds，而不是当前的 bounds
                            let saved_bounds = last_text.as_ref().and_then(|(_, b)| b.clone());
                            debug!("Showing bubble for text: '{}', bounds: {:?}", text, saved_bounds.as_ref().map(|b| (b.left, b.top, b.right, b.bottom)));
                            show_bubble(&text, saved_bounds);
                            bubble_shown_for = Some(text.clone());
                        }
                    }
                }
            }
            None => {
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
                let _ = bubble.close();
            }
        });
    }

    // 清除当前气泡单词
    let mut current = CURRENT_BUBBLE_WORD.lock().unwrap();
    *current = None;
}

/// UIA 失败时的错误信息
struct UiaError {
    focus_changed: bool,
    element_name: String,
    pid: u32,
    #[allow(dead_code)]
    message: String,
}

/// UIA 成功时的结果
struct UiaSuccess {
    text: Option<(String, Option<TextBounds>)>,
    focus_changed: bool,
}

/// 使用 UI Automation 获取选中文本及其位置（复用 automation 实例）
fn get_selected_text_with_automation(
    automation: &IUIAutomation,
    last_focus_info: &mut Option<(String, u32)>,
) -> Result<UiaSuccess, UiaError> {
    unsafe {
        // 获取焦点元素
        let focused = automation
            .GetFocusedElement()
            .map_err(|e| UiaError { focus_changed: false, element_name: String::new(), pid: 0, message: format!("GetFocusedElement failed: {:?}", e) })?;

        // 获取焦点元素的信息
        let class_name = focused.CurrentClassName().unwrap_or_default().to_string();
        let control_type = focused.CurrentControlType().unwrap_or_default().0 as i32;
        let name = focused.CurrentName().unwrap_or_default().to_string();
        let pid = focused.CurrentProcessId().unwrap_or_default() as u32;

        // 只在焦点变化时打印日志
        let current_focus = (name.clone(), pid);
        let focus_changed = last_focus_info.as_ref() != Some(&current_focus);

        if focus_changed {
            // 安全截断字符串（按字符数而非字节数）
            let truncated_name = if name.chars().count() > 30 {
                format!("{}...", name.chars().take(30).collect::<String>())
            } else {
                name.clone()
            };
            debug!(
                "Focus changed: class='{}', type={}, name='{}', pid={}",
                class_name, control_type, truncated_name, pid
            );
        }

        // 尝试获取 TextPattern
        let pattern_obj = match focused.GetCurrentPattern(UIA_TextPatternId) {
            Ok(p) => p,
            Err(e) => {
                if focus_changed {
                    debug!("TextPattern not supported: {:?}", e);
                    *last_focus_info = Some(current_focus);
                }
                return Err(UiaError { focus_changed, element_name: name, pid, message: format!("GetCurrentPattern failed: {:?}", e) });
            }
        };

        let text_pattern: IUIAutomationTextPattern = match pattern_obj.cast() {
            Ok(p) => p,
            Err(e) => {
                if focus_changed {
                    debug!("Cast to TextPattern failed: {:?}", e);
                    *last_focus_info = Some(current_focus);
                }
                return Err(UiaError { focus_changed, element_name: name, pid, message: format!("Cast to TextPattern failed: {:?}", e) });
            }
        };

        if focus_changed {
            debug!("TextPattern supported for current focus");
            *last_focus_info = Some(current_focus);
        }

        // 获取选中的文本范围
        let selection = match text_pattern.GetSelection() {
            Ok(s) => s,
            Err(_) => return Ok(UiaSuccess { text: None, focus_changed }), // 获取失败，返回无选中
        };

        let count = match selection.Length() {
            Ok(c) => c,
            Err(_) => return Ok(UiaSuccess { text: None, focus_changed }),
        };

        if count == 0 {
            return Ok(UiaSuccess { text: None, focus_changed });
        }

        // 获取第一个选中范围
        let range = match selection.GetElement(0) {
            Ok(r) => r,
            Err(_) => return Ok(UiaSuccess { text: None, focus_changed }),
        };

        // 获取文本
        let text_bstr = match range.GetText(-1) {
            Ok(t) => t,
            Err(_) => return Ok(UiaSuccess { text: None, focus_changed }),
        };

        let text = text_bstr.to_string();

        if text.is_empty() {
            return Ok(UiaSuccess { text: None, focus_changed });
        }

        // 获取边界矩形
        let bounds = get_text_bounds(&range);

        Ok(UiaSuccess { text: Some((text, bounds)), focus_changed })
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

        // 如果气泡窗口已存在，先关闭并等待
        if let Some(bubble) = app_clone.get_webview_window("bubble") {
            let _ = bubble.close();
            // 短暂延迟确保窗口完全关闭
            std::thread::sleep(std::time::Duration::from_millis(50));
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

/// 获取当前焦点的选中文本（同步调用，用于快捷键触发）
pub fn get_current_selected_text() -> Option<String> {
    // 初始化 COM（如果还没有初始化）
    unsafe {
        let _ = CoInitializeEx(None, COINIT_APARTMENTTHREADED);
    }

    // 创建 UI Automation 实例
    let automation: IUIAutomation = unsafe {
        windows::Win32::System::Com::CoCreateInstance(
            &CUIAutomation,
            None,
            windows::Win32::System::Com::CLSCTX_INPROC_SERVER,
        ).ok()?
    };

    // 获取焦点元素
    let focused = unsafe { automation.GetFocusedElement().ok()? };

    // 尝试获取 TextPattern
    let pattern_obj = unsafe { focused.GetCurrentPattern(UIA_TextPatternId).ok()? };
    let text_pattern: IUIAutomationTextPattern = pattern_obj.cast().ok()?;

    // 获取选中的文本范围
    let selection = unsafe { text_pattern.GetSelection().ok()? };
    let count = unsafe { selection.Length().ok()? };

    if count == 0 {
        return None;
    }

    // 获取第一个选中范围
    let range = unsafe { selection.GetElement(0).ok()? };

    // 获取文本
    let text_bstr = unsafe { range.GetText(-1).ok()? };
    let text = text_bstr.to_string();

    if text.is_empty() || !is_valid_word(&text) {
        return None;
    }

    Some(text.trim().to_string())
}

/// 使用 Ctrl+Insert 方案获取选中文本（UIA 失败时的回退方案）
fn get_selected_text_with_clipboard() -> Option<(String, Option<TextBounds>)> {
    use clipboard_win::{formats, get_clipboard, set_clipboard};

    // 1. 保存当前剪贴板内容
    let saved_clipboard: Option<String> = get_clipboard(formats::Unicode).ok();

    // 2. 清空剪贴板（用于检测是否有新内容）
    let _ = set_clipboard(formats::Unicode, "");

    // 3. 模拟 Ctrl+Insert
    unsafe {
        let inputs = [
            // Ctrl down
            INPUT {
                r#type: INPUT_KEYBOARD,
                Anonymous: windows::Win32::UI::Input::KeyboardAndMouse::INPUT_0 {
                    ki: KEYBDINPUT {
                        wVk: VK_CONTROL,
                        wScan: 0,
                        dwFlags: KEYBD_EVENT_FLAGS(0),
                        time: 0,
                        dwExtraInfo: 0,
                    },
                },
            },
            // Insert down
            INPUT {
                r#type: INPUT_KEYBOARD,
                Anonymous: windows::Win32::UI::Input::KeyboardAndMouse::INPUT_0 {
                    ki: KEYBDINPUT {
                        wVk: VK_INSERT,
                        wScan: 0,
                        dwFlags: KEYBD_EVENT_FLAGS(0),
                        time: 0,
                        dwExtraInfo: 0,
                    },
                },
            },
            // Insert up
            INPUT {
                r#type: INPUT_KEYBOARD,
                Anonymous: windows::Win32::UI::Input::KeyboardAndMouse::INPUT_0 {
                    ki: KEYBDINPUT {
                        wVk: VK_INSERT,
                        wScan: 0,
                        dwFlags: KEYEVENTF_KEYUP,
                        time: 0,
                        dwExtraInfo: 0,
                    },
                },
            },
            // Ctrl up
            INPUT {
                r#type: INPUT_KEYBOARD,
                Anonymous: windows::Win32::UI::Input::KeyboardAndMouse::INPUT_0 {
                    ki: KEYBDINPUT {
                        wVk: VK_CONTROL,
                        wScan: 0,
                        dwFlags: KEYEVENTF_KEYUP,
                        time: 0,
                        dwExtraInfo: 0,
                    },
                },
            },
        ];

        SendInput(&inputs, std::mem::size_of::<INPUT>() as i32);
    }

    // 4. 等待复制完成
    thread::sleep(Duration::from_millis(100));

    // 5. 读取剪贴板
    let new_text: Option<String> = get_clipboard(formats::Unicode).ok();

    // 6. 恢复剪贴板
    if let Some(saved) = saved_clipboard {
        let _ = set_clipboard(formats::Unicode, &saved);
    }

    // 7. 获取鼠标位置作为气泡显示位置（因为 Ctrl+Insert 回退无法获取精确文本边界）
    let bounds = get_mouse_position();

    // 检查是否获取到新文本
    if let Some(text) = new_text {
        let text = text.trim().to_string();
        if !text.is_empty() {
            return Some((text, bounds));
        }
    }

    None
}

/// 获取焦点元素的边界矩形
#[allow(dead_code)]
fn get_focused_element_bounds(automation: &IUIAutomation) -> Option<TextBounds> {
    unsafe {
        let focused = automation.GetFocusedElement().ok()?;
        let rect = focused.CurrentBoundingRectangle().ok()?;
        Some(TextBounds {
            left: rect.left,
            top: rect.top,
            right: rect.right,
            bottom: rect.bottom,
        })
    }
}

/// 获取鼠标位置（用于 Ctrl+Insert 回退方案）
fn get_mouse_position() -> Option<TextBounds> {
    unsafe {
        let mut point = POINT::default();
        if GetCursorPos(&mut point).is_ok() {
            // 返回以鼠标位置为中心的小矩形，气泡会显示在其下方
            Some(TextBounds {
                left: point.x,
                top: point.y,
                right: point.x,
                bottom: point.y,
            })
        } else {
            None
        }
    }
}
