//! Bakes the logo and the version into the executable as a Windows resource.
//!
//! The window icon is not a window property on Windows. GPUI's Windows backend has no
//! icon argument at all — it asks the *executable* for one
//! (`LoadImageW(module, PCWSTR(1), IMAGE_ICON, ..)` in `gpui_windows`), which is why
//! the taskbar, Alt-Tab and Explorer all show whatever `1 ICON` holds in the resource
//! section. A resource is a link-time thing, so it needs a build script.
//!
//! The version comes along for the same reason and from the same place: the release
//! workflow stamps the tag's CalVer into `[workspace.package].version` before it
//! builds, Cargo hands that to the build script as `CARGO_PKG_VERSION`, and the
//! VERSIONINFO block below is what Explorer's Details tab reads. So the tag, the exe's
//! properties, and the string on the status rail are one number.
//!
//! `embed-resource` is already in the dependency graph through GPUI's own
//! `windows-manifest` feature, so this adds no new crate to build.

fn main() {
    println!("cargo:rerun-if-changed=../../assets/debris.ico");
    // Re-stamp the VERSIONINFO when the manifest's version moves.
    println!("cargo:rerun-if-env-changed=CARGO_PKG_VERSION");

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

    let version = std::env::var("CARGO_PKG_VERSION").expect("Cargo always sets the version");
    let mut components = version
        .split('.')
        .map(|part| part.parse::<u16>().unwrap_or(0))
        .chain(std::iter::repeat(0));
    let numeric = format!(
        "{},{},{},{}",
        components.next().unwrap_or(0),
        components.next().unwrap_or(0),
        components.next().unwrap_or(0),
        components.next().unwrap_or(0),
    );

    // `#pragma code_page` because the checkout can live under a path with characters
    // outside the ANSI code page, in which case rc.exe mangles the path and then
    // cannot find the file it was handed.
    let rc = format!(
        r#"#pragma code_page(65001)
1 ICON "{icon}"

1 VERSIONINFO
FILEVERSION {numeric}
PRODUCTVERSION {numeric}
FILEFLAGSMASK 0x3fL
FILEFLAGS 0x0L
FILEOS 0x40004L
FILETYPE 0x1L
FILESUBTYPE 0x0L
BEGIN
    BLOCK "StringFileInfo"
    BEGIN
        BLOCK "040904b0"
        BEGIN
            VALUE "FileDescription", "Windows debloat tool\0"
            VALUE "FileVersion", "{version}\0"
            VALUE "ProductName", "debris\0"
            VALUE "ProductVersion", "{version}\0"
        END
    END
    BLOCK "VarFileInfo"
    BEGIN
        VALUE "Translation", 0x0409, 1200
    END
END
"#
    );

    let out = std::path::PathBuf::from(std::env::var("OUT_DIR").expect("OUT_DIR is always set"));
    let rc_path = out.join("debris.rc");
    std::fs::write(&rc_path, rc).expect("writing the .rc should succeed");

    embed_resource::compile(&rc_path, embed_resource::NONE)
        .manifest_optional()
        .unwrap();
}
