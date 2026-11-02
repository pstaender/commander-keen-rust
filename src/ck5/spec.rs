//! CK5_SPEC.C port: info plane scanning, the world map elevator doors, the
//! fuse message and the exploding galaxy (Game Over).
//! Translated from segment 0x1ed5 of KEEN5E.EXE (0x1ed5d..0x1f922).
//!
//! The original also fills `lumpneeded[]` and marks graphics chunks for
//! caching (CA_MarkGrChunk / CA_CacheMarks / FreeGraphics); everything is
//! always loaded here, so those writes are left out.

use super::act1;
use super::act2;
use super::act3;
use super::defs::*;
use super::Game;


/// Map number of the QED level (info value 41 spawns the QED there).
pub const QED_MAP: i32 = 12;
/// Map number of the Korath III base (its fuse gets a different message).
pub const KORATH_MAP: i32 = 13;

const SND_GALAXYEXPLODE: i32 = 0x35;
const SND_GALAXYEXPLODEPRE: i32 = 0x36;

// picture chunks
const KEENTALK1PIC: i32 = 0x5a;
const KEENTALK2PIC: i32 = 0x5b;
const MILKYWAYPIC: i32 = 0x59;
const GAMEOVERPIC: i32 = 0x62;

/// DS:0x18a2 palette (16 registers + overscan) used while the galaxy blinks
const GALAXY_PALETTE: [u8; 17] = [0, 1, 2, 3, 4, 5, 6, 7, 24, 25, 26, 27, 28, 29, 30, 31, 3];
/// DS:0x185a values of palette register 8 for the 18 blink steps
const GALAXY_PAL8: [u8; 18] = [8, 8, 7, 15, 7, 8, 0, 8, 7, 15, 7, 8, 0, 0, 0, 0, 0, 0];
/// DS:0x187e values of palette register 7 for the 18 blink steps
const GALAXY_PAL7: [u8; 18] = [7, 7, 7, 7, 7, 15, 7, 8, 0, 7, 15, 7, 8, 0, 0, 0, 0, 0];

/// Number of stars in the GALAXY chunk (four arrays x, xspeed, y, yspeed).
const NUMSTARS: usize = 4000;

impl Game {
    /// ScanInfoPlane (0x1ee32)
    pub fn scan_info_plane(&mut self) {
        self.init_obj_array();
        // memset(lumpneeded, 0, sizeof(lumpneeded))
        *self.numfuses_mut() = 0;
        for y in 0..self.mapheight {
            for x in 0..self.mapwidth {
                let info = self.map_tile(2, x, y) as i32;
                if info == 0 {
                    continue;
                }
                let diff = self.gs.difficulty;
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
                        self.spawn_score();
                        if self.playstate != EX_PORTOUT {
                            self.spawn_world_keen(x, y);
                        }
                    }
                    26 => {
                        if self.playstate == EX_PORTOUT {
                            self.spawn_world_keen_port(x, y);
                        }
                    }
                    4..=6 => {
                        if diff_ok(diff, info - 4) {
                            act2::spawn_21024(self, x, y);
                        }
                    }
                    7..=9 => {
                        if diff_ok(diff, info - 7) {
                            act3::spawn_21c41(self, x, y);
                        }
                    }
                    10..=12 => {
                        if diff_ok(diff, info - 10) {
                            act2::spawn_2172d(self, x, y, 0);
                        }
                    }
                    13..=15 => {
                        if diff_ok(diff, info - 13) {
                            act3::spawn_2259c(self, x, y);
                        }
                    }
                    16..=18 => {
                        if diff_ok(diff, info - 16) {
                            act3::spawn_227a1(self, x, y);
                        }
                    }
                    19..=21 => {
                        if diff_ok(diff, info - 19) {
                            act2::spawn_217cc(self, x, y);
                        }
                    }
                    22..=24 => {
                        if diff_ok(diff, info - 22) {
                            act2::spawn_2172d(self, x, y, 1);
                        }
                    }
                    25 => self.rf_set_scroll_block(x, y, true),
                    27..=30 => act1::spawn_1ffdc(self, x, y, info - 27, 0),
                    32 => act1::spawn_204e8(self, x, y),
                    35 => act1::spawn_2060c(self, x, y),
                    34 => {
                        if diff <= GD_NORMAL {
                            act1::spawn_2060c(self, x, y);
                        }
                    }
                    33 => {
                        if diff <= GD_EASY {
                            act1::spawn_2060c(self, x, y);
                        }
                    }
                    36..=39 => act1::spawn_2065f(self, x, y, info - 36, 0),
                    40 => act1::spawn_20df1(self, x, y),
                    41 => {
                        if self.gs.mapon == QED_MAP {
                            *self.numfuses_mut() = 4;
                            act3::spawn_23996(self, x, y);
                        } else {
                            *self.numfuses_mut() += 1;
                        }
                    }
                    42..=44 => {
                        if diff_ok(diff, info - 42) {
                            act2::spawn_21260(self, x, y);
                        }
                    }
                    // 45..48 always, 49..52 normal+, 53..56 hard; direction = (info - 45) & 3
                    45..=56 => {
                        if diff_ok(diff, (info - 45) / 4) {
                            act1::spawn_20eb7(self, x, y, (info - 45) & 3);
                        }
                    }
                    57..=68 => act1::spawn_1fd9f(self, x, y, info - 57),
                    // extra stunner ammo only if Keen has less than 5 shots
                    69 => {
                        if self.gs.ammo < 5 {
                            act1::spawn_1fd9f(self, x, y, 68 - 57);
                        }
                    }
                    // keycard (bonus type 12; the original passes info - 58)
                    70 => act1::spawn_1fd9f(self, x, y, info - 58),
                    71..=73 => {
                        if diff_ok(diff, info - 71) {
                            act1::spawn_20ad4(self, x, y);
                        }
                    }
                    74..=76 => {
                        if diff_ok(diff, info - 74) {
                            act2::spawn_2193e(self, x, y);
                        }
                    }
                    77..=79 => {
                        if diff_ok(diff, info - 77) {
                            act3::spawn_2289d(self, x, y);
                        }
                    }
                    80..=83 => act1::spawn_2065f(self, x, y, info - 80, 1),
                    84..=87 => act1::spawn_1ffdc(self, x, y, info - 84, 1),
                    88..=90 => {
                        if diff_ok(diff, info - 88) {
                            act3::spawn_22a87(self, x, y);
                        }
                    }
                    99..=101 => {
                        if diff_ok(diff, info - 99) {
                            act3::spawn_22ec7(self, x, y);
                        }
                    }
                    102..=104 => {
                        if diff_ok(diff, info - 102) {
                            act3::spawn_23257(self, x, y);
                        }
                    }
                    105..=107 => {
                        if diff_ok(diff, info - 105) {
                            act3::spawn_23618(self, x, y);
                        }
                    }
                    124 => act3::spawn_238c1(self, x, y),
                    // 125 only sets lumpneeded[37]
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
        // (marking the graphics chunks of the needed lumps)
        if self.gs.mapon == 0 {
            let tile = ((self.objs[self.player].y as u16) >> 8) as i32;
            if tile < 75 || tile > 100 {
                self.close_map_elevator(24, 76);
                self.open_map_elevator(22, 55);
            }
            let ld = &self.gs.leveldone;
            if ld[4] != 0 && ld[6] != 0 && ld[8] != 0 && ld[10] != 0 && (tile > 39 || tile > 100) {
                self.open_map_elevator(26, 55);
            }
            if tile <= 39 || tile > 100 {
                self.open_map_elevator(24, 30);
            }
        }
    }

    /// OpenMapElevator (0x1ed5d): copies the open door tiles (2x2 foreground
    /// tiles at map 10,0) to the world map at (x, y).
    pub fn open_map_elevator(&mut self, x: i32, y: i32) {
        self.copy_map_elevator(10, x, y);
    }

    /// CloseMapElevator (0x1edc8): copies the closed door tiles (2x2
    /// foreground tiles at map 0,0) to the world map at (x, y).
    pub fn close_map_elevator(&mut self, x: i32, y: i32) {
        self.copy_map_elevator(0, x, y);
    }

    fn copy_map_elevator(&mut self, srcx: i32, x: i32, y: i32) {
        let mut tiles = [0u16; 4];
        for ty in 0..2 {
            for tx in 0..2 {
                tiles[(ty * 2 + tx) as usize] = self.map_tile(1, srcx + tx, ty) as u16;
            }
        }
        self.rf_mem_to_map(&tiles, 1, x, y, 2, 2);
    }

    /// gamestate.numfuses (Keen 5); kept in the spec_tiles ModState until
    /// GameState has the Keen 5 fields.
    pub fn numfuses_mut(&mut self) -> &mut i32 {
        &mut self.gs.numfuses
    }

    /// FinishedFuse (0x1f844): message after a fuse was broken
    /// (playstate ex_fusebroke).
    pub fn finished_fuse(&mut self) {
        self.sd.wait_sound_done(&mut self.sys);
        // CA_UpLevel; CA_MarkGrChunk(KEENTALK1PIC/KEENTALK2PIC); CA_CacheMarks
        self.draw_playfield();
        self.us_center_window(26, 8);
        self.tw.windoww -= 0x30;
        let x = self.tw.windowx + self.tw.windoww;
        let y = self.tw.windowy;
        self.draw_pic(x, y, KEENTALK1PIC);
        self.tw.printy += 0xc;
        if self.gs.mapon == KORATH_MAP {
            self.us_cprint("I wonder what that\nfuse was for....\n");
        } else {
            self.us_cprint("One of the four\nmachines protecting the\nmain elevator shaft--\ntoast!\n");
        }
        self.sys.present();
        self.sys.wait_vbl(30);
        self.sys.input.clear_keys_down();
        self.wait_ack();
        let x = self.tw.windowx + self.tw.windoww;
        let y = self.tw.windowy;
        self.draw_pic(x, y, KEENTALK2PIC);
        self.sys.present();
        self.sys.wait_vbl(30);
        self.sys.input.clear_keys_down();
        self.wait_ack();
        // CA_DownLevel
        self.stop_music();
    }

    /// Draws one frame of the exploding galaxy (0x1f5c0): clears the page
    /// and moves/plots every star. Stars that leave the screen stop moving
    /// (their position is not stored any more).
    fn galaxy_draw_stars(&mut self, stars: &mut [u16]) {
        self.sys.surface.clear(0);
        let (xs, rest) = stars.split_at_mut(NUMSTARS);
        let (dxs, rest) = rest.split_at_mut(NUMSTARS);
        let (ys, dys) = rest.split_at_mut(NUMSTARS);
        for i in (0..NUMSTARS).rev() {
            let x = xs[i].wrapping_add(dxs[i]);
            if x > 0xa000 {
                continue;
            }
            xs[i] = x;
            let y = ys[i].wrapping_add(dys[i]);
            if y > 0x6400 {
                continue;
            }
            ys[i] = y;
            // byte offset ylookup[y >> 7] + (x >> 10), bit (x >> 7) & 7: an
            // x of exactly 320 lands on the first pixel of the next line
            let mut px = (x >> 7) as i32;
            let mut py = (y >> 7) as i32;
            if px >= 320 {
                px -= 320;
                py += 1;
            }
            if py < 200 {
                self.sys.surface.bar(px, py, 1, 1, 15);
            }
        }
    }

    /// GameOver of Keen 5: the galaxy explodes (0x1f656).
    pub fn galaxy_explode(&mut self) {
        // FreeGraphics
        self.fade_out();
        // CA_CacheGrChunk(MILKYWAYPIC/GALAXY/GAMEOVERPIC); VW_SetLineWidth(40);
        // VW_SetScreen(0, 0); bufferofs = 0
        self.sys.surface.clear(0);
        self.draw_pic(0, 0, MILKYWAYPIC);
        // VW_ScreenToScreen(0, 0x2000, 40, 200): both pages show the galaxy
        self.fade_in();
        self.sys.input.clear_keys_down();
        self.play_sound(SND_GALAXYEXPLODEPRE);
        let mut pal = GALAXY_PALETTE;
        let mut aborted = false;
        for i in 0..18 {
            pal[8] = GALAXY_PAL8[i];
            pal[7] = GALAXY_PAL7[i];
            // int 10h AX=1002h (overscan register left as the game's border)
            self.sys.palette.regs[..16].copy_from_slice(&pal[..16]);
            self.sys.palette.regs[16] = self.sys.border;
            self.sys.wait_vbl(10);
            if self.sys.input.last_scan != 0 || !self.alive() {
                aborted = true;
                break;
            }
        }
        if !aborted {
            self.play_sound(SND_GALAXYEXPLODE);
            let chunk = super::consts::GALAXY - super::consts::STARTEXTERNS;
            let data = self.data.gfx.externs.get(chunk).cloned().unwrap_or_default();
            let mut stars = vec![0u16; NUMSTARS * 4];
            for (i, s) in stars.iter_mut().enumerate() {
                if let Some(b) = data.get(i * 2..i * 2 + 2) {
                    *s = u16::from_le_bytes([b[0], b[1]]);
                }
            }
            for _ in 0..30 {
                // two pages, each drawn and shown for 4 tics
                for _page in 0..2 {
                    self.lasttimecount = self.sys.time_count();
                    self.galaxy_draw_stars(&mut stars);
                    self.sys.present();
                    let t = self.lasttimecount + 4;
                    self.sys.wait_until(t);
                }
                if self.sys.input.last_scan != 0 || !self.alive() {
                    break;
                }
            }
        }
        // VW_ClearVideo(0); VW_SetLineWidth(64)
        self.sys.surface.clear(0);
        self.set_default_palette();
        // RF_FixOfs
        self.start_music(0x12);
        self.draw_pic(32, 80, GAMEOVERPIC);
        self.sys.present();
        self.user_input(0x690);
        self.stop_music();
    }

    pub fn got_sandwich(&mut self) {}
    pub fn got_hook(&mut self) {}
    pub fn got_passcard(&mut self) {}

    /// Keen 5 has no HitSpecialTile.
    pub fn hit_special_tile(&mut self, _ob: usize, _top: bool) {}

    /// Keen 5 has no drop splash (Keen 4 only).
    pub fn spawn_drop_splash(&mut self, _x: i32, _y: i32) {}
}

/// Difficulty variant check of ScanInfoPlane: variant 0 always spawns,
/// variant 1 needs difficulty >= normal, variant 2 hard (the original
/// falls through `if (difficulty < gd_Hard) break; if (difficulty <
/// gd_Normal) break;`).
fn diff_ok(diff: i32, variant: i32) -> bool {
    match variant {
        0 => true,
        1 => diff >= GD_NORMAL,
        _ => diff >= GD_HARD,
    }
}

