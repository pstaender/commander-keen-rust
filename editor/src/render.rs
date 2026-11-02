//! Textures for the editor: the 16x16 tile sets as atlases (18 tiles per
//! row, like the TED5 tile pickers) and the sprites used as info previews.

use eframe::egui::{self, ColorImage, Pos2, Rect, TextureHandle, TextureOptions};
use keen::assets::graphics::{EgaGraph, Image, TRANSPARENT};
use keen::engine::video::EGA_RGB;
use std::collections::HashMap;

pub const TILE: usize = 16;
pub const ATLAS_COLS: usize = 18;

pub fn ega(c: u8) -> [u8; 4] {
    if c == TRANSPARENT {
        return [0, 0, 0, 0];
    }
    let v = EGA_RGB[(c & 15) as usize];
    [(v >> 16) as u8, (v >> 8) as u8, v as u8, 255]
}

/// A tile set packed into one texture.
pub struct Atlas {
    pub tex: TextureHandle,
    pub count: usize,
    pub rows: usize,
    /// tiles that exist (empty chunks are drawn as nothing)
    pub present: Vec<bool>,
}

impl Atlas {
    fn new(ctx: &egui::Context, name: &str, tiles: &[Option<Image>]) -> Atlas {
        let count = tiles.len();
        let rows = count.div_ceil(ATLAS_COLS);
        let (w, h) = (ATLAS_COLS * TILE, rows * TILE);
        let mut rgba = vec![0u8; w * h * 4];
        for (i, t) in tiles.iter().enumerate() {
            let Some(img) = t else { continue };
            let (tx, ty) = ((i % ATLAS_COLS) * TILE, (i / ATLAS_COLS) * TILE);
            for y in 0..TILE.min(img.height) {
                for x in 0..TILE.min(img.width) {
                    let o = ((ty + y) * w + tx + x) * 4;
                    rgba[o..o + 4].copy_from_slice(&ega(img.get(x, y)));
                }
            }
        }
        let tex = ctx.load_texture(name, ColorImage::from_rgba_unmultiplied([w, h], &rgba), TextureOptions::NEAREST);
        Atlas { tex, count, rows, present: tiles.iter().map(Option::is_some).collect() }
    }

    /// Texture coordinates of tile `t`.
    pub fn uv(&self, t: usize) -> Rect {
        let (cx, cy) = ((t % ATLAS_COLS) as f32, (t / ATLAS_COLS) as f32);
        let (w, h) = (ATLAS_COLS as f32, self.rows as f32);
        Rect::from_min_max(Pos2::new(cx / w, cy / h), Pos2::new((cx + 1.0) / w, (cy + 1.0) / h))
    }

    pub fn has(&self, t: usize) -> bool {
        self.present.get(t).copied().unwrap_or(false)
    }
}

pub struct Textures {
    pub bg: Atlas,
    pub fg: Atlas,
    sprites: HashMap<usize, (TextureHandle, [usize; 2])>,
}

impl Textures {
    pub fn new(ctx: &egui::Context, g: &EgaGraph) -> Textures {
        Textures { bg: Atlas::new(ctx, "tile16", &g.tile16), fg: Atlas::new(ctx, "tile16m", &g.tile16m), sprites: HashMap::new() }
    }

    /// Sprite texture (created on first use) and its size in pixels.
    pub fn sprite(&mut self, ctx: &egui::Context, g: &EgaGraph, idx: usize) -> Option<(&TextureHandle, [usize; 2])> {
        if !self.sprites.contains_key(&idx) {
            let img = g.sprites.get(idx)?;
            if img.width == 0 || img.height == 0 {
                return None;
            }
            let rgba: Vec<u8> = img.pixels.iter().flat_map(|&c| ega(c)).collect();
            let size = [img.width, img.height];
            let tex = ctx.load_texture(format!("sprite{}", idx), ColorImage::from_rgba_unmultiplied(size, &rgba), TextureOptions::NEAREST);
            self.sprites.insert(idx, (tex, size));
        }
        self.sprites.get(&idx).map(|(t, s)| (t, *s))
    }
}
