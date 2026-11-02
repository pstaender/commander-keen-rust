//! Keen 5 specific parts of the shared CK_* modules (the #if GAMEVER_KEEN5
//! branches of the original): object classes, KeenContact, stun stars.

#[allow(unused_imports)]
use super::defs::*;
#[allow(unused_imports)]
use super::keen::*;
#[allow(unused_imports)]
use super::states::*;
use super::Game;
use super::act2::AMPTONOBJ;
use super::act3::KORATHOBJ;

/// Object classes used by the shared code, values from the executable;
/// the actor modules define their own classes.
pub mod obclass {
    pub const NOTHING: i32 = 0;
    pub const INERTOBJ: i32 = 1;
    pub const KEENOBJ: i32 = 2;
    pub const STUNSHOTOBJ: i32 = 3;
    pub const BONUSOBJ: i32 = 5;
    pub const STUNNEDOBJ: i32 = 7;
    pub const PLATFORMOBJ: i32 = 6;
    pub const MAPFLAGOBJ: i32 = 8;
}

/// KeenContact (0x1c087): bonus items (incl. the keycard) and platforms.
pub fn keen_contact(g: &mut Game, ob: usize, hit: usize) {
    match g.objs[hit].obclass {
        BONUSOBJ => {
            let kind = g.objs[hit].temp1;
            if !(0..=12).contains(&kind) {
                return;
            }
            let k = kind as usize;
            g.play_sound(BONUSSOUND[k]);
            g.objs[hit].obclass = INERTOBJ;
            g.objs[hit].priority = 3;
            g.objs[hit].shapenum = BONUSSPRITE[k];
            g.give_points(BONUSPOINTS[k]);
            if k < 4 {
                g.gs.keys[k] += 1;
            } else if k == 10 {
                g.gs.lives += 1;
            } else if k == 11 {
                g.gs.ammo += AMMOBYDIFF[g.gs.difficulty.clamp(0, 3) as usize];
            } else if k == 12 {
                g.gs.keycard = 1;
            }
            g.change_state(hit, &S_BONUSRISE);
        }
        PLATFORMOBJ => {
            if g.gs.riding.is_none() {
                g.clip_to_sprite_top(ob, hit);
            }
        }
        _ => {}
    }
}

/// KeenPosContact (0x1c16f): while on a pole or looking down, platforms,
/// Amptons and Korath knock Keen off.
pub fn keen_pos_contact(g: &mut Game, ob: usize, hit: usize) {
    match g.objs[hit].obclass {
        PLATFORMOBJ => {
            g.objs[ob].needtoclip = CL_MIDCLIP;
            g.change_state(ob, &S_KEENFALL1);
            g.objs[ob].yspeed = 0;
            g.objs[ob].xspeed = 0;
            g.jumptime = 0;
            g.clip_to_sprite_top(ob, hit);
        }
        AMPTONOBJ | KORATHOBJ => {
            g.objs[ob].priority = 1;
            g.objs[ob].needtoclip = CL_MIDCLIP;
            g.change_state(ob, &S_KEENFALL1);
            g.objs[ob].yspeed = 0;
            g.objs[ob].xspeed = 0;
            g.jumptime = 0;
        }
        _ => {}
    }
}

/// KeenSlideReact (0x1cc35): sliding down a pole onto a floor, Keen stands
/// there looking down.
pub fn keen_slide_react(g: &mut Game, ob: usize) {
    let (tm, tb) = (g.objs[ob].tilemidx, g.objs[ob].tilebottom);
    if g.north(g.fg_tile(tm, tb)) == 1 {
        let o = &mut g.objs[ob];
        let d = (o.bottom & 0xff) + 1;
        o.y -= d;
        o.bottom -= d;
        o.tilebottom -= 1;
        o.needtoclip = CL_MIDCLIP;
        g.change_state(ob, &S_KEENLOOKDOWN);
    }
    g.place_sprite(ob);
}

/// CheckEnterHouse (0x1ac19): switches, doors and the security doors
/// (tile type 0x20) that need the keycard; a door without destination in
/// the info plane is a teleporter.
pub fn check_enter_house(g: &mut Game, ob: usize) -> bool {
    let (tm, tt) = (g.objs[ob].tilemidx, g.objs[ob].tiletop);
    let it = g.intile(g.fg_tile(tm, tt));
    if it == 5 || it == 6 || it == 15 {
        let target = tm * 256 - 0x40;
        if g.objs[ob].x == target {
            g.objs[ob].state = Some(&S_KEENSWITCH);
        } else {
            g.objs[ob].temp1 = target;
            g.objs[ob].state = Some(&S_KEENLINEUP);
        }
        g.upheld = true;
        return true;
    }
    if it != 2 && it != 0x20 {
        return false;
    }
    let base = tm * 256;
    let mut target = base + 0x60;
    let left = g.intile(g.fg_tile(tm - 1, tt));
    if left == 2 || left == 0x20 {
        target = base - 0xa0;
    }
    if g.objs[ob].x != target {
        g.objs[ob].temp1 = target;
        g.objs[ob].state = Some(&S_KEENLINEUP);
        g.upheld = true;
        return true;
    }
    if it == 0x20 {
        if g.gs.keycard != 0 {
            g.gs.keycard = 0;
            g.play_sound(SND_OPENCARDDOOR);
            let n = g.get_new_obj(false);
            let tb = g.objs[ob].tilebottom;
            let o = &mut g.objs[n];
            o.x = tm - 2;
            o.y = tb - 4;
            o.active = AC_ALLWAYS;
            o.needtoclip = CL_NOCLIP;
            o.obclass = INERTOBJ;
            g.new_state(n, &S_CARDDOOR);
            g.objs[ob].state = Some(&S_KEENOPENCARDDOOR);
            g.objs[ob].priority = 0;
            g.upheld = true;
            return true;
        }
        g.play_sound(SND_NOWAY);
        g.objs[ob].state = Some(&S_KEENSTAND);
        g.upheld = true;
        return false;
    }
    g.invincible = 110;
    g.objs[ob].state = Some(&S_KEENENTER1);
    g.objs[ob].priority = 0;
    if g.map_tile(2, tm, tt) == 0 {
        super::act1::spawn_teleport(g);
    }
    g.upheld = true;
    true
}

/// sound of the security door opening
const SND_OPENCARDDOOR: i32 = 0x12;

/// R_Stunned (0x1aa0a): position of the stun stars by the original class
/// of the stunned object; other classes get no offset.
pub fn stun_star_offset(class: i32) -> Option<(i32, i32)> {
    Some(match class {
        9 => (0x40, 0),
        14 | 23 => (0, -0x80),
        _ => (0, 0),
    })
}

/// first of the three stun star sprites
pub const STARSPR: i32 = 0x8f;

/// FullClipToWalls (0x19a51): clip box (width, height) by obclass.
pub fn fullclip_box(class: i32) -> Option<(i32, i32)> {
    match class {
        11 | 22 => Some((0x200, 0x200)),
        _ => None,
    }
}

/// StunObj: stunned objects hop up a little.
pub const STUN_HOP: bool = true;

/// DrawStatusWindow (0x17bac): the KEYCARD instead of Keen 6's items,
/// VITALIN for the drops; on the world map below y 0x6400 the location is
/// the Korath III Base.
pub fn draw_status_window(g: &mut Game) {
    g.draw_status_frame();
    let name = if g.ca_mapon == 0 && (g.objs[g.player].y as u16) > 0x6400 {
        "Korath III Base"
    } else {
        super::play::LEVELNAMES.get(g.gs.mapon as usize).copied().unwrap_or("")
    };
    g.draw_status_location(name);
    g.draw_status_score();
    g.tw.printy = 0x5c;
    g.tw.printx = 0x50;
    g.us_print("KEYCARD");
    g.sys.surface.bar(0x87, 0x5b, 10, 10, 0);
    if g.gs.keycard != 0 {
        g.draw_tile8(0x88, 0x5c, 0x28);
    }
    g.draw_status_level();
    g.draw_status_counts("VITALIN");
    g.draw_status_logo(0xf);
}
