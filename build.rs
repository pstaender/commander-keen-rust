//! Embeds icon.png as the icon of the Windows executables.

fn main() {
    println!("cargo:rerun-if-changed=icon.png");
    println!("cargo:rerun-if-changed=build.rs");
    #[cfg(windows)]
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("windows") {
        windows_icon();
    }
}

/// icon.png wrapped into an .ico (a PNG image entry, Vista and later) and
/// compiled into a resource that is linked into every binary.
#[cfg(windows)]
fn windows_icon() {
    use std::path::PathBuf;
    let png = std::fs::read("icon.png").expect("icon.png");
    // the size from the PNG header (IHDR), 256 is stored as 0
    let w = u32::from_be_bytes(png[16..20].try_into().unwrap());
    let h = u32::from_be_bytes(png[20..24].try_into().unwrap());
    let mut ico = vec![0, 0, 1, 0, 1, 0];
    ico.push(if w >= 256 { 0 } else { w as u8 });
    ico.push(if h >= 256 { 0 } else { h as u8 });
    ico.extend_from_slice(&[0, 0, 1, 0, 32, 0]);
    ico.extend_from_slice(&(png.len() as u32).to_le_bytes());
    ico.extend_from_slice(&22u32.to_le_bytes());
    ico.extend_from_slice(&png);
    let out = PathBuf::from(std::env::var("OUT_DIR").unwrap());
    let ico_path = out.join("keen.ico");
    std::fs::write(&ico_path, ico).unwrap();
    let rc = out.join("keen.rc");
    let ico_name = ico_path.display().to_string().replace('\\', "/");
    std::fs::write(&rc, format!("1 ICON \"{}\"\n", ico_name)).unwrap();
    embed_resource::compile(&rc, embed_resource::NONE).manifest_optional().unwrap();
}
