//! CK_KEEN.C port: Commander Keen's own behaviour inside levels.
//! Translated from the decompiled segment 0x1a2f of KEEN6C.EXE.

use super::defs::*;
use super::states::*;
use super::Game;

// sounds (indices into AUDIO.CK6)
pub const SND_KEENWALK0: i32 = 0;
pub const SND_KEENWALK1: i32 = 1;
pub const SND_JUMP: i32 = 2;
pub const SND_LAND: i32 = 3;
pub const SND_KEENFIRE: i32 = 4;
pub const SND_WORLDWALK0: i32 = 5;
pub const SND_WORLDWALK1: i32 = 6;
pub const SND_PLAYERBOUNCE: i32 = 7;
pub const SND_GETPOINTS: i32 = 8;
pub const SND_GETAMMO: i32 = 9;
pub const SND_GETWATER: i32 = 10;
pub const SND_ENTERLEVEL: i32 = 12;
pub const SND_LEVELDONE: i32 = 13;
pub const SND_NOWAY: i32 = 14;
pub const SND_HELMETHIT: i32 = 15;
pub const SND_EXTRAKEEN: i32 = 17;
pub const SND_OPENGEMDOOR: i32 = 18;
pub const SND_GETGEM: i32 = 19;
pub const SND_PLUMMET: i32 = 20;
pub const SND_USESWITCH: i32 = 21;
pub const SND_KEENDEAD: i32 = 23;
pub const SND_SHOWSTATUS: i32 = 34;
pub const SND_HIDESTATUS: i32 = 35;

/// DS:0x1202 indexed by c.xaxis+1 (K4 0x12be, K5 0x12ac)
pub const POLEXSPEED: [i32; 3] = [-8, 0, 8];
/// DS:0x1208 ammo per difficulty (K4 0x12c4, K5 0x12b2)
pub const AMMOBYDIFF: [i32; 4] = [0, 8, 5, 5];
/// DS:0x1210 bonus sounds (K4 0x12cc, K5 0x12ba: 13 entries, the last
/// one for the keycard)
pub const BONUSSOUND: &[i32] = ep_sel!(
    &[19, 19, 19, 19, 8, 8, 8, 8, 8, 8, 17, 9],
    &[19, 19, 19, 19, 8, 8, 8, 8, 8, 8, 17, 9, 55],
    &[19, 19, 19, 19, 8, 8, 8, 8, 8, 8, 17, 9]
);
/// DS:0x1228 bonus points (K4 0x12e4, K5 0x12d4)
pub const BONUSPOINTS: &[i32] = ep_sel!(
    &[0, 0, 0, 0, 100, 200, 500, 1000, 2000, 5000, 0, 0],
    &[0, 0, 0, 0, 100, 200, 500, 1000, 2000, 5000, 0, 0, 0],
    &[0, 0, 0, 0, 100, 200, 500, 1000, 2000, 5000, 0, 0]
);
/// DS:0x1240 bonus sprites shown when collected (K4 0x12fc, K5 0x12ee)
pub const BONUSSPRITE: &[i32] = ep_sel!(
    &[242, 242, 242, 242, 211, 212, 213, 214, 215, 216, 217, 218],
    &[232, 232, 232, 232, 195, 196, 197, 198, 199, 200, 201, 202, 209],
    &[172, 172, 172, 172, 138, 139, 140, 141, 142, 143, 144, 145]
);
/// sprite of the extra life after 100 drops (GetDrop)
pub const DROPLIFESPR: i32 = ep_sel!(0xd2, 0xc2, 0x89);
/// DS:0x11f2 extra xtry when walking on slopes (K4 0x12ae, K5 0x129c)
pub const SLOPEWALK: [i32; 8] = [0, 0, 4, 4, 8, -4, -4, -8];

/// Sound of opening a door with a gem (Keen 5: the security door).
pub const SND_OPENDOOR: i32 = ep_sel!(18, 0x3c, 18);

/// Keen 5: the "nopan" mode (DS 0x6fde, set by a command line switch for
/// video cards without pixel panning) snaps riding Keen to 8 pixels; not
/// supported, the normal (panning) behaviour is ported.
#[allow(dead_code)]
const NOPAN: bool = false;

impl Game {
    /// moonok (K6 DS 0xca6b, K4 0xc823): 1 while Keen touches tile type 16;
    /// Keen 4 shows the moon then and sets 2.
    pub fn moonok(&self) -> i32 {
        self.moonok
    }

    pub fn set_moonok(&mut self, v: i32) {
        self.moonok = v;
    }

    fn up_pressed_tile(&self, ob: usize) -> i32 {
        let o = &self.objs[ob];
        self.intile(self.fg_tile(o.tilemidx, o.tiletop))
    }

    /// CheckGrabPole (0x1a33e)
    pub fn check_grab_pole(&mut self, ob: usize) -> bool {
        let now = self.lasttimecount as i64;
        if self.lastpoletime <= now {
            if now - self.lastpoletime < 19 {
                return false;
            }
        } else {
            self.lastpoletime = 0;
        }
        let o = &self.objs[ob];
        // Keen 4/5 look one tile below Keen's feet when pressing down
        let y = if self.c.yaxis == -1 {
            ((o.top + 0x60) as u16 >> 8) as i32
        } else {
            ep_sel!(o.tilebottom + 1, o.tilebottom + 1, o.tilebottom)
        };
        if self.intile(self.fg_tile(o.tilemidx, y)) & 0x7f != 1 {
            return false;
        }
        let tm = self.objs[ob].tilemidx;
        self.objs[ob].x = (tm - 1) * 256 + 0x80;
        self.ph.xtry = 0;
        self.ph.ytry = self.c.yaxis * 32;
        self.objs[ob].needtoclip = CL_NOCLIP;
        self.objs[ob].state = Some(&S_KEENPOLE);
        true
    }

    /// Switches and doors when pressing up (0x1a406, K4 0x1b133); Keen 5
    /// has its own version with the security doors (src/ck5/ep.rs).
    pub fn check_enter_house(&mut self, ob: usize) -> bool {
        ep_sel!(self.check_enter_house_std(ob), super::ep::check_enter_house(self, ob), self.check_enter_house_std(ob))
    }

    #[allow(dead_code)]
    fn check_enter_house_std(&mut self, ob: usize) -> bool {
        let it = self.up_pressed_tile(ob);
        let tm = self.objs[ob].tilemidx;
        if it == 5 || it == 6 || it == 15 {
            let target = tm * 256 - 0x40;
            if self.objs[ob].x == target {
                self.objs[ob].state = Some(&S_KEENSWITCH);
            } else {
                self.objs[ob].temp1 = target;
                self.objs[ob].state = Some(&S_KEENLINEUP);
            }
            self.upheld = true;
            return true;
        }
        if it == 2 {
            let base = tm * 256;
            let o = &self.objs[ob];
            let mut target = base + 0x60;
            if self.intile(self.fg_tile(tm - 1, o.tiletop)) == 2 {
                target = base - 0xa0;
            }
            if self.objs[ob].x == target {
                self.invincible = 110;
                self.objs[ob].state = Some(&S_KEENENTER1);
            } else {
                self.objs[ob].temp1 = target;
                self.objs[ob].state = Some(&S_KEENLINEUP);
            }
            self.upheld = true;
            return true;
        }
        false
    }

    /// KillKeen (0x1b557)
    pub fn kill_keen(&mut self) {
        let p = self.player;
        if self.invincible != 0 || self.godmode || same(self.objs[p].state, &S_KEENDEAD) {
            return;
        }
        self.set_moonok(0);
        self.invincible = 30;
        self.scrolllock = true;
        self.objs[p].needtoclip = CL_NOCLIP;
        self.objs[p].priority = 3;
        // Keen 4 (0x1c264): in the underwater level 17 Keen dies in his suit
        let st = ep_sel!(
            if self.ca_mapon == 17 {
                if self.rnd() < 0x80 { &S_KEENSWIMDIE1 } else { &S_KEENSWIMDIE2 }
            } else if self.rnd() < 0x80 {
                &S_KEENDIE1
            } else {
                &S_KEENDIE2
            },
            if self.rnd() < 0x80 { &S_KEENDIE1 } else { &S_KEENDIE2 },
            if self.rnd() < 0x80 { &S_KEENDIE1 } else { &S_KEENDIE2 }
        );
        self.change_state(p, st);
        self.play_sound(SND_KEENDEAD);
        self.objs[p].yspeed = -40;
        self.objs[p].xspeed = 16;
    }

    /// HandleRiding (0x1b710): keep Keen on top of a moving platform.
    pub fn handle_riding(&mut self, ob: usize) {
        let Some(plat) = self.gs.riding else { return };
        let (pl, pr, pt, pxm, pym, px) = {
            let p = &self.objs[plat];
            (p.left, p.right, p.top, p.xmove, p.ymove, p.x)
        };
        let o = &self.objs[ob];
        if (o.right as u16) < (pl as u16) || (pr as u16) < (o.left as u16) {
            self.gs.riding = None;
        } else if o.ymove < 0 {
            self.gs.riding = None;
            if pym < 0 {
                self.ph.xtry = 0;
                self.ph.ytry = pym;
                self.push_obj(ob);
            }
        } else {
            self.ph.xtry = pxm;
            self.ph.ytry = (pt - self.objs[ob].bottom) - 16;
            self.push_obj(ob);
            // (Keen 5 in nopan mode: x = x & 0xff80 | px & 0x7f)
            self.objs[ob].x |= px & 0x1f;
            if self.objs[ob].hitsouth == 0 {
                self.objs[ob].hitnorth = 25;
            } else {
                self.gs.riding = None;
            }
        }
    }

    /// GetBonus from a tile (0x1b796)
    pub fn get_bonus_tile(&mut self, x: i32, y: i32, kind: usize) {
        self.rf_mem_to_map(&[0], 1, x, y, 1, 1);
        self.play_sound(BONUSSOUND[kind]);
        self.give_points(BONUSPOINTS[kind]);
        if kind < 4 {
            self.gs.keys[kind] += 1;
        } else if kind == 10 {
            self.gs.lives += 1;
        } else if kind == 11 {
            self.gs.ammo += AMMOBYDIFF[self.gs.difficulty.clamp(0, 3) as usize];
        }
        let n = self.get_new_obj(true);
        let o = &mut self.objs[n];
        o.obclass = INERTOBJ;
        o.priority = 3;
        o.x = x << 8;
        o.y = y << 8;
        o.ydir = -1;
        o.temp2 = BONUSSPRITE[kind];
        o.shapenum = BONUSSPRITE[kind];
        self.new_state(n, &S_BONUSRISE);
        self.objs[n].needtoclip = CL_NOCLIP;
    }

    /// GetDrop / viva (0x1b860)
    pub fn get_drop(&mut self, x: i32, y: i32) {
        self.rf_mem_to_map(&[0], 1, x, y, 1, 1);
        self.play_sound(SND_GETWATER);
        ep_sel!(self.spawn_drop_splash(x, y), super::act1::spawn_splash(self, x, y), self.spawn_drop_splash(x, y));
        self.gs.drops += 1;
        if self.gs.drops == 100 {
            self.gs.drops = 0;
            self.play_sound(SND_EXTRAKEEN);
            self.gs.lives += 1;
            let n = self.get_new_obj(true);
            let o = &mut self.objs[n];
            o.obclass = INERTOBJ;
            o.priority = 3;
            o.x = x << 8;
            o.y = (y - 1) * 256;
            o.ydir = -1;
            o.shapenum = DROPLIFESPR;
            o.temp2 = DROPLIFESPR;
            self.new_state(n, &S_BONUSRISE);
            self.objs[n].needtoclip = CL_NOCLIP;
        }
    }

    /// CheckInTiles (0x1b90a): items and special tiles touched by Keen.
    pub fn check_in_tiles(&mut self, ob: usize) {
        if self.moonok() == 1 {
            self.set_moonok(0);
        }
        let (tl, tt, tr, tb) = {
            let o = &self.objs[ob];
            (o.tileleft, o.tiletop, o.tileright, o.tilebottom)
        };
        let mut y = tt;
        while y <= tb {
            let mut x = tl;
            while x <= tr {
                let it = self.intile(self.fg_tile(x, y)) & 0x7f;
                match it {
                    3 => self.kill_keen(),
                    4 => self.get_drop(x, y),
                    7..=10 => {
                        let o = &self.objs[ob];
                        if o.tilebottom != y || o.hitnorth == 0 || same(o.state, &S_KEENKEY) {
                            return;
                        }
                        let k = (it - 7) as usize;
                        if self.gs.keys[k] == 0 {
                            return;
                        }
                        let target = (x << 8) - 0x40;
                        if self.objs[ob].x != target {
                            self.objs[ob].temp1 = target;
                            self.objs[ob].state = Some(&S_KEENLINEUP);
                            return;
                        }
                        self.gs.keys[k] -= 1;
                        self.change_state(ob, &S_KEENKEY);
                    }
                    16 => {
                        if self.moonok() == 0 {
                            self.set_moonok(1);
                        }
                    }
                    21..=28 => self.get_bonus_tile(x, y, (it - 21 + 4) as usize),
                    _ => {}
                }
                x += 1;
            }
            y += 1;
        }
    }
}

pub fn walk_sound1(g: &mut Game, _ob: usize) {
    g.play_sound(SND_KEENWALK0);
}

pub fn walk_sound2(g: &mut Game, _ob: usize) {
    g.play_sound(SND_KEENWALK1);
}

/// KeenStandThink (0x1a507)
pub fn keen_stand_think(g: &mut Game, ob: usize) {
    ep5! {
        // Keen 5 (0x1ae1b): Keen faces the player while riding a platform
        if g.objs[ob].hitnorth == 25 && !same(g.objs[ob].state, &S_KEENONPLATFORM) {
            g.objs[ob].state = Some(&S_KEENONPLATFORM);
        }
    }
    if g.c.xaxis != 0 {
        g.objs[ob].state = Some(&S_KEENWALK1);
        keen_walk_think(g, ob);
        let o = &g.objs[ob];
        g.ph.xtry = (o.xdir * o.st().xmove * g.tics) / 4;
        return;
    }
    if g.firebutton && !g.fireheld {
        g.fireheld = true;
        g.objs[ob].state = Some(if g.c.yaxis == -1 { &S_KEENSHOOTUP1 } else { &S_KEENSHOOT1 });
        return;
    }
    if g.jumpbutton && !g.jumpheld {
        g.jumpheld = true;
        g.play_sound(SND_JUMP);
        g.objs[ob].xspeed = 0;
        g.objs[ob].yspeed = -40;
        g.ph.ytry = 0;
        g.jumptime = 18;
        g.objs[ob].state = Some(&S_KEENJUMP1);
        return;
    }
    if g.pogobutton && !g.pogoheld {
        g.pogoheld = true;
        g.play_sound(SND_JUMP);
        g.objs[ob].state = Some(&S_KEENPOGODOWN);
        g.objs[ob].xspeed = 0;
        g.objs[ob].yspeed = -48;
        g.ph.ytry = 0;
        g.jumptime = 24;
        return;
    }
    if g.c.yaxis == -1 {
        if !g.check_grab_pole(ob) && (g.upheld || !g.check_enter_house(ob)) {
            g.objs[ob].state = Some(&S_KEENLOOKUP);
        }
    } else if g.c.yaxis == 1 && !g.check_grab_pole(ob) {
        g.objs[ob].state = Some(&S_KEENLOOKDOWN);
    }
}

/// KeenPauseThink (0x1a625): idle animations of the standing Keen.
/// Keen 4 (0x1b352) moons when he stood on the moon tile, Keen 5 (0x1af5d)
/// faces the player on platforms.
pub fn keen_pause_think(g: &mut Game, ob: usize) {
    ep5! {
        if g.objs[ob].hitnorth == 25 && !same(g.objs[ob].state, &S_KEENONPLATFORM) {
            g.objs[ob].state = Some(&S_KEENONPLATFORM);
        }
    }
    if g.c.dir.id() == DIR_NONE && !g.jumpbutton && !g.pogobutton && !g.firebutton {
        if (g.objs[ob].hitnorth & !7) != 24 {
            g.objs[ob].temp1 += g.tics;
        }
        let o = &mut g.objs[ob];
        match o.temp2 {
            0 => {
                if o.temp1 > 200 {
                    o.temp2 += 1;
                    o.state = Some(&S_KEENPAUSELOOK);
                    o.temp1 = 0;
                }
            }
            1 => {
                if o.temp1 > 300 {
                    o.temp2 += 1;
                    o.temp1 = 0;
                    ep4! {
                        if g.moonok() == 1 {
                            g.set_moonok(2);
                            g.objs[ob].state = Some(&S_KEENMOON1);
                            return;
                        }
                    }
                    g.objs[ob].state = Some(&S_KEENWAIT1);
                }
            }
            2 => {
                if o.temp1 > 700 {
                    o.temp2 += 1;
                    o.state = Some(&S_KEENSIT1);
                    o.temp1 = 0;
                }
            }
            _ => {}
        }
    } else {
        let o = &mut g.objs[ob];
        o.temp2 = 0;
        o.temp1 = 0;
        o.state = Some(&S_KEENSTAND);
        keen_stand_think(g, ob);
    }
}

/// KeenReadThink (0x1a6c9)
pub fn keen_read_think(g: &mut Game, ob: usize) {
    if g.storedemo {
        g.playstate = EX_ABORTGAME;
        g.sys.input.clear_keys_down();
    }
    if g.c.dir.id() != DIR_NONE || g.jumpbutton || g.pogobutton {
        let o = &mut g.objs[ob];
        o.temp2 = 0;
        o.temp1 = 0;
        o.state = Some(&S_KEENSTOPREAD1);
    }
}

/// KeenLookUpThink (0x1a707)
pub fn keen_look_up_think(g: &mut Game, ob: usize) {
    if g.c.yaxis != -1
        || g.c.xaxis != 0
        || (g.jumpbutton && !g.jumpheld)
        || (g.pogobutton && !g.pogoheld)
        || g.firebutton
    {
        g.objs[ob].state = Some(&S_KEENSTAND);
        keen_stand_think(g, ob);
    }
}

/// KeenLookDownThink (0x1a74e): also drops through one-way floors.
pub fn keen_look_down_think(g: &mut Game, ob: usize) {
    if g.jumpbutton && !g.jumpheld && (g.objs[ob].hitnorth & 7) == 1 {
        g.jumpheld = true;
        let (tm, tb) = (g.objs[ob].tilemidx, g.objs[ob].tilebottom);
        let t = g.fg_tile(tm, tb);
        if g.west(t) != 0 || g.east(t) != 0 || g.south(t) != 0 {
            return;
        }
        let t2 = g.fg_tile(tm, tb + 1);
        if g.west(t2) != 0 || g.east(t2) != 0 || g.south(t2) != 0 {
            return;
        }
        let mut ymove = g.tics.max(4) * 16;
        if let Some(r) = g.gs.riding {
            ymove += g.objs[r].ymove;
        }
        g.objs[ob].bottom += ymove;
        g.gs.riding = None;
        g.objs[ob].y += ymove;
        g.ph.ytry = 0;
        g.ph.xtry = 0;
        g.objs[ob].state = Some(&S_KEENFALL1);
        g.objs[ob].yspeed = 0;
        g.objs[ob].xspeed = 0;
        g.play_sound(SND_PLUMMET);
        return;
    }
    if g.c.yaxis != 1 || g.c.xaxis != 0 || (g.jumpbutton && !g.jumpheld) || (g.pogobutton && !g.pogoheld) {
        g.objs[ob].state = Some(&S_KEENLOOKDOWN4);
    }
}

/// KeenWalkThink (0x1a893)
pub fn keen_walk_think(g: &mut Game, ob: usize) {
    if g.c.xaxis == 0 {
        g.objs[ob].state = Some(&S_KEENSTAND);
        keen_stand_think(g, ob);
        return;
    }
    g.objs[ob].xdir = g.c.xaxis;
    if g.c.yaxis == -1 {
        if g.check_grab_pole(ob) {
            return;
        }
        if g.upheld {
            return;
        }
        if g.check_enter_house(ob) {
            return;
        }
    } else if g.c.yaxis == 1 && g.check_grab_pole(ob) {
        return;
    }
    if g.firebutton && !g.fireheld {
        g.fireheld = true;
        g.objs[ob].state = Some(if g.c.yaxis == -1 { &S_KEENSHOOTUP1 } else { &S_KEENSHOOT1 });
        return;
    }
    if g.jumpbutton && !g.jumpheld {
        g.jumpheld = true;
        g.play_sound(SND_JUMP);
        g.objs[ob].xspeed = g.objs[ob].xdir * 16;
        g.objs[ob].yspeed = -40;
        g.ph.ytry = 0;
        g.ph.xtry = 0;
        g.jumptime = 18;
        g.objs[ob].state = Some(&S_KEENJUMP1);
    }
    if g.pogobutton && !g.pogoheld {
        g.pogoheld = true;
        g.objs[ob].state = Some(&S_KEENPOGODOWN);
        g.play_sound(SND_JUMP);
        g.objs[ob].xspeed = g.objs[ob].xdir * 16;
        g.objs[ob].yspeed = -48;
        g.ph.xtry = 0;
        g.jumptime = 24;
        return;
    }
    // slopes give extra speed
    g.ph.xtry += SLOPEWALK[(g.objs[ob].hitnorth & 7) as usize] * g.tics;
    let o = &mut g.objs[ob];
    if same(o.state, &S_KEENWALK1) && o.temp3 == 0 {
        g.play_sound(SND_KEENWALK0);
        g.objs[ob].temp3 = 1;
    } else if same(o.state, &S_KEENWALK3) && o.temp3 == 0 {
        g.play_sound(SND_KEENWALK1);
        g.objs[ob].temp3 = 1;
    } else if same(o.state, &S_KEENWALK2) || same(o.state, &S_KEENWALK4) {
        o.temp3 = 0;
    }
}

/// KeenLineUpThink (0x1aa17): walk to the exact spot of a switch/door/holder.
pub fn keen_line_up_think(g: &mut Game, ob: usize) {
    let dist = g.objs[ob].temp1 - g.objs[ob].x;
    if dist < 0 {
        g.ph.xtry -= g.tics * 16;
        if dist < g.ph.xtry {
            return;
        }
    } else if dist > 0 {
        g.ph.xtry += g.tics * 16;
        if g.ph.xtry < dist {
            return;
        }
    }
    g.ph.xtry = dist;
    g.objs[ob].temp1 = 0;
    if !g.check_enter_house(ob) {
        g.objs[ob].state = Some(&S_KEENSTAND);
    }
}

/// KeenEnterThink (0x1aa7c): teleport through a door (info plane: x<<8|y).
pub fn keen_enter_think(g: &mut Game, ob: usize) {
    let (tl, tb) = (g.objs[ob].tileleft, g.objs[ob].tilebottom);
    let info = g.map_tile(2, tl, tb) as i32;
    ep5! {
        // Keen 5 (0x1b407): info 0 is a teleporter, 0xb1b1 the exit door
        if info == 0 {
            g.playstate = EX_PORTOUT;
            g.objs[ob].state = Some(&S_KEENENTER6);
            return;
        }
        if info == 0xb1b1 {
            g.playstate = EX_COMPLETED;
            g.objs[ob].state = Some(&S_KEENENTER6);
            return;
        }
    }
    g.objs[ob].y = (info & 0xff) * 256 - 0xf1;
    g.objs[ob].x = info & 0xff00;
    g.objs[ob].priority = 1;
    g.objs[ob].needtoclip = CL_NOCLIP;
    let next = g.objs[ob].st().next.unwrap_or(&S_KEENSTAND);
    g.change_state(ob, next);
    g.objs[ob].needtoclip = CL_MIDCLIP;
    g.center_actor(ob);
}

/// KeenSwitchThink (0x1aaf2)
pub fn keen_switch_think(g: &mut Game, ob: usize) {
    let (tm, tt) = (g.objs[ob].tilemidx, g.objs[ob].tiletop);
    let tile = g.fg_tile(tm, tt);
    let newtile = (tile as i32 + g.data.tileinfo.manim[tile] as i32) as u16;
    let info = g.map_tile(2, tm, tt) as i32;
    let tx = info >> 8;
    let ty = info & 0xff;
    let it = g.intile(tile);
    g.rf_mem_to_map(&[newtile], 1, tm, tt, 1, 1);
    g.play_sound(SND_USESWITCH);
    if it == 15 {
        // extend/retract a bridge: two rows starting at the target
        for y in ty..ty + 2 {
            let mut x = if y != ty { tx - 1 } else { tx };
            while x < g.mapwidth {
                let t = g.fg_tile(x, y);
                let a = g.data.tileinfo.manim[t];
                if a == 0 {
                    break;
                }
                let nt = (t as i32 + a as i32) as u16;
                g.rf_mem_to_map(&[nt], 1, x, y, 1, 1);
                x += 1;
            }
        }
    } else {
        // toggle the platform stop/go arrow in the info plane
        let v = g.map_tile(2, tx, ty) as u16;
        ep5! {
            // Keen 5 (0x1b4aa): the platform direction arrows 0x5b..0x62
            // turn around (DS:0x878)
            if (0x5b..0x63).contains(&v) {
                const TURN: [u16; 8] = [2, 3, 0, 1, 6, 7, 4, 5];
                g.set_map_tile(2, tx, ty, TURN[(v - 0x5b) as usize] + 0x5b);
                return;
            }
        }
        g.set_map_tile(2, tx, ty, v ^ 0x1f);
    }
}

/// KeenKeyThink (0x1ac60): place a gem in a holder and open its door.
pub fn keen_key_think(g: &mut Game, ob: usize) {
    let (tm, tb) = (g.objs[ob].tilemidx, g.objs[ob].tilebottom);
    let tile = g.fg_tile(tm, tb) as u16 + 0x12;
    let info = g.map_tile(2, tm, tb) as i32;
    let dx = info >> 8;
    let dy = info & 0xff;
    g.rf_mem_to_map(&[tile], 1, tm, tb, 1, 1);
    g.play_sound(SND_OPENDOOR);
    let n = g.get_new_obj(false);
    g.objs[n].x = dx;
    g.objs[n].y = dy;
    if dx > g.mapwidth || dx < 2 || dy > g.mapheight || dy < 2 {
        g.quit("Keyholder points to a bad spot!");
        return;
    }
    let doortile = g.fg_tile(dx, dy);
    let mut h = 1;
    while g.fg_tile(dx, dy + h) == doortile {
        h += 1;
    }
    let o = &mut g.objs[n];
    o.temp1 = h;
    o.active = AC_ALLWAYS;
    o.needtoclip = CL_NOCLIP;
    o.obclass = INERTOBJ;
    g.new_state(n, &S_DOOR1);
}

/// KeenAirThink (0x1ad7f)
pub fn keen_air_think(g: &mut Game, ob: usize) {
    if g.jumpcheat && g.jumpbutton {
        g.objs[ob].yspeed = -40;
        g.jumptime = 18;
        g.jumpheld = true;
    }
    if g.jumptime == 0 {
        if g.gs.difficulty == GD_EASY {
            g.do_weak_gravity(ob);
        } else {
            g.do_gravity(ob);
        }
        let o = &g.objs[ob];
        if o.yspeed > 0 && !same(o.state, &S_KEENFALL1) && !same(o.state, &S_KEENFALL2) {
            g.objs[ob].state = o.st().next;
        }
    } else {
        if g.tics < g.jumptime {
            g.ph.ytry = g.objs[ob].yspeed * g.tics;
            if !g.jumpcheat {
                g.jumptime -= g.tics;
            }
        } else {
            g.ph.ytry = g.objs[ob].yspeed * g.jumptime;
            g.jumptime = 0;
        }
        if !g.jumpbutton {
            g.jumptime = 0;
        }
        if g.jumptime == 0 && g.objs[ob].st().next.is_some() {
            g.objs[ob].state = g.objs[ob].st().next;
        }
    }
    if g.c.xaxis == 0 {
        g.friction_x(ob);
    } else {
        g.objs[ob].xdir = g.c.xaxis;
        g.accelerate_x(ob, g.c.xaxis * 2, 24);
    }
    if g.objs[ob].hitsouth == 17 {
        g.ph.xtry = 0;
        g.objs[ob].xspeed = 0;
    }
    if g.firebutton && !g.fireheld {
        g.fireheld = true;
        match g.c.yaxis {
            -1 => {
                g.objs[ob].state = Some(&S_KEENAIRSHOOTUP1);
                return;
            }
            0 => {
                g.objs[ob].state = Some(&S_KEENAIRSHOOT1);
                return;
            }
            1 => {
                g.objs[ob].state = Some(&S_KEENAIRSHOOTDOWN1);
                return;
            }
            _ => {}
        }
    }
    if g.pogobutton && !g.pogoheld {
        g.pogoheld = true;
        g.objs[ob].state = Some(&S_KEENPOGO);
        g.jumptime = 0;
    } else if g.c.yaxis == -1 {
        g.check_grab_pole(ob);
    }
}

/// KeenBounceThink (0x1aee4): start of a pogo jump.
pub fn keen_bounce_think(g: &mut Game, ob: usize) {
    g.objs[ob].yspeed = -48;
    g.ph.ytry = g.objs[ob].yspeed * 6;
    g.jumptime = 24;
    g.play_sound(SND_PLAYERBOUNCE);
}

/// KeenPogoThink (0x1af0f)
pub fn keen_pogo_think(g: &mut Game, ob: usize) {
    if g.jumptime == 0 {
        if g.gs.difficulty == GD_EASY {
            g.do_weak_gravity(ob);
        } else {
            g.do_gravity(ob);
        }
    } else {
        if !g.jumpbutton && g.jumptime > 9 {
            g.do_gravity(ob);
        } else {
            g.do_tiny_gravity(ob);
        }
        if g.tics < g.jumptime {
            g.jumptime -= g.tics;
        } else {
            g.jumptime = 0;
        }
        if g.jumptime == 0 && g.objs[ob].st().next.is_some() {
            g.objs[ob].state = g.objs[ob].st().next;
        }
    }
    if g.c.xaxis == 0 {
        g.ph.xtry += g.objs[ob].xspeed * g.tics;
        let xs = g.objs[ob].xspeed;
        if xs > 0 {
            g.objs[ob].xdir = 1;
        } else if xs < 0 {
            g.objs[ob].xdir = -1;
        }
    } else {
        if g.objs[ob].xspeed == 0 {
            g.objs[ob].xdir = g.c.xaxis;
        }
        g.accelerate_x(ob, g.c.xaxis, 24);
    }
    if g.objs[ob].hitsouth == 17 {
        g.ph.xtry = 0;
        g.objs[ob].xspeed = 0;
    }
    if g.firebutton && !g.fireheld {
        g.fireheld = true;
        match g.c.yaxis {
            -1 => {
                g.objs[ob].state = Some(&S_KEENAIRSHOOTUP1);
                return;
            }
            0 => {
                g.objs[ob].state = Some(&S_KEENAIRSHOOT1);
                return;
            }
            1 => {
                g.objs[ob].state = Some(&S_KEENAIRSHOOTDOWN1);
                return;
            }
            _ => {}
        }
    }
    if g.pogobutton && !g.pogoheld {
        g.pogoheld = true;
        g.objs[ob].state = Some(&S_KEENFALL1);
    }
}

/// PoleActions (0x1b03d)
fn pole_actions(g: &mut Game, ob: usize) {
    if g.c.xaxis != 0 {
        g.objs[ob].xdir = g.c.xaxis;
    }
    if g.firebutton && !g.fireheld {
        g.fireheld = true;
        match g.c.yaxis {
            -1 => g.objs[ob].state = Some(&S_KEENPOLESHOOTUP1),
            0 => g.objs[ob].state = Some(&S_KEENPOLESHOOT1),
            1 => g.objs[ob].state = Some(&S_KEENPOLESHOOTDOWN1),
            _ => {}
        }
    }
    if g.jumpbutton && !g.jumpheld {
        g.jumpheld = true;
        g.play_sound(SND_JUMP);
        g.objs[ob].xspeed = POLEXSPEED[(g.c.xaxis + 1) as usize];
        g.objs[ob].yspeed = -20;
        g.objs[ob].needtoclip = CL_MIDCLIP;
        g.jumptime = 10;
        g.objs[ob].state = Some(&S_KEENJUMP1);
        g.objs[ob].ydir = 1;
        g.lastpoletime = g.lasttimecount as i64;
    }
}

/// KeenPoleThink (0x1b0e2)
pub fn keen_pole_think(g: &mut Game, ob: usize) {
    if g.c.yaxis == -1 {
        g.objs[ob].state = Some(&S_KEENCLIMB1);
        g.objs[ob].ydir = -1;
    } else if g.c.yaxis == 1 {
        g.objs[ob].state = Some(&S_KEENSLIDE1);
        g.objs[ob].ydir = 1;
        keen_drop_think(g, ob);
    } else if g.c.xaxis == 0 || {
        let o = &g.objs[ob];
        g.north(g.fg_tile(o.tilemidx, o.tilebottom + 1)) == 0
    } {
        pole_actions(g, ob);
    } else {
        // walk off the pole onto a floor
        let o = &mut g.objs[ob];
        o.xspeed = 0;
        o.yspeed = 0;
        o.needtoclip = CL_MIDCLIP;
        g.jumptime = 0;
        o.state = Some(&S_KEENFALL1);
        o.ydir = 1;
        g.play_sound(SND_PLUMMET);
    }
}

/// KeenClimbThink (0x1b185)
pub fn keen_climb_think(g: &mut Game, ob: usize) {
    let o = &g.objs[ob];
    if g.intile(g.fg_tile(o.tilemidx, o.tiletop)) & 0x7f == 1 {
        if g.c.yaxis == 0 {
            g.objs[ob].state = Some(&S_KEENPOLE);
            g.objs[ob].ydir = 0;
        } else if g.c.yaxis == 1 {
            g.objs[ob].state = Some(&S_KEENSLIDE1);
            g.objs[ob].ydir = 1;
            keen_drop_think(g, ob);
        }
    } else {
        g.ph.ytry = 0;
        g.objs[ob].state = Some(&S_KEENPOLE);
    }
    pole_actions(g, ob);
}

/// KeenDropThink (0x1b20b): sliding down a pole.
pub fn keen_drop_think(g: &mut Game, ob: usize) {
    let o = &g.objs[ob];
    let y = ((o.bottom - 0x40) as u16 >> 8) as i32;
    if g.intile(g.fg_tile(o.tilemidx, y)) & 0x7f == 1 {
        if g.c.yaxis == -1 {
            g.objs[ob].state = Some(&S_KEENCLIMB1);
            g.objs[ob].ydir = -1;
        } else if g.c.yaxis == 0 {
            g.objs[ob].state = Some(&S_KEENPOLE);
            g.objs[ob].ydir = 0;
        }
        pole_actions(g, ob);
    } else {
        g.play_sound(SND_PLUMMET);
        let o = &mut g.objs[ob];
        o.state = Some(&S_KEENFALL1);
        g.jumptime = 0;
        o.xspeed = POLEXSPEED[(g.c.xaxis + 1) as usize];
        o.yspeed = 0;
        o.needtoclip = CL_MIDCLIP;
        o.tilebottom -= 1;
    }
}

/// s_keendrop think (0x1b2bb)
pub fn keen_drop_down_think(g: &mut Game, ob: usize) {
    g.objs[ob].needtoclip = CL_MIDCLIP;
}

/// KeenHoldThink (0x1b2ca, K4 0x1c00b): hanging on a ledge. Keen 5
/// (0x1bcfe) moves Keen up in the thinks of the climb up states.
pub fn keen_hold_think(g: &mut Game, ob: usize) {
    let o = g.objs[ob].clone();
    if g.c.yaxis == -1 || o.xdir == g.c.xaxis {
        g.objs[ob].state = Some(&S_KEENCLIMBUP1);
        let x = if o.xdir == 1 { o.tileright } else { o.tileleft };
        ep5! {
            g.ph.ytry = if o.xdir == 1 { -0x100 } else { -0x80 };
        }
        if g.intile(g.fg_tile(x, o.tiletop - 1)) & 0x80 == 0 {
            g.objs[ob].priority = 3;
        }
    } else if g.c.yaxis == 1 || (g.c.xaxis != 0 && o.xdir != g.c.xaxis) {
        g.objs[ob].state = Some(&S_KEENFALL1);
        g.objs[ob].needtoclip = CL_MIDCLIP;
    }
}

/// KeenShootThink (0x1b35c): spawns the stunner shot for the current state.
/// Keen 5 (0x1bdb3) shoots up/down from poles at other x offsets.
pub fn keen_shoot_think(g: &mut Game, ob: usize) {
    let o = g.objs[ob].clone();
    let st = o.st();
    let r = o.xdir == 1;
    if std::ptr::eq(st, &S_KEENSHOOT1) {
        if r {
            g.spawn_shot(o.x + 0x100, o.y + 0x40, 2);
        } else {
            g.spawn_shot(o.x - 0x80, o.y + 0x40, 6);
        }
    }
    if std::ptr::eq(st, &S_KEENAIRSHOOT2) {
        if r {
            g.spawn_shot(o.x + 0x100, o.y + 0x20, 2);
        } else {
            g.spawn_shot(o.x, o.y + 0x20, 6);
        }
    }
    if std::ptr::eq(st, &S_KEENAIRSHOOTDOWN2) {
        g.spawn_shot(o.x + 0x80, o.y + 0x120, 4);
    }
    if std::ptr::eq(st, &S_KEENAIRSHOOTUP2) {
        g.spawn_shot(o.x + 0x50, o.y - 0xa0, 0);
    }
    if std::ptr::eq(st, &S_KEENSHOOTUP1) {
        g.spawn_shot(o.x + 0x50, o.y - 0xa0, 0);
    }
    if std::ptr::eq(st, &S_KEENPOLESHOOT1) {
        if r {
            g.spawn_shot(o.x + 0x100, o.y + 0x40, 2);
        } else {
            g.spawn_shot(o.x - 0x80, o.y + 0x40, 6);
        }
    }
    if std::ptr::eq(st, &S_KEENPOLESHOOTUP1) {
        if r {
            g.spawn_shot(o.x + ep_sel!(0x20, 0x60, 0x20), o.y + 0x40, 0);
        } else {
            g.spawn_shot(o.x + 0xc0, o.y + 0x40, 0);
        }
    }
    if std::ptr::eq(st, &S_KEENPOLESHOOTDOWN1) {
        if r {
            g.spawn_shot(o.x + ep_sel!(0, 0x60, 0), o.y + 0x180, 4);
        } else {
            g.spawn_shot(o.x + ep_sel!(0x40, 0xc0, 0x40), o.y + 0x180, 4);
        }
    }
}

/// end of climbing up a ledge (0x1b4e1, K4 0x1c222, K5 0x1bfbd)
pub fn t_pull_up_done(g: &mut Game, ob: usize) {
    g.objs[ob].needtoclip = CL_MIDCLIP;
    g.objs[ob].priority = 1;
    ep5! {
        g.ph.ytry = 0x80;
    }
}

ep5! {
    /// T_PullUp1 (K5 0x1bf68): first step of climbing up a ledge.
    pub fn t_pull_up1(g: &mut Game, ob: usize) {
        if g.objs[ob].xdir == 1 {
            g.ph.xtry = 0x80;
        } else {
            g.ph.ytry = -0x80;
        }
    }

    /// T_PullUp2 (K5 0x1bf87)
    pub fn t_pull_up2(g: &mut Game, ob: usize) {
        if g.objs[ob].xdir == 1 {
            g.ph.xtry = 0x80;
            g.ph.ytry = -0x80;
        } else {
            g.ph.xtry = -0x80;
            g.ph.ytry = -0x80;
        }
    }

    /// T_PullUp3 (K5 0x1bfb2)
    pub fn t_pull_up3(g: &mut Game, _ob: usize) {
        g.ph.ytry = -0x80;
    }
}

pub fn t_x_stand(_g: &mut Game, _ob: usize) {}
pub fn t_x_jump(_g: &mut Game, _ob: usize) {}
pub fn t_x_pogo(_g: &mut Game, _ob: usize) {}
pub fn t_x_pole(_g: &mut Game, _ob: usize) {}

/// KeenDieThink (0x1b529)
pub fn keen_die_think(g: &mut Game, ob: usize) {
    g.do_weak_gravity(ob);
    g.ph.xtry = g.objs[ob].xspeed * g.tics;
    if !g.on_screen(ob) {
        g.playstate = EX_DIED;
    }
}

/// KeenContact: what touching an object does, per episode (src/ckN/ep.rs).
pub fn keen_contact(g: &mut Game, ob: usize, hit: usize) {
    super::ep::keen_contact(g, ob, hit);
}

/// KeenPosContact: while on a pole etc. landing on platform-like objects,
/// per episode (src/ckN/ep.rs).
pub fn keen_pos_contact(g: &mut Game, ob: usize, hit: usize) {
    super::ep::keen_pos_contact(g, ob, hit);
}

/// KeenPoleReact (0x1ba95)
pub fn keen_pole_react(g: &mut Game, ob: usize) {
    g.place_sprite(ob);
}

/// KeenSimpleReact (0x1babc): standing states.
pub fn keen_simple_react(g: &mut Game, ob: usize) {
    let hn = g.objs[ob].hitnorth;
    if hn == 0 {
        g.play_sound(SND_PLUMMET);
        g.objs[ob].xspeed = g.objs[ob].xdir << 3;
        g.objs[ob].yspeed = 0;
        g.change_state(ob, &S_KEENFALL1);
        g.jumptime = 0;
    } else if (hn & !7) == 8 {
        g.kill_keen();
    } else if hn == 0x29 || hn == 0x31 {
        g.ph.xtry = if hn == 0x29 { g.tics << 3 } else { g.tics * -8 };
        g.ph.ytry = 0;
        g.objs[ob].temp1 = 0;
        g.clip_to_walls(ob);
    }
    g.place_sprite(ob);
}

/// KeenWalkReact (0x1bb60)
pub fn keen_walk_react(g: &mut Game, ob: usize) {
    let hn = g.objs[ob].hitnorth;
    if hn == 0 {
        g.play_sound(SND_PLUMMET);
        g.objs[ob].xspeed = g.objs[ob].xdir << 3;
        g.objs[ob].yspeed = 0;
        g.change_state(ob, &S_KEENFALL1);
        g.jumptime = 0;
    } else if (hn & !7) == 8 {
        g.kill_keen();
    } else if hn == 0x29 {
        g.ph.xtry = g.tics << 3;
        g.ph.ytry = 0;
        g.clip_to_walls(ob);
    } else if hn == 0x31 {
        g.ph.xtry = g.tics * -8;
        g.ph.ytry = 0;
        g.clip_to_walls(ob);
    } else {
        let o = &g.objs[ob];
        if (o.hiteast != 0 && o.xdir == -1) || (o.hitwest != 0 && o.xdir == 1) {
            let o = &mut g.objs[ob];
            o.ticcount = 0;
            o.state = Some(&S_KEENSTAND);
            o.shapenum = if o.xdir == 1 { S_KEENSTAND.right } else { S_KEENSTAND.left };
        }
    }
    g.place_sprite(ob);
}

/// Common ceiling handling of the air and pogo reacts. Keen 5 has no
/// pole holes in ceilings and no helmet hit while passing a pole hole.
fn air_ceiling(g: &mut Game, ob: usize) -> bool {
    {
        let o = &g.objs[ob];
        if (o.hiteast != 0 && o.xdir == -1) || (o.hitwest != 0 && o.xdir == 1) {
            g.objs[ob].xspeed = 0;
        }
    }
    if ep_sel!(true, false, true) && g.objs[ob].hitsouth != 0 {
        // pole holes: the original scans the rows starting one tile above
        let (tl, tt, tr, tb) = {
            let o = &g.objs[ob];
            (o.tileleft, o.tiletop, o.tileright, o.tilebottom)
        };
        let mut y = tt;
        while y <= tb {
            let mut x = tl;
            while x <= tr {
                if g.south(g.fg_tile(x, y - 1)) == 17 {
                    let o = &mut g.objs[ob];
                    o.xspeed = 0;
                    o.x = o.tilemidx * 256 - 0x20;
                    let d = o.yspeed * g.tics;
                    o.y += d;
                    o.top += d;
                    return true;
                }
                x += 1;
            }
            y += 1;
        }
    }
    let hs = g.objs[ob].hitsouth;
    if hs != 0 {
        if hs == 17 {
            let o = &mut g.objs[ob];
            o.y -= 0x20;
            o.top -= 0x20;
            o.xspeed = 0;
            o.x = o.tilemidx * 256 - 0x20;
        } else if hs == 0x21 {
            ep6! { g.hit_special_tile(ob, false); }
        }
        if ep_sel!(true, hs != 17, true) && !g.jumpcheat {
            g.play_sound(SND_HELMETHIT);
            let o = &mut g.objs[ob];
            if hs < 2 {
                o.yspeed = 0;
            } else {
                o.yspeed += 16;
                if o.yspeed < 0 {
                    o.yspeed = 0;
                }
            }
            g.jumptime = 0;
        }
    }
    false
}

/// KeenAirReact (0x1bc46)
pub fn keen_air_react(g: &mut Game, ob: usize) {
    air_ceiling(g, ob);
    if g.objs[ob].hitnorth == 0 {
        // grab ledges while falling
        let o = g.objs[ob].clone();
        if o.ymove > 0 {
            let ledgey = (o.top - 0x40) & 0xff00;
            let edge = ledgey + 0x40;
            let row = ((edge as u16) >> 8) as i32 - 1;
            if ((o.top - o.ymove) as u16) < edge as u16 && edge as u16 <= o.top as u16 {
                if g.c.xaxis == -1 {
                    let mut x = o.tileleft;
                    if o.hiteast != 0 {
                        x -= 1;
                    }
                    let t = g.fg_tile(x, row);
                    let b = g.fg_tile(x, row + 1);
                    if g.east(t) == 0
                        && g.west(t) == 0
                        && g.north(t) == 0
                        && g.south(t) == 0
                        && g.east(b) != 0
                        && g.north(b) != 0
                    {
                        let o = &mut g.objs[ob];
                        o.xdir = -1;
                        o.needtoclip = CL_NOCLIP;
                        o.x = (o.x & 0xff00) + 0x80;
                        o.y = ledgey;
                        o.ymove = 0;
                        o.yspeed = 0;
                        g.change_state(ob, &S_KEENHOLDON);
                    }
                } else if g.c.xaxis == 1 {
                    let mut x = o.tileright;
                    if o.hitwest != 0 {
                        x += 1;
                    }
                    let t = g.fg_tile(x, row);
                    let b = g.fg_tile(x, row + 1);
                    if g.east(t) == 0
                        && g.west(t) == 0
                        && g.north(t) == 0
                        && g.south(t) == 0
                        && g.west(b) != 0
                        && g.north(b) != 0
                    {
                        let o = &mut g.objs[ob];
                        o.xdir = 1;
                        o.needtoclip = CL_NOCLIP;
                        o.x = (o.x & 0xff00) + 0x100;
                        o.y = ledgey;
                        o.ymove = 0;
                        o.yspeed = 0;
                        g.change_state(ob, &S_KEENHOLDON);
                    }
                }
            }
        }
    } else {
        g.objs[ob].ymove = 0;
        let hn = g.objs[ob].hitnorth;
        if (hn & !7) == 8 {
            g.kill_keen();
        } else {
            if hn == 0x21 {
                ep6! { g.hit_special_tile(ob, true); }
            }
            ep5! {
                // Keen 5 (0x1c75c): landing on a fuse
                if hn == 0x39 {
                    g.play_sound(SND_FUSE);
                }
            }
            if hn != 25 || g.jumptime == 0 {
                g.objs[ob].temp1 = 0;
                g.objs[ob].temp2 = 0;
                let st = g.objs[ob].state;
                let next: StateRef = if same(st, &S_KEENAIRSHOOT1) {
                    &S_KEENSHOOT1
                } else if same(st, &S_KEENAIRSHOOTUP1) {
                    &S_KEENSHOOTUP1
                } else if g.c.xaxis == 0 {
                    &S_KEENSTAND
                } else {
                    &S_KEENWALK1
                };
                g.change_state(ob, next);
                g.play_sound(SND_LAND);
            }
        }
    }
    g.place_sprite(ob);
}

/// KeenPogoReact (0x1c021, K4 0x1cde0 without special tiles and conveyor
/// belts; K5 0x1cafb: pogoing onto a fuse hard enough breaks it).
pub fn keen_pogo_react(g: &mut Game, ob: usize) {
    air_ceiling(g, ob);
    let hn = g.objs[ob].hitnorth;
    if hn != 0 {
        g.objs[ob].ymove = 0;
        if (hn & !7) == 8 {
            g.kill_keen();
        } else {
            ep6! {
                if hn == 0x21 {
                    g.hit_special_tile(ob, true);
                } else if hn == 0x29 {
                    let o = &mut g.objs[ob];
                    o.xspeed = (o.xspeed + 8).min(8);
                } else if hn == 0x31 {
                    let o = &mut g.objs[ob];
                    o.xspeed = (o.xspeed - 8).max(-8);
                }
            }
            ep5! {
                if hn == 0x39 {
                    if g.objs[ob].yspeed > 0x2f {
                        let (tm, tb) = (g.objs[ob].tilemidx, g.objs[ob].tilebottom);
                        g.break_fuse(tm, tb);
                        g.place_sprite(ob);
                        return;
                    }
                    g.play_sound(SND_FUSE);
                }
            }
            if hn != 25 || g.jumptime == 0 {
                g.objs[ob].yspeed = -48;
                g.jumptime = 24;
                g.play_sound(SND_PLAYERBOUNCE);
                g.change_state(ob, &S_KEENPOGO);
            }
        }
    }
    g.place_sprite(ob);
}

ep5! {
    /// sound of Keen landing on a fuse
    pub const SND_FUSE: i32 = 0x39;
    /// DS:0x130a the broken fuse (two tiles high)
    const BROKENFUSE: [u16; 2] = [1932, 1950];

    impl Game {
        /// BreakFuse (K5 0x1caa7): Keen destroyed a fuse of the QED machine.
        pub fn break_fuse(&mut self, x: i32, y: i32) {
            super::act1::spawn_fuse_flash(self, x, y);
            self.gs.numfuses -= 1;
            if self.gs.numfuses == 0 {
                super::act1::spawn_dead_machine(self);
            }
            self.rf_mem_to_map(&BROKENFUSE, 1, x, y, 1, 2);
        }
    }
}

impl Game {
    /// SpawnKeen (0x1a2f7, K4 0x1b023, K5 0x1aaf0)
    pub fn spawn_keen(&mut self, x: i32, y: i32, dir: i32) {
        let p = self.player;
        let o = &mut self.objs[p];
        o.obclass = KEENOBJ;
        o.active = AC_ALLWAYS;
        o.priority = 1;
        o.x = x << 8;
        o.y = (y << 8) - 0xf1;
        o.xdir = dir;
        o.ydir = 1;
        self.new_state(p, &S_KEENSTAND);
    }
}
