# Dictyy

Windows 桌面词典应用，基于 Tauri v2 + React 19 + TypeScript 构建。

## 功能特性

- 离线词典查询（SQLite）
- LLM 回退（词典未收录时自动调用 LLM）
- 模糊匹配建议（支持拼写纠错）
- 全局快捷键唤起（Ctrl+`）

## 开发环境

### 前置要求

- Node.js 18+
- Rust 1.70+
- Windows 11

### 安装依赖

```bash
npm install
```

### 配置

创建 `src-tauri/config.yaml`：

```yaml
llm:
  api_base: "https://api.example.com/v1"
  api_key: "your-api-key"
  model: "model-name"
  temperature: 0.3
  max_tokens: 2048
  timeout: 30
```

### 词典数据库

将 `dict.db` 放置到 `src-tauri/resources/` 目录。

### 运行

```bash
npm run tauri dev
```

### 构建

```bash
npm run tauri build
```

## 技术栈

- **前端**: React 19, TypeScript, Tailwind CSS, shadcn/ui
- **后端**: Tauri v2, Rust, SQLite
- **LLM**: OpenAI 兼容 API

## 快捷键

| 快捷键 | 功能 |
|--------|------|
| Ctrl+` | 显示/隐藏窗口 |
| Esc | 隐藏窗口 |
| ↑/↓ | 选择搜索建议 |
| Tab | 补全选中建议 |
| Enter | 查询 |

## License

MIT
