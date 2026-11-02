//! Graphics sets: the original EGA graphics converted to 32-bit textures at
//! the surface scale, optionally overridden by a replacement pack.
//!
//! Pack layout (same names `--dump` produces, every image scaled by the
//! pack's integer factor, which is given in `pack.txt` as `scale=N`):
//!   sprites/spr_NNN.png    pics/pic_NNN.png    pics/picm_NNN.png
//!   tiles/tile16/NNNN.png  tiles/tile16m/NNNN.png
//!   tiles/tile8/NNN.png    tiles/tile8m/NNN.png
//! Missing files fall back to the (upscaled) original graphics.

use super::video::Tex;
use crate::assets::graphics::{EgaGraph, Image};
use std::path::{Path, PathBuf};

pub struct Gfx {
    pub scale: usize,
    pub sprites: Vec<Tex>,
    pub pics: Vec<Tex>,
    pub picms: Vec<Tex>,
    pub tile8: Vec<Tex>,
    pub tile8m: Vec<Tex>,
    pub tile16: Vec<Option<Tex>>,
    pub tile16m: Vec<Option<Tex>>,
}

fn load_png(path: &Path) -> Option<Tex> {
    let f = std::fs::File::open(path).ok()?;
    let mut dec = png::Decoder::new(std::io::BufReader::new(f));
    dec.set_transformations(png::Transformations::EXPAND | png::Transformations::STRIP_16);
    let mut r = dec.read_info().ok()?;
    let mut buf = vec![0; r.output_buffer_size()];
    let info = r.next_frame(&mut buf).ok()?;
    let (w, h) = (info.width as usize, info.height as usize);
    let rgba: Vec<u8> = match info.color_type {
        png::ColorType::Rgba => buf[..w * h * 4].to_vec(),
        png::ColorType::Rgb => buf[..w * h * 3].chunks(3).flat_map(|c| [c[0], c[1], c[2], 255]).collect(),
        png::ColorType::GrayscaleAlpha => buf[..w * h * 2].chunks(2).flat_map(|c| [c[0], c[0], c[0], c[1]]).collect(),
        png::ColorType::Grayscale => buf[..w * h].iter().flat_map(|&c| [c, c, c, 255]).collect(),
        _ => return None,
    };
    Some(Tex::from_rgba(w, h, &rgba))
}

pub fn pack_scale(dir: &Path) -> usize {
    std::fs::read_to_string(dir.join("pack.txt"))
        .ok()
        .and_then(|s| {
            s.lines().find_map(|l| l.trim().strip_prefix("scale=").and_then(|v| v.trim().parse().ok()))
        })
        .unwrap_or(1)
        .clamp(1, 16)
}

impl Gfx {
    pub fn new(g: &EgaGraph, scale: usize, pack: Option<&Path>) -> Gfx {
        let pack: Option<PathBuf> = pack.map(|p| p.to_path_buf());
        let conv = |img: &Image, rel: String| -> Tex {
            if let Some(p) = &pack {
                if let Some(t) = load_png(&p.join(&rel)) {
                    if t.w == img.width * scale && t.h == img.height * scale {
                        return t;
                    }
                    eprintln!("pack: {} has wrong size {}x{}, expected {}x{}", rel, t.w, t.h, img.width * scale, img.height * scale);
                }
            }
            Tex::from_image(img, scale)
        };
        Gfx {
            scale,
            sprites: g.sprites.iter().enumerate().map(|(i, s)| conv(s, format!("sprites/spr_{:03}.png", i))).collect(),
            pics: g.pics.iter().enumerate().map(|(i, s)| conv(s, format!("pics/pic_{:03}.png", i))).collect(),
            picms: g.picms.iter().enumerate().map(|(i, s)| conv(s, format!("pics/picm_{:03}.png", i))).collect(),
            tile8: g.tile8.iter().enumerate().map(|(i, s)| conv(s, format!("tiles/tile8/{:03}.png", i))).collect(),
            tile8m: g.tile8m.iter().enumerate().map(|(i, s)| conv(s, format!("tiles/tile8m/{:03}.png", i))).collect(),
            tile16: g
                .tile16
                .iter()
                .enumerate()
                .map(|(i, s)| s.as_ref().map(|s| conv(s, format!("tiles/tile16/{:04}.png", i))))
                .collect(),
            tile16m: g
                .tile16m
                .iter()
                .enumerate()
                .map(|(i, s)| s.as_ref().map(|s| conv(s, format!("tiles/tile16m/{:04}.png", i))))
                .collect(),
        }
    }
}
