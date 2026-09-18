//! 把 Flux Launcher 的多分辨率图标嵌入可执行文件。
//!
//! 资源管理器为“发送到桌面”这类操作创建的快捷方式，默认从目标可执行文件的嵌入图标
//! 取图标；可执行文件没有图标资源时，快捷方式会显示空白图标。安装器显式引用 `.ico`
//! 只能覆盖它自己创建的开始菜单快捷方式，无法覆盖用户手动创建的快捷方式，因此这里把
//! 图标写入 exe 资源，作为快捷方式图标的唯一真实来源。
//!
//! 只在 Windows 目标上嵌入资源：其他目标直接返回，保持本仓库既有的跨平台编译能力。
//! 注意判断依据是 `CARGO_CFG_TARGET_OS`（目标系统）而不是 `cfg!(windows)`（宿主系统），
//! 否则从非 Windows 宿主交叉编译到 Windows 时会漏嵌图标。

fn main() {
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() != Ok("windows") {
        return;
    }

    let icon = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join("packaging")
        .join("installer")
        .join("flux-launcher.ico");

    println!("cargo:rerun-if-changed={}", icon.display());

    let icon_path = icon
        .to_str()
        .expect("the launcher icon path must be valid UTF-8");

    let mut resource = winresource::WindowsResource::new();
    resource.set_icon(icon_path);

    if let Err(error) = resource.compile() {
        panic!("无法把启动器图标嵌入可执行文件：{error}");
    }
}
