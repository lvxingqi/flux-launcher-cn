# Flux Launcher Agent Guidelines

## 1. Project scope and core constraints

Flux Launcher is a native Windows 11 launcher written in Rust.

### GUI and rendering

* The GUI must use the vendored `windui` framework exclusively.
* Do not introduce WebView, Electron, egui, iced, Tauri, or another GUI framework.
* The application is a tray-resident process.
* The window background must use the real Windows DWM Acrylic or Mica path through the existing Win32 and DirectComposition implementation.
* Keep the entire launcher surface transparent so the system material covers the complete window.
* Do not replace the system material with fake gradients, opaque cards, tinted gradients, or WCA AccentPolicy.

### Architecture

* Preserve the responsibility boundary between `flux-core`, Flux application code, and the vendored `windui` backend.
* Prefer small, platform-specific changes and avoid unnecessary broad rewrites.
* Everything integration must retain a graceful unavailable-service fallback.
* Keep Windows-specific code in appropriate platform modules.
* Preserve non-Windows cross-target compilation and the existing dependency lock where practical.

---

## 2. Repository language and commit conventions

### Source, documentation, and release notes

Preserve the existing language of each file and do not perform unrelated translation or rewriting.

New or modified content should default to Chinese, including:

* Source comments
* Project documentation
* Fix and feature documentation
* Release notes

When an English version is needed, use an `_en` suffix, for example:

```text
release-notes.md
release-notes_en.md
```

Existing content does not require an English counterpart unless explicitly requested.

### Git commits

Follow **Conventional Commits 1.0.0**. Commit messages must use English and an imperative description.

Format:

```text
<type>(launcher): <description>
```

---

## 3. Internationalization (i18n)

Flux uses `rust-i18n` for all user-facing UI text.

Translation files:

```text
crates/flux-launcher/locales/en.yml
crates/flux-launcher/locales/zh-CN.yml
```

`en.yml` is the fallback locale.

### Rules

* Every user-visible UI string must be obtained through `t!`; do not hardcode visible text in UI code.
* When text is added or changed, update both `en.yml` and `zh-CN.yml` in the same change and keep their keys identical.
* Missing or unsupported translations must fall back to English; never show a raw key or panic.
* Detect the system language with `sys-locale` at startup and apply it through `rust_i18n::set_locale`; unknown languages fall back to `en`.
* Keep the existing `i18n!("locales", fallback = "en")` path unless the related configuration is updated together.
* Version numbers, internal identifiers, colors, constants, and stderr diagnostics do not need translation.

---

## 4. Required working process

* Before editing, inspect the relevant source, repository rules, CI, tests, release history, installer, Windows validation, and issue reports. Do not guess when existing logs can answer the question.
* Confirm the problem first, then make the **smallest complete change** while preserving the existing architecture and avoiding unrelated refactoring.
* For multi-step tasks, create a plan covering investigation, implementation, validation, and delivery.
* Report meaningful checkpoints when CI is waiting or failing, external review is pending, or Windows validation is incomplete.
* Do not claim that a fix is complete or publishable until applicable local and Windows validation has passed.

---

## 5. Tool failures and incident records

* For tool failures, timeouts, unexpected results, or partial state changes, promptly create a Chinese Markdown report under `doc/question/` containing the time, tool or service, operation, impact, original error, diagnosis, recovery, and validation result.
* `AGENTS.md` and `AGENTS.en.md` are the Chinese and English versions of the same rules. Update both together and keep their requirements semantically equivalent.
* `apply_patch` is disabled; do not call it.
* Add a tool to `cline.disabledTools` only after confirming that the Cline built-in tool is unavailable or the same operation has repeatedly failed. Do not disable a tool for a single transient failure.
* Failures of `github`, `desktop-commander`, or `serena` are not Cline built-in tool failures.

---

## 6. Line endings and formatting

* Rust uses the repository `rustfmt.toml`; run `cargo fmt --all` for formatting or line-ending issues and do not manually rewrite Rust line endings.
* If a non-Rust file has CRLF, damaged line endings, or a diff caused only by line endings, stop automated editing, preserve its content, normalize it to LF manually, and verify the worktree and index with byte-level checks and `git diff --check`.
* Do not use scripts to batch-rewrite line endings unless manual repair is not feasible.
* Running only `git add` does not count as completing a fix.

---

## 7. Documentation rules

* Documentation under `doc/` must be written in Chinese.
* After a product fix, create a corresponding Chinese Markdown report under `doc/fix/` containing at least the implementation, validation result, known limitations, and user verification steps.
* New features should use a separate category such as `doc/feat/`; create the directory when it does not exist.
* Files under `doc/` are local records by default and must not be staged or committed unless the project owner explicitly requests it.
* This includes reports under `doc/fix/` and `doc/question/`.

---

## 8. Windows lifecycle and startup behavior

* The default global hotkey is `Alt+Space` and must remain configurable.
* Repeated activation toggles launcher visibility; after showing, the search box receives focus immediately.
* Clear-query-on-activation, Game Mode protection, and fullscreen hotkey protection are enabled by default.
* Application results rank before ordinary files and folders.
* Preserve the existing Flow-style keyboard navigation: `Up`, `Down`, `Home`, `End`, `Enter`, `Right`, `Left`, and `Escape`.

---

## 9. Windows startup entries and installer launch behavior

* The installer options **Start Flux Launcher with Windows** and **Launch Flux Launcher now** must remain independent. The startup option is enabled by default but cancellable; the immediate-launch option is selected by default.
* The startup registry command must use `--startup`.
* `--startup` must call `windui::start_hidden()`, creating only the tray process without showing the search window. Only the global hotkey or the tray's **Show launcher** action may show the window.
* Installer smoke tests must cover the default startup entry, the `/TASKS=!startup` opt-out path, and hidden `--startup` mode.

---

## 10. Start Menu shortcuts and icons

* The Start Menu shortcut must target the actually installed executable and explicitly reference the Flux Launcher `.ico` resource.
* The installation directory must contain the multi-resolution `.ico` resource.
* Installer smoke tests must verify the shortcut target, icon metadata, and actual icon reference rather than only checking that the shortcut exists.

---

## 11. Windows Acrylic / Mica and lifecycle invariants

* `ShowWindow` must establish visibility before show callbacks modify layout state.
* After visible activation, the first transparent D2D frame must be invalidated and presented before relying on input or query state.
* Acrylic / Mica must remain functional after repeated hide/show operations and after window-size, paint, or composition changes.
* Before release, verify dark and light readability, non-overlapping titles and result rows, clear reactive selection, and intact Windows accent-color and custom-palette fallback behavior.

---

## 12. Query history and UX invariants

* Submitted queries persist in bounded, case-insensitive, newest-first history.
* `Ctrl+H` opens history; `Enter` or a mouse click reruns the selected query.
* On an empty query, plain `Up` recalls the latest query; `Alt+Up` and `Alt+Down` cycle backward and forward.
* Settings provides a way to clear history.
* Provider status remains visible in the expanded action bar without damaging Acrylic / Mica or using an opaque replacement surface.

---

## 13. Everything integration

* After installation, always register the Everything file and folder provider; when the service is unavailable, fall back gracefully to an unavailable-service state.
* Preserve native syntax such as `ext:zip`, `parent:`, `file:`, and `dm:`.
* The launcher must continue running normally when Everything is unavailable and must not crash.
* Application results always remain ahead of ordinary Everything file and folder results.

---

## 14. Flow plugins and built-in capabilities

* Flow plugins support only native plugins and executable JSON-RPC plugins.
* Do not add Python or C# plugin execution.
* Google and Obsidian must remain built into the main executable.
* Do not split these capabilities into a separate community plugin host unless the user explicitly requests it.

---

## 15. Required validation before reporting completion

Run the applicable checks before committing or reporting completion:

```bash
source "$HOME/.cargo/env"
cargo fmt --all
cargo fmt --all -- --check
git diff --check
cargo check --workspace --target x86_64-pc-windows-gnu
cargo clippy -p flux-core -p flux-launcher --all-targets --target x86_64-pc-windows-gnu -- -D warnings
cargo test -p flux-core
```

When touching `windui`, release packaging, or the installer, run the corresponding additional checks.

When a check fails, analyze the log and determine the cause first. Rerun only after confirming that the failure is environmental or transient; never ignore a failure because it appears unrelated.

---

## 16. Windows release validation

For lifecycle, visual, installer, startup-entry, shortcut, Acrylic / Mica, or startup changes, manually run the `Windows Release` workflow with `release_channel=beta`:

```bash
gh workflow run windows-release.yml \
  --repo lvxingqi/flux-launcher-cn \
  --ref main \
  -f release_tag=vX.Y.Z \
  -f runner_label=windows-latest \
  -f release_channel=beta
```

The `release_tag` must match the actual build version and must not reuse an existing release tag.

Do not publish or report the beta as complete until the workflow succeeds.

The workflow must include:

* Installer and portable build
* Windows UI capture
* `scripts/installer-smoke.ps1`

Installer smoke tests must cover the startup entry, hidden startup, shortcut, icon, executable hash, post-install configuration, and uninstall cleanup required by sections 9 and 10.

---

## 17. Windows visual smoke tests

* When available, use the configured secondary display and cover empty startup, pre-query capture, repeated hide/show, query expansion, keyboard selection, action mode, Enter, and Settings.
* GitHub Windows runner screenshots are evidence of the rendering path only. They cannot replace or overrule observation of real Windows 11 DWM Acrylic / Mica on a physical machine.
* Release notes must state this limitation honestly.

---

## 18. Beta releases

* Prepare one **manual beta release** after each product fix.
* Generate beta releases through the `Windows Release` workflow with `release_channel=beta`; releases must use `prerelease: true` and their names must not contain `(beta)`.
* Do not create empty, duplicate, or unexplained releases, and do not create releases automatically on pushes, schedules, or internal commits.
* The Agent must actively choose and verify the release version, channel, and notes.
* Before release, confirm that the version, `Cargo.lock`, installer version, tag, artifact metadata, installer asset, and portable asset agree.
* Release notes must be written or manually reviewed in Chinese and include the change summary, actual validation, known limitations, runner / DWM limitations, Windows verification steps, and direct installer / portable download links.
* Prefer the installer over the portable download.

---

## 19. Stable releases and WinGet

* Stable releases require an explicit user instruction and a manual run of the `Windows UI Release` workflow with `release_channel=stable`.
* WinGet accepts stable versions only. The package identifier is `m1nuzz.FluxLauncher`, and the canonical path is `manifests/m/m1nuzz/FluxLauncher/<version>/`.
* Before submission, verify the URL, SHA256, schema, installer metadata, and Apps & Features name against the actual installer.
* Beta versions must not enter WinGet. WinGet automation must not create GitHub Releases and may prepare or submit a stable PR only when the stable-release policy is explicitly enabled.
* Do not create `WINGET_GITHUB_TOKEN` or signing keys unless the user explicitly requests it.
* A manual WinGet PR does not require repository secrets and must remain separate from the beta release process.

---

## 20. Completion criteria

A task may be reported as complete only when all of the following are satisfied:

* Relevant code, CI, release history, and tests were inspected and the smallest complete change was made.
* Required i18n, documentation, and fix reports were generated or updated.
* Applicable local checks and Windows validation passed.
* Lifecycle, visual, installer, or startup changes received the required Windows validation.
* Product-release fixes have a prepared beta release with verified metadata, assets, and notes.
* Known limitations are stated honestly.

Do not claim completion, validation, publishability, or delivery before these conditions are met.

---

## 21. Priority order

When rules conflict, use this priority order:

1. Windows lifecycle and security
2. User behavior and UX
3. i18n and architecture
4. Validation and correctness
5. Minimal change
6. Release and delivery
7. Documentation and commits

Do not bypass Windows lifecycle, real Acrylic / Mica, i18n, Everything fallback, startup semantics, installer smoke tests, or required Windows validation merely to pass a check.
