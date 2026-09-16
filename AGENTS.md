# Flux Launcher Agent 指南

## 1. 项目定位与核心约束

Flux Launcher 是一款使用 Rust 编写的 Windows 11 原生启动器。

### GUI 与渲染

* GUI **只能使用内嵌 `windui` 框架**。
* 禁止引入 WebView、Electron、egui、iced、Tauri 或其他 GUI 框架。
* 应用为常驻系统托盘程序。
* 窗口背景必须通过既有 Win32 + DirectComposition 路径使用真实 Windows DWM Acrylic 或 Mica。
* 必须保持启动器整个表面透明，使系统材质覆盖完整窗口。
* 不得以虚假渐变、不透明卡片、着色渐变或 WCA AccentPolicy 替代系统材质。

### 架构

* 保留 `flux-core`、Flux 应用代码与内嵌 `windui` 后端之间的职责边界。
* 优先进行小范围、平台相关的修改，避免无必要的大规模重写。
* Everything 集成必须保留优雅的不可用回落路径。
* Windows 专属代码应位于合适的平台模块中。
* 在可行时保持非 Windows 目标的跨平台编译能力及现有依赖锁定。

---

## 2. 仓库语言与提交规范

### 源码、文档与发布说明

保留已有内容的原有语言，不做无关翻译或重写。

后续新增或修改内容默认使用中文，包括：

* 源码注释
* 项目文档
* 修复 / 功能文档
* 发布说明

如需英文版本，可使用 `_en` 后缀单独提供，例如：

```text
release-notes.md
release-notes_en.md
```

除明确要求外，不强制已有内容提供英文版本。

### Git Commit

遵循 **Conventional Commits 1.0.0**；提交信息使用英文和祈使式描述。

格式：

```text
<type>(launcher): <description>
```

---

## 3. 国际化（i18n）

Flux 使用 `rust-i18n` 处理所有面向用户的 UI 文案。

翻译文件：

```text
crates/flux-launcher/locales/en.yml
crates/flux-launcher/locales/zh-CN.yml
```

其中 `en.yml` 为回退语言。

### 规则

* 所有用户可见 UI 文案必须通过 `t!` 获取，禁止在 UI 代码中硬编码。
* 新增或修改文案时，必须同步更新 `en.yml` 和 `zh-CN.yml`，两者键保持一致。
* 缺失或不支持的翻译统一回退到英文，不得显示原始键名或 panic。
* 启动时使用 `sys-locale` 检测系统语言，并通过 `rust_i18n::set_locale` 设置；未知语言回退到 `en`。
* 保持现有 `i18n!("locales", fallback = "en")` 路径不变，除非同步调整相关配置。
* 版本号、内部标识符、颜色、常量及 stderr 诊断信息无需翻译。

---

## 4. 工作方式

* 修改前先检查相关源码、仓库规则、CI、测试、发布记录、安装器、Windows 验证及问题报告；已有日志能回答的问题不得猜测。
* 先确认问题，再做**最小完整修改**，保持现有架构，避免无关重构。
* 多步骤任务先制定涵盖“调研 → 实现 → 验证 → 交付”的计划。
* 在 CI 等待 / 失败、外部评审或 Windows 验证未完成等关键节点及时汇报。
* 本地检查及适用的 Windows 验证通过前，不得声称修复已完成或可发布。

---

## 5. 工具失败与异常记录

* 工具出现失败、超时、异常返回或部分状态变更时，及时在 `doc/question/` 记录中文 Markdown 报告，包含时间、工具 / 服务、操作、影响、原始错误、诊断与恢复、验证结果。
* `apply_patch` 已禁用，禁止调用。

---

## 6. 行尾与格式化

* Rust 使用仓库现有 `rustfmt.toml`，格式或行尾问题统一执行 `cargo fmt --all`，禁止手工修改 Rust 行尾。
* 非 Rust 文件如发现 CRLF、行尾损坏或仅因行尾产生的 diff，停止自动编辑，人工规范为 LF，并通过字节级检查和 `git diff --check` 验证工作区与暂存区。
* 除非人工处理不可行，否则禁止使用脚本批量重写行尾。
* 仅执行 `git add` 不视为完成修复。

---

## 7. 文档规则

* `doc/` 下文档使用中文。
* 产品修复后，在 `doc/fix/` 生成对应修复报告，至少包含：实现说明、验证结果、已知限制、用户验证步骤。
* 新增功能使用独立分类，如 `doc/feat/`，目录不存在时创建。
* `doc/` 及其子目录 默认仅作本地记录，不加入暂存区或提交，除非项目负责人明确要求。

---

## 8. Windows 生命周期与启动行为

* 默认全局热键为 `Alt+Space`，必须保持可配置。
* 重复激活切换启动器可见性；显示后搜索框立即获得焦点。
* “激活时清空查询”、游戏模式保护、全屏热键保护默认启用。
* 应用程序结果优先于普通文件和文件夹。
* 保持既有 Flow 键盘导航：`Up`、`Down`、`Home`、`End`、`Enter`、`Right`、`Left`、`Escape`。

---

## 9. Windows 启动项与安装器启动行为

* 安装器的“Windows 启动时启动”和 `Launch Flux Launcher now` 必须独立，前者默认启用且可取消，后者默认选中。
* 启动注册表命令必须使用 `--startup`。
* `--startup` 必须调用 `windui::start_hidden()`，仅创建托盘进程，不显示搜索窗口；仅全局热键或托盘 `Show launcher` 可显示窗口。
* 安装器烟雾测试必须覆盖默认启动项、`/TASKS=!startup` 禁用路径及隐藏 `--startup` 模式。

---

## 10. 开始菜单快捷方式与图标

* 开始菜单快捷方式必须指向实际安装的可执行文件，并显式引用 Flux Launcher 的 `.ico`。
* 安装目录必须包含多分辨率 `.ico` 资源。
* 安装器烟雾测试必须验证快捷方式目标、图标元数据及实际图标引用，不得仅检查快捷方式文件存在。

---

## 11. Windows Acrylic / Mica 与生命周期不变量

* `ShowWindow` 必须先于 show 回调中的布局状态修改。
* 可见激活后，首个透明 D2D 帧必须先失效并呈现，再依赖输入或查询状态。
* 反复隐藏 / 显示及窗口尺寸、绘制、合成变化后，Acrylic / Mica 必须保持正常。
* 发布前验证暗 / 亮色可读性、标题与结果行无重叠、选中状态清晰响应，以及系统强调色和自定义调色板回退正常。

---

## 12. 查询历史与 UX 不变量

* 已提交查询持久化到有界、不区分大小写的历史记录，按最新优先。
* `Ctrl+H` 打开历史；`Enter` 或鼠标点击重新运行所选查询。
* 空查询按普通 `Up` 回溯最近查询；`Alt+Up` / `Alt+Down` 前后循环。
* 设置中提供清空历史功能。
* 展开操作栏中始终显示 Provider 状态，不得破坏 Acrylic / Mica，也不得使用不透明容器替代。

---

## 13. Everything 集成

* 安装后始终注册 Everything 文件 / 文件夹 provider；服务不可用时优雅回落为无服务状态。
* 保持现有原生语法支持，如 `ext:zip`、`parent:`、`file:`、`dm:`。
* Everything 不可用时必须保持启动器正常运行，不得崩溃。
* 应用程序结果始终优先于普通 Everything 文件 / 文件夹结果。

---

## 14. Flow 插件与内置能力

* Flow 插件仅支持原生插件和可执行 JSON-RPC 插件。
* 禁止 Python、C# 插件执行。
* Google、Obsidian 必须保留在主可执行文件中。
* 除非用户明确要求独立社区插件宿主，否则不得拆分上述内置能力。

---

## 15. 提交前验证

提交或报告任务完成前执行适用检查：

```bash
source "$HOME/.cargo/env"
cargo fmt --all
cargo fmt --all -- --check
git diff --check
cargo check --workspace --target x86_64-pc-windows-gnu
cargo clippy -p flux-core -p flux-launcher --all-targets --target x86_64-pc-windows-gnu -- -D warnings
cargo test -p flux-core
```

涉及 `windui`、发布打包或安装器时，必须执行对应额外检查。

任何检查失败都必须先分析日志并判断原因；仅在确认属于环境或瞬时问题后才可重跑，不能因“看起来无关”而忽略失败。

---

## 16. Windows 发布验证

涉及生命周期、视觉、安装器、启动项、快捷方式、Acrylic / Mica 或启动行为时，必须手工运行 `Windows 发布` workflow，并使用 `release_channel=beta`：

```bash
gh workflow run windows-release.yml \
  --repo lvxingqi/flux-launcher-cn \
  --ref main \
  -f release_tag=vX.Y.Z \
  -f runner_label=windows-latest \
  -f release_channel=beta

`release_tag` 必须对应本次实际构建版本，且不得复用已有 release tag。

工作流成功前，不得发布或报告 beta 已完成。

工作流必须包含：

* 安装程序与便携版构建
* Windows UI 捕获
* `scripts/installer-smoke.ps1`

安装器烟雾测试必须覆盖第 9、10 节规定的启动项、隐藏启动、快捷方式、图标，以及可执行文件哈希、安装后配置和卸载清理。

---

## 17. Windows 视觉烟雾测试

* 可用时使用已配置的副显示器，覆盖：空启动、查询前捕获、反复隐藏 / 显示、查询扩展、键盘选择、操作模式、Enter、设置。
* GitHub Windows runner 截图仅作为渲染路径证据，不能替代或推翻实体 Windows 11 的 DWM Acrylic / Mica 观察。
* 发布说明必须如实注明该限制。

---

## 18. Beta 发布

* 每次产品修复完成后准备一次**手工 beta 发布**。
* Beta 必须通过 `Windows 发布` workflow 的 `release_channel=beta` 生成；发布必须 `prerelease: true`，名称不得包含 `(beta)`。
* 禁止空、重复或无说明发布，也不得由 push、定时任务或内部提交自动创建。
* 发布版本、通道和说明必须由 Agent 主动选择并核对。
* 发布前确认版本、`Cargo.lock`、安装器版本、tag、产物元数据一致，且安装程序与便携版资产均存在。
* 发布说明须使用中文并由人工撰写或校对，包含变更摘要、实际验证、已知限制、runner / DWM 限制、Windows 验证步骤及安装程序 / 便携版直接下载链接。
* 下载优先级：安装程序 > 便携版。

---

## 19. 稳定发布与 WinGet

* 稳定发布必须通过明确的用户指令，并手动运行 `Windows UI 发布` workflow 的 `release_channel=stable`。
* WinGet 仅提交稳定版本，包标识符为 `m1nuzz.FluxLauncher`，路径为 `manifests/m/m1nuzz/FluxLauncher/<version>/`。
* 提交前根据实际安装程序核实 URL、SHA256、schema、安装器元数据及“应用与功能”名称。
* Beta 不得进入 WinGet；WinGet 自动化不得创建 GitHub Release，且仅在明确启用稳定版策略后才能准备或提交稳定版 PR。
* 未经用户明确要求，不得创建 `WINGET_GITHUB_TOKEN` 或签名密钥。
* 手工 WinGet PR 不需要仓库密钥，且必须与 Beta 发布流程分离。

---

## 20. 完成标准

任务仅在以下条件满足后才能报告为完成：

* 已检查相关代码、CI、发布历史和测试，并完成最小完整修改。
* 必要的 i18n、文档及修复报告已生成或更新。
* 适用的本地检查与 Windows 验证全部通过。
* 涉及生命周期、视觉、安装器或启动行为时，已完成对应 Windows 验证。
* 对属于产品发布范围的修复，已准备对应 Beta 发布，并核实发布元数据、资产和说明。
* 已知限制已如实记录。

在条件满足前，不得声称已完成、已验证、可发布或可交付。

---

## 21. 优先级原则

规则冲突时按以下优先级处理：

1. Windows 生命周期与安全
2. 用户行为与 UX
3. i18n 与架构
4. 验证与正确性
5. 最小修改
6. 发布与交付
7. 文档与提交

不得为了通过检查而绕过 Windows 生命周期、真实 Acrylic / Mica、i18n、Everything 回落、启动语义、安装器烟雾测试或必要的 Windows 验证。
