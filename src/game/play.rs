//! CK_PLAY.C port: the main play loop, camera scrolling, status window,
//! controls and music. Translated from segment 0x1672 of KEEN6C.EXE.

use super::defs::*;
use super::states::*;
use super::Game;
use super::EPISODE as EPISODE_NUM;
use crate::engine::input::*;

/// DS:0x2d8 song for each map
pub const LEVELMUSIC: [i32; 20] = ep_sel!(
    [0, 4, 3, 3, 2, 2, 4, 3, 1, 1, 1, 2, 2, 2, 2, 2, 2, 1, 3, -1],
    [11, 5, 7, 9, 10, 9, 10, 9, 10, 9, 10, 3, 13, 4, 12, 2, 6, 1, 0, 8],
    [5, 3, 1, 8, 8, 8, 7, 2, 7, 1, 3, 2, 1, 4, 4, 6, 2, 0, 0, 0]
);
/// DS:0x2a6 vertical screen shake offsets (Blooguard club smash)
pub const SHAKETABLE: [i32; 24] = [
    0, -64, -64, -64, 64, 64, 64, -200, -200, -200, 200, 200, 200, -250, -250, -250, 250, 250, 250, -250, -250, -250,
    250, 250,
];

pub const LEVELNAMES: &[&str] = ep_sel!(
    &[
        "Shadowlands",
        "Border Village",
        "Slug Village",
        "The Perilous Pit",
        "Cave of the Descendents",
        "Chasm of Chills",
        "Crystalus",
        "Hillville",
        "Sand Yego",
        "Miragia",
        "Lifewater Oasis",
        "Pyramid of the Moons",
        "Pyramid of Shadows",
        "Pyramid of the\nGnosticene Ancients",
        "Pyramid of the Forbidden",
        "Isle of Tar",
        "Isle of Fire",
        "Well of Wishes",
        "Bean-with-Bacon\nMegarocket",
    ],
    &[
        "Omegamatic",
        "Ion Ventilation System",
        "Security Center",
        "Defense Tunnel Vlook",
        "Energy Flow Systems",
        "Defense Tunnel Burrh",
        "Regulation\nControl Center",
        "Defense Tunnel Sorra",
        "Neutrino\nBurst Injector",
        "Defense Tunnel Teln",
        "Brownian\nMotion Inducer",
        "Gravitational\nDamping Hub",
        "Quantum\nExplosion Dynamo",
        "Korath III Base",
        "BWBMegarocket",
        "High Scores",
    ],
    &[
        "Fribbulus Xax",
        "Bloogwaters\nCrossing",
        "Guard Post One",
        "First Dome\nof Darkness",
        "Second Dome\nof Darkness",
        "The Bloogdome",
        "Bloogton Mfg.,\nIncorporated",
        "Bloogton Tower",
        "Bloogfoods, Inc.",
        "Guard Post Two",
        "Bloogville",
        "BASA",
        "Guard Post Three",
        "Bloogbase Rec\nDistrict",
        "Bloogbase Mgmt.\nDistrict",
        "Bloog Control Center",
        "Blooglab",
        "Bean-with-Bacon\nMegarocket",
        "High Scores",
    ]
);

pub const LEVELENTER: &[&str] = ep_sel!(
    &[
        "Keen enters the\nShadowlands",
        "Keen makes a run for\nthe Border Village",
        "Keen slips into\nSlug Village",
        "Keen plummets into\nthe The Perilous Pit",
        "Keen plods down into\nthe Cave of the\nDescendents",
        "Keen shivers along\nthe Chasm of Chills",
        "Keen reflects upon\nentering Crystalus",
        "Keen stumbles upon\nHillville",
        "Keen grits his teeth\nand enters Sand Yego",
        "Keen disappears into\nMiragia",
        "Keen crawls into\nLifewater Oasis",
        "Keen backs into the\nPyramid of the Moons",
        "Keen move silently in\nthe Pyramid of Shadows",
        "Keen reverently enters\nthe Pyramid of the\nGnosticene Ancients",
        "Keen hesitantly crosses\ninto the Pyramid of the\nForbidden",
        "Keen mucks along the\nIsle of Tar",
        "Keen blazes across the\nIsle of Fire",
        "Keen hopefully enters\nthe Well of Wishes",
        "Keen launches into the\nBean-with-Bacon\nMegarocket",
    ],
    &[
        "Keen purposefully\nwanders about the\nOmegamatic",
        "Keen investigates the\nIon Ventilation System",
        "Keen struts through\nthe Security Center",
        "Keen invades\nDefense Tunnel Vlook",
        "Keen engages\nEnergy Flow Systems",
        "Keen barrels into\nDefense Tunnel Burrh",
        "Keen goes nuts in\nthe Regulation\nControl Center",
        "Keen regrets entering\nDefense Tunnel Sorra",
        "Keen blows through\nthe Neutrino\nBurst Injector",
        "Keen trots through\nDefense Tunnel Teln",
        "Keen breaks into\nthe Brownian\nMotion Inducer",
        "Keen hurries through\nthe Gravitational\nDamping Hub",
        "Keen explodes into\nthe Quantum\nExplosion Dynamo",
        "Keen faces danger\nin the secret\nKorath III Base",
        "Keen will not be\nin the BWBMegarocket",
        "Keen unexplainedly\nfinds himself by\ntheHigh Scores",
    ],
    &[
        "Keen attacks\nFribbulus Xax",
        "Keen hops across\nBloogwaters\nCrossing",
        "Keen fights his way\nthrough Guard Post One",
        "Keen crosses into the\nFirst Dome of Darkness",
        "Keen dares to enter the\nSecond Dome of Darkness",
        "Keen foolishly enters\nthe Bloogdome",
        "Keen makes his way\ninto Bloogton\nManufacturing",
        "Keen ascends\nBloogton Tower",
        "Keen hungily enters\nBloogfoods, Inc.",
        "Keen smashes through\nGuard Post Two",
        "Keen seeks thrills\nin Bloogville",
        "Keen rockets into the\nBloog Aeronautics and\nSpace Administration",
        "Keen boldly assaults\nGuard Post Three",
        "Keen whoops it up in\nthe Bloogbae\nRecreational District",
        "Keen purposefully struts\ninto the Bloogbase\nManagement District",
        "Keen bravely enters the\nBloog Control Center,\nlooking for Molly",
        "Keen warily enters\nBlooglab Space\nStation",
        "Keen returns to the\nBean-with-Bacon\nMegarocket",
        "Keen is in the High\nScore screen. Call Id!",
    ]
);

impl Game {
    /// RF_MemToMap: write tiles into a map plane.
    pub fn rf_mem_to_map(&mut self, src: &[u16], plane: usize, x: i32, y: i32, w: i32, h: i32) {
        let mut i = 0;
        for yy in y..y + h {
            for xx in x..x + w {
                if let Some(&v) = src.get(i) {
                    self.set_map_tile(plane, xx, yy, v);
                }
                i += 1;
            }
        }
    }

    /// GivePoints (0x183bf)
    pub fn give_points(&mut self, points: i32) {
        self.gs.score += points;
        if self.demomode == 0 && self.gs.score >= self.gs.nextextra {
            self.play_sound(super::keen::SND_EXTRAKEEN);
            self.gs.lives += 1;
            self.gs.nextextra *= 2;
        }
    }

    /// StopMusic
    pub fn stop_music(&mut self) {
        self.sd.music_off();
    }

    /// StartMusic (0x18542)
    pub fn start_music(&mut self, level: i32) {
        // Keen 4: StartMusic(-1) plays song 5 (0x18752)
        if EPISODE_NUM == 4 && level == -1 {
            self.sd.music_off();
            self.sd.start_music(5);
            return;
        }
        if !(0..20).contains(&level) {
            return;
        }
        self.sd.music_off();
        let song = LEVELMUSIC[level as usize];
        if song >= 0 {
            self.sd.start_music(song as usize);
        }
    }

    /// PollControls (0x18413)
    pub fn poll_controls(&mut self) {
        self.c = self.read_control();
        if self.c.yaxis != -1 {
            self.upheld = false;
        }
        if !self.twobutton && self.demomode == 0 {
            self.jumpbutton = self.c.button0;
            self.pogobutton = self.c.button1;
            self.firebutton = self.sys.input.key(self.firescan) || self.sys.input.pad_fire;
            if !self.jumpbutton {
                self.jumpheld = false;
            }
            if !self.pogobutton {
                self.pogoheld = false;
            }
            if !self.firebutton {
                self.fireheld = false;
            }
        } else if !self.c.button0 || !self.c.button1 {
            self.fireheld = false;
            self.firebutton = false;
            if !self.c.button0 {
                self.jumpheld = false;
            }
            self.jumpbutton = self.c.button0;
            if !self.c.button1 {
                if self.pogodelay == 0 {
                    self.pogoheld = false;
                }
                self.pogobutton = self.pogodelay != 0;
                self.pogodelay = 0;
            } else if self.pogodelay < 9 {
                self.pogodelay += self.tics;
            } else {
                self.pogobutton = true;
            }
        } else {
            self.firebutton = true;
            self.pogoheld = false;
            self.jumpheld = false;
            self.pogobutton = false;
            self.jumpbutton = false;
        }
    }

    /// CenterActor (0x17d79)
    pub fn center_actor(&mut self, ob: usize) {
        self.centerlevel = 0x8c;
        let o = &self.objs[ob];
        // a wider view keeps Keen in the middle (half = 0 for 320)
        let cx = 0x980 + self.cam.extra() / 2;
        let x = if (o.x as u16) < cx as u16 { 0 } else { o.x - cx };
        let y = if self.gs.mapon == 0 {
            if (o.y as u16) < 0x500 {
                0
            } else {
                o.y - 0x500
            }
        } else if ep_sel!((o.bottom as u16) < 0x8c0, (o.bottom as u16) < 0x8c0, (o.y as u16) < 0x540) {
            0
        } else {
            o.bottom - 0x8c0
        };
        if !self.scrolllock_rf {
            self.rf_new_position(x, y);
        }
        self.update_active_bounds();
    }

    /// WorldScrollScreen (0x17e54)
    pub fn world_scroll_screen(&mut self, ob: usize) {
        if self.scrolllock {
            return;
        }
        let o = &self.objs[ob];
        let c = &self.cam;
        let half = c.extra() / 2;
        let dx = if (o.left as u16) < (c.originxglobal + 0x900 + half) as u16 {
            o.left - (c.originxglobal + 0x900 + half)
        } else if ((c.originxglobal + 0xc00 + half) as u16) < o.right as u16 {
            (o.right + 16) - (c.originxglobal + 0xc00 + half)
        } else {
            0
        };
        let dy = if (o.top as u16) < (c.originyglobal + 0x500) as u16 {
            o.top - (c.originyglobal + 0x500)
        } else if ((c.originyglobal + 0x700) as u16) < o.bottom as u16 {
            o.bottom - (c.originyglobal + 0x700)
        } else {
            0
        };
        if dx != 0 || dy != 0 {
            self.rf_scroll(dx.clamp(-255, 255), dy.clamp(-255, 255));
            self.update_active_bounds();
        }
    }

    /// ScrollScreen (0x17f9d)
    pub fn scroll_screen(&mut self, ob: usize) {
        if self.scrolllock {
            return;
        }
        let o = self.objs[ob].clone();
        let c = self.cam.clone();
        let half = c.extra() / 2;
        // walking off the map's left or right edge completes the level
        let right = if c.wide() { (self.mapwidth - 2) * 256 } else { c.originxmax + 0x1400 };
        if (o.left as u16) < c.originxmin as u16 || (right as u16) < o.right as u16 {
            self.playstate = EX_COMPLETED;
            return;
        }
        if ((c.originymax + 0xd00) as u16) < o.bottom as u16 {
            self.objs[ob].y -= o.bottom - (c.originymax + 0xd00);
            self.play_sound(super::keen::SND_PLUMMET);
            self.godmode = false;
            self.kill_keen();
            return;
        }
        let mut dx = 0;
        if (o.x as u16) < (c.originxglobal + 0x900 + half) as u16 {
            dx = o.x - (c.originxglobal + 0x900 + half);
        } else if ((c.originxglobal + 0xc00 + half) as u16) < o.x as u16 {
            dx = o.x - (c.originxglobal + 0xc00 + half);
        }
        let mut dy = 0;
        let mut checked = false;
        if same(o.state, &S_KEENLOOKUP2) {
            let mut d = self.tics;
            if (self.centerlevel + self.tics) as u16 > 0xa7 {
                d = 0xa7 - self.centerlevel;
            }
            dy = -d;
            self.centerlevel += d;
            dy <<= 4;
            checked = true;
        } else if same(o.state, &S_KEENLOOKDOWN3) {
            let mut d = self.tics;
            if ((self.centerlevel - self.tics) as u16) < 0x21 {
                d = self.centerlevel - 0x21;
            }
            dy = d;
            self.centerlevel -= d;
            dy <<= 4;
            checked = true;
        }
        let _ = checked;
        if self.shaketime != 0 {
            let i = (self.shaketime as usize).min(23);
            dy += self.scroll_target_last - (o.bottom + SHAKETABLE[i]);
        } else if o.hitnorth == 0 && o.needtoclip != 0 && !same(o.state, &S_KEENHOLDON) {
            self.centerlevel = 0x8c;
        } else if !same(o.state, &S_KEENCLIMBUP1)
            && !same(o.state, &S_KEENCLIMBUP2)
            && !same(o.state, &S_KEENCLIMBUP3)
            && !same(o.state, &S_KEENCLIMBUP4)
        {
            dy += o.ymove;
            let target = c.originyglobal + dy + self.centerlevel * 16;
            self.scroll_target_last = target;
            if o.bottom as u16 != target as u16 {
                let diff = if (o.bottom as u16) < target as u16 { target - o.bottom } else { o.bottom - target };
                let mut speed = (diff << 4) >> 7;
                if speed > 0x30 {
                    speed = 0x30;
                }
                speed *= self.tics;
                if speed < 0x10 {
                    speed = if diff > 0xf { 0x10 } else { diff };
                }
                if (o.bottom as u16) < target as u16 {
                    dy -= speed;
                } else {
                    dy += speed;
                }
            }
        }
        // keep Keen at least two tiles from the top/bottom edge
        let d = (o.bottom - 0x200) - (c.originyglobal + dy);
        if d < 0 {
            dy += d;
        }
        let d = (o.bottom + 0x200) - (c.originyglobal + dy + 0xc80);
        if d > 0 {
            dy += d;
        }
        if dx != 0 || dy != 0 {
            self.rf_scroll(dx.clamp(-255, 255), dy.clamp(-255, 255));
            self.update_active_bounds();
        }
    }

    /// DrawStatusNumber (0x174fe): right aligned digits from tile8s.
    pub fn draw_status_number(&mut self, x: i32, y: i32, digits: usize, zerotile: i32, value: i32) {
        let s = value.to_string();
        let mut x = x;
        let len = s.len();
        let mut d = digits;
        while len < d {
            self.draw_tile8(x, y, zerotile);
            x += 8;
            d -= 1;
        }
        for c in s.bytes().skip(len.saturating_sub(d)) {
            self.draw_tile8(x, y, zerotile + c as i32 - 0x2f);
            x += 8;
        }
    }

    /// DrawStatusWindow: per episode (src/ckN/ep.rs).
    fn draw_status_window(&mut self) {
        super::ep::draw_status_window(self);
    }

    /// Draws the border and background of the status window (the common
    /// start of DrawStatusWindow).
    pub fn draw_status_frame(&mut self) {
        let left = 0x40;
        let width = 0xb8;
        let height = 0x90;
        self.draw_tile8(left, 0x10, 0x36);
        self.draw_tile8(left, height + 0x10, 0x3c);
        let mut x = left + 8;
        while x <= left + width - 8 {
            self.draw_tile8(x, 0x10, 0x37);
            self.draw_tile8(x, height + 0x10, 0x3d);
            x += 8;
        }
        self.draw_tile8(x, 0x10, 0x38);
        self.draw_tile8(x, height + 0x10, 0x3e);
        let mut y = 0x18;
        while y <= height + 8 {
            self.draw_tile8(left, y, 0x39);
            self.draw_tile8(left + width, y, 0x3b);
            y += 8;
        }
        self.sys.surface.bar(0x48, 0x18, 0xb0, 0x88, 7);
    }

    /// The LOCATION box with `name` centered vertically in it.
    pub fn draw_status_location(&mut self, name: &str) {
        self.tw.printy = 0x1c;
        self.tw.windowx = 0x50;
        self.tw.windoww = 0xa0;
        self.us_cprint("LOCATION");
        self.sys.surface.bar(0x4f, 0x26, 0xa2, 0x14, 15);
        let (_, h) = self.measure_multiline(name);
        self.tw.printy = ((0x14 - h) as u16 >> 1) as i32 + 0x26;
        self.us_cprint(name);
    }

    /// SCORE and EXTRA boxes.
    pub fn draw_status_score(&mut self) {
        self.tw.printy = 0x3d;
        self.tw.windowx = 0x50;
        self.tw.windoww = 0x40;
        self.us_cprint("SCORE");
        self.sys.surface.bar(0x4f, 0x47, 0x42, 10, 0);
        let score = self.gs.score;
        self.draw_status_number(0x50, 0x48, 8, 0x29, score);
        self.tw.printy = 0x3d;
        self.tw.windowx = 0xb0;
        self.tw.windoww = 0x40;
        self.us_cprint("EXTRA");
        self.sys.surface.bar(0xaf, 0x47, 0x42, 10, 0);
        let ne = self.gs.nextextra;
        self.draw_status_number(0xb0, 0x48, 8, 0x29, ne);
    }

    /// LEVEL box with the difficulty.
    pub fn draw_status_level(&mut self) {
        self.tw.printy = 0x55;
        self.tw.windowx = 0xb0;
        self.tw.windoww = 0x40;
        self.us_cprint("LEVEL");
        self.sys.surface.bar(0xaf, 0x5f, 0x42, 10, 15);
        self.tw.printy = 0x60;
        self.tw.windowx = 0xb0;
        self.tw.windoww = 0x40;
        match self.gs.difficulty {
            1 => self.us_cprint("Easy"),
            2 => self.us_cprint("Normal"),
            3 => self.us_cprint("Hard"),
            _ => {}
        }
    }

    /// KEYS, AMMO, KEENS and the drops counter (VIVAS, DROPS or VITALIN).
    pub fn draw_status_counts(&mut self, drops_label: &str) {
        self.tw.printx = 0x50;
        self.tw.printy = 0x70;
        self.us_print("KEYS");
        self.sys.surface.bar(0x77, 0x6f, 0x22, 10, 0);
        for i in 0..4 {
            if self.gs.keys[i] != 0 {
                self.draw_tile8((i as i32 + 0xf) * 8, 0x70, i as i32 + 0x24);
            }
        }
        self.tw.printx = 0xb0;
        self.tw.printy = 0x70;
        self.us_print("AMMO");
        self.sys.surface.bar(0xd7, 0x6f, 0x1a, 10, 0);
        let ammo = self.gs.ammo;
        self.draw_status_number(0xd8, 0x70, 3, 0x29, ammo);
        self.tw.printx = 0x50;
        self.tw.printy = 0x80;
        self.us_print("KEENS");
        self.sys.surface.bar(0x7f, 0x7f, 0x12, 10, 0);
        let lives = self.gs.lives;
        self.draw_status_number(0x80, 0x80, 2, 0x29, lives);
        self.tw.printx = 0xb0;
        self.tw.printy = 0x80;
        self.us_print(drops_label);
        self.sys.surface.bar(0xe0, 0x7f, 0x10, 10, 0);
        let drops = self.gs.drops;
        self.draw_status_number(0xe0, 0x80, 2, 0x29, drops);
    }

    /// The 10x2 tile8 picture at the bottom (the wrist computer's logo).
    pub fn draw_status_logo(&mut self, col: i32) {
        for i in 0..10 {
            self.draw_tile8((i + col) * 8, 0x8c, i + 0x48);
            self.draw_tile8((i + col) * 8, 0x94, i + 0x52);
        }
    }

    pub fn measure_multiline(&self, s: &str) -> (i32, i32) {
        let mut w = 0;
        let mut h = 0;
        for l in s.as_bytes().split(|&c| c == b'\n') {
            let (lw, lh) = self.measure_string(l);
            w = w.max(lw);
            h += lh;
        }
        (w, h)
    }

    /// StatusWindow (0x17c0d): slides the status window over the play field.
    pub fn status_window(&mut self) {
        self.tw.windowx = 0;
        self.tw.windoww = 320;
        self.tw.windowy = 0;
        self.tw.windowh = 200;
        // Keen 4 and 5: holding A and 2 while opening the status window
        // activates the debug keys (0x17e01)
        if EPISODE_NUM != 6 && self.sys.input.key(SC_A) && self.sys.input.key(0x03) {
            self.us_center_window(0x14, 2);
            self.tw.printy += 2;
            self.us_print("Debug keys active");
            self.sys.present();
            self.wait_ack();
            self.opts.debug = true;
        }
        self.draw_playfield();
        // render the status window into its own surface
        let saved = self.sys.surface.px.clone();
        self.draw_status_window();
        let status = self.sys.surface.px.clone();
        self.sys.surface.px = saved;
        self.play_sound(super::keen::SND_SHOWSTATUS);
        let mut pos = 16;
        loop {
            self.compose_status(&status, pos);
            self.sys.present();
            self.calc_tics();
            if pos == 0xa8 || !self.alive() {
                break;
            }
            pos = (pos + self.tics * 8).min(0xa8);
        }
        self.sys.input.clear_keys_down();
        self.wait_ack();
        self.play_sound(super::keen::SND_HIDESTATUS);
        pos -= 16;
        loop {
            self.compose_status(&status, pos);
            self.sys.present();
            self.calc_tics();
            if pos == 0 || !self.alive() {
                break;
            }
            pos = (pos - self.tics * 8).max(0);
        }
        self.objs[self.scoreobj].x = 0;
    }

    /// ScrollStatusWindow (0x17a40): play field with the status window
    /// scrolled down `pos` pixels, hanging on its cable.
    fn compose_status(&mut self, status: &[u32], pos: i32) {
        self.draw_playfield();
        let copy_rows = |s: &mut crate::engine::video::Surface, src_y: i32, dst_y: i32, rows: i32| {
            let scale = s.scale as i32;
            for r in 0..rows * scale {
                let sy = src_y * scale + r;
                let dy = dst_y * scale + r;
                if sy < 0 || dy < 0 || sy >= s.h as i32 || dy >= s.h as i32 {
                    continue;
                }
                let x0 = ((64 + s.xoff) * scale) as usize;
                let x1 = ((256 + s.xoff) * scale) as usize;
                let (sr, dr) = (sy as usize * s.w, dy as usize * s.w);
                s.px[dr + x0..dr + x1].copy_from_slice(&status[sr + x0..sr + x1]);
            }
        };
        if pos < 0x99 {
            copy_rows(&mut self.sys.surface, 16 + 0x98 - pos, 0, pos);
        } else {
            let k = pos - 0x98;
            copy_rows(&mut self.sys.surface, 0, 0, k);
            self.draw_mpic(0x88, k - 16, ep_sel!(0x73, 0x65, 0x2d));
            copy_rows(&mut self.sys.surface, 16, k, 0x98);
        }
        if pos > 0x47 {
            self.draw_mpic(0x28, pos - 0xa8, ep_sel!(0x72, 0x64, 0x2c));
        }
    }

    /// Waits for any key/button (IN_Ack).
    pub fn wait_ack(&mut self) {
        self.sys.input.clear_keys_down();
        loop {
            self.sys.wait_vbl(1);
            if !self.alive() {
                return;
            }
            let c = self.sys.input.read_control();
            // keys still held from before (auto repeat) don't acknowledge
            let i = &self.sys.input;
            if (i.last_scan != 0 && !i.last_repeat) || c.button0 || c.button1 {
                break;
            }
        }
        // wait for release of buttons
        loop {
            let c = self.sys.input.read_control();
            if !c.button0 && !c.button1 {
                break;
            }
            self.sys.wait_vbl(1);
            if !self.alive() {
                return;
            }
        }
        self.sys.input.clear_keys_down();
    }

    /// IN_UserInput(delay): waits up to `delay` tics for a key; true if pressed.
    pub fn user_input(&mut self, delay: i32) -> bool {
        let start = self.sys.time_count();
        self.sys.input.clear_keys_down();
        loop {
            self.sys.wait_vbl(1);
            if !self.alive() {
                return true;
            }
            let c = self.sys.input.read_control();
            if self.sys.input.last_scan != 0 || c.button0 || c.button1 {
                self.sys.input.clear_keys_down();
                return true;
            }
            if self.sys.time_count() - start >= delay {
                return false;
            }
        }
    }

    /// CheckKeys (0x172b5)
    fn check_keys(&mut self) {
        if self.screenfaded {
            return;
        }
        if self.sys.input.key(SC_ENTER) {
            self.status_window();
            self.sys.input.clear_keys_down();
            self.lasttimecount = self.sys.time_count();
        }
        if self.sys.input.paused {
            self.sd.music_off();
            self.draw_playfield();
            self.us_center_window(8, 3);
            self.us_print_centered("PAUSED");
            self.sys.present();
            self.wait_ack();
            self.sys.input.paused = false;
            self.sd.music_on();
            self.lasttimecount = self.sys.time_count();
        }
        // Keen 4 and 5: F1 shows the help screens (0x17433)
        if EPISODE_NUM != 6 && self.sys.input.last_scan == SC_F1 {
            self.stop_music();
            self.help_screens();
            self.start_music(self.gs.mapon);
            if self.showscorebox {
                let s = self.scoreobj;
                let o = &mut self.objs[s];
                o.temp1 = -1;
                o.temp2 = -1;
                o.temp3 = -1;
                o.temp4 = -1;
            }
            self.rf_force_refresh();
        }
        // --cheatmode: F2 quick saves, F3 quick loads the last used slot
        // (only once a game was loaded or saved, otherwise the panel opens)
        let ls = self.sys.input.last_scan;
        if self.opts.cheatmode && !self.storedemo && (ls == SC_F2 || ls == SC_F3) {
            let done = if ls == SC_F2 { self.quick_save() } else { self.quick_load() };
            if done {
                self.rf_force_refresh();
                self.sys.input.clear_keys_down();
                self.lasttimecount = self.sys.time_count();
            }
        }
        if !self.storedemo {
            let ls = self.sys.input.last_scan;
            // Keen 6 opens the control panel with F1 too
            if (SC_F1 < ls && ls < SC_F8) || ls == SC_ESCAPE || (EPISODE_NUM == 6 && ls == SC_F1) {
                self.stop_music();
                self.control_panel(true);
                self.rf_force_refresh();
                if self.restartgame {
                    self.playstate = EX_RESETGAME;
                } else if self.loadedgame {
                    self.playstate = EX_LOADEDGAME;
                }
                if self.abortgame {
                    self.abortgame = false;
                    self.playstate = EX_ABORTGAME;
                }
                if self.playstate == EX_STILLPLAYING {
                    self.start_music(self.gs.mapon);
                }
                self.sys.input.clear_keys_down();
                self.lasttimecount = self.sys.time_count();
            }
        }
        // Keen 4 and 5: B+A+T gives all keys, 99 shots and an extra Keen
        if EPISODE_NUM != 6 && self.sys.input.key(SC_B) && self.sys.input.key(SC_A) && self.sys.input.key(SC_T) {
            self.bat_cheat();
        }
        if self.opts.debug && self.sys.input.key(SC_F10) && self.debug_keys() {
            self.lasttimecount = self.sys.time_count();
        }
        if self.sys.input.key(SC_CONTROL) && self.sys.input.last_scan == SC_Q {
            self.request_quit = true;
        }
    }

    /// The B+A+T cheat (Keen 4 0x173bf, Keen 5 0x177e5): only works when no
    /// other letter key from A to M (scan codes 0x1e..0x2c) is held.
    fn bat_cheat(&mut self) {
        for sc in 0x1e..=0x2c {
            if sc != SC_B && sc != SC_A && sc != SC_T && self.sys.input.key(sc) {
                return;
            }
        }
        self.us_center_window(0x14, 7);
        self.tw.printy += 2;
        self.us_cprint("Cheat Option!\n\nYou just got all\nthe keys, 99 shots,\nand an extra keen!");
        self.sys.present();
        self.wait_ack();
        self.rf_force_refresh();
        self.gs.lives += 1;
        self.gs.keys = [1; 4];
        if EPISODE_NUM == 5 {
            self.gs.keycard = 1;
        }
        self.gs.ammo = 99;
    }

    /// PlayLoop (0x18611)
    pub fn play_loop(&mut self) {
        self.start_music(self.gs.mapon);
        self.jumpheld = false;
        self.upheld = false;
        self.pogoheld = false;
        self.fireheld = false;
        self.inlevel = true;
        self.playstate = EX_STILLPLAYING;
        self.pogodelay = 0;
        self.scrolllock = false;
        self.invincible = 0;
        let p = self.player;
        self.center_actor(p);
        self.init_rnd(self.demomode == 0);
        self.tics = 3;
        self.lasttimecount = 3;
        self.sys.set_time_count(3);
        loop {
            if !self.alive() {
                self.playstate = EX_ABORTGAME;
                break;
            }
            self.poll_controls();
            // activate / move objects
            let mut cur = Some(self.player);
            while let Some(ob) = cur {
                let o = &self.objs[ob];
                let c = &self.cam;
                if o.active == AC_NO
                    && ((c.originxtile - 1) as u16) <= o.tileright as u16
                    && (o.tileleft as u16) <= (c.originxtilemax + 1) as u16
                    && (o.tiletop as u16) <= (c.originytilemax + 1) as u16
                    && ((c.originytile - 1) as u16) <= o.tilebottom as u16
                {
                    self.objs[ob].needtoreact = true;
                    self.objs[ob].active = AC_YES;
                }
                if self.objs[ob].active != AC_NO {
                    let o = &self.objs[ob];
                    let c = &self.cam;
                    if (o.tileright as u16) < c.inactivateleft as u16
                        || (c.inactivateright as u16) < o.tileleft as u16
                        || (c.inactivatebottom as u16) < o.tiletop as u16
                        || (o.tilebottom as u16) < c.inactivatetop as u16
                    {
                        if self.objs[ob].active == AC_REMOVABLE {
                            let nx = self.objs[ob].next;
                            self.remove_obj(ob);
                            cur = nx;
                            continue;
                        } else if self.objs[ob].active != AC_ALLWAYS {
                            let r = self.rnd() as i32;
                            if r < self.tics * 2 || self.screenfaded || self.loadedgame {
                                self.remove_sprite(ob);
                                self.objs[ob].active = AC_NO;
                                cur = self.objs[ob].next;
                                continue;
                            }
                        }
                    }
                    self.state_machine(ob);
                    // like the original, a removed object keeps its next link
                    cur = self.objs[ob].next;
                    continue;
                }
                cur = self.objs[ob].next;
            }
            if self.gs.riding.is_some() {
                let p = self.player;
                self.handle_riding(p);
            }
            // contacts
            let mut cur = Some(self.player);
            while let Some(ob) = cur {
                if self.objs[ob].active != AC_NO {
                    let mut ch = self.objs[ob].next;
                    while let Some(h) = ch {
                        let (a, b) = (&self.objs[ob], &self.objs[h]);
                        if b.active != AC_NO
                            && (b.left as u16) < (a.right as u16)
                            && (a.left as u16) < (b.right as u16)
                            && (a.top as u16) < (b.bottom as u16)
                            && (b.top as u16) < (a.bottom as u16)
                        {
                            if let Some(c) = self.objs[ob].state.and_then(|s| s.contact) {
                                c(self, ob, h);
                            }
                            if let Some(c) = self.objs[h].state.and_then(|s| s.contact) {
                                c(self, h, ob);
                            }
                            if self.objs[ob].obclass == NOTHING {
                                break;
                            }
                        }
                        ch = self.objs[h].next;
                    }
                }
                cur = self.objs[ob].next;
            }
            let p = self.player;
            if self.gs.mapon == 0 {
                self.check_world_in_tiles(p);
            } else {
                self.check_in_tiles(p);
            }
            // reactions
            let mut cur = Some(self.player);
            while let Some(ob) = cur {
                let nx = self.objs[ob].next;
                if self.objs[ob].active != AC_NO {
                    if (self.objs[ob].tilebottom as u16) < (self.mapheight - 1) as u16 {
                        if self.objs[ob].needtoreact {
                            if let Some(r) = self.objs[ob].state.and_then(|s| s.react) {
                                self.objs[ob].needtoreact = false;
                                r(self, ob);
                            }
                        }
                    } else if self.objs[ob].obclass == KEENOBJ {
                        self.playstate = EX_DIED;
                    } else {
                        self.remove_obj(ob);
                        cur = nx;
                        continue;
                    }
                }
                cur = self.objs[ob].next;
            }
            let p = self.player;
            // Keen 4's underwater level 17 scrolls like the world map
            if self.gs.mapon == 0 || (EPISODE_NUM == 4 && self.ca_mapon == 0x11) {
                self.world_scroll_screen(p);
            } else {
                self.scroll_screen(p);
            }
            if self.loadedgame {
                self.loadedgame = false;
            }
            // follow window resizes / fullscreen toggles
            self.update_view();
            self.rf_refresh();
            self.trace_frame();
            if self.invincible != 0 {
                self.invincible = (self.invincible - self.tics).max(0);
            }
            if self.shaketime != 0 {
                self.shaketime = (self.shaketime - self.tics).max(0);
            }
            if self.slowmotion {
                self.sys.wait_vbl(14);
                self.lasttimecount = self.sys.time_count();
            }
            if self.extravbls != 0 {
                self.sys.wait_vbl(self.extravbls);
            }
            if self.demomode == 2 {
                if !self.screenfaded && self.demo_user_input() {
                    self.playstate = EX_COMPLETED;
                    if self.sys.input.last_scan != SC_F1 {
                        self.sys.input.last_scan = SC_SPACE;
                    }
                }
            } else if self.demomode == 3 {
                self.playstate = EX_COMPLETED;
            } else {
                self.check_keys();
            }
            if self.sys.input.key(SC_E) && self.sys.input.key(SC_N) && self.sys.input.key(SC_D) {
                // the E+N+D cheat ends the game: Keen 4 as if the last
                // council member was rescued, Keen 5 the QED destroyed
                self.playstate = ep_sel!(EX_RESCUED, EX_QEDBROKE, EX_MOLLY);
            }
            if self.playstate != EX_STILLPLAYING {
                break;
            }
        }
        self.inlevel = false;
        self.stop_music();
    }

    fn demo_user_input(&mut self) -> bool {
        self.sys.poll();
        let c = self.sys.input.read_control();
        self.sys.input.last_scan != 0 || c.button0 || c.button1
    }
}

/// Debug aid: with `KEEN6_TRACE=file`, every refresh appends
/// `mapon x y shapenum` of the player, for comparison against a trace of
/// the original read from DOSBox memory (`tools/dosbox_trace.py`).
static TRACE: std::sync::OnceLock<Option<std::sync::Mutex<std::fs::File>>> =
    std::sync::OnceLock::new();

impl Game {
    fn trace_frame(&self) {
        let t = TRACE.get_or_init(|| {
            let path = std::env::var_os("KEEN6_TRACE")?;
            std::fs::File::create(path).ok().map(std::sync::Mutex::new)
        });
        if let Some(f) = t {
            use std::io::Write;
            let o = &self.objs[self.player];
            let _ = writeln!(
                f.lock().unwrap(),
                "{} {} {} {}",
                self.gs.mapon,
                o.x as u16,
                o.y as u16,
                o.shapenum
            );
        }
    }
}
