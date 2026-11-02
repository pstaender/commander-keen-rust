//! CK4 actors (act3): code segment 0x20e6 of KEEN4E.EXE (linear 0x20e60 ..
//! 0x21e27): treasure eater, mimrock, dopefish (+ bubbles, eaten objects),
//! schoolfish, sprite (+ its shot), mine, Princess Lindsey, dart guns
//! (+ darts) and the wetsuit.

use super::defs::*;
use super::states::*;
use super::Game;

// ---- object classes (Keen 4 values) ----------------------------------------

/// bonus item (owned by the item code; eaten by the treasure eater)
const BONUSOBJ: i32 = 4;
/// stunned enemy (Keen 4 value)
const STUNNEDOBJ: i32 = 33;
pub const MIMROCKOBJ: i32 = 19;
pub const DOPEFISHOBJ: i32 = 21;
pub const SCHOOLFISHOBJ: i32 = 22;
pub const SPRITEOBJ: i32 = 23;
pub const LINDSEYOBJ: i32 = 24;
pub const EATEROBJ: i32 = 26;
pub const WETSUITOBJ: i32 = 30;
/// enemy shot (sprite shot, darts)
pub const MSHOTOBJ: i32 = 31;
pub const MINEOBJ: i32 = 32;

// ---- sounds (indices into AUDIO.CK4), named after their use here -----------

const SND_KEENFIRE: i32 = 4;
const SND_MIMROCKBOUNCE: i32 = 15;
const SND_KEENEATEN: i32 = 23;
const SND_EATERCHOMP: i32 = 29;
const SND_EATERTELEPORT: i32 = 30;
const SND_MINEEXPLODE: i32 = 37;
const SND_SPRITESHOOT: i32 = 38;
const SND_DARTSHOOT: i32 = 41;
const SND_DOPEFISHBURP: i32 = 42;
const SND_GETWETSUIT: i32 = 45;

// ---- dart gun shapes (sprite chunks of the dart pointing in that direction) -

const DARTUPSPR: i32 = 0x174;
const DARTDOWNSPR: i32 = 0x176;
const DARTRIGHTSPR: i32 = 0x178;
const DARTLEFTSPR: i32 = 0x17a;

/// The blank tile word at DS 0x1314 that EaterInTile writes over eaten items.
const BLANKTILE: [u16; 1] = [0];

#[inline]
fn u(v: i32) -> u16 {
    v as u16
}

/// 16-bit signed difference `a - b` like the original int arithmetic.
#[inline]
fn d16(a: i32, b: i32) -> i32 {
    a.wrapping_sub(b) as i16 as i32
}

/// `abs()` of a 16-bit int (wraps for -32768 like the original).
#[inline]
fn abs16(v: i32) -> i32 {
    (v as i16).wrapping_abs() as i32
}

/// Random xdir used by the spawn functions: `US_RndT() < 0x80 ? 1 : -1`.
fn rnd_dir(g: &mut Game) -> i32 {
    if g.rnd() < 0x80 {
        1
    } else {
        -1
    }
}

/// Foreground tile at a linear map position (word offset), 0 outside the map.
fn fg_linear(g: &Game, idx: i32) -> usize {
    if idx < 0 {
        return 0;
    }
    g.mapplanes[1].get(idx as usize).copied().unwrap_or(0) as usize
}

/// Items the treasure eater goes for: INTILE 4 or 0x15..0x1c.
#[inline]
fn is_item_intile(v: i32) -> bool {
    v == 4 || (0x15..=0x1c).contains(&v)
}

/// Moves `ob` towards (dx, dy) at most 32 units per tic (xtry/ytry), returns
/// true when the target is reached this frame (T_DopeAttack / T_DopeReturn).
fn move_towards(g: &mut Game, dx: i32, dy: i32) -> bool {
    let step = (g.tics * 32) as i16 as i32;
    g.ph.xtry = if dx < 0 {
        if -step < dx {
            dx
        } else {
            -step
        }
    } else if dx < step {
        dx
    } else {
        step
    };
    g.ph.ytry = if dy < 0 {
        if -step < dy {
            dy
        } else {
            -step
        }
    } else if dy < step {
        dy
    } else {
        step
    };
    g.ph.xtry == dx && g.ph.ytry == dy
}

/// The inline stun of C_Eater / C_Mimrock: remember the class in temp4,
/// become a stunned object and hop up a little.
fn stun_inline(g: &mut Game, ob: usize, shot: usize, st: StateRef) {
    {
        let o = &mut g.objs[ob];
        o.temp1 = 0;
        o.temp2 = 0;
        o.temp3 = 0;
        o.temp4 = o.obclass;
        o.obclass = STUNNEDOBJ;
    }
    g.explode_shot(shot);
    g.change_state(ob, st);
    g.objs[ob].yspeed -= 16;
}

// ---- treasure eater -----------------------------------------------------------

/// SpawnEater (0x20e60)
pub fn spawn_eater(g: &mut Game, x: i32, y: i32) {
    let n = g.get_new_obj(false);
    {
        let o = &mut g.objs[n];
        o.obclass = EATEROBJ;
        o.active = AC_YES;
        o.priority = 3;
        o.x = x << 8;
        o.y = (y << 8) - 0x180;
    }
    g.objs[n].xdir = rnd_dir(g);
    g.objs[n].ydir = 1;
    g.new_state(n, &S_EATERSTAND1);
}
pub use self::spawn_eater as spawn_20e60;

/// T_EaterJump (0x20ecd): jump at nearby items, else hop along the floor;
/// after two turns at a ledge teleport away.
pub fn t_eater_jump(g: &mut Game, ob: usize) {
    g.objs[ob].state = Some(&S_EATERJUMP1);

    // jump after an item object close above
    let mut i = g.objs[g.player].next;
    while let Some(n) = i {
        let (o, b) = (&g.objs[ob], &g.objs[n]);
        if b.obclass == BONUSOBJ
            && b.active == AC_YES
            && u(b.right) > u(o.left)
            && u(b.left) < u(o.right)
            && u(b.bottom) < u(o.top)
            && u(b.bottom.wrapping_add(0x300)) > u(o.top)
        {
            g.objs[ob].xspeed = 0;
            g.objs[ob].yspeed = -48;
            return;
        }
        i = g.objs[n].next;
    }

    // jump after item tiles in the three rows above
    let mw = g.mapwidth;
    let (tl, tt, tr, tb, tmx, xdir, temp1) = {
        let o = &g.objs[ob];
        (o.tileleft, o.tiletop, o.tileright, o.tilebottom, o.tilemidx, o.xdir, o.temp1)
    };
    let width = tr - tl + 1;
    let mut idx = (tt - 3) * mw + tl;
    for _ in 0..3 {
        let mut x = 0;
        while (x as u16) < (width as u16) {
            if is_item_intile(g.intile(fg_linear(g, idx)) & 0xff) {
                g.objs[ob].xspeed = 0;
                g.objs[ob].yspeed = -48;
                return;
            }
            x += 1;
            idx += 1;
        }
        idx += mw - width;
    }

    if temp1 >= 2 {
        g.objs[ob].state = Some(&S_EATERTELEPORTOUT1);
        return;
    }

    // is there floor 4 tiles ahead?
    let mut idx = (tb - 2) * mw + tmx + xdir * 4;
    let mut floor = false;
    for _ in 0..4 {
        if g.north(fg_linear(g, idx)) != 0 {
            floor = true;
            break;
        }
        idx += mw;
    }
    if !floor {
        g.objs[ob].temp1 += 1;
        if g.objs[ob].temp1 == 2 {
            g.play_sound(SND_EATERTELEPORT);
            g.objs[ob].state = Some(&S_EATERTELEPORTOUT1);
            return;
        }
        g.objs[ob].xdir = -g.objs[ob].xdir;
    }
    let o = &mut g.objs[ob];
    o.xspeed = o.xdir * 20;
    o.yspeed = -24;
}

/// T_EaterTeleport (0x2105e): reappear at the first item object, or vanish
/// if there is none left.
pub fn t_eater_teleport(g: &mut Game, ob: usize) {
    g.objs[ob].temp1 = 0;
    let mut i = g.objs[g.player].next;
    while let Some(n) = i {
        if g.objs[n].obclass == BONUSOBJ {
            g.objs[ob].x = g.objs[n].x - 0x80;
            g.objs[ob].y = g.objs[n].y;
            g.new_state(ob, &S_EATERTELEPORTIN1);
            return;
        }
        i = g.objs[n].next;
    }
    g.remove_obj(ob);
}

/// C_Eater (0x210aa): eats item objects, stunned by shots.
pub fn c_eater(g: &mut Game, ob: usize, hit: usize) {
    if g.objs[hit].obclass == BONUSOBJ {
        g.objs[hit].obclass = INERTOBJ;
        g.objs[hit].priority = 3;
        g.change_state(hit, &S_EATENITEM1);
        g.play_sound(SND_EATERCHOMP);
    } else if g.objs[hit].obclass == STUNSHOTOBJ {
        stun_inline(g, ob, hit, &S_EATERSTUN);
    }
}

/// EaterInTile (0x21117): eats the item tiles the eater touches (one per
/// row), leaving a vanishing sparkle.
pub fn eater_in_tile(g: &mut Game, ob: usize) {
    let mw = g.mapwidth;
    let (tl, tt, tr, tb) = {
        let o = &g.objs[ob];
        (o.tileleft, o.tiletop, o.tileright, o.tilebottom)
    };
    let rowdiff = mw - (tr - tl + 1);
    let mut idx = tt * mw + tl;
    let mut y = tt;
    while u(tb) >= u(y) {
        let mut x = tl;
        while u(tr) >= u(x) {
            if is_item_intile(g.intile(fg_linear(g, idx)) & 0x7f) {
                g.rf_mem_to_map(&BLANKTILE, 1, x, y, 1, 1);
                let n = g.get_new_obj(true);
                {
                    let s = &mut g.objs[n];
                    s.obclass = INERTOBJ;
                    s.priority = 3;
                    s.needtoclip = CL_NOCLIP;
                    s.x = x << 8;
                    s.y = y << 8;
                    s.active = AC_REMOVABLE;
                }
                g.change_state(n, &S_EATENITEM1);
                // the pointer is not advanced for the rest of the row
                break;
            }
            x += 1;
            idx += 1;
        }
        y += 1;
        idx += rowdiff;
    }
}

/// R_EaterAir (0x2120d)
pub fn r_eater_air(g: &mut Game, ob: usize) {
    eater_in_tile(g, ob);
    if g.objs[ob].hitnorth != 0 {
        g.change_state(ob, &S_EATERSTAND1);
    }
    {
        let o = &mut g.objs[ob];
        if o.hiteast != 0 || o.hitwest != 0 {
            o.temp1 += 1;
            o.xdir = -o.xdir;
            o.xspeed = 0;
        }
        if o.hitnorth != 0 {
            o.yspeed = 0;
        }
    }
    g.place_sprite(ob);
}

// ---- mimrock --------------------------------------------------------------------

/// SpawnMimrock (0x21275)
pub fn spawn_mimrock(g: &mut Game, x: i32, y: i32) {
    let n = g.get_new_obj(false);
    {
        let o = &mut g.objs[n];
        o.obclass = MIMROCKOBJ;
        o.active = AC_YES;
        o.priority = 3;
        o.x = x << 8;
        o.y = (y << 8) - 0xd0;
        o.xdir = 1;
        o.ydir = 1;
    }
    g.new_state(n, &S_MIMROCKWAIT);
}
pub use self::spawn_mimrock as spawn_21275;

/// T_MimrockWait (0x212c4): sneak up on Keen while he looks away.
pub fn t_mimrock_wait(g: &mut Game, ob: usize) {
    let (o, p) = (&g.objs[ob], &g.objs[g.player]);
    if abs16(d16(o.bottom, p.bottom)) <= 0x500 && abs16(d16(o.x, p.x)) > 0x300 {
        if u(p.x) < u(o.x) {
            if p.xdir == -1 {
                g.objs[ob].xdir = -1;
                g.objs[ob].state = Some(&S_MIMROCKWALK1);
            }
        } else if p.xdir == 1 {
            g.objs[ob].xdir = 1;
            g.objs[ob].state = Some(&S_MIMROCKWALK1);
        }
    }
}

/// T_MimrockSneak (0x21324): jump at Keen when close, stop when he turns.
pub fn t_mimrock_sneak(g: &mut Game, ob: usize) {
    let (o, p) = (&g.objs[ob], &g.objs[g.player]);
    if abs16(d16(o.bottom, p.bottom)) <= 0x500 && o.xdir == p.xdir {
        if abs16(d16(o.x, p.x)) < 0x400 {
            let o = &mut g.objs[ob];
            o.xspeed = o.xdir * 20;
            o.yspeed = -40;
            g.ph.ytry = g.objs[ob].yspeed * g.tics;
            g.objs[ob].state = Some(&S_MIMROCKJUMP1);
        }
    } else {
        g.objs[ob].state = Some(&S_MIMROCKWAIT);
    }
}

/// C_Mimrock (0x21384): stunned by shots.
pub fn c_mimrock(g: &mut Game, ob: usize, hit: usize) {
    if g.objs[hit].obclass == STUNSHOTOBJ {
        stun_inline(g, ob, hit, &S_MIMROCKSTUN1);
    }
}

/// C_MimLethal (0x213c9): a jumping mimrock kills Keen.
pub fn c_mim_lethal(g: &mut Game, ob: usize, hit: usize) {
    if g.objs[hit].obclass == KEENOBJ {
        g.kill_keen();
    } else {
        c_mimrock(g, ob, hit);
    }
}

/// R_MimAir (0x213ea)
pub fn r_mim_air(g: &mut Game, ob: usize) {
    if g.objs[ob].hitnorth != 0 {
        g.play_sound(SND_MIMROCKBOUNCE);
        g.objs[ob].yspeed = -20;
        g.change_state(ob, &S_MIMROCKBOUNCE);
    }
    let o = &mut g.objs[ob];
    if o.hiteast != 0 || o.hitwest != 0 {
        o.xspeed = 0;
    }
    if o.hitnorth != 0 || o.hitsouth != 0 {
        o.yspeed = 0;
    }
    g.place_sprite(ob);
}

/// R_MimBounce (0x21456)
pub fn r_mim_bounce(g: &mut Game, ob: usize) {
    if g.objs[ob].hitnorth != 0 {
        g.play_sound(SND_MIMROCKBOUNCE);
        g.change_state(ob, &S_MIMROCKWAIT);
    }
    let o = &mut g.objs[ob];
    if o.hiteast != 0 || o.hitwest != 0 {
        o.xspeed = 0;
    }
    if o.hitnorth != 0 {
        o.yspeed = 0;
    }
    g.place_sprite(ob);
}

// ---- dopefish ---------------------------------------------------------------------

/// SpawnDopefish (0x214b7)
pub fn spawn_dopefish(g: &mut Game, x: i32, y: i32) {
    let n = g.get_new_obj(false);
    {
        let o = &mut g.objs[n];
        o.obclass = DOPEFISHOBJ;
        o.active = AC_YES;
        o.priority = 2;
        o.needtoclip = CL_FULLCLIP;
        o.x = x << 8;
        o.y = (y << 8) - 0x300;
    }
    g.objs[n].xdir = rnd_dir(g);
    g.objs[n].ydir = 1;
    g.new_state(n, &S_DOPEFISHSWIM1);
}
pub use self::spawn_dopefish as spawn_214b7;

/// T_EatPlayer (0x21529): Keen has been digested.
pub fn t_eat_player(g: &mut Game, ob: usize) {
    let _ = ob;
    g.playstate = EX_DIED;
}

/// T_DopeHunt (0x2153c): swim towards Keen (temp1 != 0: keep the xdir after
/// bumping into something). Also T_SchoolfishMove.
pub fn t_dope_hunt(g: &mut Game, ob: usize) {
    if g.objs[ob].temp1 == 0 {
        g.objs[ob].xdir = if u(g.objs[ob].x) < u(g.objs[g.player].x) { 1 } else { -1 };
    }
    let xdir = g.objs[ob].xdir;
    g.accelerate_xv(ob, xdir, 10);
    let ydir = if u(g.objs[ob].y) < u(g.objs[g.player].y) { 1 } else { -1 };
    g.accelerate_y(ob, ydir, 10);
}

/// T_DopeAttack (0x2159a): move so that the victim (temp4) is in the mouth,
/// then swallow it.
pub fn t_dope_attack(g: &mut Game, ob: usize) {
    let v = g.objs[ob].temp4 as usize;
    let dy = d16(g.objs[v].y - 0x100, g.objs[ob].y);
    let dx = if g.objs[ob].xdir == 1 {
        d16(g.objs[v].right + 0x20, g.objs[ob].right)
    } else {
        d16(g.objs[v].left - 0x20, g.objs[ob].left)
    };
    if move_towards(g, dx, dy) {
        if v == g.player {
            g.change_state(v, &S_KEENEATEN2);
        } else if let Some(nx) = g.objs[v].state.and_then(|s| s.next) {
            g.change_state(v, nx);
        } else {
            g.remove_obj(v);
        }
        g.objs[ob].state = Some(&S_DOPEFISHEAT);
    }
}

/// T_DopeReturn (0x21689): swim back to the attack start position (temp2,
/// temp3), then continue with the next state.
pub fn t_dope_return(g: &mut Game, ob: usize) {
    let dy = d16(g.objs[ob].temp3, g.objs[ob].y);
    let dx = d16(g.objs[ob].temp2, g.objs[ob].x);
    if move_towards(g, dx, dy) {
        let nx = g.objs[ob].st().next;
        g.objs[ob].state = nx;
        g.objs[ob].needtoclip = CL_FULLCLIP;
    }
}

/// T_Burp (0x21728): the dopefish burps out a big bubble.
pub fn t_burp(g: &mut Game, ob: usize) {
    let n = g.get_new_obj(true);
    let (ox, oy) = (g.objs[ob].x, g.objs[ob].y);
    {
        let s = &mut g.objs[n];
        s.x = ox + 0x380;
        s.y = oy + 0x200;
        s.obclass = INERTOBJ;
        s.priority = 3;
        s.active = AC_REMOVABLE;
        s.needtoclip = CL_NOCLIP;
        s.yspeed = -20;
        s.xspeed = 4;
    }
    g.new_state(n, &S_BUBBLE1);
    g.play_sound(SND_DOPEFISHBURP);
}

/// T_Bubble (0x2178b): bubbles rise and wobble, removed near the map top.
pub fn t_bubble(g: &mut Game, ob: usize) {
    g.ph.xtry = g.objs[ob].xspeed * g.tics;
    g.ph.ytry = g.objs[ob].yspeed * g.tics;
    if (g.rnd() as u16) < u(g.tics << 4) {
        g.objs[ob].xspeed = -g.objs[ob].xspeed;
    }
    if u(g.objs[ob].y) < 0x300 {
        g.remove_obj(ob);
    }
}

/// C_Dope (0x217c5): eat a schoolfish or Keen.
pub fn c_dope(g: &mut Game, ob: usize, hit: usize) {
    if g.objs[hit].obclass == SCHOOLFISHOBJ {
        g.change_state(hit, &S_SCHOOLFISHEATEN);
    } else if g.objs[hit].obclass == KEENOBJ && !g.godmode {
        g.objs[hit].obclass = INERTOBJ;
        g.objs[hit].needtoclip = CL_NOCLIP;
        g.play_sound(SND_KEENEATEN);
        g.change_state(hit, &S_KEENEATEN1);
    } else {
        return;
    }
    {
        let (x, y) = (g.objs[ob].x, g.objs[ob].y);
        let o = &mut g.objs[ob];
        o.temp2 = x;
        o.temp3 = y;
        o.temp4 = hit as i32;
    }
    g.objs[ob].xdir = if u(g.objs[hit].midx) < u(g.objs[ob].midx) { -1 } else { 1 };
    g.change_state(ob, &S_DOPEFISHATTACK);
    g.objs[ob].needtoclip = CL_NOCLIP;
}

/// R_Fish (0x21840): temp1 is set while the fish slides along a wall.
pub fn r_fish(g: &mut Game, ob: usize) {
    {
        let o = &mut g.objs[ob];
        if (o.hitsouth != 0 || o.hitnorth != 0) && o.temp1 == 0 {
            o.temp1 += 1;
        }
        if o.hiteast != 0 || o.hitwest != 0 {
            o.xspeed = 0;
            o.xdir = -o.xdir;
            o.temp1 = 1;
        }
        if o.hitsouth == 0 && o.hitnorth == 0 {
            o.temp1 = 0;
        }
    }
    g.place_sprite(ob);
}

// ---- schoolfish ---------------------------------------------------------------------

/// SpawnSchoolfish (0x218ab)
pub fn spawn_schoolfish(g: &mut Game, x: i32, y: i32) {
    let n = g.get_new_obj(false);
    {
        let o = &mut g.objs[n];
        o.obclass = SCHOOLFISHOBJ;
        o.active = AC_YES;
        o.needtoclip = CL_FULLCLIP;
        o.priority = 0;
        o.x = x << 8;
        o.y = y << 8;
        o.xdir = 1;
        o.ydir = 1;
    }
    g.new_state(n, &S_SCHOOLFISHSWIM1);
}
pub use self::spawn_schoolfish as spawn_218ab;

/// T_SchoolfishMove (0x218fc): same code as T_DopeHunt.
pub fn t_schoolfish_move(g: &mut Game, ob: usize) {
    t_dope_hunt(g, ob);
}

// ---- sprite (the little sprites that shoot at Keen) ---------------------------------

/// SpawnSprite (0x2195a)
pub fn spawn_sprite(g: &mut Game, x: i32, y: i32) {
    let n = g.get_new_obj(false);
    {
        let o = &mut g.objs[n];
        o.obclass = SPRITEOBJ;
        o.active = AC_YES;
        o.priority = 0;
        o.needtoclip = CL_NOCLIP;
        o.x = x << 8;
        o.temp1 = y << 8;
        o.y = y << 8;
        o.xdir = 1;
        o.ydir = 1;
    }
    g.new_state(n, &S_SPRITEFLOAT);
}
pub use self::spawn_sprite as spawn_2195a;

/// Float up and down around temp1 (T_Sprite / T_Lindsey).
fn float_around(g: &mut Game, ob: usize) {
    let ydir = g.objs[ob].ydir;
    g.accelerate_y(ob, ydir, 8);
    let o = &mut g.objs[ob];
    if d16(o.temp1, o.y) > 0x20 {
        o.ydir = 1;
    }
    if d16(o.y, o.temp1) > 0x20 {
        o.ydir = -1;
    }
}

/// Keen overlaps the object vertically (unsigned compares).
fn keen_at_height(g: &Game, ob: usize) -> bool {
    let (o, p) = (&g.objs[ob], &g.objs[g.player]);
    u(p.top) < u(o.bottom) && u(o.top) < u(p.bottom)
}

/// T_Sprite (0x219ae): float; turn to Keen when he is at the same height.
pub fn t_sprite(g: &mut Game, ob: usize) {
    float_around(g, ob);
    if keen_at_height(g, ob) {
        g.objs[ob].xdir = if u(g.objs[g.player].x) < u(g.objs[ob].x) { -1 } else { 1 };
        g.objs[ob].state = Some(&S_SPRITELOOK);
    }
}

/// T_SpriteLook (0x21a15): shoot if Keen is still at the same height.
pub fn t_sprite_look(g: &mut Game, ob: usize) {
    if keen_at_height(g, ob) {
        g.objs[ob].state = Some(&S_SPRITESHOOT1);
    }
}

/// T_SpriteShoot (0x21a38)
pub fn t_sprite_shoot(g: &mut Game, ob: usize) {
    let n = g.get_new_obj(true);
    let (ox, oy, oxdir) = (g.objs[ob].x, g.objs[ob].y, g.objs[ob].xdir);
    {
        let s = &mut g.objs[n];
        s.x = ox;
        s.y = oy + 0x80;
        s.priority = 0;
        s.obclass = MSHOTOBJ;
        s.active = AC_REMOVABLE;
    }
    g.play_sound(SND_KEENFIRE);
    g.objs[n].xdir = oxdir;
    g.new_state(n, &S_SPRITESHOT1);
    g.play_sound(SND_SPRITESHOOT);
}

/// R_Mshot (0x21a9e): sprite shots and darts vanish when they hit a wall.
pub fn r_mshot(g: &mut Game, ob: usize) {
    let hit = {
        let o = &g.objs[ob];
        o.hitnorth != 0 || o.hiteast != 0 || o.hitsouth != 0 || o.hitwest != 0
    };
    if hit {
        g.remove_obj(ob);
    } else {
        g.place_sprite(ob);
    }
}

// ---- mine -----------------------------------------------------------------------------

/// SpawnMine (0x21ae7): moves like a platform in direction `dir`
/// (0 up, 1 right, 2 down, 3 left).
pub fn spawn_mine(g: &mut Game, x: i32, y: i32, dir: i32) {
    let n = g.get_new_obj(false);
    {
        let o = &mut g.objs[n];
        o.obclass = MINEOBJ;
        o.active = AC_ALLWAYS;
        o.priority = 0;
        o.x = x << 8;
        o.y = y << 8;
        match dir as u16 {
            0 => {
                o.xdir = 0;
                o.ydir = -1;
            }
            1 => {
                o.xdir = 1;
                o.ydir = 0;
            }
            2 => {
                o.xdir = 0;
                o.ydir = 1;
            }
            3 => {
                o.xdir = -1;
                o.ydir = 0;
            }
            _ => {}
        }
    }
    g.new_state(n, &S_MINE);
}
pub use self::spawn_mine as spawn_21ae7;

/// C_Mine (0x21b7e)
pub fn c_mine(g: &mut Game, ob: usize, hit: usize) {
    if g.objs[hit].obclass == KEENOBJ {
        g.change_state(ob, &S_MINEBOOM1);
        g.play_sound(SND_MINEEXPLODE);
        g.kill_keen();
    }
}

// ---- Princess Lindsey -----------------------------------------------------------------

/// SpawnLindsey (0x21bac)
pub fn spawn_lindsey(g: &mut Game, x: i32, y: i32) {
    let n = g.get_new_obj(false);
    {
        let o = &mut g.objs[n];
        o.obclass = LINDSEYOBJ;
        o.active = AC_YES;
        o.priority = 0;
        o.x = x << 8;
        let yy = (y << 8) - 0x100;
        o.temp1 = yy;
        o.y = yy;
        o.ydir = 1;
    }
    g.new_state(n, &S_LINDSEY1);
}
pub use self::spawn_lindsey as spawn_21bac;

/// T_Lindsey (0x21bfa): float up and down.
pub fn t_lindsey(g: &mut Game, ob: usize) {
    float_around(g, ob);
}

// ---- dart guns ------------------------------------------------------------------------

/// SpawnDartGun (0x21c34): an invisible object that shoots a dart in
/// direction `dir` (0 up, 1 right, 2 down, 3 left) every 150 tics.
pub fn spawn_dart_gun(g: &mut Game, x: i32, y: i32, dir: i32) {
    let n = g.get_new_obj(false);
    {
        let o = &mut g.objs[n];
        o.obclass = INERTOBJ;
        o.active = AC_YES;
        o.x = x << 8;
        o.needtoclip = CL_NOCLIP;
        o.y = y << 8;
        o.temp1 = dir;
        match dir as u16 {
            0 => {
                o.y -= 0x30;
                o.x += 0x90;
                o.shapenum = DARTUPSPR;
            }
            1 => {
                o.x += 0x80;
                o.y += 0x50;
                o.shapenum = DARTRIGHTSPR;
            }
            2 => {
                o.x += 0x90;
                o.shapenum = DARTDOWNSPR;
            }
            3 => {
                o.y += 0x70;
                o.x -= 0x30;
                o.shapenum = DARTLEFTSPR;
            }
            _ => {}
        }
    }
    g.new_state(n, &S_DARTGUN);
}
pub use self::spawn_dart_gun as spawn_21c34;

/// T_DartShoot (0x21cde)
pub fn t_dart_shoot(g: &mut Game, ob: usize) {
    let n = g.get_new_obj(true);
    let (ox, oy, dir) = (g.objs[ob].x, g.objs[ob].y, g.objs[ob].temp1);
    {
        let s = &mut g.objs[n];
        s.x = ox;
        s.y = oy;
        s.obclass = MSHOTOBJ;
        s.active = AC_REMOVABLE;
    }
    let st: Option<StateRef> = match dir as u16 {
        0 => {
            g.objs[n].xdir = 0;
            g.objs[n].ydir = -1;
            Some(&S_DARTUP1)
        }
        1 => {
            g.objs[n].xdir = 1;
            g.objs[n].ydir = 0;
            Some(&S_DARTHORZ1)
        }
        2 => {
            g.objs[n].xdir = 0;
            g.objs[n].ydir = 1;
            Some(&S_DARTDOWN1)
        }
        3 => {
            g.objs[n].xdir = -1;
            g.objs[n].ydir = 0;
            Some(&S_DARTHORZ1)
        }
        _ => None,
    };
    if let Some(st) = st {
        g.new_state(n, st);
    }
    g.play_sound(SND_DARTSHOOT);
}

/// R_DartGun? (0x21d82): a plain R_Draw that no state record references.
#[allow(dead_code)]
pub fn r_draw_unused(g: &mut Game, ob: usize) {
    g.place_sprite(ob);
}

// ---- wetsuit ----------------------------------------------------------------------------

/// SpawnWetsuit (0x21da9)
pub fn spawn_wetsuit(g: &mut Game, x: i32, y: i32) {
    let n = g.get_new_obj(false);
    {
        let o = &mut g.objs[n];
        o.obclass = WETSUITOBJ;
        o.active = AC_YES;
        o.x = x << 8;
        o.y = (y << 8) - 0x100;
    }
    g.new_state(n, &S_WETSUIT);
}
pub use self::spawn_wetsuit as spawn_21da9;

/// C_Wetsuit (0x21dea): Keen standing on the wetsuit takes it and the level
/// is completed.
pub fn c_wetsuit(g: &mut Game, ob: usize, hit: usize) {
    let _ = ob;
    if g.objs[hit].obclass == KEENOBJ && g.objs[hit].hitnorth != 0 {
        // gamestate.wetsuit (K4 DS 0x77db)
        g.gs.wetsuit = 1;
        g.play_sound(SND_GETWETSUIT);
        // GotWetsuit (0x1e93a, matches K6 GotSandwich): "Cool! I can
        // breathe under water now!" window
        g.got_scuba();
        g.rf_force_refresh();
        g.playstate = EX_COMPLETED;
    }
}

/// Module-private state stored in `Game::act3` (none needed).
#[derive(Default, Clone)]
pub struct ModState {}
