//! CK_KEEN2.C port: score box, world map Keen, flags, stunner shots, doors.
//! Translated from the decompiled segment 0x1c20 of KEEN6C.EXE.

use super::defs::*;
use super::states::*;
use super::Game;

pub const SND_SHOTEXPLODE: i32 = 25;
pub const SND_TELEPORT: i32 = 41;
pub const SND_FLAGSPIN: i32 = 43;
pub const SND_FLAGLAND: i32 = 44;

pub const SCOREBOXSPR: i32 = ep_sel!(0xf5, 0xeb, 175);
pub const DEMOPLAQUESPR: i32 = ep_sel!(0x79, 0x6b, 51);

/// DS:0x1398 world map Keen base shapes per direction (dir_North..)
/// (K4 0x14ae, K5 0x146a)
pub const WORLDSHAPES: [i32; 8] = ep_sel!(
    [251, 266, 248, 257, 254, 260, 245, 263],
    [247, 262, 244, 253, 250, 256, 241, 259],
    [189, 204, 186, 195, 192, 198, 183, 201]
);
/// DS:0x13a8 walk animation offsets
pub const WORLDANIMS: [i32; 4] = [2, 3, 1, 3];
/// shape of the standing world map Keen
pub const WORLDKEENSPR: i32 = ep_sel!(0xf8, 0xf4, 0xba);

ep4! {
    /// DS:0x14c6 swimming world map Keen: base shapes per direction
    const WORLDSWIMSHAPES: [i32; 8] = [272, 280, 274, 282, 276, 284, 278, 286];
    /// DS:0x14d6 direction Keen is pushed into the water by the shore tile
    /// type (0x0b..0x0e)
    const SHOREDIR: [i32; 4] = [4, 6, 0, 2];
    /// DS:0x1336 opposite directions (leaving the water)
    const OPPOSITEDIR: [i32; 8] = [4, 5, 6, 7, 0, 1, 2, 3];
    const SND_SWIM1: i32 = 0x1a;
    const SND_SWIM2: i32 = 0x1b;
}

ep5! {
    /// sound of the world map elevators
    const SND_ELEVATOR: i32 = 0x3f;
}

/// Borland C rand(), used for purely visual randomness.
pub fn borland_rand(seed: &mut u32) -> i32 {
    *seed = seed.wrapping_mul(0x015a_4e35).wrapping_add(1);
    ((*seed >> 16) & 0x7fff) as i32
}

impl Game {
    /// Draws the score box (UpdateScore 0x1c34e, drawn as an overlay).
    pub fn draw_score_box(&mut self) {
        if !self.inlevel {
            return;
        }
        // drawn at the top left of the (possibly wide) play field
        let xoff = std::mem::take(&mut self.sys.surface.xoff);
        self.draw_score_box_at();
        self.sys.surface.xoff = xoff;
    }

    fn draw_score_box_at(&mut self) {
        let scale = self.sys.surface.scale as i32;
        if self.demomode != 0 {
            // DEMO plaque at origin + (0x800,0x80) global units
            self.draw_overlay_sprite(DEMOPLAQUESPR, 0x80, 0x08, scale);
            return;
        }
        if !self.showscorebox {
            return;
        }
        self.draw_overlay_sprite(SCOREBOXSPR, 4, 4, scale);
        let base_x = 4 + self.sprite_origin(SCOREBOXSPR).0;
        let base_y = 4 + self.sprite_origin(SCOREBOXSPR).1;
        // score: 9 digits at (8,4)
        let s = self.gs.score.to_string();
        self.draw_score_digits(&s, 9, base_x + 8, base_y + 4);
        // ammo: 2 digits at (64,20)
        let a = if self.gs.ammo < 100 { self.gs.ammo.to_string() } else { "99".into() };
        self.draw_score_digits(&a, 2, base_x + 64, base_y + 20);
        // lives: 2 digits at (24,20)
        let l = if self.gs.lives < 100 { self.gs.lives.to_string() } else { "99".into() };
        self.draw_score_digits(&l, 2, base_x + 24, base_y + 20);
    }

    fn sprite_origin(&self, shape: i32) -> (i32, i32) {
        let i = self.sprite_info(shape);
        (i.orgx as i32 >> 4, i.orgy as i32 >> 4)
    }

    fn draw_overlay_sprite(&mut self, shape: i32, x: i32, y: i32, scale: i32) {
        let idx = (shape - super::consts::STARTSPRITES as i32) as usize;
        let (ox, oy) = self.sprite_origin(shape);
        if let Some(t) = self.gfx.sprites.get(idx) {
            self.sys.surface.blit_phys(t, (x + ox) * scale, (y + oy) * scale);
        }
    }

    fn draw_score_digits(&mut self, s: &str, width: usize, x: i32, y: i32) {
        let mut cx = x;
        for _ in s.len()..width {
            self.draw_tile8(cx, y, 0x29);
            cx += 8;
        }
        for c in s.bytes() {
            self.draw_tile8(cx, y, c as i32 - 6);
            cx += 8;
        }
    }

    pub fn draw_tile8(&mut self, x: i32, y: i32, t: i32) {
        if let Some(tex) = self.gfx.tile8.get(t.max(0) as usize) {
            self.sys.surface.blit(tex, x, y);
        }
    }

    /// SpawnWorldKeen (0x1c701, K5 0x1d1be). Keen 4 (0x1d4de) after the
    /// foot ride (playstate EX_FOOT) flies to the other side of the map.
    pub fn spawn_world_keen(&mut self, tilex: i32, tiley: i32) {
        let p = self.player;
        ep4! {
            if self.playstate == EX_FOOT {
                let (wx, wy) = (self.gs.worldx, self.gs.worldy);
                let o = &mut self.objs[p];
                o.needtoclip = CL_NOCLIP;
                o.obclass = KEENOBJ;
                o.x = wx;
                o.y = wy;
                o.active = AC_ALLWAYS;
                o.priority = 3;
                o.xdir = 0;
                o.ydir = 0;
                if (wx as u16) < 0x1400 {
                    o.temp1 = 0x118;
                    o.xspeed = (0x1e00u16.wrapping_sub(o.x as u16) / 0x118) as i16 as i32 + 1;
                    o.yspeed = (0x3700u16.wrapping_sub(o.y as u16) / 0x118) as i16 as i32 + 1;
                } else {
                    o.temp1 = 0x8c;
                    o.xspeed = (0x1000i32 - o.x) as i16 as i32 / 0x8c + 1;
                    o.yspeed = (0x2f00i32 - o.y) as i16 as i32 / 0x8c + 1;
                }
                self.new_state(p, &S_KEENONFOOT1);
                return;
            }
        }
        self.objs[p].obclass = KEENOBJ;
        if self.gs.worldx == 0 {
            self.objs[p].x = tilex << 8;
            self.objs[p].y = tiley << 8;
        } else {
            self.objs[p].x = self.gs.worldx;
            self.objs[p].y = self.gs.worldy;
        }
        let o = &mut self.objs[p];
        o.active = AC_ALLWAYS;
        o.priority = 1;
        o.xdir = 0;
        o.ydir = 0;
        o.temp1 = 6;
        o.temp2 = 3;
        o.temp3 = 0;
        o.shapenum = WORLDKEENSPR;
        self.new_state(p, &S_WORLDKEEN);
    }

    /// CheckEnterLevel (0x1c772)
    fn check_enter_level(&mut self, ob: usize) {
        let (tl, tt, tr, tb) = {
            let o = &self.objs[ob];
            (o.tileleft, o.tiletop, o.tileright, o.tilebottom)
        };
        for y in tt..=tb {
            for x in tl..=tr {
                let info = self.map_tile(2, x, y) as u32;
                if info > 0xc000 && info < 0xc013 {
                    self.gs.worldx = self.objs[ob].x;
                    self.gs.worldy = self.objs[ob].y;
                    self.gs.mapon = (info - 0xc000) as i32;
                    self.playstate = EX_COMPLETED;
                    self.play_sound(super::keen::SND_ENTERLEVEL);
                }
            }
        }
    }

    /// Teleporter on the world map (0x1c8dd, K5 0x1d400: Keen animates
    /// while flying, other tiles); Keen 4 has none.
    #[allow(dead_code)]
    fn teleport(&mut self, tx: i32, ty: i32) {
        let p = self.player;
        self.play_sound(SND_TELEPORT);
        let gx = tx << 8;
        let gy = ty << 8;
        let mut time = 0;
        while time < 130 && self.alive() {
            self.rf_refresh();
            let step = self.tics * 2;
            time += self.tics;
            let o = &mut self.objs[p];
            if o.x as u16 == gx as u16 && o.y as u16 == gy as u16 {
                break;
            }
            if (o.y as u16) < gy as u16 {
                o.y += step;
                if (gy as u16) < o.y as u16 {
                    o.y = gy;
                }
            } else if (gy as u16) < o.y as u16 {
                o.y -= step;
                if (o.y as u16) < gy as u16 {
                    o.y = gy;
                }
            }
            if (o.x as u16) < gx as u16 {
                o.x += step;
                if (gx as u16) < o.x as u16 {
                    o.x = gx;
                }
            } else if (gx as u16) < o.x as u16 {
                o.x -= step;
                if (o.x as u16) < gx as u16 {
                    o.x = gx;
                }
            }
            ep5! {
                self.objs[p].shapenum = (((self.sys.time_count() as u32) >> 3) % 3) as i32 + 0xf8;
            }
            self.place_sprite(p);
            let t = ep_sel!(
                (((self.sys.time_count() >> 2) & 3) + 0xa35) as u16,
                (((self.sys.time_count() >> 2) & 1) + 0xa7f) as u16,
                (((self.sys.time_count() >> 2) & 3) + 0xa35) as u16
            );
            self.rf_mem_to_map(&[t], 1, tx, ty, 1, 1);
        }
        self.rf_mem_to_map(&[ep_sel!(0xa45, 0x427, 0xa45)], 1, tx, ty, 1, 1);
        let dest = self.map_tile(2, tx, ty) as i32;
        let o = &mut self.objs[p];
        o.x = dest & 0xff00;
        o.y = (dest & 0x7f) << 8;
        o.xdir = 0;
        o.ydir = 1;
        o.temp1 = 4;
        let st = o.st();
        self.new_state(p, st);
        self.center_actor(p);
        // activate the flags around the new position
        let mut i = self.objs[p].next;
        while let Some(n) = i {
            let o = &self.objs[n];
            if o.active == AC_NO
                && o.obclass == MAPFLAGOBJ
                && (self.cam.originxtile - 1) as u16 <= o.tileright as u16
                && o.tileleft as u16 <= (self.cam.originxtilemax + 1) as u16
                && o.tiletop as u16 <= (self.cam.originytilemax + 1) as u16
                && (self.cam.originytile - 1) as u16 <= o.tilebottom as u16
            {
                self.objs[n].needtoreact = true;
                self.objs[n].active = AC_YES;
                self.place_sprite(n);
            }
            i = self.objs[n].next;
        }
        self.rf_refresh();
        self.rf_refresh();
        self.play_sound(SND_TELEPORT);
        let mut time = 0;
        ep_sel!(self.objs[p].shapenum = 0xc2, (), self.objs[p].shapenum = 0xc2);
        while time < 90 && self.alive() {
            self.rf_refresh();
            time += self.tics;
            self.objs[p].y += self.tics * 3;
            ep5! {
                self.objs[p].shapenum = (((self.sys.time_count() as u32) >> 3) % 3) as i32 + 0xfb;
            }
            self.place_sprite(p);
            let t = ep_sel!(
                (((self.sys.time_count() >> 2) & 3) + 0xa35) as u16,
                (((self.sys.time_count() >> 2) & 1) + 0xa7f) as u16,
                (((self.sys.time_count() >> 2) & 3) + 0xa35) as u16
            );
            self.rf_mem_to_map(&[t], 1, tx, ty, 1, 1);
        }
        self.rf_mem_to_map(&[ep_sel!(0xa45, 0, 0xa45)], 1, tx, ty, 1, 1);
        self.ph.ytry = 0;
        self.ph.xtry = 0;
        self.clip_to_walls(p);
    }

    /// CheckWorldInTiles (0x1cba9): teleporters; Keen 5 (0x1db95) also
    /// elevators, Keen 4 (0x1d849) the shores of lakes (no teleporters).
    pub fn check_world_in_tiles(&mut self, ob: usize) {
        if self.objs[ob].temp3 != 0 {
            return;
        }
        let o = &self.objs[ob];
        let midy = ((o.top + ((o.bottom - o.top) as u16 >> 1) as i32) as u16 >> 8) as i32;
        let tm = o.tilemidx;
        let it = self.intile(self.fg_tile(tm, midy));
        ep4! {
            if !(0xb..=0xe).contains(&it) {
                return;
            }
            if self.gs.wetsuit == 0 {
                self.play_sound(super::keen::SND_NOWAY);
                self.cant_swim();
                self.rf_force_refresh();
                self.ph.xtry = -self.objs[ob].xmove;
                self.ph.ytry = -self.objs[ob].ymove;
                self.objs[ob].xdir = 0;
                self.objs[ob].ydir = 0;
                self.clip_to_walls(ob);
                return;
            }
            // walk into the water (or out of it) in the direction of the shore
            let swimming = same(self.objs[ob].state, &S_WORLDSWIM);
            let o = &mut self.objs[ob];
            o.temp1 = SHOREDIR[(it - 0xb) as usize];
            if swimming {
                o.temp1 = OPPOSITEDIR[o.temp1 as usize];
            }
            match o.temp1 {
                0 => {
                    o.xdir = 0;
                    o.ydir = -1;
                }
                2 => {
                    o.xdir = 1;
                    o.ydir = 0;
                }
                4 => {
                    o.xdir = 0;
                    o.ydir = 1;
                }
                6 => {
                    o.xdir = -1;
                    o.ydir = 0;
                }
                _ => {}
            }
            o.temp2 = 0;
            o.temp3 = 18;
            self.change_state(ob, if swimming { &S_WORLDKEENWALK } else { &S_WORLDSWIM });
        }
        ep5! {
            match it {
                20 => self.teleport(tm, midy),
                0x21 => self.elevator_enter(tm, midy, 0),
                0x22 => self.elevator_enter(tm, midy, -1),
                _ => {}
            }
        }
        ep6! {
            if it == 20 {
                self.teleport(tm, midy);
            }
        }
    }

    /// SpawnFlag: a flag planted on a completed level (0x1cc11, K4 0x1d984
    /// and K5 0x1dc37 without changing the map tile)
    pub fn spawn_flag(&mut self, tx: i32, ty: i32) {
        let n = self.get_new_obj(false);
        let o = &mut self.objs[n];
        o.needtoclip = CL_NOCLIP;
        o.priority = 3;
        o.obclass = MAPFLAGOBJ;
        o.active = AC_YES;
        o.x = tx * 256 + ep_sel!(0x60, -0x50, 0xe0);
        o.y = ty * 256 + ep_sel!(-0x1e0, -0x1e0, -0x1a0);
        ep6! {
            let t = self.fg_tile(tx, ty) as u16 + 1;
            self.rf_mem_to_map(&[t], 1, tx, ty, 1, 1);
        }
        let r = self.rnd() as i32;
        self.objs[n].ticcount = r / 16;
        self.new_state(n, &S_FLAGWAVE1);
    }

    /// SpawnThrowFlag: Keen throws a flag after finishing a level (0x1ccc3,
    /// K4 0x1d9e4). Keen 5 does not throw flags: the flag is just planted.
    pub fn spawn_throw_flag(&mut self, tx: i32, ty: i32) {
        ep5! {
            self.spawn_flag(tx, ty);
        }
        ep_sel!(self.spawn_throw_flag_k46(tx, ty), (), self.spawn_throw_flag_k46(tx, ty));
    }

    #[allow(dead_code)]
    fn spawn_throw_flag_k46(&mut self, tx: i32, ty: i32) {
        let n = self.get_new_obj(false);
        let (sx, sy) = (self.gs.worldx - 0x100, self.gs.worldy - 0x100);
        {
            let o = &mut self.objs[n];
            o.needtoclip = CL_NOCLIP;
            o.priority = 3;
            o.obclass = MAPFLAGOBJ;
            o.active = AC_ALLWAYS;
            o.x = sx;
            o.y = sy;
            o.temp1 = tx * 256 + ep_sel!(0x60, 0, 0xe0);
            o.temp2 = ty * 256 + ep_sel!(-0x260, 0, -0x220);
        }
        ep6! {
            self.flagtarget = (tx, ty);
        }
        let dx = self.objs[n].temp1 as i64 - sx as i64;
        let dy = self.objs[n].temp2 as i64 - sy as i64;
        for i in 0..30i64 {
            let px = sx as i64 + dx * i.min(24) / 24;
            let mut py = sy as i64 + dy * i / 30;
            let yoff = if i < 10 {
                i * 48
            } else if i < 15 {
                (i + 20) * 16
            } else if i < 20 {
                (20 - i + 30) * 16
            } else {
                (29 - i) * 48
            };
            py -= yoff;
            self.flagpath[i as usize] = (px as i32, py as i32);
        }
        ep_sel!(self.new_state(n, &S_THROWFLAG0), (), self.new_state(n, &S_THROWFLAG0));
    }

    /// SpawnShot (0x1cf04); in Keen 4 (0x1dbd8) and Keen 5 (0x1dc95) the
    /// new shot does not hit objects right away.
    pub fn spawn_shot(&mut self, x: i32, y: i32, dir: i32) {
        if self.gs.ammo == 0 {
            self.play_sound(super::keen::SND_USESWITCH);
            return;
        }
        self.gs.ammo -= 1;
        let n = self.get_new_obj(true);
        {
            let o = &mut self.objs[n];
            o.x = x;
            o.y = y;
            o.priority = 0;
            o.obclass = STUNSHOTOBJ;
            o.active = AC_ALLWAYS;
        }
        self.play_sound(super::keen::SND_KEENFIRE);
        let (xd, yd) = match dir {
            0 => (0, -1),
            2 => (1, 0),
            4 => (0, 1),
            6 => (-1, 0),
            _ => {
                self.quit("SpawnShot: Bad dir!");
                (0, 0)
            }
        };
        self.objs[n].xdir = xd;
        self.objs[n].ydir = yd;
        self.new_state(n, &S_STUNRAY1);
        if ep_sel!(true, true, false) {
            return;
        }
        // hit something right away?
        let mut i = self.objs[self.player].next;
        while let Some(h) = i {
            let s = &self.objs[n];
            let o = &self.objs[h];
            if o.active != AC_NO
                && (s.right as u16) > (o.left as u16)
                && (s.left as u16) < (o.right as u16)
                && (s.top as u16) < (o.bottom as u16)
                && (s.bottom as u16) > (o.top as u16)
            {
                if let Some(c) = o.state.and_then(|s| s.contact) {
                    c(self, h, n);
                    return;
                }
            }
            i = self.objs[h].next;
        }
    }

    /// ExplodeShot (0x1d026)
    pub fn explode_shot(&mut self, ob: usize) {
        self.objs[ob].obclass = INERTOBJ;
        self.change_state(ob, &S_STUNHIT);
        self.play_sound(SND_SHOTEXPLODE);
    }
}

/// T_WorldWait (0x1c7f2)
pub fn t_world_wait(g: &mut Game, ob: usize) {
    if g.c.dir.id() != DIR_NONE {
        g.objs[ob].state = Some(&S_WORLDKEENWALK);
        g.objs[ob].temp2 = 0;
        t_world_walk(g, ob);
    }
    if g.jumpbutton || g.pogobutton || g.firebutton {
        g.check_enter_level(ob);
    }
}

/// T_WorldWalk (0x1c831)
pub fn t_world_walk(g: &mut Game, ob: usize) {
    if g.objs[ob].temp3 == 0 {
        g.objs[ob].xdir = g.c.xaxis;
        g.objs[ob].ydir = g.c.yaxis;
        if g.pogobutton || g.firebutton || g.jumpbutton {
            g.check_enter_level(ob);
        }
        if g.c.dir.id() == DIR_NONE {
            g.objs[ob].state = Some(&S_WORLDKEEN);
            g.objs[ob].shapenum = WORLDSHAPES[g.objs[ob].temp1.clamp(0, 7) as usize] + 3;
            return;
        }
        g.objs[ob].temp1 = g.c.dir.id();
    } else {
        g.objs[ob].temp3 = (g.objs[ob].temp3 - 4).max(0);
    }
    let o = &mut g.objs[ob];
    o.temp2 += 1;
    if o.temp2 == 4 {
        o.temp2 = 0;
    }
    o.shapenum = WORLDSHAPES[o.temp1.clamp(0, 7) as usize] + WORLDANIMS[o.temp2 as usize];
    match o.temp2 {
        1 => g.play_sound(super::keen::SND_KEENWALK0),
        3 => g.play_sound(super::keen::SND_KEENWALK1),
        _ => {}
    }
}

/// T_FlagAlign (0x1ce16)
pub fn t_flag_align(g: &mut Game, ob: usize) {
    if !g.screenfaded {
        g.sd.wait_sound_done(&mut g.sys);
        g.play_sound(SND_FLAGSPIN);
        g.objs[ob].state = g.objs[ob].st().next;
    }
}

/// T_FlagFly (0x1ce40)
pub fn t_flag_fly(g: &mut Game, ob: usize) {
    let o = &mut g.objs[ob];
    o.temp3 += g.tics;
    if o.temp3 > 58 {
        o.temp3 = 58;
    }
    let (x, y) = g.flagpath[(o.temp3 / 2) as usize];
    o.x = x;
    o.y = y;
    o.needtoreact = true;
    if o.temp1 == 0 {
        g.play_sound(SND_FLAGSPIN);
    }
}

/// T_FlagLand (0x1cea1, K4 0x1dbb4 without changing the map tile)
pub fn t_flag_land(g: &mut Game, ob: usize) {
    let o = &mut g.objs[ob];
    o.x = o.temp1;
    o.y = o.temp2 + 0x80;
    g.play_sound(SND_FLAGLAND);
    ep6! {
        let (tx, ty) = g.flagtarget;
        let t = g.fg_tile(tx, ty) as u16 + 1;
        g.rf_mem_to_map(&[t], 1, tx, ty, 1, 1);
    }
}

/// T_Shot (0x1d04c): shots also hit inactive objects just off screen.
pub fn t_shot(g: &mut Game, ob: usize) {
    let o = &g.objs[ob];
    let c = &g.cam;
    let off = (o.tileright as u16) < c.originxtile as u16
        || (o.tilebottom as u16) < c.originytile as u16
        || (c.originxtilemax as u16) < o.tileleft as u16
        || (c.originytilemax as u16) < o.tiletop as u16;
    if !off {
        return;
    }
    if ((o.tileright + 10) as u16) < c.originxtile as u16
        || (c.originxtilemax as u16) < (o.tileleft - 10) as u16
        || ((o.tilebottom + 6) as u16) < c.originytile as u16
        || (c.originytilemax as u16) < (o.tiletop - 6) as u16
    {
        g.remove_obj(ob);
        return;
    }
    let mut i = g.objs[g.player].next;
    while let Some(h) = i {
        let s = &g.objs[ob];
        let o = &g.objs[h];
        if o.active == AC_NO
            && (o.left as u16) < (s.right as u16)
            && (s.left as u16) < (o.right as u16)
            && (s.top as u16) < (o.bottom as u16)
            && (o.top as u16) < (s.bottom as u16)
        {
            if let Some(cf) = o.state.and_then(|s| s.contact) {
                cf(g, h, ob);
                g.objs[h].needtoreact = true;
                g.objs[h].active = AC_YES;
            }
            if g.objs[ob].obclass == NOTHING {
                return;
            }
        }
        i = g.objs[h].next;
    }
}

/// R_Shot (0x1d114, K4 0x1dd90; Keen 5: r_shot_k5)
pub fn r_shot(g: &mut Game, ob: usize) {
    ep_sel!(r_shot_std(g, ob), r_shot_k5(g, ob), r_shot_std(g, ob))
}

#[allow(dead_code)]
fn r_shot_std(g: &mut Game, ob: usize) {
    let o = &g.objs[ob];
    if o.hitnorth == 17 || o.hitsouth == 17 {
        // pass through pole holes
        let d = o.st().ymove * g.tics * o.ydir;
        g.ph.ytry = d;
        let o = &mut g.objs[ob];
        o.y += d;
        o.top += d;
        o.bottom += d;
        o.tiletop = (o.top as u16 >> 8) as i32;
        o.tilebottom = (o.bottom as u16 >> 8) as i32;
    } else if o.hitnorth != 0 || o.hitsouth != 0 || o.hiteast != 0 || o.hitwest != 0 {
        g.explode_shot(ob);
    }
    g.place_sprite(ob);
}

ep5! {
    /// R_Shot (K5 0x1de52): shots fly through pole holes; a shot hitting
    /// the floor/ceiling next to a hole is moved onto the hole.
    fn r_shot_k5(g: &mut Game, ob: usize) {
        let (tl, tt, tr, tb) = {
            let o = &g.objs[ob];
            (o.tileleft, o.tiletop, o.tileright, o.tilebottom)
        };
        if g.objs[ob].hitnorth == 1 && tl != tr {
            if g.north(g.fg_tile(tr, tt - 1)) == 17 {
                let o = &mut g.objs[ob];
                o.hitnorth = 17;
                o.x += 0x100 - (o.x & 0xff);
            }
        } else if g.objs[ob].hitnorth == 17 && tl != tr {
            g.objs[ob].x &= 0xff00;
        }
        if g.objs[ob].hitsouth == 1 && tl != tr {
            if g.south(g.fg_tile(tr, tb + 1)) == 17 {
                let o = &mut g.objs[ob];
                o.hitsouth = 17;
                o.x += 0x100 - (o.x & 0xff);
            }
        } else if g.objs[ob].hitsouth == 17 && tl != tr {
            g.objs[ob].x &= 0xff00;
        }
        let o = &g.objs[ob];
        if o.hitnorth == 17 || o.hitsouth == 17 {
            let d = o.st().ymove * g.tics * o.ydir;
            g.ph.ytry = d;
            let o = &mut g.objs[ob];
            o.y += d;
            o.top += d;
            o.bottom += d;
            o.tiletop = (o.top as u16 >> 8) as i32;
            o.tilebottom = (o.bottom as u16 >> 8) as i32;
        } else if o.hitnorth != 0 || o.hitsouth != 0 || o.hiteast != 0 || o.hitwest != 0 {
            g.explode_shot(ob);
        }
        g.place_sprite(ob);
    }

    /// T_CardDoor (K5 0x1e023): the security door opens in three steps
    /// (4x4 tiles, each step 4 tiles back in the tile set).
    pub fn t_card_door(g: &mut Game, ob: usize) {
        let (x, y) = (g.objs[ob].x, g.objs[ob].y);
        let mut tiles = [0u16; 16];
        for yy in 0..4 {
            for xx in 0..4 {
                tiles[(yy * 4 + xx) as usize] = (g.fg_tile(x + xx, y + yy) as u16).wrapping_sub(4);
            }
        }
        g.rf_mem_to_map(&tiles, 1, x, y, 4, 4);
        g.objs[ob].temp1 += 1;
        if g.objs[ob].temp1 == 3 {
            g.objs[ob].state = None;
        }
    }

    impl Game {
        /// SpawnWorldKeenPort (K5 0x1d22f): world map Keen after a
        /// teleporter in a level (playstate EX_PORTOUT), at the spot given
        /// by the info plane.
        pub fn spawn_world_keen_port(&mut self, tilex: i32, tiley: i32) {
            let p = self.player;
            let o = &mut self.objs[p];
            o.obclass = KEENOBJ;
            o.x = tilex << 8;
            o.y = tiley << 8;
            o.active = AC_ALLWAYS;
            o.priority = 1;
            o.xdir = 0;
            o.ydir = 0;
            o.temp1 = 6;
            o.temp2 = 3;
            o.temp3 = 0;
            o.shapenum = 0xf4;
            self.new_state(p, &S_WORLDKEEN);
        }

        /// The elevator door animation (six frames of 2x2 tiles at the top
        /// left of the map) drawn at (x, y).
        fn elevator_door(&mut self, frame: i32, x: i32, y: i32) {
            let mut tiles = [0u16; 4];
            for r in 0..2 {
                for c in 0..2 {
                    tiles[(r * 2 + c) as usize] = self.fg_tile(frame * 2 + c, r) as u16;
                }
            }
            self.rf_mem_to_map(&tiles, 1, x, y, 2, 2);
            self.rf_refresh();
            self.sys.wait_vbl(8);
        }

        /// Hit box of an object from its shape (K5 0x1975f), without the
        /// tile coordinates.
        fn calc_hit_box(&mut self, ob: usize) {
            let info = self.sprite_info(self.objs[ob].shapenum);
            let o = &mut self.objs[ob];
            o.left = o.x + info.xl as i32;
            o.right = o.x + info.xh as i32;
            o.top = o.y + info.yl as i32;
            o.bottom = o.y + info.yh as i32;
            o.midx = o.left + ((o.right - o.left) as u16 >> 1) as i32;
        }

        /// ElevatorEnter (K5 0x1d959): Keen walks into a world map elevator
        /// (door at x + dx), the door closes and Keen rides to the floor
        /// given by the info plane (s_worldelevate).
        fn elevator_enter(&mut self, tx: i32, ty: i32, dx: i32) {
            let p = self.player;
            let gx = tx << 8;
            let gy = ty << 8;
            let mut time: i32 = 0;
            while (time as u16) < 0x82 && self.alive() {
                self.calc_hit_box(p);
                self.world_scroll_screen(p);
                self.rf_refresh();
                let step = self.tics * 2;
                time += self.tics;
                let o = &mut self.objs[p];
                if o.x as u16 == gx as u16 && o.y as u16 == gy as u16 {
                    break;
                }
                if (o.y as u16) < gy as u16 {
                    o.y += step;
                    if (gy as u16) < o.y as u16 {
                        o.y = gy;
                    }
                } else if (gy as u16) < o.y as u16 {
                    o.y -= step;
                    if (o.y as u16) < gy as u16 {
                        o.y = gy;
                    }
                }
                if (o.x as u16) < gx as u16 {
                    o.x += step;
                    if (gx as u16) < o.x as u16 {
                        o.x = gx;
                    }
                } else if (gx as u16) < o.x as u16 {
                    o.x -= step;
                    if (o.x as u16) < gx as u16 {
                        o.x = gx;
                    }
                }
                o.shapenum = ((time as u16 >> 3) % 3) as i32 + 0xf8;
                self.place_sprite(p);
            }
            self.play_sound(SND_ELEVATOR);
            for frame in (0..6).rev() {
                self.elevator_door(frame, tx + dx, ty - 1);
            }
            self.remove_sprite(p);
            let info = self.map_tile(2, tx, ty) as i32;
            let o = &mut self.objs[p];
            o.temp2 = info & 0xff00;
            o.temp1 = ((info & 0x7f) + 1) << 8;
            o.ydir = if (o.temp1 as u16) < o.y as u16 { -1 } else { 1 };
            o.xdir = if (o.temp2 as u16) < o.x as u16 { -1 } else { 1 };
            o.needtoclip = CL_NOCLIP;
            o.state = Some(&S_WORLDELEVATE);
        }
    }

    /// T_Elevate (K5 0x1d719): the invisible Keen moves to the target
    /// elevator (temp2, temp1), then the door opens and he walks out.
    pub fn t_elevate(g: &mut Game, ob: usize) {
        g.ph.ytry = g.objs[ob].ydir * 0x40 * g.tics;
        {
            let o = &g.objs[ob];
            if o.x != o.temp2 {
                g.ph.xtry = o.xdir * 12 * g.tics;
                let nx = (o.x + g.ph.xtry) as u16;
                if (o.xdir == 1 && (o.temp2 as u16) < nx) || (o.xdir == -1 && nx < o.temp2 as u16) {
                    g.ph.xtry = o.temp2 - o.x;
                }
            }
        }
        {
            let (xt, yt) = (g.ph.xtry, g.ph.ytry);
            let o = &mut g.objs[ob];
            o.left = o.x + xt;
            o.right = o.left + 0xff;
            o.top = o.y + yt;
            o.bottom = o.top + 0xff;
            let ny = (o.y + yt) as u16;
            if o.ydir == 1 {
                if ny < o.temp1 as u16 {
                    return;
                }
            } else if (o.temp1 as u16) < ny {
                return;
            }
        }
        // arrived
        g.ph.ytry = 0;
        g.ph.xtry = 0;
        {
            let o = &mut g.objs[ob];
            o.x = o.temp2;
            o.y = o.temp1;
            o.priority = 1;
            o.temp1 = 4;
            o.temp2 = 3;
            o.temp3 = 0;
        }
        let p = g.player;
        g.objs[p].xdir = 0;
        g.objs[p].ydir = 0;
        {
            let o = &mut g.objs[ob];
            o.state = Some(&S_WORLDKEEN);
            o.shapenum = 0xfd;
            o.needtoclip = CL_MIDCLIP;
        }
        let tx = (g.objs[ob].x as u16 >> 8) as i32;
        let ty = (g.objs[ob].y as u16 >> 8) as i32;
        g.world_scroll_screen(ob);
        g.rf_refresh();
        g.rf_refresh();
        g.objs[ob].y -= 0x100;
        g.place_sprite(ob);
        g.play_sound(SND_ELEVATOR);
        for frame in 0..6 {
            g.elevator_door(frame, tx, ty - 2);
        }
        for i in 0..0x20 {
            g.objs[ob].y += 8;
            g.objs[ob].shapenum = (i / 4) % 3 + 0xfb;
            g.place_sprite(ob);
            g.rf_refresh();
        }
        g.objs[ob].needtoclip = CL_MIDCLIP;
    }
}

ep4! {
    /// T_FootFly (K4 0x1d758): Keen rides the giant foot across the world
    /// map; temp1 counts down the flight time.
    pub fn t_foot_fly(g: &mut Game, ob: usize) {
        g.objs[ob].temp1 -= g.tics;
        g.ph.xtry = g.objs[ob].xspeed * g.tics;
        g.ph.ytry = g.objs[ob].yspeed * g.tics;
        if g.objs[ob].temp1 < 1 {
            let o = &mut g.objs[ob];
            g.ph.xtry -= o.xspeed * -o.temp1;
            g.ph.ytry -= o.yspeed * -o.temp1;
            o.priority = 1;
            o.temp1 = 6;
            o.temp2 = 3;
            o.temp3 = 0;
            let p = g.player;
            g.objs[p].xdir = 0;
            g.objs[p].ydir = 0;
            let o = &mut g.objs[ob];
            o.state = Some(&S_WORLDKEEN);
            o.shapenum = WORLDKEENSPR;
            o.needtoclip = CL_MIDCLIP;
        }
    }

    /// T_WorldSwim (K4 0x1d7d3): swimming world map Keen (with the wetsuit).
    pub fn t_world_swim(g: &mut Game, ob: usize) {
        if g.objs[ob].temp3 == 0 {
            g.objs[ob].xdir = g.c.xaxis;
            g.objs[ob].ydir = g.c.yaxis;
            if g.c.xaxis != 0 || g.c.yaxis != 0 {
                g.objs[ob].temp1 = g.c.dir.id();
            }
        } else {
            g.objs[ob].temp3 -= 6;
            if g.objs[ob].temp3 < 0 {
                g.objs[ob].temp3 = 0;
            }
        }
        let o = &mut g.objs[ob];
        o.shapenum = WORLDSWIMSHAPES[o.temp1.clamp(0, 7) as usize] + o.temp2;
        o.temp2 += 1;
        if o.temp2 == 2 {
            o.temp2 = 0;
        }
        let snd = if o.temp2 == 0 { SND_SWIM1 } else { SND_SWIM2 };
        g.play_sound(snd);
    }
}

/// DoorOpen (0x1d193): each step advances the door tiles one frame.
pub fn door_open(g: &mut Game, ob: usize) {
    let (x, y, h) = (g.objs[ob].x, g.objs[ob].y, g.objs[ob].temp1);
    let tiles: Vec<u16> = (0..h + 2).map(|i| g.fg_tile(x, y + i) as u16 + 1).collect();
    g.rf_mem_to_map(&tiles, 1, x, y, 1, h + 2);
}

impl Game {
    /// SpawnScore (0x1c203)
    pub fn spawn_score(&mut self) {
        let s = self.scoreobj;
        let o = &mut self.objs[s];
        o.obclass = INERTOBJ;
        o.priority = 3;
        o.active = AC_ALLWAYS;
        o.needtoclip = CL_NOCLIP;
        o.temp1 = -1;
        o.temp2 = -1;
        o.temp3 = -1;
        o.temp4 = -1;
        let st = if self.demomode == 0 { &S_SCORE } else { &S_DEMOSCORE };
        self.new_state(s, st);
    }
}
