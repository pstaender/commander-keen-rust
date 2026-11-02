//! 32-bit software renderer.
//!
//! All game code works in the original 320x200 coordinate system. The
//! surface has an integer `scale` factor: with the original EGA graphics it
//! is 1, with a high resolution replacement pack (see `gfxpack`) it is the
//! pack's scale, and every blit lands at `x*scale, y*scale`. Colours are
//! 0xAARRGGBB; EGA indices are converted through `EGA_RGB`.

use crate::assets::graphics::{Font, Image, TRANSPARENT};

pub const SCREEN_W: usize = 320;
pub const SCREEN_H: usize = 200;

pub const EGA_RGB: [u32; 16] = [
    0x000000, 0x0000aa, 0x00aa00, 0x00aaaa, 0xaa0000, 0xaa00aa, 0xaa5500, 0xaaaaaa, 0x555555, 0x5555ff, 0x55ff55,
    0x55ffff, 0xff5555, 0xff55ff, 0xffff55, 0xffffff,
];

/// Converts an EGA 6-bit palette register value (rgbRGB) to RGB.
pub fn ega64_rgb(v: u8) -> u32 {
    let c = |hi: u8, lo: u8| -> u32 { ((v >> hi) & 1) as u32 * 0xaa + ((v >> lo) & 1) as u32 * 0x55 };
    (c(2, 5) << 16) | (c(1, 4) << 8) | c(0, 3)
}

/// A texture at the surface scale. Alpha 0 = transparent.
#[derive(Clone, Default)]
pub struct Tex {
    pub w: usize,
    pub h: usize,
    pub px: Vec<u32>,
    /// true if any pixel is transparent (fast path for opaque tiles)
    pub has_alpha: bool,
}

impl Tex {
    pub fn from_image(img: &Image, scale: usize) -> Tex {
        let w = img.width * scale;
        let h = img.height * scale;
        let mut px = vec![0u32; w * h];
        let mut has_alpha = false;
        for y in 0..h {
            for x in 0..w {
                let p = img.pixels[(y / scale) * img.width + x / scale];
                px[y * w + x] = if p == TRANSPARENT {
                    has_alpha = true;
                    0
                } else {
                    0xff00_0000 | EGA_RGB[(p & 15) as usize]
                };
            }
        }
        Tex { w, h, px, has_alpha }
    }

    pub fn from_rgba(w: usize, h: usize, rgba: &[u8]) -> Tex {
        let mut px = Vec::with_capacity(w * h);
        let mut has_alpha = false;
        for c in rgba.chunks_exact(4) {
            if c[3] < 128 {
                has_alpha = true;
                px.push(0);
            } else {
                px.push(0xff00_0000 | (c[0] as u32) << 16 | (c[1] as u32) << 8 | c[2] as u32);
            }
        }
        Tex { w, h, px, has_alpha }
    }
}

/// The screen: logical 320x200 at `scale`.
pub struct Surface {
    pub scale: usize,
    pub w: usize,
    pub h: usize,
    pub px: Vec<u32>,
    /// logical x offset added by all logical-coordinate drawing: the
    /// 320x200 screens and windows are centered in a wider view, the play
    /// field and the score box draw at 0
    pub xoff: i32,
    /// clip rectangle in physical pixels
    clip: (i32, i32, i32, i32),
}

impl Surface {
    pub fn new(scale: usize) -> Surface {
        let w = SCREEN_W * scale;
        let h = SCREEN_H * scale;
        Surface { scale, w, h, px: vec![0xff000000; w * h], xoff: 0, clip: (0, 0, w as i32, h as i32) }
    }

    /// Logical width (320 for the original view, wider for widescreen).
    pub fn logical_w(&self) -> usize {
        self.w / self.scale
    }

    /// Changes the logical width; the content is cleared and 320 wide
    /// drawing gets centered.
    pub fn set_width(&mut self, lw: usize) {
        self.w = lw * self.scale;
        self.px = vec![0xff000000; self.w * self.h];
        self.xoff = (lw as i32 - SCREEN_W as i32) / 2;
        self.reset_clip();
    }

    /// Restricts drawing to a logical rectangle.
    pub fn set_clip(&mut self, x: i32, y: i32, w: i32, h: i32) {
        let s = self.scale as i32;
        let x = x + self.xoff;
        let x0 = (x * s).clamp(0, self.w as i32);
        let y0 = (y * s).clamp(0, self.h as i32);
        let x1 = ((x + w) * s).clamp(0, self.w as i32);
        let y1 = ((y + h) * s).clamp(0, self.h as i32);
        self.clip = (x0, y0, x1, y1);
    }

    pub fn reset_clip(&mut self) {
        self.clip = (0, 0, self.w as i32, self.h as i32);
    }

    pub fn clear(&mut self, color: u8) {
        let c = 0xff000000 | EGA_RGB[(color & 15) as usize];
        self.px.fill(c);
    }

    /// Fills a logical rectangle with an EGA colour.
    pub fn bar(&mut self, x: i32, y: i32, w: i32, h: i32, color: u8) {
        self.bar_rgb(x, y, w, h, 0xff000000 | EGA_RGB[(color & 15) as usize]);
    }

    pub fn bar_rgb(&mut self, x: i32, y: i32, w: i32, h: i32, c: u32) {
        let s = self.scale as i32;
        let x = x + self.xoff;
        let (cx0, cy0, cx1, cy1) = self.clip;
        let x0 = (x * s).max(cx0);
        let y0 = (y * s).max(cy0);
        let x1 = ((x + w) * s).min(cx1);
        let y1 = ((y + h) * s).min(cy1);
        if x0 >= x1 || y0 >= y1 {
            return;
        }
        for yy in y0..y1 {
            let row = yy as usize * self.w;
            self.px[row + x0 as usize..row + x1 as usize].fill(c);
        }
    }

    /// Draws a texture with its top-left corner at logical (x, y).
    pub fn blit(&mut self, t: &Tex, x: i32, y: i32) {
        let s = self.scale as i32;
        self.blit_phys(t, (x + self.xoff) * s, y * s);
    }

    /// Draws a texture at physical coordinates (used for sub-pixel
    /// positioning of high resolution sprites).
    pub fn blit_phys(&mut self, t: &Tex, px: i32, py: i32) {
        let (cx0, cy0, cx1, cy1) = self.clip;
        let x0 = px.max(cx0);
        let y0 = py.max(cy0);
        let x1 = (px + t.w as i32).min(cx1);
        let y1 = (py + t.h as i32).min(cy1);
        if x0 >= x1 || y0 >= y1 {
            return;
        }
        let tw = t.w;
        for yy in y0..y1 {
            let srow = (yy - py) as usize * tw;
            let drow = yy as usize * self.w;
            let sx0 = (x0 - px) as usize;
            let n = (x1 - x0) as usize;
            let src = &t.px[srow + sx0..srow + sx0 + n];
            let dst = &mut self.px[drow + x0 as usize..drow + x0 as usize + n];
            if t.has_alpha {
                for (d, &s) in dst.iter_mut().zip(src) {
                    if s != 0 {
                        *d = s;
                    }
                }
            } else {
                dst.copy_from_slice(src);
            }
        }
    }

    /// Draws the opaque pixels of a texture in a single colour.
    pub fn blit_phys_solid(&mut self, t: &Tex, px: i32, py: i32, color: u32) {
        let (cx0, cy0, cx1, cy1) = self.clip;
        for y in py.max(cy0)..(py + t.h as i32).min(cy1) {
            for x in px.max(cx0)..(px + t.w as i32).min(cx1) {
                if t.px[(y - py) as usize * t.w + (x - px) as usize] != 0 {
                    self.px[y as usize * self.w + x as usize] = color;
                }
            }
        }
    }

    /// Draws one glyph of an original font in the given EGA colour.
    pub fn glyph(&mut self, font: &Font, ch: u8, x: i32, y: i32, color: u8) -> i32 {
        let w = font.widths.get(ch as usize).copied().unwrap_or(0) as i32;
        let h = font.height as i32;
        if w == 0 {
            return 0;
        }
        let g = &font.glyphs[ch as usize];
        let c = 0xff000000 | EGA_RGB[(color & 15) as usize];
        let s = self.scale as i32;
        let (cx0, cy0, cx1, cy1) = self.clip;
        for gy in 0..h {
            for gx in 0..w {
                if g[(gy * w + gx) as usize] == 0 {
                    continue;
                }
                for sy in 0..s {
                    let yy = (y + gy) * s + sy;
                    if yy < cy0 || yy >= cy1 {
                        continue;
                    }
                    for sx in 0..s {
                        let xx = (x + self.xoff + gx) * s + sx;
                        if xx < cx0 || xx >= cx1 {
                            continue;
                        }
                        self.px[yy as usize * self.w + xx as usize] = c;
                    }
                }
            }
        }
        w
    }

    /// Draws one glyph XORing the EGA colour index (ID_VW pdrawmode XOR).
    pub fn glyph_xor(&mut self, font: &Font, ch: u8, x: i32, y: i32, color: u8) -> i32 {
        let w = font.widths.get(ch as usize).copied().unwrap_or(0) as i32;
        let h = font.height as i32;
        if w == 0 {
            return 0;
        }
        let g = &font.glyphs[ch as usize];
        let s = self.scale as i32;
        let (cx0, cy0, cx1, cy1) = self.clip;
        for gy in 0..h {
            for gx in 0..w {
                if g[(gy * w + gx) as usize] == 0 {
                    continue;
                }
                for sy in 0..s {
                    let yy = (y + gy) * s + sy;
                    if yy < cy0 || yy >= cy1 {
                        continue;
                    }
                    for sx in 0..s {
                        let xx = (x + self.xoff + gx) * s + sx;
                        if xx < cx0 || xx >= cx1 {
                            continue;
                        }
                        let p = &mut self.px[yy as usize * self.w + xx as usize];
                        let rgb = *p & 0xffffff;
                        *p = 0xff000000
                            | match EGA_RGB.iter().position(|&c| c == rgb) {
                                Some(i) => EGA_RGB[(i as u8 ^ (color & 15)) as usize],
                                None => {
                                    if color & 15 != 0 {
                                        !rgb & 0xffffff
                                    } else {
                                        rgb
                                    }
                                }
                            };
                    }
                }
            }
        }
        w
    }

    /// Draws a string in a proportional font, returns width.
    pub fn print(&mut self, font: &Font, s: &[u8], x: i32, y: i32, color: u8) -> i32 {
        let mut cx = x;
        for &c in s {
            cx += self.glyph(font, c, cx, y, color);
        }
        cx - x
    }

    /// XORs a logical rectangle (used for blinking cursors).
    pub fn xor_bar(&mut self, x: i32, y: i32, w: i32, h: i32, color: u8) {
        let s = self.scale as i32;
        let c = EGA_RGB[(color & 15) as usize];
        let x = x + self.xoff;
        for yy in (y * s).max(0)..((y + h) * s).min(self.h as i32) {
            for xx in (x * s).max(0)..((x + w) * s).min(self.w as i32) {
                let p = &mut self.px[yy as usize * self.w + xx as usize];
                *p = 0xff000000 | ((*p ^ c) & 0xffffff);
            }
        }
    }

    /// Copies a logical rectangle from another surface of the same scale.
    pub fn copy_from(&mut self, other: &Surface, x: i32, y: i32, w: i32, h: i32) {
        let s = self.scale as i32;
        let x = x + self.xoff;
        for yy in (y * s).max(0)..((y + h) * s).min(self.h as i32) {
            let row = yy as usize * self.w;
            let x0 = (x * s).max(0) as usize;
            let x1 = ((x + w) * s).min(self.w as i32) as usize;
            if x0 < x1 {
                self.px[row + x0..row + x1].copy_from_slice(&other.px[row + x0..row + x1]);
            }
        }
    }
}

pub fn string_width(font: &Font, s: &[u8]) -> i32 {
    s.iter().map(|&c| font.widths.get(c as usize).copied().unwrap_or(0) as i32).sum()
}

/// Palette state used for EGA style fades. `regs` holds the 16 EGA palette
/// register values plus the border colour (like ID_VW `colors[]`).
#[derive(Clone)]
pub struct Palette {
    pub regs: [u8; 17],
}

pub const DEFAULT_PALETTE: [u8; 17] = [0, 1, 2, 3, 4, 5, 6, 7, 0x18, 0x19, 0x1a, 0x1b, 0x1c, 0x1d, 0x1e, 0x1f, 0];

/// VW fade steps (ID_VW colors[7][17], read from the executable data).
pub const FADE_STEPS: [[u8; 17]; 6] = [
    [0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0],
    [0, 0, 0, 0, 0, 0, 0, 0, 0, 1, 2, 3, 4, 5, 6, 7, 0],
    [0, 0, 0, 0, 0, 0, 0, 0, 24, 25, 26, 27, 28, 29, 30, 31, 0],
    [0, 1, 2, 3, 4, 5, 6, 7, 24, 25, 26, 27, 28, 29, 30, 31, 0],
    [0, 1, 2, 3, 4, 5, 6, 7, 31, 31, 31, 31, 31, 31, 31, 31, 0],
    [31, 31, 31, 31, 31, 31, 31, 31, 31, 31, 31, 31, 31, 31, 31, 31, 31],
];

impl Default for Palette {
    fn default() -> Self {
        Palette { regs: DEFAULT_PALETTE }
    }
}

impl Palette {
    pub fn is_default(&self) -> bool {
        self.regs[..16] == DEFAULT_PALETTE[..16]
    }

    /// Builds a mapping for the 16 standard colours and a brightness factor
    /// (0..=256, 256 = unchanged, >256 brighter) for other (high colour)
    /// pixels, approximating the palette effect.
    pub fn mapping(&self) -> ([u32; 16], i32) {
        let mut map = [0u32; 16];
        let mut lum_src = 0i32;
        let mut lum_dst = 0i32;
        for i in 0..16 {
            map[i] = ega64_rgb(self.regs[i]);
            let l = |c: u32| ((c >> 16) & 255) as i32 + ((c >> 8) & 255) as i32 + (c & 255) as i32;
            lum_src += l(EGA_RGB[i]);
            lum_dst += l(map[i]);
        }
        let factor = if lum_src == 0 { 256 } else { lum_dst * 256 / lum_src };
        (map, factor)
    }
}
