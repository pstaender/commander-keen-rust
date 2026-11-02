//! CK6 actors, part 2: Nospike, Gik, cannon (+ its shots), Orbatrix, Bip
//! (the pilot that jumps out of a crashed Bipship), Bipship and Flect.
//! Translated from the decompiled code segment 0x200d of KEEN6C.EXE
//! (linear 0x200d0 .. 0x21078).

use super::defs::*;
use super::phys;
use super::states::*;
use super::Game;

// object classes not (yet) named in defs.rs
/// enemy shot (cannon shots, Bipship shots)
pub const MSHOTOBJ: i32 = 4;
/// the little Bip that walks away from a crashed Bipship
pub const BIPPILOTOBJ: i32 = 20;

// sounds (indices into AUDIO.CK6), named after their use here
const SND_BIPSHIPSHOOT: i32 = 4;
const SND_SQUISH: i32 = 22;
const SND_BIPSHIPCRASH: i32 = 24;
const SND_ENEMYSHOT: i32 = 29;
const SND_ENEMYSHOTEXPLODE: i32 = 30;
const SND_GIKJUMP: i32 = 36;
const SND_GIKLAND: i32 = 37;
const SND_ORBATRIXBUMP: i32 = 38;
const SND_REFLECT: i32 = 42;
const SND_NOSPIKECOLLIDE: i32 = 52;

/// Question mark sprite shown when a Nospike runs off a ledge.
const QUESTIONMARKSPR: i32 = 0x13c;

/// Gik slide friction tic masks by floor slope (DS 0x26a2 / 0x26b2).
const GIK_RTICMASK: [i64; 8] = [0, 7, 0, 0, 0, 3, 3, 1];
const GIK_LTICMASK: [i64; 8] = [0, 7, 3, 3, 1, 0, 0, 0];

#[inline]
fn u(v: i32) -> u16 {
    v as u16
}

/// 16-bit signed difference `a - b` like the original int arithmetic.
#[inline]
fn d16(a: i32, b: i32) -> i32 {
    a.wrapping_sub(b) as i16 as i32
}

/// Random xdir used by the spawn functions: `US_RndT() < 0x80 ? 1 : -1`.
fn rnd_dir(g: &mut Game) -> i32 {
    if g.rnd() < 0x80 {
        1
    } else {
        -1
    }
}

#[inline]
fn is(g: &Game, ob: usize, st: StateRef) -> bool {
    same(g.objs[ob].state, st)
}

// ===========================================================================
// Nospike
// ===========================================================================

/// SpawnNospike (0x200d0)
pub fn spawn_nospike(g: &mut Game, x: i32, y: i32) {
    let n = g.get_new_obj(false);
    {
        let o = &mut g.objs[n];
        o.obclass = NOSPIKEOBJ;
        o.active = AC_YES;
        o.priority = 0;
        o.x = x << 8;
        o.y = (y << 8) - 0x180;
    }
    g.objs[n].xdir = rnd_dir(g);
    g.objs[n].ydir = 1;
    g.new_state(n, &S_NOSPIKESIT);
    // temp4 = hit points
    g.objs[n].temp4 = 4;
}

/// T_NospikeWalk (0x20146)
pub fn t_nospike(g: &mut Game, ob: usize) {
    if g.rnd() < 0x10 {
        g.objs[ob].state = Some(&S_NOSPIKESIT);
        return;
    }
    let p = g.player;
    if g.objs[ob].bottom != g.objs[p].bottom || g.rnd() >= 0x21 {
        return;
    }
    g.objs[ob].xdir = if u(g.objs[ob].x) < u(g.objs[p].x) { 1 } else { -1 };
    g.objs[ob].temp1 = 0;
    g.objs[ob].temp2 = 1;
    let next: Option<StateRef> = if is(g, ob, &S_NOSPIKEWALK1) {
        Some(&S_NOSPIKECHARGE2)
    } else if is(g, ob, &S_NOSPIKEWALK2) {
        Some(&S_NOSPIKECHARGE3)
    } else if is(g, ob, &S_NOSPIKEWALK3) {
        Some(&S_NOSPIKECHARGE4)
    } else if is(g, ob, &S_NOSPIKEWALK4) {
        Some(&S_NOSPIKECHARGE1)
    } else {
        None
    };
    if next.is_some() {
        g.objs[ob].state = next;
    }
}

/// T_NospikeCharge (0x201cf)
pub fn t_nospike_charge(g: &mut Game, ob: usize) {
    if g.objs[ob].temp1 != 0 {
        return;
    }
    let p = g.player;
    let (o, pl) = (&g.objs[ob], &g.objs[p]);
    let still_facing = pl.bottom == o.bottom
        && !(o.xdir == -1 && u(o.x) < u(pl.x))
        && !(o.xdir == 1 && u(o.x) > u(pl.x));
    let keep_charging = (still_facing || g.rnd() >= 8) && g.on_screen(ob);
    if keep_charging {
        return;
    }
    g.objs[ob].temp2 = 0;
    let next: Option<StateRef> = if is(g, ob, &S_NOSPIKECHARGE1) {
        Some(&S_NOSPIKEWALK2)
    } else if is(g, ob, &S_NOSPIKECHARGE2) {
        Some(&S_NOSPIKEWALK3)
    } else if is(g, ob, &S_NOSPIKECHARGE3) {
        Some(&S_NOSPIKEWALK4)
    } else if is(g, ob, &S_NOSPIKECHARGE4) {
        Some(&S_NOSPIKEWALK1)
    } else {
        None
    };
    if next.is_some() {
        g.objs[ob].state = next;
    }
}

/// C_Nospike (0x2025c)
pub fn c_nospike(g: &mut Game, ob: usize, hit: usize) {
    let hc = g.objs[hit].obclass;
    if hc == KEENOBJ {
        g.kill_keen();
    } else if hc == STUNSHOTOBJ {
        g.objs[ob].temp4 -= 1;
        if g.objs[ob].temp4 == 0 {
            g.stun_obj(ob, hit, &S_NOSPIKESTUN);
            g.objs[ob].yspeed = -24;
            return;
        }
        let p = g.player;
        g.objs[ob].xdir = if u(g.objs[p].x) > u(g.objs[ob].x) { 1 } else { -1 };
        // high byte of temp2: tics of white flashing
        g.objs[ob].temp2 |= 0x400;
        g.objs[ob].needtoreact = true;
        let next: Option<StateRef> = if is(g, ob, &S_NOSPIKESIT) || is(g, ob, &S_NOSPIKEWALK1) {
            Some(&S_NOSPIKECHARGE2)
        } else if is(g, ob, &S_NOSPIKEWALK2) {
            Some(&S_NOSPIKECHARGE3)
        } else if is(g, ob, &S_NOSPIKEWALK3) {
            Some(&S_NOSPIKECHARGE4)
        } else if is(g, ob, &S_NOSPIKEWALK4) {
            Some(&S_NOSPIKECHARGE1)
        } else {
            None
        };
        if let Some(st) = next {
            g.change_state(ob, st);
        }
        g.explode_shot(hit);
    } else if hc == NOSPIKEOBJ {
        // two charging Nospikes running into each other stun each other
        if g.objs[hit].temp2 & 0xff == 0
            || g.objs[ob].temp2 & 0xff == 0
            || g.objs[hit].xdir == g.objs[ob].xdir
        {
            return;
        }
        for i in [hit, ob] {
            g.objs[i].temp1 = 0;
            g.objs[i].temp2 = 0;
            g.objs[i].temp3 = 0;
        }
        let cls = g.objs[ob].obclass;
        g.objs[hit].temp4 = cls;
        g.objs[ob].temp4 = cls;
        g.change_state(ob, &S_NOSPIKESTUN);
        g.change_state(hit, &S_NOSPIKESTUN);
        g.play_sound(SND_NOSPIKECOLLIDE);
        g.objs[hit].obclass = STUNNEDOBJ;
        g.objs[ob].obclass = STUNNEDOBJ;
        g.objs[hit].yspeed = -24;
        g.objs[ob].yspeed = -24;
    }
}

/// T_NospikeConfused (0x20372): removes the question mark (sprite at temp3).
pub fn t_nospike_confused(g: &mut Game, ob: usize) {
    g.remove_stars(ob);
}

/// R_NospikeConfused (0x20385): draws the Nospike and a question mark.
pub fn r_nospike_confused(g: &mut Game, ob: usize) {
    g.place_sprite(ob);
    let (x, y) = (g.objs[ob].x + 0x100, g.objs[ob].y - 0x80);
    g.place_stars(ob, x, y, QUESTIONMARKSPR, 3);
}

/// R_NospikeFall (0x203d3)
pub fn r_nospike_fall(g: &mut Game, ob: usize) {
    g.place_sprite(ob);
    if g.objs[ob].hitnorth != 0 {
        let o = &mut g.objs[ob];
        o.temp1 = 0;
        o.temp2 = 0;
        o.temp3 = 0;
        o.temp4 = o.obclass;
        g.change_state(ob, &S_NOSPIKESTUN);
        g.play_sound(SND_NOSPIKECOLLIDE);
        g.objs[ob].obclass = STUNNEDOBJ;
        g.objs[ob].yspeed = -24;
    }
}

/// R_NospikeCharge (0x20431)
pub fn r_nospike_charge(g: &mut Game, ob: usize) {
    if g.objs[ob].hitnorth == 0 {
        // temp1 counts the frames without ground below (ran off a ledge)
        g.objs[ob].temp1 += 1;
        if g.objs[ob].temp1 == 6 {
            g.change_state(ob, &S_NOSPIKECONFUSED1);
        }
    } else {
        g.objs[ob].temp1 = 0;
        if g.objs[ob].hiteast != 0 || g.objs[ob].hitwest != 0 {
            let o = &mut g.objs[ob];
            o.x = d16(o.x, o.xdir * 0x80) & 0xffff;
            g.new_state(ob, &S_NOSPIKESIT);
            g.place_sprite(ob);
            g.objs[ob].temp2 = 0;
            return;
        }
    }
    // drawn white (maskdraw) while the high byte of temp2 is non-zero
    if g.objs[ob].temp2 as u16 & 0xff00 != 0 {
        g.objs[ob].temp2 -= 0x100;
        g.place_sprite(ob);
        g.set_sprite_white(ob, true);
    } else {
        g.place_sprite(ob);
    }
}

// ===========================================================================
// Gik
// ===========================================================================

/// SpawnGik (0x204d6)
pub fn spawn_gik(g: &mut Game, x: i32, y: i32) {
    let n = g.get_new_obj(false);
    {
        let o = &mut g.objs[n];
        o.obclass = GIKOBJ;
        o.active = AC_YES;
        o.priority = 0;
        o.x = x << 8;
        o.y = y << 8;
    }
    g.objs[n].xdir = rnd_dir(g);
    g.objs[n].ydir = 1;
    g.new_state(n, &S_GIKWALK1);
}

/// T_GikWalk (0x20540): jumps at Keen when he is near.
pub fn t_gik(g: &mut Game, ob: usize) {
    if g.objs[ob].hitnorth == 9 {
        return;
    }
    let p = g.player;
    let (o, pl) = (&g.objs[ob], &g.objs[p]);
    let dx = d16(pl.x, o.x);
    if !(u(pl.bottom) <= u(o.bottom) && u(o.bottom.wrapping_sub(pl.bottom)) < 0x401) {
        return;
    }
    g.objs[ob].xdir = if dx < 0 { -1 } else { 1 };
    if dx > -0x701 && dx < 0x701 && (dx < -0xff || dx > 0xff) {
        g.objs[ob].xspeed = if dx < 0 { -40 } else { 40 };
        g.objs[ob].yspeed = -28;
        g.objs[ob].state = Some(&S_GIKJUMP);
        g.play_sound(SND_GIKJUMP);
    }
}

/// T_GikSlide (0x205be): slides with slope dependent friction.
pub fn t_gik_slide(g: &mut Game, ob: usize) {
    g.do_gravity(ob);
    let slope = (g.objs[ob].hitnorth & 7) as usize;
    let ticmask = if g.objs[ob].xdir == 1 { GIK_RTICMASK[slope] } else { GIK_LTICMASK[slope] };
    if g.objs[ob].xspeed == 0 && g.objs[ob].hitnorth != 0 {
        g.objs[ob].state = Some(&S_GIKSTAND);
        return;
    }
    let last = g.lasttimecount as i64;
    for i in (last - g.tics as i64)..last {
        if ticmask != 0 && (i & ticmask) == 0 {
            let o = &mut g.objs[ob];
            if o.xspeed < 0 {
                o.xspeed += 1;
                if o.xspeed == 0 {
                    o.state = Some(&S_GIKSTAND);
                    return;
                }
            }
            if o.xspeed > 0 {
                o.xspeed -= 1;
                if o.xspeed == 0 {
                    o.state = Some(&S_GIKSTAND);
                    return;
                }
            }
        }
        g.ph.xtry += g.objs[ob].xspeed;
    }
}

/// R_GikJump (0x20678)
pub fn r_gik_jump(g: &mut Game, ob: usize) {
    {
        let o = &mut g.objs[ob];
        if o.hiteast != 0 || o.hitwest != 0 {
            o.xspeed = 0;
        }
        if o.hitsouth != 0 {
            o.yspeed = 0;
        }
    }
    if g.objs[ob].hitnorth != 0 {
        g.objs[ob].yspeed = 0;
        g.play_sound(SND_GIKLAND);
        if let Some(next) = g.objs[ob].st().next {
            g.change_state(ob, next);
        }
    }
    g.place_sprite(ob);
}

/// R_GikSlide (0x206e0)
pub fn r_gik_slide(g: &mut Game, ob: usize) {
    let o = &mut g.objs[ob];
    if (o.hiteast != 0 && o.xspeed < 0) || (o.hitwest != 0 && o.xspeed > 0) {
        o.xspeed = 0;
    }
    g.place_sprite(ob);
}

// ===========================================================================
// Cannon (info values 53-56) and its shots
// ===========================================================================

/// SpawnCannon (0x20724): an invisible shooter; dir 0=up 1=right 2=down 3=left.
pub fn spawn_cannon(g: &mut Game, x: i32, y: i32, dir: i32) {
    let n = g.get_new_obj(false);
    {
        let o = &mut g.objs[n];
        o.obclass = CANNONOBJ;
        o.active = AC_YES;
        o.tileleft = x;
        o.tileright = x;
        o.tilebottom = y;
        o.tiletop = y;
        let gx = x << 8;
        o.right = gx;
        o.left = gx;
        o.x = gx;
        let gy = y << 8;
        o.bottom = gy;
        o.top = gy;
        o.y = gy;
        o.temp1 = dir;
    }
    g.new_state(n, &S_CANNON);
}

/// T_Cannon (0x2078c): fires a shot in the cannon's direction.
pub fn t_cannon(g: &mut Game, ob: usize) {
    let n = g.get_new_obj(true);
    let (x, y, dir) = (g.objs[ob].x, g.objs[ob].y, g.objs[ob].temp1);
    {
        let o = &mut g.objs[n];
        o.obclass = MSHOTOBJ;
        o.active = AC_YES;
        o.x = x;
        o.y = y;
        match dir {
            0 => o.yspeed = -64,
            1 => o.xspeed = 64,
            2 => o.yspeed = 64,
            3 => o.xspeed = -64,
            _ => {}
        }
    }
    g.new_state(n, &S_CSHOT1);
    g.play_sound(SND_ENEMYSHOT);
}

/// C_CannonShot (0x20816)
pub fn c_cshot(g: &mut Game, ob: usize, hit: usize) {
    if g.objs[hit].obclass == KEENOBJ {
        g.kill_keen();
        g.change_state(ob, &S_CSHOTHIT1);
    }
}

/// R_CannonShot (0x20839)
pub fn r_cshot(g: &mut Game, ob: usize) {
    let o = &g.objs[ob];
    if o.hitnorth != 0 || o.hiteast != 0 || o.hitsouth != 0 || o.hitwest != 0 {
        g.play_sound(SND_ENEMYSHOTEXPLODE);
        g.change_state(ob, &S_CSHOTHIT1);
    }
    g.place_sprite(ob);
}

// ===========================================================================
// Orbatrix
// ===========================================================================

/// SpawnOrbatrix (0x20890)
pub fn spawn_orbatrix(g: &mut Game, x: i32, y: i32) {
    let n = g.get_new_obj(false);
    {
        let o = &mut g.objs[n];
        o.obclass = ORBATRIXOBJ;
        o.active = AC_YES;
        o.priority = 0;
        o.x = x << 8;
        o.y = (y << 8) - 0x180;
    }
    g.objs[n].xdir = rnd_dir(g);
    g.objs[n].ydir = 1;
    // temp3 = hover offset, temp4 = hover direction
    g.objs[n].temp4 = 1;
    g.new_state(n, &S_ORBATRIXFLOAT1);
}

/// T_OrbatrixFly (0x20902)
pub fn t_orbatrix_float(g: &mut Game, ob: usize) {
    if g.rnd() < 0x20 {
        g.objs[ob].state = Some(&S_ORBATRIXSPIN1);
        return;
    }
    let p = g.player;
    if g.objs[ob].bottom != g.objs[p].bottom {
        return;
    }
    let dx = d16(g.objs[p].x, g.objs[ob].x);
    g.objs[ob].xdir = if dx < 0 { -1 } else { 1 };
    if dx > -0x500 && dx < 0x500 {
        g.objs[ob].state = Some(&S_ORBATRIXUNCURL1);
    }
}

/// C_Orbatrix (0x20957)
pub fn c_orbatrix(g: &mut Game, ob: usize, hit: usize) {
    if g.objs[hit].obclass == STUNSHOTOBJ {
        g.explode_shot(hit);
        g.change_state(ob, &S_ORBATRIXSPIN1);
    }
}

/// R_Orbatrix (0x2097d): R_Walk with a hovering offset (temp3).
pub fn r_orbatrix(g: &mut Game, ob: usize) {
    g.objs[ob].y -= g.objs[ob].temp3;
    phys::r_walk(g, ob);
    let tics = g.tics;
    let o = &mut g.objs[ob];
    o.y += o.temp3;
    o.temp3 += o.temp4 * tics * 4;
    if o.temp3 > 0x80 {
        o.temp3 = 0x80;
        o.temp4 = -1;
    } else if o.temp3 < -0x80 {
        o.temp3 = -0x80;
        o.temp4 = 1;
    }
}

/// R_OrbatrixBounce (0x209d1)
pub fn r_orbatrix_bounce(g: &mut Game, ob: usize) {
    g.place_sprite(ob);
    let o = &mut g.objs[ob];
    if o.hitnorth != 0 {
        o.yspeed = -o.yspeed;
    }
    if o.hitnorth != 0 || o.hitwest != 0 || o.hiteast != 0 {
        o.xspeed = -o.xspeed;
        g.play_sound(SND_ORBATRIXBUMP);
        if g.objs[ob].hitnorth != 0 {
            g.objs[ob].temp1 -= 1;
            if g.objs[ob].temp1 == 0 {
                g.change_state(ob, &S_ORBATRIXCURL);
                g.objs[ob].temp2 = 0x180;
            }
        }
    }
}

/// T_OrbatrixDrop (0x20a48): starts bouncing once hovering low enough.
pub fn t_orbatrix_drop(g: &mut Game, ob: usize) {
    let o = &mut g.objs[ob];
    if o.temp3 > 0xf {
        o.xspeed = o.xdir * 60;
        o.yspeed = -32;
        o.y -= o.temp3;
        // temp1 = number of floor bounces
        o.temp1 = 5;
        o.state = o.st().next;
    }
    o.needtoreact = true;
}

/// T_OrbatrixCurl (0x20a81): rises back to hovering height (temp2).
pub fn t_orbatrix_curl(g: &mut Game, ob: usize) {
    g.ph.ytry = g.tics * -8;
    let o = &mut g.objs[ob];
    o.temp2 += g.ph.ytry;
    if o.temp2 < 1 {
        g.ph.ytry -= o.temp2;
        o.state = o.st().next;
    }
}

/// C_OrbatrixBounce (0x20aaf)
pub fn c_orbatrix_bounce(g: &mut Game, ob: usize, hit: usize) {
    let hc = g.objs[hit].obclass;
    if hc == KEENOBJ {
        g.kill_keen();
    } else if hc == STUNSHOTOBJ {
        g.explode_shot(hit);
        g.objs[ob].xspeed = 0;
    }
}

// ===========================================================================
// Bip (states 0x2974.., spawned by a crashing Bipship)
// ===========================================================================

/// T_BipWalk (0x20adc)
pub fn t_bip_walk(g: &mut Game, ob: usize) {
    let p = g.player;
    if g.objs[ob].bottom == g.objs[p].bottom {
        let (o, pl) = (&g.objs[ob], &g.objs[p]);
        let mut xdir = o.xdir;
        if u(o.right) < u(pl.left - 0x40) {
            xdir = 1;
        }
        if u(pl.right + 0x40) < u(o.left) {
            xdir = -1;
        }
        g.objs[ob].xdir = xdir;
    } else if g.rnd() < 0x10 {
        g.objs[ob].xdir = -g.objs[ob].xdir;
        g.objs[ob].state = Some(&S_BIPSTAND);
    }
}

/// C_Bip (0x20b2f): squished when Keen lands on it.
pub fn c_bip(g: &mut Game, ob: usize, hit: usize) {
    if g.objs[hit].obclass == KEENOBJ && g.objs[hit].ymove > 0 {
        g.play_sound(SND_SQUISH);
        g.objs[ob].obclass = INERTOBJ;
        g.change_state(ob, &S_BIPSQUISHED);
    }
}

// ===========================================================================
// Bipship
// ===========================================================================

/// SpawnBipship (0x20b65)
pub fn spawn_bipship(g: &mut Game, x: i32, y: i32) {
    let n = g.get_new_obj(false);
    {
        let o = &mut g.objs[n];
        o.obclass = BIPSHIPOBJ;
        o.active = AC_YES;
        o.x = x << 8;
        o.y = (y << 8) - 0x180;
    }
    g.objs[n].xdir = rnd_dir(g);
    g.objs[n].xspeed = g.objs[n].xdir * 20;
    g.new_state(n, &S_BIPSHIP);
}

/// R_BipshipShot (0x20bd3)
pub fn r_bipship_shot(g: &mut Game, ob: usize) {
    let o = &g.objs[ob];
    if o.hitnorth == 0 && o.hitsouth == 0 && o.hiteast == 0 && o.hitwest == 0 {
        g.place_sprite(ob);
    } else {
        g.remove_obj(ob);
    }
}

/// T_BipshipTurn (0x20c1c)
pub fn t_bipship_turn(g: &mut Game, ob: usize) {
    let xdir = g.objs[ob].xdir;
    g.accelerate_x(ob, xdir, 20);
}

/// T_BipshipFly (0x20c36): flies, shoots at Keen and turns at walls / edges.
pub fn t_bipship_fly(g: &mut Game, ob: usize) {
    let xdir = g.objs[ob].xdir;
    g.accelerate_x(ob, xdir, 20);
    let mut dir = g.objs[ob].xdir;
    let p = g.player;
    if u(g.objs[p].bottom + 0x100 - g.objs[ob].bottom) < 0x201 {
        dir = if u(g.objs[p].x) < u(g.objs[ob].x) { -1 } else { 1 };
        if g.objs[ob].xdir == dir && (g.rnd() as u16) < u(g.tics << 2) {
            g.play_sound(SND_BIPSHIPSHOOT);
            let n = g.get_new_obj(true);
            let (ox, oy, oxdir) = (g.objs[ob].x, g.objs[ob].y, g.objs[ob].xdir);
            let s = &mut g.objs[n];
            s.obclass = MSHOTOBJ;
            s.active = AC_REMOVABLE;
            s.priority = 1;
            if oxdir == 1 {
                s.x = ox + 0x100;
                s.xspeed = 64;
            } else {
                s.x = ox;
                s.xspeed = -64;
            }
            s.y = oy + 0xa0;
            s.yspeed = 16;
            g.new_state(n, &S_BIPSHIPSHOT);
        }
    }
    // turn at walls in front or when there is no floor below the front
    let (tx, top, bottom) = (g.objs[ob].tilemidx + dir * 4, g.objs[ob].tiletop, g.objs[ob].tilebottom);
    let mut y = top;
    let mut blocked = false;
    while u(y) <= u(bottom) {
        let t = g.fg_tile(tx, y);
        if g.east(t) != 0 || g.west(t) != 0 {
            blocked = true;
            break;
        }
        y += 1;
    }
    if !blocked {
        let t = g.fg_tile(tx, y);
        blocked = g.north(t) == 0;
    }
    if blocked {
        dir = -dir;
    }
    if g.objs[ob].xdir != dir {
        g.objs[ob].xdir = dir;
        g.change_state(ob, &S_BIPSHIPTURN1);
    }
}

/// T_BipshipExplode (0x20dad): fire and the escaping Bip.
pub fn t_bipship_explode(g: &mut Game, ob: usize) {
    g.play_sound(SND_BIPSHIPCRASH);
    let (x, y) = (g.objs[ob].x, g.objs[ob].y);
    let n = g.get_new_obj(true);
    {
        let o = &mut g.objs[n];
        o.obclass = INERTOBJ;
        o.active = AC_YES;
        o.priority = 2;
        o.x = x;
        o.y = y - 0x180;
    }
    g.new_state(n, &S_BIPSHIPCRASH1);
    let n = g.get_new_obj(true);
    {
        let o = &mut g.objs[n];
        o.obclass = BIPPILOTOBJ;
        o.active = AC_YES;
        o.priority = 0;
        o.x = x;
        o.y = y - 0x80;
    }
    g.objs[n].xdir = rnd_dir(g);
    g.new_state(n, &S_BIPSTAND);
}

/// C_Bipship (0x20e58)
pub fn c_bipship(g: &mut Game, ob: usize, hit: usize) {
    if g.objs[hit].obclass == STUNSHOTOBJ {
        g.explode_shot(hit);
        g.change_state(ob, &S_BIPSHIPHIT);
    }
}

// ===========================================================================
// Flect
// ===========================================================================

/// SpawnFlect (0x20e7e)
pub fn spawn_flect(g: &mut Game, x: i32, y: i32) {
    let n = g.get_new_obj(false);
    {
        let o = &mut g.objs[n];
        o.obclass = FLECTOBJ;
        o.active = AC_YES;
        o.priority = 0;
        o.x = x << 8;
        o.y = (y << 8) - 0x100;
    }
    g.objs[n].xdir = rnd_dir(g);
    g.objs[n].ydir = 1;
    g.new_state(n, &S_FLECTWALK1);
}

/// T_FlectStand (0x20eeb): turns towards Keen.
pub fn t_flect_stand(g: &mut Game, ob: usize) {
    let p = g.player;
    let want = if u(g.objs[p].x) < u(g.objs[ob].x) { -1 } else { 1 };
    let o = &mut g.objs[ob];
    if o.xdir == want {
        o.state = Some(&S_FLECTWALK1);
    } else {
        o.state = Some(&S_FLECTTURN);
        o.xdir = want;
    }
}

/// T_FlectWalk (0x20f31)
pub fn t_flect_walk(g: &mut Game, ob: usize) {
    let p = g.player;
    let px = u(g.objs[p].x);
    {
        let o = &mut g.objs[ob];
        if px < u(o.x) && o.xdir == 1 {
            o.state = Some(&S_FLECTTURN);
            o.xdir = -1;
        }
        if u(o.x) < px && o.xdir == -1 {
            o.state = Some(&S_FLECTTURN);
            o.xdir = 1;
        }
    }
    if g.rnd() < 0x20 {
        g.objs[ob].state = Some(&S_FLECTSTAND);
    }
}

/// C_Flect (0x20f8e): blocks Keen, reflects shots that hit its front.
pub fn c_flect(g: &mut Game, ob: usize, hit: usize) {
    let hc = g.objs[hit].obclass;
    if hc == KEENOBJ {
        g.clip_to_sprite_side(hit, ob);
    } else if hc == STUNSHOTOBJ {
        if g.objs[hit].xdir == 0 {
            g.stun_obj(ob, hit, &S_FLECTSTUN);
        } else if g.objs[hit].xdir != g.objs[ob].xdir {
            g.objs[hit].xdir = g.objs[ob].xdir;
            g.objs[hit].temp4 = 1;
            g.play_sound(SND_REFLECT);
        }
    }
}

/// R_Flect (0x20fe7)
pub fn r_flect(g: &mut Game, ob: usize) {
    let o = &g.objs[ob];
    if o.xdir == 1 && o.hitwest != 0 {
        g.objs[ob].x -= g.objs[ob].xmove;
        g.objs[ob].xdir = -1;
        g.objs[ob].nothink = (g.rnd() >> 5) as i32;
    } else if o.xdir == -1 && o.hiteast != 0 {
        g.objs[ob].x -= g.objs[ob].xmove;
        g.objs[ob].xdir = 1;
        g.objs[ob].nothink = (g.rnd() >> 5) as i32;
    } else if o.hitnorth == 0 {
        g.objs[ob].x -= g.objs[ob].xmove;
        g.objs[ob].xdir = -g.objs[ob].xdir;
    } else {
        g.place_sprite(ob);
        return;
    }
    let st = g.objs[ob].st();
    g.change_state(ob, st);
    g.place_sprite(ob);
}

/// Module-private state stored in `Game::act2` (add fields as needed).
#[derive(Default, Clone)]
pub struct ModState {}
