//! CK5 actors, part 2: Sparky, Little Ampton, Slicestar and Shelley.
//! Translated from the decompiled code segment 0x2102 of KEEN5E.EXE
//! (segment 0x1102 in the file, linear 0x21024 .. 0x21c40; SpawnMine at
//! 0x21c41 is already in the next segment 0x11c4, the mine code of act3).

use super::defs::*;
use super::phys::u16shr;
use super::states::*;
use super::Game;

// object classes (Keen 5 numbering)
/// enemy shot (Shelley explodes when one hits it)
const MSHOTOBJ: i32 = 4;
/// Sparky; SpawnShelley uses the same class
pub const SPARKYOBJ: i32 = 9;
pub const SLICESTAROBJ: i32 = 11;
pub const AMPTONOBJ: i32 = 14;

// sounds (indices into AUDIO.CK5), named after their use here
const SND_SPARKYRUN: i32 = 0;
const SND_SLICESTARBOUNCE: i32 = 6;
const SND_AMPTONWALK1: i32 = 31;
const SND_AMPTONWALK2: i32 = 32;
const SND_AMPTONSTUN: i32 = 33;
const SND_SHELLEYEXPLODE: i32 = 36;
const SND_SPARKYPREPCHARGE: i32 = 58;

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
// Sparky
// ===========================================================================

/// SpawnSparky (0x21024)
pub fn spawn_sparky(g: &mut Game, x: i32, y: i32) {
    let n = g.get_new_obj(false);
    {
        let o = &mut g.objs[n];
        o.obclass = SPARKYOBJ;
        o.active = AC_YES;
        o.priority = 0;
        o.x = x << 8;
        o.y = (y << 8) - 0x100;
    }
    g.objs[n].xdir = rnd_dir(g);
    g.objs[n].ydir = 1;
    g.new_state(n, &S_SPARKYWALK1);
}
pub use self::spawn_sparky as spawn_21024;

/// T_Sparky (0x2108f): stops now and then to look around.
pub fn t_sparky(g: &mut Game, ob: usize) {
    if g.rnd() < 0x40 {
        g.objs[ob].state = Some(&S_SPARKYLOOK1);
        g.ph.xtry = 0;
    }
}

/// T_ChargeCount (0x210ae): temp1 = speed up cycles before charging.
pub fn t_charge_count(g: &mut Game, ob: usize) {
    g.objs[ob].temp1 -= 1;
    if g.objs[ob].temp1 == 0 {
        g.objs[ob].state = Some(&S_SPARKYCHARGE1);
    }
}

/// T_SparkyLookL (0x210c2): charges left when Keen is on its level and left.
pub fn t_sparky_look_l(g: &mut Game, ob: usize) {
    let p = g.player;
    let dist = u(g.objs[p].bottom + 0x100 - g.objs[ob].bottom);
    if dist <= 0x200 && u(g.objs[p].x) < u(g.objs[ob].x) {
        g.objs[ob].xdir = -1;
        g.play_sound(SND_SPARKYPREPCHARGE);
        g.objs[ob].state = Some(&S_SPARKYSPEED1);
        g.objs[ob].temp1 = 3;
    }
}

/// T_SparkyLookR (0x2110f): charges right when Keen is on its level and right.
pub fn t_sparky_look_r(g: &mut Game, ob: usize) {
    let p = g.player;
    let dist = u(g.objs[p].bottom + 0x100 - g.objs[ob].bottom);
    if dist <= 0x200 && u(g.objs[p].x) > u(g.objs[ob].x) {
        g.objs[ob].xdir = 1;
        g.play_sound(SND_SPARKYPREPCHARGE);
        g.objs[ob].state = Some(&S_SPARKYSPEED1);
        g.objs[ob].temp1 = 3;
    }
}

/// T_RunSnd1 (0x2115c)
pub fn t_run_snd1(g: &mut Game, _ob: usize) {
    g.play_sound(SND_SPARKYRUN);
}

/// T_RunSnd2 (0x2116c)
pub fn t_run_snd2(g: &mut Game, _ob: usize) {
    g.play_sound(SND_SPARKYRUN);
}

/// C_Sparky (0x2117c)
pub fn c_sparky(g: &mut Game, ob: usize, hit: usize) {
    let hc = g.objs[hit].obclass;
    if hc == KEENOBJ {
        g.kill_keen();
    } else if hc == STUNSHOTOBJ {
        g.stun_obj(ob, hit, &S_SPARKYSTUN);
    }
}

/// R_Sparky (0x211a8): turns at walls and ledges, pausing a random time.
pub fn r_sparky(g: &mut Game, ob: usize) {
    let o = &g.objs[ob];
    let turn = if o.xdir == 1 && o.hitwest != 0 {
        Some(-1)
    } else if o.xdir == -1 && o.hiteast != 0 {
        Some(1)
    } else if o.hitnorth == 0 {
        Some(-o.xdir)
    } else {
        None
    };
    if let Some(xdir) = turn {
        let o = &mut g.objs[ob];
        o.x -= o.xmove;
        o.xdir = xdir;
        g.objs[ob].nothink = (g.rnd() >> 5) as i32;
        g.change_state(ob, &S_SPARKYTURN1);
    }
    g.place_sprite(ob);
}

// ===========================================================================
// Little Ampton
// ===========================================================================

/// SpawnAmpton (0x21260)
pub fn spawn_ampton(g: &mut Game, x: i32, y: i32) {
    let n = g.get_new_obj(false);
    {
        let o = &mut g.objs[n];
        o.obclass = AMPTONOBJ;
        o.active = AC_YES;
        o.priority = 0;
        o.x = x << 8;
        o.y = (y << 8) - 0x80;
    }
    g.objs[n].xdir = rnd_dir(g);
    g.objs[n].ydir = 1;
    g.new_state(n, &S_AMPTONWALK1);
}
pub use self::spawn_ampton as spawn_21260;

/// T_Ampton (0x212cb): walking sounds, flips switches (intile 0x1f) and
/// climbs poles (intile 1) when in the middle of a tile.
pub fn t_ampton(g: &mut Game, ob: usize) {
    if is(g, ob, &S_AMPTONWALK1) {
        g.play_sound(SND_AMPTONWALK1);
    } else if is(g, ob, &S_AMPTONWALK3) {
        g.play_sound(SND_AMPTONWALK2);
    }
    if g.objs[ob].x & 0xff == 0 {
        return;
    }
    let (tx, ty) = (g.objs[ob].tileleft + 1, g.objs[ob].tiletop);
    let t = g.fg_tile(tx, ty);
    let misc = g.intile(t) & 0x7f;
    if misc == 0x1f {
        g.objs[ob].state = Some(&S_AMPTONSWITCH1);
        return;
    }
    if misc != 1 || g.rnd() >= 0xc4 {
        return;
    }
    let t = g.fg_tile(tx, ty + 2);
    let mut down = g.intile(t) & 0x7f == 1;
    let t = g.fg_tile(tx, ty - 2);
    let mut up = g.intile(t) & 0x7f == 1;
    if up && down {
        if g.rnd() < 0x80 {
            up = false;
        } else {
            down = false;
        }
    }
    let ydir = if up {
        -1
    } else if down {
        1
    } else {
        return;
    };
    let o = &mut g.objs[ob];
    o.ydir = ydir;
    o.state = Some(&S_AMPTONSTARTPOLE1);
    o.needtoclip = CL_NOCLIP;
    o.nothink = 6;
    g.ph.xtry = 0;
}

/// T_AmptonClimb (0x21425): climbs until the pole ends or a floor is reached.
pub fn t_ampton_climb(g: &mut Game, ob: usize) {
    let newtile = u16shr(g.objs[ob].bottom + g.ph.ytry, 8);
    if g.objs[ob].tilebottom == newtile {
        return;
    }
    let tx = g.objs[ob].tileleft + 1;
    if g.objs[ob].ydir == -1 {
        let t = g.fg_tile(tx, newtile);
        let t2 = g.fg_tile(tx, newtile + 1);
        if g.north(t) == 0 && g.north(t2) != 0 {
            // reached a floor: maybe climb on if the pole goes on
            let t = g.fg_tile(tx, newtile - 4);
            if g.intile(t) & 0x7f == 1 && g.rnd() < 0x80 {
                return;
            }
            let o = &mut g.objs[ob];
            let mv = (o.bottom & 0xff) + 1;
            o.y -= mv;
            o.bottom -= mv;
            o.needtoclip = CL_MIDCLIP;
            o.state = Some(&S_AMPTONSTOPPOLE1);
            g.ph.ytry = 16;
            g.objs[ob].ydir = 1;
            g.clip_to_walls(ob);
            g.objs[ob].nothink = 4;
            return;
        }
        let t = g.fg_tile(tx, newtile - 1);
        if g.intile(t) & 0x7f == 1 {
            return;
        }
        g.ph.ytry = 0;
        g.objs[ob].ydir = 1;
    } else {
        let t = g.fg_tile(tx, newtile);
        let t2 = g.fg_tile(tx, newtile - 1);
        if g.north(t) != 0 && g.north(t2) == 0 {
            // reached a floor: maybe climb on if the pole goes on
            let t = g.fg_tile(tx, newtile + 2);
            if g.intile(t) & 0x7f == 1 && g.rnd() < 0x80 {
                return;
            }
            let o = &mut g.objs[ob];
            let mv = 0xff - (o.bottom & 0xff);
            o.y += mv;
            o.bottom += mv;
            o.needtoclip = CL_MIDCLIP;
            o.state = Some(&S_AMPTONSTOPPOLE1);
            g.ph.ytry = 16;
            g.clip_to_walls(ob);
            g.objs[ob].nothink = 4;
            return;
        }
        let t = g.fg_tile(tx, newtile);
        if g.intile(t) & 0x7f != 1 {
            g.ph.ytry = 0;
            g.objs[ob].ydir = -1;
        }
    }
}

/// T_SetNoThink (0x21626)
pub fn t_set_nothink(g: &mut Game, ob: usize) {
    g.objs[ob].nothink = 4;
}

/// C_Ampton (0x21635): deadly only while climbing, otherwise solid.
pub fn c_ampton(g: &mut Game, ob: usize, hit: usize) {
    let hc = g.objs[hit].obclass;
    if hc == KEENOBJ {
        if is(g, ob, &S_AMPTONPOLECLIMB) {
            g.kill_keen();
        } else {
            g.clip_to_sprite_side(hit, ob);
        }
    } else if hc == STUNSHOTOBJ {
        let o = &mut g.objs[ob];
        o.needtoclip = CL_MIDCLIP;
        o.ydir = 1;
        o.yspeed = 0;
        g.play_sound(SND_AMPTONSTUN);
        g.stun_obj(ob, hit, &S_AMPTONSTUN);
    }
}

/// R_Ampton (0x21699): turns at walls and ledges.
pub fn r_ampton(g: &mut Game, ob: usize) {
    let o = &g.objs[ob];
    let turn = if o.xdir == 1 && o.hitwest != 0 {
        Some(-1)
    } else if o.xdir == -1 && o.hiteast != 0 {
        Some(1)
    } else if o.hitnorth == 0 {
        Some(-o.xdir)
    } else {
        None
    };
    if let Some(xdir) = turn {
        let o = &mut g.objs[ob];
        o.x -= o.xmove;
        o.xdir = xdir;
        g.change_state(ob, &S_AMPTONTURN);
    }
    g.place_sprite(ob);
}

// ===========================================================================
// Slicestar
// ===========================================================================

/// SpawnSlicestarSlide (0x2172d): moves in a straight line;
/// dir 0=up 1=right 2=down 3=left.
pub fn spawn_slicestar_slide(g: &mut Game, x: i32, y: i32, dir: i32) {
    let n = g.get_new_obj(false);
    let o = &mut g.objs[n];
    o.obclass = SLICESTAROBJ;
    o.active = AC_YES;
    o.priority = 2;
    o.x = x << 8;
    o.y = y << 8;
    // temp4 = hit points
    o.temp4 = 20;
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
    g.new_state(n, &S_SLICESTARSLIDE);
}
pub use self::spawn_slicestar_slide as spawn_2172d;

/// SpawnSlicestarBounce (0x217cc): bounces diagonally off walls.
pub fn spawn_slicestar_bounce(g: &mut Game, x: i32, y: i32) {
    let n = g.get_new_obj(false);
    {
        let o = &mut g.objs[n];
        o.obclass = SLICESTAROBJ;
        o.active = AC_YES;
        o.priority = 2;
        o.needtoclip = CL_FULLCLIP;
        o.x = x << 8;
        o.y = y << 8;
        o.temp4 = 50;
    }
    let (xdir, ydir) = match g.rnd() / 0x40 {
        0 => (-1, -1),
        1 => (1, 1),
        2 => (-1, 1),
        _ => (1, -1),
    };
    g.objs[n].xdir = xdir;
    g.objs[n].ydir = ydir;
    g.new_state(n, &S_SLICESTARBOUNCE);
}
pub use self::spawn_slicestar_bounce as spawn_217cc;

/// C_Slicestar (0x2187a): deadly; destroyed after temp4 shots.
pub fn c_slicestar(g: &mut Game, ob: usize, hit: usize) {
    let hc = g.objs[hit].obclass;
    if hc == KEENOBJ {
        g.kill_keen();
    } else if hc == STUNSHOTOBJ {
        g.explode_shot(hit);
        g.objs[ob].temp4 -= 1;
        if g.objs[ob].temp4 == 0 {
            g.change_state(ob, &S_SLICESTARBOOM);
        }
    }
}

/// R_Slicestar (0x218b7): bounces off walls.
pub fn r_slicestar(g: &mut Game, ob: usize) {
    if g.objs[ob].hitnorth != 0 {
        g.objs[ob].ydir = -1;
        g.play_sound(SND_SLICESTARBOUNCE);
    } else if g.objs[ob].hitsouth != 0 {
        g.objs[ob].ydir = 1;
        g.play_sound(SND_SLICESTARBOUNCE);
    }
    if g.objs[ob].hitwest != 0 {
        g.objs[ob].xdir = -1;
        g.play_sound(SND_SLICESTARBOUNCE);
    } else if g.objs[ob].hiteast != 0 {
        g.objs[ob].xdir = 1;
        g.play_sound(SND_SLICESTARBOUNCE);
    }
    g.place_sprite(ob);
}

// ===========================================================================
// Shelley
// ===========================================================================

/// SpawnShelley (0x2193e)
pub fn spawn_shelley(g: &mut Game, x: i32, y: i32) {
    let n = g.get_new_obj(false);
    {
        let o = &mut g.objs[n];
        o.obclass = SPARKYOBJ; // sic: same class as Sparky
        o.active = AC_YES;
        o.priority = 0;
        o.x = x << 8;
        o.y = y << 8;
    }
    g.objs[n].xdir = rnd_dir(g);
    g.objs[n].ydir = 1;
    g.new_state(n, &S_SHELLEYWALK1);
}
pub use self::spawn_shelley as spawn_2193e;

/// T_Shelley (0x219a6): at a ledge, jumps down at Keen when he is below.
pub fn t_shelley(g: &mut Game, ob: usize) {
    let p = g.player;
    if u(g.objs[p].top) < u(g.objs[ob].bottom) {
        return;
    }
    let dx = d16(g.objs[p].midx, g.objs[ob].midx);
    let xspeed = if g.objs[ob].xdir == 1 {
        if dx > 0x100 && dx < 0x300 {
            16
        } else {
            return;
        }
    } else if dx < -0x100 && dx > -0x300 {
        -16
    } else {
        return;
    };
    let o = &mut g.objs[ob];
    o.xspeed = xspeed;
    o.yspeed = -24;
    o.state = Some(&S_SHELLEYJUMP1);
    g.ph.ytry = 0;
    g.ph.xtry = 0;
}

/// T_Turn (0x21a1a)
pub fn t_turn(g: &mut Game, ob: usize) {
    g.objs[ob].xdir = -g.objs[ob].xdir;
}

/// T_ShelleyBreak (0x21a2c): throws two shell pieces.
pub fn t_shelley_break(g: &mut Game, ob: usize) {
    let (x, y) = (g.objs[ob].x, g.objs[ob].y);
    for (xspeed, st) in [(32, &S_SHELLEYPIECE1), (-32, &S_SHELLEYPIECE2)] {
        let n = g.get_new_obj(true);
        let o = &mut g.objs[n];
        o.x = x;
        o.y = y;
        o.xspeed = xspeed;
        o.yspeed = -24;
        g.new_state(n, st);
    }
}

/// Shelley explosion: smash state and the lethal smoke cloud (part of 0x21a9c /
/// 0x21bc1).
fn shelley_explode(g: &mut Game, ob: usize, st: StateRef) {
    g.play_sound(SND_SHELLEYEXPLODE);
    g.change_state(ob, st);
    let (x, y) = (g.objs[ob].x, g.objs[ob].y);
    let n = g.get_new_obj(true);
    g.objs[n].x = x;
    g.objs[n].y = y;
    g.new_state(n, &S_SHELLEYSMOKE1);
}

/// C_Shelley (0x21a9c): solid for Keen; explodes when Keen touches its
/// middle or a shot hits it.
pub fn c_shelley(g: &mut Game, ob: usize, hit: usize) {
    let hc = g.objs[hit].obclass;
    if hc == KEENOBJ {
        g.clip_to_sprite_side(hit, ob);
        let p = g.player;
        let midx = u(g.objs[p].midx);
        if midx < u(g.objs[ob].left) || midx > u(g.objs[ob].right) {
            return;
        }
    } else if hc == STUNSHOTOBJ {
        g.explode_shot(hit);
    } else if hc == MSHOTOBJ {
        g.remove_obj(hit);
    } else {
        return;
    }
    let st: StateRef = if g.objs[ob].hitnorth != 0 { &S_SHELLEYSMASHGROUND } else { &S_SHELLEYSMASHAIR };
    shelley_explode(g, ob, st);
}

/// R_Shelley (0x21b4f): turns at walls, looks down at ledges.
pub fn r_shelley(g: &mut Game, ob: usize) {
    let o = &mut g.objs[ob];
    if o.xdir == 1 && o.hitwest != 0 {
        o.x -= o.xmove;
        o.xdir = -1;
    } else if o.xdir == -1 && o.hiteast != 0 {
        o.x -= o.xmove;
        o.xdir = 1;
    } else if o.hitnorth == 0 {
        o.x -= o.xmove;
        g.change_state(ob, &S_SHELLEYLOOK);
    }
    g.place_sprite(ob);
}

/// R_ShelleyAir (0x21bc1): explodes on landing.
pub fn r_shelley_air(g: &mut Game, ob: usize) {
    {
        let o = &mut g.objs[ob];
        if o.hiteast != 0 || o.hitwest != 0 {
            o.xspeed = 0;
        }
    }
    if g.objs[ob].hitnorth != 0 {
        shelley_explode(g, ob, &S_SHELLEYSMASHAIR);
    }
    g.place_sprite(ob);
}

/// Module-private state stored in `Game::act2` (none needed).
#[derive(Default, Clone)]
pub struct ModState {}
