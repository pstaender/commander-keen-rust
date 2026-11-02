//! Decoding of EGAGRAPH.CKn: planar EGA graphics into 8-bit indexed images.
//! Pixel values 0..15 are EGA colour indices; `TRANSPARENT` marks masked-out
//! pixels of sprites, masked pictures and masked tiles.

use super::episode::Layout;
use super::huffman::Huffman;

pub const TRANSPARENT: u8 = 0xff;

#[derive(Clone, Default)]
pub struct Image {
    pub width: usize,
    pub height: usize,
    pub pixels: Vec<u8>,
}

impl Image {
    pub fn new(width: usize, height: usize) -> Self {
        Image { width, height, pixels: vec![0; width * height] }
    }
    #[inline]
    pub fn get(&self, x: usize, y: usize) -> u8 {
        self.pixels[y * self.width + x]
    }
}

#[derive(Clone, Copy, Default, Debug)]
pub struct SpriteInfo {
    /// width in bytes (8 pixel units) and height in pixels
    pub width: i16,
    pub height: i16,
    /// origin and hit box, in global units (1/16 pixel)
    pub orgx: i16,
    pub orgy: i16,
    pub xl: i16,
    pub yl: i16,
    pub xh: i16,
    pub yh: i16,
    pub shifts: i16,
}

#[derive(Clone, Default)]
pub struct Font {
    pub height: usize,
    pub widths: Vec<u8>,
    /// glyph bitmaps, 1 = pixel set; each glyph is width*height
    pub glyphs: Vec<Vec<u8>>,
}

pub struct EgaGraph {
    pub layout: &'static Layout,
    pub pictable: Vec<(usize, usize)>,
    pub picmtable: Vec<(usize, usize)>,
    pub spritetable: Vec<SpriteInfo>,
    pub fonts: Vec<Font>,
    pub pics: Vec<Image>,
    pub picms: Vec<Image>,
    pub sprites: Vec<Image>,
    pub tile8: Vec<Image>,
    pub tile8m: Vec<Image>,
    pub tile16: Vec<Option<Image>>,
    pub tile16m: Vec<Option<Image>>,
    pub externs: Vec<Vec<u8>>,
}

fn le16(d: &[u8], o: usize) -> i16 {
    i16::from_le_bytes([d[o], d[o + 1]])
}

/// Decodes `planes` consecutive bit planes of `wbytes*h` bytes each.
/// If `masked`, the first plane is the mask (1 = transparent).
fn planar(data: &[u8], wbytes: usize, h: usize, masked: bool) -> Image {
    let w = wbytes * 8;
    let plane = wbytes * h;
    let mut img = Image::new(w, h);
    let base = if masked { plane } else { 0 };
    for y in 0..h {
        for xb in 0..wbytes {
            let o = y * wbytes + xb;
            let p0 = *data.get(base + o).unwrap_or(&0);
            let p1 = *data.get(base + plane + o).unwrap_or(&0);
            let p2 = *data.get(base + plane * 2 + o).unwrap_or(&0);
            let p3 = *data.get(base + plane * 3 + o).unwrap_or(&0);
            let m = if masked { *data.get(o).unwrap_or(&0) } else { 0 };
            for bit in 0..8 {
                let s = 7 - bit;
                let c = ((p0 >> s) & 1) | (((p1 >> s) & 1) << 1) | (((p2 >> s) & 1) << 2) | (((p3 >> s) & 1) << 3);
                let transparent = masked && (m >> s) & 1 == 1;
                img.pixels[y * w + xb * 8 + bit] = if transparent { TRANSPARENT } else { c };
            }
        }
    }
    img
}

/// Tiles store each plane as consecutive rows of the whole tile.
fn tile(data: &[u8], size: usize, masked: bool) -> Image {
    planar(data, size / 8, size, masked)
}

impl EgaGraph {
    pub fn load(lay: &'static Layout, file: &[u8], head: &[u32], dict: &[u8]) -> EgaGraph {
        let huff = Huffman::new(dict);
        let chunk_pos = |c: usize| -> Option<usize> {
            let o = head[c];
            if o == 0xffffff || o as usize >= file.len() {
                None
            } else {
                Some(o as usize)
            }
        };
        // standard chunk: 4-byte expanded length, then huffman data
        let chunk = |c: usize| -> Option<Vec<u8>> {
            let o = chunk_pos(c)?;
            let len = u32::from_le_bytes(file[o..o + 4].try_into().unwrap()) as usize;
            Some(huff.expand(&file[o + 4..], len))
        };
        let fixed = |c: usize, len: usize| -> Option<Vec<u8>> {
            let o = chunk_pos(c)?;
            Some(huff.expand(&file[o..], len))
        };

        let pt = chunk(0).unwrap_or_default();
        let pictable: Vec<(usize, usize)> = (0..lay.numpics)
            .map(|i| (le16(&pt, i * 4) as usize, le16(&pt, i * 4 + 2) as usize))
            .collect();
        let pmt = chunk(1).unwrap_or_default();
        let picmtable: Vec<(usize, usize)> = (0..lay.numpicm)
            .map(|i| (le16(&pmt, i * 4) as usize, le16(&pmt, i * 4 + 2) as usize))
            .collect();
        let st = chunk(2).unwrap_or_default();
        let spritetable: Vec<SpriteInfo> = (0..lay.numsprites)
            .map(|i| {
                let o = i * 18;
                SpriteInfo {
                    width: le16(&st, o),
                    height: le16(&st, o + 2),
                    orgx: le16(&st, o + 4),
                    orgy: le16(&st, o + 6),
                    xl: le16(&st, o + 8),
                    yl: le16(&st, o + 10),
                    xh: le16(&st, o + 12),
                    yh: le16(&st, o + 14),
                    shifts: le16(&st, o + 16),
                }
            })
            .collect();

        let fonts = (0..lay.numfont)
            .map(|i| match chunk(lay.startfont() + i) {
                Some(d) if d.len() >= 770 => {
                    let height = le16(&d, 0) as usize;
                    let widths = d[514..770].to_vec();

                    let glyphs = (0..256)
                        .map(|c| {
                            let w = widths[c] as usize;
                            let loc = le16(&d, 2 + c * 2) as u16 as usize;
                            let bw = (w + 7) / 8;
                            let mut g = vec![0u8; w * height];
                            for y in 0..height {
                                for x in 0..w {
                                    let b = *d.get(loc + y * bw + x / 8).unwrap_or(&0);
                                    g[y * w + x] = (b >> (7 - (x & 7))) & 1;
                                }
                            }
                            g
                        })
                        .collect();
                    Font { height, widths, glyphs }
                }
                _ => Font::default(),
            })
            .collect();

        let pics = (0..lay.numpics)
            .map(|i| {
                let (w, h) = pictable[i];
                chunk(lay.startpics() + i).map(|d| planar(&d, w, h, false)).unwrap_or_default()
            })
            .collect();
        let picms = (0..lay.numpicm)
            .map(|i| {
                let (w, h) = picmtable[i];
                chunk(lay.startpicm() + i).map(|d| planar(&d, w, h, true)).unwrap_or_default()
            })
            .collect();
        let sprites = (0..lay.numsprites)
            .map(|i| {
                let s = spritetable[i];
                chunk(lay.startsprites() + i)
                    .map(|d| planar(&d, s.width.max(0) as usize, s.height.max(0) as usize, true))
                    .unwrap_or_default()
            })
            .collect();

        let t8 = fixed(lay.starttile8(), lay.numtile8 * 32).unwrap_or_default();
        let tile8 = (0..lay.numtile8).map(|i| tile(&t8[i * 32..i * 32 + 32], 8, false)).collect();
        let t8m = fixed(lay.starttile8m(), lay.numtile8m * 40).unwrap_or_default();
        let tile8m = (0..lay.numtile8m).map(|i| tile(&t8m[i * 40..i * 40 + 40], 8, true)).collect();

        let tile16 = (0..lay.numtile16)
            .map(|i| fixed(lay.starttile16() + i, 128).map(|d| tile(&d, 16, false)))
            .collect();
        let tile16m = (0..lay.numtile16m)
            .map(|i| fixed(lay.starttile16m() + i, 160).map(|d| tile(&d, 16, true)))
            .collect();
        let externs = (0..lay.numexterns).map(|i| chunk(lay.startexterns() + i).unwrap_or_default()).collect();

        EgaGraph { layout: lay, pictable, picmtable, spritetable, fonts, pics, picms, sprites, tile8, tile8m, tile16, tile16m, externs }
    }
}
