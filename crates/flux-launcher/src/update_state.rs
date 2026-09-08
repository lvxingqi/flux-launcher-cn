use std::cell::Cell;
use std::rc::Rc;
use std::sync::{Arc, RwLock};

use flux_core::Settings;
use windui::app::App;
use windui::prelude::{Sender, Signal};

pub(crate) struct UpdateRuntimeState {
    pub(crate) install_in_flight: Rc<Cell<bool>>,
    pub(crate) check_in_flight: Rc<Cell<bool>>,
}

pub(crate) struct UpdateChannels {
    pub(crate) install_sender: Sender<UpdateInstallResponse>,
    pub(crate) update_sender: Sender<crate::updater::UpdateCheckResponse>,
    pub(crate) install_in_flight: Rc<Cell<bool>>,
    pub(crate) check_in_flight: Rc<Cell<bool>>,
}

pub(crate) fn register_update_channels(
    app: &mut App,
    shared_settings: Arc<RwLock<Settings>>,
    update_status: Signal<String>,
    update_available: Signal<Option<crate::updater::StableUpdate>>,
    update_install_progress: Signal<Option<(String, crate::updater::DownloadProgress)>>,
    update_installing: Signal<bool>,
) -> UpdateChannels {
    let runtime = UpdateRuntimeState::new();
    let install_in_flight = runtime.install_in_flight;
    let check_in_flight = runtime.check_in_flight;
    let install_in_flight_for_channel = Rc::clone(&install_in_flight);
    let install_sender = app.channel::<UpdateInstallResponse>(move |ctx, response| {
        match apply_install_response(
            response,
            &install_in_flight_for_channel,
            update_installing,
            update_install_progress,
            update_status,
        ) {
            UpdateInstallUiAction::None => {}
            UpdateInstallUiAction::Toast(message) => ctx.toast_ok(message),
            UpdateInstallUiAction::ToastAndQuit(message) => {
                ctx.toast_ok(message);
                ctx.quit();
            }
        }
    });
    let install_sender_for_check = install_sender.clone();
    let check_in_flight_for_channel = Rc::clone(&check_in_flight);
    let install_in_flight_for_check = Rc::clone(&install_in_flight);
    let update_sender = app.channel::<crate::updater::UpdateCheckResponse>(move |ctx, response| {
        check_in_flight_for_channel.set(false);
        if let Ok(mut settings) = shared_settings.write() {
            settings.last_update_check_unix = response.checked_at;
            let _ = crate::settings_state::save_settings(&settings);
        }
        match response.result {
            Ok(Some(update)) => {
                let message = t!("updater.available", version = update.version).into_owned();
                update_status.set(message.clone());
                update_available.set(Some(update.clone()));
                let auto_install = shared_settings
                    .read()
                    .map(|settings| settings.auto_install_updates)
                    .unwrap_or(false);
                if auto_install {
                    let relaunch_mode = crate::relaunch_mode_for_auto_install();
                    update_installing.set(true);
                    update_status
                        .set(t!("updater.preparing", version = update.version).into_owned());
                    if !request_update_install(
                        update,
                        install_sender_for_check.clone(),
                        &install_in_flight_for_check,
                        relaunch_mode,
                    ) {
                        update_installing.set(false);
                        update_status.set(t!("updater.already_installing").into_owned());
                    }
                } else {
                    ctx.toast_ok(message);
                }
            }
            Ok(None) => {
                update_available.set(None);
                update_status
                    .set(t!("updater.up_to_date", version = crate::CURRENT_VERSION).into_owned());
            }
            Err(error) => {
                update_status.set(t!("updater.check_failed", error = error).into_owned());
            }
        }
    });
    UpdateChannels {
        install_sender,
        update_sender,
        install_in_flight,
        check_in_flight,
    }
}

impl UpdateRuntimeState {
    pub(crate) fn new() -> Self {
        Self {
            install_in_flight: Rc::new(Cell::new(false)),
            check_in_flight: Rc::new(Cell::new(false)),
        }
    }
}

#[derive(Clone, Debug)]
pub(crate) enum UpdateInstallResponse {
    Progress {
        version: String,
        progress: crate::updater::DownloadProgress,
    },
    Started {
        version: String,
    },
    Failed {
        version: String,
        error: String,
    },
}

pub(crate) enum UpdateInstallUiAction {
    None,
    Toast(String),
    ToastAndQuit(String),
}

pub(crate) fn apply_install_response(
    response: UpdateInstallResponse,
    install_in_flight: &Cell<bool>,
    installing: Signal<bool>,
    progress: Signal<Option<(String, crate::updater::DownloadProgress)>>,
    status: Signal<String>,
) -> UpdateInstallUiAction {
    match response {
        UpdateInstallResponse::Progress {
            version,
            progress: value,
        } => {
            progress.set(Some((version.clone(), value.clone())));
            status.set(format_update_progress(&version, &value));
            UpdateInstallUiAction::None
        }
        UpdateInstallResponse::Started { version } => {
            install_in_flight.set(false);
            installing.set(false);
            progress.set(None);
            status.set(t!("updater.installing_restarting", version = version).into_owned());
            UpdateInstallUiAction::ToastAndQuit(
                t!("updater.installing", version = version).into_owned(),
            )
        }
        UpdateInstallResponse::Failed { version, error } => {
            install_in_flight.set(false);
            installing.set(false);
            progress.set(None);
            status.set(t!("updater.install_failed", version = version, error = error).into_owned());
            UpdateInstallUiAction::Toast(
                t!("updater.install_failed_toast", error = error).into_owned(),
            )
        }
    }
}

pub(crate) fn request_update_check(
    sender: Sender<crate::updater::UpdateCheckResponse>,
    in_flight: &Cell<bool>,
) -> bool {
    if in_flight.replace(true) {
        return false;
    }
    spawn_update_check(sender);
    true
}

fn spawn_update_check(sender: Sender<crate::updater::UpdateCheckResponse>) {
    let _ = std::thread::Builder::new()
        .name(String::from("flux-update-check"))
        .spawn(move || {
            let checked_at = crate::updater::unix_now();
            let result = crate::updater::check_stable(crate::CURRENT_VERSION);
            let _ = sender.send(crate::updater::UpdateCheckResponse { checked_at, result });
        });
}

pub(crate) fn request_update_install(
    update: crate::updater::StableUpdate,
    sender: Sender<UpdateInstallResponse>,
    in_flight: &Cell<bool>,
    relaunch_mode: crate::updater::RelaunchMode,
) -> bool {
    if in_flight.replace(true) {
        return false;
    }
    spawn_update_install(update, sender, relaunch_mode);
    true
}

fn spawn_update_install(
    update: crate::updater::StableUpdate,
    sender: Sender<UpdateInstallResponse>,
    relaunch_mode: crate::updater::RelaunchMode,
) {
    let _ = std::thread::Builder::new()
        .name(String::from("flux-update-install"))
        .spawn(move || {
            let version = update.version.to_string();
            trace_update_event(&format!("update-install-start\\t{version}"));
            let installer_path =
                std::env::temp_dir().join(format!("FluxLauncher-update-{}.exe", update.version));
            let version_for_progress = version.clone();
            let progress_sender = sender.clone();
            let download = crate::updater::download_installer_to_path(
                &update,
                &installer_path,
                move |progress| {
                    trace_update_event(&format!(
                        "update-progress\t{}\t{}\t{:?}",
                        version_for_progress, progress.received_bytes, progress.total_bytes
                    ));
                    let _ = progress_sender.send(UpdateInstallResponse::Progress {
                        version: version_for_progress.clone(),
                        progress,
                    });
                },
            );
            match download {
                Ok(_) => match crate::updater::handoff_installer(&installer_path, relaunch_mode) {
                    Ok(()) => {
                        trace_update_event(&format!("update-installer-started\\t{version}"));
                        let _ = sender.send(UpdateInstallResponse::Started { version });
                    }
                    Err(error) => {
                        trace_update_event(&format!("update-failed\\t{version}\\t{error}"));
                        let _ = std::fs::remove_file(&installer_path);
                        let _ = sender.send(UpdateInstallResponse::Failed { version, error });
                    }
                },
                Err(error) => {
                    trace_update_event(&format!("update-failed\\t{version}\\t{error}"));
                    let _ = sender.send(UpdateInstallResponse::Failed { version, error });
                }
            }
        });
}

pub(crate) fn format_bytes(bytes: u64) -> String {
    const KIB: u64 = 1024;
    const MIB: u64 = KIB * 1024;
    if bytes >= MIB {
        format!("{:.1} MiB", bytes as f64 / MIB as f64)
    } else if bytes >= KIB {
        format!("{:.0} KiB", bytes as f64 / KIB as f64)
    } else {
        format!("{bytes} B")
    }
}

fn trace_update_event(event: &str) {
    let Some(path) = std::env::var_os("FLUX_UPDATE_TRACE_FILE") else {
        return;
    };
    if let Ok(mut file) = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
    {
        use std::io::Write as _;
        let _ = writeln!(file, "{event}");
    }
}

pub(crate) fn update_check_due(settings: &Settings) -> bool {
    let forced = std::env::var("FLUX_FORCE_UPDATE_CHECK")
        .map(|value| value == "1")
        .unwrap_or(false);
    forced
        || crate::updater::should_check(
            crate::updater::unix_now(),
            settings.last_update_check_unix,
            settings.update_interval_hours,
        )
}

pub(crate) fn format_update_progress(
    version: &str,
    progress: &crate::updater::DownloadProgress,
) -> String {
    match progress.total_bytes.filter(|total| *total > 0) {
        Some(total) => {
            let received = progress.received_bytes.min(total);
            let percent = received.saturating_mul(100) / total;
            let remaining = total.saturating_sub(received);
            format!(
                "Downloading stable {version}: {percent}% — {} / {} ({} remaining)",
                format_bytes(received),
                format_bytes(total),
                format_bytes(remaining)
            )
        }
        None => format!(
            "Downloading stable {version}: {} received",
            format_bytes(progress.received_bytes)
        ),
    }
}
