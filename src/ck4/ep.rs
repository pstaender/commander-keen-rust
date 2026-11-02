//! Keen 4 specific parts of the shared CK_* modules (the #if GAMEVER_KEEN4
//! branches of the original): object classes, KeenContact, stun stars.

#[allow(unused_imports)]
use super::defs::*;
#[allow(unused_imports)]
use super::keen::*;
#[allow(unused_imports)]
use super::states::*;
use super::Game;
use super::act1::{ARACHNUTOBJ, MADMUSHROOMOBJ, ORACLEOBJ};
use super::act2::{BERKELOIDOBJ, BOUNDEROBJ, FOOTOBJ};
use super::act3::LINDSEYOBJ;

/// sound of a rescued council member
const SND_RESCUED: i32 = 0x1f;

/// Object classes used by the shared code, values from the executable;
/// the actor modules define their own classes.
pub mod obclass {
    pub const NOTHING: i32 = 0;
    pub const INERTOBJ: i32 = 1;
    pub const KEENOBJ: i32 = 2;
    pub const STUNSHOTOBJ: i32 = 3;
    pub const BONUSOBJ: i32 = 4;
    pub const PLATFORMOBJ: i32 = 20;
    pub const STUNNEDOBJ: i32 = 33;
    pub const MAPFLAGOBJ: i32 = 34;
}

/// KeenContact (0x1c304): bonus items, the council members (on map 14
/// the janitor), the foot to the secret level, bounders and platforms to
/// stand on, Princess Lindsey.
pub fn keen_contact(g: &mut Game, ob: usize, hit: usize) {
    match g.objs[hit].obclass {
        BONUSOBJ => {
            let kind = g.objs[hit].temp1;
            if !(0..=11).contains(&kind) {
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
            }
            g.change_state(hit, &S_BONUSRISE);
        }
        ORACLEOBJ => {
            if g.objs[ob].hitnorth == 0 {
                return;
            }
            if g.ca_mapon == 14 {
                g.rescue_janitor();
                g.rf_force_refresh();
                g.remove_obj(hit);
            } else {
                g.play_sound(SND_RESCUED);
                g.playstate = EX_RESCUED;
            }
        }
        BOUNDEROBJ => g.clip_to_sprite(ob, hit, false),
        FOOTOBJ => g.playstate = EX_FOOT,
        PLATFORMOBJ => {
            if g.gs.riding.is_none() {
                g.clip_to_sprite_top(ob, hit);
            }
        }
        LINDSEYOBJ => {
            g.princess_lindsey();
            g.remove_obj(hit);
            g.rf_force_refresh();
        }
        STUNNEDOBJ => {
            // a stunned bounder can be stood on like a platform
            if g.objs[hit].temp4 == BOUNDEROBJ && g.gs.riding.is_none() {
                g.clip_to_sprite_top(ob, hit);
            }
        }
        _ => {}
    }
}

/// KeenPosContact (0x1c468): while on a pole or looking down, some enemies
/// kill Keen, bounders and platforms knock him off.
pub fn keen_pos_contact(g: &mut Game, ob: usize, hit: usize) {
    match g.objs[hit].obclass {
        MADMUSHROOMOBJ | ARACHNUTOBJ | BERKELOIDOBJ => g.kill_keen(),
        BOUNDEROBJ => {
            pos_fall(g, ob);
            g.clip_to_sprite(ob, hit, false);
        }
        PLATFORMOBJ => {
            pos_fall(g, ob);
            g.clip_to_sprite_top(ob, hit);
        }
        _ => {}
    }
}

fn pos_fall(g: &mut Game, ob: usize) {
    g.objs[ob].needtoclip = CL_MIDCLIP;
    g.change_state(ob, &S_KEENFALL1);
    g.objs[ob].yspeed = 0;
    g.objs[ob].xspeed = 0;
    g.jumptime = 0;
}

/// KeenSlideReact (0x1cf69): sliding down a pole onto a floor, Keen stands
/// there looking down.
pub fn keen_slide_react(g: &mut Game, ob: usize) {
    let (tr, tb) = (g.objs[ob].tileright, g.objs[ob].tilebottom);
    if g.north(g.fg_tile(tr, tb)) == 1 {
        let o = &mut g.objs[ob];
        o.y -= o.bottom & 0xff;
        o.needtoclip = CL_MIDCLIP;
        g.change_state(ob, &S_KEENLOOKDOWN);
    }
    g.place_sprite(ob);
}

/// R_Stunned (0x1aef2): position of the stun stars by the original class
/// of the stunned object (jump table at 196c:1937); classes outside the
/// table get no offset.
pub fn stun_star_offset(class: i32) -> Option<(i32, i32)> {
    Some(match class {
        5 | 18 => (0, -0x80),
        8 => (0x80, -0x80),
        12 => (0x40, -0x15e),
        15 => (0x40, -0x80),
        16 => (-0x40, -0x80),
        19 => (0, -0x40),
        26 => (0x80, 0),
        _ => (0, 0),
    })
}

/// first of the three stun star sprites
pub const STARSPR: i32 = 0x9c;

/// FullClipToWalls (0x19fe0): clip box (width, height) by obclass.
pub fn fullclip_box(class: i32) -> Option<(i32, i32)> {
    match class {
        2 => Some((0x280, 0x180)),
        27 => Some((0x400, 0x200)),
        21 => Some((0x580, 0x400)),
        22 => Some((0x100, 0x80)),
        _ => None,
    }
}

/// StunObj: Keen 4 stunned objects don't hop.
pub const STUN_HOP: bool = false;

/// DrawStatusWindow (0x1775a): the RESCUED council members and the
/// wetsuit instead of Keen 6's items, DROPS for the lifewater drops.
pub fn draw_status_window(g: &mut Game) {
    g.draw_status_frame();
    let name = super::play::LEVELNAMES.get(g.gs.mapon as usize).copied().unwrap_or("");
    g.draw_status_location(name);
    g.draw_status_score();
    g.tw.printy = 0x55;
    g.tw.windowx = 0x50;
    g.tw.windoww = 0x40;
    g.us_cprint("RESCUED");
    g.sys.surface.bar(0x4f, 0x5f, 0x42, 10, 0);
    for i in 0..g.gs.rescued {
        g.draw_tile8((i + 10) * 8, 0x60, 0x28);
    }
    g.draw_status_level();
    g.draw_status_counts("DROPS");
    g.sys.surface.bar(0x4f, 0x8f, 0x42, 10, 15);
    g.tw.printy = 0x90;
    g.tw.windowx = 0x50;
    g.tw.windoww = 0x40;
    g.us_cprint(if g.gs.wetsuit != 0 { "Wetsuit" } else { "???" });
    g.draw_status_logo(0x14);
}
