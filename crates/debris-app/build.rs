//! Bakes the logo into the executable as its icon resource.
//!
//! The window icon is not a window property on Windows. GPUI's Windows backend has
//! no icon argument at all — it asks the *executable* for one
//! (`LoadImageW(module, PCWSTR(1), IMAGE_ICON, ..)` in `gpui_windows`), which is why
//! the taskbar, Alt-Tab, and Explorer all show whatever `1 ICON` holds in the
//! resource section. A resource is a link-time thing, so it needs a build script.
//!
//! `embed-resource` is already in the dependency graph through GPUI's own
//! `windows-manifest` feature, so this adds no new crate to build.

fn main() {
    println!("cargo:rerun-if-changed=../../assets/debris.ico");

    // A build script is a host binary, so its own `cfg(target_os)` says nothing about
    // what we are compiling *for*. Ask Cargo instead.
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() != Ok("windows") {
        return;
    }

    let icon = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../assets/debris.ico");
    let icon = icon
        .canonicalize()
        .expect("assets/debris.ico should exist in the source tree");
    let icon = icon.to_string_lossy().replace('\\', "\\\\");

    // `#pragma code_page` because the checkout can live under a path with characters
    // outside the ANSI code page, in which case rc.exe mangles the path and then
    // cannot find the file it was handed.
    let rc = format!("#pragma code_page(65001)\n1 ICON \"{icon}\"\n");

    let out = std::path::PathBuf::from(std::env::var("OUT_DIR").expect("OUT_DIR is always set"));
    let rc_path = out.join("debris.rc");
    std::fs::write(&rc_path, rc).expect("writing the .rc should succeed");

    embed_resource::compile(&rc_path, embed_resource::NONE)
        .manifest_optional()
        .unwrap();
}
