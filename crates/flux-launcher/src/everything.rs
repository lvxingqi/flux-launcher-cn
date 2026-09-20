#[cfg(windows)]
use std::os::windows::process::CommandExt;
use std::path::PathBuf;
#[cfg(windows)]
use std::process::Command;
#[cfg(windows)]
use std::sync::OnceLock;
use std::sync::{
    atomic::{AtomicBool, Ordering},
    mpsc::{self, SyncSender},
    Arc, Mutex,
};
use std::thread;
use std::time::Duration;

use everything_ipc::wm::{EverythingClient, RequestFlags, Sort};
use flux_core::Settings;
use flux_core::{ResultKind, SearchResult};

use crate::applications::canonical_application_id;
use windui::prelude::{signal, Sender, Signal};

/// 低成本刷新 Everything 状态文本：注册表读取（约 0ms）+ IPC 探测（约 3-5ms），
/// 不枚举进程、也不拉起 Everything，可安全用于窗口激活与设置保存等 UI 线程路径。
///
/// 目的：状态文案反映真实可用性——用户在外部退出 Everything 后，这里会立即
/// 把状态改成「已安装但 IPC 不可用」，而不是保留启动时的陈旧结论。
pub(crate) fn refresh_everything_status_cheap(
    auto_enable: Signal<bool>,
    installed: Signal<bool>,
    status: Signal<String>,
) {
    if !auto_enable.get() {
        status.set(t!("everything.auto_enable_disabled").into_owned());
        return;
    }
    let outcome = refresh_state_cheap();
    installed.set(outcome.is_installed());
    status.set(outcome.status_message());
}

/// 把 `.ext query` 形式的查询规范化为 Everything 的 `ext:` 原生语法。
pub(crate) fn normalize_everything_query(query: &str) -> String {
    let trimmed = query.trim();
    let Some(rest) = trimmed.strip_prefix('.') else {
        return trimmed.to_owned();
    };
    let (extension, remainder) = rest.split_once(char::is_whitespace).unwrap_or((rest, ""));
    if extension.is_empty()
        || !extension
            .chars()
            .all(|character| character.is_ascii_alphanumeric())
    {
        return trimmed.to_owned();
    }
    let remainder = remainder.trim();
    if remainder.is_empty() {
        format!("ext:{extension}")
    } else {
        format!("ext:{extension} {remainder}")
    }
}

const MAX_RESULTS: u32 = 16;
const QUERY_TIMEOUT: Duration = Duration::from_millis(350);
pub const WINGET_PACKAGE_ID: &str = "voidtools.Everything";

pub(crate) struct EverythingRuntimeState {
    pub(crate) installed: Signal<bool>,
    pub(crate) prompt_visible_at_start: bool,
    pub(crate) prompt_visible: Signal<bool>,
    pub(crate) status: Signal<String>,
}

impl EverythingRuntimeState {
    pub(crate) fn new(settings: &Settings) -> Self {
        let installation = installation_state();
        let installed_at_start = installation.is_installed();
        let prompt_disabled = std::env::var("FLUX_DISABLE_EVERYTHING_PROMPT")
            .ok()
            .as_deref()
            == Some("1");
        let prompt_visible_at_start = crate::window_state::should_show_everything_install_prompt(
            installed_at_start,
            settings.auto_enable_everything,
            settings.everything_install_prompt_seen,
            prompt_disabled,
        );
        let installed = signal(installed_at_start);
        let status = signal(if !installed_at_start {
            t!("everything.not_installed_winget").into_owned()
        } else if settings.auto_enable_everything {
            // 真实的 IPC 可用性由紧随其后的 start_background_if_installed() 回填；
            // 这里不预判「已启用 IPC」，避免状态文案与事实不符。
            t!("everything.detecting").into_owned()
        } else {
            t!("everything.auto_enable_disabled").into_owned()
        });
        Self {
            installed,
            prompt_visible_at_start,
            prompt_visible: signal(prompt_visible_at_start),
            status,
        }
    }
}

#[cfg(windows)]
const CREATE_NO_WINDOW: u32 = 0x0800_0000;

fn startup_args() -> [&'static str; 1] {
    ["-startup"]
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum InstallationState {
    Installed(PathBuf),
    Missing,
}

impl InstallationState {
    pub fn is_installed(&self) -> bool {
        matches!(self, Self::Installed(_))
    }
}

/// 检测结果：安装状态 + 真实 IPC 可用性 + 本次是否尝试后台拉起 Everything。
///
/// 安装状态来自注册表（静态），IPC 可用性来自窗口消息连接（动态）。两者必须分开
/// 传递，否则 UI 会把「装了」当成「可用」——用户退出 Everything 后仍显示
/// 「IPC 可用」正是这样产生的。
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EverythingStartup {
    installation: InstallationState,
    ipc_available: bool,
    started_now: bool,
}

impl EverythingStartup {
    fn new(installation: InstallationState, ipc_available: bool, started_now: bool) -> Self {
        Self {
            installation,
            ipc_available,
            started_now,
        }
    }

    pub fn is_installed(&self) -> bool {
        self.installation.is_installed()
    }

    /// 当前探测时刻 IPC 是否真实可用。
    #[cfg(test)]
    pub fn ipc_available(&self) -> bool {
        self.ipc_available
    }

    /// 本次检测是否刚刚在后台拉起 Everything（IPC 可能仍在初始化）。
    #[cfg(test)]
    pub fn started_now(&self) -> bool {
        self.started_now
    }

    pub(crate) fn status_kind(&self) -> EverythingStatusKind {
        status_kind(
            self.installation.is_installed(),
            self.ipc_available,
            self.started_now,
        )
    }

    /// 设置页状态文案（i18n），与真实可用性一致。
    pub(crate) fn status_message(&self) -> String {
        self.status_kind().message()
    }
}

/// 状态文案分档：把「安装状态 + 真实 IPC 可用性」映射到用户可见文案。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum EverythingStatusKind {
    NotInstalled,
    IpcAvailable,
    IpcUnavailable,
    IpcUnavailableAfterStart,
}

impl EverythingStatusKind {
    pub(crate) fn message(self) -> String {
        match self {
            Self::NotInstalled => t!("everything.not_installed_winget").into_owned(),
            Self::IpcAvailable => t!("everything.ipc_available").into_owned(),
            Self::IpcUnavailable => t!("everything.ipc_unavailable").into_owned(),
            Self::IpcUnavailableAfterStart => {
                t!("everything.ipc_unavailable_after_start").into_owned()
            }
        }
    }
}

fn status_kind(installed: bool, ipc_available: bool, started_now: bool) -> EverythingStatusKind {
    if !installed {
        EverythingStatusKind::NotInstalled
    } else if ipc_available {
        EverythingStatusKind::IpcAvailable
    } else if started_now {
        EverythingStatusKind::IpcUnavailableAfterStart
    } else {
        EverythingStatusKind::IpcUnavailable
    }
}

#[cfg(windows)]
fn probe_ipc() -> bool {
    EverythingClient::new().is_ok()
}

#[cfg(not(windows))]
fn probe_ipc() -> bool {
    false
}

/// 窗口显示时的低成本状态刷新：注册表读取（约 0ms）+ IPC 探测（实测约 3-5ms）。
///
/// 刻意不枚举 `tasklist`、也不拉起 Everything，因此可以安全地放在激活回调里；
/// 用途是把「用户刚退出了 Everything」这类运行时变化如实反映到设置页状态，
/// 而不是沿用启动时的陈旧结论。
pub(crate) fn refresh_state_cheap() -> EverythingStartup {
    let installation = installation_state();
    let ipc_available = installation.is_installed() && probe_ipc();
    EverythingStartup::new(installation, ipc_available, false)
}

pub fn installation_state() -> InstallationState {
    if std::env::var_os("FLUX_SMOKE_EVERYTHING_MISSING").is_some() {
        return InstallationState::Missing;
    }
    if std::env::var_os("FLUX_SMOKE_EVERYTHING_INSTALLED").is_some() {
        return InstallationState::Installed(PathBuf::from("Everything.exe"));
    }
    installed_executable()
        .map(InstallationState::Installed)
        .unwrap_or(InstallationState::Missing)
}

pub fn winget_install_args() -> [&'static str; 4] {
    ["install", "-e", "--id", WINGET_PACKAGE_ID]
}

#[cfg(windows)]
fn everything_start_lock() -> &'static Mutex<()> {
    static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
    LOCK.get_or_init(|| Mutex::new(()))
}

#[cfg(windows)]
fn everything_start_requested() -> &'static AtomicBool {
    static REQUESTED: AtomicBool = AtomicBool::new(false);
    &REQUESTED
}

fn should_start_everything(ipc_available: bool, process_running: bool) -> bool {
    !ipc_available && !process_running
}

#[cfg(windows)]
fn everything_process_running() -> bool {
    let Ok(output) = Command::new("tasklist")
        .args(["/FI", "IMAGENAME eq Everything.exe", "/FO", "CSV", "/NH"])
        .creation_flags(CREATE_NO_WINDOW)
        .output()
    else {
        return false;
    };

    if !output.status.success() {
        return false;
    }

    String::from_utf8_lossy(&output.stdout)
        .lines()
        .any(|line| line.trim_start().starts_with("\"Everything.exe\""))
}

pub fn start_background_if_installed() -> Result<EverythingStartup, String> {
    let state = installation_state();
    let InstallationState::Installed(path) = &state else {
        return Ok(EverythingStartup::new(state, false, false));
    };

    #[cfg(windows)]
    {
        let _guard = everything_start_lock()
            .lock()
            .map_err(|_| String::from("Everything startup lock is poisoned"))?;
        let ipc_available = probe_ipc();
        if ipc_available {
            everything_start_requested().store(false, Ordering::Release);
            return Ok(EverythingStartup::new(state, true, false));
        }
        let process_running = everything_process_running();
        if !should_start_everything(ipc_available, process_running) {
            everything_start_requested().store(false, Ordering::Release);
            return Ok(EverythingStartup::new(state, false, false));
        }
        if everything_start_requested().load(Ordering::Acquire) {
            // 本进程已请求过拉起，IPC 仍未就绪：如实报告「已尝试启动，但仍不可用」。
            return Ok(EverythingStartup::new(state, false, true));
        }

        everything_start_requested().store(true, Ordering::Release);
        if let Err(error) = Command::new(path)
            .args(startup_args())
            .creation_flags(CREATE_NO_WINDOW)
            .spawn()
        {
            everything_start_requested().store(false, Ordering::Release);
            return Err(format!(
                "Unable to start Everything in the background: {error}"
            ));
        }
        // 刚拉起时 IPC 尚未就绪，不能声称可用。
        Ok(EverythingStartup::new(state, false, true))
    }

    #[cfg(not(windows))]
    {
        Ok(EverythingStartup::new(state, false, false))
    }
}

pub fn launch_winget_install() -> Result<(), String> {
    #[cfg(windows)]
    {
        Command::new("winget")
            .args(winget_install_args())
            .creation_flags(CREATE_NO_WINDOW)
            .spawn()
            .map(|_| ())
            .map_err(|error| format!("Unable to start winget: {error}"))
    }
    #[cfg(not(windows))]
    {
        Err(String::from("winget is only available on Windows"))
    }
}

#[cfg(windows)]
fn installed_executable() -> Option<PathBuf> {
    use windows::core::{w, PCWSTR};
    use windows::Win32::Foundation::ERROR_SUCCESS;
    use windows::Win32::System::Registry::{
        RegCloseKey, RegOpenKeyExW, RegQueryValueExW, HKEY, HKEY_CURRENT_USER, HKEY_LOCAL_MACHINE,
        KEY_READ, REG_EXPAND_SZ, REG_SZ,
    };

    unsafe fn read_string(root: HKEY, subkey: PCWSTR, value_name: PCWSTR) -> Option<String> {
        let mut key = HKEY::default();
        if RegOpenKeyExW(root, subkey, None, KEY_READ, &mut key) != ERROR_SUCCESS {
            return None;
        }

        let mut value_type = REG_SZ;
        let mut bytes = vec![0_u8; 32 * 1024];
        let mut byte_len = bytes.len() as u32;
        let result = RegQueryValueExW(
            key,
            value_name,
            None,
            Some(&mut value_type),
            Some(bytes.as_mut_ptr()),
            Some(&mut byte_len),
        );
        let _ = RegCloseKey(key);
        if result != ERROR_SUCCESS
            || (value_type != REG_SZ && value_type != REG_EXPAND_SZ)
            || byte_len < 2
        {
            return None;
        }

        let words = std::slice::from_raw_parts(bytes.as_ptr().cast::<u16>(), byte_len as usize / 2);
        Some(
            String::from_utf16_lossy(words)
                .trim_end_matches('\0')
                .trim()
                .to_owned(),
        )
    }

    unsafe fn read_install_path(root: HKEY, subkey: PCWSTR) -> Option<PathBuf> {
        if let Some(exe_path) = read_string(root, subkey, w!("ExePath")) {
            let path = PathBuf::from(exe_path);
            if path.is_file() {
                return Some(path);
            }
        }
        let install_location = read_string(root, subkey, w!("InstallLocation"))?;
        let path = PathBuf::from(install_location).join("Everything.exe");
        path.is_file().then_some(path)
    }

    let subkeys = [
        w!("Software\\voidtools\\Everything"),
        w!("Software\\WOW6432Node\\voidtools\\Everything"),
    ];
    let roots = [HKEY_LOCAL_MACHINE, HKEY_CURRENT_USER];
    for root in roots {
        for subkey in subkeys {
            if let Some(path) = unsafe { read_install_path(root, subkey) } {
                return Some(path);
            }
        }
    }

    None
}

#[cfg(not(windows))]
fn installed_executable() -> Option<PathBuf> {
    None
}

#[derive(Clone, Debug)]
pub struct EverythingResponse {
    pub sequence: u64,
    pub query: String,
    pub results: Vec<SearchResult>,
    pub status: String,
    pub available: bool,
}

#[derive(Clone, Debug)]
struct EverythingRequest {
    sequence: u64,
    query: String,
}

pub struct EverythingWorker {
    latest: Arc<Mutex<Option<EverythingRequest>>>,
    wake: SyncSender<()>,
}

impl EverythingWorker {
    pub fn spawn(output: Sender<EverythingResponse>) -> Self {
        let latest = Arc::new(Mutex::new(None::<EverythingRequest>));
        let latest_for_worker = Arc::clone(&latest);
        let (wake, receiver) = mpsc::sync_channel::<()>(1);

        thread::Builder::new()
            .name(String::from("flux-everything"))
            .spawn(move || {
                let mut client = None::<EverythingClient>;
                while receiver.recv().is_ok() {
                    let Some(request) = latest_for_worker
                        .lock()
                        .ok()
                        .and_then(|mut slot| slot.take())
                    else {
                        continue;
                    };
                    let response = query_everything(&mut client, request);
                    let _ = output.send(response);
                }
            })
            .expect("failed to create Everything worker thread");

        Self { latest, wake }
    }

    pub fn request(&self, sequence: u64, query: String) {
        if let Ok(mut latest) = self.latest.lock() {
            *latest = Some(EverythingRequest { sequence, query });
            let _ = self.wake.try_send(());
        }
    }
}

fn query_everything(
    client: &mut Option<EverythingClient>,
    request: EverythingRequest,
) -> EverythingResponse {
    if client.is_none() {
        let connect_started = std::time::Instant::now();
        *client = EverythingClient::new().ok();
        crate::launch::trace_query_profile(
            "everything-connect",
            &format!(
                "{:.1}ms\t{}",
                connect_started.elapsed().as_secs_f64() * 1000.0,
                client.is_some()
            ),
        );
    }

    let Some(ipc_client) = client.as_ref() else {
        return EverythingResponse {
            sequence: request.sequence,
            query: request.query,
            results: Vec::new(),
            status: t!("everything.unavailable").into_owned(),
            available: false,
        };
    };

    let query = request.query.clone();
    let query_started = std::time::Instant::now();
    let list = ipc_client
        .query_wait(&query)
        .request_flags(RequestFlags::FileName | RequestFlags::Path)
        .sort(Sort::DateModifiedDescending)
        .max_results(MAX_RESULTS)
        .timeout(QUERY_TIMEOUT)
        .call();
    crate::launch::trace_query_profile(
        "everything-query",
        &format!(
            "{:.1}ms\t{}",
            query_started.elapsed().as_secs_f64() * 1000.0,
            list.is_ok()
        ),
    );

    match list {
        Ok(list) => {
            let results = list
                .iter()
                .filter_map(|item| {
                    let title = item.get_string(RequestFlags::FileName)?;
                    let folder = item.get_string(RequestFlags::Path).unwrap_or_default();
                    let path = join_everything_path(&folder, &title);
                    let mut result = SearchResult::file(path.clone(), title, folder);
                    if result.kind == ResultKind::Application {
                        if let Some(canonical_id) = canonical_application_id(&path) {
                            result.id = canonical_id;
                        }
                    }
                    Some(result)
                })
                .collect::<Vec<_>>();
            EverythingResponse {
                sequence: request.sequence,
                query: request.query,
                status: t!("everything.result_count", count = results.len()).into_owned(),
                results,
                available: true,
            }
        }
        Err(error) => {
            *client = None;
            EverythingResponse {
                sequence: request.sequence,
                query: request.query,
                results: Vec::new(),
                status: t!("everything.query_failed", error = error).into_owned(),
                available: false,
            }
        }
    }
}

fn join_everything_path(folder: &str, filename: &str) -> String {
    if folder.is_empty() {
        return filename.to_owned();
    }
    if folder.ends_with('\\') {
        return format!("{folder}{filename}");
    }
    format!("{folder}\\{filename}")
}

#[cfg(test)]
mod tests {
    use super::{
        join_everything_path, should_start_everything, startup_args, winget_install_args,
        WINGET_PACKAGE_ID,
    };

    #[test]
    fn startup_uses_official_background_option() {
        assert_eq!(startup_args(), ["-startup"]);
    }

    #[test]
    fn winget_install_uses_exact_official_package_id() {
        assert_eq!(
            winget_install_args(),
            ["install", "-e", "--id", WINGET_PACKAGE_ID]
        );
    }

    #[test]
    fn does_not_start_when_everything_ipc_is_already_available() {
        assert!(!should_start_everything(true, false));
    }

    #[test]
    fn does_not_start_when_everything_process_is_already_running() {
        assert!(!should_start_everything(false, true));
    }

    #[test]
    fn starts_only_when_ipc_and_process_are_both_absent() {
        assert!(should_start_everything(false, false));
    }

    #[test]
    fn joins_windows_folder_and_filename_without_duplicate_separator() {
        assert_eq!(
            join_everything_path(r"C:\Windows", "explorer.exe"),
            r"C:\Windows\explorer.exe"
        );
        assert_eq!(
            join_everything_path(r"C:\Windows\", "explorer.exe"),
            r"C:\Windows\explorer.exe"
        );
    }

    #[test]
    fn status_kind_separates_installation_from_live_ipc_availability() {
        use super::{status_kind, EverythingStatusKind};

        // 未安装：无论 IPC 探测结果如何，都只能报未安装。
        assert_eq!(
            status_kind(false, false, false),
            EverythingStatusKind::NotInstalled
        );
        // 已安装且 IPC 真实可用。
        assert_eq!(
            status_kind(true, true, false),
            EverythingStatusKind::IpcAvailable
        );
        // 已安装但 IPC 不可用：这是「用户退出 Everything」后的真实状态，
        // 不得再显示为可用。
        assert_eq!(
            status_kind(true, false, false),
            EverythingStatusKind::IpcUnavailable
        );
        // 已尝试后台拉起但 IPC 仍未就绪。
        assert_eq!(
            status_kind(true, false, true),
            EverythingStatusKind::IpcUnavailableAfterStart
        );
    }

    #[test]
    fn startup_outcome_reports_truthful_ipc_state_when_process_was_started() {
        use super::{EverythingStartup, EverythingStatusKind, InstallationState};

        let outcome = EverythingStartup::new(
            InstallationState::Installed(std::path::PathBuf::from("Everything.exe")),
            false,
            true,
        );
        assert!(outcome.is_installed());
        assert!(!outcome.ipc_available());
        assert!(outcome.started_now());
        assert_eq!(
            outcome.status_kind(),
            EverythingStatusKind::IpcUnavailableAfterStart
        );

        let available = EverythingStartup::new(
            InstallationState::Installed(std::path::PathBuf::from("Everything.exe")),
            true,
            false,
        );
        assert_eq!(available.status_kind(), EverythingStatusKind::IpcAvailable);
        assert!(available.ipc_available());
        assert!(!available.started_now());
    }
}
