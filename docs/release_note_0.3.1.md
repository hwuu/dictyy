# Dictyy v0.3.1 Release Notes

## Bug 修复

- **修复焦点在主窗口时按快捷键不隐藏的问题**：通过进程 ID 检测，避免获取自身窗口的选中文本
- **修复不支持 TextPattern 的网页快捷键查询失败**：为 `get_current_selected_text()` 添加 clipboard fallback 支持
- **修复气泡延迟显示问题**：将空结果冷却时间从 5 秒缩短到 1.5 秒
- **修复焦点变化后文本检测逻辑**：焦点变化时检测到选中文本会在稳定后开始计时显示气泡

## 改进

- **优化 Clipboard Fallback 策略**：
  - 对所有不支持 TextPattern 的应用默认使用 Ctrl+Insert fallback
  - 黑名单排除：Terminal、PowerShell、CMD、桌面、任务栏等（避免干扰）
  - 移除了复杂的应用白名单判断，使用更简单的黑名单策略
- **完善快捷键查询**：Ctrl+` 在不支持 TextPattern 的页面也能正常查询选中文本

## 技术细节

- 新增 `should_skip_clipboard_fallback()` 函数用于黑名单检查
- 新增 `is_focus_in_current_app()` 函数用于检测焦点是否在当前应用
- `get_current_selected_text()` 支持 clipboard fallback（与轮询逻辑保持一致）
- 优化焦点变化后的文本稳定性检测逻辑
