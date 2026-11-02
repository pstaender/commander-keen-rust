//! CK4_SPEC.C port: info plane scanning, the story message boxes (Princess
//! Lindsey, the janitor, the rescued council members, scuba gear) and the
//! swimming Keen of the underwater levels.
//! Translated from segment 0x1de8 of KEEN4E.EXE (linear 0x1de89..0x1efa0).
//!
//! The caching done by the original (lumpneeded[], CA_MarkGrChunk,
//! CA_UpLevel/CA_DownLevel, CA_CacheMarks) has no effect here, as all graphics
//! are always loaded; it is left out (noted in comments where it happens).

use super::act1;
use super::act2;
use super::act3;
use super::defs::*;
use super::states::*;
use super::Game;

// object classes (Keen 4) used here
pub const BONUSOBJ4: i32 = 4;
pub const ORACLEOBJ: i32 = 6;

// sounds
pub const SND_MAKEFOOT: i32 = 0x2d;
pub const SND_BLUB: i32 = 0x24;

// pictures (EGAGRAPH chunks)
pub const ORACLEPIC: i32 = 0x66;
pub const KEENTALK1PIC: i32 = 0x67;
pub const KEENTALK2PIC: i32 = 0x68;
pub const KEENMADPIC: i32 = 0x69;
pub const LINDSEYPIC: i32 = 0x6a;

/// DS:0x12c4 ammo per clip by difficulty
pub const SHOTSINCLIP: [i32; 4] = [0, 8, 5, 5];
/// DS:0x12cc bonus sounds
pub const BONUSSOUND: [i32; 12] = [19, 19, 19, 19, 8, 8, 8, 8, 8, 8, 17, 9];
/// DS:0x12e4 bonus points
pub const BONUSPOINTS: [i32; 12] = [0, 0, 0, 0, 100, 200, 500, 1000, 2000, 5000, 0, 0];
/// DS:0x12fc bonus sprites shown when collected
pub const BONUSSPRITE: [i32; 12] = [242, 242, 242, 242, 211, 212, 213, 214, 215, 216, 217, 218];

/// DS:0x18cc Keen's answers to the rescued council members (by gamestate.rescued)
const RESCUEANSWER: [&str; 8] = [
    "No sweat, oh guardian\nof wisdom!",
    "Sounds like a plan,\nbearded one!",
    "No problemo.",
    "Great.  You know, you\nlook a lot like the\nlast guy I rescued...",
    "Good idea, Gramps.",
    "May the road rise\nto meet your feet,\nMr. Member.",
    "Wise plan of action,\nyour ancientness.",
    "You're the last one,\nfella.  Let's both\nget back to the\nOracle chamber!",
];

impl Game {
    /// gamestate.rescued (K4 DS 0x77dd: council members rescued so far).
    pub fn rescued_mut(&mut self) -> &mut i32 {
        &mut self.gs.rescued
    }

    /// ScanInfoPlane (0x1de89)
    pub fn scan_info_plane(&mut self) {
        self.init_obj_array();
        // (memset(lumpneeded, 0, sizeof(lumpneeded)))
        let diff = self.gs.difficulty;
        for y in 0..self.mapheight {
            for x in 0..self.mapwidth {
                let info = self.map_tile(2, x, y) as i32;
                if info == 0 {
                    continue;
                }
                // the original falls through from the hard to the normal
                // case and from there into the easy one
                let hard = diff >= 3;
                let normal = diff >= 2;
                match info {
                    1 => {
                        self.spawn_keen(x, y, 1);
                        self.spawn_score();
                        // (lumpneeded[KEEN_LUMP], CA_MarkGrChunk(SCOREBOXSPR))
                    }
                    2 => {
                        self.spawn_keen(x, y, -1);
                        self.spawn_score();
                    }
                    3 => {
                        self.spawn_world_keen(x, y);
                        self.spawn_score();
                    }
                    4 => act1::spawn_1f208(self, x, y), // council member
                    50 if hard => act2::spawn_2007c(self, x, y),
                    49 if normal => act2::spawn_2007c(self, x, y),
                    5 => act2::spawn_2007c(self, x, y), // berkeloid
                    6 => act3::spawn_21bac(self, x, y), // Princess Lindsey
                    52 if hard => act2::spawn_1fcef(self, x, y),
                    51 if normal => act2::spawn_1fcef(self, x, y),
                    7 => act2::spawn_1fcef(self, x, y), // wormmouth
                    46 if hard => act1::spawn_1fad3(self, x, y),
                    45 if normal => act1::spawn_1fad3(self, x, y),
                    8 => act1::spawn_1fad3(self, x, y), // skypest
                    9 => act2::spawn_1fea0(self, x, y), // thundercloud
                    10 => act2::spawn_203d5(self, x, y), // foot
                    11 => act2::spawn_20359(self, x, y), // inchworm
                    12 => act2::spawn_2059e(self, x, y), // bounder
                    13 => act1::spawn_1f4ef(self, x, y), // egg (eggbird)
                    48 if hard => act2::spawn_20760(self, x, y),
                    47 if normal => act2::spawn_20760(self, x, y),
                    14 => act2::spawn_20760(self, x, y), // lick
                    88 if hard => act3::spawn_214b7(self, x, y),
                    87 if normal => act3::spawn_214b7(self, x, y),
                    15 => act3::spawn_214b7(self, x, y), // dopefish
                    16 => act3::spawn_218ab(self, x, y), // schoolfish
                    24 if hard => act3::spawn_2195a(self, x, y),
                    23 if normal => act3::spawn_2195a(self, x, y),
                    17 => act3::spawn_2195a(self, x, y), // sprite (pixie)
                    18 => act3::spawn_20e60(self, x, y), // treasure eater
                    19 => act3::spawn_21275(self, x, y), // mimrock
                    74 if hard => act1::spawn_1f9fa(self, x, y),
                    73 if normal => act1::spawn_1f9fa(self, x, y),
                    20 => act1::spawn_1f9fa(self, x, y), // arachnut
                    21 => act1::spawn_1f3db(self, x, y), // mad mushroom
                    44 if hard => act1::spawn_1f29e(self, x, y),
                    43 if normal => act1::spawn_1f29e(self, x, y),
                    22 => act1::spawn_1f29e(self, x, y), // slug
                    25 => self.rf_set_scroll_block(x, y, true),
                    26 => self.rf_set_scroll_block(x, y, false),
                    27..=30 => act2::spawn_20935(self, x, y, info - 27), // platform
                    32 => act2::spawn_20d34(self, x, y), // dropping platform
                    33 => act1::spawn_1efa1(self, x, y), // Miragia
                    34 => {
                        if self.gs.ammo < 5 {
                            act1::spawn_1f13f(self, x, y, 11); // ammo (only if low)
                        }
                    }
                    35 => act3::spawn_21da9(self, x, y), // scuba gear
                    42 => {
                        spawn_swim_keen(self, x, y);
                        self.spawn_score();
                    }
                    83..=86 if diff >= 3 => act3::spawn_21c34(self, x, y, info - 83),
                    79..=82 if normal => act3::spawn_21c34(self, x, y, info - 79),
                    53..=56 => act3::spawn_21c34(self, x, y, info - 53), // dart gun
                    57..=68 => act1::spawn_1f13f(self, x, y, info - 57), // bonus items
                    69..=72 => act3::spawn_21ae7(self, x, y, info - 69), // mine
                    75 => {} // (lumpneeded[MOON_LUMP]: Keen mooning, no object)
                    78 if hard => act1::spawn_1f54d(self, x, y),
                    77 if normal => act1::spawn_1f54d(self, x, y),
                    76 => act1::spawn_1f54d(self, x, y), // eggbird (bird)
                    _ => {}
                }
            }
        }
        // everything except "always active" objects starts inactive
        let mut i = Some(self.player);
        while let Some(n) = i {
            if self.objs[n].active != AC_ALLWAYS {
                self.objs[n].active = AC_NO;
            }
            i = self.objs[n].next;
        }
        // (CA_MarkGrChunk for every chunk of the needed lumps)
    }

    /// Window with a picture of the speaker on the left (the first part of
    /// the story messages): US_CenterWindow(26, 8), the picture at WindowX
    /// and the text right of it.
    fn k4_left_pic_window(&mut self, pic: i32) {
        self.us_center_window(26, 8);
        let (x, y) = (self.tw.windowx, self.tw.windowy);
        self.draw_pic(x, y, pic);
        self.tw.printy += 6;
        self.tw.windoww -= 0x30;
        self.tw.windowx += 0x30;
    }

    /// Window with Keen's picture on the right (his answer).
    fn k4_keen_window(&mut self, pic: i32) {
        self.us_center_window(26, 8);
        let x = self.tw.windowx + self.tw.windoww - 0x30;
        let y = self.tw.windowy;
        self.draw_pic(x, y, pic);
        self.tw.windoww -= 0x30;
        self.tw.printy += 12;
    }

    /// VW_UpdateScreen, VW_WaitVBL(vbls), IN_ClearKeysDown, IN_Ack
    fn k4_show_and_ack(&mut self, vbls: i32) {
        self.sys.present();
        self.sys.wait_vbl(vbls);
        self.sys.input.clear_keys_down();
        self.wait_ack();
    }

    /// Keen's second talking picture (KEENTALK2PIC) over the first one at the
    /// right edge of the window.
    fn k4_keen_talk2(&mut self) {
        let x = self.tw.windowx + self.tw.windoww;
        let y = self.tw.windowy;
        self.draw_pic(x, y, KEENTALK2PIC);
        self.k4_show_and_ack(30);
    }

    /// PrincessLindsey (0x1e4c5)
    pub fn princess_lindsey(&mut self) {
        self.sd.wait_sound_done(&mut self.sys);
        self.stop_music();
        // (CA_UpLevel, CA_MarkGrChunk(LINDSEYPIC, KEENTALK1PIC, KEENTALK2PIC), CA_CacheMarks)
        self.draw_playfield(); // VW_FixRefreshBuffer
        self.k4_left_pic_window(LINDSEYPIC);
        self.us_cprint("Princess Lindsey says:\n");
        if self.ca_mapon == 7 {
            self.us_cprint("There's gear to help\nyou swim in Three-Tooth\nLake. It is hidden in\nMiragia.\n");
        } else {
            self.us_cprint("The way to the Pyramid\nof the Forbidden lies\nunder the Pyramid of\nMoons.\n");
        }
        self.sys.present();
        self.play_sound(SND_MAKEFOOT);
        self.sys.wait_vbl(60);
        self.sys.input.clear_keys_down();
        self.wait_ack();

        self.k4_keen_window(KEENTALK1PIC);
        if self.ca_mapon == 7 {
            self.us_cprint("Thanks, your Highness!");
        } else {
            self.us_cprint("Thanks for the\nmysterious clue,\nPrincess!\n");
        }
        self.k4_show_and_ack(30);
        self.k4_keen_talk2();
        // (CA_DownLevel)
        self.start_music(self.gs.mapon);
        // the score box sprites may have been recached: redraw everything
        let s = self.scoreobj;
        self.objs[s].temp2 = -1;
        self.objs[s].temp1 = -1;
        self.objs[s].temp3 = -1;
        self.objs[s].temp4 = -1;
    }

    /// RescueJanitor (0x1e640)
    pub fn rescue_janitor(&mut self) {
        self.sd.wait_sound_done(&mut self.sys);
        // (CA_UpLevel, CA_MarkGrChunk(ORACLEPIC, KEENTALK1PIC, KEENMADPIC), CA_CacheMarks)
        self.draw_playfield(); // VW_FixRefreshBuffer
        self.start_music(-1);

        // the texts are far strings (_fstrcpy into a local buffer first)
        self.k4_left_pic_window(ORACLEPIC);
        self.us_cprint(
            "Thanks for going to all\nthat trouble, but I'm\njust the janitor for the\nHigh Council.",
        );
        self.k4_show_and_ack(60);

        self.k4_left_pic_window(ORACLEPIC);
        self.us_cprint("I tried to tell the\nShikadi that but they\njust wouldn't listen...");
        self.k4_show_and_ack(60);

        self.k4_keen_window(KEENTALK1PIC);
        self.us_cprint("This had better\nbe a joke.");
        self.k4_show_and_ack(60);

        self.k4_left_pic_window(ORACLEPIC);
        self.us_cprint("Sorry.  You aren't\nmad, are you?");
        self.k4_show_and_ack(60);

        self.us_center_window(26, 8);
        let x = self.tw.windowx + self.tw.windoww - 0x30;
        let y = self.tw.windowy;
        self.draw_pic(x, y, KEENTALK1PIC);
        let x = self.tw.windowx + self.tw.windoww - 0x28;
        let y = self.tw.windowy + 0x18;
        self.draw_pic(x, y, KEENMADPIC);
        self.k4_show_and_ack(30);

        self.stop_music();
        // (CA_DownLevel)
        self.start_music(self.gs.mapon);
    }

    /// CantSwim (0x1e8c1)
    pub fn cant_swim(&mut self) {
        self.sd.wait_sound_done(&mut self.sys);
        // (CA_UpLevel, CA_CacheGrChunk(KEENTALK1PIC))
        self.k4_keen_window(KEENTALK1PIC);
        self.us_cprint("I can't swim!");
        self.k4_show_and_ack(30);
        // (CA_DownLevel)
    }

    /// GotScuba (0x1e93a)
    pub fn got_scuba(&mut self) {
        self.sd.wait_sound_done(&mut self.sys);
        // (CA_UpLevel, CA_MarkGrChunk(KEENTALK1PIC, KEENTALK2PIC), CA_CacheMarks)
        self.k4_keen_window(KEENTALK1PIC);
        self.us_cprint("Cool!  I can breathe\nunder water now!");
        self.k4_show_and_ack(30);
        self.k4_keen_talk2();
        // (CA_DownLevel)
    }

    /// RescuedMember (0x1e9f9)
    pub fn rescued_member(&mut self) {
        self.sd.wait_sound_done(&mut self.sys);
        // (CA_UpLevel, CA_MarkGrChunk(ORACLEPIC, KEENTALK1PIC, KEENTALK2PIC), CA_CacheMarks)
        self.start_music(-1);
        self.draw_playfield(); // VW_FixRefreshBuffer
        self.k4_left_pic_window(ORACLEPIC);
        if self.ca_mapon == 17 {
            // the member rescued under water
            self.us_cprint(
                "Ggoh thig you sogh mg\nfgor regscuing mgge!\nI'gll regur tgo the\nGoracle chagber\nigmediatggely. Blub.",
            );
        } else {
            self.us_cprint(
                "Oh thank you so much\nfor rescuing me!\nI'll return to the\nOracle chamber\nimmediately.",
            );
        }
        self.k4_show_and_ack(60);

        self.k4_keen_window(KEENTALK1PIC);
        let r = *self.rescued_mut();
        // the original indexes the pointer table without a check
        let text = RESCUEANSWER.get(r as usize).copied().unwrap_or("");
        self.us_cprint(text);
        self.k4_show_and_ack(30);
        self.k4_keen_talk2();

        *self.rescued_mut() += 1;
        // (CA_DownLevel)
        self.stop_music();
    }

    // hooks of the shared code (Keen 6 items that Keen 4 does not have)
    pub fn got_sandwich(&mut self) {}
    pub fn got_hook(&mut self) {}
    pub fn got_passcard(&mut self) {}

    /// Keen 4 has no switch tiles (HitSpecialTile is Keen 6 only).
    pub fn hit_special_tile(&mut self, _ob: usize, _top: bool) {}

    /// SpawnSplash (0x1f1a9, CK4_ACT1) when Keen collects a drop.
    pub fn spawn_drop_splash(&mut self, x: i32, y: i32) {
        act1::spawn_1f1a9(self, x, y);
    }
}

/// SpawnSwimKeen (0x1eb42): Keen in his scuba gear (info value 42).
pub fn spawn_swim_keen(g: &mut Game, x: i32, y: i32) {
    let p = g.player;
    {
        let o = &mut g.objs[p];
        o.obclass = KEENOBJ;
        o.active = AC_ALLWAYS;
        o.priority = 1;
        o.x = x << 8;
        o.y = y << 8;
        o.xdir = 1;
        o.ydir = 1;
        o.needtoclip = CL_FULLCLIP;
    }
    g.new_state(p, &S_KEENSWIMSLOW1);
}
pub use self::spawn_swim_keen as spawn_1eb42;

/// SpawnKbubble (0x1eb8a): an air bubble rising from the swimming Keen.
pub fn spawn_kbubble(g: &mut Game, ob: usize) {
    g.objs[ob].temp4 = 0;
    let n = g.get_new_obj(true);
    let x = if g.objs[ob].xdir == -1 { g.objs[ob].x } else { g.objs[ob].x + 0x180 };
    let y = g.objs[ob].y;
    {
        let o = &mut g.objs[n];
        o.x = x;
        o.y = y;
        o.obclass = INERTOBJ;
        o.priority = 3;
        o.active = AC_REMOVABLE;
        o.needtoclip = CL_NOCLIP;
        o.yspeed = -24;
        o.xspeed = 4;
    }
    match g.rnd() as i32 / 0x40 {
        0 => g.new_state(n, &S_KBUBBLE1),
        1 => g.new_state(n, &S_KBUBBLE2),
        2 => g.new_state(n, &S_KBUBBLE3),
        3 => g.new_state(n, &S_KBUBBLE4),
        _ => {}
    }
    g.play_sound(SND_BLUB);
}

/// Bubble timer of the swimming Keen: a bubble every 60 tics.
fn swim_bubble(g: &mut Game, ob: usize) {
    g.objs[ob].temp4 += g.tics;
    if g.objs[ob].temp4 > 0x3c {
        spawn_kbubble(g, ob);
    }
}

/// T_KeenSwimSlow (0x1ec2f): swimming with water friction, one speed
/// update every 8 tics; jump gives a stroke (s_keenswim).
pub fn t_keen_swim_slow(g: &mut Game, ob: usize) {
    let xwasneg = g.objs[ob].xspeed < 0;
    let ywasup = g.objs[ob].yspeed < 4;
    swim_bubble(g, ob);
    if g.jumpbutton && !g.jumpheld {
        g.jumpheld = true;
        if g.c.xaxis != 0 {
            g.objs[ob].xspeed = g.c.xaxis * 18;
        }
        if g.c.yaxis != 0 {
            g.objs[ob].yspeed = g.c.yaxis * 18;
        }
        // ob->state = ob->state->nextstate (keeps ticcount)
        g.objs[ob].state = g.objs[ob].st().next;
    }
    if g.c.xaxis != 0 {
        g.objs[ob].xdir = g.c.xaxis;
    }
    let end = g.lasttimecount;
    let mut i = end - g.tics;
    while i < end {
        if i & 7 == 0 {
            let xs = g.objs[ob].xspeed;
            let mut d = if xs > 12 {
                -3
            } else if xs > 0 {
                -1
            } else if xs > -12 {
                1
            } else {
                3
            };
            d += g.c.xaxis;
            d += g.c.xaxis;
            g.objs[ob].xspeed += d;
            if g.c.xaxis == 0 && (g.objs[ob].xspeed < 0) != xwasneg {
                g.objs[ob].xspeed = 0;
            }

            let ys = g.objs[ob].yspeed;
            let mut d = if ys > 12 {
                -3
            } else if ys > 4 {
                -1
            } else if ys > -12 {
                1
            } else {
                3
            };
            d += g.c.yaxis;
            d += g.c.yaxis;
            g.objs[ob].yspeed += d;
            if g.c.yaxis == 0 && g.objs[ob].yspeed > 4 && ywasup {
                g.objs[ob].yspeed = 0;
            }
        }
        g.ph.xtry += g.objs[ob].xspeed;
        g.ph.ytry += g.objs[ob].yspeed;
        i += 1;
    }
}

/// T_KeenSwim (0x1edc5): during a stroke; another jump starts the next one.
pub fn t_keen_swim(g: &mut Game, ob: usize) {
    swim_bubble(g, ob);
    if g.jumpbutton && !g.jumpheld {
        g.jumpheld = true;
        g.objs[ob].xspeed = g.c.xaxis * 18;
        if g.c.yaxis != 0 {
            g.objs[ob].yspeed = g.c.yaxis * 18;
        }
        let st = g.objs[ob].st();
        g.objs[ob].state = Some(if std::ptr::eq(st, &S_KEENSWIM1) { &S_KEENSWIM2 } else { &S_KEENSWIM1 });
    }
    g.ph.xtry += g.objs[ob].xspeed * g.tics;
    g.ph.ytry += g.objs[ob].yspeed * g.tics;
    if g.ph.xtry > 0 {
        g.objs[ob].xdir = 1;
    } else if g.ph.xtry < 0 {
        g.objs[ob].xdir = -1;
    }
    g.ph.ytry += g.tics * 4;
}

/// C_KeenSwim (0x1ee77): collecting bonus items and rescuing the council
/// member while swimming.
pub fn c_keen_swim(g: &mut Game, _ob: usize, hit: usize) {
    match g.objs[hit].obclass {
        BONUSOBJ4 => {
            let kind = g.objs[hit].temp1;
            if !(0..=11).contains(&kind) {
                return;
            }
            let k = kind as usize;
            g.play_sound(BONUSSOUND[k]);
            g.objs[hit].obclass = INERTOBJ;
            g.objs[hit].priority = 3;
            g.objs[hit].shapenum = BONUSSPRITE[k];
            g.give_points(BONUSPOINTS[k]);
            if k < 4 {
                g.gs.keys[k] += 1;
            } else if k == 10 {
                g.gs.lives += 1;
            } else if k == 11 {
                g.gs.ammo += SHOTSINCLIP[g.gs.difficulty.clamp(0, 3) as usize];
            }
            g.change_state(hit, &S_BONUSRISE);
        }
        ORACLEOBJ => g.playstate = EX_RESCUED,
        _ => {}
    }
}

/// R_KeenSwim (0x1ef40): stops at walls and draws Keen.
pub fn r_keen_swim(g: &mut Game, ob: usize) {
    let o = &mut g.objs[ob];
    if (o.hiteast != 0 && o.xspeed < 0) || (o.hitwest != 0 && o.xspeed > 0) {
        o.xspeed = 0;
    }
    if (o.hitnorth != 0 && o.yspeed > 0) || (o.hitsouth != 0 && o.yspeed < 0) {
        o.yspeed = 0;
    }
    g.place_sprite(ob);
}
