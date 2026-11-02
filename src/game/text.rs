//! CK_TEXT.C: text page layout engine (code segment 0x1d20, linear
//! 0x1d20d..0x1da07). In Keen 6 v1.4 it is only used for the ending
//! (FinaleLayout with the T_ENDART text); there is no HelpScreens.
//! Keen 4 (K4 0x18be1..) and Keen 5 (K5 0x1e0c2..) also have the help
//! screens (HelpMenu + HelpScreens, F1); Keen 5 draws a border around the
//! pages and has a second ending text (T_SECRETART).
//!
//! Layout commands: ^P (page start), ^E (end of file), ^Cx (text colour,
//! hex digit), ^Gy,x,pic (graphic, adjusts the margins), ^Ty,x,pic,tics
//! (timed graphic), ^By,x,w,h (bar in BACKCOLOR), ^Ly,x (locate).

use super::Game;
use super::consts::{STARTEXTERNS, STARTPICS, T_ENDART};

/// Background colour of the text pages (light gray, Keen 5: red).
const BACKCOLOR: u8 = ep_sel!(7, 4, 7);
const WORDLIMIT: usize = 80;
const FONTHEIGHT: i32 = 10;
const TOPMARGIN: i32 = 10;
const LEFTMARGIN: i32 = 10;
const RIGHTMARGIN: i32 = 10;
const PICMARGIN: i32 = 8;
const SPACEWIDTH: i32 = 7;
const TEXTROWS: usize = 18;
const SCREENPIXWIDTH: i32 = 320;
const SCREENMID: i32 = SCREENPIXWIDTH / 2;

const H_FLASHARROW1PIC: i32 = ep_sel!(0x2f, 0x1a, 27);
const H_FLASHARROW2PIC: i32 = ep_sel!(0x30, 0x1b, 28);
/// Song passed to StartMusic by FinaleLayout.
const ENDINGMUSIC: i32 = ep_sel!(7, 14, 1);
/// Keen 4/5 help screens: H_HELPPIC (menu title), the menu cursor and the
/// text chunk of each of the five topics (helpextern, K4 DS:0x624, K5
/// DS:0x1646).
const H_HELPPIC: i32 = 6;
const H_HANDPIC: i32 = ep_sel!(0x2d, 0x18, 0);
const HELPEXTERN: [usize; 5] = ep_sel!(
    [STARTEXTERNS + 4, STARTEXTERNS + 6, STARTEXTERNS + 5, STARTEXTERNS + 10, STARTEXTERNS + 7],
    [STARTEXTERNS, STARTEXTERNS + 1, STARTEXTERNS + 2, STARTEXTERNS + 6, STARTEXTERNS + 3],
    [0; 5]
);
/// Keen 5 page border pictures (top, left, right, bottom with / without
/// page number line)
const H_TOPWINDOWPIC: i32 = 0x1c;
const H_LEFTWINDOWPIC: i32 = 0x1d;
const H_RIGHTWINDOWPIC: i32 = 0x1e;
const H_BOTTOMINFOPIC: i32 = 0x1f;
const H_BOTTOMWINDOWPIC: i32 = 0x20;
/// Keen 5: StartMusic argument of the help screens
const HELPMUSIC: i32 = 19;

/// Module-private state stored in `Game::text` (the CK_TEXT globals).
#[derive(Default, Clone)]
pub struct ModState {
    /// The text being laid out (`text` far pointer = buf + pos, DS:cb41).
    pub buf: Vec<u8>,
    pub pos: i32,
    /// DS:cb13
    pub layoutdone: bool,
    /// DS:cb15 / DS:caef
    pub leftmargin: [i32; TEXTROWS],
    pub rightmargin: [i32; TEXTROWS],
    /// DS:cb49
    pub rowon: i32,
    /// DS:cb47, cb45, cb3b, cb3f
    pub picx: i32,
    pub picy: i32,
    pub picnum: i32,
    pub picdelay: i32,
    /// DS:cb3d, cb39
    pub numpages: i32,
    pub pagenum: i32,
    /// Keen 4/5 helpmenupos (K4 DS:c667, K5 DS:c201)
    pub helpmenupos: i32,
    /// px / py (DS:59ac / 59ae, the VW print position)
    pub px: i32,
    pub py: i32,
}

impl ModState {
    /// Character at `text + off` (0 outside the text, like the zeroed
    /// memory after the cached chunk).
    fn at(&self, off: i32) -> u8 {
        let i = self.pos + off;
        if i < 0 {
            return 0;
        }
        self.buf.get(i as usize).copied().unwrap_or(0)
    }

    /// leftmargin[rowon] / rightmargin[rowon] (rows outside the table,
    /// possible after ^L, read as 0 / 0xffff).
    fn lmargin(&self) -> i32 {
        self.leftmargin.get(self.rowon as usize).copied().unwrap_or(0)
    }

    fn rmargin(&self) -> i32 {
        self.rightmargin.get(self.rowon as usize).copied().unwrap_or(0xffff)
    }

    fn cur(&self) -> u8 {
        self.at(0)
    }

    /// True once the pointer ran far past the end of the text (the original
    /// would read random memory; used to stop otherwise endless scans).
    fn overrun(&self) -> bool {
        self.pos < 0 || self.pos as usize > self.buf.len() + 1
    }

    /// RipToEOL (0x1d20d)
    fn rip_to_eol(&mut self) {
        loop {
            let c = self.cur();
            self.pos += 1;
            if c == b'\n' || self.overrun() {
                break;
            }
        }
    }

    /// ParseNumber (0x1d220)
    fn parse_number(&mut self) -> i32 {
        let mut c = self.cur() as i8;
        while !(b'0' as i8..=b'9' as i8).contains(&c) {
            if self.overrun() {
                return 0;
            }
            self.pos += 1;
            c = self.cur() as i8;
        }
        let mut buffer: Vec<u8> = Vec::new();
        loop {
            buffer.push(c as u8);
            self.pos += 1;
            c = self.cur() as i8;
            if !(b'0' as i8..=b'9' as i8).contains(&c) {
                break;
            }
        }
        atoi(&buffer)
    }

    /// ParsePicCommand (0x1d27b)
    fn parse_pic_command(&mut self) {
        self.picy = self.parse_number();
        self.picx = self.parse_number();
        self.picnum = self.parse_number();
        self.rip_to_eol();
    }

    /// ParseTimedCommand (0x1d299)
    fn parse_timed_command(&mut self) {
        self.picy = self.parse_number();
        self.picx = self.parse_number();
        self.picnum = self.parse_number();
        self.picdelay = self.parse_number();
        self.rip_to_eol();
    }
}

/// Borland atoi on a string of digits (16-bit int).
fn atoi(s: &[u8]) -> i32 {
    let mut n: i16 = 0;
    for &c in s {
        n = n.wrapping_mul(10).wrapping_add((c - b'0') as i16);
    }
    n as i32
}

fn toupper(c: u8) -> u8 {
    c.to_ascii_uppercase()
}

impl Game {
    /// TimedPicCommand (0x1d2be)
    fn timed_pic_command(&mut self) {
        self.text.parse_timed_command();
        // update the screen (VW_WaitVBL + VW_ScreenToScreen buffer->display)
        self.sys.wait_vbl(1);
        // wait for time
        self.sys.set_time_count(0);
        let delay = self.text.picdelay as i16 as i64 as u32;
        while delay > self.sys.time_count() as u32 {
            if !self.alive() {
                return;
            }
            let t = self.sys.time_count() + 1;
            self.sys.wait_until(t);
        }
        // draw pic
        let t = &self.text;
        let (x, y, n) = (t.picx & !7, t.picy, t.picnum);
        self.draw_pic(x, y, n);
    }

    /// HandleCommand (0x1d31f)
    fn handle_command(&mut self) {
        self.text.pos += 1;
        match toupper(self.text.cur()) {
            b'B' => {
                let t = &mut self.text;
                t.picy = t.parse_number();
                t.picx = t.parse_number();
                let picwidth = t.parse_number();
                let picheight = t.parse_number();
                let (x, y) = (t.picx, t.picy);
                self.sys.surface.bar(x, y, picwidth, picheight, BACKCOLOR);
                self.text.rip_to_eol();
            }
            b'P' | b'E' => {
                // ^P is start of next page, ^E is end of file
                self.text.layoutdone = true;
                self.text.pos -= 1;
            }
            b'C' => {
                // ^c<hex digit> changes text color
                self.text.pos += 1;
                let i = toupper(self.text.cur());
                if i.is_ascii_digit() {
                    self.tw.fontcolor = i.wrapping_sub(b'0');
                } else if (b'A'..=b'F').contains(&i) {
                    self.tw.fontcolor = i.wrapping_sub(b'A').wrapping_add(10);
                }
                self.tw.fontcolor ^= BACKCOLOR;
                self.text.pos += 1;
            }
            b'L' => {
                let t = &mut self.text;
                t.py = t.parse_number();
                t.rowon = (t.py - TOPMARGIN) / FONTHEIGHT;
                t.py = t.rowon * FONTHEIGHT + TOPMARGIN;
                t.px = t.parse_number();
                t.rip_to_eol();
            }
            b'T' => {
                // ^Tyyy,xxx,ppp,ttt waits ttt tics, then draws pic
                self.timed_pic_command();
            }
            b'G' => {
                // ^Gyyy,xxx,ppp draws graphic
                self.text.parse_pic_command();
                let (picx, picy, picnum) = (self.text.picx, self.text.picy, self.text.picnum);
                self.draw_pic(picx & !7, picy, picnum);
                let (pw, ph) = self
                    .data
                    .gfx
                    .pictable
                    .get((picnum - STARTPICS as i32).max(0) as usize)
                    .copied()
                    .unwrap_or((0, 0));
                let picwidth = pw as i32 * 8;
                let picheight = ph as i32;
                let picmid = picx + picwidth / 2;
                // adjust margins
                let margin = if picmid > SCREENMID {
                    picx - PICMARGIN // new right margin
                } else {
                    picx + picwidth + PICMARGIN // new left margin
                };
                let top = ((picy - TOPMARGIN) / FONTHEIGHT).max(0);
                let mut bottom = (picy + picheight - TOPMARGIN) / FONTHEIGHT;
                if bottom >= TEXTROWS as i32 {
                    bottom = TEXTROWS as i32 - 1;
                }
                let t = &mut self.text;
                for i in top..=bottom {
                    if picmid > SCREENMID {
                        t.rightmargin[i as usize] = margin;
                    } else {
                        t.leftmargin[i as usize] = margin;
                    }
                }
                // adjust this line if needed
                let lm = t.lmargin();
                if (lm as u16) > (t.px as u16) {
                    t.px = lm;
                }
            }
            _ => {}
        }
    }

    /// NewLine (0x1d53c)
    fn new_line(&mut self) {
        let t = &mut self.text;
        t.rowon += 1;
        if t.rowon == TEXTROWS as i32 {
            // overflowed the page, so skip until next page break
            t.layoutdone = true;
            loop {
                t.pos += 1;
                if t.overrun() {
                    return;
                }
                if t.cur() == b'^' {
                    let c = toupper(t.at(1));
                    if c == b'E' || c == b'P' {
                        t.layoutdone = true;
                        return;
                    }
                }
            }
        }
        t.px = t.lmargin();
        t.py += FONTHEIGHT;
    }

    /// HandleCtrls (0x1d59b)
    fn handle_ctrls(&mut self) {
        let c = self.text.cur();
        self.text.pos += 1;
        if c == b'\n' {
            self.new_line();
        }
    }

    /// HandleWord (0x1d5bc)
    fn handle_word(&mut self) {
        // copy the next word into [word]
        let mut word: Vec<u8> = Vec::with_capacity(WORDLIMIT);
        word.push(self.text.cur());
        self.text.pos += 1;
        while self.text.cur() as i8 > b' ' as i8 {
            word.push(self.text.cur());
            self.text.pos += 1;
            if word.len() == WORDLIMIT {
                self.quit("PageLayout: Word limit exceeded");
                return;
            }
        }

        // see if it fits on this line
        let (wwidth, _wheight) = self.measure_string(&word);
        loop {
            let t = &self.text;
            if (t.rmargin() as u16) >= (t.px + wwidth) as u16 {
                break;
            }
            self.new_line();
            if self.text.layoutdone {
                return; // overflowed page
            }
        }

        // print it
        let newpos = self.text.px + wwidth;
        let (x, y) = (self.text.px, self.text.py);
        self.draw_string_at(&word, x, y);
        self.text.px = newpos;

        // suck up any extra spaces
        while self.text.cur() == b' ' {
            self.text.px += SPACEWIDTH;
            self.text.pos += 1;
        }
    }

    /// PageLayout (0x1d66f)
    pub fn page_layout(&mut self, shownumber: bool) {
        let oldcolor = self.tw.fontcolor;
        self.tw.fontcolor = if super::EPISODE == 5 { 10 } else { 15 };
        // clear the screen
        self.sys.surface.bar(0, 0, 320, 200, BACKCOLOR);
        if super::EPISODE == 5 {
            self.draw_pic(0, 0, H_TOPWINDOWPIC);
            self.draw_pic(0, 8, H_LEFTWINDOWPIC);
            self.draw_pic(312, 8, H_RIGHTWINDOWPIC);
            if shownumber {
                self.draw_pic(8, 176, H_BOTTOMINFOPIC);
            } else {
                self.draw_pic(8, 192, H_BOTTOMWINDOWPIC);
            }
        }

        let t = &mut self.text;
        for i in 0..TEXTROWS {
            t.leftmargin[i] = LEFTMARGIN;
            t.rightmargin[i] = SCREENPIXWIDTH - RIGHTMARGIN;
        }
        t.px = LEFTMARGIN;
        t.py = TOPMARGIN;
        t.rowon = 0;
        t.layoutdone = false;

        // make sure we are starting layout text (^P first command)
        while t.cur() as i8 <= b' ' as i8 && !t.overrun() {
            t.pos += 1;
        }
        let mut ok = t.cur() == b'^';
        if ok {
            t.pos += 1;
            ok = toupper(t.cur()) == b'P';
        }
        if !ok {
            self.quit("PageLayout: Text not headed with ^P");
            return;
        }
        self.text.rip_to_eol();

        // process text stream
        while !self.text.layoutdone {
            if !self.alive() {
                return;
            }
            let ch = self.text.cur();
            if ch == b'^' {
                self.handle_command();
            } else if ch as i8 <= b' ' as i8 {
                self.handle_ctrls();
            } else {
                self.handle_word();
            }
            if self.text.overrun() {
                self.text.layoutdone = true;
            }
        }

        self.text.pagenum += 1;

        if shownumber {
            let s = format!("pg {} of {}", self.text.pagenum, self.text.numpages);
            if super::EPISODE == 5 {
                self.tw.fontcolor = 8;
                self.text.py = 186;
                self.text.px = 218;
            } else {
                self.tw.fontcolor = 15;
                self.text.py = 190;
                self.text.px = 248;
            }
            let (x, y) = (self.text.px, self.text.py);
            self.draw_string_at(s.as_bytes(), x, y);
            let (w, _) = self.measure_string(s.as_bytes());
            self.text.px += w;
        }
        if super::EPISODE == 5 {
            self.tw.fontcolor = oldcolor;
        }
    }

    /// BackPage (0x1d7d5)
    pub fn back_page(&mut self) {
        let t = &mut self.text;
        t.pagenum -= 1;
        loop {
            t.pos -= 1;
            if t.pos < 0 {
                t.pos = 0;
                return;
            }
            if t.cur() == b'^' && toupper(t.at(1)) == b'P' {
                return;
            }
        }
    }

    /// CacheLayoutGraphics (0x1d7fe): counts the pages (the graphics are
    /// always loaded in the port).
    pub fn cache_layout_graphics(&mut self) {
        let t = &mut self.text;
        let textstart = t.pos;
        let bombpoint = t.pos + 30000;
        t.numpages = 0;
        t.pagenum = 0;
        loop {
            if t.cur() == b'^' {
                t.pos += 1;
                let ch = toupper(t.cur());
                if ch == b'P' {
                    // start of a page
                    t.numpages += 1;
                }
                if ch == b'E' {
                    // end of file, so load graphics and return
                    t.pos = textstart;
                    return;
                }
                if ch == b'G' {
                    // draw graphic command, so mark graphics
                    t.parse_pic_command();
                }
                if ch == b'T' {
                    // timed draw graphic command, so mark graphics
                    t.parse_timed_command();
                }
            } else {
                t.pos += 1;
            }
            if t.pos >= bombpoint {
                break;
            }
        }
        self.quit("CacheLayoutGraphics: No ^E to terminate file!");
    }

    /// Waits up to 70 VBLs for IN_IsUserInput (LastScan).
    fn finale_wait(&mut self) -> bool {
        for _ in 0..70 {
            if self.sys.input.last_scan != 0 || !self.alive() {
                return true;
            }
            self.sys.wait_vbl(1);
        }
        false
    }

    /// FinaleLayout (0x1d8cc, K4 0x19593, K5 0x1ebf7): the ending text.
    /// Keen 5 shows T_SECRETART instead when the QED level was left by
    /// destroying the QED (leveldone[13] == ex_qedbroke).
    pub fn finale(&mut self) {
        self.sys.surface.clear(BACKCOLOR); // VW_ClearVideo(BACKCOLOR)
        let mut chunk = T_ENDART;
        if super::EPISODE == 5 && self.gs.leveldone[13] == 14 {
            chunk = T_ENDART + 1;
        }
        self.text.buf = self.data.gfx.externs.get(chunk - STARTEXTERNS).cloned().unwrap_or_default();
        self.text.pos = 0;
        self.cache_layout_graphics();
        self.start_music(ENDINGMUSIC);
        while self.text.pagenum < self.text.numpages {
            if !self.alive() {
                break;
            }
            self.page_layout(false);
            self.sys.input.clear_keys_down();
            self.sys.present(); // VW_SetScreen(bufferofs, 0)
            loop {
                self.draw_pic(298, 184, H_FLASHARROW1PIC);
                if self.finale_wait() {
                    break;
                }
                self.draw_pic(298, 184, H_FLASHARROW2PIC);
                if self.finale_wait() {
                    break;
                }
            }
        }
        self.stop_music();
        self.text.buf = Vec::new();
        self.sys.input.clear_keys_down();
        self.sys.surface.clear(BACKCOLOR);
    }

    /// HelpMenu (K4 0x192a2, K5 0x1e854): the topic menu of the help
    /// screens; returns the topic or -1 (Escape).
    fn help_menu(&mut self) -> i32 {
        self.sys.surface.bar(0, 0, 320, 200, BACKCOLOR);
        if super::EPISODE == 5 {
            self.draw_pic(0, 0, H_TOPWINDOWPIC);
            self.draw_pic(0, 8, H_LEFTWINDOWPIC);
            self.draw_pic(312, 8, H_RIGHTWINDOWPIC);
            self.draw_pic(8, 192, H_BOTTOMWINDOWPIC);
        }
        self.draw_pic(96, 8, H_HELPPIC);
        let mut ycursor: i32 = 0;
        self.sys.input.clear_keys_down();
        loop {
            if !self.alive() {
                return -1;
            }
            let t = &mut self.text;
            if t.helpmenupos < 0 {
                t.helpmenupos = 0;
            } else if t.helpmenupos > 4 {
                t.helpmenupos = 4;
            }
            let y = t.helpmenupos * 24 + 48;
            self.draw_pic(48, y, H_HANDPIC);
            self.sys.wait_vbl(1); // VW_UpdateScreen
            self.sys.surface.bar(48, y, 39, 24, BACKCOLOR);
            // IN_ReadControl(0, &control) + IN_ReadCursor (no mouse here)
            let c = self.sys.input.read_control();
            let cursory = 0;
            let ls = self.sys.input.last_scan;
            if ls != 0 {
                self.sys.input.clear_keys_down();
                match ls {
                    crate::engine::input::SC_UPARROW => self.text.helpmenupos -= 1,
                    crate::engine::input::SC_DOWNARROW => self.text.helpmenupos += 1,
                    crate::engine::input::SC_ESCAPE => {
                        self.sys.surface.clear(BACKCOLOR);
                        return -1;
                    }
                    crate::engine::input::SC_ENTER => {
                        self.sys.surface.clear(BACKCOLOR);
                        return self.text.helpmenupos;
                    }
                    _ => {}
                }
            }
            ycursor += cursory;
            if c.button0 || c.button1 {
                self.sys.surface.clear(BACKCOLOR);
                return self.text.helpmenupos;
            }
            if ycursor < -40 {
                ycursor += 40;
                self.text.helpmenupos -= 1;
            } else if ycursor > 40 {
                ycursor -= 40;
                self.text.helpmenupos += 1;
            }
        }
    }

    /// HelpScreens (K4 0x1941b, K5 0x1ea5d): F1 help, a topic menu and the
    /// pages of the chosen text (arrows / PgUp / PgDn, Escape back to the
    /// menu, Escape in the menu leaves). Keen 5 plays its help music.
    pub fn help_screens(&mut self) {
        use crate::engine::input::*;
        self.sys.surface.clear(BACKCOLOR);
        if super::EPISODE == 5 {
            self.start_music(HELPMUSIC);
        }
        loop {
            let n = self.help_menu();
            self.sys.surface.clear(BACKCOLOR);
            if n == -1 || !self.alive() {
                self.sys.input.clear_keys_down();
                self.sys.surface.clear(BACKCOLOR);
                if super::EPISODE == 5 {
                    self.stop_music();
                }
                return;
            }
            let chunk = HELPEXTERN[n as usize];
            self.text.buf = self.data.gfx.externs.get(chunk - STARTEXTERNS).cloned().unwrap_or_default();
            self.text.pos = 0;
            self.cache_layout_graphics();
            let mut newpage = true;
            loop {
                if !self.alive() {
                    return;
                }
                if newpage {
                    newpage = false;
                    self.page_layout(true);
                    self.sys.present(); // VW_SetScreen(bufferofs, 0)
                }
                self.sys.input.last_scan = 0;
                while self.sys.input.last_scan == 0 && self.alive() {
                    self.sys.wait_vbl(1);
                }
                let ls = self.sys.input.last_scan;
                match ls {
                    SC_UPARROW | SC_PGUP | SC_LEFTARROW => {
                        if self.text.pagenum > 1 {
                            self.back_page();
                            self.back_page();
                            newpage = true;
                        }
                    }
                    SC_DOWNARROW | SC_PGDN | SC_RIGHTARROW => {
                        if self.text.pagenum < self.text.numpages {
                            newpage = true;
                        }
                    }
                    _ => {}
                }
                if ls == SC_ESCAPE {
                    break;
                }
            }
            self.text.buf = Vec::new();
            self.sys.input.clear_keys_down();
        }
    }
}
