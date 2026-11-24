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
/// help_menu results: Escape, and the hand on the HELP title (the new
/// features, not in the original)
const HELPEXIT: i32 = -100;
const NEWHELP: i32 = -1;

/// The help pages about this version (not in the original), written like
/// the ComputerWrist help: `{H}` heading colour, `{B}` text colour, `{T}`
/// title colour, `{D}` the page decoration, `{K}` the program name.
const NEW_HELP: &str = r#"^P
{D}{T}WHAT'S NEW
{B}
This version of Commander Keen runs on today's computers, and it has learned a few new tricks along the way. Don't worry: the creatures, the levels and their secrets are just as they always were.

Use the arrow keys to page forward and backward. When you are done reading the text, press Esc.

Press the right arrow key now to go on.
^P
{D}{H}WIDESCREEN
{B}
Keen no longer has to peek at his world through a little box. Make the window as wide as you like, or switch to fullscreen, and you will see more of the level around you. The ComputerWrist and the id intro stretch along with it.

Press F11 (or Cmd+F on a Mac, Win+F on Windows) to switch between a window and fullscreen.
^P
{D}{H}ZOOM
{B}
Keen shows you the level at 75 percent, so you see more of it. With two or more players it zooms out to 50 percent, with one player back to 75. Press the minus key while you play to zoom out, and the plus key to zoom in, up to the original 100 percent.

You can also change Zoom under Extended Options. Keen remembers your choice.
^P
{D}{H}GAMEPADS
{B}
If you plug in a gamepad, the Configure menu shows Use Gamepad instead of the joystick choices. There you can set up the movement and the buttons for jump, pogo, fire, the ComputerWrist, saving and loading. Default Layout puts everything back the way it was.

To set a button, move to it, press Enter, then press the button on the gamepad. Pressing Space, or the button it already has, clears it.
^P
{D}{H}GAMEPADS (cont'd.)
{B}
In the ComputerWrist the gamepad moves the hand, the jump button chooses an item, and the pogo, Start or Select buttons back out.  When the ComputerWrist asks a question, the fire button means "yes".

No calibration is needed.  Even cheap gamepads that the computer doesn't know work fine.
^P
{D}{H}MULTIPLAYER
{B}
Bring your friends: up to four players can play at once! Choose the number of Players under Extended Options or Configure, Use Gamepad (two players are chosen by themselves when two gamepads are plugged in at the start). Every player needs a gamepad, except the last one, who may use the keyboard.

Player 2 wears a blue helmet and a green shirt, player 3 a red helmet and a cyan shirt, player 4 a green helmet and a red shirt. Each player has his own score box.
^P
{D}{H}MULTIPLAYER (cont'd.)
{B}
The view follows the leading Keen. A Keen lost off the screen is brought back. On the world map only the leading player walks, and the others tag along behind him.

Keys, gems and lives are shared (you start with three lives for each player), shots you find go to everybody, but each Keen keeps his own score.
^P
{D}{H}MULTIPLAYER (cont'd.)
{B}

Careful where you aim: a stunner shot makes another Keen dizzy for two seconds. He falls off his pogo stick, sits down and sees stars. Hit the leading Keen, and the view follows you.

When the leading Keen dies, the next one plays on, and the view follows him. The fallen Keen comes back for one of the lives. If there are none left, he waits for the next level.
^P
{D}{H}BATTLE
{B}
New Game has a Battle: two to four players play one level against each other. Choose the players, how hard it is, the level (or a random one) and the time with the left and right arrow keys.

Collect items and stun monsters for points. Items come back after ten seconds. On Easy there are no monsters, on Hard they wake up again after five seconds.
^P
{D}{H}BATTLE (cont'd.)
{B}
Nobody can leave the level, not even through a door. A shot at another Keen knocks him out for a moment, and if the screen was following him, it follows you now.

The score boxes count how often each Keen was knocked out (0, -1, -2...), and the clock at the top shows the time left. When it runs out, the high score screen shows everybody's knock-outs and score.
^P
{D}{H}EXTENDED OPTIONS
{B}
The main menu of the ComputerWrist has Extended Options, on two pages (the arrow keys turn them). On the first you can show the inventory, zoom out, choose the number of players, play with unlimited lives or shots and enter finished levels again; on the second add lives and shots and look at The Keenest (the high scores, quietly, until you press a key), even in the middle of a game. End Game is there, too. Keen remembers your choices.

The inventory at the bottom right shows the gems and other items you are carrying.
^P
{D}{H}QUICK SAVE
{B}
Is that last jump giving you trouble? Press the gamepad's Save State button to save the game right where you are, and Load State brings you back there. You can also load the quick save from Load Game: press the down arrow key past the last game, and you will find it on the second page.

F2 saves the game in the slot you used last, and F3 loads it again, without any questions. If you haven't saved or loaded a game yet, F2 and F3 use the quick save.
^P
{D}{H}AUTOSAVE
{B}
Every five seconds your game is saved quietly in the background. If your computer crashes, or you just quit, Keen will ask you whether you want to resume your last game the next time you start. He won't save while he is being eaten, of course!

The autosave is on the second page of Load Game, too.
^P
{D}{H}MUSIC AND SOUND
{B}
Choices for Music are now none, Adlib/Sound Blaster, and MIDI Music. MIDI Music plays the same tunes with the instruments of a SoundFont (a .sf2 file in the game's directory). If there is no SoundFont, MIDI Music will have an "X" by it.

The Sound and Music menus both have a Volume bar. Move to it and press the left and right arrow keys to make things quieter or louder.
^P
{D}{H}COMMAND LINE PARAMETERS
{B}
Here are some new command line parameters:
  {K} --no-fullscreen  (Start in a window.)
  {K} --window-scale 4  (Make the window four times as big.)
  {K} --classic-view  (Keep the original narrow view.)
  {K} --classic  (Play the game just as it was in 1991, with none of the new features.)
^P
{D}{H}COMMAND LINE PARAMETERS (cont'd.)
{B}
  {K} --no-auto-save  (Don't save the game every five seconds.)
  {K} --zoom 50  (Zoom out to 50 percent.)
  {K} --level 3  (Jump right into level 3.)
  {K} --difficulty hard  (Use with --level: easy, normal or hard.)
  {K} --maps DIR  (Play the levels you made with the Commander Keen Editor.)
^P
{D}{H}COMMAND LINE PARAMETERS (cont'd.)
{B}
  {K} --audio-pack DIR  (New sound effects and music.)
  {K} --gfx-pack DIR  (High resolution graphics.)
  {K} --soundfont FILE  (The SoundFont for MIDI Music.)
  {K} --export-midi DIR  (Write the music out as MIDI files.)
  {K} --export-sfx DIR  (Write the sound effects out as WAV files.)
  {K} --help  (Show all of the parameters.)
^P
{D}{C}CONCLUSION
{B}
Now go try out the new tricks, and have fun playing "{N}"!
^E
"#;

/// The new help pages with the colours and decoration of the episode's
/// help texts.
fn new_help_text() -> Vec<u8> {
    let (h, b, t, d, k, n) = if super::EPISODE == 5 {
        ("^cb", "^ce", "^cF", "", "keen5", "The Armageddon Machine")
    } else {
        ("^c4", "^c8", "^c9", "^G184,108,11\n", "keen4", "Secret of the Oracle")
    };
    let c = if super::EPISODE == 5 { "^cF" } else { "^C2" };
    NEW_HELP
        .replace("{H}", h)
        .replace("{B}", b)
        .replace("{T}", t)
        .replace("{C}", c)
        .replace("{D}", d)
        .replace("{K}", k)
        .replace("{N}", n)
        .into_bytes()
}

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
        self.draw_help_menu();
        let mut ycursor: i32 = 0;
        // the hand can point at the HELP title: the new features (not in
        // the original, not with --classic)
        let first = if self.opts.classic { 0 } else { NEWHELP };
        self.sys.input.clear_keys_down();
        loop {
            if !self.alive() {
                return -1;
            }
            // follow window resizes
            if self.set_screen_width(true) {
                self.sys.surface.clear(BACKCOLOR);
                self.draw_help_menu();
            }
            let t = &mut self.text;
            if t.helpmenupos < first {
                t.helpmenupos = first;
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
                        return HELPEXIT;
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

    /// The background, border and title of the help menu.
    fn draw_help_menu(&mut self) {
        self.sys.surface.bar(0, 0, 320, 200, BACKCOLOR);
        if super::EPISODE == 5 {
            self.draw_pic(0, 0, H_TOPWINDOWPIC);
            self.draw_pic(0, 8, H_LEFTWINDOWPIC);
            self.draw_pic(312, 8, H_RIGHTWINDOWPIC);
            self.draw_pic(8, 192, H_BOTTOMWINDOWPIC);
        }
        self.draw_pic(96, 8, H_HELPPIC);
    }

    /// HelpScreens (K4 0x1941b, K5 0x1ea5d): F1 help, a topic menu and the
    /// pages of the chosen text (arrows / PgUp / PgDn, Escape back to the
    /// menu, Escape in the menu leaves). Keen 5 plays its help music.
    /// Not in the original: the screen is as wide as the window (the gray
    /// fills it, the pages stay centered), and the topic above the first
    /// one shows what is new in this version.
    pub fn help_screens(&mut self) {
        let was_wide = self.sys.surface.logical_w() != 320;
        self.set_screen_width(true);
        self.help_screens_inner();
        self.set_screen_width(was_wide);
    }

    fn help_screens_inner(&mut self) {
        use crate::engine::input::*;
        self.sys.surface.clear(BACKCOLOR);
        if super::EPISODE == 5 {
            self.start_music(HELPMUSIC);
        }
        loop {
            let n = self.help_menu();
            self.sys.surface.clear(BACKCOLOR);
            if n == HELPEXIT || !self.alive() {
                self.sys.input.clear_keys_down();
                self.sys.surface.clear(BACKCOLOR);
                if super::EPISODE == 5 {
                    self.stop_music();
                }
                return;
            }
            self.text.buf = if n == NEWHELP {
                new_help_text()
            } else {
                let chunk = HELPEXTERN[n as usize];
                self.data.gfx.externs.get(chunk - STARTEXTERNS).cloned().unwrap_or_default()
            };
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
                let mut resized = false;
                while self.sys.input.last_scan == 0 && self.alive() {
                    self.sys.wait_vbl(1);
                    if self.set_screen_width(true) {
                        resized = true;
                        break;
                    }
                }
                if resized {
                    // lay the same page out again on the new screen
                    self.sys.surface.clear(BACKCOLOR);
                    self.back_page();
                    newpage = true;
                    continue;
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
