//! CK_GAME.C port: game loop, level setup, death handling.
//! Translated from segment 0x157f of KEEN6C.EXE.

use super::defs::*;
use super::play::{LEVELENTER, LEVELNAMES};
use super::Game;
use super::EPISODE as EPISODE_NUM;
use crate::engine::input::*;

impl Game {
    /// NewGame (0x1584e)
    pub fn new_game(&mut self) {
        self.gs = GameState::default();
        self.gs.nextextra = 20000;
        self.gs.lives = 3;
        self.gs.ammo = 5;
    }

    /// GameOver (0x158ad)
    pub fn game_over(&mut self) {
        self.us_center_window(16, 3);
        self.us_print_centered("Game Over!");
        self.sys.present();
        self.sys.input.clear_keys_down();
        self.user_input(4 * 70);
    }

    /// Loads a map into the working planes (CA_CacheMap).
    pub fn cache_map(&mut self, n: i32) -> bool {
        let Some(Some(m)) = self.data.maps.maps.get(n as usize) else {
            return false;
        };
        self.mapwidth = m.width as i32;
        self.mapheight = m.height as i32;
        self.mapname = m.name.clone();
        self.ca_mapon = n;
        self.mapplanes = m.planes.clone();
        true
    }

    /// Plants flags and removes blockers on the world map (0x15c6c).
    fn scan_world_flags(&mut self) {
        for y in 0..self.mapheight {
            for x in 0..self.mapwidth {
                let v = self.map_tile(2, x, y) as i32;
                let level = v & 0xff;
                // Keen 4 and 5 have flags for 17 levels, Keen 6 for 16
                if level == 0 || level >= ep_sel!(0x12, 0x12, 0x11) || self.gs.leveldone[level as usize] == 0 {
                    continue;
                }
                let kind = v >> 8;
                // --cheatmode "ALLOW RE-ENTER": finished levels keep their entry
                if kind == 0xc0 && self.reenter_allowed() {
                    continue;
                }
                self.set_map_tile(2, x, y, 0);
                if kind == 0xd0 {
                    self.set_map_tile(1, x, y, 0);
                } else if kind == 0xf0 {
                    // Keen 5 has no flag throwing (0x1607b)
                    if EPISODE_NUM != 5 && self.lastlevel == level {
                        self.spawn_throw_flag(x, y);
                    } else {
                        self.spawn_flag(x, y);
                    }
                }
            }
        }
    }

    /// The cheat option "ALLOW RE-ENTER" is on (never in demos).
    fn reenter_allowed(&self) -> bool {
        self.opts.allow_reenter && self.demomode == 0
    }

    /// Cheat card: adds or removes the entries of finished levels on the
    /// world map currently loaded, after "ALLOW RE-ENTER" was toggled.
    pub fn update_reenter_entries(&mut self) {
        let Some(Some(m)) = self.data.maps.maps.get(self.ca_mapon as usize) else {
            return;
        };
        let orig = m.planes[2].clone();
        let reenter = self.reenter_allowed();
        for y in 0..self.mapheight {
            for x in 0..self.mapwidth {
                let Some(&v) = orig.get((y * self.mapwidth + x) as usize) else {
                    continue;
                };
                let level = (v & 0xff) as i32;
                if v >> 8 != 0xc0 || level == 0 || level >= ep_sel!(0x12, 0x12, 0x11) || self.gs.leveldone[level as usize] == 0 {
                    continue;
                }
                self.set_map_tile(2, x, y, if reenter { v } else { 0 });
            }
        }
    }

    /// SetupGameLevel (0x15d75)
    pub fn setup_game_level(&mut self, loadnow: bool) -> bool {
        if self.demomode == 0 {
            self.init_rnd(true);
        } else {
            self.init_rnd(false);
            self.gs.difficulty = GD_NORMAL;
        }
        if !self.cache_map(self.gs.mapon) {
            return false;
        }
        self.rf_new_map();
        self.scan_info_plane();
        if self.gs.mapon == 0 {
            self.scan_world_flags();
        }
        self.rf_mark_tile_graphics_public();
        if loadnow {
            let text = if self.demomode == 0 {
                LEVELENTER.get(self.gs.mapon as usize).copied().unwrap_or("").to_string()
            } else {
                "DEMO".to_string()
            };
            self.level_entry_window(&text);
        }
        true
    }

    /// The "Keen attacks ..." box shown while a level loads (CA_CacheMarks
    /// with the DialogDraw/DialogUpdate callbacks, 0x15e39/0x15fb4).
    fn level_entry_window(&mut self, text: &str) {
        self.fade_out();
        self.sys.surface.clear(0);
        self.us_center_window(26, 8);
        let (wx, wy) = (self.tw.windowx, self.tw.windowy);
        // KEENCOUNT1PIC: the loading box picture, then 5 animation frames
        let countpic = ep_sel!(0x6b, 0x5c, 0x25);
        self.draw_pic(wx, wy, countpic);
        self.tw.windoww -= 0x30;
        self.tw.windowx += 0x30;
        let (_, h) = self.measure_multiline(text);
        self.tw.printy += ((self.tw.windowh - h) as u16 >> 1) as i32 - 4;
        self.us_cprint(text);
        self.screenfaded_present();
        self.fade_in();
        // the original animates Keen while loading the chunks: show the
        // frames at a fixed pace instead
        // (5 frames: the original stops after frame 4, 0x163e5 in Keen 5;
        // the picture after them is e.g. Keen 5's GAMEOVERPIC)
        for frame in 0..5 {
            let (x, y) = (self.tw.windowx - 0x18, self.tw.windowy + 0x28);
            self.draw_pic(x, y, countpic + 1 + frame);
            self.sys.wait_vbl(6);
        }
        self.sys.wait_vbl(10);
    }

    fn screenfaded_present(&mut self) {
        self.sys.present();
    }

    /// HandleDeath (0x161b5)
    pub fn handle_death(&mut self) {
        let name = LEVELNAMES.get(self.gs.mapon as usize).copied().unwrap_or("").to_string();
        let (_, h) = self.measure_multiline(&name);
        self.gs.keys = [0; 4];
        self.gs.lives -= 1;
        if self.gs.lives < 0 {
            return;
        }
        self.draw_playfield();
        self.us_center_window(20, 8);
        self.tw.printy += 3;
        self.us_cprint("You didn't make it past");
        let top = self.tw.printy + 0x16;
        if h < 15 {
            self.tw.printy += 4;
        }
        self.us_cprint(&name);
        self.tw.printy = top + 2;
        self.us_cprint("Try Again");
        let bottom = self.tw.printy + 2;
        self.tw.printy += 4;
        self.us_cprint(ep_sel!("Exit to Shadowlands", "Exit to Armageddon", "Exit to Fribbulus Xax"));
        self.sys.input.clear_keys_down();
        let mut sel = 0;
        loop {
            let y = if sel != 0 { bottom } else { top };
            let col = if (self.sys.time_count() >> 4) & 1 == 0 { 1 } else { 12 };
            self.selection_box(y, col);
            self.sys.present();
            self.sys.wait_vbl(1);
            self.selection_box(y, 15);
            if !self.alive() {
                return;
            }
            let ls = self.sys.input.last_scan;
            if ls == SC_ESCAPE {
                self.gs.mapon = 0;
                self.sys.input.clear_keys_down();
                return;
            }
            self.c = self.read_control();
            if self.c.button0 || self.c.button1 || ls == SC_ENTER || ls == SC_SPACE {
                if sel != 0 {
                    self.gs.mapon = 0;
                }
                self.sys.input.clear_keys_down();
                return;
            }
            if self.c.yaxis == -1 || ls == SC_UPARROW {
                sel = 0;
            } else if self.c.yaxis == 1 || ls == SC_DOWNARROW {
                sel = 1;
            }
            self.sys.input.last_scan = 0;
        }
    }

    /// Draws the rectangle around a menu entry of the death box.
    fn selection_box(&mut self, y: i32, color: u8) {
        let x1 = self.tw.windowx + 4;
        let x2 = self.tw.windowx + self.tw.windoww - 4;
        let s = &mut self.sys.surface;
        for dy in [0, 1, 12, 13] {
            s.bar(x1, y + dy, x2 - x1 + 1, 1, color);
        }
        for x in [x1, x1 + 1, x2, x2 - 1] {
            s.bar(x, y + 1, 1, 11, color);
        }
    }

    /// The "One moment" box when entering a level from the map.
    fn one_moment(&mut self) {
        self.us_center_window(26, 8);
        self.tw.printy += 25;
        self.us_cprint("One moment");
        self.sys.present();
    }

    /// GameLoop with the wide view enabled while it runs.
    pub fn game_loop(&mut self) {
        self.wide_ok = true;
        self.game_loop_inner();
        self.wide_ok = false;
        self.update_view();
    }

    /// GameLoop (0x16528)
    fn game_loop_inner(&mut self) {
        let mut skip_setup = self.playstate == EX_LOADEDGAME;
        if !skip_setup {
            if self.restartdifficulty != 0 {
                self.gs.difficulty = self.restartdifficulty;
            }
            self.restartdifficulty = 0;
        }
        self.restartgame = false;
        loop {
            if !skip_setup && !self.setup_game_level(true) {
                self.gs.mapon = 0;
                if !self.setup_game_level(true) {
                    self.quit("GameLoop: Insufficient memory to load world map!");
                    return;
                }
            }
            skip_setup = false;
            loop {
                self.scrolllock = false;
                self.sd.wait_sound_done(&mut self.sys);
                self.play_loop();
                if !self.alive() {
                    return;
                }
                // Keen 4 clears the keys after every level (0x16638), Keen 5
                // the keys and the keycard unless a game was loaded (0x1697c)
                ep4! { self.gs.keys = [0; 4]; }
                ep5! {
                    if self.playstate != EX_LOADEDGAME {
                        self.gs.keys = [0; 4];
                        self.gs.keycard = 0;
                    }
                }
                if self.playstate == EX_LOADEDGAME {
                    continue;
                }
                break;
            }
            self.lastlevel = -1;
            match self.playstate {
                EX_DIED => self.handle_death(),
                EX_RESCUED if EPISODE_NUM == 4 => {
                    // Keen 4: a council member was rescued (0x16732)
                    if self.ca_mapon != 0 {
                        self.play_sound(super::keen::SND_LEVELDONE);
                    }
                    let done = self.ca_mapon;
                    self.lastlevel = done;
                    // cheat "ALLOW RE-ENTER": a member is only rescued once
                    if self.reenter_allowed() && self.gs.leveldone.get(done as usize).is_some_and(|&d| d != 0) {
                        self.gs.mapon = 0;
                        continue;
                    }
                    if let Some(d) = self.gs.leveldone.get_mut(done as usize) {
                        *d = 1;
                    }
                    ep4! { self.rescued_member(); }
                    if self.gs.rescued != 8 {
                        self.gs.mapon = 0;
                    } else {
                        self.wide_ok = false;
                        self.update_view();
                        self.finale();
                        let (score, rescued) = (self.gs.score, self.gs.rescued);
                        self.check_high_score(score, rescued);
                        return;
                    }
                }
                EX_FUSEBROKE if EPISODE_NUM == 5 => {
                    // Keen 5: a fuse of the Armageddon Machine (0x16a79)
                    self.play_sound(super::keen::SND_LEVELDONE);
                    let done = self.ca_mapon;
                    self.lastlevel = done;
                    if let Some(d) = self.gs.leveldone.get_mut(done as usize) {
                        *d = EX_FUSEBROKE;
                    }
                    ep5! { self.finished_fuse(); }
                    self.gs.mapon = 0;
                }
                EX_QEDBROKE if EPISODE_NUM == 5 => {
                    // Keen 5: the QED is destroyed, the game is won (0x16aa5)
                    self.wide_ok = false;
                    self.update_view();
                    self.finale();
                    let score = self.gs.score;
                    self.check_high_score(score, 0);
                    return;
                }
                EX_COMPLETED | EX_FOOT | EX_SANDWICH | EX_HOOK | EX_CARD | EX_PORTOUT => {
                    match self.playstate {
                        EX_SANDWICH => self.got_sandwich(),
                        EX_HOOK => self.got_hook(),
                        EX_CARD => self.got_passcard(),
                        _ => {}
                    }
                    // the original tests the cached map (ID_CA mapon), not
                    // gamestate.mapon, which CheckEnterLevel has already set
                    // to the level being entered
                    if self.ca_mapon != 0 {
                        self.play_sound(super::keen::SND_LEVELDONE);
                        let done = self.ca_mapon;
                        self.gs.mapon = 0;
                        self.lastlevel = done;
                        // (maps beyond the original levels only exist in edited map sets)
                        let reenter = self.reenter_allowed();
                        if let Some(d) = self.gs.leveldone.get_mut(done as usize) {
                            // a re-entered level keeps its result (Keen 5: fuse broken)
                            if !(reenter && *d != 0) {
                                *d = 1;
                            }
                        }
                        if self.storedemo && done == 2 {
                            self.sys.input.clear_keys_down();
                            return;
                        }
                    } else {
                        self.one_moment();
                    }
                }
                EX_RESETGAME => {
                    if self.restartdifficulty != 0 {
                        self.gs.difficulty = self.restartdifficulty;
                    }
                    self.restartdifficulty = 0;
                    self.restartgame = false;
                    continue;
                }
                EX_ABORTGAME => {
                    self.sys.input.clear_keys_down();
                    return;
                }
                EX_MOLLY => {
                    self.wide_ok = false;
                    self.update_view();
                    self.finale();
                    self.check_high_score_end();
                    return;
                }
                _ => {}
            }
            if self.gs.lives < 0 {
                if EPISODE_NUM == 5 {
                    // Keen 5: the galaxy explodes (0x1f656)
                    self.wide_ok = false;
                    self.update_view();
                    ep5! { self.galaxy_explode(); }
                    let score = self.gs.score;
                    self.check_high_score(score, 0);
                    return;
                }
                self.game_over();
                self.wide_ok = false;
                self.update_view();
                self.check_high_score_end();
                return;
            }
        }
    }

    fn check_high_score_end(&mut self) {
        // Keen 4 records the rescued council members instead of the levels
        let done = if EPISODE_NUM == 4 {
            self.gs.rescued
        } else {
            self.gs.leveldone.iter().filter(|&&d| d != 0).count() as i32
        };
        let score = self.gs.score;
        self.check_high_score(score, done);
    }
}
