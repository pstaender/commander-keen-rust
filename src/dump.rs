//! `--dump` support: exports the original graphics as PNG files. This is the
//! starting point for high resolution / 32-bit replacement packs.

use crate::assets::graphics::{Image, TRANSPARENT};
use crate::assets::GameData;
use std::path::Path;

pub const EGA_RGB: [u32; 16] = [
    0x000000, 0x0000aa, 0x00aa00, 0x00aaaa, 0xaa0000, 0xaa00aa, 0xaa5500, 0xaaaaaa, 0x555555, 0x5555ff, 0x55ff55,
    0x55ffff, 0xff5555, 0xff55ff, 0xffff55, 0xffffff,
];

pub fn write_png(path: &Path, w: usize, h: usize, rgba: &[u8]) -> std::io::Result<()> {
    let f = std::fs::File::create(path)?;
    let mut enc = png::Encoder::new(std::io::BufWriter::new(f), w as u32, h as u32);
    enc.set_color(png::ColorType::Rgba);
    enc.set_depth(png::BitDepth::Eight);
    let mut wr = enc.write_header().map_err(std::io::Error::other)?;
    wr.write_image_data(rgba).map_err(std::io::Error::other)?;
    Ok(())
}

pub fn image_rgba(img: &Image) -> Vec<u8> {
    let mut v = Vec::with_capacity(img.pixels.len() * 4);
    for &p in &img.pixels {
        if p == TRANSPARENT {
            v.extend_from_slice(&[0, 0, 0, 0]);
        } else {
            let c = EGA_RGB[(p & 15) as usize];
            v.extend_from_slice(&[(c >> 16) as u8, (c >> 8) as u8, c as u8, 255]);
        }
    }
    v
}

fn save(dir: &Path, name: &str, img: &Image) {
    if img.width == 0 || img.height == 0 {
        return;
    }
    let _ = write_png(&dir.join(name), img.width, img.height, &image_rgba(img));
}

/// Draws a list of images into a labelled-less grid sheet.
fn sheet(imgs: &[&Image], cols: usize, cell: usize) -> Image {
    let rows = (imgs.len() + cols - 1) / cols;
    let mut s = Image::new(cols * cell, rows * cell);
    s.pixels.fill(8);
    for (i, im) in imgs.iter().enumerate() {
        let ox = (i % cols) * cell;
        let oy = (i / cols) * cell;
        for y in 0..im.height.min(cell) {
            for x in 0..im.width.min(cell) {
                let p = im.get(x, y);
                if p != TRANSPARENT {
                    s.pixels[(oy + y) * s.width + ox + x] = p;
                }
            }
        }
    }
    s
}

pub fn dump(data: &GameData, out: &Path) -> std::io::Result<()> {
    let g = &data.gfx;
    for sub in ["pics", "sprites", "tiles", "maps", "tiles/tile16", "tiles/tile16m", "tiles/tile8", "tiles/tile8m"] {
        std::fs::create_dir_all(out.join(sub))?;
    }
    std::fs::write(
        out.join("pack.txt"),
        "# Graphics replacement pack. Scale every PNG by the same integer factor\n\
         # and set it here, e.g. scale=4 for 1280x800. Delete files you don't replace.\n\
         scale=1\n",
    )?;
    for (i, t) in g.tile16.iter().enumerate() {
        if let Some(t) = t {
            save(&out.join("tiles/tile16"), &format!("{:04}.png", i), t);
        }
    }
    for (i, t) in g.tile16m.iter().enumerate() {
        if let Some(t) = t {
            save(&out.join("tiles/tile16m"), &format!("{:04}.png", i), t);
        }
    }
    for (i, t) in g.tile8.iter().enumerate() {
        save(&out.join("tiles/tile8"), &format!("{:03}.png", i), t);
    }
    for (i, t) in g.tile8m.iter().enumerate() {
        save(&out.join("tiles/tile8m"), &format!("{:03}.png", i), t);
    }
    for (i, p) in g.pics.iter().enumerate() {
        save(&out.join("pics"), &format!("pic_{:03}.png", i), p);
    }
    for (i, p) in g.picms.iter().enumerate() {
        save(&out.join("pics"), &format!("picm_{:03}.png", i), p);
    }
    for (i, p) in g.sprites.iter().enumerate() {
        save(&out.join("sprites"), &format!("spr_{:03}.png", i), p);
    }
    let empty = Image::new(16, 16);
    let t16: Vec<&Image> = g.tile16.iter().map(|t| t.as_ref().unwrap_or(&empty)).collect();
    save(&out.join("tiles"), "tile16.png", &sheet(&t16, 18, 16));
    let t16m: Vec<&Image> = g.tile16m.iter().map(|t| t.as_ref().unwrap_or(&empty)).collect();
    save(&out.join("tiles"), "tile16m.png", &sheet(&t16m, 18, 16));
    let t8: Vec<&Image> = g.tile8.iter().collect();
    save(&out.join("tiles"), "tile8.png", &sheet(&t8, 13, 8));
    let t8m: Vec<&Image> = g.tile8m.iter().collect();
    save(&out.join("tiles"), "tile8m.png", &sheet(&t8m, 10, 8));
    // maps: background + foreground
    for (mi, m) in data.maps.maps.iter().enumerate() {
        let Some(m) = m else { continue };
        let mut img = Image::new(m.width * 16, m.height * 16);
        for ty in 0..m.height {
            for tx in 0..m.width {
                let bg = m.planes[0][ty * m.width + tx] as usize;
                let fg = m.planes[1][ty * m.width + tx] as usize;
                for (t, list) in [(bg, &g.tile16), (fg, &g.tile16m)] {
                    if list as *const _ == &g.tile16m as *const _ && t == 0 {
                        continue;
                    }
                    if let Some(Some(tile)) = list.get(t) {
                        for y in 0..16 {
                            for x in 0..16 {
                                let p = tile.get(x, y);
                                if p != TRANSPARENT {
                                    img.pixels[(ty * 16 + y) * img.width + tx * 16 + x] = p;
                                }
                            }
                        }
                    }
                }
            }
        }
        save(&out.join("maps"), &format!("map{:02}_{}.png", mi, m.name.replace(' ', "_")), &img);
        // tile planes as text (width height, then one row per line: bg, fg),
        // used by tools/remaster_gfx.py to find repeating fill patterns
        let mut s = format!("{} {}\n", m.width, m.height);
        for p in 0..2 {
            for row in m.planes[p].chunks(m.width) {
                s += &row.iter().map(|t| t.to_string()).collect::<Vec<_>>().join(" ");
                s += "\n";
            }
        }
        std::fs::write(out.join("maps").join(format!("map{:02}.txt", mi)), s)?;
    }
    for (i, e) in g.externs.iter().enumerate() {
        std::fs::write(out.join(format!("extern_{}.bin", i + data.layout.startexterns())), e)?;
    }
    // sprite table as text
    let mut s = String::new();
    for (i, t) in g.spritetable.iter().enumerate() {
        s += &format!("{:3} chunk {:3}: {:?}\n", i, i + data.layout.startsprites(), t);
    }
    std::fs::write(out.join("spritetable.txt"), s)?;
    Ok(())
}
