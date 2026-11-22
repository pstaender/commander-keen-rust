//! Embeds icon.png as the icon of the Windows executables.

fn main() {
    println!("cargo:rerun-if-changed=icon.png");
    println!("cargo:rerun-if-changed=build.rs");
    #[cfg(windows)]
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("windows") {
        windows_icon();
    }
}

/// icon.png as a 256x256 (at most) RGBA PNG: an .ico entry can't be
/// larger. Larger icons are scaled down by averaging.
#[cfg(windows)]
fn icon_png_256() -> Vec<u8> {
    let data = std::fs::read("icon.png").expect("icon.png");
    let mut r = png::Decoder::new(std::io::Cursor::new(&data)).read_info().expect("icon.png");
    let mut buf = vec![0; r.output_buffer_size()];
    let info = r.next_frame(&mut buf).expect("icon.png");
    let (w, h) = (info.width as usize, info.height as usize);
    let bpp = match info.color_type {
        png::ColorType::Rgba => 4,
        png::ColorType::Rgb => 3,
        _ => panic!("icon.png: needs 8 bit RGB or RGBA"),
    };
    let px = |x: usize, y: usize, c: usize| -> u32 {
        if c == 3 && bpp == 3 {
            255
        } else {
            buf[(y * w + x) * bpp + c] as u32
        }
    };
    let (dw, dh) = (w.min(256), h.min(256));
    let mut out = Vec::with_capacity(dw * dh * 4);
    for y in 0..dh {
        let (y0, y1) = (y * h / dh, ((y + 1) * h / dh).max(y * h / dh + 1));
        for x in 0..dw {
            let (x0, x1) = (x * w / dw, ((x + 1) * w / dw).max(x * w / dw + 1));
            let n = ((x1 - x0) * (y1 - y0)) as u32;
            for c in 0..4 {
                let sum: u32 = (y0..y1).flat_map(|sy| (x0..x1).map(move |sx| (sx, sy))).map(|(sx, sy)| px(sx, sy, c)).sum();
                out.push((sum / n) as u8);
            }
        }
    }
    let mut png_out = Vec::new();
    {
        let mut e = png::Encoder::new(&mut png_out, dw as u32, dh as u32);
        e.set_color(png::ColorType::Rgba);
        e.set_depth(png::BitDepth::Eight);
        e.write_header().unwrap().write_image_data(&out).unwrap();
    }
    png_out
}

/// icon.png wrapped into an .ico (a PNG image entry, Vista and later) and
/// compiled into a resource that is linked into every binary.
#[cfg(windows)]
fn windows_icon() {
    use std::path::PathBuf;
    let png = icon_png_256();
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
