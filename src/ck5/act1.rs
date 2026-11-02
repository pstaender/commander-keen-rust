//! CK5_ACT1 port: generic contact/react helpers, enemy shots, bonus items,
//! vitalin splash, teleporter beam, fuse flash, the dead machine (end of a
//! fuse level), platforms (normal, slot, dropping, static, Go, sneaky),
//! Volte-face and the laser turret (cannon) with its shots.
//! Translated from the decompiled code segment 0x1f92 (linear
//! 0x1f923..0x2108e) of KEEN5E.EXE.

use super::defs::*;
use super::phys::u16shr;
use super::states::*;
use super::Game;

// object classes of this module (Keen 5 values)
/// enemy shot (turret shots and the shots of other enemies)
pub const MSHOTOBJ: i32 = 4;
/// bonus item
pub const BONUSOBJ: i32 = 5;
/// every kind of platform
pub const PLATFORMOBJ: i32 = 6;
/// laser turret
pub const CANNONOBJ: i32 = 15;
/// Volte-face
pub const VOLTEOBJ: i32 = 16;
/// teleporter beam / fuse flash effects
pub const TELEPORTEROBJ: i32 = 24;


// sounds
const SND_ENEMYSHOOT: i32 = 29;
const SND_ENEMYSHOTEXPLODE: i32 = 30;
const SND_TELEPORT: i32 = 41;
const SND_BWOOM: i32 = 52;

/// first direction arrow in the info plane
const DIRARROWSTART: i32 = 0x5b;
/// platform stop block in the info plane
const PLATFORMBLOCK: usize = 0x1f;

/// DS:0x1926 x direction per arrow (9th entry overlaps PDIRY[0], as in the exe)
const PDIRX: [i32; 9] = [0, 1, 0, -1, 1, 1, -1, -1, -1];
/// DS:0x1936 y direction per arrow
const PDIRY: [i32; 9] = [-1, 0, 1, 0, -1, 1, 1, -1, 0];

/// DS:0x1a54 bonus item shapes (temp2 = first frame, temp3 = temp2 + 2):
/// gems 0-3, points 4-9, 1UP 10, stunner 11, keycard 12
const BONUSSHAPE: [i32; 13] = [224, 226, 228, 230, 210, 212, 214, 216, 218, 220, 222, 233, 207];

/// DS:0x7f8 bounceangle[hitnorth][angle]; rows 9.. are whatever follows the
/// table in DGROUP (read by the original for unusual hitnorth values).
const BOUNCEANGLE: [[u16; 8]; 26] = [
    [0, 0, 0, 0, 0, 0, 0, 0],
    [7, 6, 5, 4, 3, 2, 1, 0],
    [5, 4, 3, 2, 1, 0, 15, 14],
    [5, 4, 3, 2, 1, 0, 15, 14],
    [3, 2, 1, 0, 15, 14, 13, 12],
    [9, 8, 7, 6, 5, 4, 3, 2],
    [9, 8, 7, 6, 5, 4, 3, 2],
    [11, 10, 9, 8, 7, 6, 5, 4],
    [2, 3, 0, 1, 6, 7, 4, 5],
    [116, 108, 3, 0, 1, 4, 0, 32],
    [1133, 2735, 5527, 2735, 6854, 2735, 2184, 139],
    [139, 3, 0, 1, 4, 0, 32, 811],
    [2735, 5527, 2735, 6854, 2735, 2214, 124, 124],
    [3, 0, 1, 60, 0, 0, 1133, 2735],
    [5527, 2735, 6854, 2735, 2184, 126, 126, 3],
    [0, 1, 90, 0, 0, 1133, 2735, 5527],
    [2735, 6854, 2735, 2304, 125, 125, 3, 0],
    [1, 10, 0, 0, 1133, 2735, 5527, 2735],
    [6854, 2735, 2334, 126, 126, 3, 0, 1],
    [90, 0, 0, 1133, 2735, 5527, 2735, 6854],
    [2735, 2364, 125, 125, 3, 0, 1, 10],
    [0, 0, 1133, 2735, 5527, 2735, 6854, 2735],
    [2394, 126, 126, 3, 0, 1, 90, 0],
    [0, 1133, 2735, 5527, 2735, 6854, 2735, 2424],
    [127, 127, 3, 0, 1, 70, 0, 0],
    [1133, 2735, 5527, 2735, 6854, 2735, 2184, 128],
];

/// 16-bit signed wrap of a value
#[inline]
fn w16(v: i32) -> i32 {
    v as i16 as i32
}

// ---------------------------------------------------------------------------
// enemy shots, generic contact/react functions

/// SpawnEnemyShot (0x1f923): x, y in global units. Returns 0, or -1 if
/// there was no free object or the shot would start inside a wall.
pub fn spawn_enemy_shot(g: &mut Game, x: i32, y: i32, st: StateRef) -> i32 {
    let n = g.get_new_obj(true);
    if n == DUMMYOBJ {
        return -1;
    }
    {
        let o = &mut g.objs[n];
        o.x = x;
        o.y = y;
        o.obclass = MSHOTOBJ;
        o.active = AC_REMOVABLE;
    }
    g.new_state(n, st);
    if !g.check_position(n) {
        g.remove_obj(n);
        return -1;
    }
    0
}

/// C_ClipSide (0x1f986): Keen is pushed sideways by the object.
pub fn c_clip_side(g: &mut Game, ob: usize, hit: usize) {
    if g.objs[hit].obclass == KEENOBJ {
        g.ph.nowall = true;
        g.clip_to_sprite_side(hit, ob);
        g.ph.nowall = false;
    }
}

/// C_ClipTop (0x1f9ad): Keen can stand on the object.
pub fn c_clip_top(g: &mut Game, ob: usize, hit: usize) {
    if g.objs[hit].obclass == KEENOBJ {
        g.clip_to_sprite_top(hit, ob);
    }
}

/// R_Land (0x1f9c8): falls until it lands, then goes to the next state.
pub fn r_land(g: &mut Game, ob: usize) {
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
        match g.objs[ob].st().next {
            Some(next) => g.change_state(ob, next),
            None => {
                g.remove_obj(ob);
                return;
            }
        }
    }
    g.place_sprite(ob);
}

/// R_Bounce (0x1fa39): bounces off walls and slopes, loses speed.
pub fn r_bounce(g: &mut Game, ob: usize) {
    g.place_sprite(ob);
    {
        let o = &mut g.objs[ob];
        if o.hiteast != 0 || o.hitwest != 0 {
            o.xspeed = w16(-o.xspeed) / 2;
        }
        if o.hitsouth != 0 {
            o.yspeed = w16(-o.yspeed) / 2;
            return;
        }
    }
    let wall = g.objs[ob].hitnorth;
    if wall == 0 {
        return;
    }
    if g.objs[ob].yspeed < 0 {
        g.objs[ob].yspeed = 0;
    }
    let xs = g.objs[ob].xspeed;
    let absx = (xs as i16).unsigned_abs() as u32;
    let absy = g.objs[ob].yspeed as u16 as u32;
    let mut angle: usize;
    let mut speed: u32;
    if absx > absy {
        if ((absy << 1) & 0xffff) < absx {
            angle = 0;
            speed = (absx * 286) & 0xffff;
        } else {
            angle = 1;
            speed = (absx * 362) & 0xffff;
        }
    } else if ((absx << 1) & 0xffff) < absy {
        angle = 3;
        speed = (absy << 8) & 0xffff;
    } else {
        angle = 2;
        speed = (absy * 286) & 0xffff;
    }
    if xs > 0 {
        angle = 7 - angle;
    }
    speed >>= 1;
    let newangle = BOUNCEANGLE.get(wall as usize).map(|r| r[angle]).unwrap_or(0xffff);
    let d286 = (speed / 286) as i32;
    let d362 = (speed / 362) as i32;
    let d256 = (speed >> 8) as i32;
    let o = &mut g.objs[ob];
    match newangle {
        0 => {
            o.xspeed = d286;
            o.yspeed = w16(-o.xspeed) / 2;
        }
        1 => {
            o.xspeed = d362;
            o.yspeed = -o.xspeed;
        }
        2 => {
            o.yspeed = -d286;
            o.xspeed = w16(-o.yspeed) / 2;
        }
        3 | 4 | 11 | 12 => {
            o.xspeed = 0;
            o.yspeed = -d256;
        }
        5 => {
            o.yspeed = -d286;
            o.xspeed = o.yspeed / 2;
        }
        6 => {
            o.yspeed = -d362;
            o.xspeed = o.yspeed;
        }
        7 => {
            o.xspeed = -d286;
            o.yspeed = o.xspeed / 2;
        }
        8 => {
            o.xspeed = -d286;
            o.yspeed = w16(-o.xspeed) / 2;
        }
        9 => {
            o.xspeed = -d362;
            o.yspeed = -o.xspeed;
        }
        10 => {
            o.yspeed = d286;
            o.xspeed = w16(-o.yspeed) / 2;
        }
        13 => {
            o.yspeed = d286;
            o.xspeed = o.yspeed / 2;
        }
        14 => {
            o.xspeed = d362;
            o.yspeed = d362;
        }
        15 => {
            o.xspeed = d286;
            o.yspeed = o.xspeed / 2;
        }
        _ => {}
    }
    if speed < 0x1000 {
        // the original passes state->nextstate even when it is NULL
        match g.objs[ob].st().next {
            Some(next) => g.change_state(ob, next),
            None => g.change_state_null(ob),
        }
    }
}

// ---------------------------------------------------------------------------
// bonus items, splash, teleporter, fuses

/// SpawnBonus (0x1fd9f): kind = index into the bonus shape table.
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

/// SpawnSplash (0x1fe08): the splash of a collected vitalin drop.
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

/// T_Bonus (0x1fe4a): animates the item.
pub fn t_bonus(g: &mut Game, ob: usize) {
    let o = &mut g.objs[ob];
    o.shapenum += 1;
    if o.shapenum == o.temp3 {
        o.shapenum = o.temp2;
    }
}

/// T_FallingBonus (0x1fe65): an item that falls until it lands.
pub fn t_falling_bonus(g: &mut Game, ob: usize) {
    {
        let o = &mut g.objs[ob];
        if o.hitnorth != 0 {
            o.state = Some(&S_BONUS1);
        }
        o.shapenum += 1;
        if o.shapenum == o.temp3 {
            o.shapenum = o.temp2;
        }
    }
    g.do_gravity(ob);
}

/// SpawnTeleport (0x1fe94): the beam and the lightning of a teleporter,
/// at Keen's position.
pub fn spawn_teleport(g: &mut Game) {
    let p = g.player;
    let n = g.get_new_obj(true);
    {
        let (tl, tb) = (g.objs[p].tileleft, g.objs[p].tilebottom);
        let o = &mut g.objs[n];
        o.priority = 3;
        o.needtoclip = CL_NOCLIP;
        o.obclass = TELEPORTEROBJ;
        o.x = w16(tl * 0x100 - 0x80);
        o.y = w16(tb * 0x100 - 0x500);
    }
    g.new_state(n, &S_TELEPORT1);
    let n = g.get_new_obj(true);
    {
        let (tl, tt) = (g.objs[p].tileleft, g.objs[p].tiletop);
        let o = &mut g.objs[n];
        o.priority = 3;
        o.needtoclip = CL_NOCLIP;
        o.obclass = TELEPORTEROBJ;
        o.x = w16(tl << 8);
        o.y = w16(tt * 0x100 - 0x80);
    }
    g.new_state(n, &S_TELEPORTZAP1);
    g.play_sound(SND_TELEPORT);
}

/// SpawnFuseFlash (0x1ff48): sparks of a destroyed fuse (tile coordinates).
pub fn spawn_fuse_flash(g: &mut Game, x: i32, y: i32) {
    let n = g.get_new_obj(true);
    {
        let o = &mut g.objs[n];
        o.priority = 3;
        o.needtoclip = CL_NOCLIP;
        o.obclass = TELEPORTEROBJ;
        o.x = w16((x - 1) * 0x100);
        o.y = w16(y << 8);
    }
    g.new_state(n, &S_FUSEFLASH1);
    g.play_sound(SND_BWOOM);
}

/// SpawnDeadMachine (0x1ff97): all fuses are gone, ends the level after
/// a moment.
pub fn spawn_dead_machine(g: &mut Game) {
    let n = g.get_new_obj(false);
    g.objs[n].active = AC_ALLWAYS;
    g.objs[n].needtoclip = CL_NOCLIP;
    g.new_state(n, &S_DEADMACHINE);
}

/// T_DeadMachine (0x1ffc2)
pub fn t_dead_machine(g: &mut Game, _ob: usize) {
    if g.ca_mapon == 12 {
        g.playstate = EX_QEDBROKE;
    } else {
        g.playstate = EX_FUSEBROKE;
    }
}

// ---------------------------------------------------------------------------
// platforms

/// SpawnPlatform (0x1ffdc): dir 0 up, 1 right, 2 down, 3 left; slot != 0 is
/// the small platform that runs in a slot of the floor (offset by 4 pixels).
pub fn spawn_platform(g: &mut Game, x: i32, y: i32, dir: i32, slot: i32) {
    let n = g.get_new_obj(false);
    {
        let o = &mut g.objs[n];
        o.obclass = PLATFORMOBJ;
        o.active = AC_ALLWAYS;
        o.priority = 0;
        o.x = w16(x << 8);
        o.y = w16(y << 8);
        match dir {
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
    if slot != 0 {
        let o = &mut g.objs[n];
        o.x = w16(o.x + 0x40);
        o.y = w16(o.y + 0x40);
        g.new_state(n, &S_SLOTPLAT1);
    } else {
        g.new_state(n, &S_PLATFORM);
    }
}

/// Common code of T_Platform and T_SlotPlat: moves until it hits a
/// platform block, then turns. `slot` selects the tile columns the slot
/// platform checks when moving vertically.
fn platform_move(g: &mut Game, ob: usize, slot: bool) {
    if g.ph.xtry != 0 || g.ph.ytry != 0 {
        return;
    }
    let tics = g.tics;
    let (xdir, ydir) = (g.objs[ob].xdir, g.objs[ob].ydir);
    g.ph.xtry = w16(w16(xdir * 12) * tics);
    g.ph.ytry = w16(w16(ydir * 12) * tics);
    let o = g.objs[ob].clone();
    let col = if slot { o.tileleft + 1 } else { o.tileleft };
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
        // (the slot platform checks column tileleft+1 here, but the
        // tile two rows up in column tileleft, as in the original)
        if o.tilebottom != t && g.map_tile(2, col, t) == PLATFORMBLOCK {
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
        if o.tiletop != t && g.map_tile(2, col, t) == PLATFORMBLOCK {
            if g.map_tile(2, col, t + 2) == PLATFORMBLOCK {
                g.ph.ytry = 0;
                g.objs[ob].needtoreact = true;
            } else {
                g.objs[ob].ydir = 1;
                g.ph.ytry = w16(g.ph.ytry + (0x100 - (v & 0xff)));
            }
        }
    }
}

/// T_Platform (0x20096)
pub fn t_platform(g: &mut Game, ob: usize) {
    platform_move(g, ob, false);
}

/// T_SlotPlat (0x202bc): the platform running in a floor slot.
pub fn t_slot_plat(g: &mut Game, ob: usize) {
    platform_move(g, ob, true);
}

/// SpawnDropPlat (0x204e8)
pub fn spawn_drop_plat(g: &mut Game, x: i32, y: i32) {
    let n = g.get_new_obj(false);
    {
        let o = &mut g.objs[n];
        o.obclass = PLATFORMOBJ;
        o.active = AC_ALLWAYS;
        o.priority = 0;
        o.x = w16(x << 8);
        o.temp1 = w16(y << 8);
        o.y = o.temp1;
        o.xdir = 0;
        o.ydir = 1;
        o.needtoclip = CL_NOCLIP;
    }
    g.new_state(n, &S_DROPPLATSIT);
}

/// T_DropPlatSit (0x2053b): sinks a little while Keen stands on it.
pub fn t_drop_plat_sit(g: &mut Game, ob: usize) {
    if g.gs.riding != Some(ob) {
        return;
    }
    g.ph.ytry = w16(g.tics << 4);
    g.objs[ob].yspeed = 0;
    let o = &g.objs[ob];
    if ((o.y + g.ph.ytry - o.temp1) as u16) >= 0x80 {
        g.objs[ob].state = Some(&S_DROPPLATFALL);
    }
}

/// T_DropPlatFall (0x2056e)
pub fn t_drop_plat_fall(g: &mut Game, ob: usize) {
    g.do_gravity(ob);
    let o = g.objs[ob].clone();
    let t = u16shr(o.bottom + g.ph.ytry, 8);
    if o.tilebottom != t && g.map_tile(2, o.tileleft, t) == PLATFORMBLOCK {
        g.ph.ytry = 0xff - (o.bottom & 0xff);
        if g.gs.riding != Some(ob) {
            g.objs[ob].state = Some(&S_DROPPLATRISE);
        }
    }
}

/// T_DropPlatRise (0x205d9)
pub fn t_drop_plat_rise(g: &mut Game, ob: usize) {
    if g.gs.riding == Some(ob) {
        g.objs[ob].yspeed = 0;
        g.objs[ob].state = Some(&S_DROPPLATFALL);
    } else if (g.objs[ob].y as u16) <= (g.objs[ob].temp1 as u16) {
        g.ph.ytry = w16(g.objs[ob].temp1 - g.objs[ob].y);
        g.objs[ob].state = Some(&S_DROPPLATSIT);
    }
}

/// SpawnStaticPlat (0x2060c)
pub fn spawn_static_plat(g: &mut Game, x: i32, y: i32) {
    let n = g.get_new_obj(false);
    {
        let o = &mut g.objs[n];
        o.obclass = PLATFORMOBJ;
        o.active = AC_YES;
        o.priority = 0;
        o.x = w16(x << 8);
        o.temp1 = w16(y << 8);
        o.y = o.temp1;
        o.xdir = 0;
        o.ydir = 1;
        o.needtoclip = CL_NOCLIP;
    }
    g.new_state(n, &S_STATICPLAT);
}

/// SpawnGoPlat (0x2065f): follows the direction arrows in the info plane;
/// slot != 0 is the slot platform version.
pub fn spawn_goplat(g: &mut Game, x: i32, y: i32, dir: i32, slot: i32) {
    let n = g.get_new_obj(false);
    {
        let o = &mut g.objs[n];
        o.obclass = PLATFORMOBJ;
        o.active = AC_ALLWAYS;
        o.priority = 0;
        o.x = w16(x << 8);
        o.y = w16(y << 8);
        o.xdir = 0;
        o.ydir = 1;
        o.needtoclip = CL_NOCLIP;
    }
    if slot == 0 {
        g.new_state(n, &S_GOPLAT);
    } else {
        let o = &mut g.objs[n];
        o.x = w16(o.x + 0x40);
        o.y = w16(o.y + 0x40);
        g.new_state(n, &S_GOSLOTPLAT1);
    }
    g.set_map_tile(2, x, y, (dir + DIRARROWSTART) as u16);
    g.objs[n].temp1 = dir;
    g.objs[n].temp2 = 0x100;
}

/// What an arrow follower does when it reaches a tile without an arrow.
enum BadSpot {
    /// Quit with "<text>: x,y" (coordinates in hex)
    Coords(&'static str),
    /// Quit with the text
    Plain(&'static str),
}

/// Common code of T_GoPlat, T_GoSlotPlat and T_Volte: moves `speed` units
/// per tic along the arrows of the info plane. temp1 = arrow direction,
/// temp2 = distance to the next tile. `off` is added to the position when
/// looking up the next arrow.
fn follow_arrows(g: &mut Game, ob: usize, speed: i32, off: i32, bad: BadSpot) {
    if g.ph.xtry != 0 || g.ph.ytry != 0 {
        return;
    }
    let mv = (g.tics * speed) & 0xffff;
    let t1 = (g.objs[ob].temp1 as usize).min(8);
    if (g.objs[ob].temp2 as u16 as i32) > mv {
        g.objs[ob].temp2 = w16(g.objs[ob].temp2 - mv);
        match PDIRX[t1] {
            1 => g.ph.xtry = w16(g.ph.xtry + mv),
            -1 => g.ph.xtry = w16(g.ph.xtry - mv),
            _ => {}
        }
        match PDIRY[t1] {
            1 => g.ph.ytry = w16(g.ph.ytry + mv),
            -1 => g.ph.ytry = w16(g.ph.ytry - mv),
            _ => {}
        }
        return;
    }
    let t2 = g.objs[ob].temp2;
    match PDIRX[t1] {
        1 => g.ph.xtry = t2,
        -1 => g.ph.xtry = w16(-t2),
        _ => {}
    }
    match PDIRY[t1] {
        1 => g.ph.ytry = t2,
        -1 => g.ph.ytry = w16(-t2),
        _ => {}
    }
    let tx = u16shr(g.objs[ob].x + g.ph.xtry + off, 8);
    let ty = u16shr(g.objs[ob].y + g.ph.ytry + off, 8);
    let t1 = g.map_tile(2, tx, ty) as i32 - DIRARROWSTART;
    g.objs[ob].temp1 = t1;
    if !(0..=8).contains(&t1) {
        let msg = match bad {
            BadSpot::Coords(s) => {
                format!("{}{:x},{:x}", s, g.objs[ob].x as u16, g.objs[ob].y as u16)
            }
            BadSpot::Plain(s) => s.to_string(),
        };
        g.quit(&msg);
        return;
    }
    let rest = w16(mv - t2);
    g.objs[ob].temp2 = w16(0x100 - rest);
    let t1 = t1 as usize;
    match PDIRX[t1] {
        1 => g.ph.xtry = w16(g.ph.xtry + rest),
        -1 => g.ph.xtry = w16(g.ph.xtry - rest),
        _ => {}
    }
    match PDIRY[t1] {
        1 => g.ph.ytry = w16(g.ph.ytry + rest),
        -1 => g.ph.ytry = w16(g.ph.ytry - rest),
        _ => {}
    }
}

/// T_GoPlat (0x20705)
pub fn t_goplat(g: &mut Game, ob: usize) {
    follow_arrows(g, ob, 12, 0, BadSpot::Coords("Goplat moved to a bad spot: "));
}

/// T_GoSlotPlat (0x20925): the Go platform that runs in a floor slot
/// (its position is offset by 4 pixels).
pub fn t_go_slot_plat(g: &mut Game, ob: usize) {
    follow_arrows(g, ob, 12, 0x40, BadSpot::Plain("Goplat moved to a bad spot!"));
}

// ---------------------------------------------------------------------------
// Volte-face

/// SpawnVolte (0x20ad4): looks at the neighbouring arrows to find the
/// direction to start with.
pub fn spawn_volte(g: &mut Game, x: i32, y: i32) {
    let n = g.get_new_obj(false);
    {
        let o = &mut g.objs[n];
        o.obclass = VOLTEOBJ;
        o.active = AC_ALLWAYS;
        o.priority = 2;
        o.x = w16(x << 8);
        o.y = w16(y << 8);
        o.needtoclip = CL_NOCLIP;
    }
    g.new_state(n, &S_VOLTEFACE1);
    // the original leaves the direction uninitialized (register si) if no
    // arrow points to the spawn spot
    let dir = if g.map_tile(2, x - 1, y) == 0x5c {
        1
    } else if g.map_tile(2, x + 1, y) == 0x5e {
        3
    } else if g.map_tile(2, x, y - 1) == 0x5d {
        2
    } else {
        // (also the fallback when the tile below isn't an "up" arrow)
        0
    };
    g.set_map_tile(2, x, y, (dir + DIRARROWSTART) as u16);
    g.objs[n].temp1 = dir;
    g.objs[n].temp2 = 0x100;
}

/// T_Volte (0x20b9e): moves along the arrows, fast.
pub fn t_volte(g: &mut Game, ob: usize) {
    follow_arrows(g, ob, 0x20, 0, BadSpot::Coords("Volte moved to a bad spot: "));
}

/// C_Volte (0x20dbd): deadly; a shot stuns it for a while.
pub fn c_volte(g: &mut Game, ob: usize, hit: usize) {
    if g.objs[hit].obclass == KEENOBJ {
        g.kill_keen();
        return;
    }
    if g.objs[hit].obclass == STUNSHOTOBJ {
        g.explode_shot(hit);
        g.change_state(ob, &S_VOLTEFACESTUN);
    }
}

// ---------------------------------------------------------------------------
// sneaky platform

/// SpawnSneakPlat (0x20df1)
pub fn spawn_sneak_plat(g: &mut Game, x: i32, y: i32) {
    let n = g.get_new_obj(false);
    {
        let o = &mut g.objs[n];
        o.obclass = PLATFORMOBJ;
        o.active = AC_ALLWAYS;
        o.priority = 0;
        o.temp1 = w16(x << 8);
        o.x = o.temp1;
        o.y = w16(y << 8);
        o.xdir = 0;
        o.ydir = 1;
        o.needtoclip = CL_NOCLIP;
    }
    g.new_state(n, &S_SNEAKPLATSIT);
}

/// T_SneakPlat (0x20e44): dodges when Keen jumps towards it.
pub fn t_sneak_plat(g: &mut Game, ob: usize) {
    let p = g.player;
    if !same(g.objs[p].state, &S_KEENJUMP1) {
        return;
    }
    let (pl, o) = (&g.objs[p], &g.objs[ob]);
    let dist = if pl.xdir == 1 { w16(o.left - pl.right) } else { w16(pl.left - o.right) };
    if dist > 0x400 || dist < 0 {
        return;
    }
    let dy = w16(pl.y - o.y);
    if !(-0x600..=0x600).contains(&dy) {
        return;
    }
    let xdir = pl.xdir;
    g.objs[ob].xdir = xdir;
    g.objs[ob].state = Some(&S_SNEAKPLATDODGE);
}

// ---------------------------------------------------------------------------
// laser turret

/// SpawnCannon (0x20eb7): dir 0 up, 1 right, 2 down, 3 left.
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
        let gx = w16(x << 8);
        o.right = gx;
        o.left = gx;
        o.x = gx;
        let gy = w16(y << 8);
        o.bottom = gy;
        o.top = gy;
        o.y = gy;
        o.temp1 = dir;
    }
    g.new_state(n, &S_CANNON);
}

/// T_Cannon (0x20f1d): fires a shot in the turret's direction.
pub fn t_cannon(g: &mut Game, ob: usize) {
    let n = g.get_new_obj(true);
    let (x, y, dir) = (g.objs[ob].x, g.objs[ob].y, g.objs[ob].temp1);
    {
        let o = &mut g.objs[n];
        o.obclass = MSHOTOBJ;
        o.active = AC_REMOVABLE;
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
    g.play_sound(SND_ENEMYSHOOT);
}

/// C_CShot (0x20fa9)
pub fn c_cshot(g: &mut Game, ob: usize, hit: usize) {
    if g.objs[hit].obclass == KEENOBJ {
        g.kill_keen();
        g.change_state(ob, &S_CSHOTHIT1);
    }
}

/// R_CShot (0x20fcc): explodes when it hits a wall.
pub fn r_cshot(g: &mut Game, ob: usize) {
    let o = &g.objs[ob];
    if o.hitnorth != 0 || o.hiteast != 0 || o.hitsouth != 0 || o.hitwest != 0 {
        g.play_sound(SND_ENEMYSHOTEXPLODE);
        g.change_state(ob, &S_CSHOTHIT1);
    }
    g.place_sprite(ob);
}

// spawn functions under their address (called by ScanInfoPlane)
pub use self::spawn_bonus as spawn_1fd9f;
pub use self::spawn_cannon as spawn_20eb7;
pub use self::spawn_dead_machine as spawn_1ff97;
pub use self::spawn_drop_plat as spawn_204e8;
pub use self::spawn_enemy_shot as spawn_1f923;
pub use self::spawn_fuse_flash as spawn_1ff48;
pub use self::spawn_goplat as spawn_2065f;
pub use self::spawn_platform as spawn_1ffdc;
pub use self::spawn_sneak_plat as spawn_20df1;
pub use self::spawn_splash as spawn_1fe08;
pub use self::spawn_static_plat as spawn_2060c;
pub use self::spawn_teleport as spawn_1fe94;
pub use self::spawn_volte as spawn_20ad4;

/// Module-private state stored in `Game::act1` (none needed).
#[derive(Default, Clone)]
pub struct ModState {}
