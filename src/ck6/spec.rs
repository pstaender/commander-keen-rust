//! CK6_SPEC.C port: info plane scanning and the special item messages.
//! Translated from segment 0x1da0 of KEEN6C.EXE.

use super::act1::*;
use super::act2::*;
use super::act3::*;
use super::defs::*;
use super::Game;

pub const SND_GETSPECIAL: i32 = 45;

impl Game {
    /// ScanInfoPlane (0x1da07)
    pub fn scan_info_plane(&mut self) {
        self.init_obj_array();
        let diff = self.gs.difficulty;
        for y in 0..self.mapheight {
            for x in 0..self.mapwidth {
                let info = self.map_tile(2, x, y) as i32;
                if info == 0 {
                    continue;
                }
                match info {
                    1 => {
                        self.spawn_keen(x, y, 1);
                        self.spawn_score();
                    }
                    2 => {
                        self.spawn_keen(x, y, -1);
                        self.spawn_score();
                    }
                    3 => {
                        self.spawn_world_keen(x, y);
                        self.spawn_score();
                    }
                    4..=6 => {
                        if info - 4 < diff {
                            spawn_bloog(self, x, y);
                        }
                    }
                    7..=14 => spawn_blooglet(self, x, y, info - 7),
                    15 | 16 => spawn_grapple_spot(self, x, y, info - 15),
                    18..=20 => {
                        if info - 18 < diff {
                            spawn_fleex(self, x, y);
                        }
                    }
                    21..=24 => spawn_molly(self, x, y),
                    25 => self.rf_set_scroll_block(x, y, true),
                    26 => self.rf_set_scroll_block(x, y, false),
                    27..=30 => spawn_platform(self, x, y, info - 27),
                    32 => spawn_drop_plat(self, x, y),
                    35 => spawn_static_plat(self, x, y),
                    34 => {
                        if diff <= 2 {
                            spawn_static_plat(self, x, y);
                        }
                    }
                    33 => {
                        if diff <= 1 {
                            spawn_static_plat(self, x, y);
                        }
                    }
                    36..=39 => spawn_goplat(self, x, y, info - 36),
                    40 => spawn_sneak_plat(self, x, y),
                    41..=43 => {
                        if info - 41 < diff {
                            spawn_bobba(self, x, y);
                        }
                    }
                    44 | 45 => spawn_satellite_stop(self, x, y, info - 44),
                    47..=49 => {
                        if info - 47 < diff {
                            spawn_nospike(self, x, y);
                        }
                    }
                    50..=52 => {
                        if info - 50 < diff {
                            spawn_gik(self, x, y);
                        }
                    }
                    53..=56 => spawn_cannon(self, x, y, info - 53),
                    57..=68 => spawn_bonus(self, x, y, info - 57),
                    69 => {
                        if self.gs.ammo < 5 {
                            spawn_bonus(self, x, y, 68 - 57);
                        }
                    }
                    70..=72 => {
                        if info - 70 < diff {
                            spawn_orbatrix(self, x, y);
                        }
                    }
                    73..=75 => {
                        if info - 73 < diff {
                            spawn_bipship(self, x, y);
                        }
                    }
                    76..=78 => {
                        if info - 76 < diff {
                            spawn_flect(self, x, y);
                        }
                    }
                    79..=81 => {
                        if info - 79 < diff {
                            spawn_blorb(self, x, y);
                        }
                    }
                    82..=84 => {
                        if info - 82 < diff {
                            spawn_ceilick(self, x, y);
                        }
                    }
                    85..=87 => {
                        if info - 85 < diff {
                            spawn_blooguard(self, x, y);
                        }
                    }
                    88 => spawn_grabbiter(self, x, y),
                    89 => spawn_satellite(self, x, y),
                    99 => spawn_hook(self, x, y),
                    100 => spawn_sandwich(self, x, y),
                    101 => spawn_passcard(self, x, y),
                    102..=104 => {
                        if info - 102 < diff {
                            spawn_babobba(self, x, y);
                        }
                    }
                    105 | 106 => spawn_rocket(self, x, y, info - 105),
                    _ => {}
                }
            }
        }
        // everything except "always active" objects starts inactive
        let mut i = Some(self.player);
        while let Some(n) = i {
            if self.objs[n].active != AC_ALLWAYS {
                self.objs[n].active = AC_NO;
            }
            i = self.objs[n].next;
        }
    }

    /// Shows a message box with the Keen picture (sandwich/rope/passcard).
    fn special_item_message(&mut self, text: &str, yadd: i32) {
        self.sd.wait_sound_done(&mut self.sys);
        self.play_sound(SND_GETSPECIAL);
        self.draw_playfield();
        self.us_center_window(26, 8);
        let x = self.tw.windowx + self.tw.windoww - 0x30;
        let y = self.tw.windowy;
        self.draw_pic(x, y, 0x23);
        self.tw.windoww -= 0x30;
        self.tw.printy += yadd;
        self.us_cprint(text);
        self.sys.present();
        self.sys.wait_vbl(30);
        self.sys.input.clear_keys_down();
        self.wait_ack();
    }

    /// GotSandwich (0x1e3ba)
    pub fn got_sandwich(&mut self) {
        self.special_item_message("This is the second\nbiggest sandwich\nI ever saw!\n", 0xc);
        self.gs.sandwichstate = 1;
    }

    /// GotHook (0x1e444)
    pub fn got_hook(&mut self) {
        self.special_item_message("Wow! A rope and\ngrappling hook!\nThey look useful!\n", 0xc);
        self.gs.hookstate = 1;
    }

    /// GotPasscard (0x1e4ce)
    pub fn got_passcard(&mut self) {
        self.special_item_message(
            "What's this? Cool!\nA passcard for\nthe Bloogstar Rocket!\n(It can fly through\ntheir force field.)",
            4,
        );
        self.gs.passcardstate = 1;
    }

    /// HitSpecialTile (0x1dff1): see spec_tiles.rs. Switch blocks hit from below (top=false)
    /// or landed on (top=true).
    pub fn hit_special_tile(&mut self, ob: usize, top: bool) {
        super::spec_tiles::hit_special_tile(self, ob, top);
    }

    pub fn spawn_drop_splash(&mut self, x: i32, y: i32) {
        spawn_drop_splash(self, x, y);
    }
}
