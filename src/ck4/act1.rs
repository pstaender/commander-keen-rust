//! CK4_ACT1 port: Miragia (world map), bonus items, raindrop splash,
//! council member, poison slug, mad mushroom, eggs and eggbirds, arachnut
//! and skypest.
//! Translated from the decompiled segment 0x1efa (linear 0x1efce..0x1fd5b)
//! of KEEN4E.EXE.

use super::defs::*;
use super::states::*;
use super::Game;

// object classes (Keen 4 values)
/// bonus items (gems, points, extra life, stunner)
pub const BONUSOBJ: i32 = 4;
/// poison slug
pub const SLUGOBJ: i32 = 5;
/// council member (the "oracle" members Keen has to rescue)
pub const ORACLEOBJ: i32 = 6;
/// eggbird egg
pub const EGGOBJ: i32 = 8;
/// mad mushroom
pub const MADMUSHROOMOBJ: i32 = 9;
/// arachnut
pub const ARACHNUTOBJ: i32 = 10;
/// skypest
pub const SKYPESTOBJ: i32 = 11;
/// eggbird (hatched from an egg or placed directly)
pub const EGGBIRDOBJ: i32 = 27;

// sounds used in this module
/// slug leaves a slime puddle
const SND_SLUGPOO: i32 = 46;
/// big (third) bounce of a mad mushroom
const SND_BOUNCE2: i32 = 16;
/// normal bounce of a mad mushroom
const SND_BOUNCE1: i32 = 28;
/// skypest squashed by the pogo stick
const SND_SQUISH: i32 = 22;

/// DS:0x1e90 bonus item shapes (temp2 = first frame, temp3 = temp2 + 2)
const BONUSSHAPE: [i32; 12] = [234, 236, 238, 240, 219, 221, 223, 225, 227, 229, 231, 243];

/// 16-bit signed wrap of a value
#[inline]
fn w16(v: i32) -> i32 {
    v as i16 as i32
}

/// xdir/ydir towards a value: 1 if `a < b` (unsigned 16-bit compare), else -1
#[inline]
fn dir_to(a: i32, b: i32) -> i32 {
    if (a as u16) < (b as u16) {
        1
    } else {
        -1
    }
}

/// RF_MapToMap (0x26491): copies a block of all three map planes
/// (background, foreground, info) inside the map. The original also
/// removes / re-adds tile animations of the block; the Rust refresh manager
/// animates by tile number, so a plain copy is enough.
fn rf_map_to_map(g: &mut Game, srcx: i32, srcy: i32, destx: i32, desty: i32, width: i32, height: i32) {
    for y in 0..height {
        for x in 0..width {
            for plane in 0..3 {
                let t = g.map_tile(plane, srcx + x, srcy + y) as u16;
                g.set_map_tile(plane, destx + x, desty + y, t);
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Miragia (the world map city that fades in and out)
// ob->x / ob->y are tile coordinates of the 6x4 block, the frames are stored
// in the map at (2|8|14|20|26, 60).

/// shows one Miragia frame
fn miragia_frame(g: &mut Game, ob: usize, srcx: i32) {
    let (x, y) = (g.objs[ob].x, g.objs[ob].y);
    rf_map_to_map(g, srcx, 0x3c, x, y, 6, 4);
}

/// T_Miragia0 (0x1efce): starts to appear unless Keen stands in the block.
pub fn t_miragia0(g: &mut Game, ob: usize) {
    let p = &g.objs[g.player];
    let o = &g.objs[ob];
    if (p.tileright as u16) < (o.x as u16)
        || (p.tileleft as u16) > (w16(o.x + 5) as u16)
        || (p.tiletop as u16) < (o.y as u16)
        || (p.tilebottom as u16) > (w16(o.y + 4) as u16)
    {
        miragia_frame(g, ob, 8);
    } else {
        // don't appear on top of Keen
        g.objs[ob].state = Some(&S_MIRAGIA7);
    }
}

/// T_Miragia1 (0x1f027)
pub fn t_miragia1(g: &mut Game, ob: usize) {
    miragia_frame(g, ob, 14);
}

/// T_Miragia2 (0x1f04f)
pub fn t_miragia2(g: &mut Game, ob: usize) {
    miragia_frame(g, ob, 20);
}

/// T_Miragia3 (0x1f077)
pub fn t_miragia3(g: &mut Game, ob: usize) {
    miragia_frame(g, ob, 26);
}

/// T_Miragia4 (0x1f09f)
pub fn t_miragia4(g: &mut Game, ob: usize) {
    miragia_frame(g, ob, 20);
}

/// T_Miragia5 (0x1f0c7)
pub fn t_miragia5(g: &mut Game, ob: usize) {
    miragia_frame(g, ob, 14);
}

/// T_Miragia6 (0x1f0ef)
pub fn t_miragia6(g: &mut Game, ob: usize) {
    miragia_frame(g, ob, 8);
}

/// T_Miragia7 (0x1f117)
pub fn t_miragia7(g: &mut Game, ob: usize) {
    miragia_frame(g, ob, 2);
}

// ---------------------------------------------------------------------------
// bonus items

/// SpawnBonus (0x1f13f)
pub fn spawn_bonus(g: &mut Game, x: i32, y: i32, kind: i32) {
    let n = g.get_new_obj(false);
    let o = &mut g.objs[n];
    o.needtoclip = CL_NOCLIP;
    o.priority = 2;
    o.obclass = BONUSOBJ;
    o.x = w16(x << 8);
    o.y = w16(y << 8);
    o.ydir = -1;
    o.temp1 = kind;
    let shape = BONUSSHAPE.get(kind as usize).copied().unwrap_or(0);
    o.shapenum = shape;
    o.temp2 = shape;
    o.temp3 = o.temp2 + 2;
    g.new_state(n, &S_BONUS1);
}

/// SpawnSplash (0x1f1a9): the splash of a collected raindrop.
pub fn spawn_splash(g: &mut Game, x: i32, y: i32) {
    let n = g.get_new_obj(true);
    let o = &mut g.objs[n];
    o.needtoclip = CL_NOCLIP;
    o.priority = 3;
    o.obclass = INERTOBJ;
    o.x = w16(x << 8);
    o.y = w16(y << 8);
    g.new_state(n, &S_SPLASH1);
}

/// T_Bonus (0x1f1ed): animates the item.
pub fn t_bonus(g: &mut Game, ob: usize) {
    let o = &mut g.objs[ob];
    o.shapenum += 1;
    if o.shapenum == o.temp3 {
        o.shapenum = o.temp2;
    }
}

// ---------------------------------------------------------------------------
// council member

/// SpawnCouncil (0x1f208)
pub fn spawn_council(g: &mut Game, x: i32, y: i32) {
    let n = g.get_new_obj(false);
    {
        let o = &mut g.objs[n];
        o.obclass = ORACLEOBJ;
        o.active = AC_YES;
        o.priority = 0;
        o.x = w16(x << 8);
        o.y = w16((y << 8) - 0x171);
    }
    let xdir = if g.rnd() < 0x80 { 1 } else { -1 };
    g.objs[n].xdir = xdir;
    g.objs[n].ydir = 1;
    g.new_state(n, &S_COUNCILWALK1);
}

/// T_Council (0x1f275): stops now and then.
pub fn t_council(g: &mut Game, ob: usize) {
    let r = g.rnd() as u32;
    if r < ((g.tics * 8) as u16 as u32) {
        g.objs[ob].state = Some(&S_COUNCILSTAND);
    }
}

// ---------------------------------------------------------------------------
// poison slug

/// SpawnSlug (0x1f29e)
pub fn spawn_slug(g: &mut Game, x: i32, y: i32) {
    let n = g.get_new_obj(false);
    {
        let o = &mut g.objs[n];
        o.obclass = SLUGOBJ;
        o.active = AC_YES;
        o.priority = 2;
        o.x = w16(x << 8);
        o.y = w16((y << 8) - 0x71);
    }
    let xdir = if g.rnd() < 0x80 { 1 } else { -1 };
    g.objs[n].xdir = xdir;
    g.objs[n].ydir = 1;
    g.new_state(n, &S_SLUGWALK1);
}

/// T_Slug (0x1f30b): sometimes turns to Keen and leaves slime.
pub fn t_slug(g: &mut Game, ob: usize) {
    if g.rnd() < 0x10 {
        let px = g.objs[g.player].x;
        let o = &mut g.objs[ob];
        o.xdir = dir_to(o.x, px);
        o.state = Some(&S_SLUGPISS1);
        g.play_sound(SND_SLUGPOO);
    }
}

/// T_SlugPiss (0x1f347): spawns the slime puddle.
pub fn t_slug_piss(g: &mut Game, ob: usize) {
    let n = g.get_new_obj(true);
    let (x, bottom) = (g.objs[ob].x, g.objs[ob].bottom);
    {
        let o = &mut g.objs[n];
        o.obclass = INERTOBJ;
        o.active = AC_REMOVABLE;
        o.priority = 0;
        o.x = x;
        o.y = w16(bottom - 0x80);
    }
    g.new_state(n, &S_SLUGSLIME1);
}

/// C_Slug (0x1f38d)
pub fn c_slug(g: &mut Game, ob: usize, hit: usize) {
    match g.objs[hit].obclass {
        KEENOBJ => g.kill_keen(),
        STUNSHOTOBJ => {
            let st = if g.rnd() < 0x80 { &S_SLUGSTUN } else { &S_SLUGSTUNALT };
            g.stun_obj(ob, hit, st);
            let o = &mut g.objs[ob];
            o.yspeed = -24;
            o.xspeed = w16(o.xdir << 3);
        }
        _ => {}
    }
}

// ---------------------------------------------------------------------------
// mad mushroom

/// SpawnMadMushroom (0x1f3db)
pub fn spawn_mad_mushroom(g: &mut Game, x: i32, y: i32) {
    let n = g.get_new_obj(false);
    {
        let o = &mut g.objs[n];
        o.obclass = MADMUSHROOMOBJ;
        o.active = AC_YES;
        o.priority = 0;
        o.x = w16(x << 8);
        o.y = w16((y << 8) - 0xf1);
        o.xdir = 1;
        o.ydir = 1;
    }
    g.new_state(n, &S_MUSHROOM1);
}

/// T_Mushroom (0x1f42b): faces Keen and falls.
pub fn t_mushroom(g: &mut Game, ob: usize) {
    let px = g.objs[g.player].x;
    let o = &mut g.objs[ob];
    o.xdir = if (px as u16) < (o.x as u16) { -1 } else { 1 };
    g.do_weak_gravity(ob);
}

/// C_Mushroom (0x1f455): absorbs shots, kills Keen.
pub fn c_mushroom(g: &mut Game, _ob: usize, hit: usize) {
    match g.objs[hit].obclass {
        STUNSHOTOBJ => g.explode_shot(hit),
        KEENOBJ => g.kill_keen(),
        _ => {}
    }
}

/// R_Mushroom (0x1f480): bounces, every third bounce is a high one.
pub fn r_mushroom(g: &mut Game, ob: usize) {
    if g.objs[ob].hitsouth != 0 {
        g.objs[ob].yspeed = 0;
    }
    if g.objs[ob].hitnorth != 0 {
        g.objs[ob].yspeed = 0;
        g.objs[ob].temp1 += 1;
        if g.objs[ob].temp1 == 3 {
            g.objs[ob].temp1 = 0;
            g.objs[ob].yspeed = -68;
            g.play_sound(SND_BOUNCE2);
        } else {
            g.play_sound(SND_BOUNCE1);
            g.objs[ob].yspeed = -40;
        }
    }
    g.place_sprite(ob);
}

// ---------------------------------------------------------------------------
// eggs and eggbirds

/// SpawnEgg (0x1f4ef)
pub fn spawn_egg(g: &mut Game, x: i32, y: i32) {
    let n = g.get_new_obj(false);
    {
        let o = &mut g.objs[n];
        o.obclass = EGGOBJ;
        o.active = AC_YES;
        o.priority = 2;
        o.x = w16(x << 8);
        o.y = w16((y << 8) - 0x71);
        o.xdir = 1;
        o.ydir = 1;
    }
    g.new_state(n, &S_EGG);
}

/// T_EggUnstun (0x1f53f): the eggbird recovers from being stunned.
pub fn t_egg_unstun(g: &mut Game, ob: usize) {
    g.objs[ob].obclass = EGGBIRDOBJ;
}

/// SpawnEggbird (0x1f54d): an eggbird placed directly in the map.
pub fn spawn_eggbird(g: &mut Game, x: i32, y: i32) {
    let n = g.get_new_obj(true);
    let px = g.objs[g.player].x;
    let o = &mut g.objs[n];
    o.obclass = EGGBIRDOBJ;
    o.active = AC_YES;
    o.priority = 2;
    o.x = w16(x << 8);
    o.y = w16((y << 8) - 0xf1);
    o.xdir = dir_to(o.x, px);
    o.ydir = 1;
    g.new_state(n, &S_EGGBIRDPAUSE);
}

/// C_Egg (0x1f5bd): a shot or Keen breaks the egg, the eggbird hatches.
pub fn c_egg(g: &mut Game, ob: usize, hit: usize) {
    let hc = g.objs[hit].obclass;
    if hc != STUNSHOTOBJ && hc != KEENOBJ {
        return;
    }
    if hc == STUNSHOTOBJ {
        g.explode_shot(hit);
    }
    g.objs[ob].obclass = INERTOBJ;
    g.objs[ob].active = AC_REMOVABLE;
    g.change_state(ob, &S_EGGBROKE);

    let (x, y) = (g.objs[ob].x, g.objs[ob].y);
    let px = g.objs[g.player].x;
    let n = g.get_new_obj(true);
    {
        let o = &mut g.objs[n];
        o.obclass = EGGBIRDOBJ;
        o.active = AC_YES;
        o.x = x;
        o.y = w16(y - 0x80);
        o.xdir = dir_to(x, px);
        o.ydir = 1;
    }
    g.new_state(n, &S_EGGBIRDPAUSE);

    for (xs, ys, st) in [(-28, -40, &S_EGGCHIP1), (28, -40, &S_EGGCHIP2), (0, -56, &S_EGGCHIP3)] {
        let n = g.get_new_obj(true);
        {
            let o = &mut g.objs[n];
            o.obclass = INERTOBJ;
            o.active = AC_REMOVABLE;
            o.x = x;
            o.y = y;
            o.xspeed = xs;
            o.yspeed = ys;
        }
        g.new_state(n, st);
    }
}

/// T_Eggbird (0x1f716): walks towards Keen, takes off when Keen stands
/// on a ledge at least 3 tiles higher.
pub fn t_eggbird(g: &mut Game, ob: usize) {
    let p = g.player;
    g.objs[ob].xdir = dir_to(g.objs[ob].x, g.objs[p].x);
    if (w16(g.objs[p].bottom + 0x300) as u16) <= (g.objs[ob].bottom as u16)
        && g.objs[p].hitnorth != 0
        && g.state_position_ok(ob, &S_EGGBIRDFLY1)
    {
        let o = &mut g.objs[ob];
        o.state = Some(&S_EGGBIRDFLY1);
        o.needtoclip = CL_FULLCLIP;
        o.yspeed = -8;
        o.temp1 = 0;
    }
}

/// T_EggbirdFly (0x1f772): flies towards Keen.
pub fn t_eggbird_fly(g: &mut Game, ob: usize) {
    let p = g.player;
    if g.objs[ob].temp1 == 0 {
        g.objs[ob].xdir = dir_to(g.objs[ob].x, g.objs[p].x);
    }
    let xdir = g.objs[ob].xdir;
    g.accelerate_xv(ob, xdir, 0x10);
    let ydir = dir_to(g.objs[ob].y, g.objs[p].y);
    g.accelerate_y(ob, ydir, 0x10);
}

/// C_Eggbird (0x1f7d0)
pub fn c_eggbird(g: &mut Game, ob: usize, hit: usize) {
    match g.objs[hit].obclass {
        KEENOBJ => g.kill_keen(),
        STUNSHOTOBJ => {
            g.objs[ob].xspeed = 0;
            g.objs[ob].needtoclip = CL_MIDCLIP;
            g.stun_obj(ob, hit, &S_EGGBIRDSTUN);
        }
        _ => {}
    }
}

/// C_EggbirdStun (0x1f808): harmless eggbird (stunned or just hatched).
pub fn c_eggbird_stun(g: &mut Game, ob: usize, hit: usize) {
    if g.objs[hit].obclass == STUNSHOTOBJ {
        g.objs[ob].xspeed = 0;
        g.stun_obj(ob, hit, &S_EGGBIRDSTUN);
    }
}

/// R_Eggbird (0x1f82f): walking, turns at walls, flies off ledges.
pub fn r_eggbird(g: &mut Game, ob: usize) {
    let turned = {
        let o = &mut g.objs[ob];
        if o.xdir == 1 && o.hitwest != 0 {
            o.x = w16(o.x - o.xmove);
            o.xdir = -1;
            true
        } else if o.xdir == -1 && o.hiteast != 0 {
            o.x = w16(o.x - o.xmove);
            o.xdir = 1;
            true
        } else {
            false
        }
    };
    if turned {
        g.objs[ob].nothink = (g.rnd() >> 5) as i32;
        let st = g.objs[ob].st();
        g.change_state(ob, st);
    }
    if g.objs[ob].hitnorth == 0 {
        g.objs[ob].yspeed = -16;
        g.objs[ob].needtoclip = CL_FULLCLIP;
        g.change_state(ob, &S_EGGBIRDFLY1);
    }
    g.place_sprite(ob);
}

/// R_EggbirdDrop (0x1f8bb): lands and walks again.
pub fn r_eggbird_drop(g: &mut Game, ob: usize) {
    if g.objs[ob].hitnorth != 0 {
        g.change_state(ob, &S_EGGBIRDWALK1);
    }
    g.place_sprite(ob);
}

/// R_Chunk (0x1f8f5): egg shell pieces stop when they land.
pub fn r_chunk(g: &mut Game, ob: usize) {
    if g.objs[ob].hitnorth != 0 {
        g.objs[ob].yspeed = 0;
        g.objs[ob].xspeed = 0;
    }
    g.place_sprite(ob);
}

/// R_EggbirdFly (0x1f92a): bounces off walls, lands near Keen's height.
pub fn r_eggbird_fly(g: &mut Game, ob: usize) {
    {
        let o = &mut g.objs[ob];
        if (o.hitsouth != 0 || o.hitnorth != 0) && o.temp1 == 0 {
            o.temp1 += 1;
        }
        if (o.hiteast != 0 && o.xspeed < 0) || (o.hitwest != 0 && o.xspeed > 0) {
            o.xspeed = 0;
            o.xdir = -o.xdir;
        }
    }
    let pbottom = g.objs[g.player].bottom;
    if g.objs[ob].hitnorth == 1 && (w16(pbottom - g.objs[ob].bottom) as u16) < 0x80 {
        let oldstate = g.objs[ob].st();
        g.objs[ob].needtoclip = CL_MIDCLIP;
        g.change_state(ob, &S_EGGBIRDWALK1);
        g.ph.xtry = 0;
        g.ph.ytry = 0x80;
        g.push_obj(ob);
        if g.objs[ob].hitnorth == 0 {
            g.objs[ob].needtoclip = CL_FULLCLIP;
            g.change_state(ob, oldstate);
        }
        // the original returns without placing the sprite
        return;
    }
    if g.objs[ob].hitsouth == 0 && g.objs[ob].hitnorth == 0 {
        g.objs[ob].temp1 = 0;
    }
    g.place_sprite(ob);
}

// ---------------------------------------------------------------------------
// arachnut

/// SpawnArachnut (0x1f9fa)
pub fn spawn_arachnut(g: &mut Game, x: i32, y: i32) {
    let n = g.get_new_obj(false);
    {
        let o = &mut g.objs[n];
        o.obclass = ARACHNUTOBJ;
        o.active = AC_YES;
        o.priority = 0;
        o.x = w16(x << 8);
        o.y = w16((y << 8) - 0x171);
    }
    let xdir = if g.rnd() < 0x80 { 1 } else { -1 };
    g.objs[n].xdir = xdir;
    g.objs[n].ydir = 1;
    g.new_state(n, &S_ARACHNUTWALK1);
}

/// T_Arachnut (0x1fa67): turns to Keen.
pub fn t_arachnut(g: &mut Game, ob: usize) {
    let px = g.objs[g.player].x;
    let o = &mut g.objs[ob];
    o.xdir = if (px as u16) < (o.x as u16) { -1 } else { 1 };
}

/// C_Arachnut (0x1fa89)
pub fn c_arachnut(g: &mut Game, ob: usize, hit: usize) {
    match g.objs[hit].obclass {
        STUNSHOTOBJ => g.stun_obj(ob, hit, &S_ARACHNUTSTUN1),
        KEENOBJ => g.kill_keen(),
        _ => {}
    }
}

/// C_ArachnutStunned (0x1fab4): shots stun it again.
pub fn c_arachnut_stunned(g: &mut Game, ob: usize, hit: usize) {
    if g.objs[hit].obclass == STUNSHOTOBJ {
        g.stun_obj(ob, hit, &S_ARACHNUTSTUN1);
    }
}

// ---------------------------------------------------------------------------
// skypest

/// SpawnSkypest (0x1fad3)
pub fn spawn_skypest(g: &mut Game, x: i32, y: i32) {
    let n = g.get_new_obj(false);
    {
        let o = &mut g.objs[n];
        o.obclass = SKYPESTOBJ;
        o.active = AC_YES;
        o.priority = 0;
        o.x = w16(x << 8);
        o.y = w16(y << 8);
    }
    let xdir = if g.rnd() < 0x80 { 1 } else { -1 };
    g.objs[n].xdir = xdir;
    let ydir = if g.rnd() < 0x80 { 1 } else { -1 };
    g.objs[n].ydir = ydir;
    g.new_state(n, &S_SKYPESTFLY1);
}

/// T_SkypestFly (0x1fb52): random flight.
pub fn t_skypest_fly(g: &mut Game, ob: usize) {
    let tics = g.tics as u16 as u32;
    if (g.rnd() as u32) < tics {
        g.objs[ob].xdir = -g.objs[ob].xdir;
    }
    if g.objs[ob].ydir == -1 && (g.rnd() as u32) < tics {
        g.objs[ob].ydir = 1;
    }
    if g.objs[ob].ydir == 1 && (g.rnd() as u32) < ((tics << 1) & 0xffff) {
        g.objs[ob].ydir = -g.objs[ob].ydir;
    }
    let (xdir, ydir) = (g.objs[ob].xdir, g.objs[ob].ydir);
    g.accelerate_x(ob, xdir, 0x14);
    g.accelerate_y(ob, ydir, 0x14);
}

/// C_Skypest (0x1fbc2): kills Keen; shots only push it away.
pub fn c_skypest(g: &mut Game, ob: usize, hit: usize) {
    if g.objs[hit].obclass == KEENOBJ {
        g.kill_keen();
    }
    if g.objs[hit].obclass == STUNSHOTOBJ {
        let (hx, hy) = (g.objs[hit].xdir, g.objs[hit].ydir);
        let o = &mut g.objs[ob];
        if hx == 1 {
            o.xspeed = 0x14;
        } else if hx == -1 {
            o.xspeed = -0x14;
        } else if hy == 1 {
            o.yspeed = 0x14;
        } else if hy == -1 {
            o.yspeed = -0x14;
        }
        g.explode_shot(hit);
    }
}

/// C_Squish (0x1fc1a): a sitting skypest is squashed by Keen's pogo.
pub fn c_squish(g: &mut Game, ob: usize, hit: usize) {
    let st = g.objs[hit].state;
    if same(st, &S_KEENPOGODOWN) || same(st, &S_KEENPOGO) || same(st, &S_KEENPOGO2) {
        g.change_state(ob, &S_SKYPESTSQUISHED);
        g.play_sound(SND_SQUISH);
        g.objs[ob].obclass = INERTOBJ;
    }
}

/// T_SkypestTakeoff (0x1fc5a)
pub fn t_skypest_takeoff(g: &mut Game, ob: usize) {
    g.objs[ob].ydir = -1;
    g.objs[ob].yspeed = -16;
    g.ph.ytry = -0x90;
}

/// R_Skypest (0x1fc74): bounces off walls and the ceiling, lands on floors.
pub fn r_skypest(g: &mut Game, ob: usize) {
    if g.objs[ob].hitsouth != 0 {
        g.objs[ob].yspeed = 8;
        g.objs[ob].ydir = 1;
    }
    let land = {
        let o = &g.objs[ob];
        o.hitnorth != 0 && o.hiteast == 0 && o.hitwest == 0
    };
    if land {
        g.objs[ob].y = w16(g.objs[ob].y + 0x80);
        g.change_state(ob, &S_SKYPESTSIT);
    }
    {
        let o = &mut g.objs[ob];
        if o.hitwest != 0 {
            o.xspeed = 0;
            o.xdir = -1;
        }
        if o.hiteast != 0 {
            o.xspeed = 0;
            o.xdir = 1;
        }
    }
    g.place_sprite(ob);
}

// spawn functions by address (called by ScanInfoPlane)
pub use self::spawn_arachnut as spawn_1f9fa;
pub use self::spawn_bonus as spawn_1f13f;
pub use self::spawn_council as spawn_1f208;
pub use self::spawn_egg as spawn_1f4ef;
pub use self::spawn_eggbird as spawn_1f54d;
pub use self::spawn_mad_mushroom as spawn_1f3db;
pub use self::spawn_skypest as spawn_1fad3;
pub use self::spawn_slug as spawn_1f29e;
pub use self::spawn_splash as spawn_1f1a9;

/// SpawnMiragia (0x1efa1): the inert controller object of the mirage
/// village; x/y stay tile coordinates, the state is set directly.
pub fn spawn_miragia(g: &mut Game, x: i32, y: i32) {
    let n = g.get_new_obj(false);
    let o = &mut g.objs[n];
    o.obclass = INERTOBJ;
    o.active = AC_ALLWAYS;
    o.x = x;
    o.y = y;
    o.state = Some(&S_MIRAGIA0);
}
pub use self::spawn_miragia as spawn_1efa1;


#[derive(Default)]
pub struct ModState {}
