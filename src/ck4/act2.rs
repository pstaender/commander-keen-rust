//! CK4_ACT2 port: Wormouth, Thundercloud (+ lightning bolt), Berkeloid
//! (+ fireball), Inchworm and Foot, Bounder, Lick, platforms and dropping
//! platforms.
//! Translated from the decompiled code segment 0x1fce of KEEN4E.EXE
//! (linear 0x1fcef .. 0x20ecc).

use super::defs::*;
use super::phys::u16shr;
use super::states::*;
use super::Game;

// object classes of this module (Keen 4 values)
pub const WORMOUTHOBJ: i32 = 12;
pub const CLOUDOBJ: i32 = 13;
/// the Berkeloid and its fireballs
pub const BERKELOIDOBJ: i32 = 14;
pub const BOUNDEROBJ: i32 = 15;
pub const INCHWORMOBJ: i32 = 16;
pub const FOOTOBJ: i32 = 17;
pub const LICKOBJ: i32 = 18;
pub const PLATFORMOBJ: i32 = 20;
/// lightning bolt of the Thundercloud
pub const BOLTOBJ: i32 = 25;
/// stunned enemies in Keen 4 (set by StunObj; defs::STUNNEDOBJ is Keen 6)
pub const STUNNEDOBJ: i32 = 33;

// sounds (indices into AUDIO.CK4), named after their use here
const SND_WORMOUTHATTACK: i32 = 5;
const SND_BOUNCE: i32 = 16;
const SND_LICKATTACK: i32 = 32;
const SND_BERKELOIDTHROW: i32 = 33;
const SND_THUNDER: i32 = 39;
const SND_FIREBALLLAND: i32 = 40;
const SND_MAKEFOOT: i32 = 45;

/// first platform jet sprite (side jets 477/478, bottom left 481/482,
/// bottom right 479/480)
const PLATFORMSIDEJETSPR: i32 = 0x1dd;
const PLATFORMRJETSPR: i32 = 0x1df;
const PLATFORMLJETSPR: i32 = 0x1e1;

/// platform stop block in the info plane
const PLATFORMBLOCK: usize = 0x1f;

/// 16-bit signed wrap of a value
#[inline]
fn w16(v: i32) -> i32 {
    v as i16 as i32
}

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

// ===========================================================================
// Wormouth
// ===========================================================================

/// SpawnWormouth (0x1fcef)
pub fn spawn_wormouth(g: &mut Game, x: i32, y: i32) {
    let n = g.get_new_obj(false);
    {
        let o = &mut g.objs[n];
        o.obclass = WORMOUTHOBJ;
        o.active = AC_YES;
        o.priority = 0;
        o.x = x << 8;
        o.y = (y << 8) + 0x8f;
    }
    g.objs[n].xdir = rnd_dir(g);
    g.objs[n].ydir = 1;
    g.new_state(n, &S_WORMOUTH);
}

/// T_WormouthLookRight (0x1fd5c)
pub fn t_wormouth_look_right(g: &mut Game, ob: usize) {
    let p = g.player;
    if u(g.objs[ob].x) < u(g.objs[p].x) {
        g.objs[ob].xdir = 1;
        g.objs[ob].state = Some(&S_WORMOUTH);
    }
}

/// T_WormouthLook (0x1fd7c)
pub fn t_wormouth_look(g: &mut Game, ob: usize) {
    let p = g.player;
    g.objs[ob].xdir = if u(g.objs[p].x) > u(g.objs[ob].x) { 1 } else { -1 };
}

/// T_WormouthLookLeft (0x1fd9e)
pub fn t_wormouth_look_left(g: &mut Game, ob: usize) {
    let p = g.player;
    if u(g.objs[p].x) < u(g.objs[ob].x) {
        g.objs[ob].xdir = -1;
        g.objs[ob].state = Some(&S_WORMOUTH);
    }
}

/// T_Wormouth (0x1fdbe): crawls; peeks around now and then, bites Keen.
pub fn t_wormouth(g: &mut Game, ob: usize) {
    let p = g.player;
    let xdist = d16(g.objs[p].x, g.objs[ob].x);
    let ydist = d16(g.objs[p].bottom, g.objs[ob].bottom);
    if (xdist < -0x300 || xdist > 0x300) && g.rnd() < 6 {
        g.objs[ob].state = Some(&S_WORMOUTHPEEK1);
        return;
    }
    if ydist < -0x100 || ydist > 0x100 {
        return;
    }
    let xdir = g.objs[ob].xdir;
    if (xdir == 1 && xdist > 0x80 && xdist < 0x180) || (xdir == -1 && xdist < -0x80 && xdist > -0x200) {
        g.play_sound(SND_WORMOUTHATTACK);
        g.objs[ob].state = Some(&S_WORMOUTHBITE1);
    }
}

/// C_Wormouth (0x1fe41)
pub fn c_wormouth(g: &mut Game, ob: usize, hit: usize) {
    if g.objs[hit].obclass == STUNSHOTOBJ {
        g.stun_obj(ob, hit, &S_WORMOUTHSTUN);
    }
}

/// C_WormouthBite (0x1fe60): kills Keen unless he is behind the Wormouth.
pub fn c_wormouth_bite(g: &mut Game, ob: usize, hit: usize) {
    if g.objs[hit].obclass == KEENOBJ {
        let (o, h) = (&g.objs[ob], &g.objs[hit]);
        if (o.xdir == 1 && u(o.x) > u(h.x)) || (o.xdir == -1 && u(o.x) < u(h.x)) {
            return;
        }
        g.kill_keen();
    } else {
        c_wormouth(g, ob, hit);
    }
}

// ===========================================================================
// Thundercloud
// ===========================================================================

/// SpawnCloud (0x1fea0)
pub fn spawn_cloud(g: &mut Game, x: i32, y: i32) {
    let n = g.get_new_obj(false);
    let o = &mut g.objs[n];
    o.obclass = CLOUDOBJ;
    o.active = AC_YES;
    o.priority = 2;
    o.x = x << 8;
    o.y = y << 8;
    o.xdir = 1;
    o.ydir = 1;
    g.new_state(n, &S_CLOUDSLEEP);
}

/// T_Cloud (0x1feec): follows Keen, goes to S_CLOUDALIGN when above him.
pub fn t_cloud(g: &mut Game, ob: usize) {
    let p = g.player;
    if (g.rnd() as u16) < u(g.tics) {
        g.objs[ob].xdir = if u(g.objs[ob].x) > u(g.objs[p].x) { -1 } else { 1 };
    }
    let xdir = g.objs[ob].xdir;
    g.accelerate_x(ob, xdir, 10);
    let (o, pl) = (&g.objs[ob], &g.objs[p]);
    if u(pl.bottom) >= u(o.bottom)
        && d16(pl.top, o.bottom) <= 0x400
        && u(o.left) < u(pl.right)
        && u(o.right) > u(pl.left)
    {
        g.objs[ob].state = Some(&S_CLOUDALIGN);
    }
}

/// T_CloudAlign (0x1ff58): moves on to the next half tile, then attacks.
pub fn t_cloud_align(g: &mut Game, ob: usize) {
    let xdir = g.objs[ob].xdir;
    g.accelerate_x(ob, xdir, 10);
    let frac = g.objs[ob].x & 0x7f;
    if g.ph.xtry < 0 && w16(frac + g.ph.xtry) <= 0 {
        g.ph.xtry = -frac;
        g.objs[ob].state = Some(&S_CLOUDATTACK1);
    }
    if g.ph.xtry > 0 && u(frac + g.ph.xtry) >= 0x80 {
        g.ph.xtry = 0x80 - frac;
        g.objs[ob].state = Some(&S_CLOUDATTACK1);
    }
}

/// R_Cloud (0x1ffc1): turns around at walls.
pub fn r_cloud(g: &mut Game, ob: usize) {
    {
        let o = &mut g.objs[ob];
        if o.hitwest != 0 {
            o.xspeed = 0;
            o.xdir = -1;
        } else if o.hiteast != 0 {
            o.xspeed = 0;
            o.xdir = 1;
        }
    }
    g.place_sprite(ob);
}

/// T_CloudShoot (0x2000a): spawns the lightning bolt.
pub fn t_cloud_shoot(g: &mut Game, ob: usize) {
    let n = g.get_new_obj(true);
    {
        let (x, y) = (g.objs[ob].x + 0x100, g.objs[ob].y + 0x100);
        let o = &mut g.objs[n];
        o.obclass = BOLTOBJ;
        o.active = AC_REMOVABLE;
        o.needtoclip = CL_NOCLIP;
        o.x = w16(x);
        o.y = w16(y);
    }
    g.new_state(n, &S_BOLT1);
    g.play_sound(SND_THUNDER);
}

/// C_CloudSleep (0x2005e): Keen wakes the cloud up.
pub fn c_cloud_sleep(g: &mut Game, ob: usize, hit: usize) {
    if g.objs[hit].obclass == KEENOBJ {
        g.change_state(ob, &S_CLOUDWAKE);
    }
}

// ===========================================================================
// Berkeloid
// ===========================================================================

/// SpawnBerkeloid (0x2007c)
pub fn spawn_berkeloid(g: &mut Game, x: i32, y: i32) {
    let n = g.get_new_obj(false);
    {
        let o = &mut g.objs[n];
        o.obclass = BERKELOIDOBJ;
        o.active = AC_YES;
        o.priority = 2;
        o.x = x << 8;
        o.y = w16((y << 8) - 0x200);
    }
    g.objs[n].xdir = rnd_dir(g);
    g.objs[n].ydir = 1;
    // temp1 = float offset, temp2 = float speed
    g.objs[n].temp2 = 8;
    g.new_state(n, &S_BERKELOID1);
}

/// T_Berkeloid (0x200ee): turns towards Keen, throws fireballs.
pub fn t_berkeloid(g: &mut Game, ob: usize) {
    let p = g.player;
    if g.rnd() < 0x20 {
        g.objs[ob].xdir = if u(g.objs[p].x) < u(g.objs[ob].x) { -1 } else { 1 };
    }
    if g.rnd() < 8 {
        g.objs[ob].state = Some(&S_BERKELOIDGRAB1);
        return;
    }
    if g.rnd() > 0x40 {
        return;
    }
    let xdist = d16(g.objs[p].x, g.objs[ob].x);
    let ydist = d16(g.objs[p].y, g.objs[ob].y);
    if ydist < -0x100 || ydist > 0x100 {
        return;
    }
    let xdir = g.objs[ob].xdir;
    if (xdir == 1 && xdist > 0) || (xdir == -1 && xdist < 0) {
        g.objs[ob].state = Some(&S_BERKELOIDGRAB1);
        g.play_sound(SND_BERKELOIDTHROW);
    }
}

/// T_BerkeloidThrow (0x20181): spawns the fireball.
pub fn t_berkeloid_throw(g: &mut Game, ob: usize) {
    let n = g.get_new_obj(true);
    let (bx, by, bdir) = (g.objs[ob].x, g.objs[ob].y, g.objs[ob].xdir);
    {
        let o = &mut g.objs[n];
        o.active = AC_REMOVABLE;
        o.obclass = BERKELOIDOBJ;
        o.y = w16(by + 0x80);
        o.yspeed = -8;
        if bdir == 1 {
            o.xspeed = 48;
            o.x = w16(bx + 0x200);
            o.xdir = 1;
        } else {
            o.xspeed = -48;
            o.x = w16(bx - 0x100);
            o.xdir = -1;
        }
    }
    g.new_state(n, &S_FIREBALL1);
    g.objs[ob].needtoreact = true;
}

/// T_BerkeloidThrowDone (0x201f8)
pub fn t_berkeloid_throw_done(g: &mut Game, ob: usize) {
    g.objs[ob].nothink = 4;
    g.objs[ob].needtoreact = true;
}

/// C_Berkeloid (0x2020c): invulnerable; shots explode, Keen dies.
pub fn c_berkeloid(g: &mut Game, _ob: usize, hit: usize) {
    let hc = g.objs[hit].obclass;
    if hc == STUNSHOTOBJ {
        g.explode_shot(hit);
    } else if hc == KEENOBJ {
        g.kill_keen();
    }
}

/// R_Fireball (0x20237)
pub fn r_fireball(g: &mut Game, ob: usize) {
    if g.objs[ob].hiteast != 0 || g.objs[ob].hitwest != 0 {
        g.objs[ob].xspeed = 0;
    }
    if g.objs[ob].hitnorth != 0 {
        g.play_sound(SND_FIREBALLLAND);
        g.change_state(ob, &S_FIREBALLLAND1);
    }
    g.place_sprite(ob);
}

/// R_BerkeloidFloat (0x2028d): bobs up and down (temp1 offset, temp2 speed).
pub fn r_berkeloid_float(g: &mut Game, ob: usize) {
    let tics = g.tics;
    let o = &mut g.objs[ob];
    o.temp1 = w16(o.temp1 + w16(o.temp2 * tics));
    if o.temp1 > 0 {
        o.temp1 = 0;
        o.temp2 = -8;
    } else if o.temp1 < -0x100 {
        o.temp1 = -0x100;
        o.temp2 = 8;
    }
    let (x, y, shape) = (o.x, w16(o.y + o.temp1), o.shapenum);
    g.place_sprite_at(ob, x, y, shape, 0);
}

/// R_Berkeloid (0x202e8): R_Walk variant that pauses at turns.
pub fn r_berkeloid(g: &mut Game, ob: usize) {
    let (xdir, hitwest, hiteast, hitnorth) =
        (g.objs[ob].xdir, g.objs[ob].hitwest, g.objs[ob].hiteast, g.objs[ob].hitnorth);
    let turn = if xdir == 1 && hitwest != 0 {
        g.objs[ob].x = w16(g.objs[ob].x - g.objs[ob].xmove);
        g.objs[ob].xdir = -1;
        true
    } else if xdir == -1 && hiteast != 0 {
        g.objs[ob].x = w16(g.objs[ob].x - g.objs[ob].xmove);
        g.objs[ob].xdir = 1;
        true
    } else if hitnorth == 0 {
        g.objs[ob].x = w16(g.objs[ob].x - g.objs[ob].xmove * 2);
        g.objs[ob].xdir = -g.objs[ob].xdir;
        true
    } else {
        false
    };
    if turn {
        g.objs[ob].nothink = (g.rnd() as i32) >> 5;
        let st = g.objs[ob].st();
        g.change_state(ob, st);
    }
    r_berkeloid_float(g, ob);
}

// ===========================================================================
// Inchworm and Foot
// ===========================================================================

/// SpawnInchworm (0x20359)
pub fn spawn_inchworm(g: &mut Game, x: i32, y: i32) {
    let n = g.get_new_obj(false);
    {
        let o = &mut g.objs[n];
        o.obclass = INCHWORMOBJ;
        o.active = AC_YES;
        o.priority = 2;
        o.x = x << 8;
        o.y = y << 8;
    }
    g.objs[n].xdir = rnd_dir(g);
    g.objs[n].ydir = 1;
    g.new_state(n, &S_INCHWORM1);
    g.objs[n].ticcount = g.rnd() as i32 / 32;
}

/// SpawnFoot (0x203d5)
pub fn spawn_foot(g: &mut Game, x: i32, y: i32) {
    let n = g.get_new_obj(false);
    let o = &mut g.objs[n];
    o.obclass = FOOTOBJ;
    o.active = AC_YES;
    o.priority = 0;
    o.x = x << 8;
    o.y = w16((y - 3) << 8);
    g.new_state(n, &S_FOOT);
}

/// T_Inchworm (0x2041b): crawls towards Keen.
pub fn t_inchworm(g: &mut Game, ob: usize) {
    let p = g.player;
    g.objs[ob].xdir = if u(g.objs[ob].x) > u(g.objs[p].x) { -1 } else { 1 };
}

/// C_Inchworm (0x2043d): eleven inchworms touching in one tic become the foot.
pub fn c_inchworm(g: &mut Game, ob: usize, hit: usize) {
    if g.objs[hit].obclass != INCHWORMOBJ {
        return;
    }
    // temp1 = low word of the TimeCount of the counting, temp2 = count
    let now = w16(g.lasttimecount);
    if g.objs[ob].temp1 != now {
        g.objs[ob].temp1 = now;
        g.objs[ob].temp2 = 0;
    }
    g.objs[ob].temp2 += 1;
    if g.objs[ob].temp2 != 11 {
        return;
    }
    g.play_sound(SND_MAKEFOOT);
    g.objs[ob].y = w16(g.objs[ob].y - 0x500);
    g.objs[ob].obclass = FOOTOBJ;
    g.change_state(ob, &S_FOOT);
    let (x, y) = (g.objs[ob].x, g.objs[ob].y);
    for (dx, dy) in [(-0x80, 0x100), (0x100, 0x180), (0x280, 0x100), (0, -0x80)] {
        let n = g.get_new_obj(true);
        let o = &mut g.objs[n];
        o.x = w16(x + dx);
        o.y = w16(y + dy);
        o.priority = 3;
        g.new_state(n, &S_FOOTSMOKE1);
    }
    // remove all the other inchworms (RemoveObj keeps the next pointer)
    let mut i = g.objs[g.player].next;
    while let Some(n) = i {
        if g.objs[n].obclass == INCHWORMOBJ {
            g.remove_obj(n);
        }
        i = g.objs[n].next;
    }
}

/// C_Foot (0x20589): does nothing (KeenContact handles the foot).
pub fn c_foot(_g: &mut Game, _ob: usize, _hit: usize) {}

// ===========================================================================
// Bounder
// ===========================================================================

/// SpawnBounder (0x2059e)
pub fn spawn_bounder(g: &mut Game, x: i32, y: i32) {
    let n = g.get_new_obj(false);
    let o = &mut g.objs[n];
    o.obclass = BOUNDEROBJ;
    o.active = AC_YES;
    o.priority = 0;
    o.x = x << 8;
    o.y = w16((y << 8) - 0x80);
    o.ydir = 1;
    o.xdir = 0;
    g.new_state(n, &S_BOUNDERUP1);
}

/// C_Bounder (0x205ee)
pub fn c_bounder(g: &mut Game, ob: usize, hit: usize) {
    if g.objs[hit].obclass == STUNSHOTOBJ {
        {
            let o = &mut g.objs[ob];
            o.temp1 = 0;
            o.temp2 = 0;
            o.temp3 = 0;
            o.temp4 = o.obclass;
        }
        g.explode_shot(hit);
        g.change_state(ob, &S_BOUNDERSTUN);
        g.objs[ob].obclass = STUNNEDOBJ;
        g.objs[ob].yspeed -= 32;
    }
}

/// R_Bounder (0x20633): bounces; follows Keen while he rides it.
/// temp1 = moved sideways on the last bounce, temp2 = bounces since Keen left.
pub fn r_bounder(g: &mut Game, ob: usize) {
    if g.objs[ob].hitsouth != 0 {
        g.objs[ob].yspeed = 0;
    }
    if g.objs[ob].hitnorth != 0 {
        g.objs[ob].temp2 += 1;
        if g.on_screen(ob) {
            g.play_sound(SND_BOUNCE);
        }
        g.objs[ob].yspeed = -50;
        let setdir = if g.gs.riding == Some(ob) {
            g.objs[ob].temp2 = 0;
            let p = g.player;
            let (o, pl) = (&g.objs[ob], &g.objs[p]);
            g.objs[ob].xdir = if u(pl.left) < u(o.left - 0x40) {
                -1
            } else if u(pl.right) > u(o.right + 0x40) {
                1
            } else {
                0
            };
            true
        } else if g.objs[ob].temp2 > 2 && g.objs[ob].temp1 != 0 {
            g.objs[ob].temp1 = 0;
            let r = g.rnd();
            g.objs[ob].xdir = if r < 100 {
                -1
            } else if r < 200 {
                1
            } else {
                0
            };
            true
        } else {
            g.objs[ob].temp1 = 1;
            g.objs[ob].xspeed = 0;
            g.objs[ob].xdir = 0;
            g.change_state(ob, &S_BOUNDERUP1);
            false
        };
        if setdir {
            g.objs[ob].xspeed = w16(g.objs[ob].xdir * 24);
        }
        let st: StateRef = if g.objs[ob].xdir == 0 { &S_BOUNDERUP1 } else { &S_BOUNDERSIDE1 };
        g.change_state(ob, st);
    }
    {
        let o = &mut g.objs[ob];
        if o.hiteast != 0 || o.hitwest != 0 {
            o.xdir = -o.xdir;
            o.xspeed = -o.xspeed;
        }
    }
    g.place_sprite(ob);
}

// ===========================================================================
// Lick
// ===========================================================================

/// SpawnLick (0x20760)
pub fn spawn_lick(g: &mut Game, x: i32, y: i32) {
    let n = g.get_new_obj(false);
    {
        let o = &mut g.objs[n];
        o.obclass = LICKOBJ;
        o.active = AC_YES;
        o.priority = 2;
        o.x = x << 8;
        o.y = y << 8;
    }
    g.objs[n].xdir = rnd_dir(g);
    g.objs[n].ydir = 1;
    g.objs[n].nothink = g.rnd() as i32 / 64;
    g.new_state(n, &S_LICKJUMP2);
}

/// T_Lick (0x207dc): hops towards Keen, breathes fire when close.
pub fn t_lick(g: &mut Game, ob: usize) {
    let p = g.player;
    g.objs[ob].xdir = if u(g.objs[ob].x) > u(g.objs[p].x) { -1 } else { 1 };
    let xdist = d16(g.objs[p].x, g.objs[ob].x);
    let ydist = d16(g.objs[p].y, g.objs[ob].y);
    let xdir = g.objs[ob].xdir;
    if (-0x100..=0x100).contains(&ydist)
        && ((xdir == 1 && xdist > -0x20 && xdist < 0x180) || (xdir == -1 && xdist < 0x20 && xdist > -0x200))
    {
        g.play_sound(SND_LICKATTACK);
        g.objs[ob].state = Some(&S_LICKATTACK1);
        return;
    }
    let o = &mut g.objs[ob];
    if w16(xdist.abs()) > 0x300 {
        o.xspeed = w16(o.xdir << 5);
        o.yspeed = -32;
    } else {
        o.xspeed = w16(o.xdir << 5) / 2;
        o.yspeed = -16;
    }
}

/// C_Lick (0x2088d)
pub fn c_lick(g: &mut Game, ob: usize, hit: usize) {
    if g.objs[hit].obclass == STUNSHOTOBJ {
        g.stun_obj(ob, hit, &S_LICKSTUN);
        g.objs[ob].yspeed -= 16;
    }
}

/// C_LickAttack (0x208b3): the flame kills Keen in front of the Lick.
pub fn c_lick_attack(g: &mut Game, ob: usize, hit: usize) {
    if g.objs[hit].obclass == KEENOBJ {
        let p = g.player;
        let (o, pl) = (&g.objs[ob], &g.objs[p]);
        if (o.xdir == 1 && u(pl.x) > u(o.x)) || (o.xdir == -1 && u(pl.x) < u(o.x)) {
            g.kill_keen();
        }
    } else {
        c_lick(g, ob, hit);
    }
}

/// R_LickJump (0x208fb)
pub fn r_lick_jump(g: &mut Game, ob: usize) {
    if g.objs[ob].hitnorth != 0 {
        g.change_state(ob, &S_LICKLAND);
    }
    g.place_sprite(ob);
}

// ===========================================================================
// Platforms
// ===========================================================================

/// SpawnPlatform (0x20935): dir 0 up, 1 right, 2 down, 3 left.
pub fn spawn_platform(g: &mut Game, x: i32, y: i32, dir: i32) {
    let n = g.get_new_obj(false);
    {
        let o = &mut g.objs[n];
        o.obclass = PLATFORMOBJ;
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
    g.new_state(n, &S_PLATFORM);
}

/// T_Platform (0x209cd): moves until it hits a platform block, then turns.
pub fn t_platform(g: &mut Game, ob: usize) {
    let tics = g.tics;
    let (xdir, ydir) = (g.objs[ob].xdir, g.objs[ob].ydir);
    g.ph.xtry = w16(w16(xdir * 12) * tics);
    g.ph.ytry = w16(w16(ydir * 12) * tics);
    let o = g.objs[ob].clone();
    if xdir == 1 {
        let v = w16(o.right + g.ph.xtry);
        let t = u16shr(v, 8);
        if o.tileright != t && g.map_tile(2, t, o.tiletop) == PLATFORMBLOCK {
            g.objs[ob].xdir = -1;
            g.ph.xtry = w16(g.ph.xtry - (v & 0xff));
        }
    } else if xdir == -1 {
        let v = w16(o.left + g.ph.xtry);
        let t = u16shr(v, 8);
        if o.tileleft != t && g.map_tile(2, t, o.tiletop) == PLATFORMBLOCK {
            g.objs[ob].xdir = 1;
            g.ph.xtry = w16(g.ph.xtry + (0x100 - (v & 0xff)));
        }
    } else if ydir == 1 {
        let v = w16(o.bottom + g.ph.ytry);
        let t = u16shr(v, 8);
        if o.tilebottom != t && g.map_tile(2, o.tileleft, t) == PLATFORMBLOCK {
            if g.map_tile(2, o.tileleft, t - 2) == PLATFORMBLOCK {
                // blocked at both ends: stay
                g.ph.ytry = 0;
                g.objs[ob].needtoreact = true;
            } else {
                g.objs[ob].ydir = -1;
                g.ph.ytry = w16(g.ph.ytry - (v & 0xff));
            }
        }
    } else if ydir == -1 {
        let v = w16(o.top + g.ph.ytry);
        let t = u16shr(v, 8);
        if o.tiletop != t && g.map_tile(2, o.tileleft, t) == PLATFORMBLOCK {
            if g.map_tile(2, o.tileleft, t + 2) == PLATFORMBLOCK {
                g.ph.ytry = 0;
                g.objs[ob].needtoreact = true;
            } else {
                g.objs[ob].ydir = 1;
                g.ph.ytry = w16(g.ph.ytry + (0x100 - (v & 0xff)));
            }
        }
    }
}

// The platform's jets are two extra sprites the original keeps in temp2
// (left/bottom-left jet) and temp3 (right/bottom-right jet); temp2/temp3 are
// nonzero while the sprite exists. Here jet 0 uses the stars slot of the
// object and jet 1 a slot behind the regular ones (the shared code has only
// one extra sprite per object).

fn jet_slot(g: &mut Game, ob: usize, jet: usize) -> usize {
    let n = MAXACTORS + 1;
    if jet == 0 {
        return ob;
    }
    if g.rf.stars.len() < 2 * n {
        g.rf.stars.resize(2 * n, Default::default());
    }
    n + ob
}

/// RF_PlaceSprite(&ob->temp2 / &ob->temp3, ...)
fn place_jet(g: &mut Game, ob: usize, jet: usize, x: i32, y: i32, shape: i32, priority: i32) {
    let slot = jet_slot(g, ob, jet);
    g.place_stars(slot, x, y, shape, priority);
    if jet == 0 {
        g.objs[ob].temp2 = 1;
    } else {
        g.objs[ob].temp3 = 1;
    }
}

/// RF_RemoveSprite(&ob->temp2 / &ob->temp3)
fn remove_jet(g: &mut Game, ob: usize, jet: usize) {
    let slot = jet_slot(g, ob, jet);
    g.remove_stars(slot);
    if jet == 0 {
        g.objs[ob].temp2 = 0;
    } else {
        g.objs[ob].temp3 = 0;
    }
}

/// R_Platform (0x20b9c): draws the platform and its flickering jets.
pub fn r_platform(g: &mut Game, ob: usize) {
    g.place_sprite(ob);
    let frame = (g.lasttimecount >> 2) & 1;
    let o = g.objs[ob].clone();
    if o.xdir == 1 {
        place_jet(g, ob, 0, w16(o.x - 0x10), w16(o.y + 0x30), frame + PLATFORMSIDEJETSPR, 0);
        if o.temp3 != 0 {
            remove_jet(g, ob, 1);
        }
    } else if o.xdir == -1 {
        if o.temp2 != 0 {
            remove_jet(g, ob, 0);
        }
        place_jet(g, ob, 1, w16(o.x + 0x300), w16(o.y + 0x50), frame + PLATFORMSIDEJETSPR, 1);
    } else if o.ydir == -1 {
        place_jet(g, ob, 0, w16(o.x + 0x20), w16(o.y + 0x90), frame + PLATFORMLJETSPR, 0);
        place_jet(g, ob, 1, w16(o.x + 0x2e0), w16(o.y + 0x80), frame + PLATFORMRJETSPR, 0);
    } else if o.ydir == 1 {
        if frame != 0 {
            place_jet(g, ob, 0, w16(o.x + 0x20), w16(o.y + 0x90), frame + PLATFORMLJETSPR, 0);
            place_jet(g, ob, 1, w16(o.x + 0x2e0), w16(o.y + 0x80), frame + PLATFORMRJETSPR, 0);
        } else {
            if o.temp2 != 0 {
                remove_jet(g, ob, 0);
            }
            if o.temp3 != 0 {
                remove_jet(g, ob, 1);
            }
        }
    }
}

// ===========================================================================
// Dropping platforms
// ===========================================================================

/// SpawnDropPlat (0x20d34)
pub fn spawn_drop_plat(g: &mut Game, x: i32, y: i32) {
    let n = g.get_new_obj(false);
    {
        let o = &mut g.objs[n];
        o.obclass = PLATFORMOBJ;
        o.active = AC_ALLWAYS;
        o.priority = 0;
        o.x = x << 8;
        o.temp1 = y << 8;
        o.y = o.temp1;
        o.xdir = 0;
        o.ydir = 1;
        o.needtoclip = CL_NOCLIP;
    }
    g.new_state(n, &S_DROPPLATSIT);
}

/// T_DropPlatSit (0x20d89): sinks a little while Keen stands on it.
pub fn t_drop_plat_sit(g: &mut Game, ob: usize) {
    if g.gs.riding != Some(ob) {
        return;
    }
    g.ph.ytry = w16(g.tics << 4);
    g.objs[ob].yspeed = 0;
    let o = &g.objs[ob];
    if u(o.y + g.ph.ytry - o.temp1) >= 0x80 {
        g.objs[ob].state = Some(&S_DROPPLATFALL);
    }
}

/// T_DropPlatFall (0x20dbc)
pub fn t_drop_plat_fall(g: &mut Game, ob: usize) {
    g.do_gravity(ob);
    if g.ph.ytry >= 0xf0 {
        g.ph.ytry = 0xf0;
    }
    let o = g.objs[ob].clone();
    let t = u16shr(o.bottom + g.ph.ytry, 8);
    if o.tilebottom != t && g.map_tile(2, o.tileleft, t) == PLATFORMBLOCK {
        g.ph.ytry = 0xff - (o.bottom & 0xff);
        if g.gs.riding != Some(ob) {
            g.objs[ob].state = Some(&S_DROPPLATRISE);
        }
    }
}

/// T_DropPlatRise (0x20e2e)
pub fn t_drop_plat_rise(g: &mut Game, ob: usize) {
    if g.gs.riding == Some(ob) {
        g.objs[ob].yspeed = 0;
        g.objs[ob].state = Some(&S_DROPPLATFALL);
    } else if u(g.objs[ob].y) <= u(g.objs[ob].temp1) {
        g.ph.ytry = w16(g.objs[ob].temp1 - g.objs[ob].y);
        g.objs[ob].state = Some(&S_DROPPLATSIT);
    }
}

// spawn functions by address (called from ScanInfoPlane)
pub use self::spawn_berkeloid as spawn_2007c;
pub use self::spawn_bounder as spawn_2059e;
pub use self::spawn_cloud as spawn_1fea0;
pub use self::spawn_drop_plat as spawn_20d34;
pub use self::spawn_foot as spawn_203d5;
pub use self::spawn_inchworm as spawn_20359;
pub use self::spawn_lick as spawn_20760;
pub use self::spawn_platform as spawn_20935;
pub use self::spawn_wormouth as spawn_1fcef;

#[derive(Default)]
pub struct ModState {}
