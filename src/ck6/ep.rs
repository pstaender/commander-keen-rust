//! Keen 6 specific parts of the shared CK_* modules (the #if GAMEVER_KEEN6
//! branches of the original): object classes, KeenContact, stun stars.

use super::defs::*;
use super::keen::*;
use super::states::*;
use super::Game;

/// Object classes, values from the spawn functions in the executable.
pub mod obclass {
    pub const NOTHING: i32 = 0;
    pub const INERTOBJ: i32 = 1;
    pub const KEENOBJ: i32 = 2;
    pub const STUNSHOTOBJ: i32 = 3;
    pub const BONUSOBJ: i32 = 5;
    pub const PLATFORMOBJ: i32 = 6;
    pub const BLOOGOBJ: i32 = 7;
    pub const BLOOGLETOBJ: i32 = 8;
    pub const FLEEXOBJ: i32 = 10;
    pub const MOLLYOBJ: i32 = 12;
    pub const BABOBBAOBJ: i32 = 13;
    pub const BOBBAOBJ: i32 = 14;
    pub const NOSPIKEOBJ: i32 = 16;
    pub const GIKOBJ: i32 = 17;
    pub const CANNONOBJ: i32 = 18;
    pub const ORBATRIXOBJ: i32 = 19;
    pub const FLECTOBJ: i32 = 21;
    pub const BLORBOBJ: i32 = 22;
    pub const CEILICKOBJ: i32 = 23;
    pub const BLOOGUARDOBJ: i32 = 24;
    pub const STUNNEDOBJ: i32 = 25;
    pub const BIPSHIPOBJ: i32 = 26;
    pub const SANDWICHOBJ: i32 = 27;
    pub const HOOKOBJ: i32 = 28;
    pub const PASSCARDOBJ: i32 = 29;
    pub const GRABBITEROBJ: i32 = 30;
    pub const ROCKETOBJ: i32 = 31;
    pub const GRAPPLEOBJ: i32 = 32;
    pub const SATELLITEOBJ: i32 = 33;
    pub const SATELLITESTOPOBJ: i32 = 34;
    pub const MAPFLAGOBJ: i32 = 35;
}

/// KeenContact (0x1b5c9)
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
        STUNSHOTOBJ => {
            if g.objs[hit].temp4 != 0 {
                g.explode_shot(hit);
                ep6! { g.change_state(ob, &S_KEENSTUNNED); }
            }
            if g.gs.riding.is_none() {
                g.clip_to_sprite_top(ob, hit);
            }
        }
        PLATFORMOBJ => {
            if g.gs.riding.is_none() {
                g.clip_to_sprite_top(ob, hit);
            }
        }
        _ => {}
    }
}

/// KeenPosContact (0x1b6be): while on a pole/lookdown etc, landing on
/// platform-like objects.
pub fn keen_pos_contact(g: &mut Game, ob: usize, hit: usize) {
    let c = g.objs[hit].obclass;
    if c == 0x11 || c == 6 || c == 8 || c == 0x15 {
        g.objs[ob].needtoclip = CL_MIDCLIP;
        g.change_state(ob, &S_KEENFALL1);
        g.objs[ob].yspeed = 0;
        g.objs[ob].xspeed = 0;
        g.jumptime = 0;
        g.clip_to_sprite_top(ob, hit);
    }
}

/// R_Stunned (0x1a1c1): position of the stun stars by the original class
/// of the stunned object; None = "No star spec for object!".
pub fn stun_star_offset(class: i32) -> Option<(i32, i32)> {
    Some(match class {
        BLOOGOBJ | NOSPIKEOBJ => (0x80, -0x40),
        BLOOGLETOBJ | BABOBBAOBJ => (0, -0x80),
        FLEEXOBJ => (0x100, 0x80),
        FLECTOBJ => (0x40, -0x40),
        CEILICKOBJ => (0, 0xc0),
        BLOOGUARDOBJ => (0x100, -0x40),
        _ => return None,
    })
}

/// first of the three stun star sprites
pub const STARSPR: i32 = 0x56;

/// FullClipToWalls (0x192d1): clip box (width, height) by obclass.
pub fn fullclip_box(class: i32) -> Option<(i32, i32)> {
    match class {
        22 => Some((0x200, 0x200)),
        _ => None,
    }
}

/// StunObj: stunned objects hop up a little.
pub const STUN_HOP: bool = true;

/// DrawStatusWindow (0x1757c)
pub fn draw_status_window(g: &mut Game) {
    g.draw_status_frame();
    let name = super::play::LEVELNAMES.get(g.gs.mapon as usize).copied().unwrap_or("");
    g.draw_status_location(name);
    g.draw_status_score();
    g.draw_status_level();
    g.tw.printx = 0x50;
    g.tw.printy = 0x60;
    g.us_print("ITEMS");
    g.sys.surface.bar(0x7f, 0x5f, 0x1a, 10, 0);
    g.draw_tile8(0x80, 0x60, if g.gs.sandwichstate == 1 { 2 } else { 1 });
    g.draw_tile8(0x88, 0x60, if g.gs.hookstate == 1 { 4 } else { 3 });
    g.draw_tile8(0x90, 0x60, if g.gs.passcardstate == 1 { 6 } else { 5 });
    g.draw_status_counts("VIVAS");
    g.draw_status_logo(0xf);
}
