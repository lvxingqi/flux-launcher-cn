# Flux Launcher CN

> **兼容性说明：** Flux Launcher CN 目前仅在 Windows 11 上验证过。其他 Windows 版本尚未经过正式确认。

<p align="center">
  <img src="assets/logotype.png" alt="Flux Launcher CN logo" width="520">
</p>

<p align="center">
  <strong>一款使用 Rust 构建的轻量级原生 Windows 11 启动器与文件搜索工具。</strong>
</p>

<p align="center">
  <a href="https://github.com/lvxingqi/flux-launcher-cn/releases/latest"><img src="https://img.shields.io/github/v/release/lvxingqi/flux-launcher-cn?label=latest%20release" alt="最新版本"></a>
  <a href="https://github.com/lvxingqi/flux-launcher-cn/actions/workflows/ci.yml"><img src="https://github.com/lvxingqi/flux-launcher-cn/actions/workflows/ci.yml/badge.svg?branch=main" alt="CI 状态"></a>
  <a href="https://github.com/lvxingqi/flux-launcher-cn/blob/main/LICENSE"><img src="https://img.shields.io/badge/license-MIT-2ea44f.svg" alt="MIT license"></a>
  <a href="https://www.rust-lang.org/"><img src="https://img.shields.io/badge/built%20with-Rust-orange.svg" alt="使用 Rust 构建"></a>
</p>

<p align="center">
  <a href="https://github.com/lvxingqi/flux-launcher-cn/releases/latest/download/FluxLauncher-Setup.exe">下载</a>
  · <a href="#功能特性">功能特性</a>
  · <a href="#使用方法">使用方法</a>
  · <a href="#性能">性能</a>
  · <a href="#插件与-provider">插件</a>
  · <a href="#从源码构建">从源码构建</a>
</p>

Flux Launcher CN 是一款面向 Windows 11 的轻量级、键盘优先的原生启动器与文件搜索工具。它使用 `Alt+空格键` 启动，可以查找应用程序和文件，支持 Everything 搜索语法，可以启动网页搜索，支持 Obsidian Vault，并且兼容 Flow Launcher 的原生可执行插件。

Flux Launcher CN 的界面完全基于 windui 和 Windows 11 Acrylic/DWM 组件路径构建，不使用 WebView、Electron、Tauri 或任何浏览器引擎。

## 功能特性

Flux Launcher CN 面向希望拥有一款快速、键盘优先、外观原生、资源占用可预测且分发体积小的 Windows 启动器的用户。

| 能力            | Flux Launcher CN 提供的功能                                                 |
| ------------- | ---------------------------------------------------------------------- |
| 原生 Windows UI | 仅使用 Rust 与 windui，支持 Windows 11 Acrylic，并在合成不可用时提供半透明回退                |
| 应用优先搜索        | 已安装的应用程序与快捷方式排在普通索引文件与文件夹之前                                            |
| 文件搜索          | Everything IPC，支持 `ext:zip`、`parent:`、`file:`、`folder:`、`dm:today` 等查询 |
| 内置 provider   | Google 搜索（`g`）与 Obsidian 笔记库搜索（`ob`）                                   |
| 插件兼容          | 兼容旧版 Flow `Executable`/`Executable_V2` JSON-RPC 插件，以及隔离的原生 Rust 社区插件   |
| 键盘工作流         | 结果导航、操作模式、历史记录、复制路径、以管理员身份运行、打开文件位置                                    |
| Windows 集成    | 全局热键、全屏感知的游戏模式、Windows 强调色、系统托盘、回收站命令、显示器选择、可选的 Windows 开机自启           |

## 安装

对于大多数用户，推荐使用[最新的 Windows 11 安装程序](https://github.com/lvxingqi/flux-launcher-cn/releases/latest/download/FluxLauncher-Setup.exe)安装 Flux Launcher CN。安装程序是推荐选项，会默认勾选 **Start Flux Launcher automatically with Windows**（随 Windows 自动启动 Flux Launcher CN）并创建开始菜单快捷方式。该设置之后可在 `Settings > General > Windows startup` 中更改。

也可以通过 WinGet 安装或升级 Flux Launcher CN：

```powershell
winget install --id lvxingqi.FluxLauncherCN --exact
```

如果不希望使用安装程序，可下载[最新的便携版](https://github.com/lvxingqi/flux-launcher-cn/releases/latest/download/FluxLauncher-Portable.exe)直接运行。便携版使用相同的开机自启偏好；如果不希望注册开机自启，可在 Settings 中关闭 `Start Flux automatically with Windows`。

Flux Launcher CN 不强制要求 Everything，但推荐安装它用于索引文件与文件夹搜索。如果未安装，Flux Launcher CN 可在 Settings 中提供以下命令：

```powershell
winget install -e --id voidtools.Everything
```

Flux Launcher CN 将用户设置存储在 `%APPDATA%\FluxLauncher\settings.json`。默认激活热键为 `Alt+Space`，默认显示器为鼠标光标所在的显示器，且默认启用全屏热键抑制。稳定版更新检查默认开启，每 24 小时运行一次，错过检查间隔后会立即补检。Flux Launcher CN 只检查 GitHub 稳定版本，忽略 beta/prerelease 版本。在 `Settings > General > Updates` 中可更改检查间隔，并选择“安装前询问”或“自动安装稳定更新”。

## 使用方法

| 输入                            | 结果                       |
| ----------------------------- | ------------------------ |
| `Alt+Space`                   | 显示或隐藏启动器                 |
| `Steam`、`Chrome` 或任意应用名       | 优先搜索已安装的应用程序             |
| `ext:zip`、`.zip`、`.mp4 video` | 按文件扩展名搜索 Everything      |
| `g space exploration`         | 在默认浏览器中打开 Google 搜索      |
| `ob project roadmap`          | 搜索 Obsidian 笔记库          |
| `Ctrl+H`                      | 打开已提交的查询历史               |
| `ArrowUp` / `ArrowDown`       | 循环移动选择结果                 |
| `Tab` / `Shift+Tab`           | 使用 Flow 风格的 Tab 导航移动选择结果 |
| `Enter`                       | 启动选中的结果或执行其插件操作          |
| `ArrowRight`                  | 打开选中结果的操作菜单              |
| `Ctrl+C`                      | 可用时复制选中路径                |
| `Ctrl+R`                      | 以管理员身份运行选中的应用程序          |
| `Escape`                      | 从操作返回或隐藏启动器              |

### 键盘布局与输入法（IME）

Flux Launcher CN 使用 windui 提供的 Unicode Win32 输入路径。普通文本、Unicode `WM_IME_CHAR` 结果以及已提交的 `WM_IME_COMPOSITION` 结果都经过同一个 UTF-16 解码器，因此中文字符和增补平面 Unicode 文本遵循相同的焦点与代理对行为。输入法组合窗口在组合进行期间定位于搜索光标的焦点位置。

`Settings > General > Start typing in English and restore the previous layout on hide`（开始输入时切换英文布局，隐藏时恢复之前的布局）选项仍然可用；禁用该选项可让 Windows 保留用户选择的输入法。

如果键盘输入无法到达搜索框，可通过 PowerShell 为单次会话启用诊断：

```powershell
$env:FLUX_INPUT_TRACE_FILE = Join-Path $env:TEMP "flux-input-trace.log"

.\flux-launcher.exe
```

跟踪信息仅包含消息名称、HWND/线程关系、活动 HKL 标识符、IME 组合状态与路由标志。它绝不记录键入字符、查询文本、剪贴板内容或私有文件路径。诊断结束后移除该环境变量即可恢复默认的零跟踪路径。

搜索结果每个 provider 最多 16 条。应用程序会去重并排在普通 Everything 文件之前。查询历史原子化持久化、不区分大小写去重，且上限为 32 条。

## 性能

### Flux Launcher CN 内存测量

以下测量来自最近一次成功的 Windows smoke 运行，是进程自有内存占用的有用指标。

| 状态     |   Working set | Private bytes |
| ------ | ------------: | ------------: |
| 空闲、空查询 | **33.75 MiB** |  **8.72 MiB** |
| 查询进行中  | **41.97 MiB** | **19.23 MiB** |
| 历史面板   | **57.69 MiB** | **24.30 MiB** |

这些测量是特定时间点的证据，而非普遍保证。它们采集自 GitHub 托管的 Windows Server 2025 运行器（[smoke run 32301567507](https://github.com/lvxingqi/flux-launcher-cn/actions/runs/32301567507)）。桌面合成、显示器数量、DPI 缩放、字体、驱动、Everything 以及已安装的插件都可能改变内存占用。

### 与 Flow Launcher 的对比

Flux Launcher CN 被设计为**内存更低的 Flow Launcher 替代品**。Flow Launcher 自己的 issue 跟踪器中，维护者在某一配置下引用了约 **130–160 MB** 的常规基线，并报告了打开 Settings、使用插件或浏览插件商店后更高的占用 [1]。Flux Launcher CN 在上述 smoke 运行中测得**空闲时私有字节 8.72 MiB**、**查询进行中 19.23 MiB**。

这是方向性对比，而非实验室的逐项基准：Flow 的数据来自不同机器与配置的社区报告，而 Flux Launcher CN 的数据是自动化 CI 测量。重要区别在于 Flux Launcher CN 发布了具体测量值，而不是宣称一个通用的内存数字。

## 插件与 provider

Flux Launcher CN 采用混合插件架构。Google 搜索与 Obsidian 内置于 `flux-launcher.exe`，不产生插件子进程。旧版 Flow 可执行插件仍通过有界的换行分隔 JSON-RPC 获得支持。新的社区插件可以用 Rust 编写为 `cdylib` DLL，使用稳定的 `flux-plugin-sdk` C ABI。

原生社区宿主与 UI 进程隔离，仅当 `%APPDATA%\FluxLauncher\NativePlugins\` 中存在已安装的插件时才启动。同一可执行文件充当宿主：

```text
flux-launcher.exe --plugin-host <plugin-root>
```

原生插件包包含 `plugin.toml` 及其平台匹配的 DLL。清单声明 API 版本、动作关键词与权限。声明式动作包括 `OpenUrl`、`OpenPath` 与 `CopyText`。如果宿主退出或插件崩溃，Flux Launcher CN 会丢弃原生结果并在后续查询中重试宿主，而不终止启动器 UI。

仓库包含完整的 SDK 与示例插件：

| 路径                           | 用途                               |
| ---------------------------- | -------------------------------- |
| `crates/flux-plugin-sdk`     | 稳定的 C ABI 类型、缓冲区所有权、清单校验、权限与动作   |
| `crates/flux-plugin-example` | Rust `cdylib` 示例，用于原生宿主 smoke 测试 |
| `crates/flow-plugin-fixture` | 用于兼容性测试的原生可执行 Flow JSON-RPC 夹具   |

## 从源码构建

Flux Launcher CN 面向 **Windows 11 x64**，使用 `x86_64-pc-windows-msvc` Rust 目标。安装稳定版 Rust 工具链与 Visual Studio C++ 构建工具，然后运行：

```powershell
rustup target add x86_64-pc-windows-msvc
cargo build --workspace --release --target x86_64-pc-windows-msvc
```

启动器可执行文件输出到：

```text
target\x86_64-pc-windows-msvc\release\flux-launcher.exe
```

运行便携测试：

```bash
cargo test -p flux-core -p flux-plugin-sdk
```

Windows CI 工作流还会运行格式化、Clippy（警告视为错误）、workspace 测试与 release 构建。Windows 视觉 smoke 工作流检查启动/隐藏循环、Acrylic 生命周期行为、键盘选择、Settings、Everything 语法、历史记录、原生 Flow 兼容性与显示器位置。

## 发布通道

Windows 发布统一通过 `Windows 发布` 工作流执行，并通过 `release_channel` 区分发布通道。

Beta 发布用于测试，使用 `release_channel=beta`，GitHub Release 标记为 prerelease。Flux Launcher CN 稳定版更新器不会消费 beta/prerelease 版本，Beta 版本也不会提交到 WinGet。

稳定版发布使用 `release_channel=stable`，仅在明确执行稳定版发布时创建。稳定版必须对应非 draft、非 prerelease 的 GitHub Release，稳定版安装程序及其校验信息随后可用于 WinGet 提交。

Windows 发布不会因为 push、schedule 或内部提交自动创建 GitHub Release。Beta 和稳定版发布均由维护者手动触发。

稳定版与 Beta 共享同一个 Windows 发布入口，正式发布前应完成 Windows 构建、测试、安装器和便携版验证。

## WinGet 与 SmartScreen

Flux Launcher CN 的 WinGet PackageIdentifier 为 `lvxingqi.FluxLauncherCN`。

项目在 [`packaging/winget/manifests`](packaging/winget/manifests) 下维护 schema 1.12 的多文件 WinGet 清单模板，并通过 [`scripts/generate-winget-manifest.ps1`](scripts/generate-winget-manifest.ps1) 根据稳定版发布信息自动生成版本、下载地址、SHA256 和显示版本等字段。

生成的清单需要提交到官方 [`microsoft/winget-pkgs`](https://github.com/microsoft/winget-pkgs) 仓库。向本仓库添加清单文件只是准备提交，不会自动将 Flux Launcher CN 发布到 WinGet。

WinGet 仅针对稳定版 GitHub Release，不使用 beta/prerelease，也不使用可变的 `latest` 下载地址。

稳定版 Windows 产物可在配置签名证书后执行 Authenticode 签名。签名用于提供发布者身份并帮助建立发布信誉，但新文件仍可能受到 SmartScreen 初始信誉提示影响。Beta 构建无需签名即可作为测试产物使用。

验证与发布签名流程见 [`packaging/winget/README.md`](packaging/winget/README.md)。

## 项目状态

Flux Launcher CN 正在积极开发中。`main` 分支可能包含尚未打包到稳定版中的改进。下载[最新的安装程序](https://github.com/lvxingqi/flux-launcher-cn/releases/latest/download/FluxLauncher-Setup.exe)，或在 [issue 跟踪器](https://github.com/lvxingqi/flux-launcher-cn/issues)中关注开发进展。

## 许可证

Flux Launcher CN 依据 [MIT License](LICENSE) 分发。

本项目基于 [m1nuzz/flux-launcher](https://github.com/m1nuzz/flux-launcher) 开发，并保留上游项目的 MIT License 与相应版权声明。

## 参考资料

| 参考                                                                                              | 提供内容                                                                                 |
| ----------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------ |
| [上游 Flux Launcher](https://github.com/m1nuzz/flux-launcher)                                     | 本项目的上游项目与基础实现                                                                        |
| [Flow Launcher](https://github.com/Flow-Launcher/Flow.Launcher)                                 | 键盘优先的 Windows 启动器 UX、Everything 集成、查询历史、热键与旧版插件兼容性的参考                                |
| [windui](https://github.com/huanfeng/wind-ui-rust)                                              | Flux Launcher CN 使用的原生 Rust GUI 框架                                                   |
| [look](https://github.com/kunkka19xx/look)                                                      | 搜索框平滑光标（Smooth Caret）交互的参考                                                           |
| [Windows Acrylic material](https://learn.microsoft.com/en-us/windows/apps/design/style/acrylic) | Flux Launcher CN 使用的 Windows 11 Acrylic/DWM 背景参考；Flux Launcher CN 使用 Acrylic 而非 Mica |

[1]: https://github.com/Flow-Launcher/Flow.Launcher/issues/2940 "Flow Launcher 内存占用讨论"
[2]: https://github.com/Flow-Launcher/Flow.Launcher/blob/dev/README.md "Flow Launcher README"
[3]: https://github.com/matiassingers/awesome-readme "Awesome README 示例"
[4]: https://github.com/banesullivan/README "README 编写指南"
[5]: https://www.voidtools.com/support/everything/ipc/ "Everything IPC 文档"
[6]: https://learn.microsoft.com/en-us/windows/apps/design/style/acrylic "Windows Acrylic 材质"
