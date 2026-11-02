//! ID_US_1 text window helpers: windows built from tile8 border pieces,
//! proportional text printed in XOR mode (like VW pdrawmode = XOR).

use super::Game;
use crate::engine::video::string_width;

#[derive(Clone, Copy, Default)]
pub struct TextWin {
    pub printx: i32,
    pub printy: i32,
    pub windowx: i32,
    pub windowy: i32,
    pub windoww: i32,
    pub windowh: i32,
    pub fontnumber: usize,
    pub fontcolor: u8,
}

impl Game {
    pub fn measure_string(&self, s: &[u8]) -> (i32, i32) {
        let f = &self.data.gfx.fonts[self.tw.fontnumber];
        (string_width(f, s), f.height as i32)
    }

    pub fn draw_string_at(&mut self, s: &[u8], x: i32, y: i32) {
        let f = &self.data.gfx.fonts[self.tw.fontnumber];
        let mut cx = x;
        for &c in s {
            cx += self.sys.surface.glyph_xor(f, c, cx, y, self.tw.fontcolor);
        }
    }

    /// US_Print
    pub fn us_print(&mut self, s: &str) {
        let lines: Vec<&[u8]> = s.as_bytes().split(|&c| c == b'\n').collect();
        let last = lines.len() - 1;
        for (i, line) in lines.iter().enumerate() {
            let (w, h) = self.measure_string(line);
            let (x, y) = (self.tw.printx, self.tw.printy);
            self.draw_string_at(line, x, y);
            if i < last {
                self.tw.printx = self.tw.windowx;
                self.tw.printy += h;
            } else {
                self.tw.printx += w;
            }
        }
    }

    pub fn us_print_unsigned(&mut self, n: u32) {
        self.us_print(&n.to_string());
    }

    pub fn us_print_signed(&mut self, n: i32) {
        self.us_print(&n.to_string());
    }

    /// US_CPrintLine
    pub fn us_cprint_line(&mut self, s: &[u8]) {
        let (w, h) = self.measure_string(s);
        let x = self.tw.windowx + ((self.tw.windoww - w) as u16 >> 1) as i32;
        let y = self.tw.printy;
        self.draw_string_at(s, x, y);
        self.tw.printy += h;
    }

    /// US_CPrint: every line centered horizontally.
    pub fn us_cprint(&mut self, s: &str) {
        for line in s.as_bytes().split(|&c| c == b'\n') {
            self.us_cprint_line(line);
        }
    }

    /// US_PrintCentered: centered in the window both ways.
    pub fn us_print_centered(&mut self, s: &str) {
        let (w, h) = self.measure_string(s.as_bytes());
        let x = self.tw.windowx + ((self.tw.windoww - w) as u16 >> 1) as i32;
        let y = self.tw.windowy + ((self.tw.windowh - h) as u16 >> 1) as i32;
        self.draw_string_at(s.as_bytes(), x, y);
    }

    /// US_ClearWindow
    pub fn us_clear_window(&mut self) {
        let t = self.tw;
        self.sys.surface.bar(t.windowx, t.windowy, t.windoww, t.windowh, 15);
        self.tw.printx = t.windowx;
        self.tw.printy = t.windowy;
    }

    pub fn draw_tile8m(&mut self, x: i32, y: i32, t: i32) {
        if let Some(tex) = self.gfx.tile8m.get(t.max(0) as usize) {
            self.sys.surface.blit(tex, x, y);
        }
    }

    /// US_DrawWindow (coordinates in 8 pixel units)
    pub fn us_draw_window(&mut self, x: i32, y: i32, w: i32, h: i32) {
        self.tw.printx = x * 8;
        self.tw.printy = y * 8;
        self.tw.windoww = w * 8;
        self.tw.windowh = h * 8;
        self.tw.windowx = self.tw.printx;
        self.tw.windowy = self.tw.printy;
        let sx = (x - 1) * 8;
        let sy = (y - 1) * 8;
        let sw = (w + 1) * 8;
        let sh = (h + 1) * 8;
        self.us_clear_window();
        self.draw_tile8m(sx, sy, 0);
        self.draw_tile8m(sx, sy + sh, 6);
        let mut i = sx + 8;
        while i <= sx + sw - 8 {
            self.draw_tile8m(i, sy, 1);
            self.draw_tile8m(i, sy + sh, 7);
            i += 8;
        }
        self.draw_tile8m(i, sy, 2);
        self.draw_tile8m(i, sy + sh, 8);
        let mut j = sy + 8;
        while j <= sy + sh - 8 {
            self.draw_tile8m(sx, j, 3);
            self.draw_tile8m(sx + sw, j, 5);
            j += 8;
        }
    }

    /// US_CenterWindow
    pub fn us_center_window(&mut self, w: i32, h: i32) {
        self.us_draw_window(((40 - w) as u16 >> 1) as i32, ((25 - h) as u16 >> 1) as i32, w, h);
    }
}
