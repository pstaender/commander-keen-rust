//! CK5 actors, part 3: Shikadi mine, Robo Red, Spirogrip, Spindred,
//! Shikadi master, Shikadi, Shockshund, Sphereful, Korath and the QED.
//! Translated from code segment 0x21c4 of KEEN5E.EXE (linear 0x21c41 ..
//! 0x23a0b; SpawnMine at 0x21c41 is not in Ghidra's function list).

use super::defs::*;
use super::keen;
use super::rf::SpriteSlot;
use super::states::*;
use super::Game;

// ---- object classes ---------------------------------------------------------

/// enemy shot (Robo Red, Shikadi master and Shockshund shots, pole shock)
pub const MSHOTOBJ: i32 = 4;
pub const MINEOBJ: i32 = 10;
pub const ROBOREDOBJ: i32 = 12;
pub const SPIROGRIPOBJ: i32 = 13;
pub const SPINDREDOBJ: i32 = 18;
pub const SHIKADIMASTEROBJ: i32 = 19;
pub const SHIKADIOBJ: i32 = 20;
pub const SHOCKSHUNDOBJ: i32 = 21;
pub const SPHEREFULOBJ: i32 = 22;
pub const KORATHOBJ: i32 = 23;
/// fuse flash of the destroyed QED (spawned by the act1 SpawnFuseFlash)
pub const QEDFLASHOBJ: i32 = 24;
pub const QEDOBJ: i32 = 25;

// ---- sounds (named after their use here) ------------------------------------
const SND_MINEEXPLODE: i32 = 5;
const SND_SPINDREDFLYUP: i32 = 16;
const SND_SPIROGRIPLAND: i32 = 27;
const SND_SPINDREDBOUNCE: i32 = 28;
const SND_ROBOSHOOT: i32 = 29;
const SND_ROBOSHOTHIT: i32 = 30;
const SND_SPINDREDSLAM: i32 = 37;
const SND_MASTERATTACK: i32 = 38;
const SND_MASTERTELEPORT: i32 = 39;
const SND_SHIKADIATTACK: i32 = 40;
const SND_SHOCKSHUNDBARK: i32 = 42;
const SND_BARKSHOTDIE: i32 = 45;
const SND_SPHEREFULBOUNCE: i32 = 59;
const SND_SPHEREFULCEILING: i32 = 16;
const SND_SPIROGRIPSPIN: i32 = 61;

// ---- sprites ----------------------------------------------------------------
/// the eye of the Shikadi mine
const MINEEYESPR: i32 = 379;
/// first of the 4 Sphereful diamond sprites
const SPHEREFULDIAMONDSPR: i32 = 0x16a;

/// opposite direction (dirtype 0..8) for ChaseThink (DS 0x25f6)
const OPPOSITE: [i32; 9] = [2, 3, 0, 1, 6, 7, 4, 5, 8];
const NODIR: i32 = 8;

/// Sphereful diamond orbit offsets (DS 0x30b2)
const DIAMONDPOS: [i32; 16] = [384, 369, 328, 265, 192, 119, 58, 15, 0, 15, 58, 119, 192, 265, 328, 369];

#[derive(Default)]
pub struct ModState {}

// ---- helpers ------------------------------------------------------------------

#[inline]
fn u(v: i32) -> u16 {
    v as u16
}

/// Random xdir used by the spawn functions: `US_RndT() < 0x80 ? 1 : -1`.
fn rnd_dir(g: &mut Game) -> i32 {
    if g.rnd() < 0x80 {
        1
    } else {
        -1
    }
}

/// Direction towards the player: `ob->x < player->x ? 1 : -1` (unsigned).
fn dir_to_player(g: &Game, ob: usize) -> i32 {
    if u(g.objs[g.player].x) > u(g.objs[ob].x) {
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

/// Foreground tile at (x, y) addressed like the original (mapbwidthtable[y] + x).
fn fg_at(g: &Game, x: i32, y: i32) -> usize {
    fg_linear(g, y * g.mapwidth + x)
}

fn any_wall(g: &Game, t: usize) -> bool {
    g.north(t) != 0 || g.east(t) != 0 || g.south(t) != 0 || g.west(t) != 0
}

fn any_hit(g: &Game, ob: usize) -> bool {
    let o = &g.objs[ob];
    o.hitnorth != 0 || o.hiteast != 0 || o.hitsouth != 0 || o.hitwest != 0
}

/// R_Draw with the white flash (RF_PlaceSprite maskdraw) while temp3 counts down.
fn draw_flash(g: &mut Game, ob: usize) {
    if g.objs[ob].temp3 != 0 {
        g.objs[ob].temp3 -= 1;
        g.place_sprite(ob);
        g.set_sprite_white(ob, true);
    } else {
        g.place_sprite(ob);
    }
}

/// RF_PlaceSprite for the additional sprite pointers temp1..temp3 of an
/// object (only the Sphereful uses them). The shared refresh manager has
/// two slots per object (sprite, and `stars` = temp4); these are kept in
/// `rf.sprites` behind the object slots: index MAXACTORS+1 + ob*3 + slot.
/// TODO: replace with a shared per-object extra sprite API.
fn place_extra_sprite(g: &mut Game, ob: usize, slot: usize, x: i32, y: i32, shape: i32, priority: i32) {
    let idx = MAXACTORS + 1 + ob * 3 + slot;
    if g.rf.sprites.len() <= idx {
        g.rf.sprites.resize(MAXACTORS + 1 + (MAXACTORS + 1) * 3, SpriteSlot::default());
    }
    g.rf.order += 1;
    let order = g.rf.order;
    g.rf.sprites[idx] = SpriteSlot { visible: shape != 0, x, y, shape, priority, order, white: false };
}

/// SpawnEnemyShot (0x1f923, act1): Some(new object), or None if there
/// is no free object or the shot would start inside a wall.
fn spawn_enemy_shot(g: &mut Game, x: i32, y: i32, st: StateRef) -> Option<usize> {
    if super::act1::spawn_enemy_shot(g, x, y, st) == -1 {
        None
    } else {
        Some(g.newobj)
    }
}

use super::act1::{spawn_dead_machine, spawn_fuse_flash};

/// RF_MapToMap (0x28255): copies a block of all three map planes.
fn rf_map_to_map(g: &mut Game, srcx: i32, srcy: i32, destx: i32, desty: i32, w: i32, h: i32) {
    for y in 0..h {
        for x in 0..w {
            for plane in 0..3 {
                let t = g.map_tile(plane, srcx + x, srcy + y) as u16;
                g.set_map_tile(plane, destx + x, desty + y, t);
            }
        }
    }
}

// ===========================================================================
// Shikadi mine
// temp2/temp3 = position of the eye (relative to the mine), temp4 = eye sprite
// (the `stars` sprite slot), xspeed = distance left to the next tile
// ===========================================================================

/// SpawnMine (0x21c41)
pub fn spawn_mine(g: &mut Game, x: i32, y: i32) {
    let n = g.get_new_obj(false);
    {
        let o = &mut g.objs[n];
        o.obclass = MINEOBJ;
        o.active = AC_YES;
        o.needtoclip = CL_NOCLIP;
        o.x = x << 8;
        o.y = (y << 8) - 0x1f1;
        o.temp2 = 0x100;
        o.temp3 = 0xd0;
    }
    g.new_state(n, &S_MINESHIFT);
    g.objs[n].xspeed = 0x100;
    for dir in 0..=3 {
        if mine_walk(g, n, dir) {
            break;
        }
    }
}
pub use self::spawn_mine as spawn_21c41;

/// MinePosCheck (0x21cb5): true if the 3x2 tiles at (tilex, tiley) have no walls.
fn mine_pos_check(g: &Game, tilex: i32, tiley: i32) -> bool {
    let base = tiley * g.mapwidth + tilex;
    for y in 0..2 {
        for x in 0..3 {
            let t = fg_linear(g, base + y * g.mapwidth + x);
            if any_wall(g, t) {
                return false;
            }
        }
    }
    true
}

/// Walk (0x21d3a): tries to move the mine one tile in `dir` (0 up, 1 right,
/// 2 down, 3 left).
fn mine_walk(g: &mut Game, ob: usize, dir: i32) -> bool {
    let tilex = u(g.objs[ob].x + g.ph.xtry) as i32 >> 8;
    let tiley = u(g.objs[ob].y + g.ph.ytry) as i32 >> 8;
    let (cx, cy, xd, yd) = match dir {
        0 => (tilex, tiley - 1, 0, -1),
        1 => (tilex + 1, tiley, 1, 0),
        2 => (tilex, tiley + 1, 0, 1),
        3 => (tilex - 1, tiley, -1, 0),
        _ => {
            g.quit("Walk: Bad dir");
            return false;
        }
    };
    if mine_pos_check(g, cx, cy) {
        g.objs[ob].xdir = xd;
        g.objs[ob].ydir = yd;
        return true;
    }
    false
}

/// ChaseThink (0x21e3f): picks the next direction towards Keen.
fn mine_chase_think(g: &mut Game, ob: usize) {
    let (xdir, ydir) = (g.objs[ob].xdir, g.objs[ob].ydir);
    let mut olddir = 0;
    if xdir == 1 {
        olddir = 1;
    } else if xdir == -1 {
        olddir = 3;
    } else if ydir == -1 {
        olddir = 0;
    } else if ydir == 1 {
        olddir = 2;
    }
    let turnaround = OPPOSITE[olddir as usize];
    let p = g.player;
    let deltax = (g.objs[p].x - (g.objs[ob].x + g.ph.xtry)) as i16 as i32;
    let deltay = (g.objs[p].y - (g.objs[ob].y + g.ph.ytry)) as i16 as i32;
    let mut d0 = NODIR;
    let mut d1 = NODIR;
    if deltax > 0 {
        d0 = 1;
    }
    if deltax < 0 {
        d0 = 3;
    }
    if deltay > 0 {
        d1 = 2;
    }
    if deltay < 0 {
        d1 = 0;
    }
    if deltay.abs() > deltax.abs() {
        std::mem::swap(&mut d0, &mut d1);
    }
    if d0 == turnaround {
        d0 = NODIR;
    }
    if d1 == turnaround {
        d1 = NODIR;
    }
    if d1 != NODIR && mine_walk(g, ob, d1) {
        return;
    }
    if d0 != NODIR && mine_walk(g, ob, d0) {
        return;
    }
    if mine_walk(g, ob, olddir) {
        return;
    }
    if g.rnd() > 0x80 {
        for dir in 0..=3 {
            if dir != turnaround && mine_walk(g, ob, dir) {
                return;
            }
        }
    } else {
        for dir in (0..=3).rev() {
            if dir != turnaround && mine_walk(g, ob, dir) {
                return;
            }
        }
    }
    mine_walk(g, ob, turnaround);
}

/// Keen close enough to the mine (dx/dy = mine - player)?
fn mine_near(g: &Game, ob: usize, xlo: i32, ylo: i32) -> bool {
    let dx = (g.objs[ob].x - g.objs[g.player].x) as i16 as i32;
    let dy = (g.objs[ob].y - g.objs[g.player].y) as i16 as i32;
    dx <= 0x200 && dx >= xlo && dy <= 0x300 && dy >= ylo
}

/// T_Mine (0x21faa): moves tile by tile, explodes near Keen.
pub fn t_mine(g: &mut Game, ob: usize) {
    if mine_near(g, ob, -0x500, -0x50) {
        g.play_sound(SND_MINEEXPLODE);
        g.objs[ob].state = Some(&S_MINEEXPLODE1);
        g.remove_stars(ob);
        return;
    }
    let mut speed = g.tics * 10;
    if g.objs[ob].xspeed <= speed {
        let (xdir, ydir, xs) = (g.objs[ob].xdir, g.objs[ob].ydir, g.objs[ob].xspeed);
        g.ph.xtry = xdir * xs;
        g.ph.ytry = ydir * xs;
        speed -= xs;
        mine_chase_think(g, ob);
        g.objs[ob].xspeed = 0x100;
        if g.objs[ob].xdir != xdir || g.objs[ob].ydir != ydir {
            g.objs[ob].state = Some(&S_MINECENTER);
            return;
        }
    }
    g.objs[ob].xspeed -= speed;
    g.ph.xtry += g.objs[ob].xdir * speed;
    g.ph.ytry += g.objs[ob].ydir * speed;
}

/// Moves the eye coordinate `v` towards `target` by tics*4; when it gets
/// there, the mine goes on to the next state.
fn mine_eye_move(g: &mut Game, ob: usize, temp3: bool, target: i32) {
    let step = g.tics * 4;
    let o = &mut g.objs[ob];
    let v = if temp3 { &mut o.temp3 } else { &mut o.temp2 };
    let mut arrived = false;
    if *v < target {
        *v += step;
        if *v >= target {
            *v = target;
            arrived = true;
        }
    } else if *v > target {
        *v -= step;
        if *v <= target {
            *v = target;
            arrived = true;
        }
    }
    if arrived {
        o.state = o.st().next;
    }
}

/// T_MineCenter (0x22147): moves the eye to the center.
pub fn t_mine_center(g: &mut Game, ob: usize) {
    if mine_near(g, ob, -0x300, -0x300) {
        g.objs[ob].state = Some(&S_MINEEXPLODE1);
        g.remove_stars(ob);
        return;
    }
    g.objs[ob].needtoreact = true;
    mine_eye_move(g, ob, false, 0x100);
    mine_eye_move(g, ob, true, 0xd0);
}

/// T_MineShift (0x22266): moves the eye towards the new direction.
pub fn t_mine_shift(g: &mut Game, ob: usize) {
    if mine_near(g, ob, -0x300, -0x300) {
        g.objs[ob].state = Some(&S_MINEEXPLODE1);
        g.remove_stars(ob);
        return;
    }
    g.objs[ob].needtoreact = true;
    let xt = match g.objs[ob].xdir {
        -1 => 0x80,
        0 => 0x100,
        _ => 0x180,
    };
    let yt = match g.objs[ob].ydir {
        -1 => 0x50,
        0 => 0xd0,
        _ => 0x150,
    };
    mine_eye_move(g, ob, false, xt);
    mine_eye_move(g, ob, true, yt);
}

/// C_Solid (0x2207e): the mine absorbs shots.
pub fn c_mine(g: &mut Game, _ob: usize, hit: usize) {
    if g.objs[hit].obclass == STUNSHOTOBJ {
        g.explode_shot(hit);
    }
}

/// T_MineFrags (0x223c3): the mine bursts into six pieces.
pub fn t_mine_frags(g: &mut Game, ob: usize) {
    g.play_sound(SND_MINEEXPLODE);
    let (x, y) = (g.objs[ob].x, g.objs[ob].y);
    // (x offset, xspeed from US_RndT, yspeed) of the six pieces
    for k in 0..6 {
        let n = g.get_new_obj(true);
        g.objs[n].x = if k & 1 == 0 { x } else { x + 0x100 };
        g.objs[n].y = y;
        let (xs, ys) = match k {
            0 => (-(g.rnd() as i32 >> 3), -48),
            1 => (g.rnd() as i32 >> 3, -48),
            2 => ((g.rnd() as i32 >> 4) + 40, -24),
            3 => (-40 - (g.rnd() as i32 >> 4), -24),
            _ => (24, 16),
        };
        g.objs[n].xspeed = xs;
        g.objs[n].yspeed = ys;
        g.new_state(n, &S_MINEPIECE);
    }
}

/// C_MineFrag (0x22096): a piece of a mine destroys the QED.
pub fn c_mine_frag(g: &mut Game, _ob: usize, hit: usize) {
    match g.objs[hit].obclass {
        STUNSHOTOBJ => g.explode_shot(hit),
        KEENOBJ => g.kill_keen(),
        QEDOBJ => {
            let (l, t, r, b) = {
                let h = &g.objs[hit];
                (h.tileleft, h.tiletop, h.tileright, h.tilebottom)
            };
            spawn_fuse_flash(g, l, t);
            spawn_fuse_flash(g, r, t);
            spawn_fuse_flash(g, l, b);
            spawn_fuse_flash(g, r, b);
            rf_map_to_map(g, 0, 0, 16, 11, 4, 2);
            rf_map_to_map(g, 4, 0, 16, 13, 4, 2);
            spawn_dead_machine(g);
            g.remove_obj(hit);
        }
        _ => {}
    }
}

/// R_Mine (0x2254e): draws the mine and its eye.
pub fn r_mine(g: &mut Game, ob: usize) {
    g.place_sprite(ob);
    let (x, y, ex, ey) = {
        let o = &g.objs[ob];
        (o.x, o.y, o.temp2, o.temp3)
    };
    g.place_stars(ob, x + ex, y + ey, MINEEYESPR, 2);
}

// ===========================================================================
// Robo Red
// ===========================================================================

/// SpawnRoboRed (0x2259c)
pub fn spawn_robored(g: &mut Game, x: i32, y: i32) {
    let n = g.get_new_obj(false);
    {
        let o = &mut g.objs[n];
        o.obclass = ROBOREDOBJ;
        o.active = AC_YES;
        o.x = x << 8;
        o.y = (y << 8) - 0x400;
    }
    g.objs[n].xdir = rnd_dir(g);
    g.new_state(n, &S_ROBORED);
}
pub use self::spawn_robored as spawn_2259c;

/// T_RoboRed (0x225fc): sometimes starts shooting when Keen is level with it.
pub fn t_robored(g: &mut Game, ob: usize) {
    let p = g.player;
    if u(g.objs[ob].x) & 0x40 == 0
        && u(g.objs[p].bottom) > u(g.objs[ob].top)
        && u(g.objs[p].top) < u(g.objs[ob].bottom)
        && g.rnd() < 0x10
    {
        g.objs[ob].xdir = if u(g.objs[ob].x) > u(g.objs[p].x) { -1 } else { 1 };
        g.objs[ob].temp1 = 10;
        g.objs[ob].state = Some(&S_ROBOREDATTACK1);
    }
}

/// C_RoboRed (0x2264d): a shot makes it fire at Keen.
pub fn c_robored(g: &mut Game, ob: usize, hit: usize) {
    if g.objs[hit].obclass == STUNSHOTOBJ {
        g.explode_shot(hit);
        g.objs[ob].xdir = dir_to_player(g, ob);
        g.objs[ob].temp1 = 10;
        g.change_state(ob, &S_ROBOREDATTACK1);
    } else if g.objs[hit].obclass == KEENOBJ {
        g.kill_keen();
    }
}

/// T_RoboShoot (0x226a1): fires temp1 shots, recoiling each time.
pub fn t_robored_shoot(g: &mut Game, ob: usize) {
    g.objs[ob].temp1 -= 1;
    if g.objs[ob].temp1 == 0 {
        g.objs[ob].state = Some(&S_ROBORED);
    }
    let (ox, oy, xdir, t1) = {
        let o = &g.objs[ob];
        (o.x, o.y, o.xdir, o.temp1)
    };
    let x = if xdir == 1 { ox + 0x380 } else { ox };
    if let Some(n) = spawn_enemy_shot(g, x, oy + 0x200, &S_ROBOSHOT1) {
        g.objs[n].xspeed = xdir * 60;
        g.objs[n].yspeed = if t1 & 1 != 0 { 8 } else { -8 };
        g.play_sound(SND_ROBOSHOOT);
        g.ph.xtry = if xdir == 1 { -0x40 } else { 0x40 };
    }
}

/// C_RoboShot (0x22726)
pub fn c_robo_shot(g: &mut Game, ob: usize, hit: usize) {
    if g.objs[hit].obclass == KEENOBJ {
        g.kill_keen();
        g.change_state(ob, &S_ROBOSHOTEXPLODE1);
    }
}

/// R_RoboShot (0x22749)
pub fn r_robo_shot(g: &mut Game, ob: usize) {
    if any_hit(g, ob) {
        g.play_sound(SND_ROBOSHOTHIT);
        g.change_state(ob, &S_ROBOSHOTEXPLODE1);
    }
    g.place_sprite(ob);
}

// ===========================================================================
// Spirogrip
// ===========================================================================

/// SpawnSpirogrip (0x227a1)
pub fn spawn_spirogrip(g: &mut Game, x: i32, y: i32) {
    let n = g.get_new_obj(false);
    {
        let o = &mut g.objs[n];
        o.obclass = SPIROGRIPOBJ;
        o.active = AC_YES;
        o.x = x << 8;
        o.y = (y << 8) - 0x100;
        o.xdir = 1;
        o.ydir = 1;
    }
    g.new_state(n, &S_SPIROGRIPDOWN1);
}
pub use self::spawn_spirogrip as spawn_227a1;

/// T_SpirogripSpin (0x227ea): sometimes launches in the current direction.
pub fn t_spirogrip_spin(g: &mut Game, ob: usize) {
    if g.rnd() > 0x14 {
        return;
    }
    g.play_sound(SND_SPIROGRIPSPIN);
    let st = g.objs[ob].state;
    if same(st, &S_SPIROGRIPSPIN2) {
        g.objs[ob].state = Some(&S_SPIROGRIPFLYUP);
    } else if same(st, &S_SPIROGRIPSPIN4) {
        g.objs[ob].state = Some(&S_SPIROGRIPFLYRIGHT);
    } else if same(st, &S_SPIROGRIPSPIN6) {
        g.objs[ob].state = Some(&S_SPIROGRIPFLYDOWN);
    } else if same(st, &S_SPIROGRIPSPIN8) {
        g.objs[ob].state = Some(&S_SPIROGRIPFLYLEFT);
    }
}

/// R_SpirogripFlying (0x22843): attaches to the wall it hits.
pub fn r_spirogrip_flying(g: &mut Game, ob: usize) {
    g.place_sprite(ob);
    if any_hit(g, ob) {
        if let Some(next) = g.objs[ob].st().next {
            g.change_state(ob, next);
        }
        g.play_sound(SND_SPIROGRIPLAND);
    }
}

// ===========================================================================
// Spindred
// ===========================================================================

/// SpawnSpindred (0x2289d)
pub fn spawn_spindred(g: &mut Game, x: i32, y: i32) {
    let n = g.get_new_obj(false);
    {
        let o = &mut g.objs[n];
        o.obclass = SPINDREDOBJ;
        o.active = AC_YES;
        o.priority = 0;
        o.x = x << 8;
        o.y = (y << 8) - 0x80;
        o.ydir = 1;
    }
    g.new_state(n, &S_SPINDRED1);
}
pub use self::spawn_spindred as spawn_2289d;

/// T_Spindred (0x228e6): gravity in the direction of ydir.
pub fn t_spindred(g: &mut Game, ob: usize) {
    let last = g.lasttimecount as i64;
    for i in (last - g.tics as i64)..last {
        let o = &mut g.objs[ob];
        if i & 1 != 0 {
            if o.ydir == 1 {
                if o.yspeed < 0 && o.yspeed >= -3 {
                    g.ph.ytry += o.yspeed;
                    o.yspeed = 0;
                    return;
                }
                o.yspeed += 3;
                if o.yspeed > 70 {
                    o.yspeed = 70;
                }
            } else {
                if o.yspeed > 0 && o.yspeed <= 3 {
                    g.ph.ytry += o.yspeed;
                    o.yspeed = 0;
                    return;
                }
                o.yspeed -= 3;
                if o.yspeed < -70 {
                    o.yspeed = -70;
                }
            }
        }
        g.ph.ytry += o.yspeed;
    }
}

/// C_Spindred (0x229a1): absorbs shots, kills Keen (also used by many other
/// act3 states).
pub fn c_spindred(g: &mut Game, _ob: usize, hit: usize) {
    if g.objs[hit].obclass == STUNSHOTOBJ {
        g.explode_shot(hit);
    } else if g.objs[hit].obclass == KEENOBJ {
        g.kill_keen();
    }
}

/// R_Spindred (0x229c6): bounces; the third bounce reverses gravity.
pub fn r_spindred(g: &mut Game, ob: usize) {
    if g.objs[ob].hitsouth != 0 {
        g.objs[ob].yspeed = 0;
        if g.objs[ob].ydir == -1 {
            g.objs[ob].temp1 += 1;
            if g.objs[ob].temp1 == 3 {
                let o = &mut g.objs[ob];
                o.temp1 = 0;
                o.yspeed = 0x44;
                o.ydir = -o.ydir;
                g.play_sound(SND_SPINDREDSLAM);
            } else {
                g.play_sound(SND_SPINDREDBOUNCE);
                g.objs[ob].yspeed = 0x28;
            }
        }
    }
    if g.objs[ob].hitnorth != 0 {
        g.objs[ob].yspeed = 0;
        if g.objs[ob].ydir == 1 {
            g.objs[ob].temp1 += 1;
            if g.objs[ob].temp1 == 3 {
                let o = &mut g.objs[ob];
                o.temp1 = 0;
                o.yspeed = -0x44;
                o.ydir = -o.ydir;
                g.play_sound(SND_SPINDREDFLYUP);
            } else {
                g.play_sound(SND_SPINDREDBOUNCE);
                g.objs[ob].yspeed = -0x28;
            }
        }
    }
    g.place_sprite(ob);
}

// ===========================================================================
// Shikadi master
// ===========================================================================

/// SpawnMaster (0x22a87)
pub fn spawn_master(g: &mut Game, x: i32, y: i32) {
    let n = g.get_new_obj(false);
    {
        let o = &mut g.objs[n];
        o.obclass = SHIKADIMASTEROBJ;
        o.active = AC_YES;
        o.x = x << 8;
        o.y = (y << 8) - 0x180;
    }
    g.new_state(n, &S_MASTER1);
}
pub use self::spawn_master as spawn_22a87;

/// T_Master (0x22ac6): alternates between shooting and teleporting.
pub fn t_master(g: &mut Game, ob: usize) {
    if g.rnd() < 0x40 {
        if g.objs[ob].temp1 != 0 {
            g.objs[ob].state = Some(&S_MASTERTELEPORT1);
            g.objs[ob].temp1 = 0;
            return;
        }
        g.objs[ob].temp1 = 1;
        g.objs[ob].xdir = dir_to_player(g, ob);
        g.objs[ob].state = Some(&S_MASTERSHOOT1);
    }
}

/// T_MasterShoot (0x22b1a)
pub fn t_master_shoot(g: &mut Game, ob: usize) {
    let (ox, oy, xdir) = (g.objs[ob].x, g.objs[ob].y, g.objs[ob].xdir);
    let x = if xdir == 1 { ox + 0x100 } else { ox };
    if let Some(n) = spawn_enemy_shot(g, x, oy + 0x80, &S_MASTERBALL1) {
        g.objs[n].xspeed = xdir * 48;
        g.objs[n].yspeed = -16;
        g.play_sound(SND_MASTERATTACK);
    }
}

/// C_Master (0x22b72): a shot makes it fire back.
pub fn c_master(g: &mut Game, ob: usize, hit: usize) {
    if g.objs[hit].obclass == STUNSHOTOBJ {
        g.explode_shot(hit);
        g.objs[ob].xdir = dir_to_player(g, ob);
        g.objs[ob].temp1 = 1;
        g.change_state(ob, &S_MASTERSHOOT1);
    } else if g.objs[hit].obclass == KEENOBJ {
        g.kill_keen();
    }
}

/// Two sparks flying left and right (master teleport / master ball landing).
fn spawn_master_spark(g: &mut Game, ob: usize, xspeed: i32) {
    let n = g.get_new_obj(true);
    g.objs[n].x = g.objs[ob].x;
    g.objs[n].y = g.objs[ob].y;
    g.objs[n].xspeed = xspeed;
    g.new_state(n, &S_ORBATRIXBOUNCE1);
}

/// T_MasterTPort (0x22bc6): teleports to a free spot near Keen.
pub fn t_master_teleport(g: &mut Game, ob: usize) {
    let (oldx, oldy) = (g.objs[ob].x, g.objs[ob].y);
    spawn_master_spark(g, ob, 48);
    spawn_master_spark(g, ob, -48);
    g.play_sound(SND_MASTERTELEPORT);
    let mut count = 0;
    'retry: loop {
        count += 1;
        if count == 10 {
            g.rnd();
            let o = &mut g.objs[ob];
            o.state = Some(&S_MASTER1);
            o.x = oldx - 1;
            o.y = oldy;
            g.ph.xtry = 1;
            g.ph.ytry = 0;
            return;
        }
        let p = g.player;
        let tx = g.objs[p].tilemidx - 16 + (g.rnd() as i32 >> 3);
        let ty = g.objs[p].tilebottom - 16 + (g.rnd() as i32 >> 3);
        if tx < 2 || ty < 2 || u(g.mapwidth - 5) < u(tx) || u(g.mapheight - 5) < u(ty) {
            continue;
        }
        {
            let o = &mut g.objs[ob];
            o.x = tx << 8;
            o.y = ty << 8;
            o.tileleft = tx - 1;
            o.tileright = tx + 4;
            o.tiletop = ty - 1;
            o.tilebottom = ty + 4;
        }
        let (l, t, r, b) = {
            let o = &g.objs[ob];
            (o.tileleft, o.tiletop, o.tileright, o.tilebottom)
        };
        for y in t..=b {
            for x in l..=r {
                let tile = fg_at(g, x, y);
                if g.intile(tile) & 0x80 != 0 || any_wall(g, tile) {
                    continue 'retry;
                }
            }
        }
        g.ph.ytry = 0;
        g.ph.xtry = 0;
        return;
    }
}

/// C_MasterBall (0x22db3)
pub fn c_master_ball(g: &mut Game, ob: usize, hit: usize) {
    if g.objs[hit].obclass == KEENOBJ {
        g.kill_keen();
        g.remove_obj(ob);
    } else if g.objs[hit].obclass == STUNSHOTOBJ {
        g.explode_shot(hit);
        g.remove_obj(ob);
    }
}

/// R_MasterBall (0x22df0): bursts into sparks when it lands.
pub fn r_master_ball(g: &mut Game, ob: usize) {
    {
        let o = &mut g.objs[ob];
        if o.hiteast != 0 || o.hitwest != 0 {
            o.xspeed = -o.xspeed;
        }
        if o.hitsouth != 0 {
            o.yspeed = 0;
        }
    }
    if g.objs[ob].hitnorth != 0 {
        g.play_sound(SND_MASTERATTACK);
        g.objs[ob].xspeed = 48;
        g.change_state(ob, &S_ORBATRIXBOUNCE1);
        spawn_master_spark(g, ob, -48);
    }
    g.place_sprite(ob);
}

/// R_MasterSparks (0x22e88): vanish at walls.
pub fn r_master_sparks(g: &mut Game, ob: usize) {
    if g.objs[ob].hiteast != 0 || g.objs[ob].hitwest != 0 {
        g.remove_obj(ob);
        return;
    }
    g.place_sprite(ob);
}

// ===========================================================================
// Shikadi (temp1 = pole column, temp2 = health, temp3 = white flash)
// ===========================================================================

/// SpawnShikadi (0x22ec7)
pub fn spawn_shikadi(g: &mut Game, x: i32, y: i32) {
    let n = g.get_new_obj(false);
    {
        let o = &mut g.objs[n];
        o.obclass = SHIKADIOBJ;
        o.active = AC_YES;
        o.x = x << 8;
        o.y = (y << 8) - 0x100;
        o.temp2 = 4;
    }
    g.objs[n].xdir = rnd_dir(g);
    g.new_state(n, &S_FLECTTURN);
}
pub use self::spawn_shikadi as spawn_22ec7;

/// T_Shikadi (0x22f2c): walks; zaps a pole next to it when Keen is on it
/// or on its level.
pub fn t_shikadi(g: &mut Game, ob: usize) {
    let p = g.player;
    let onpole = g.objs[p]
        .st()
        .contact
        .is_some_and(|f| std::ptr::fn_addr_eq(f, keen::keen_pos_contact as ContactFn));
    let poletx;
    if onpole || u(g.objs[ob].bottom - g.objs[p].bottom + 0x100) <= 0x200 {
        if u(g.objs[ob].x) > u(g.objs[p].x + 0x100) {
            g.objs[ob].xdir = -1;
        } else if u(g.objs[ob].x) < u(g.objs[p].x - 0x100) {
            g.objs[ob].xdir = 1;
        }
        poletx = if g.objs[ob].xdir == 1 { g.objs[ob].tileright } else { g.objs[ob].tileleft };
        if g.objs[p].tilemidx != poletx {
            return;
        }
    } else {
        if g.rnd() < 0x10 {
            g.ph.xtry = 0;
            g.objs[ob].state = Some(&S_FLECTTURN);
            return;
        }
        if u(g.objs[ob].x) & 0xff != 0 || !g.on_screen(ob) {
            return;
        }
        poletx = if g.objs[ob].xdir == 1 { g.objs[ob].tileright } else { g.objs[ob].tileleft };
    }
    let tile = fg_at(g, poletx, g.objs[ob].tiletop);
    if g.intile(tile) == 1 {
        g.objs[ob].temp1 = poletx;
        g.objs[ob].state = Some(&S_SHIKADIPOLEZAP1);
        g.ph.xtry = 0;
        g.play_sound(SND_SHIKADIATTACK);
    }
}

/// C_Shikadi (0x23043): takes four shots.
pub fn c_shikadi(g: &mut Game, ob: usize, hit: usize) {
    if g.objs[hit].obclass == KEENOBJ {
        g.kill_keen();
    }
    if g.objs[hit].obclass == STUNSHOTOBJ {
        g.objs[ob].temp2 -= 1;
        if g.objs[ob].temp2 == 0 {
            g.objs[ob].xspeed = 0;
            g.objs[ob].yspeed = 0;
            g.stun_obj(ob, hit, &S_FLEEXSTUN);
            return;
        }
        g.objs[ob].temp3 = 2;
        g.objs[ob].needtoreact = true;
        g.explode_shot(hit);
    }
}

/// T_PoleShock (0x23095): sends a shock along the pole towards Keen.
pub fn t_shikadi_pole_zap(g: &mut Game, ob: usize) {
    g.objs[ob].nothink = 2;
    let x = (if g.objs[ob].xdir == 1 { g.objs[ob].tileright } else { g.objs[ob].tileleft }) << 8;
    let n = g.get_new_obj(true);
    {
        let y = g.objs[ob].y + 0x80;
        let s = &mut g.objs[n];
        s.x = x;
        s.y = y;
        s.obclass = MSHOTOBJ;
        s.active = AC_REMOVABLE;
        s.needtoclip = CL_NOCLIP;
    }
    g.new_state(n, &S_POLESHOCK1);
    g.objs[n].ydir = if u(g.objs[ob].y) < u(g.objs[g.player].y) { 1 } else { -1 };
    g.play_sound(SND_SHIKADIATTACK);
}

/// T_PoleShock (0x23123): the shock moves along the pole until it ends.
pub fn t_pole_shock(g: &mut Game, ob: usize) {
    if g.ph.ytry == 0 {
        g.ph.ytry = g.objs[ob].ydir * 48;
        let tile = fg_at(g, g.objs[ob].tilemidx, g.objs[ob].tiletop);
        if g.intile(tile) != 1 {
            g.objs[ob].state = None;
        }
    }
}

/// Turns a walker around after a wall/ledge (common part of R_Shikadi and
/// R_Shockshund).
fn turn_around(g: &mut Game, ob: usize, newdir: i32, xmovefactor: i32) {
    {
        let o = &mut g.objs[ob];
        o.x -= o.xmove * xmovefactor;
        o.xdir = newdir;
    }
    g.objs[ob].nothink = g.rnd() as i32 >> 5;
    let st = g.objs[ob].st();
    g.change_state(ob, st);
}

/// R_Shikadi (0x23178)
pub fn r_shikadi(g: &mut Game, ob: usize) {
    let (xdir, hn, he, hw) = {
        let o = &g.objs[ob];
        (o.xdir, o.hitnorth, o.hiteast, o.hitwest)
    };
    if xdir == 1 && hw != 0 {
        turn_around(g, ob, -1, 1);
    } else if xdir == -1 && he != 0 {
        turn_around(g, ob, 1, 1);
    } else if hn == 0 {
        turn_around(g, ob, -xdir, 1);
    }
    draw_flash(g, ob);
}

// ===========================================================================
// Shockshund (temp1 = sit timer, temp2 = health, temp3 = white flash)
// ===========================================================================

/// SpawnShockshund (0x23257)
pub fn spawn_shockshund(g: &mut Game, x: i32, y: i32) {
    let n = g.get_new_obj(false);
    {
        let o = &mut g.objs[n];
        o.obclass = SHOCKSHUNDOBJ;
        o.active = AC_YES;
        o.x = x << 8;
        o.y = (y << 8) - 0x80;
        o.temp2 = 2;
        o.ydir = 1;
        o.xdir = 1;
    }
    g.new_state(n, &S_SHOCKSHUNDWALK1);
}
pub use self::spawn_shockshund as spawn_23257;

fn shockshund_jump(g: &mut Game, ob: usize) {
    let o = &mut g.objs[ob];
    o.xspeed = if o.xdir == 1 { 40 } else { -40 };
    o.yspeed = -40;
}

/// T_Shockshund (0x232a4)
pub fn t_shockshund(g: &mut Game, ob: usize) {
    g.objs[ob].xdir = if u(g.objs[ob].x) > u(g.objs[g.player].x) { -1 } else { 1 };
    if g.rnd() < 0x10 {
        g.ph.xtry = 0;
        g.objs[ob].state = Some(&S_SHOCKSHUNDSIT1);
        g.objs[ob].temp1 = 10;
        return;
    }
    if g.objs[ob].bottom != g.objs[g.player].bottom {
        g.objs[ob].state = Some(&S_SHOCKSHUNDJUMP);
        shockshund_jump(g, ob);
    }
    if g.rnd() < 0x80 {
        g.ph.xtry = 0;
        g.objs[ob].state = Some(&S_SHOCKSHUNDBARK1);
    }
}

/// T_Shockshund sit (0x23320)
pub fn t_shockshund_sit(g: &mut Game, ob: usize) {
    g.objs[ob].temp1 -= 1;
    if g.objs[ob].temp1 == 0 {
        g.objs[ob].state = Some(&S_SHOCKSHUNDWALK1);
    }
}

/// T_BarkShoot (0x23334)
pub fn t_shockshund_bark(g: &mut Game, ob: usize) {
    let (ox, oy, xdir) = (g.objs[ob].x, g.objs[ob].y, g.objs[ob].xdir);
    let x = if xdir == 1 { ox + 0x70 } else { ox };
    if let Some(n) = spawn_enemy_shot(g, x, oy + 0x40, &S_BARKSHOT1) {
        g.objs[n].xspeed = xdir * 60;
        g.objs[n].xdir = xdir;
        g.play_sound(SND_SHOCKSHUNDBARK);
    }
}

/// C_Shockshund (0x2338d): takes two shots.
pub fn c_shockshund(g: &mut Game, ob: usize, hit: usize) {
    if g.objs[hit].obclass == KEENOBJ {
        g.kill_keen();
    }
    if g.objs[hit].obclass == STUNSHOTOBJ {
        g.objs[ob].temp2 -= 1;
        if g.objs[ob].temp2 == 0 {
            g.objs[ob].xspeed = 0;
            g.objs[ob].yspeed = 0;
            g.stun_obj(ob, hit, &S_SHOCKSHUNDSTUN);
            return;
        }
        g.objs[ob].temp3 = 2;
        g.objs[ob].needtoreact = true;
        g.explode_shot(hit);
    }
}

/// R_Shockshund (0x233df): jumps off ledges towards Keen.
pub fn r_shockshund(g: &mut Game, ob: usize) {
    let (xdir, hn, he, hw) = {
        let o = &g.objs[ob];
        (o.xdir, o.hitnorth, o.hiteast, o.hitwest)
    };
    if xdir == 1 && hw != 0 {
        turn_around(g, ob, -1, 1);
    } else if xdir == -1 && he != 0 {
        turn_around(g, ob, 1, 1);
    } else if hn == 0 {
        let (ox, px) = (u(g.objs[ob].x), u(g.objs[g.player].x));
        if (xdir == 1 && px > ox) || (xdir == -1 && px < ox) {
            g.change_state(ob, &S_SHOCKSHUNDJUMP);
            shockshund_jump(g, ob);
        } else {
            turn_around(g, ob, -xdir, 2);
        }
    }
    draw_flash(g, ob);
}

/// R_ShockshundJump (0x2350b)
pub fn r_shockshund_jump(g: &mut Game, ob: usize) {
    if g.objs[ob].hitnorth != 0 {
        g.objs[ob].nothink = g.rnd() as i32 >> 5;
        g.change_state(ob, &S_SHOCKSHUNDWALK1);
    }
    draw_flash(g, ob);
}

/// C_BarkShot (0x2357b)
pub fn c_bark_shot(g: &mut Game, ob: usize, hit: usize) {
    if g.objs[hit].obclass == KEENOBJ {
        g.kill_keen();
        g.change_state(ob, &S_CEILICKATTACK11);
    } else if g.objs[hit].obclass == STUNSHOTOBJ {
        g.explode_shot(hit);
        g.change_state(ob, &S_CEILICKATTACK11);
    }
}

/// R_BarkShot (0x235c0)
pub fn r_bark_shot(g: &mut Game, ob: usize) {
    if any_hit(g, ob) {
        g.play_sound(SND_BARKSHOTDIE);
        g.change_state(ob, &S_CEILICKATTACK11);
    }
    g.place_sprite(ob);
}

// ===========================================================================
// Sphereful (temp1..temp4 = sprites of the four orbiting diamonds)
// ===========================================================================

/// SpawnSphereful (0x23618)
pub fn spawn_sphereful(g: &mut Game, x: i32, y: i32) {
    let n = g.get_new_obj(false);
    {
        let o = &mut g.objs[n];
        o.obclass = SPHEREFULOBJ;
        o.needtoclip = CL_FULLCLIP;
        o.active = AC_YES;
        o.x = x << 8;
        o.y = (y << 8) - 0x100;
        o.priority = 1;
    }
    g.new_state(n, &S_SPHEREFUL1);
}
pub use self::spawn_sphereful as spawn_23618;

/// T_Sphereful (0x23661): floats towards Keen.
pub fn t_sphereful(g: &mut Game, ob: usize) {
    g.objs[ob].needtoreact = true;
    if g.ph.xtry != 0 || g.ph.ytry != 0 {
        return;
    }
    let px = u(g.objs[g.player].x);
    let last = g.lasttimecount as i64;
    for i in (last - g.tics as i64)..last {
        let o = &mut g.objs[ob];
        if i & 0xf == 0 {
            if o.yspeed < 8 {
                o.yspeed += 1;
            }
            if u(o.x) < px && o.xspeed < 8 {
                o.xspeed += 1;
            }
            if u(o.x) > px && o.xspeed > -8 {
                o.xspeed -= 1;
            }
        }
        g.ph.ytry += o.yspeed;
        g.ph.xtry += o.xspeed;
    }
}

/// R_Sphereful (0x23704): bounces and draws the orbiting diamonds.
pub fn r_sphereful(g: &mut Game, ob: usize) {
    if g.objs[ob].hitwest != 0 || g.objs[ob].hiteast != 0 {
        g.objs[ob].xspeed = -g.objs[ob].xspeed;
        g.play_sound(SND_SPHEREFULBOUNCE);
    }
    if g.objs[ob].hitsouth != 0 {
        g.objs[ob].yspeed = -g.objs[ob].yspeed;
        g.play_sound(SND_SPHEREFULBOUNCE);
    }
    if g.objs[ob].hitnorth != 0 {
        let py = u(g.objs[g.player].y);
        let o = &mut g.objs[ob];
        o.yspeed = -o.yspeed;
        if py < u(o.y) {
            o.yspeed -= 1;
        } else {
            o.yspeed += 1;
        }
        if o.yspeed > -4 {
            o.yspeed = -4;
        } else if o.yspeed < -12 {
            o.yspeed = -12;
        }
        g.play_sound(SND_SPHEREFULCEILING);
    }
    g.place_sprite(ob);
    let (x, y) = (g.objs[ob].x, g.objs[ob].y);
    let mut t = ((g.lasttimecount as u16) >> 3 & 0xf) as usize;
    let shape = (t as i32 >> 2) + SPHEREFULDIAMONDSPR;
    let mut pr = if t < 8 { 0 } else { 2 };
    place_extra_sprite(g, ob, 0, x + DIAMONDPOS[t], y + DIAMONDPOS[t], shape, pr);
    place_extra_sprite(g, ob, 1, x + 0x180 - DIAMONDPOS[t], y + DIAMONDPOS[t], shape, pr);
    t = (t + 8) & 0xf;
    pr = if t < 8 { 0 } else { 2 };
    place_extra_sprite(g, ob, 2, x + DIAMONDPOS[t], y + DIAMONDPOS[t], shape, pr);
    g.place_stars(ob, x + 0x180 - DIAMONDPOS[t], y + DIAMONDPOS[t], shape, pr);
}

// ===========================================================================
// Korath
// ===========================================================================

/// SpawnKorath (0x238c1)
pub fn spawn_korath(g: &mut Game, x: i32, y: i32) {
    let n = g.get_new_obj(false);
    {
        let o = &mut g.objs[n];
        o.obclass = KORATHOBJ;
        o.active = AC_YES;
        o.x = x << 8;
        o.y = (y << 8) - 0x80;
    }
    g.objs[n].xdir = rnd_dir(g);
    g.new_state(n, &S_KORATHWALK1);
}
pub use self::spawn_korath as spawn_238c1;

/// T_Korath (0x23921): sometimes stops and picks a new direction.
pub fn t_korath(g: &mut Game, ob: usize) {
    if g.rnd() < 0x10 {
        g.ph.xtry = 0;
        g.objs[ob].xdir = rnd_dir(g);
        g.objs[ob].state = Some(&S_KORATHWAIT);
    }
}

/// C_Korath (0x23956): pushes Keen, can be stunned.
pub fn c_korath(g: &mut Game, ob: usize, hit: usize) {
    if g.objs[hit].obclass == KEENOBJ && g.objs[hit].st().contact.is_some() {
        g.clip_to_sprite_side(hit, ob);
        return;
    }
    if g.objs[hit].obclass == STUNSHOTOBJ {
        g.stun_obj(ob, hit, &S_KORATHSTUN);
    }
}

// ===========================================================================
// QED (Quantum Explosion Dynamo, destroyed by a mine piece)
// ===========================================================================

/// SpawnQED (0x23996): an invisible 2x2 tile target.
pub fn spawn_qed(g: &mut Game, x: i32, y: i32) {
    let n = g.get_new_obj(false);
    {
        let o = &mut g.objs[n];
        o.obclass = QEDOBJ;
        o.active = AC_YES;
        o.tileleft = x;
        o.tiletop = y;
        o.tileright = o.tileleft + 1;
        o.tilebottom = o.tiletop + 1;
        o.left = (x << 8) - 0x10;
        o.x = o.left;
        o.top = (y << 8) - 0x10;
        o.y = o.top;
        o.right = o.left + 0x220;
        o.bottom = o.top + 0x220;
    }
    g.new_state(n, &S_QED);
}
pub use self::spawn_qed as spawn_23996;
