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
* 非 Rust 文件如发现 CRLF、行尾损坏或仅因行尾产生的 diff，停止自动编辑，人工规范为 LF，并通过抽样十六进制检查与 `git diff --check` 验证工作区与暂存区。
* 除非人工处理不可行，否则禁止使用脚本批量重写行尾。
* 仅执行 `git add` 不视为完成修复。

---

## 7. 文档规则

* `doc/` 下文档使用中文。
* `doc/` 根目录不得直接存放文档，只能包含分类子目录；固定分类：
  * `doc/fix/`：产品修复报告，至少包含实现说明、验证结果、已知限制、用户验证步骤。
  * `doc/feat/`：新增功能说明与使用方式。
  * `doc/plan/`：Plan 模式产出计划与 Act 模式回写。
  * `doc/question/`：工具失败、异常与外部阻塞记录。
  * `doc/test/`：测试脚本说明、探针设计与验证记录。
  * `doc/guide/`：指南、项目概览与参考文档。
  * `doc/upstream/`：vendor / 上游改动记录。
* 分类目录不存在时创建；新增分类必须在本节登记后再使用。
* 任何修改都必须有文档：产品代码、测试脚本、workflow、i18n、AGENTS.md 自身的改动，均须在对应分类生成或更新文档；修复进 `doc/fix/`，功能进 `doc/feat/`，规划与回写进 `doc/plan/`，测试或脚本调整进 `doc/test/`。
* 无需文档的豁免仅限：纯行尾或 `cargo fmt` 格式化、`temp/` 下草稿、无任何语义变化的重跑；豁免之外不得跳过。
* `doc/` 及其子目录默认仅作本地记录，不加入暂存区或提交，除非项目负责人明确要求。

### README.md 同步要求

* `README.md`（中文）为权威版本，内容、命令、链接与发布说明以它为准；`README.en.md` 是其英文版本，必须与 `README.md` 保持同步。
* 涉及用户可见功能、默认行为、快捷键、安装方式、发布流程、WinGet、构建命令、产品身份或项目链接的改动，必须检查并在需要时同步更新 `README.md`；仅内部重构、测试或 CI 实现细节变化，且不影响用户使用或项目维护方式时无需修改。
* 更新时保持现有结构，避免无关改写，并确保产品名称、链接、命令和发布说明与当前实现一致，不得引用已删除或改名的 workflow、脚本或产物。
* 未改动 `README.md` 语义时，不对 `README.en.md` 做无关编辑。


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

按改动范围选择适用检查：Rust 改动执行下列全部；仅文档改动执行 `git diff --check`；仅 PowerShell 改动另见下方「PowerShell 脚本验证」。

```powershell
cargo fmt --all
cargo fmt --all -- --check
git diff --check
cargo check --workspace --target x86_64-pc-windows-msvc
cargo clippy --workspace --all-targets --target x86_64-pc-windows-msvc -- -D warnings
cargo test --workspace --target x86_64-pc-windows-msvc
```

目标与 CI（`ci.yml`）保持一致，均为 `x86_64-pc-windows-msvc`。本机若已安装 MinGW 工具链，
可额外执行 GNU 目标检查，但 CI 不强制。`cargo check` 等命令需要 Cargo 环境时，先按本机
方式加载（Windows 上通常已由 rustup 配置，无需 `source` 脚本）。

涉及 `windui`、发布打包或安装器时，必须执行对应额外检查。

### PowerShell 脚本验证

凡改动 `scripts/*.ps1`、`PSScriptAnalyzerSettings.psd1` 或 workflow 内嵌 PowerShell 片段，
提交前必须执行以下检查：

```powershell
Set-PSRepository PSGallery -InstallationPolicy Trusted
Install-Module PSScriptAnalyzer -Scope CurrentUser -Force
Invoke-ScriptAnalyzer -Path scripts -Recurse -Settings ./PSScriptAnalyzerSettings.psd1 -Severity Warning,Error
```

零输出方为通过。同时必须全量解析 `scripts/*.ps1` 语法（PowerShell AST Parser），零错误。

- 未改动任何 PowerShell 内容时不强制执行本节。
- 改动 `PSScriptAnalyzerSettings.psd1` 本身时必须全量复扫，并逐条说明新增例外的理由。
- 禁止为绕过告警新增 ExcludeRules 或内联抑制；确需排除必须在设置文件内以注释写明理由
  （现有 `PSAvoidUsingWriteHost` 例外即此先例）。
- 上述检查已在 `ci.yml` 的「验证 PowerShell 脚本」步骤中强制；脚本改动未经本地检查即推送
  会导致 CI 失败。

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
```
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

## 18. 版本与 Release 规范

* 项目版本遵循 **Semantic Versioning 2.0.0（SemVer）**。
* GitHub Release 使用 v 前缀的版本 tag，例如 `v0.1.0`。

---

## 19. Beta 发布

* 每次产品修复完成后准备一次**手工 beta 发布**。
* Beta 必须通过 `Windows 发布` workflow 的 `release_channel=beta` 生成；发布必须 `prerelease: true`，名称不得包含 `(beta)`。
* 禁止空、重复或无说明发布，也不得由 push、定时任务或内部提交自动创建。
* 发布版本、通道和说明必须由 Agent 主动选择并核对。
* 发布前确认版本、`Cargo.lock`、安装器版本、tag、产物元数据一致，且安装程序与便携版资产均存在。
* 发布说明须使用中文并由人工撰写或校对，包含变更摘要、实际验证、已知限制、runner / DWM 限制、Windows 验证步骤及安装程序 / 便携版直接下载链接。
* 下载优先级：安装程序 > 便携版。

---

## 20. 稳定发布与 WinGet

* 稳定发布必须通过明确的用户指令，并手动运行 `Windows UI 发布` workflow 的 `release_channel=stable`。
* WinGet 仅提交稳定版本，包标识符为 `lvxingqi.FluxLauncherCN`，路径为 `manifests/l/lvxingqi/FluxLauncherCN/<version>/`。
* 提交前根据实际安装程序核实 URL、SHA256、schema、安装器元数据及“应用与功能”名称。
* Beta 不得进入 WinGet；WinGet 自动化不得创建 GitHub Release，且仅在明确启用稳定版策略后才能准备或提交稳定版 PR。
* 未经用户明确要求，不得创建 `WINGET_GITHUB_TOKEN` 或签名密钥。
* 手工 WinGet PR 不需要仓库密钥，且必须与 Beta 发布流程分离。

---

## 21. 完成标准

任务仅在以下条件满足后才能报告为完成：

* 已检查相关代码、CI、发布历史和测试，并完成最小完整修改。
* 必要的 i18n、文档及修复报告已生成或更新。
* 适用的本地检查与 Windows 验证全部通过。
* 涉及生命周期、视觉、安装器或启动行为时，已完成对应 Windows 验证。
* 对属于产品发布范围的修复，已准备对应 Beta 发布，并核实发布元数据、资产和说明。
* 已知限制已如实记录。

在条件满足前，不得声称已完成、已验证、可发布或可交付。

---

## 22. Plan / Act 协作

### Plan 模式
- 必须把规划结果写入 `doc/plan/<任务名>.md`，包含：任务目标、编号步骤清单、每步验收标准。
- 步骤清单必须编号，便于 Act 模式回写状态。

### Act 模式
- 开始前必须先查找 `doc/plan/` 下与当前任务对应的最近一份计划文档并阅读，严格按其中步骤执行。
- 存在多份相关计划文档时，以最近一份且尚未完成的为准；状态只回写正在执行的那份。
- 不得擅自扩大范围或跳过步骤。
- 若遇到计划未覆盖的新问题、方案不成立、依赖缺失等阻塞，必须立即停止，禁止猜测或绕过。
- 停止后，在计划文档对应步骤下追加「执行阻塞」小节，写清：现象、报错、已尝试方法、阻塞点。
- 回写完成后结束本次 Act，等待用户手动切回 Plan 重新规划。

---

## 23. 优先级原则

规则冲突时按以下优先级处理：

1. Windows 生命周期与安全
2. 用户行为与 UX
3. i18n 与架构
4. 验证与正确性
5. 最小修改
6. 发布与交付
7. 文档与提交

不得为了通过检查而绕过 Windows 生命周期、真实 Acrylic / Mica、i18n、Everything 回落、启动语义、安装器烟雾测试或必要的 Windows 验证。

---

## 24. 测试与验证资产规范

适用于 `scripts/*.ps1`、`.github/workflows/` 与 Rust 测试代码。

### 探针与烟雾测试

* 探针必须断言产品不变量（隐藏态近乎零 CPU、热键切换可见性、Acrylic / Mica 生效、启动语义等），不得只检查“进程没崩溃”或“文件存在”。
* 探针必须先建立前置条件再断言，且不得假定前置条件在多次操作之间保持不变；对焦点、可见性这类会被环境改变的状态，须在派发前重新确认，并对竞态场景恢复前置条件后重试，而不是把一次派发结果当作失败。
* 断言失败必须先看日志与 artifact 再定性；定性为竞态或环境问题时，报告须附 run id、trace 证据与「为何不是产品回归」的依据。禁止凭猜测把失败归因于 flake。
* 断言消息必须写明被破坏的不变量，便于从 CI 日志定位。
* 新增或修改烟雾测试须同步三处：脚本本身、workflow 的输入开关与默认值、`doc/` 下的验证记录。

### 测试代码

* 新增功能必须带单元测试；可测逻辑不得只靠手工验证。
* 单元测试与集成测试沿用现有组织方式（crate 内 `tests.rs` 与既有 helper），不引入新的测试框架或外部测试依赖。
* 涉及 Windows 生命周期的行为，测试须覆盖隐藏 / 显示往返，不得只测单次转换。

### Workflow

* workflow 改动必须先做本地可验证的部分（脚本语法解析、PSSA、`actionlint` 等可用检查），再推送。
* 嵌入 workflow 的 PowerShell 片段受第 15 节 PowerShell 检查约束。
* 新增 workflow 输入须同步默认值、调用方与文档；不得留下只在 CI 中可用而无法本地复现的开关。
* CI 目标平台即为本地检查目标平台，二者必须一致，不得要求本地执行 CI 无法执行的检查。
* workflow 引用的 secrets / vars 名称必须维持在允许清单内，并由 `scripts/validate-workflow-secret-references.ps1` 在 CI 校验；新增、改名或删除密钥时必须同步该脚本的清单与第 20 节。
* 编辑器提示 “Context access might be invalid: NAME” 属预期：本仓库按需配置密钥，workflow 在运行时自行守卫（无提交凭据时 fail-fast、签名仅限稳定通道），因此不得为了消除提示而创建空密钥或伪造变量。

---

## 25. AGENTS.md 自身维护

* 规则修改的理由必须记录：规则调整进 `doc/plan/`，发现的矛盾或失效规则进 `doc/question/` 或对应计划文档。修改本文档本身也属于第 7 节“任何修改都必须有文档”的范围。
* 禁止写入时效性快照：版本号、发布日期、当前 CI / 发布状态、临时路径等易腐内容一律不写入，改为引用当前实现或规则本身。
* 格式规范：章节标题统一为 `## N. 标题`（编号后有空格）；条目使用 `*` 或编号列表，不混用缩进层级。
* 修改章节编号前必须全文检查交叉引用（例如“第 9、10 节”“第 15 节”），并同步更新引用。
* 增量克制：优先修订既有条目，不为单次事件新增规则；新增规则必须能被“违反时如何发现”支撑，否则应写成注意事项而非硬约束。
* 新增规则不得与第 23 节优先级冲突；冲突时应修订既有条目而不是叠加例外。
