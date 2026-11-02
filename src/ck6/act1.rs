//! CK6_ACT1.C port: bonus items, Grabbiter, rocket, grappling spots,
//! satellite, special items, Molly, platforms and Bloogs.
//! Translated from the decompiled segment 0x1e55 (linear 0x1e558..0x200d0)
//! of KEEN6C.EXE.

use super::defs::*;
use super::phys::u16shr;
use super::states::*;
use super::Game;

// sounds used in this module
const SND_BLOOGLETGEM: i32 = 5;
const SND_NOWAY: i32 = 14;
const SND_SATELLITE: i32 = 33;
const SND_BLOOGUARDSTOMP: i32 = 52;
const SND_THROWROPE: i32 = 53;
const SND_ROCKETFLY: i32 = 54;
const SND_ROCKETLAUNCH: i32 = 56;
const SND_GRABBITER: i32 = 57;

/// picture chunk with Keen's face used in the message windows
const KEENTALKPIC: i32 = 0x23;

/// first direction arrow in the info plane
const DIRARROWSTART: i32 = 0x5b;
/// platform stop block in the info plane
const PLATFORMBLOCK: usize = 0x1f;

/// DS:0x1916 x direction per arrow (9th entry overlaps PDIRY[0], as in the exe)
const PDIRX: [i32; 9] = [0, 1, 0, -1, 1, 1, -1, -1, -1];
/// DS:0x1926 y direction per arrow
const PDIRY: [i32; 9] = [-1, 0, 1, 0, -1, 1, 1, -1, 0];

/// DS:0x1a44 bonus item shapes (temp2 = first frame, temp3 = temp2 + 2)
const BONUSSHAPE: [i32; 12] = [164, 166, 168, 170, 150, 152, 154, 156, 158, 160, 162, 173];

/// DS:0x77c bounceangle[hitnorth][angle]; rows 8.. are whatever follows the
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
    [60, 52, 3, 1, 1, 0, 0, 0],
    [4613, 2607, 4825, 2607, 6092, 2607, 0, 66],
    [58, 3, 0, 1, 0, 0, 0, 4626],
    [2607, 4825, 2607, 6486, 2607, 0, 135, 133],
    [3, 0, 1, 0, 0, 0, 4639, 2607],
    [4825, 2607, 6486, 2607, 0, 109, 102, 3],
    [0, 1, 0, 0, 0, 4652, 2607, 4825],
    [2607, 6092, 2607, 0, 60, 52, 3, 0],
    [1, 4, 0, 32, 821, 2607, 4825, 2607],
    [6092, 2607, 2164, 68, 68, 3, 0, 1],
    [60, 0, 0, 821, 2607, 4825, 2607, 6092],
    [2607, 2164, 70, 70, 3, 0, 1, 90],
    [0, 0, 821, 2607, 4825, 2607, 6092, 2607],
    [2254, 69, 69, 3, 0, 1, 10, 0],
    [0, 821, 2607, 4825, 2607, 6092, 2607, 2284],
    [70, 70, 3, 0, 1, 90, 0, 0],
    [821, 2607, 4825, 2607, 6092, 2607, 2314, 69],
    [69, 3, 0, 1, 10, 0, 0, 821],
];

/// 16-bit signed wrap of a value
#[inline]
fn w16(v: i32) -> i32 {
    v as i16 as i32
}

/// Shows one of the "Keen talks" message windows (common code of the
/// Grabbiter, rocket door and grappling spot messages).
fn talk_window(g: &mut Game, text: &str, yadd: i32, before: Option<i32>, after: Option<i32>) {
    // CA_CacheGrChunk(KEENTALKPIC): graphics are always loaded
    if let Some(s) = before {
        g.play_sound(s);
    }
    g.draw_playfield();
    g.us_center_window(26, 8);
    let x = g.tw.windowx + g.tw.windoww - 0x30;
    let y = g.tw.windowy;
    g.draw_pic(x, y, KEENTALKPIC);
    g.tw.windoww -= 0x30;
    g.tw.printy += yadd;
    g.us_cprint(text);
    g.sys.present();
    if let Some(s) = after {
        g.play_sound(s);
    }
    g.sys.wait_vbl(30);
    g.sys.input.clear_keys_down();
    g.wait_ack();
}

/// Pushes Keen back to where he came from (after a "no way" message).
fn push_back(g: &mut Game, hit: usize) {
    g.ph.xtry = -g.objs[hit].xmove;
    g.ph.ytry = -g.objs[hit].ymove;
    g.objs[hit].xdir = 0;
    g.objs[hit].ydir = 0;
    g.clip_to_walls(hit);
}

// ---------------------------------------------------------------------------
// generic contact/react functions used by act2/act3 actors

/// C_ClipSide (0x1e558): Keen is pushed sideways by the object.
pub fn c_clip_side(g: &mut Game, ob: usize, hit: usize) {
    if g.objs[hit].obclass == KEENOBJ {
        g.ph.nowall = true;
        g.clip_to_sprite_side(hit, ob);
        g.ph.nowall = false;
    }
}

/// C_ClipTop (0x1e57f): Keen can stand on the object.
pub fn c_clip_top(g: &mut Game, ob: usize, hit: usize) {
    if g.objs[hit].obclass == KEENOBJ {
        g.clip_to_sprite_top(hit, ob);
    }
}

/// R_Land (0x1e59a): falls until it lands, then goes to the next state.
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

/// R_Bounce (0x1e60c): bounces off walls and slopes, loses speed.
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
        if let Some(next) = g.objs[ob].st().next {
            g.change_state(ob, next);
        }
    }
}

// ---------------------------------------------------------------------------
// bonus items

/// SpawnBonus (0x1e929)
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

/// SpawnSplash (0x1e993): the splash of a collected raindrop.
pub fn spawn_drop_splash(g: &mut Game, x: i32, y: i32) {
    let n = g.get_new_obj(true);
    let o = &mut g.objs[n];
    o.needtoclip = CL_NOCLIP;
    o.priority = 3;
    o.obclass = INERTOBJ;
    o.x = w16(x << 8);
    o.y = w16(y << 8);
    g.new_state(n, &S_SPLASH1);
}

/// T_Bonus (0x1e9d7): animates the item.
pub fn t_bonus(g: &mut Game, ob: usize) {
    let o = &mut g.objs[ob];
    o.shapenum += 1;
    if o.shapenum == o.temp3 {
        o.shapenum = o.temp2;
    }
}

/// T_FallingBonus (0x1e9f2): an item that falls until it lands.
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

// ---------------------------------------------------------------------------
// Grabbiter

/// SpawnGrabbiter (0x1ea20)
pub fn spawn_grabbiter(g: &mut Game, x: i32, y: i32) {
    let n = g.get_new_obj(false);
    let o = &mut g.objs[n];
    o.active = AC_YES;
    o.needtoclip = CL_NOCLIP;
    o.priority = 2;
    o.obclass = GRABBITEROBJ;
    o.x = w16(x << 8);
    o.y = w16(y << 8);
    let st = if g.gs.sandwichstate == 2 { &S_GRABBITER_NAPPING1 } else { &S_GRABBITER_HUNGRY1 };
    g.new_state(n, st);
}

/// C_Grabbiter (0x1ea74)
pub fn c_grabbiter(g: &mut Game, ob: usize, hit: usize) {
    match g.gs.sandwichstate {
        0 => {
            talk_window(
                g,
                "Oh, no!\nIt's a slavering\nGrabbiter! He says,\n\"Get me lunch and\nI'll tell ya a secret!\"",
                5,
                Some(SND_GRABBITER),
                Some(SND_NOWAY),
            );
            g.rf_force_refresh();
            push_back(g, hit);
        }
        1 => {
            g.gs.sandwichstate = 2;
            talk_window(
                g,
                "The Grabbiter grabs\nthe gigantic sandwich,\ndowns it in one bite,\nand says,\"Here's your\nsecret. Big meals\nmake me sleepy!\n",
                2,
                None,
                None,
            );
            g.change_state(ob, &S_GRABBITER_NAPPING1);
            g.rf_force_refresh();
        }
        _ => {}
    }
}

// ---------------------------------------------------------------------------
// Bloogstar rocket

/// SpawnRocket (0x1ebba)
pub fn spawn_rocket(g: &mut Game, x: i32, y: i32, dir: i32) {
    if g.gs.rocketstate != dir {
        return;
    }
    let n = g.get_new_obj(false);
    let o = &mut g.objs[n];
    o.active = AC_YES;
    o.needtoclip = CL_NOCLIP;
    o.priority = 3;
    o.obclass = ROCKETOBJ;
    o.x = w16(x << 8);
    o.y = w16(y << 8);
    g.new_state(n, &S_ROCKET);
}

/// T_RocketSit (0x1ec0a)
pub fn t_rocket_sit(g: &mut Game, ob: usize) {
    g.objs[ob].needtoreact = true;
}

/// C_Rocket (0x1ec19): Keen touches the parked rocket.
pub fn c_rocket(g: &mut Game, ob: usize, hit: usize) {
    match g.gs.passcardstate {
        0 => {
            talk_window(
                g,
                "The door makes a loud\nblooping noise.\nIt says,\n\"Passcard required\nfor entry.\"",
                5,
                None,
                Some(SND_NOWAY),
            );
            g.rf_force_refresh();
            push_back(g, hit);
        }
        1 => {
            g.objs[ob].temp1 = 0;
            g.objs[ob].temp2 = 0x100;
            g.change_state(ob, &S_ROCKETFLY1);
            g.objs[hit].x = g.objs[ob].x;
            g.objs[hit].y = w16(g.objs[ob].y + 0x100);
            g.objs[hit].needtoclip = CL_NOCLIP;
            g.change_state(hit, &S_ROCKETSIT);
            g.play_sound(SND_ROCKETLAUNCH);
            g.sd.wait_sound_done(&mut g.sys);
        }
        _ => {}
    }
}

/// C_RocketFly (0x1ed1e): keeps Keen inside the flying rocket.
pub fn c_rocket_fly(g: &mut Game, ob: usize, hit: usize) {
    if g.objs[hit].obclass == KEENOBJ {
        g.objs[hit].x = g.objs[ob].x;
        g.objs[hit].y = w16(g.objs[ob].y + 0x100);
        let st = g.objs[hit].st();
        g.change_state(hit, st);
    }
}

/// T_RocketFly (0x1ed4d): follows the arrows in the info plane.
pub fn t_rocket_fly(g: &mut Game, ob: usize) {
    if g.ph.xtry != 0 || g.ph.ytry != 0 {
        return;
    }
    if g.sd.sound_playing() == 0 {
        g.play_sound(SND_ROCKETFLY);
    }
    let mut mv = (g.tics << 5) & 0xffff;
    let t1 = (g.objs[ob].temp1 as usize).min(8);
    if (g.objs[ob].temp2 as u16 as i32) > mv {
        g.objs[ob].temp2 = w16(g.objs[ob].temp2 - mv);
        match PDIRX[t1] {
            1 => g.ph.xtry = w16(mv),
            -1 => g.ph.xtry = w16(-mv),
            _ => {}
        }
        match PDIRY[t1] {
            1 => g.ph.ytry = w16(mv),
            -1 => g.ph.ytry = w16(-mv),
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
    let tx = u16shr(g.objs[ob].x + g.ph.xtry, 8);
    let ty = u16shr(g.objs[ob].y + g.ph.ytry, 8);
    let t1 = g.map_tile(2, tx, ty) as i32 - DIRARROWSTART;
    g.objs[ob].temp1 = t1;
    if !(0..=8).contains(&t1) {
        // landed
        g.objs[ob].x = w16(g.objs[ob].x + g.ph.xtry);
        g.objs[ob].y = w16(g.objs[ob].y + g.ph.ytry);
        g.change_state(ob, &S_ROCKET);
        let p = g.player;
        {
            let o = &mut g.objs[p];
            o.x = w16(((tx + 1) << 8) + 0x10);
            o.y = w16((ty + 1) << 8);
            o.obclass = KEENOBJ;
            o.shapenum = 0xbd;
            o.needtoclip = CL_MIDCLIP;
        }
        g.new_state(p, &S_WORLDKEEN);
        g.gs.rocketstate ^= 1;
        return;
    }
    mv -= t2;
    g.objs[ob].temp2 = w16(0x100 - mv);
    let t1 = t1 as usize;
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
}

// ---------------------------------------------------------------------------
// grappling hook spots (world map cliffs)

/// SpawnGrappleSpot (0x1ef20): kind 0 = cliff top, 1 = cliff bottom.
pub fn spawn_grapple_spot(g: &mut Game, x: i32, y: i32, kind: i32) {
    let n = g.get_new_obj(false);
    {
        let o = &mut g.objs[n];
        o.active = AC_YES;
        o.needtoclip = CL_NOCLIP;
        o.priority = 2;
        o.obclass = GRAPPLEOBJ;
        o.tileleft = x;
        o.tileright = x;
        o.tiletop = y;
        o.tilebottom = y;
        o.temp1 = kind;
        o.left = w16(x << 8);
        o.x = o.left;
        o.right = w16(o.left + 0x100);
        let top = if kind == 0 { w16(y << 8) } else { w16(((y + 1) << 8) - 1) };
        o.top = top;
        o.y = top;
        o.bottom = w16(o.top + 1);
    }
    g.new_state(n, &S_GRAPPLESPOT);
    if g.gs.hookstate == 2 && kind != 0 {
        let r = g.get_new_obj(false);
        let o = &mut g.objs[r];
        o.active = AC_YES;
        o.needtoclip = CL_NOCLIP;
        o.priority = 0;
        o.obclass = INERTOBJ;
        o.x = w16(x << 8);
        o.y = w16((y + 1) << 8);
        g.new_state(r, &S_MAPROPE);
    }
}

/// T_ThrowRope (0x1f003): the rope is attached, Keen is back to normal.
pub fn t_throw_rope(g: &mut Game, ob: usize) {
    let n = g.get_new_obj(false);
    {
        let (tr, y) = (g.objs[ob].tileright, g.objs[ob].y);
        let o = &mut g.objs[n];
        o.active = AC_YES;
        o.needtoclip = CL_NOCLIP;
        o.priority = 0;
        o.obclass = INERTOBJ;
        o.x = w16(tr << 8);
        o.y = w16(y - 0x200);
    }
    g.new_state(n, &S_MAPROPE);
    g.objs[ob].obclass = KEENOBJ;
    g.objs[ob].shapenum = 0xc0;
}

/// T_ClimbRope (0x1f05a)
pub fn t_climb_rope(g: &mut Game, ob: usize) {
    g.objs[ob].temp4 -= 1;
    if g.objs[ob].temp4 != 0 {
        return;
    }
    {
        let o = &mut g.objs[ob];
        if o.ydir == 1 {
            o.y = w16(o.y + 0x30);
            o.shapenum = 0xc3;
        } else {
            o.y = w16(o.y - 0x30);
            o.shapenum = 0xc0;
        }
        o.obclass = KEENOBJ;
    }
    g.new_state(ob, &S_WORLDKEEN);
}

/// C_GrappleSpot (0x1f094)
pub fn c_grapple_spot(g: &mut Game, ob: usize, hit: usize) {
    if g.objs[hit].obclass != KEENOBJ {
        return;
    }
    match g.gs.hookstate {
        0 => {
            talk_window(
                g,
                "What a tall cliff!\nWish I had a rope\nand grappling hook.\n",
                0xf,
                None,
                Some(SND_NOWAY),
            );
            g.rf_force_refresh();
            push_back(g, hit);
            return;
        }
        1 => {
            g.gs.hookstate = 2;
            g.play_sound(SND_THROWROPE);
            g.change_state(hit, &S_THROWROPE1);
        }
        2 => {
            {
                let up = g.objs[ob].temp1 == 0;
                let o = &mut g.objs[hit];
                if up {
                    o.y = w16(o.y - 0x40);
                    o.temp4 = 6;
                    o.ydir = -1;
                } else {
                    o.y = w16(o.y + 0x40);
                    o.temp4 = 6;
                    o.ydir = 1;
                }
            }
            g.new_state(hit, &S_CLIMBROPE1);
        }
        _ => return,
    }
    g.objs[hit].obclass = INERTOBJ;
}

// ---------------------------------------------------------------------------
// satellite

/// SpawnSatelliteStop (0x1f1b1)
pub fn spawn_satellite_stop(g: &mut Game, x: i32, y: i32, kind: i32) {
    let n = g.get_new_obj(false);
    {
        let o = &mut g.objs[n];
        o.active = AC_ALLWAYS;
        o.needtoclip = CL_NOCLIP;
        o.priority = 2;
        o.obclass = SATELLITESTOPOBJ;
        o.tileleft = x;
        o.tileright = x;
        o.tiletop = y;
        o.tilebottom = y;
        o.temp1 = ((kind as u16 ^ 1) as i32) + 1;
        o.left = w16(x << 8);
        o.x = o.left;
        o.right = w16(o.left + 0x100);
        o.top = w16(y << 8);
        o.y = o.top;
        o.bottom = w16(o.top + 0x100);
    }
    g.new_state(n, &S_SATELLITESTOP);
}

/// SpawnSatellite (0x1f233)
pub fn spawn_satellite(g: &mut Game, x: i32, y: i32) {
    let n = g.get_new_obj(false);
    {
        let o = &mut g.objs[n];
        o.needtoclip = CL_NOCLIP;
        o.priority = 2;
        o.active = AC_ALLWAYS;
        o.obclass = SATELLITEOBJ;
        o.x = w16(x << 8);
        o.y = w16(y << 8);
        o.xdir = 0;
        o.ydir = 1;
    }
    g.new_state(n, &S_SATELLITE1);
    g.set_map_tile(2, x, y, 0x5e);
    let o = &mut g.objs[n];
    o.temp1 = 3;
    o.temp2 = 0x100;
    o.temp4 = 2;
}

/// T_Satellite (0x1f2c9): moves along the arrows like a GoPlat.
pub fn t_satellite(g: &mut Game, ob: usize) {
    if g.ph.xtry != 0 || g.ph.ytry != 0 {
        return;
    }
    {
        let o = &mut g.objs[ob];
        if o.temp3 & 0xff == 0 && o.temp3 & 0xff00 != 0 {
            o.temp4 = (o.temp3 as i16 >> 8) as i32;
        }
        o.temp3 = w16(o.temp3 << 8);
    }
    t_goplat(g, ob);
}

/// C_Satellite (0x1f306): picks up / drops Keen at the satellite stops.
pub fn c_satellite(g: &mut Game, ob: usize, hit: usize) {
    if same(g.objs[hit].state, &S_SATELLITESTOP) {
        g.objs[ob].temp3 |= g.objs[hit].temp1;
        return;
    }
    if g.objs[hit].obclass != KEENOBJ {
        return;
    }
    let stop = (g.objs[ob].temp3 as i16 >> 8) as i32;
    if stop == 0 || g.objs[ob].temp4 == stop {
        if same(g.objs[hit].state, &S_SATELLITEHOOK) {
            g.objs[hit].x = w16(g.objs[ob].x + 0xc0);
            g.objs[hit].y = w16(g.objs[ob].y + 0x100);
            let st = g.objs[hit].st();
            g.change_state(hit, st);
        }
        return;
    }
    g.play_sound(SND_SATELLITE);
    g.objs[ob].temp4 = stop;
    let p = g.player;
    if same(g.objs[p].state, &S_SATELLITEHOOK) {
        // drop Keen at the stop
        let mut i = g.objs[p].next;
        while let Some(n) = i {
            if g.objs[n].obclass == SATELLITESTOPOBJ && g.objs[n].temp1 == stop {
                g.objs[hit].x = g.objs[n].x;
                g.objs[hit].y = g.objs[n].y;
                g.objs[hit].shapenum = 0xc0;
                g.change_state(p, &S_WORLDKEEN);
                g.objs[hit].needtoclip = CL_MIDCLIP;
                return;
            }
            i = g.objs[n].next;
        }
    } else {
        // pick Keen up
        g.objs[hit].x = w16(g.objs[ob].x + 0xc0);
        g.objs[hit].y = w16(g.objs[ob].y + 0x100);
        g.objs[hit].needtoclip = CL_NOCLIP;
        g.change_state(p, &S_SATELLITEHOOK);
    }
}

/// R_SatelliteHook (0x1f408): draws Keen hanging below the satellite.
pub fn r_satellite_hook(g: &mut Game, ob: usize) {
    let (x, y) = (g.objs[ob].x + 0x40, g.objs[ob].y + 0x80);
    g.place_sprite_at(ob, x, y, 0xe3, 1);
}

// ---------------------------------------------------------------------------
// special items and Molly

fn spawn_special(g: &mut Game, x: i32, y: i32, class: i32, st: StateRef) {
    let n = g.get_new_obj(false);
    let o = &mut g.objs[n];
    o.needtoclip = CL_NOCLIP;
    o.priority = 2;
    o.obclass = class;
    o.x = w16(x << 8);
    o.y = w16(y << 8);
    g.new_state(n, st);
}

/// SpawnSandwich (0x1f439)
pub fn spawn_sandwich(g: &mut Game, x: i32, y: i32) {
    spawn_special(g, x, y, SANDWICHOBJ, &S_SANDWICH);
}

/// SpawnHook (0x1f47c)
pub fn spawn_hook(g: &mut Game, x: i32, y: i32) {
    spawn_special(g, x, y, HOOKOBJ, &S_HOOK);
}

/// SpawnPasscard (0x1f4bf)
pub fn spawn_passcard(g: &mut Game, x: i32, y: i32) {
    spawn_special(g, x, y, PASSCARDOBJ, &S_PASSCARD);
}

/// C_SpecialItem (0x1f502): touching a special item (or Molly) ends the level.
pub fn c_special_item(g: &mut Game, ob: usize, hit: usize) {
    if g.objs[hit].obclass != KEENOBJ {
        return;
    }
    match g.objs[ob].obclass {
        HOOKOBJ => g.playstate = EX_HOOK,
        MOLLYOBJ => g.playstate = EX_MOLLY,
        SANDWICHOBJ => g.playstate = EX_SANDWICH,
        PASSCARDOBJ => g.playstate = EX_CARD,
        _ => {}
    }
}

/// SpawnMolly (0x1f550)
pub fn spawn_molly(g: &mut Game, x: i32, y: i32) {
    let n = g.get_new_obj(false);
    {
        let o = &mut g.objs[n];
        o.obclass = MOLLYOBJ;
        o.active = AC_YES;
        o.priority = 0;
        o.x = w16(x << 8);
        o.y = w16((y << 8) - 0x80);
    }
    g.objs[n].xdir = if (g.rnd() as i32) < 0x80 { 1 } else { -1 };
    g.objs[n].ydir = 1;
    g.new_state(n, &S_MOLLY1);
}

// ---------------------------------------------------------------------------
// platforms

/// SpawnPlatform (0x1f5bd): dir 0 up, 1 right, 2 down, 3 left.
pub fn spawn_platform(g: &mut Game, x: i32, y: i32, dir: i32) {
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
    g.new_state(n, &S_PLATFORM);
}

/// T_Platform (0x1f654): moves until it hits a platform block, then turns.
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

/// SpawnDropPlat (0x1f823)
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

/// T_DropPlatSit (0x1f878): sinks a little while Keen stands on it.
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

/// T_DropPlatFall (0x1f8ab)
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

/// T_DropPlatRise (0x1f90f)
pub fn t_drop_plat_rise(g: &mut Game, ob: usize) {
    if g.gs.riding == Some(ob) {
        g.objs[ob].yspeed = 0;
        g.objs[ob].state = Some(&S_DROPPLATFALL);
    } else if (g.objs[ob].y as u16) <= (g.objs[ob].temp1 as u16) {
        g.ph.ytry = w16(g.objs[ob].temp1 - g.objs[ob].y);
        g.objs[ob].state = Some(&S_DROPPLATSIT);
    }
}

/// SpawnStaticPlat (0x1f941)
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

/// SpawnGoPlat (0x1f996): follows the direction arrows in the info plane.
pub fn spawn_goplat(g: &mut Game, x: i32, y: i32, dir: i32) {
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
    g.new_state(n, &S_GOPLAT);
    g.set_map_tile(2, x, y, (dir + DIRARROWSTART) as u16);
    g.objs[n].temp1 = dir;
    g.objs[n].temp2 = 0x100;
}

/// T_GoPlat (0x1fa1e): temp1 = arrow direction, temp2 = distance to the
/// next tile. Also used by the satellite.
pub fn t_goplat(g: &mut Game, ob: usize) {
    let tics = g.tics;
    let mv = (tics * 12) & 0xffff;
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
        1 => g.ph.xtry = w16(g.ph.xtry + t2),
        -1 => g.ph.xtry = w16(g.ph.xtry - t2),
        _ => {}
    }
    match PDIRY[t1] {
        1 => g.ph.ytry = w16(g.ph.ytry + t2),
        -1 => g.ph.ytry = w16(g.ph.ytry - t2),
        _ => {}
    }
    let tx = u16shr(g.objs[ob].x + g.ph.xtry, 8);
    let ty = u16shr(g.objs[ob].y + g.ph.ytry, 8);
    let t1 = g.map_tile(2, tx, ty) as i32 - DIRARROWSTART;
    g.objs[ob].temp1 = t1;
    if !(0..=8).contains(&t1) {
        g.quit("Goplat moved to a bad spot!");
        return;
    }
    let rest = mv - t2;
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

/// R_GoPlat (0x1fb94): draws the platform and its direction arrow (the
/// original keeps the arrow sprite in temp3; here it uses the stars slot).
pub fn r_goplat(g: &mut Game, ob: usize) {
    g.place_sprite(ob);
    let o = &g.objs[ob];
    let (x, y, shape, pr) = (o.x + 0x100, o.y + 0x100, o.temp1 + 0x1a9, o.priority);
    g.place_stars(ob, x, y, shape, pr);
}

/// SpawnSneakPlat (0x1fbe4)
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

/// T_SneakPlat (0x1fc39): dodges when Keen jumps towards it.
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
// Bloog, Blooguard, Blooglets

/// SpawnBloog (0x1fcab)
pub fn spawn_bloog(g: &mut Game, x: i32, y: i32) {
    let n = g.get_new_obj(false);
    {
        let o = &mut g.objs[n];
        o.obclass = BLOOGOBJ;
        o.active = AC_YES;
        o.priority = 0;
        o.x = w16(x << 8);
        o.y = w16((y << 8) - 0x200);
    }
    g.objs[n].xdir = if (g.rnd() as i32) < 0x80 { 1 } else { -1 };
    g.objs[n].ydir = 1;
    g.new_state(n, &S_BLOOGWALK1);
}

/// T_Bloog (0x1fd18): sometimes turns towards Keen.
pub fn t_bloog(g: &mut Game, ob: usize) {
    if (g.rnd() as i32) < 0x20 {
        let px = g.objs[g.player].x;
        g.objs[ob].xdir = if (g.objs[ob].x as u16) < (px as u16) { 1 } else { -1 };
    }
}

/// C_Bloog (0x1fd44)
pub fn c_bloog(g: &mut Game, ob: usize, hit: usize) {
    match g.objs[hit].obclass {
        KEENOBJ => g.kill_keen(),
        STUNSHOTOBJ => g.stun_obj(ob, hit, &S_BLOOGSTUN),
        _ => {}
    }
}

/// SpawnBlooguard (0x1fd6f)
pub fn spawn_blooguard(g: &mut Game, x: i32, y: i32) {
    let n = g.get_new_obj(false);
    {
        let o = &mut g.objs[n];
        o.obclass = BLOOGUARDOBJ;
        o.active = AC_YES;
        o.priority = 0;
        o.x = w16(x << 8);
        o.y = w16((y << 8) - 0x280);
    }
    g.objs[n].xdir = if (g.rnd() as i32) < 0x80 { 1 } else { -1 };
    g.objs[n].ydir = 1;
    g.objs[n].temp2 = 3;
    g.new_state(n, &S_BLOOGUARDWALK1);
}

/// T_BlooguardWalk (0x1fde1)
pub fn t_blooguard(g: &mut Game, ob: usize) {
    let p = g.player;
    if (g.rnd() as i32) < 0x20 {
        g.objs[ob].xdir = if (g.objs[ob].x as u16) < (g.objs[p].x as u16) { 1 } else { -1 };
    }
    let (ox, px) = (g.objs[ob].x as u16, g.objs[p].x as u16);
    let xdir = g.objs[ob].xdir;
    if ((xdir == 1 && ox < px) || (xdir == -1 && px < ox))
        && g.objs[ob].bottom == g.objs[p].bottom
        && (g.rnd() as i32) < 0x20
    {
        g.objs[ob].state = Some(&S_BLOOGUARDATTACK1);
    }
}

/// T_BlooguardAttack (0x1fe4c): the club hits the ground, the screen shakes
/// and Keen is stunned if he stands on the ground.
pub fn t_blooguard_attack(g: &mut Game, _ob: usize) {
    g.play_sound(SND_BLOOGUARDSTOMP);
    g.shaketime = 0x17;
    let p = g.player;
    if g.objs[p].hitnorth != 0 {
        g.change_state(p, &S_KEENSTUNNED);
    }
}

/// C_Blooguard (0x1fe7c): takes three shots to stun.
pub fn c_blooguard(g: &mut Game, ob: usize, hit: usize) {
    if g.objs[hit].obclass == KEENOBJ {
        g.kill_keen();
    }
    if g.objs[hit].obclass == STUNSHOTOBJ {
        g.objs[ob].temp2 -= 1;
        if g.objs[ob].temp2 == 0 {
            g.stun_obj(ob, hit, &S_BLOOGUARDSTUN);
        } else {
            g.objs[ob].temp1 = 2;
            g.objs[ob].needtoreact = true;
            g.explode_shot(hit);
        }
    }
}

/// R_Blooguard (0x1fec1): R_Walk that flashes while temp1 counts down.
/// The sprite is drawn white (maskdraw) while temp1 != 0.
pub fn r_blooguard(g: &mut Game, ob: usize) {
    let o = g.objs[ob].clone();
    let turned = if o.xdir == 1 && o.hitwest != 0 {
        g.objs[ob].x = w16(o.x - o.xmove);
        g.objs[ob].xdir = -1;
        true
    } else if o.xdir == -1 && o.hiteast != 0 {
        g.objs[ob].x = w16(o.x - o.xmove);
        g.objs[ob].xdir = 1;
        true
    } else if o.hitnorth == 0 {
        g.objs[ob].x = w16(o.x - o.xmove * 2);
        g.objs[ob].xdir = -o.xdir;
        true
    } else {
        false
    };
    if turned {
        g.objs[ob].nothink = (g.rnd() >> 5) as i32;
        let st = g.objs[ob].st();
        g.change_state(ob, st);
    }
    let white = g.objs[ob].temp1 != 0;
    if white {
        g.objs[ob].temp1 -= 1;
    }
    g.place_sprite(ob);
    g.set_sprite_white(ob, white);
}

/// DS:0x22ae stunned states of the four blooglet colours
const BLOOGLETSTUN: [&State; 4] = [&S_BLOOGLETRSTUN, &S_BLOOGLETYSTUN, &S_BLOOGLETBSTUN, &S_BLOOGLETGSTUN];

/// SpawnBlooglet (0x1ff59): kind & 3 = colour, kind > 3 carries a gem.
pub fn spawn_blooglet(g: &mut Game, x: i32, y: i32, kind: i32) {
    let n = g.get_new_obj(false);
    {
        let o = &mut g.objs[n];
        o.obclass = BLOOGLETOBJ;
        o.active = AC_YES;
        o.priority = 0;
        o.x = w16(x << 8);
        o.y = w16((y << 8) - 0x80);
    }
    g.objs[n].xdir = if (g.rnd() as i32) < 0x80 { 1 } else { -1 };
    g.objs[n].ydir = 1;
    g.objs[n].temp1 = kind;
    let st: StateRef = match kind % 4 {
        0 => &S_BLOOGLETRWALK1,
        1 => &S_BLOOGLETYWALK1,
        2 => &S_BLOOGLETBWALK1,
        3 => &S_BLOOGLETGWALK1,
        _ => return,
    };
    g.new_state(n, st);
}

/// C_Blooglet (0x1fffc): Keen can push blooglets; a shot stuns them and
/// releases their gem.
pub fn c_blooglet(g: &mut Game, ob: usize, hit: usize) {
    if g.objs[hit].obclass == KEENOBJ && g.objs[hit].st().contact.is_some() {
        g.ph.nowall = true;
        g.clip_to_sprite_side(hit, ob);
        g.ph.nowall = false;
        return;
    }
    if g.objs[hit].obclass != STUNSHOTOBJ {
        return;
    }
    let color = (g.objs[ob].temp1 & 3) as usize;
    if g.objs[ob].temp1 > 3 {
        let n = g.get_new_obj(false);
        {
            let (x, y) = (g.objs[ob].x, g.objs[ob].y);
            let o = &mut g.objs[n];
            o.needtoclip = CL_MIDCLIP;
            o.priority = 2;
            o.obclass = BONUSOBJ;
            o.x = x;
            o.y = y;
            o.ydir = -1;
            o.yspeed = -40;
            o.temp1 = color as i32;
            o.shapenum = BONUSSHAPE[color];
            o.temp2 = o.shapenum;
            o.temp3 = o.temp2 + 2;
        }
        g.new_state(n, &S_BONUS3);
        g.play_sound(SND_BLOOGLETGEM);
    }
    g.stun_obj(ob, hit, BLOOGLETSTUN[color]);
}

/// Module-private state stored in `Game::act1` (none needed).
#[derive(Default, Clone)]
pub struct ModState {}
