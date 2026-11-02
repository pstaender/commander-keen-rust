//! CK_DEMO.C (segment 0x13bb of KEEN6C.EXE, linear 0x13bb3..0x157f1) and
//! the start-up / demo loop part of CK_MAIN.C (0x1374e..0x13bb3).
//!
//! The "Terminator" intro and the "Star Wars" story scroller work directly
//! on EGA video memory (planes, CRTC start address, pel panning, page
//! flipping inside a 248 byte wide virtual screen). They are ported on top
//! of a small emulation of the EGA memory (`Ega`) that is shown on the 32
//! bit surface, so the byte level drawing code can be translated 1:1.

use super::defs::*;
use super::Game;
use crate::assets::graphics::Image;
use crate::engine::input::{ControlInfo, Direction};
use crate::engine::input::SC_F1;
use crate::engine::input::SC_SPACE;
use crate::engine::video::EGA_RGB;

/// One entry of the high score table (HighScore, 0x40 bytes at DS:0x3d88).
#[derive(Clone, Debug, Default)]
pub struct HighScore {
    pub name: Vec<u8>,
    pub score: i32,
    pub completed: i32,
}

pub const MAXSCORES: usize = 8;

/// Default high score table (DS:0x3d88).
pub fn default_scores() -> Vec<HighScore> {
    ["Id Software - '91", "", "Jason Blochowiak", "Adrian Carmack", "John Carmack", "Tom Hall", "John Romero", ""]
        .iter()
        .map(|n| HighScore { name: n.as_bytes().to_vec(), score: 10000, completed: 0 })
        .collect()
}

/// Demo playback state and the other CK_DEMO globals.
#[derive(Clone)]
pub struct DemoState {
    /// demo buffer (count, input) byte pairs
    pub buffer: Vec<u8>,
    pub pos: usize,
    /// Scores[] (DS:0x3d88)
    pub scores: Vec<HighScore>,
    /// HighScoresDirty (DS:0xa776): set when the table changed
    pub scores_changed: bool,
    /// scorescreenkludge (DS:0xa34f)
    pub scorescreenkludge: bool,
    /// camera origin when DrawHighScores drew into the master screen
    hs_origin: (i32, i32),
}

impl Default for DemoState {
    fn default() -> Self {
        DemoState {
            buffer: Vec::new(),
            pos: 0,
            scores: default_scores(),
            scores_changed: false,
            scorescreenkludge: false,
            hs_origin: (0, 0),
        }
    }
}

// ---------------------------------------------------------------------------
// EGA memory emulation

/// The four 64K EGA planes plus the registers the intro code uses.
struct Ega {
    planes: [Vec<u8>; 4],
    /// line width in bytes (VW_SetLineWidth, DS:0xa33d)
    lw: u16,
    /// sequencer map mask
    mask: u8,
}

impl Ega {
    fn new() -> Ega {
        Ega { planes: std::array::from_fn(|_| vec![0u8; 0x10000]), lw: 64, mask: 15 }
    }

    #[inline]
    fn wr(&mut self, a: u16, v: u8) {
        for p in 0..4 {
            if self.mask & (1 << p) != 0 {
                self.planes[p][a as usize] = v;
            }
        }
    }

    #[inline]
    fn ylookup(&self, y: i32) -> u16 {
        (y as u16).wrapping_mul(self.lw)
    }

    /// VW_ClearVideo(0) (0x2bda9)
    fn clear_video(&mut self) {
        for p in self.planes.iter_mut() {
            p.fill(0);
        }
    }

    /// VW_SetLineWidth (0x2bd77)
    fn set_line_width(&mut self, w: u16) {
        self.lw = w;
    }

    /// VW_ScreenToScreen (0x2cfc7): latch copy of all planes
    fn screen_to_screen(&mut self, src: u16, dst: u16, w: u16, h: u16) {
        let skip = self.lw.wrapping_sub(w);
        let (mut s, mut d) = (src, dst);
        for _ in 0..h {
            for _ in 0..w {
                for p in 0..4 {
                    self.planes[p][d as usize] = self.planes[p][s as usize];
                }
                s = s.wrapping_add(1);
                d = d.wrapping_add(1);
            }
            s = s.wrapping_add(skip);
            d = d.wrapping_add(skip);
        }
    }

    fn pset(&mut self, ofs: u16, x: i32, y: i32, color: u8) {
        let a = ofs.wrapping_add(self.ylookup(y)).wrapping_add((x >> 3) as u16) as usize;
        let bit = 0x80u8 >> (x & 7);
        for p in 0..4 {
            if color & (1 << p) != 0 {
                self.planes[p][a] |= bit;
            } else {
                self.planes[p][a] &= !bit;
            }
        }
    }

    /// VW_Bar (all planes, at bufferofs `ofs`)
    fn bar(&mut self, ofs: u16, x: i32, y: i32, w: i32, h: i32, color: u8) {
        for yy in y..y + h {
            for xx in x..x + w {
                self.pset(ofs, xx, yy, color);
            }
        }
    }

    /// VW_DrawPic from an unmasked picture
    fn draw_pic(&mut self, ofs: u16, x: i32, y: i32, img: &Image) {
        for py in 0..img.height {
            for px in 0..img.width {
                self.pset(ofs, x + px as i32, y + py as i32, img.get(px, py) & 15);
            }
        }
    }
}

impl Game {
    /// Shows the EGA memory starting at CRTC address `start` with pel
    /// panning `pel` on the surface (in logical 320x200 pixels).
    fn ega_render(&mut self, ega: &Ega, start: u16, pel: u16) {
        let s = &mut self.sys.surface;
        let sc = s.scale;
        let colors: [u32; 16] = std::array::from_fn(|i| 0xff00_0000 | EGA_RGB[i]);
        for y in 0..200usize {
            let row = start.wrapping_add((y as u16).wrapping_mul(ega.lw));
            for x in 0..320usize {
                let px = x + pel as usize;
                let a = row.wrapping_add((px >> 3) as u16) as usize;
                let bit = 7 - (px & 7);
                let c = ((ega.planes[0][a] >> bit) & 1)
                    | (((ega.planes[1][a] >> bit) & 1) << 1)
                    | (((ega.planes[2][a] >> bit) & 1) << 2)
                    | (((ega.planes[3][a] >> bit) & 1) << 3);
                let col = colors[c as usize];
                for yy in y * sc..(y + 1) * sc {
                    let r = yy * s.w;
                    s.px[r + x * sc..r + (x + 1) * sc].fill(col);
                }
            }
        }
    }

    /// VW_SetScreen (0x2d28b): sets the display start and waits for the
    /// vertical retrace.
    fn ega_set_screen(&mut self, ega: &Ega, start: u16, pel: u16) {
        self.ega_render(ega, start, pel);
        self.sys.wait_vbl(1);
    }

    /// Sets all 16 palette registers from a palette table, keeping the
    /// border colour (the int 10h/1002h calls).
    fn set_palette_regs(&mut self, pal: &[u8; 16]) {
        self.sys.palette.regs[..16].copy_from_slice(pal);
        self.sys.palette.regs[16] = self.sys.border;
    }

    /// VW_SetDefaultColors (0x2bbf0)
    fn set_default_colors(&mut self) {
        self.set_default_palette();
    }

    /// IN_IsUserInput (0x23e8e)
    fn is_user_input(&mut self) -> bool {
        if !self.alive() {
            return true;
        }
        let c = self.sys.input.read_control();
        self.sys.input.last_scan != 0 || c.button0 || c.button1
    }

    /// The common "any key aborts" check of the intro loops: a key sets
    /// LastScan to space (unless it is F1); true if aborted.
    fn intro_aborted(&mut self) -> bool {
        if self.is_user_input() && self.sys.input.last_scan != SC_F1 {
            self.sys.input.last_scan = SC_SPACE;
        }
        self.sys.input.last_scan != 0 || !self.alive()
    }

    /// IN_UserInput(delay, clear) (0x23edf)
    fn in_user_input(&mut self, delay: i32, clear: bool) -> bool {
        let lasttime = self.sys.time_count();
        loop {
            if self.is_user_input() {
                if clear {
                    self.sys.input.clear_keys_down();
                }
                return true;
            }
            self.sys.wait_vbl(1);
            if self.sys.time_count() - lasttime >= delay {
                return false;
            }
        }
    }

    /// Waits until at least `min` tics passed since lasttimecount; returns
    /// the tics and updates lasttimecount.
    fn wait_tics(&mut self, min: i32) -> i32 {
        loop {
            let now = self.sys.time_count();
            let tics = now - self.lasttimecount;
            if tics >= min || self.sys.quit {
                self.lasttimecount = now;
                return tics;
            }
            if self.sys.headless_time.is_none() {
                std::thread::sleep(std::time::Duration::from_millis(1));
            }
            self.sys.poll();
        }
    }
}

// ---------------------------------------------------------------------------
// Terminator intro

/// palette with all registers black (DS:0x4846)
const PAL_BLACK: [u8; 16] = [0; 16];
/// palette during the scrolling part (K6 DS:0x116, K4 0x116, K5 0x1b8)
const PAL_TERM1: [u8; 16] = ep_sel!(
    [0, 24, 24, 7, 1, 1, 1, 1, 17, 17, 17, 17, 19, 19, 19, 19],
    [0, 4, 4, 28, 1, 1, 1, 1, 17, 17, 17, 17, 19, 19, 19, 19],
    [0, 5, 5, 21, 1, 1, 1, 1, 17, 17, 17, 17, 19, 19, 19, 19]
);
/// palette during the zoom (K6 DS:0x127, K4 0x127, K5 0x1c9)
const PAL_TERM2: [u8; 16] = ep_sel!(
    [0, 24, 24, 7, 1, 1, 1, 1, 17, 17, 17, 17, 19, 19, 19, 24],
    [0, 4, 4, 28, 1, 1, 1, 1, 17, 17, 17, 17, 19, 19, 19, 4],
    [0, 5, 5, 21, 1, 1, 1, 1, 17, 17, 17, 17, 19, 19, 19, 5]
);
/// palette of the story scroller (K6 DS:0xfc, K4 0xfc, K5 0x19e)
const PAL_STARWARS: [u8; 16] = ep_sel!(
    [0, 1, 2, 3, 4, 16, 6, 7, 31, 31, 31, 31, 31, 31, 31, 31],
    [0, 1, 24, 30, 31, 28, 6, 7, 19, 19, 19, 19, 19, 19, 19, 19],
    [0, 1, 24, 25, 4, 28, 6, 7, 31, 31, 31, 31, 31, 31, 31, 31]
);
/// left / right bit masks (DS:0x138 / DS:0x140)
const LEFTMASKS: [u8; 8] = [255, 127, 63, 31, 15, 7, 3, 1];
const RIGHTMASKS: [u8; 8] = [0, 128, 192, 224, 240, 248, 252, 254];
/// credit pictures shown during the scroll (K6 DS:0x10e, K4 0x10e, K5 0x1b0)
const CREDITPICS: [usize; 4] = ep_sel!([96, 97, 98, 99], [83, 84, 85, 86], [29, 30, 31, 32]);
/// fizzle fade bit masks (DS:0x148)
const FIZZLEMASKS: [u8; 8] = [1, 2, 4, 8, 16, 32, 64, 128];

use super::consts::{BIGCOMMANDER, BIGKEEN, DEMO0};
const TITLEPICPIC: i32 = ep_sel!(0x65, 0x58, 34);
const STARWARSPIC: usize = ep_sel!(0x64, 0x57, 33);
const SWFONT: usize = 2;
/// StartMusic argument of StarWars (LEVELMUSIC index of the story song)
const STARWARSMUSIC: i32 = ep_sel!(12, 17, 13);
/// map number of the high score level (CheckHighScore)
const HIGHSCORELEVEL: i32 = ep_sel!(19, 15, 18);
/// high score table layout: first line y, name x, right edge of the score
const HS_TOP: i32 = ep_sel!(0x33, 0x23, 0x33);
const HS_NAMEX: i32 = ep_sel!(0x18, 0x28, 0x28);
const HS_SCORERIGHT: i32 = ep_sel!(0x128, 0x118, 0x118);
/// font colour of the high score table (Keen 5 draws it in light red)
const HS_COLOR: u8 = ep_sel!(15, 12, 15);
/// Keen 4: tile8 drawn once per rescued elder next to the name (x 0x98..)
const HS_ELDERTILE: i32 = 0x47;

/// A run length encoded 1 bit shape (BIGCOMMANDER / BIGKEEN externs):
/// word width, then 200 line offsets, each line a list of alternating
/// clear / set run lengths terminated by 0xffff.
struct Shape {
    width: u16,
    lines: Vec<Vec<u16>>,
}

impl Shape {
    fn load(data: &[u8]) -> Shape {
        let w = |o: usize| -> u16 {
            if o + 1 < data.len() {
                u16::from_le_bytes([data[o], data[o + 1]])
            } else {
                0xffff
            }
        };
        let mut lines = Vec::with_capacity(200);
        for y in 0..200 {
            let mut o = w(4 + y * 2) as usize;
            let mut runs = Vec::new();
            loop {
                let r = w(o);
                o += 2;
                if r == 0xffff {
                    break;
                }
                runs.push(r);
            }
            lines.push(runs);
        }
        Shape { width: w(2), lines }
    }
}

/// Converts one shape line to a bit mask (0x14237), returns the bytes in
/// the order they are written.
fn rle_line(runs: &[u16]) -> Vec<u8> {
    let mut out = Vec::new();
    let mut pos: u16 = 0;
    let mut acc: u8 = 0;
    let mut it = runs.iter();
    loop {
        let Some(&n) = it.next() else { break };
        pos = pos.wrapping_add(n);
        if pos > 7 {
            out.push(acc);
            acc = 0;
            for _ in 1..(pos >> 3) {
                out.push(0);
            }
            pos &= 7;
        }
        let Some(&n) = it.next() else { break };
        acc |= LEFTMASKS[pos as usize];
        pos = pos.wrapping_add(n);
        if pos > 7 {
            out.push(acc);
            acc = 0xff;
            for _ in 1..(pos >> 3) {
                out.push(0xff);
            }
            pos &= 7;
        }
        acc &= RIGHTMASKS[pos as usize];
    }
    out.push(acc);
    out.push(0);
    out
}

/// A credit picture (planes 0 and 1 only, 0x13bff).
struct CreditPic {
    /// width in bytes
    width: u16,
    height: u16,
    /// plane 0 bytes followed by plane 1 bytes
    data: Vec<u8>,
}

impl CreditPic {
    fn load(img: &Image, width: usize, height: usize) -> CreditPic {
        let mut data = vec![0u8; width * height * 2];
        for p in 0..2 {
            for y in 0..height {
                for c in 0..width {
                    let mut b = 0u8;
                    for bit in 0..8 {
                        let x = c * 8 + bit;
                        if x < img.width && y < img.height && (img.get(x, y) >> p) & 1 != 0 {
                            b |= 0x80 >> bit;
                        }
                    }
                    data[p * width * height + y * width + c] = b;
                }
            }
        }
        CreditPic { width: width as u16, height: height as u16, data }
    }
}

/// All the state of the Terminator intro.
struct Term {
    ega: Ega,
    keen: Shape,
    commander: Shape,
    /// width of a pre-shifted commander line in bytes (DS:0xa376)
    shiftwidth: u16,
    /// commander at half height, pre-shifted 0..7 pixels (DS:0xa325)
    shifted: Vec<Vec<u8>>,
    credits: Vec<CreditPic>,
    /// screen page (DS:0x6df8) and its offset (DS:0xa371)
    page: usize,
    pageofs: u16,
    /// final screen start of the scroll (DS:0xa335)
    screenofs: u16,
    /// credit state machine (DS:0x6dfa state, 0x6dfc time, 0x6df2 pic,
    /// 0x6df0 y, 0x6df4 old bottoms)
    cstate: i16,
    ctime: i16,
    cpic: usize,
    cy: i16,
    oldbottom: [i16; 2],
    /// current credit pic metrics (a357 width, a003 clear words,
    /// a361 height, a355 line skip, a363 plane offset, a5d0 data)
    cwidth: u16,
    cclear: u16,
    cheight: u16,
    cskip: u16,
    cplane: u16,
    cur: usize,
}

impl Term {
    /// credit Y position state machine (0x13e70)
    fn credit_y(&mut self, t: i16) -> i16 {
        let half = (self.cheight >> 1) as i16;
        match self.cstate {
            -1 => {
                let c = &self.credits[self.cpic];
                self.cur = self.cpic;
                self.cwidth = c.width;
                self.cclear = (c.width + 3) >> 1;
                self.cheight = c.height;
                self.cskip = self.ega.lw.wrapping_sub(c.width + 1);
                self.cplane = c.width.wrapping_mul(c.height);
                self.cstate += 1;
                self.ctime = t;
                self.cy = 0xf0;
                self.credit_rise(t)
            }
            0 => self.credit_rise(t),
            1 => {
                if t.wrapping_sub(self.ctime) > 200 {
                    self.cstate += 1;
                    self.ctime = t;
                }
                100 - half
            }
            2 => {
                self.cy = self.cy.wrapping_sub(t.wrapping_sub(self.ctime).wrapping_mul(2));
                if self.cy < -40 {
                    self.cy = -40;
                    self.cpic += 1;
                    self.cstate = if self.cpic < 4 { -1 } else { 3 };
                }
                self.ctime = t;
                self.cy - half
            }
            _ => -40,
        }
    }

    fn credit_rise(&mut self, t: i16) -> i16 {
        self.cy = self.cy.wrapping_sub(t.wrapping_sub(self.ctime).wrapping_mul(2));
        if self.cy < 100 {
            self.cy = 100;
            self.cstate += 1;
        }
        self.ctime = t;
        self.cy - (self.cheight >> 1) as i16
    }

    /// Draws the current credit picture into planes 2 and 3 (0x13cd0).
    fn draw_credit(&mut self, t: i16, x: u16) {
        let shift = x & 7;
        let dest = self.pageofs.wrapping_add(x >> 3).wrapping_add(0x14u16.wrapping_sub(self.cwidth >> 1));
        self.ega.mask = 0xc;
        let mut top = self.credit_y(t);
        let mut bottom = top.wrapping_add(self.cheight as i16);
        if bottom < 0 {
            bottom = 0;
        }
        let old = self.oldbottom[self.page];
        if bottom < 200 && old > bottom {
            // erase what the picture left below itself
            let skip = self.ega.lw.wrapping_sub(self.cclear).wrapping_sub(self.cclear);
            let mut di = dest.wrapping_add(self.ega.ylookup(bottom as i32));
            for _ in 0..(old - bottom) {
                for _ in 0..self.cclear * 2 {
                    self.ega.wr(di, 0);
                    di = di.wrapping_add(1);
                }
                di = di.wrapping_add(skip);
            }
        }
        if bottom > 200 {
            bottom = 200;
        }
        self.oldbottom[self.page] = bottom;
        let mut rows = self.cheight as i16;
        let mut srcoff: usize = 0;
        if top < 0 {
            srcoff = self.cwidth as usize * (-top) as usize;
            rows = rows.wrapping_add(top);
            top = 0;
        } else if (top as u16).wrapping_add(self.cheight) > 200 {
            rows = 200 - top;
        }
        if rows <= 0 {
            return;
        }
        let di0 = dest.wrapping_add(self.ega.ylookup(top as i32));
        let width = self.cwidth as usize;
        let data = &self.credits[self.cur].data;
        for (plane, mut src) in [(4u8, srcoff), (8u8, srcoff + self.cplane as usize)] {
            self.ega.mask = plane;
            let mut di = di0;
            for _ in 0..rows {
                let mut al: u8 = 0;
                for _ in 0..width {
                    let b = data.get(src).copied().unwrap_or(0);
                    src += 1;
                    let lo = if shift == 0 { b } else { b >> shift };
                    let hi = if shift == 0 { 0 } else { b << (8 - shift) };
                    self.ega.wr(di, al | lo);
                    di = di.wrapping_add(1);
                    al = hi;
                }
                self.ega.wr(di, al);
                di = di.wrapping_add(1);
                self.ega.wr(di, 0);
                self.ega.wr(di.wrapping_add(1), 0);
                di = di.wrapping_add(self.cskip);
            }
        }
    }

    fn flip(&mut self) {
        self.page ^= 1;
        self.pageofs = if self.page != 0 { 0x7c } else { 0 };
    }
}

impl Game {
    /// Terminator (0x14b61): COMMANDER and KEEN fly past each other with
    /// the credits, the logo zooms out and the title screen fizzles in.
    fn terminator(&mut self) {
        let mut ega = Ega::new();
        self.set_palette_regs(&PAL_BLACK);
        ega.clear_video();
        ega.set_line_width(0xf8);
        let keen = Shape::load(&self.data.gfx.externs[BIGKEEN - super::consts::STARTEXTERNS]);
        let commander = Shape::load(&self.data.gfx.externs[BIGCOMMANDER - super::consts::STARTEXTERNS]);

        // KEEN goes into plane 0, 200 pixels into the virtual screen
        ega.mask = 1;
        let a341 = keen.width.wrapping_add(200);
        self.ega_set_screen(&ega, (a341 >> 3) + 1, 0);
        for y in 0..200 {
            let mut a = ega.ylookup(y).wrapping_add(0x19);
            for b in rle_line(&keen.lines[y as usize]) {
                ega.wr(a, b);
                a = a.wrapping_add(1);
            }
        }
        ega.screen_to_screen(0, 0x7c, 0x6d, 200);

        // COMMANDER at half height, shifted 0..7 pixels
        let shiftwidth = ((commander.width.wrapping_add(7) >> 3) + 3) & 0xfffe;
        let size = shiftwidth as usize * 100;
        let mut first = vec![0u8; size + 256];
        for y in 0..100 {
            let o = y * shiftwidth as usize;
            for (i, b) in rle_line(&commander.lines[y * 2]).into_iter().enumerate() {
                first[o + i] = b;
            }
        }
        first.truncate(size);
        let mut shifted = vec![first];
        for s in 1..8 {
            let prev = &shifted[s - 1];
            let mut carry = 0u8;
            let next: Vec<u8> = prev
                .iter()
                .map(|&b| {
                    let r = (b >> 1) | (carry << 7);
                    carry = b & 1;
                    r
                })
                .collect();
            shifted.push(next);
        }

        self.set_palette_regs(&PAL_TERM1);
        let credits = CREDITPICS
            .iter()
            .map(|&c| {
                let (w, h) = self.data.gfx.pictable[c - super::consts::STARTPICS];
                CreditPic::load(&self.data.gfx.pics[c - super::consts::STARTPICS], w, h)
            })
            .collect();
        let mut t = Term {
            ega,
            keen,
            commander,
            shiftwidth,
            shifted,
            credits,
            page: 0,
            pageofs: 0,
            screenofs: 0,
            cstate: -1,
            ctime: 0,
            cpic: 0,
            cy: 0,
            oldbottom: [0, 0],
            cwidth: 0,
            cclear: 0,
            cheight: 0,
            cskip: 0,
            cplane: 0,
            cur: 0,
        };
        self.scroll_commander(&mut t);
        if self.sys.input.last_scan == 0 && self.alive() {
            self.zoom_out(&mut t);
        }
        let mut fizzled = false;
        if self.sys.input.last_scan == 0 && self.alive() {
            self.fizzle_title(&mut t);
            fizzled = true;
        }
        // VW_ClearVideo(0), VW_SetLineWidth(64), VW_SetDefaultColors
        self.sys.surface.clear(0);
        self.set_default_colors();
        if super::EPISODE == 6 {
            self.check_last_scan();
        } else {
            self.terminator_last_scan(fizzled);
        }
    }

    /// Keen 4/5 end of Terminator (an inlined CheckLastScan): F1 shows the
    /// help screens; any other key first redraws the title screen (Keen 5:
    /// only if the fizzle was interrupted) before the control panel opens.
    fn terminator_last_scan(&mut self, fizzled: bool) {
        if self.sys.input.last_scan == 0 || !self.alive() {
            return;
        }
        if self.sys.input.last_scan == SC_F1 {
            self.help_screens();
            return;
        }
        if super::EPISODE == 5 && fizzled {
            // Keen 5: the title is on screen, straight to the control panel
            self.control_panel_from_demo();
            return;
        }
        // VWB_DrawPic(0, 0, TITLEPICPIC) and show it
        self.draw_pic(0, 0, TITLEPICPIC);
        self.sys.present();
        // IN_StartAck-like: forget the key that was pressed
        let i = &mut self.sys.input;
        let ls = i.last_scan as usize;
        if ls < i.keyboard.len() {
            i.masked[ls] |= i.keyboard[ls];
            i.keyboard[ls] = false;
        }
        i.last_scan = 0;
        if self.storedemo {
            self.playstate = EX_RESETGAME;
            self.restartgame = true;
            self.restartdifficulty = GD_NORMAL;
            self.sys.input.clear_keys_down();
            self.new_game();
        } else {
            self.control_panel_from_demo();
        }
    }

    /// The scrolling part of the intro (0x13f85).
    fn scroll_commander(&mut self, t: &mut Term) {
        let a341 = t.keen.width.wrapping_add(200) as i32;
        let endx = 0x78 - t.commander.width as i16 as i32;
        let startx = 0x140;
        let delta = endx - startx;
        let dist = delta.unsigned_abs() as i32;
        t.page = 0;
        t.pageofs = 0;
        self.lasttimecount = self.sys.time_count();
        while self.sys.time_count() == self.lasttimecount && !self.sys.quit {
            self.sys.poll();
        }
        self.lasttimecount = self.sys.time_count();
        let mut time: i32 = 0;
        let mut pos: i32 = 0;
        while time as u16 <= dist as u16 {
            pos = (dist - time) * a341 / dist;
            t.draw_credit(time as i16, pos as u16);
            let mut x = startx + delta * time / dist;
            x += pos & 7;
            let col = (x + 0x800) / 8 - 0x100;
            let shift = ((x + 0x800) & 7) as usize;
            let mut srcoff: i32 = 0;
            let mut di = t.pageofs.wrapping_add((pos / 8) as u16);
            let sw = t.shiftwidth as i32;
            let (lblank, copy, rblank);
            if col > 0 {
                lblank = (col + 1) / 2;
                if col & 1 != 0 {
                    di = di.wrapping_sub(1);
                }
                copy = 0x15 - lblank;
                rblank = 0;
            } else {
                if 0x28 - sw < col {
                    lblank = 0;
                    copy = 0x15;
                    rblank = 0;
                } else {
                    lblank = 0;
                    copy = (sw + col) / 2;
                    rblank = 0x15 - copy;
                }
                srcoff -= col;
            }
            let srcskip = sw - copy * 2;
            let dstskip = (0xf8 - (lblank + copy + rblank) * 2) as u16;
            t.ega.mask = 2;
            let buf = &t.shifted[shift];
            let mut si = srcoff;
            let mut saved = si;
            for dx in (1..=200).rev() {
                for _ in 0..lblank * 2 {
                    t.ega.wr(di, 0);
                    di = di.wrapping_add(1);
                }
                for _ in 0..copy * 2 {
                    let b = buf.get(si as usize).copied().unwrap_or(0);
                    t.ega.wr(di, b);
                    di = di.wrapping_add(1);
                    si += 1;
                }
                for _ in 0..rblank * 2 {
                    t.ega.wr(di, 0);
                    di = di.wrapping_add(1);
                }
                if dx & 1 == 0 {
                    si = saved;
                } else {
                    si += srcskip;
                    saved = si;
                }
                di = di.wrapping_add(dstskip);
            }
            let start = t.pageofs.wrapping_add((pos / 8) as u16);
            self.ega_set_screen(&t.ega, start, (pos & 7) as u16);
            t.flip();
            let tics = self.wait_tics(2);
            self.tics = tics;
            if self.intro_aborted() {
                return;
            }
            time += tics;
        }
        t.screenofs = (pos / 8) as u16;
    }

    /// Scales one line of the combined logo (0x1440f).
    fn scale_line(t: &mut Term, src: &[u16], y: i32, x: i32, table: &[u16]) {
        #[derive(PartialEq)]
        enum At {
            Next,
            OffEnd,
            OnEnd,
        }
        let tv = |v: u16| -> i16 { table.get(v as usize).copied().unwrap_or(0x7fff) as i16 };
        let rd = |i: &mut usize| -> u16 {
            let v = src.get(*i).copied().unwrap_or(0xffff);
            *i += 1;
            v
        };
        let limit = (0x140 - x) as i16;
        let mut dest = t.pageofs.wrapping_add(t.screenofs).wrapping_add(t.ega.ylookup(y));
        let mut acc: u8 = 0;
        let mut bitpos: u16;
        let mut last: i16;
        let mut idx = 0usize;
        let mut si: i16;
        let mut at;
        if x < 0 {
            let skip = (-x) as i16;
            bitpos = 0;
            loop {
                si = tv(rd(&mut idx));
                if si > skip {
                    at = At::OnEnd;
                    break;
                }
                si = tv(rd(&mut idx));
                if si > skip {
                    at = At::OffEnd;
                    break;
                }
            }
            last = skip;
        } else {
            bitpos = (x & 7) as u16;
            dest = dest.wrapping_add((x >> 3) as u16);
            last = 0;
            idx = 1;
            si = 0;
            at = At::Next;
        }
        let mut put = |dest: &mut u16, v: u8| {
            t.ega.wr(*dest, v);
            *dest = dest.wrapping_add(1);
        };
        loop {
            if at == At::Next {
                si = tv(rd(&mut idx));
                at = At::OffEnd;
            }
            if at == At::OffEnd {
                // a clear run ends at si
                let run = si.wrapping_sub(last);
                last = si;
                bitpos = bitpos.wrapping_add(run as u16);
                if bitpos > 7 {
                    put(&mut dest, acc);
                    for _ in 1..(bitpos >> 3) {
                        put(&mut dest, 0);
                    }
                    acc = 0;
                    bitpos &= 7;
                }
                if si > limit {
                    return;
                }
                let v = rd(&mut idx);
                if v == 0xffff {
                    let run = 0x140i16.wrapping_sub(last);
                    bitpos = bitpos.wrapping_add(run as u16);
                    if bitpos > 7 {
                        put(&mut dest, acc);
                        for _ in 1..(bitpos >> 3) {
                            put(&mut dest, 0);
                        }
                    }
                    return;
                }
                si = tv(v);
            }
            // a set run ends at si
            let run = si.wrapping_sub(last);
            last = si;
            acc |= LEFTMASKS[bitpos as usize & 7];
            bitpos = bitpos.wrapping_add(run as u16);
            if bitpos > 7 {
                put(&mut dest, acc);
                for _ in 1..(bitpos >> 3) {
                    put(&mut dest, 0xff);
                }
                acc = 0xff;
                bitpos &= 7;
            }
            if si > limit {
                return;
            }
            acc &= RIGHTMASKS[bitpos as usize & 7];
            at = At::Next;
        }
    }

    /// Zooms the COMMANDER KEEN logo to its title position (0x145cd).
    fn zoom_out(&mut self, t: &mut Term) {
        self.set_palette_regs(&PAL_TERM2);
        let startx = 0x78 - t.commander.width as i32;
        // build the combined line boundaries (0x1430d)
        let mut lines: Vec<Vec<u16>> = Vec::with_capacity(200);
        for y in 0..200 {
            let mut list = Vec::new();
            let mut si: u16 = 0;
            let c = &t.commander.lines[y];
            let mut i = 0;
            let next = |i: &mut usize, l: &Vec<u16>| -> u16 {
                let v = l.get(*i).copied().unwrap_or(0xffff);
                *i += 1;
                v
            };
            let mut di = next(&mut i, c);
            loop {
                list.push(si);
                si = si.wrapping_add(di);
                di = next(&mut i, c);
                if di == 0xffff {
                    break;
                }
            }
            si = si.wrapping_add(0x50);
            let k = &t.keen.lines[y];
            let mut i = 1;
            let mut di = next(&mut i, k);
            loop {
                list.push(si);
                si = si.wrapping_add(di);
                di = next(&mut i, k);
                if di == 0xffff {
                    break;
                }
            }
            list.push(si);
            list.push(0xffff);
            lines.push(list);
        }
        let mut table = vec![0u16; 2500];
        let finalscale: u16 = 0x21;
        let steps: u16 = 30;
        let mut oldbottom = [200i32; 2];
        let mut step: u16 = 1;
        while step <= steps {
            let (scale, x, top);
            if step == steps {
                scale = finalscale;
                x = 0;
                top = 4;
            } else {
                scale = 0x100 - (((0x100 - finalscale) as u32 * step as u32) as u16) / steps;
                x = startx * (steps - step) as i32 / steps as i32;
                top = (step << 2) / steps;
            }
            let mut acc: u32 = 0;
            for e in table.iter_mut() {
                *e = (acc >> 8) as u16;
                acc = acc.wrapping_add(scale as u32);
            }
            let height = table[200];
            let mut srcy: u16 = 0;
            let ystep = (0x10000u32 / scale as u32) as u16;
            let bufofs = t.pageofs.wrapping_add(t.screenofs);
            if top > 0 {
                t.ega.bar(bufofs, 0, 0, 0x140, top as i32, 0);
            }
            t.ega.mask = 0xf;
            for si in 0..height {
                let line = &lines[(srcy >> 8) as usize];
                Self::scale_line(t, line, (si + top) as i32, x, &table);
                srcy = srcy.wrapping_add(ystep);
            }
            let bottom = (height + top) as i32;
            if oldbottom[t.page] > bottom {
                t.ega.bar(bufofs, 0, bottom, 0x140, oldbottom[t.page] - bottom, 0);
                oldbottom[t.page] = bottom;
            }
            self.ega_set_screen(&t.ega, bufofs, 0);
            t.flip();
            let now = self.sys.time_count();
            let mut tics = now - self.lasttimecount;
            self.lasttimecount = now;
            if tics as u16 > 8 {
                tics = 8;
            }
            self.tics = tics;
            if step == steps {
                break;
            }
            step = step.wrapping_add(tics as u16);
            if step > steps {
                step = steps;
            }
            if self.intro_aborted() {
                return;
            }
        }
    }

    /// Dissolves the title picture over the logo (0x148ab).
    fn fizzle_title(&mut self, t: &mut Term) {
        let mut cols: Vec<usize> = (0..320).collect();
        for i in 0..320 {
            let r = super::keen2::borland_rand(&mut self.crand) as i64;
            let j = (r * 320 / 0x8000) as usize;
            cols.swap(i, j);
        }
        let mut rowpos: Vec<usize> = cols[..200].to_vec();
        self.set_default_colors();
        let (drawofs, dispofs) = if t.page != 0 {
            (t.screenofs.wrapping_add(0x7c), t.screenofs)
        } else {
            (t.screenofs, t.screenofs.wrapping_add(0x7c))
        };
        self.ega_set_screen(&t.ega, dispofs, 0);
        let pic = &self.data.gfx.pics[TITLEPICPIC as usize - super::consts::STARTPICS];
        t.ega.draw_pic(drawofs, 0, 0, pic);
        let diff = dispofs.wrapping_sub(drawofs);
        for i in 0..=0x167i32 {
            let top = (i - 0xa0).max(0);
            let bottom = i.min(199);
            for y in top..=bottom {
                let lineofs = drawofs.wrapping_add(t.ega.ylookup(y));
                for _ in 0..2 {
                    let x = cols[rowpos[y as usize]];
                    rowpos[y as usize] += 1;
                    if rowpos[y as usize] == 320 {
                        rowpos[y as usize] = 0;
                    }
                    let m = FIZZLEMASKS[x & 7];
                    let s = lineofs.wrapping_add((x >> 3) as u16) as usize;
                    let d = (s as u16).wrapping_add(diff) as usize;
                    for p in 0..4 {
                        let pl = &mut t.ega.planes[p];
                        pl[d] = (pl[s] & m) | (pl[d] & !m);
                    }
                }
            }
            // VW_WaitVBL(1)
            self.ega_render(&t.ega, dispofs, 0);
            self.sys.wait_vbl(1);
            if self.intro_aborted() {
                return;
            }
        }
        self.in_user_input(0x1a4, false);
    }
}

// ---------------------------------------------------------------------------
// Star Wars story scroller

/// The story text (K6 segment 0x2d88, K4 0x2db4, K5 0x2fde).
const STORYTEXT: &str = ep_sel!(
    "Episode Four\n\nSecret of the Oracle\n\nAfter delivering a\ncrippling blow to the\nplans of Mortimer\nMcMire and receiving\nthe praise of the\nVorticon race,\nCommander Keen\nreturned to his home in\nthe suburbs.\n\nHere he was forced to\ngo to bed at an early\nhour, and to eat mashed\npotatoes.\n\nMonths later, Billy\ntinkered around with\nhis latest invention,\nthe Photachyon\nTransceiver, or faster-\nthan-light radio. After\npicking up a lot of bad\nalien sitcoms, he\nstumbled upon a strange\nmessage of terrible\nimportance....\n",
    "Episode Five\n\nThe Armageddon Machine\n\nAfter learning the\nlocation of the secret\nShikadi base, Keen\njumped in the trusty\nBean-with-Bacon\nMegarocket and blasted\nacross interstellar\nspace.\n\nSeventy-five furious\ngames of Paddle War\nlater, Keen dropped\nout of lightspeed near\n the Korath system.\n\nHe flew toward the\nplanet, keeping it\nbetween him and the\nbase.\n\nPulling up underside\nand docking at the\nIon Ventilation System,\nKeen must destroy the\nShikadi Armageddon\nMachine before it\nexplodes and destroys\nthe Milky Way!  He\nsteps into the dark\nventilation duct and\nbegins his most\ndangerous adventure\nyet...\n",
    "Episode Six\n\nAliens Ate My\nBaby Sitter!\n\nWhile out in his\nbackyard clubhouse,\nBilly's baby sitter\nMolly calls him for\ndinner. He continues\nworking on his new\nwrist computer.\n\nSuddenly, there is a\nloud noise outside.\n\nRushing out, Keen finds\nhis baby sitter gone\nand a note on a patch\nof scorched grass.  The\nBloogs of Fribbulus Xax\nare going to make a\nmeal out of Molly!\n\nYou've got to rescue\nher, because your\nparents will never\nbelieve you when you\ntell them...\n\n\"Aliens Ate My\nBaby Sitter!\"\n"
);

/// Remaps the story text to the characters of the big font (0x150f0).
fn translate_story(s: &mut [u8]) {
    for c in s.iter_mut() {
        *c = match *c {
            b'A'..=b'Z' => *c - 0x21,
            b'a'..=b'z' => *c - 0x27,
            b',' => b'U',
            b'-' => b'V',
            b'"' => b'W',
            b' ' => b'X',
            b'!' => b'Y',
            b'\'' => b'Z',
            b'\n' => b'\n',
            _ => b'T',
        };
    }
}

/// One compiled perspective scaler line (built by 0x14eca).
struct SwLine {
    /// screen offset of the first byte (ylookup + x/8)
    addr: u16,
    /// first destination bit
    dstbit: u8,
    /// source x coordinate of every destination pixel
    samples: Vec<u16>,
}

impl Game {
    /// Renders the story into a 336 pixel wide bit plane (0x15170).
    /// Returns the bitmap and the number of lines.
    fn render_story(&self) -> (Vec<u8>, u16) {
        let font = &self.data.gfx.fonts[SWFONT];
        let windoww: i32 = 0x150;
        let mut printy: i32 = 1;
        let mut ofsrows: i32 = 0;
        let mut lines: u16 = 0;
        let mut bits: Vec<u8> = Vec::new();
        for raw in STORYTEXT.split_inclusive('\n') {
            let mut line = raw.as_bytes().to_vec();
            translate_story(&mut line);
            // US_CPrint: every '\n' separated part is a centered line
            let mut s = &line[..];
            loop {
                let end = s.iter().position(|&c| c == b'\n').unwrap_or(s.len());
                let part = &s[..end];
                let w = crate::engine::video::string_width(font, part);
                let h = font.height as i32;
                let mut px = ((windoww - w) as u16 >> 1) as i32;
                let py = ofsrows + printy;
                let need = ((py + h) * 42) as usize;
                if bits.len() < need {
                    bits.resize(need, 0);
                }
                for &c in part {
                    let cw = font.widths.get(c as usize).copied().unwrap_or(0) as i32;
                    if let Some(g) = font.glyphs.get(c as usize) {
                        for gy in 0..h {
                            for gx in 0..cw {
                                if g.get((gy * cw + gx) as usize).copied().unwrap_or(0) != 0 {
                                    let x = px + gx;
                                    if (0..336).contains(&x) {
                                        bits[((py + gy) * 42 + (x >> 3)) as usize] |= 0x80 >> (x & 7);
                                    }
                                }
                            }
                        }
                    }
                    px += cw;
                }
                printy += h;
                if end >= s.len() {
                    break;
                }
                s = &s[end + 1..];
                if s.is_empty() {
                    break;
                }
            }
            ofsrows += printy;
            lines = lines.wrapping_add(printy as u16);
            printy = 0;
        }
        bits.resize(ofsrows as usize * 42, 0);
        (bits, lines)
    }

    /// Builds the scaler lines and reveals the background (0x14eca);
    /// stops early (with LastScan set) when a key is pressed.
    fn compile_sw(&mut self, ega: &mut Ega) -> (Vec<SwLine>, Vec<u16>) {
        let mut out: Vec<SwLine> = (0..200).map(|_| SwLine { addr: 0, dstbit: 0, samples: Vec::new() }).collect();
        let mut linestart = vec![0u16; 200];
        let mut width: i32 = 0xa0000;
        let mut linedist: u32 = 0;
        for y in (0..200).rev() {
            // VWB_Hlin(0, 320, y, 1) is overwritten by the copy below at once
            let sw = (((width / 2) >> 11) << 1) as u16;
            linestart[y] = (linedist >> 11) as u16;
            let srcstep = if sw != 0 { (0xa8000u32 / sw as u32) as u16 } else { 0 };
            linedist = linedist.wrapping_add(srcstep as u32);
            let destx = 0xa0u16.wrapping_sub(sw >> 1);
            let addr = ega.ylookup(y as i32).wrapping_add(destx >> 3);
            let mut srcx: u32 = 0;
            let mut samples = Vec::with_capacity(sw as usize);
            for _ in 0..sw {
                samples.push((srcx >> 11) as u16);
                srcx = srcx.wrapping_add(srcstep as u32);
            }
            out[y] = SwLine { addr, dstbit: (destx & 7) as u8, samples };
            width -= 0xb33;
            let l = ega.ylookup(y as i32);
            ega.screen_to_screen(l.wrapping_add(0x8000), l, 0x28, 1);
            if y % 4 == 0 {
                self.ega_render(ega, 0, 0);
                self.sys.present();
            }
            if self.is_user_input() && self.sys.input.last_scan != SC_F1 {
                self.sys.input.last_scan = SC_SPACE;
            }
            if self.sys.input.last_scan != 0 || !self.alive() {
                break;
            }
        }
        (out, linestart)
    }

    /// Scrolls the story into the distance (0x15259).
    fn scroll_sw(&mut self, ega: &mut Ega, lines: &[SwLine], linestart: &[u16], text: &[u8], textlines: u16) {
        self.lasttimecount = 0;
        self.sys.set_time_count(0);
        let mut tics: u16 = 0;
        ega.mask = 8;
        let mut page: u16 = 0;
        let mut pageofs: u16 = 0;
        let mut pos: u16 = 0;
        while textlines.wrapping_add(400) >= pos {
            for y in (0..200).rev() {
                let d = pos.wrapping_sub(linestart[y]) as i16;
                let bp = if d < 0 || d as u16 >= textlines { 0 } else { (d as u16).wrapping_mul(0x2a) as usize };
                let l = &lines[y];
                let mut di = pageofs.wrapping_add(l.addr);
                let mut al = 0u8;
                let mut dbit = l.dstbit;
                for &sx in &l.samples {
                    let b = text.get(bp + (sx >> 3) as usize).copied().unwrap_or(0);
                    if b & (0x80 >> (sx & 7)) != 0 {
                        al |= 0x80 >> dbit;
                    }
                    dbit += 1;
                    if dbit == 8 {
                        dbit = 0;
                        ega.wr(di, al);
                        di = di.wrapping_add(1);
                        al = 0;
                    }
                }
                if dbit != 0 {
                    ega.wr(di, al);
                }
            }
            self.ega_set_screen(ega, pageofs, 0);
            page ^= 1;
            pageofs = page << 15;
            let now = self.sys.time_count();
            tics = tics.wrapping_add((now - self.lasttimecount) as u16);
            self.lasttimecount = now;
            if tics > 0x14 {
                tics = 0x14;
            }
            pos = pos.wrapping_add(tics >> 2);
            tics &= 3;
            if self.intro_aborted() {
                break;
            }
        }
    }

    /// StarWars (0x1539e): the story text scrolling into the distance.
    fn star_wars(&mut self) {
        let mut ega = Ega::new();
        self.set_palette_regs(&PAL_BLACK);
        ega.clear_video();
        ega.set_line_width(0x2a);
        self.ega_set_screen(&ega, 0, 0);
        let (text, textlines) = self.render_story();
        let pic = &self.data.gfx.pics[STARWARSPIC - super::consts::STARTPICS];
        ega.draw_pic(0x8000, 0, 0, pic);
        self.set_palette_regs(&PAL_STARWARS);
        let (lines, linestart) = self.compile_sw(&mut ega);
        if self.sys.input.last_scan == 0 && self.alive() {
            self.start_music(STARWARSMUSIC);
            self.scroll_sw(&mut ega, &lines, &linestart, &text, textlines);
            self.stop_music();
        }
        self.sys.surface.clear(0);
        self.set_default_colors();
        self.check_last_scan();
    }
}

// ---------------------------------------------------------------------------
// title, demos, high scores, demo loop

/// RF hook that draws the high score table "into the master screen".
fn high_scores_hook(g: &mut Game) {
    g.draw_high_scores_text();
}

impl Game {
    /// IN_ReadControl including demo playback (0x23c19)
    pub fn read_control(&mut self) -> ControlInfo {
        if self.demomode == 2 {
            let d = &mut self.demo;
            if d.pos + 1 >= d.buffer.len() {
                self.demomode = 3;
                return ControlInfo::default();
            }
            let b = d.buffer[d.pos + 1];
            let my = (b & 3) as i32 - 1;
            let mx = ((b >> 2) & 3) as i32 - 1;
            let buttons = (b >> 4) & 3;
            d.buffer[d.pos] = d.buffer[d.pos].wrapping_sub(1);
            if d.buffer[d.pos] == 0 {
                d.pos += 2;
                if d.pos >= d.buffer.len() {
                    self.demomode = 3;
                }
            }
            let dir = match (mx, my) {
                (-1, -1) => Direction::NorthWest,
                (0, -1) => Direction::North,
                (1, -1) => Direction::NorthEast,
                (-1, 0) => Direction::West,
                (1, 0) => Direction::East,
                (-1, 1) => Direction::SouthWest,
                (0, 1) => Direction::South,
                (1, 1) => Direction::SouthEast,
                _ => Direction::None,
            };
            return ControlInfo { button0: buttons & 1 != 0, button1: buttons & 2 != 0, button2: false, xaxis: mx, yaxis: my, dir };
        }
        let c = self.sys.input.read_control();
        if self.demomode == 1 {
            self.record_control(&c);
        }
        c
    }

    /// The demo recording part of IN_ReadControl: runs of identical input
    /// bytes are stored as (count, input) pairs.
    fn record_control(&mut self, c: &ControlInfo) {
        let buttons = c.button0 as u8 | (c.button1 as u8) << 1;
        let dbyte = buttons << 4 | ((c.xaxis.clamp(-1, 1) + 1) as u8) << 2 | (c.yaxis.clamp(-1, 1) + 1) as u8;
        let d = &mut self.demo;
        let o = d.pos;
        if d.buffer[o + 1] == dbyte && d.buffer[o] < 255 {
            d.buffer[o] += 1;
            return;
        }
        if o != 0 || d.buffer[o] != 0 {
            d.pos += 2;
        }
        if d.pos + 1 >= d.buffer.len() {
            self.quit("Demo buffer overflow");
            return;
        }
        d.buffer[d.pos] = 1;
        d.buffer[d.pos + 1] = dbyte;
    }

    /// StartDemoRecord (0x1602a), F10+D: warps to a level and records.
    pub fn start_demo_record(&mut self) {
        let Some(s) = self.debug_input(0x1e, "  Record a demo from level(0-21):", 2) else { return };
        let Ok(level) = s.trim().parse::<i32>() else { return };
        if (0..0x16).contains(&level) {
            self.playstate = EX_WARPED;
            self.gs.mapon = level;
            // IN_StartDemoRecord(0x1000)
            self.demo.buffer = vec![0; 0x1000];
            self.demo.pos = 0;
            self.demomode = 1;
        }
    }

    /// EndDemoRecord (0x160ba), F10+D while recording: saves DEMOn.CK6 to
    /// the data directory.
    pub fn end_demo_record(&mut self) {
        // IN_StopDemo
        if self.demomode == 1 && self.demo.pos != 0 {
            self.demo.pos += 2;
        }
        self.demomode = 0;
        let input = self.debug_input(0x16, "  Save as demo #(0-9):", 1);
        if let Some(n) = input.and_then(|s| s.chars().next()).filter(|c| c.is_ascii_digit()) {
            let len = self.demo.pos.min(self.demo.buffer.len());
            let mut out = Vec::with_capacity(len + 4);
            out.extend_from_slice(&(self.ca_mapon as u16).to_le_bytes());
            out.extend_from_slice(&(len as u16).to_le_bytes());
            out.extend_from_slice(&self.demo.buffer[..len]);
            if std::fs::write(self.data.dir.join(format!("DEMO{}.{}", n, super::LAYOUT.ext)), out).is_err() {
                self.quit("EndDemoRecord:  Cannot write demo file!");
            }
        }
        // IN_FreeDemoBuffer
        self.demo.buffer.clear();
        self.demo.pos = 0;
    }

    /// CheckLastScan (0x13bb3): a key pressed during the demo loop opens
    /// the control panel.
    pub fn check_last_scan(&mut self) {
        if self.sys.input.last_scan == 0 || !self.alive() {
            return;
        }
        // Keen 4/5: F1 shows the help screens instead of the control panel
        // (Keen 4 checks storedemo first, Keen 5 afterwards)
        if super::EPISODE != 6 && !self.storedemo && self.sys.input.last_scan == SC_F1 {
            self.help_screens();
            return;
        }
        self.last_scan_action();
    }

    /// The part of CheckLastScan after a key was seen (start a stored demo
    /// game, or open the control panel).
    fn last_scan_action(&mut self) {
        if self.storedemo {
            self.playstate = EX_RESETGAME;
            self.restartgame = true;
            self.restartdifficulty = GD_NORMAL;
            self.sys.input.clear_keys_down();
            self.new_game();
        } else {
            self.control_panel_from_demo();
        }
    }

    /// US_ControlPanel from the demo loop; a chosen game ends the loop.
    fn control_panel_from_demo(&mut self) {
        self.control_panel(false);
        if self.restartgame || self.restartdifficulty != 0 {
            self.playstate = EX_RESETGAME;
        } else if self.loadedgame {
            self.playstate = EX_LOADEDGAME;
        }
    }

    /// ShowTitle (0x154a6)
    fn show_title(&mut self) {
        self.draw_pic(0, 0, TITLEPICPIC);
        self.sys.present();
        self.in_user_input(0x1a4, false);
        self.check_last_scan();
    }

    /// RunDemo (0x1550c): plays one of the recorded demos (4 = the high
    /// score screen).
    pub fn run_demo(&mut self, n: usize) {
        let chunk = self.data.gfx.externs[DEMO0 - super::consts::STARTEXTERNS + n].clone();
        self.run_demo_data(&chunk);
    }

    /// Plays a demo in the DEMOn.CK6 format: map, length, (count, input)
    /// pairs.
    pub fn run_demo_data(&mut self, chunk: &[u8]) {
        self.new_game();
        if chunk.len() < 4 {
            return;
        }
        self.gs.mapon = u16::from_le_bytes([chunk[0], chunk[1]]) as i16 as i32;
        let len = u16::from_le_bytes([chunk[2], chunk[3]]) as usize;
        let data = chunk[4..(4 + len).min(chunk.len())].to_vec();
        // IN_StartDemoPlayback
        self.demo.buffer = data;
        self.demo.pos = 0;
        self.demomode = 2;
        if self.setup_game_level(true) {
            if self.demo.scorescreenkludge {
                self.draw_high_scores();
            }
            self.play_loop();
        }
        // IN_StopDemo
        self.demomode = 0;
        if self.demo.scorescreenkludge {
            self.rf.bg_hook = None;
        }
        self.check_last_scan();
    }

    /// ShowHighScores (0x157d6)
    fn show_high_scores(&mut self) {
        self.demo.scorescreenkludge = true;
        if super::EPISODE == 5 {
            // Keen 5 (0x15bf9): IN_ClearKeysDown first
            self.sys.input.clear_keys_down();
        }
        self.run_demo(4);
        self.demo.scorescreenkludge = false;
    }

    /// DrawHighScores (0x155e7): the table is printed into the master
    /// screen, i.e. behind the actors; here it is redrawn by an RF hook.
    fn draw_high_scores(&mut self) {
        self.rf_new_position(0, 0);
        self.demo.hs_origin = (self.cam.originxglobal, self.cam.originyglobal);
        self.rf.bg_hook = Some(high_scores_hook);
    }

    fn draw_high_scores_text(&mut self) {
        let dx = (self.demo.hs_origin.0 - self.cam.originxglobal) >> 4;
        let dy = (self.demo.hs_origin.1 - self.cam.originyglobal) >> 4;
        if super::EPISODE == 5 {
            self.tw.fontcolor = HS_COLOR;
        }
        for i in 0..MAXSCORES {
            let e = self.demo.scores[i].clone();
            self.tw.printy = (i as i32) * 0x10 + HS_TOP + dy;
            self.tw.printx = HS_NAMEX + dx;
            self.us_print(&String::from_utf8_lossy(&e.name));
            if super::EPISODE == 4 {
                // one elder icon per rescued council member
                self.tw.printx = 0x98 + dx;
                for _ in 0..(e.completed as u16) {
                    let (x, y) = (self.tw.printx, self.tw.printy + 1);
                    self.draw_tile8(x, y, HS_ELDERTILE);
                    self.tw.printx += 8;
                }
            }
            let mut digits = e.score.to_string().into_bytes();
            for c in digits.iter_mut() {
                *c = c.wrapping_add(0x51);
            }
            let (w, _) = self.measure_string(&digits);
            self.tw.printx = HS_SCORERIGHT - w + dx;
            let y = self.tw.printy;
            self.draw_string_at(&digits, self.tw.printx, y);
            self.tw.printx += w;
        }
        self.tw.fontcolor = 15;
    }

    /// CheckHighScore (0x1569a): enters a new score into the table and
    /// lets the player type a name.
    pub fn check_high_score(&mut self, score: i32, levels: i32) {
        let mut n: Option<usize> = None;
        for i in 0..MAXSCORES {
            let e = &self.demo.scores[i];
            if e.score < score || (e.score == score && (e.completed as u16) < levels as u16) {
                n = Some(i);
                break;
            }
        }
        let Some(n) = n else { return };
        for i in (n + 1..MAXSCORES).rev() {
            self.demo.scores[i] = self.demo.scores[i - 1].clone();
        }
        self.demo.scores[n] = HighScore { name: Vec::new(), score, completed: levels };
        self.demo.scores_changed = true;

        self.demo.scorescreenkludge = true;
        self.gs.mapon = HIGHSCORELEVEL;
        self.setup_game_level(true);
        self.draw_high_scores();
        if super::EPISODE == 5 {
            self.tw.fontcolor = HS_COLOR;
        }
        self.rf_refresh();
        self.rf_refresh();
        self.rf.bg_hook = None;
        self.tw.printy = n as i32 * 0x10 + HS_TOP;
        self.tw.printx = HS_NAMEX;
        let y = self.tw.printy;
        if let Some(name) = self.us_line_input(HS_NAMEX, y, None, true, 0x39, 0x70) {
            self.demo.scores[n].name = name.into_bytes();
        }
        self.demo.scorescreenkludge = false;
        if super::EPISODE == 5 {
            self.tw.fontcolor = 15;
        }
    }

    /// Starts a new game (or the loaded game) chosen in the control panel
    /// and shows the high scores / title afterwards (inner loop of
    /// DemoLoop).
    fn run_chosen_games(&mut self) {
        while self.alive() && (self.playstate == EX_RESETGAME || self.playstate == EX_LOADEDGAME) {
            // GameLoop consumes restartgame (restartgame = gd_Continue)
            self.restartgame = false;
            self.game_loop();
            if !self.alive() {
                return;
            }
            self.show_high_scores();
            if self.playstate == EX_RESETGAME || self.playstate == EX_LOADEDGAME {
                continue;
            }
            self.show_title();
        }
    }

    /// DemoLoop (0x13a4f): title, demos and story until a game is started.
    pub fn demo_loop(&mut self) {
        // InitGame (0x137e2)
        self.tw.fontcolor = 15;
        if let Some(l) = self.opts.start_level {
            // tedlevel: start the level directly, exit afterwards (TEDDeath)
            self.new_game();
            self.gs.mapon = l;
            self.restartdifficulty = self.opts.start_difficulty.unwrap_or(GD_NORMAL);
            self.playstate = EX_STILLPLAYING;
            self.game_loop();
            self.request_quit = true;
            return;
        }
        self.playstate = EX_STILLPLAYING;
        let mut i = 0;
        while self.alive() {
            let n = i;
            i += 1;
            match n {
                // Keen 5: with NOPAN (not supported here) only the title
                0 => self.terminator(),
                1 => self.run_demo(0),
                2 => self.star_wars(),
                3 => self.run_demo(1),
                4 => self.show_high_scores(),
                5 => self.run_demo(2),
                6 => {
                    i = 0;
                    self.run_demo(3);
                }
                _ => {}
            }
            self.run_chosen_games();
        }
    }
}
