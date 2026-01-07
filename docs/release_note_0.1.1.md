# Dictyy v0.1.1 Release Notes

## 新功能

- **日志持久化**：使用 tauri-plugin-log 将日志写入 `%LOCALAPPDATA%\Dictyy\logs\`
- **LLM 结构化输出**：LLM 返回结构化 JSON，与普通单词显示格式一致
- **记忆技巧**：新增记忆技巧折叠区域，辅助单词记忆

## 改进

- 新增可拖动 Caption Bar，方便移动窗口
- Status Bar 显示 LLM 配置信息（api_base | model）
- 简化托盘菜单，合并 Show/New Query 选项
- 优化输入框聚焦样式

## 文档

- 更新 README 和 CLAUDE.md 文档

---

## v0.1.0 (Initial Release)

### 核心功能

- **离线词典查询**：SQLite 存储，支持 CET4/CET6/TOEFL/IELTS/GRE/GMAT 词库
- **LLM Fallback**：离线词典未收录时自动调用 LLM 查询
- **搜索建议**：输入时模糊匹配，显示候选词列表
- **全局快捷键**：`Ctrl+`` 唤出/隐藏窗口
- **系统托盘**：常驻托盘，关闭窗口自动隐藏
