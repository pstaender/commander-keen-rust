//! CK6 actors (act3): code segment CODE_12 (linear 0x21078 .. 0x218da).
//!
//! Note on naming: the function/state names come from `tools/gen_states.py`.
//! Judging from the behaviour, class 10 ("Babobba", 0x21078) is the Fleex
//! (4 hit points, walks, stops to look when Keen stands still), class 13
//! ("Bip", 0x21458) is the Babobba (jumps, sits, naps, spits cinders), and
//! class 14 is the Bobba (jumps, throws fireballs).

use super::defs::*;
use super::states::*;
use super::Game;

/// Object class of enemy projectiles (Bobba fireball, Babobba cinder).
const MSHOTOBJ: i32 = 4;

/// Direction towards the player, using the unsigned x compare of the original.
fn dir_to_player(g: &Game, ob: usize) -> i32 {
    if (g.objs[ob].x as u16) < (g.objs[g.player].x as u16) {
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

/// The ledge check shared by T_Bobba and T_Bip: walks up to 4 tiles in xdir
/// along the row below the object; if a column has no north wall in the rows
/// tilebottom .. tilebottom+2, the object turns around.
fn check_ledge(g: &mut Game, ob: usize) {
    let mw = g.mapwidth;
    let (xdir, tb, tmx) = {
        let o = &g.objs[ob];
        (o.xdir, o.tilebottom, o.tilemidx)
    };
    let mut idx = (tb + 1) * mw + tmx;
    for _ in 0..4 {
        if g.north(fg_linear(g, idx)) == 0
            && g.north(fg_linear(g, idx - mw)) == 0
            && g.north(fg_linear(g, idx + mw)) == 0
        {
            g.objs[ob].xdir = -g.objs[ob].xdir;
            break;
        }
        idx += xdir;
    }
}

// ---- "Babobba" (class 10, behaves like the Fleex) ---------------------------

/// SpawnBabobba (0x21078)
pub fn spawn_fleex(g: &mut Game, x: i32, y: i32) {
    let n = g.get_new_obj(false);
    {
        let o = &mut g.objs[n];
        o.obclass = FLEEXOBJ;
        o.active = AC_YES;
        o.priority = 0;
        o.x = x << 8;
        o.y = (y << 8) - 0x280;
    }
    g.objs[n].xdir = if g.rnd() < 0x80 { 1 } else { -1 };
    g.objs[n].ydir = 1;
    g.new_state(n, &S_FLEEXWALK1);
    g.objs[n].temp2 = 4;
}

/// T_BabobbaJump (0x210ee): stop to look if Keen stands still, else follow him.
pub fn t_fleex_walk(g: &mut Game, ob: usize) {
    if g.objs[g.player].xmove == 0 && g.objs[ob].temp1 == 0 {
        g.objs[ob].state = Some(&S_FLEEXLOOK1);
    } else {
        g.objs[ob].xdir = dir_to_player(g, ob);
    }
}

/// T_BabobbaShoot (0x21126): face the player.
pub fn t_fleex_look(g: &mut Game, ob: usize) {
    g.objs[ob].xdir = dir_to_player(g, ob);
}

/// C_Babobba (0x21147)
pub fn c_fleex(g: &mut Game, ob: usize, hit: usize) {
    if g.objs[hit].obclass == KEENOBJ {
        g.kill_keen();
    }
    if g.objs[hit].obclass == STUNSHOTOBJ {
        g.objs[ob].temp2 -= 1;
        if g.objs[ob].temp2 == 0 {
            g.stun_obj(ob, hit, &S_FLEEXSTUN);
            g.objs[ob].yspeed = -20;
        } else {
            g.objs[ob].temp1 = 2;
            g.objs[ob].needtoreact = true;
            g.explode_shot(hit);
            if same(g.objs[ob].state, &S_FLEEXLOOK1) || same(g.objs[ob].state, &S_FLEEXLOOK2) {
                g.objs[ob].xdir = dir_to_player(g, ob);
                g.change_state(ob, &S_FLEEXWALK1);
            }
        }
    }
}

// ---- Bobba (class 14) -------------------------------------------------------

/// SpawnBobba (0x211c3)
pub fn spawn_bobba(g: &mut Game, x: i32, y: i32) {
    let n = g.get_new_obj(false);
    {
        let o = &mut g.objs[n];
        o.obclass = BOBBAOBJ;
        o.active = AC_YES;
        o.priority = 0;
        o.x = x << 8;
        o.y = (y << 8) - 0x200;
    }
    g.objs[n].xdir = if g.rnd() < 0x80 { 1 } else { -1 };
    g.objs[n].ydir = 1;
    g.new_state(n, &S_BOBBAJUMP1);
}

/// T_BobbaShoot (0x21230): fireball launch sound.
pub fn t_bobba_shoot(g: &mut Game, ob: usize) {
    let _ = ob;
    g.play_sound(0x1f);
}

/// T_Bobba (0x21240): every third landing throw a fireball, else jump on.
pub fn t_bobba(g: &mut Game, ob: usize) {
    g.objs[ob].temp1 += 1;
    if g.objs[ob].temp1 == 3 {
        g.objs[ob].temp1 = 0;
        let n = g.get_new_obj(true);
        let (ox, oy, oxdir) = {
            let o = &g.objs[ob];
            (o.x, o.y, o.xdir)
        };
        {
            let s = &mut g.objs[n];
            s.active = AC_REMOVABLE;
            s.obclass = MSHOTOBJ;
            s.y = oy + 0xb0;
            s.xdir = oxdir;
            s.x = if oxdir == 1 { ox + 0x100 } else { ox + 0xb0 };
        }
        g.new_state(n, &S_BOBBACHARGE1);
        g.objs[n].priority = 2;
        g.objs[ob].state = Some(&S_BOBBASTAND2);
    } else {
        check_ledge(g, ob);
        let o = &mut g.objs[ob];
        o.xspeed = o.xdir << 5;
        o.yspeed = -32;
        g.play_sound(0x1a);
    }
}

/// C_Bobba (0x21366)
pub fn c_bobba(g: &mut Game, ob: usize, hit: usize) {
    if g.objs[hit].obclass == KEENOBJ {
        g.kill_keen();
    } else if g.objs[hit].obclass == STUNSHOTOBJ {
        g.explode_shot(hit);
        if g.objs[hit].xdir != 0 {
            g.objs[ob].xdir = -g.objs[hit].xdir;
        }
    }
}

/// R_Bobba (0x2139c)
pub fn r_bobba(g: &mut Game, ob: usize) {
    {
        let o = &mut g.objs[ob];
        if o.hiteast != 0 {
            o.xdir = 1;
            o.xspeed = -o.xspeed;
        } else if o.hitwest != 0 {
            o.xdir = -1;
            o.xspeed = -o.xspeed;
        }
        if o.hitsouth != 0 {
            o.yspeed = 0;
        }
    }
    if g.objs[ob].hitnorth != 0 {
        g.play_sound(0x1b);
        g.change_state(ob, &S_BOBBASTAND);
    }
    g.place_sprite(ob);
}

/// R_BobbaShot (0x2140c): the fireball vanishes when it hits any wall.
pub fn r_bobba_shot(g: &mut Game, ob: usize) {
    let hit = {
        let o = &g.objs[ob];
        o.hitnorth != 0 || o.hitsouth != 0 || o.hiteast != 0 || o.hitwest != 0
    };
    if hit {
        g.change_state(ob, &S_BOBBASHOTVANISH1);
    }
    g.place_sprite(ob);
}

// ---- "Bip" (class 13, behaves like the Babobba) -----------------------------

/// SpawnBip (0x21458)
pub fn spawn_babobba(g: &mut Game, x: i32, y: i32) {
    let n = g.get_new_obj(false);
    {
        let o = &mut g.objs[n];
        o.obclass = BABOBBAOBJ;
        o.active = AC_YES;
        o.priority = 0;
        o.x = x << 8;
        o.y = (y << 8) - 0x200;
    }
    g.objs[n].xdir = if g.rnd() < 0x80 { 1 } else { -1 };
    g.objs[n].ydir = 1;
    g.new_state(n, &S_BABOBBAJUMP1);
}

/// T_Bip (0x214c5): sometimes nap, every third landing spit a cinder, else jump.
pub fn t_babobba(g: &mut Game, ob: usize) {
    if g.rnd() < 4 {
        g.objs[ob].temp1 = 0;
        g.objs[ob].state = Some(&S_BABOBBANAP1);
        return;
    }
    g.objs[ob].temp1 += 1;
    if g.objs[ob].temp1 == 3 {
        g.objs[ob].temp1 = 0;
        let n = g.get_new_obj(true);
        let (ox, oy, oxdir) = {
            let o = &g.objs[ob];
            (o.x, o.y, o.xdir)
        };
        {
            let s = &mut g.objs[n];
            s.active = AC_REMOVABLE;
            s.obclass = MSHOTOBJ;
            s.y = oy + 0x40;
            s.xdir = oxdir;
            s.x = if oxdir == 1 { ox + 0x100 } else { ox + 0xb0 };
            s.xspeed = s.xdir << 5;
        }
        g.new_state(n, &S_CINDERFLY);
        g.objs[n].priority = 2;
        g.objs[ob].state = Some(&S_BABOBBASIT2);
    } else {
        check_ledge(g, ob);
        let o = &mut g.objs[ob];
        o.xspeed = o.xdir * 24;
        o.yspeed = -32;
    }
}

/// C_Bip (0x21606)
pub fn c_babobba(g: &mut Game, ob: usize, hit: usize) {
    if g.objs[hit].obclass == KEENOBJ {
        g.kill_keen();
    } else if g.objs[hit].obclass == STUNSHOTOBJ {
        g.stun_obj(ob, hit, &S_BABOBBASTUN);
    }
}

/// C_BipWalk (0x21631): harmless while napping, can still be stunned.
pub fn c_babobba_nap(g: &mut Game, ob: usize, hit: usize) {
    if g.objs[hit].obclass == STUNSHOTOBJ {
        g.stun_obj(ob, hit, &S_BABOBBASTUN);
    }
}

/// R_Bip (0x21650)
pub fn r_babobba(g: &mut Game, ob: usize) {
    {
        let o = &mut g.objs[ob];
        if o.hiteast != 0 {
            o.xdir = 1;
            o.xspeed = -o.xspeed;
        } else if o.hitwest != 0 {
            o.xdir = -1;
            o.xspeed = -o.xspeed;
        }
        if o.hitsouth != 0 {
            o.yspeed = 0;
        }
    }
    if g.objs[ob].hitnorth != 0 {
        g.change_state(ob, &S_BABOBBASIT);
    }
    g.place_sprite(ob);
}

/// T_Squished1 (0x216b5): cinder burns for 10 cycles, then starts blinking.
pub fn t_cinder1(g: &mut Game, ob: usize) {
    g.objs[ob].temp1 += 1;
    if g.objs[ob].temp1 == 10 {
        g.objs[ob].temp1 = 0;
        g.objs[ob].state = Some(&S_CINDER3);
    }
}

/// T_Squished2 (0x216d4): cinder removed after 5 blink cycles.
pub fn t_cinder2(g: &mut Game, ob: usize) {
    g.objs[ob].temp1 += 1;
    if g.objs[ob].temp1 == 5 {
        g.remove_obj(ob);
    }
}

// ---- Blorb (class 22) ---------------------------------------------------------

/// SpawnBlorb (0x216f1)
pub fn spawn_blorb(g: &mut Game, x: i32, y: i32) {
    let n = g.get_new_obj(false);
    {
        let o = &mut g.objs[n];
        o.obclass = BLORBOBJ;
        o.active = AC_YES;
        o.priority = 0;
        o.x = x << 8;
        o.y = (y << 8) - 0x80;
    }
    g.objs[n].xdir = if g.rnd() < 0x80 { 1 } else { -1 };
    g.objs[n].ydir = if g.rnd() < 0x80 { 1 } else { -1 };
    g.objs[n].needtoclip = CL_FULLCLIP;
    g.new_state(n, &S_BLORB1);
}

/// R_Blorb (0x2177c): bounce off walls diagonally.
pub fn r_blorb(g: &mut Game, ob: usize) {
    if g.objs[ob].hitnorth != 0 {
        g.objs[ob].ydir = -1;
        g.play_sound(6);
    } else if g.objs[ob].hitsouth != 0 {
        g.objs[ob].ydir = 1;
        g.play_sound(6);
    }
    if g.objs[ob].hitwest != 0 {
        g.objs[ob].xdir = -1;
        g.play_sound(6);
    } else if g.objs[ob].hiteast != 0 {
        g.objs[ob].xdir = 1;
        g.play_sound(6);
    }
    g.place_sprite(ob);
}

// ---- Ceilick (class 23) -------------------------------------------------------

/// SpawnCeilick (0x217e9)
pub fn spawn_ceilick(g: &mut Game, x: i32, y: i32) {
    let n = g.get_new_obj(false);
    {
        let o = &mut g.objs[n];
        o.obclass = CEILICKOBJ;
        o.active = AC_YES;
        o.priority = 0;
        o.needtoclip = CL_NOCLIP;
        o.x = x << 8;
        o.temp1 = y << 8;
        o.y = y << 8;
        o.ydir = 1;
    }
    g.new_state(n, &S_CEILICKHIDDEN);
}

/// T_Ceilick (0x21839): attack when Keen is just below.
pub fn t_ceilick(g: &mut Game, ob: usize) {
    let p = &g.objs[g.player];
    let o = &g.objs[ob];
    if ((p.y - o.y) as u16) <= 0x280
        && (p.left as u16) < ((o.right + 0x10) as u16)
        && (p.right as u16) > ((o.left - 0x10) as u16)
    {
        g.play_sound(0x33);
        g.objs[ob].state = Some(&S_CEILICKATTACK1);
    }
}

/// T_CeilickLaugh (0x21878)
pub fn t_ceilick_laugh(g: &mut Game, ob: usize) {
    let _ = ob;
    g.play_sound(0x37);
}

/// T_CeilickStun (0x21888)
pub fn t_ceilick_stun(g: &mut Game, ob: usize) {
    g.objs[ob].needtoreact = true;
}

/// C_Ceilick (0x21897): stunned by a shot (back at its ceiling position).
pub fn c_ceilick(g: &mut Game, ob: usize, hit: usize) {
    if g.objs[hit].obclass == STUNSHOTOBJ {
        g.objs[ob].y = g.objs[ob].temp1;
        g.explode_shot(hit);
        {
            let o = &mut g.objs[ob];
            o.temp3 = 0;
            o.temp2 = 0;
            o.temp1 = 0;
            o.temp4 = o.obclass;
        }
        g.change_state(ob, &S_CEILICKSTUN);
        g.objs[ob].obclass = STUNNEDOBJ;
    }
}

/// Module-private state stored in `Game::act3` (none needed).
#[derive(Default, Clone)]
pub struct ModState {}
