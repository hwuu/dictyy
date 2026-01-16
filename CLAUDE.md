1. 如果我要求先讨论方案时不要着急修改代码，直到方案确定才可以修改代码。

2. 方案讨论需要在我们双方都没疑问的情况下才可以输出具体方案文档。

3. 方案评估请主动思考需求边界，合理质疑当下方案的完善性，方案需包含：重要逻辑的实现思路、需求按技术实现的依赖关系拆解并排序，便于后续渐进式开发、输出修改或新增文件的路径、输出测试要点利于需求完成后的自动化测试。

4. 方案讨论或代码编写时，如果遇到了争议或不确定性请主动告知我，请牢记让我决策而不是默认采用一种方案实现，重点强调。

5. 开发项目必须严格按步骤执行，每次只专注当前讨论的步骤，要求：不允许跨步骤实现功能或"顺便"完成其他步骤任务、实现前必须先确认技术方案和实现细节、每个步骤完成后必须明确汇报，等待 Review 确认后才能进入下一步。

6. 进行代码提交时，请先给我梳理提交的内容，等待 Review 确认后才能进行提交。

7. 与第五,六点类似，任何代码修改请始终遵守最小改动原则，除非我主动要求优化或者重构。

8. 代码实现请先思考哪些业务可以参考或复用，尽可能参考现有业务的实现风格，如果你不明确可让我为你提供，避免重复造轮子。

9. 不要在源码中插入mock的硬编码数据。

10. 同步更新相关文档。

11. 使用TDD开发模式开发。

12. 小步快跑，对一步都进行测试，并保证不影响现有用例。

13. 使用中文回答

14. 记得每次测试完后，清理下测试文件。

15. 在 bug 修复时如果超过 2 次修复失败，请主动添加关键日志后再进行尝试修复，在我反馈修复后主动清除之前的日志信息。

16. 项目中的重试过2次以上环境配置问题或其他重复犯错的问题，请在项目的CLAUDE.md中做记录。常用的命令，请记录在项目的CLAUDE.md中。

17. 进行了比较重要的修改之后，更新 docs/design.md

18. 及时在 tests/unit 中添加单元测试

19. 文档中如果要画流程框图，那么框图中的文字用英文，框线要对齐；其余内容保持中文

---

## 技术架构

### 模块结构

**screen_capture** - 屏幕取词核心模块
- UI Automation API 轮询（200ms 间隔）
- Clipboard Fallback 策略（黑名单机制）
- 气泡显示逻辑（文本稳定 500ms 后触发）
- 焦点检测和变化处理

**dictionary** - 词典查询模块
- 主词典、柯林斯、词根词缀、GPT4 并行查询
- 词形还原（lemmatization）支持复数、过去式、进行时等变形
- 词典摘要预加载（启动时加载约 5 万条摘要到内存）

**llm** - LLM 查询模块
- OpenAI 兼容 API
- 词典未收录时自动回退
- 支持自定义 API base、model、temperature 等参数

**shortcuts** - 全局快捷键模块
- Ctrl+` 智能切换窗口
- 支持选中文本自动查询（TextPattern + Clipboard Fallback）
- 焦点检测避免获取自身窗口文本

### 关键设计决策

**为什么用轮询而不是钩子？**
- Windows 全局钩子需要管理员权限
- 轮询方案更轻量，不需要特殊权限
- 200ms 间隔在性能和响应速度间取得平衡

**为什么气泡稳定时间是 500ms？**
- 太短（<300ms）：选择过程中频繁弹出气泡，干扰用户
- 太长（>1s）：响应迟钝，用户体验差
- 500ms 是平衡响应速度和避免误触发的最佳值

**为什么用黑名单而不是白名单？**
- 简化逻辑，覆盖更多应用（默认启用 fallback）
- 只需维护少数会被干扰的应用列表（Terminal 等）
- 新应用无需配置即可工作

**为什么 Clipboard Fallback 用 Ctrl+Insert 而不是 Ctrl+C？**
- Ctrl+C 在 Terminal 中是中断信号（SIGINT），会干扰正在运行的程序
- Ctrl+Insert 是 Windows 的标准复制快捷键，更安全

---

## 技术约束

**平台限制**：
- Windows Only：依赖 Windows UI Automation API
- 不支持管理员权限应用：无法在以管理员身份运行的应用中获取选中文本
- DPI 感知：需要正确处理不同缩放因子的屏幕

**Clipboard 副作用**：
- Ctrl+Insert 会修改剪贴板内容
- 必须先保存、后恢复剪贴板内容
- 可能与用户的剪贴板操作产生竞态条件（通过 1 秒冷却时间缓解）

**性能考虑**：
- 词典数据库约 50MB
- 启动时加载摘要到内存（约 5 万条，10-20MB RAM）
- 轮询线程 CPU 占用 <1%

**Tauri v2 权限模型**：
- 前端调用 Window API 需要在 `capabilities/default.json` 中显式授权
- 每个新的 Window API 都需要添加对应的权限

---

## 注意事项 ⚠️

### 不要做

- ❌ **不要在 Terminal 中使用 Ctrl+Insert fallback** - 会被误解为粘贴命令，干扰用户操作
- ❌ **不要在 TextPattern 可用时使用 clipboard fallback** - 性能差且有副作用
- ❌ **不要在焦点变化时立即触发气泡** - 会在窗口切换时频繁弹出
- ❌ **不要使用 `git commit --amend`** - 除非符合严格条件（见工作流程规范第 6 点）
- ❌ **不要修改版本号后直接 build** - 构建缓存可能导致版本号错误，需先 `cargo clean`
- ❌ **不要跨步骤实现功能** - 严格按照工作流程规范第 5 点执行

### 必须做

- ✅ **修改代码后运行测试** - 确保不影响现有功能
- ✅ **提交前等待 Review** - 给出提交内容梳理，等待确认
- ✅ **遇到重复问题记录到 CLAUDE.md** - 避免重复犯错
- ✅ **重要修改后更新 docs/design.md** - 保持文档同步
- ✅ **添加单元测试到 tests/unit** - 及时补充测试覆盖

---

## 测试清单

### 屏幕取词功能测试

**基础功能**：
- [ ] VS Code 中选中文本，气泡在 500ms 后显示
- [ ] Chrome 网页选中文本，气泡显示
- [ ] 知乎网页选中文本，气泡显示（clipboard fallback）
- [ ] Edge PDF 选中文本，气泡显示（clipboard fallback）
- [ ] 选中无效文本（非英文），气泡不显示
- [ ] 取消选中，气泡关闭

**边界情况**：
- [ ] Terminal 中选中文本，气泡不显示（黑名单）
- [ ] PowerShell 中选中文本，气泡不显示（黑名单）
- [ ] 快速切换窗口，不频繁弹出气泡（焦点变化时不触发）
- [ ] 窗口切换后停留在已选中文本的窗口，约 700ms 后显示气泡

**性能测试**：
- [ ] 气泡查询响应时间 <50ms（预加载摘要）
- [ ] 轮询 CPU 占用 <1%
- [ ] 长时间运行无内存泄漏

### 快捷键功能测试

**Ctrl+` 快捷键**：
- [ ] 窗口不可见 + 选中文本 → 显示窗口并查询
- [ ] 窗口不可见 + 无选中文本 → 显示窗口并聚焦输入框
- [ ] 窗口可见 + 选中文本 → 查询新单词（窗口保持显示）
- [ ] 窗口可见 + 无选中文本 → 隐藏窗口
- [ ] 焦点在主窗口 + Ctrl+` → 隐藏窗口（不获取自身窗口文本）

**TextPattern + Clipboard Fallback**：
- [ ] VS Code 选中文本 + Ctrl+` → 查询（TextPattern）
- [ ] 知乎网页选中文本 + Ctrl+` → 查询（Clipboard Fallback）
- [ ] Terminal 选中文本 + Ctrl+` → 不查询（黑名单）

### 词典查询功能测试

**基础查询**：
- [ ] 查询已收录单词，显示主词典结果
- [ ] 查询未收录单词，自动回退到 LLM
- [ ] 输入拼写错误，显示相似单词建议
- [ ] 输入词形变化（如 resources），自动还原并查询（resource）

**并行查询**：
- [ ] 主词典、柯林斯、词根词缀、GPT4 同时查询
- [ ] 只显示有内容的 Tab
- [ ] 自动切换到第一个有内容的 Tab

### 构建和发布测试

**版本号同步**：
- [ ] 更新 VERSION 文件
- [ ] 运行 `npm run build` 后，package.json、Cargo.toml、tauri.conf.json 版本号一致
- [ ] `cargo clean` 后构建，生成的 exe 文件名包含正确版本号

---

## 性能优化记录

### v0.3.0 (2024-01)
- **词典摘要预加载**：启动时加载 5 万条摘要到内存
  - 气泡查询从 ~200ms 降到 <10ms
  - 内存占用增加约 10-20MB
  - 启动时间增加约 100-200ms（可接受）

### v0.3.1 (2026-01)
- **空结果冷却时间优化**：5秒 → 1.5秒
  - 减少气泡延迟显示问题
  - 仍能避免频繁重试
- **焦点变化后文本检测优化**：稳定后开始计时
  - 焦点变化时不立即触发，但稳定后会显示
  - 避免窗口切换时频繁弹出，同时不会永远不显示

---

## 常见问题记录

### 屏幕取词 Clipboard Fallback 策略

**问题**：某些应用（如知乎网页、Edge PDF）不支持 UI Automation TextPattern，需要使用 Ctrl+Insert 作为 fallback。

**解决方案**：
- 对所有不支持 TextPattern 的应用默认使用 Ctrl+Insert fallback
- 黑名单排除：Terminal、PowerShell、CMD、桌面、任务栏等（Ctrl+Insert 可能干扰这些应用）

**实现细节**：
- `should_skip_clipboard_fallback()` 函数检查应用是否在黑名单中
- 通过进程名和元素名判断（如 `terminal`, `powershell`, `cmd` 等）

**影响模块**：`src-tauri/src/screen_capture.rs`

---

### Tauri v2 前端 Window API 权限问题

**问题**：Tauri v2 前端调用 `window.hide()`、`window.show()` 等 API 不生效，Esc 键无法隐藏窗口。

**原因**：Tauri v2 采用细粒度权限模型，前端调用 window API 需要在 `capabilities/default.json` 中显式授权。

**解决方案**：在 `src-tauri/capabilities/default.json` 添加所需权限：
```json
{
  "permissions": [
    "core:default",
    "opener:default",
    "core:window:allow-hide",
    "core:window:allow-show",
    "core:window:allow-set-focus"
  ]
}
```

**影响模块**：`src-tauri/capabilities/default.json`

---

### Windows 路径解析问题

**问题**：在 Windows 环境下，使用简单的 `split(':')` 解析包含文件路径的工具输出会失败。

**原因**：Windows 绝对路径包含盘符（如 `C:\`），冒号会干扰基于冒号的分割逻辑。

**示例**：
```
mypy 输出: C:\Users\hwuu\file.py:3:12: error: Message
简单分割: ['C', '\Users\hwuu\file.py', '3', '12', ' error: Message']  # 错误！
```

**解决方案**：使用正则表达式匹配关键信息，绕开路径部分：
```python
import re
match = re.search(r':(\d+):(\d+):\s*(\w+):\s*(.+)', line)
if match:
    line_num = int(match.group(1))
    col_num = int(match.group(2))
    severity = match.group(3)
    message = match.group(4)
```

**影响模块**：`pyscan/layer1/mypy_analyzer.py`

---

### Windows Subprocess 编码问题

**问题**：在 Windows 环境下使用 `subprocess.run` 读取 git 命令输出时，遇到 `UnicodeDecodeError: 'gbk' codec can't decode byte...`

**原因**：
- Windows 系统默认使用 GBK 编码
- `subprocess.run` 在 `text=True` 时会使用系统默认编码
- Git 输出（如 git blame）可能包含 UTF-8 编码的内容（中文作者名、中文注释等）
- GBK 无法正确解码某些 UTF-8 字节序列

**错误示例**：
```
UnicodeDecodeError: 'gbk' codec can't decode byte 0xa6 in position 20560: illegal multibyte sequence
```

**解决方案**：在所有 `subprocess.run` 调用中明确指定 UTF-8 编码和错误处理：
```python
result = subprocess.run(
    ['git', 'blame', '--line-porcelain', file_path],
    capture_output=True,
    text=True,
    encoding='utf-8',        # 明确指定 UTF-8 编码
    errors='replace',        # 遇到无法解码的字符时替换而不是报错
    check=True,
    timeout=30
)
```

**影响模块**：`pyscan_viz/git_analyzer.py`

**关键点**：
- 所有调用 git 命令的地方都需要指定 `encoding='utf-8'`
- 使用 `errors='replace'` 确保即使遇到特殊字符也不会崩溃
- 包括：`git rev-parse`、`git remote get-url`、`git blame` 等

---

### Dictyy 配置文件路径

**生产环境配置路径**：
- Windows: `%LOCALAPPDATA%\Dictyy\config.yaml` (即 `C:\Users\<用户名>\AppData\Local\Dictyy\config.yaml`)

**开发环境配置路径**：
- `src-tauri/config.yaml` (优先使用，如存在)

**日志文件路径**：
- Windows: `%LOCALAPPDATA%\Dictyy\logs\Dictyy.log` (即 `C:\Users\<用户名>\AppData\Local\Dictyy\logs\Dictyy.log`)
- 使用 `tauri-plugin-log` 自动写入
- 开发模式：DEBUG 级别，生产模式：INFO 级别

**说明**：首次启动时会自动创建配置目录和模板文件，用户需要手动编辑 API 配置。

---

## 常用命令

### Dictyy 开发命令
```bash
# 启动开发服务器
npm run tauri dev

# 构建生产版本
npm run tauri build

# 仅启动前端 (不启动 Tauri)
npm run dev
```

### 测试命令
```bash
# 运行所有测试
python -m pytest tests/ -v

# 运行特定测试文件
python -m pytest tests/test_layer1/test_mypy_analyzer.py -v

# 运行特定测试（显示输出）
python -m pytest tests/test_e2e_layer1.py::TestLayer1E2E::test_analyze_code_with_type_errors -v -s

# 运行测试并查看覆盖率
python -m pytest tests/ --cov=pyscan --cov-report=html
```

### 开发命令
```bash
# 安装依赖
pip install -r requirements.txt

# 运行 pyscan
python -m pyscan <目录> --config config.yaml
```
