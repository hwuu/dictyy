# Dictyy - 轻量级 LLM 字典

## 概述

一个运行在 Windows 11 上的轻量级字典应用。常驻系统托盘，通过全局快捷键快速唤出，输入单词后查询多个词典数据源，未找到时调用大模型获取释义。

## 技术栈

| 组件 | 选型 | 说明 |
|------|------|------|
| 框架 | Tauri v2 | Rust + Web 前端，轻量级桌面应用框架 |
| 前端 | React 19 + TypeScript | 现代化前端开发体验 |
| 构建 | Vite | 快速开发和构建 |
| 样式 | Tailwind CSS v4 | 原生 CSS 支持，无需 PostCSS |
| 组件库 | shadcn/ui | 基于 Radix UI 的高质量组件 |
| 状态管理 | Jotai | 轻量级原子状态管理 |
| LLM | OpenAI 兼容 API | 前端直接调用 |
| 数据库 | SQLite | 存储词典数据 |

> 参考项目：[Aictionary](C:\Users\hwuu\dev\github\ahpxex\Aictionary)

### 技术选型说明

| 组件 | 为什么选它 | 替代方案 |
|------|-----------|---------|
| Vite | 启动快、Tauri 官方推荐 | Webpack（慢）、Parcel |
| Tailwind CSS | 原子化快速开发、shadcn/ui 依赖 | CSS Modules、styled-components |
| shadcn/ui | 代码可控、设计精美、轻量 | Ant Design（重）、MUI |
| Jotai | 极轻量(3KB)、API 简单 | Zustand、Redux Toolkit（重） |

## 核心功能

### 1. 系统托盘
- 应用启动后最小化到系统托盘
- 托盘图标右键菜单：显示窗口、设置、退出
- 关闭窗口时隐藏到托盘而非退出

### 2. 全局快捷键
- 默认快捷键：`Ctrl+\``（可配置，避免与系统快捷键冲突）
- 按下快捷键：显示/隐藏查询窗口
- 窗口显示时自动聚焦输入框

### 3. 查询窗口
- 极简设计：一个输入框 + 结果展示区
- 输入单词后按 Enter 或点击按钮查询
- 支持流式输出（streaming）显示 LLM 响应
- **窗口动画**：弹出时淡入 + 滑入效果
- **动态尺寸**：宽度为屏幕的 2/3，高度为屏幕的 3/4，居中显示

### 4. 查询逻辑（多数据源架构）

```
User Input
    |
    v
+-------------------+
| Query all sources |
| in parallel       |
+-------------------+
    |
    +---> Main Dict (有道)
    +---> Collins Dict
    +---> Etyma Dict
    +---> GPT4 Cache
    |
    v
+-------------------+
| Any result found? |
| Yes -> Show tabs  |
| No  -> Query LLM  |
+-------------------+
    |
    v
Display result + Cache
```

### 5. 离线词典

#### 数据来源

| 词典 | 来源 | 说明 |
|------|------|------|
| 主词典 | [kajweb/dict](https://github.com/kajweb/dict) | 有道词典数据，含音标、释义、例句、近义词 |
| 柯林斯 | 柯林斯英汉双解词典 MDX | 权威释义，含词频、词性、例句 |
| 词根词缀 | 英语词根词缀词频 MDX | 词源解析、相关词汇 |
| GPT4 缓存 | 自建 | 历史 LLM 查询结果缓存 |

### 6. LLM 集成
- 调用 OpenAI 兼容 API（支持自定义 endpoint）
- 前端直接调用（`dangerouslyAllowBrowser: true`）
- 配置项：API Key、Base URL、Model
- Prompt 设计：针对单词释义优化，结构化输出
- 仅在所有离线词典都未找到时触发

### 7. 搜索建议
- 输入时实时显示搜索建议
- 查询所有词典（主词典、柯林斯、词根词缀）
- 显示词条简要信息
- 键盘导航（上下箭头、Tab 补全、Enter 选择）

### 8. 多语言支持
- 英→中（默认）
- 中→英
- 自动检测输入语言
- 后期可扩展其他语言

### 9. 屏幕取词

用户在任意应用中选中文本后，自动弹出气泡显示释义。

#### 实现原理

采用 UI Automation API 轮询方案，直接读取选中文本，无需模拟键盘操作：

```
Polling Loop (200ms interval)
    |
    v
+-----------------------------+
| Screen capture enabled?     |--No--> Continue loop
+-----------------------------+
    |Yes
    v
+-----------------------------+
| GetFocusedElement()         |
| GetTextPattern()            |
| GetSelection()              |
+-----------------------------+
    |
    v
+-----------------------------+
| Valid word?                 |--No--> Close bubble
| (1-50 chars, English only)  |        Reset state
+-----------------------------+
    |Yes
    v
+-----------------------------+
| Text changed?               |--Yes--> Reset timer
+-----------------------------+
    |No
    v
+-----------------------------+
| Stable for 500ms?           |--No--> Continue loop
+-----------------------------+
    |Yes
    v
+-----------------------------+
| Query word_abstracts        |
| (in-memory HashMap)         |
+-----------------------------+
    |
    v
+-----------------------------+
| Show bubble at text bounds  |
+-----------------------------+
```

#### 气泡窗口

```
+---------------------------+
|  word  /phonetic/         |
|  n. definition...         |
|                 [详细 ->] |
+---------------------------+

Size: 320 x 150 (logical pixels)
Style: No border, rounded corners, shadow, always on top, transparent bg
Position: Near selected text (auto-adjusted to stay within screen bounds)
Dismiss: Text selection cleared or changed
Link: Click [详细] -> Open main window with word query
```

#### 词形还原

气泡查询支持词形还原，自动将变形词还原为原形：

| 输入 | 还原为 |
|------|--------|
| resources | resource |
| removed | remove |
| walking | walk |
| studies | study |

#### 开关

- 托盘菜单「屏幕取词」开关
- 默认开启

## 架构

```
+-----------------------------------------------------------+
|  Tauri App                                                 |
|                                                            |
|  +------------------------------------------------------+  |
|  |  Frontend (React + TypeScript)                       |  |
|  |                                                      |  |
|  |  +-------------+  +---------------+  +------------+  |  |
|  |  | SearchInput |  | Tab Results   |  | Bubble     |  |  |
|  |  +-------------+  +---------------+  +------------+  |  |
|  |                   | - Main Dict   |                  |  |
|  |                   | - Collins     |                  |  |
|  |                   | - Etyma       |                  |  |
|  |                   | - GPT4 Cache  |                  |  |
|  |                   | - LLM         |                  |  |
|  |                   +---------------+                  |  |
|  |                                                      |  |
|  |  +------------------------------------------------+  |  |
|  |  |  Services                                      |  |  |
|  |  |  - llm.ts        (LLM API call)               |  |  |
|  |  |  - dictionary.ts (Dictionary lookup)          |  |  |
|  |  +------------------------------------------------+  |  |
|  +------------------------------------------------------+  |
|                                                            |
|  +------------------------------------------------------+  |
|  |  Backend (Rust)                                      |  |
|  |  - tray.rs           System tray                     |  |
|  |  - shortcuts.rs      Global shortcuts                |  |
|  |  - dictionary.rs     SQLite dictionary queries       |  |
|  |  - llm.rs            LLM config management           |  |
|  |  - screen_capture.rs Screen word capture             |  |
|  +------------------------------------------------------+  |
|                                                            |
|  +------------------------------------------------------+  |
|  |  Data (SQLite: dict.db)                              |  |
|  |  - words         Main dictionary                     |  |
|  |  - collins_words Collins dictionary                  |  |
|  |  - etyma_words   Etymology dictionary                |  |
|  |  - gpt4_words    GPT4 cache                          |  |
|  +------------------------------------------------------+  |
+-----------------------------------------------------------+
```

## 窗口设计

```
+--------------------------------------+
|  +----------------------------+  X   |  <- Caption bar (draggable)
|  | Input word...              |      |
|  +----------------------------+      |
+--------------------------------------+
| [Main] [Collins] [Etyma] [GPT4]      |  <- Tab switcher
+--------------------------------------+
|                                      |
|  ephemeral                     [VOL] |
|  /ifem(e)rel/                        |
|                                      |
|  adj. Short-lived; transient         |
|                                      |
|  Examples:                           |
|  Fame is ephemeral.                  |
|  ...                                 |
|                                      |
+--------------------------------------+
| Ctrl+\` Hide              v0.3.0     |  <- Status bar
+--------------------------------------+

Size: 2/3 screen width x 3/4 screen height
Position: Center of screen
Style: No border, rounded corners, shadow
Animation: Fade in (150ms) + slide up (8px)
```

## 数据库结构

SQLite 数据库 `dict.db` 包含以下表：

### words 表（主词典）
```sql
CREATE TABLE words (
    id INTEGER PRIMARY KEY,
    word TEXT NOT NULL,
    content TEXT NOT NULL  -- JSON 格式的词条内容
);
```

### collins_words 表（柯林斯词典）
```sql
CREATE TABLE collins_words (
    id INTEGER PRIMARY KEY,
    word TEXT NOT NULL,
    content TEXT NOT NULL,  -- JSON 格式
    is_link INTEGER DEFAULT 0,
    link_target TEXT
);
```

### etyma_words 表（词根词缀词典）
```sql
CREATE TABLE etyma_words (
    id INTEGER PRIMARY KEY,
    word TEXT NOT NULL,
    content TEXT NOT NULL,  -- JSON 格式
    is_link INTEGER DEFAULT 0,
    link_target TEXT
);
```

### gpt4_words 表（GPT4 缓存）
```sql
CREATE TABLE gpt4_words (
    id INTEGER PRIMARY KEY,
    word TEXT NOT NULL UNIQUE,
    content TEXT NOT NULL  -- Markdown 格式
);
```

### word_abstracts 表（单词摘要）
```sql
CREATE TABLE word_abstracts (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    word TEXT NOT NULL UNIQUE,
    phonetic TEXT,
    main_def TEXT,
    collins_def TEXT,
    etyma_def TEXT,
    gpt4_def TEXT
);
CREATE INDEX idx_word_abstracts_word ON word_abstracts(word);
```

此表用于气泡窗口的快速查询，启动时加载到内存（约 5 万条）。

## 配置项

配置文件 `config.yaml` 存储在用户数据目录：
- Windows: `%LOCALAPPDATA%\Dictyy\config.yaml`

```yaml
llm:
  api_key: "sk-xxx"
  api_base: "https://api.openai.com/v1"
  model: "gpt-4o-mini"
```

## 目录结构

```
dictyy/
+-- docs/
|   +-- design.md
|   +-- release_note_*.md
+-- scripts/
|   +-- import_mdx.py         # MDX dictionary importer
|   +-- generate_abstracts.py # Word abstracts generator
|   +-- sync-version.cjs      # Version sync script
+-- src/                      # Frontend
|   +-- components/
|   |   +-- ui/               # shadcn/ui components
|   |   +-- WordResult.tsx
|   |   +-- CollinsResult.tsx
|   |   +-- EtymaResult.tsx
|   |   +-- Gpt4Result.tsx
|   |   +-- SearchSuggestions.tsx
|   +-- hooks/
|   |   +-- useDebounce.ts
|   +-- services/
|   |   +-- dictionary.ts     # Dictionary API
|   +-- App.tsx               # Main window
|   +-- Bubble.tsx            # Bubble window
|   +-- main.tsx
|   +-- index.css
+-- src-tauri/                # Rust backend
|   +-- src/
|   |   +-- main.rs
|   |   +-- lib.rs
|   |   +-- tray.rs           # System tray
|   |   +-- shortcuts.rs      # Global shortcuts
|   |   +-- dictionary.rs     # SQLite queries
|   |   +-- llm.rs            # LLM config
|   |   +-- screen_capture.rs # Screen word capture
|   +-- resources/
|   |   +-- dict.db           # SQLite dictionary
|   +-- Cargo.toml
|   +-- tauri.conf.json
+-- package.json
+-- VERSION
+-- CLAUDE.md
```

## Tauri 插件

```toml
# Cargo.toml
[dependencies]
tauri-plugin-store = "2"           # Config persistence
tauri-plugin-global-shortcut = "2" # Global shortcuts
tauri-plugin-single-instance = "2" # Single instance
tauri-plugin-clipboard-manager = "2" # Clipboard

# Windows API for screen capture
windows = { version = "0.58", features = [
    "Win32_UI_WindowsAndMessaging",
    "Win32_Foundation",
    "Win32_UI_Input_KeyboardAndMouse"
]}
```

## 功能确认

- [x] 系统托盘 + 全局快捷键
- [x] 多词典查询（主词典、柯林斯、词根词缀、GPT4 缓存）
- [x] Tab 切换多数据源结果
- [x] 搜索建议（查询所有词典）
- [x] LLM fallback（所有词典未找到时）
- [x] 窗口弹出动画
- [x] 动态窗口尺寸（2/3 x 3/4 屏幕）
- [x] 窗口关闭改为隐藏到托盘
- [x] 屏幕取词（选中文本自动弹出气泡）
- [x] 词形还原（复数、过去式等变形词自动还原）
- [x] 日志持久化
