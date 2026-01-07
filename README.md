# Dictyy

Windows 桌面词典应用，基于 Tauri v2 + React 19 + TypeScript 构建。

## 功能特性

- 离线词典查询（SQLite）
- LLM 回退（词典未收录时自动调用 LLM）
- 模糊匹配建议（支持拼写纠错）
- 屏幕取词（选中文本自动弹出释义气泡）
- 全局快捷键唤起（Ctrl+`）
- 系统托盘支持

## 安装

下载 [Releases](https://github.com/hwuu/dictyy/releases) 页面的 `Dictyy_x.x.x_x64-setup.exe`，运行安装即可。

### 配置 LLM

首次启动后，编辑配置文件：

```
%LOCALAPPDATA%\Dictyy\config.yaml
```

即 `C:\Users\<用户名>\AppData\Local\Dictyy\config.yaml`：

```yaml
llm:
  api_base: "https://api.example.com/v1"
  api_key: "your-api-key"
  model: "model-name"
  temperature: 0.3
  max_tokens: 2048
  timeout: 30
```

支持任何 OpenAI 兼容的 API。

### 日志

日志文件位于 `%LOCALAPPDATA%\Dictyy\debug.log`。

## 开发

### 前置要求

- Node.js 18+
- Rust 1.70+
- Windows 11

### 安装依赖

```bash
npm install
```

### 开发模式配置

创建 `src-tauri/config.yaml`（开发模式优先使用此配置）：

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

输出：`src-tauri/target/release/bundle/nsis/Dictyy_x.x.x_x64-setup.exe`

## 技术栈

- **前端**: React 19, TypeScript, Tailwind CSS, shadcn/ui
- **后端**: Tauri v2, Rust, SQLite
- **LLM**: OpenAI 兼容 API

## 快捷键

| 快捷键 | 功能 |
|--------|------|
| Ctrl+` | 显示/隐藏窗口 |
| ↑/↓ | 选择搜索建议 |
| Tab | 补全选中建议 |
| Enter | 查询 |

## License

MIT
