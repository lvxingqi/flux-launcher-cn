//! 临时帧调度诊断（`WINDUI_TRACE_FILE` 环境变量门控；诊断目的见
//! flux-launcher 仓库 `doc/plan/u-key-lag-frame-scheduling-2026-09-18.md`）。
//!
//! 行格式与 flux 的查询性能日志一致：`{unix_ms}\t{event}\t{detail}`。
//! 未设置环境变量时仅一次原子读，零开销；写入端加进程内锁后整行单次写，
//! 避免 UI 线程与 worker 线程并发追加交错出坏行。

use std::fs::OpenOptions;
use std::io::Write;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;

static ENABLED: AtomicBool = AtomicBool::new(false);
static SINK: Mutex<Option<std::fs::File>> = Mutex::new(None);

/// 进程启动时按环境变量初始化；重复调用幂等。
pub(crate) fn init_from_env() {
    if ENABLED.load(Ordering::Relaxed) {
        return;
    }
    let opened = std::env::var("WINDUI_TRACE_FILE")
        .ok()
        .and_then(|path| OpenOptions::new().create(true).append(true).open(path).ok());
    *SINK.lock().unwrap() = opened;
    ENABLED.store(true, Ordering::Relaxed);
}

/// 追加一行诊断（时间戳含小数毫秒）；detail 中的换行压成空格。
pub(crate) fn event(event: &str, detail: &str) {
    if !ENABLED.load(Ordering::Relaxed) {
        return;
    }
    let Ok(mut guard) = SINK.lock() else {
        return;
    };
    let Some(file) = guard.as_mut() else {
        return;
    };
    let unix_ms = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs_f64() * 1000.0)
        .unwrap_or_default();
    let line = format!(
        "{:.3}\t{}\t{}\n",
        unix_ms,
        event,
        detail.replace(['\r', '\n'], " ")
    );
    let _ = file.write_all(line.as_bytes());
}
