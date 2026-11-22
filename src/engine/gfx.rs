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
    /// two player mode: the sprites in player 2's colors, made when first
    /// drawn (`alt_sprite`)
    pub sprites_alt: Vec<Option<Tex>>,
}

/// Player 2's colors: Keen's yellow helmet turns blue with red stripes
/// (green before), his pink shirt green and his cyan trousers gray.
/// (EGA color, replacement) pairs, applied in one pass; a pixel near one
/// of the EGA colors is moved by the same difference, so the shading of
/// high resolution packs survives.
const ALT_COLORS: [(u32, u32); 5] = [
    (0xffff55, 0x5555ff),
    (0x00aa00, 0xaa0000),
    (0xff55ff, 0x55ff55),
    (0xaa00aa, 0x00aa00),
    (0x55ffff, 0xaaaaaa),
];

fn alt_color(p: u32) -> u32 {
    if p >> 24 == 0 {
        return p;
    }
    let ch = |c: u32, s: u32| ((c >> s) & 0xff) as i32;
    let mut best: Option<(i32, u32, u32)> = None;
    for &(from, to) in &ALT_COLORS {
        let d: i32 = [16, 8, 0].iter().map(|&s| (ch(p, s) - ch(from, s)).pow(2)).sum();
        if d < 48 * 48 && best.is_none_or(|b| d < b.0) {
            best = Some((d, from, to));
        }
    }
    let Some((_, from, to)) = best else { return p };
    let mut out = p & 0xff00_0000;
    for s in [16, 8, 0] {
        out |= ((ch(p, s) + ch(to, s) - ch(from, s)).clamp(0, 255) as u32) << s;
    }
    out
}

impl Gfx {
    /// Sprite `idx` in player 2's colors.
    pub fn alt_sprite(&mut self, idx: usize) -> &Tex {
        if self.sprites_alt.len() < self.sprites.len() {
            self.sprites_alt.resize(self.sprites.len(), None);
        }
        let src = &self.sprites[idx];
        self.sprites_alt[idx].get_or_insert_with(|| Tex {
            w: src.w,
            h: src.h,
            px: src.px.iter().map(|&p| alt_color(p)).collect(),
            has_alpha: src.has_alpha,
        })
    }
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
            sprites_alt: Vec::new(),
        }
    }
}
