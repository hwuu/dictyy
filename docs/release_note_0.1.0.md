# Dictyy v0.1.0

首个发布版本。

## 功能

- 离线词典查询（基于 SQLite）
- LLM 回退（词典未收录时自动调用 LLM，返回结构化结果）
- 模糊匹配建议（支持拼写纠错）
- 全局快捷键唤起（Ctrl+`）
- 系统托盘支持
- 日志持久化

## 安装

1. 下载 Dictyy_0.1.0_x64-setup.exe 并运行
2. 编辑 %LOCALAPPDATA%\Dictyy\config.yaml 配置 LLM API

## 配置示例

```yaml
llm:
  api_base: "https://api.example.com/v1"
  api_key: "your-api-key"
  model: "model-name"
```

## 快捷键

| 快捷键 | 功能          |
|--------|---------------|
| Ctrl+` | 显示/隐藏窗口 |
| Esc    | 隐藏窗口      |
| ↑/↓    | 选择搜索建议  |
| Tab    | 补全选中建议  |
| Enter  | 查询          |
