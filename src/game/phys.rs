//! CK_PHYS.C port: clipping, actor state machine, gravity, generic thinks.
//! Follows the decompiled code of KEEN6C.EXE segment 0x189b closely.

use super::defs::*;
use super::Game;

/// wallclip[8][16] (DS:0x5f2)
pub const WALLCLIP: [[i32; 16]; 8] = [
    [256; 16],
    [0; 16],
    [0, 8, 16, 24, 32, 40, 48, 56, 64, 72, 80, 88, 96, 104, 112, 120],
    [128, 136, 144, 152, 160, 168, 176, 184, 192, 200, 208, 216, 224, 232, 240, 248],
    [0, 16, 32, 48, 64, 80, 96, 112, 128, 144, 160, 176, 192, 208, 224, 240],
    [120, 112, 104, 96, 88, 80, 72, 64, 56, 48, 40, 32, 24, 16, 8, 0],
    [248, 240, 232, 224, 216, 208, 200, 192, 184, 176, 168, 160, 152, 144, 136, 128],
    [240, 224, 208, 192, 176, 160, 144, 128, 112, 96, 80, 64, 48, 32, 16, 0],
];

#[derive(Default, Clone)]
pub struct PhysTemps {
    pub xtry: i32,
    pub ytry: i32,
    pub oldtileleft: i32,
    pub oldtiletop: i32,
    pub oldtileright: i32,
    pub oldtilebottom: i32,
    pub oldtilemidx: i32,
    pub oldleft: i32,
    pub oldtop: i32,
    pub oldright: i32,
    pub oldbottom: i32,
    pub oldmidx: i32,
    pub leftmoved: i32,
    pub topmoved: i32,
    pub rightmoved: i32,
    pub bottommoved: i32,
    pub midxmoved: i32,
    pub nowall: bool,
}

/// 16-bit unsigned shift like the original `(unsigned)v >> 8`
#[inline]
pub fn u16shr(v: i32, s: i32) -> i32 {
    ((v as u16) >> s) as i32
}

impl Game {
    // ---- map access -----------------------------------------------------

    #[inline]
    pub fn fg_tile(&self, x: i32, y: i32) -> usize {
        self.map_tile(1, x, y)
    }

    #[inline]
    pub fn map_tile(&self, plane: usize, x: i32, y: i32) -> usize {
        if x < 0 || y < 0 || x >= self.mapwidth || y >= self.mapheight {
            return 0;
        }
        self.mapplanes[plane][(y * self.mapwidth + x) as usize] as usize
    }

    #[inline]
    pub fn set_map_tile(&mut self, plane: usize, x: i32, y: i32, v: u16) {
        if x < 0 || y < 0 || x >= self.mapwidth || y >= self.mapheight {
            return;
        }
        self.mapplanes[plane][(y * self.mapwidth + x) as usize] = v;
    }

    #[inline]
    pub fn north(&self, t: usize) -> i32 {
        self.data.tileinfo.north.get(t).copied().unwrap_or(0) as i32
    }
    #[inline]
    pub fn east(&self, t: usize) -> i32 {
        self.data.tileinfo.east.get(t).copied().unwrap_or(0) as i32
    }
    #[inline]
    pub fn south(&self, t: usize) -> i32 {
        self.data.tileinfo.south.get(t).copied().unwrap_or(0) as i32
    }
    #[inline]
    pub fn west(&self, t: usize) -> i32 {
        self.data.tileinfo.west.get(t).copied().unwrap_or(0) as i32
    }
    #[inline]
    pub fn intile(&self, t: usize) -> i32 {
        self.data.tileinfo.intile.get(t).copied().unwrap_or(0) as i32
    }

    pub fn sprite_info(&self, shapenum: i32) -> crate::assets::graphics::SpriteInfo {
        let idx = (shapenum - super::consts::STARTSPRITES as i32)
            .clamp(0, super::consts::NUMSPRITES as i32 - 1) as usize;
        self.data.gfx.spritetable[idx]
    }

    // ---- CK_PHYS --------------------------------------------------------

    pub fn move_obj_vert(&mut self, ob: usize, ymove: i32) {
        let o = &mut self.objs[ob];
        o.y += ymove;
        o.top += ymove;
        o.bottom += ymove;
        o.tiletop = u16shr(o.top, 8);
        o.tilebottom = u16shr(o.bottom, 8);
    }

    pub fn move_obj_horz(&mut self, ob: usize, xmove: i32) {
        let o = &mut self.objs[ob];
        o.x += xmove;
        o.left += xmove;
        o.right += xmove;
        o.tileleft = u16shr(o.left, 8);
        o.tileright = u16shr(o.right, 8);
    }

    /// Lets Keen walk off the edge of a ledge onto a lower ledge.
    fn player_bottom_kludge(&mut self, ob: usize) {
        let o = self.objs[ob].clone();
        let y = o.tilebottom - 1;
        let (xpix, xmove, facetile, below);
        if o.xdir == 1 {
            xpix = 0;
            xmove = o.right - o.midx;
            let x = o.tileright;
            // the tile above the destination must be clear of walls
            if self.west(self.fg_tile(x, y - 1)) != 0 {
                return;
            }
            facetile = self.west(self.fg_tile(x, y));
            below = self.fg_tile(x, y);
        } else {
            xpix = 15;
            xmove = o.left - o.midx;
            let x = o.tileleft;
            if self.east(self.fg_tile(x, y - 1)) != 0 {
                return;
            }
            facetile = self.east(self.fg_tile(x, y));
            below = self.fg_tile(x, y);
        }
        if facetile != 0 || self.north(below) != 0 {
            return;
        }
        let x = if o.xdir == 1 { o.tileright } else { o.tileleft };
        let tile = self.fg_tile(x, y + 1);
        if self.north(tile) != 1 {
            return;
        }
        let ymove = (o.tilebottom * 256 + WALLCLIP[1][xpix] - 1) - o.bottom;
        if ymove < 1 && (-ymove == self.ph.bottommoved || -self.ph.bottommoved < ymove) {
            self.objs[ob].hitnorth = 1;
            self.move_obj_vert(ob, ymove);
            self.move_obj_horz(ob, xmove);
        }
    }

    fn player_top_kludge(&mut self, ob: usize) {
        let o = self.objs[ob].clone();
        let y = o.tiletop + 1;
        let (x, xpix) = if o.xdir == 1 { (o.tileright, 0usize) } else { (o.tileleft, 15usize) };
        let (wall_here, wall_down) = if o.xdir == 1 {
            (self.west(self.fg_tile(x, y + 1)), self.west(self.fg_tile(x, y + 2)))
        } else {
            (self.east(self.fg_tile(x, y + 1)), self.east(self.fg_tile(x, y + 2)))
        };
        if wall_here != 0 {
            return;
        }
        if wall_down != 0 {
            return;
        }
        if self.south(self.fg_tile(x, y)) != 0 {
            return;
        }
        let wall = self.south(self.fg_tile(x, y - 1));
        if wall != 0 {
            let ymove = ((o.tiletop + 1) * 256 - WALLCLIP[(wall & 7) as usize][xpix]) - o.top;
            if ymove >= 0 && ymove <= -self.ph.topmoved {
                self.objs[ob].hitsouth = wall;
                self.move_obj_vert(ob, ymove);
            }
        }
    }

    fn clip_to_ends(&mut self, ob: usize) {
        let o = self.objs[ob].clone();
        let midxpix = (u16shr(o.midx & 0xf0, 4)) as usize;
        let absmid = self.ph.midxmoved.abs();
        let maxmove = -absmid - self.ph.bottommoved - 16;
        let mut y = self.ph.oldtilebottom - 1;
        while y <= o.tilebottom {
            let wall = self.north(self.fg_tile(o.tilemidx, y));
            if wall != 0 {
                let mv = (y * 256 + WALLCLIP[(wall & 7) as usize][midxpix] - 1) - self.objs[ob].bottom;
                if mv < 0 && mv >= maxmove {
                    self.objs[ob].hitnorth = wall;
                    self.move_obj_vert(ob, mv);
                    return;
                }
            }
            y += 1;
        }
        let maxmove = absmid - self.ph.topmoved;
        let mut y = self.ph.oldtiletop + 1;
        while y >= self.objs[ob].tiletop {
            let wall = self.south(self.fg_tile(o.tilemidx, y));
            if wall != 0 {
                let mv = ((y + 1) * 256 - WALLCLIP[(wall & 7) as usize][midxpix]) - self.objs[ob].top;
                if mv > 0 && mv <= maxmove + 16 {
                    let total = self.ph.ytry + mv;
                    if total < 256 && total > -256 {
                        self.objs[ob].hitsouth = wall;
                        self.move_obj_vert(ob, mv);
                    }
                }
            }
            y -= 1;
        }
    }

    fn clip_to_sides(&mut self, ob: usize) {
        let o = self.objs[ob].clone();
        let mut top = o.tiletop;
        if o.hitsouth > 1 {
            top += 1;
        }
        let mut bottom = o.tilebottom;
        if o.hitnorth > 1 {
            bottom -= 1;
        }
        let mut y = top;
        while y <= bottom {
            let w = self.east(self.fg_tile(o.tileleft, y));
            self.objs[ob].hiteast = w;
            if w != 0 {
                let mv = (o.tileleft + 1) * 256 - o.left;
                self.move_obj_horz(ob, mv);
                return;
            }
            y += 1;
        }
        let mut y = top;
        while y <= bottom {
            let w = self.west(self.fg_tile(o.tileright, y));
            self.objs[ob].hitwest = w;
            if w != 0 {
                let mv = (o.tileright * 256 - 1) - o.right;
                self.move_obj_horz(ob, mv);
                return;
            }
            y += 1;
        }
    }

    /// Returns true if the object's bounding tiles contain no walls.
    pub fn check_position(&self, ob: usize) -> bool {
        let o = &self.objs[ob];
        for y in o.tiletop..=o.tilebottom {
            for x in o.tileleft..=o.tileright {
                let t = self.fg_tile(x, y);
                if self.north(t) != 0 || self.east(t) != 0 || self.south(t) != 0 || self.west(t) != 0 {
                    return false;
                }
            }
        }
        true
    }

    /// Sets the shape of a state and checks if the object fits.
    pub fn state_position_ok(&mut self, ob: usize, st: StateRef) -> bool {
        let shape = if self.objs[ob].xdir > 0 { st.right } else { st.left };
        self.objs[ob].shapenum = shape;
        let s = self.sprite_info(shape);
        let o = &mut self.objs[ob];
        o.left = o.x + s.xl as i32;
        o.right = o.x + s.xh as i32;
        o.top = o.y + s.yl as i32;
        o.bottom = o.y + s.yh as i32;
        o.midx = o.left + u16shr(o.right - o.left, 1);
        o.tileleft = u16shr(o.left, 8);
        o.tileright = u16shr(o.right, 8);
        o.tiletop = u16shr(o.top, 8);
        o.tilebottom = u16shr(o.bottom, 8);
        o.tilemidx = u16shr(o.midx, 8);
        self.check_position(ob)
    }

    fn calc_bounds(&mut self, ob: usize) {
        let s = self.sprite_info(self.objs[ob].shapenum);
        let o = &mut self.objs[ob];
        o.left = o.x + s.xl as i32;
        o.right = o.x + s.xh as i32;
        o.top = o.y + s.yl as i32;
        o.bottom = o.y + s.yh as i32;
        o.midx = o.left + u16shr(o.right - o.left, 1);
        o.tileleft = u16shr(o.left, 8);
        o.tileright = u16shr(o.right, 8);
        o.tiletop = u16shr(o.top, 8);
        o.tilebottom = u16shr(o.bottom, 8);
        o.tilemidx = u16shr(o.midx, 8);
    }

    fn save_old(&mut self, ob: usize) {
        let o = &self.objs[ob];
        let p = &mut self.ph;
        p.oldtileright = o.tileright;
        p.oldtiletop = o.tiletop;
        p.oldtileleft = o.tileleft;
        p.oldtilebottom = o.tilebottom;
        p.oldtilemidx = o.tilemidx;
        p.oldright = o.right;
        p.oldtop = o.top;
        p.oldleft = o.left;
        p.oldbottom = o.bottom;
        p.oldmidx = o.midx;
    }

    fn calc_moved(&mut self, ob: usize) {
        let o = &self.objs[ob];
        let p = &mut self.ph;
        p.leftmoved = o.left - p.oldleft;
        p.rightmoved = o.right - p.oldright;
        p.topmoved = o.top - p.oldtop;
        p.bottommoved = o.bottom - p.oldbottom;
        p.midxmoved = o.midx - p.oldmidx;
    }

    pub fn clip_to_walls(&mut self, ob: usize) {
        self.clip_to_walls_push(ob, None);
    }

    /// ClipToWalls; `push_override` replaces state->pushtofloor (the original
    /// temporarily patches the state table).
    pub fn clip_to_walls_push(&mut self, ob: usize, push_override: Option<bool>) {
        let oldx = self.objs[ob].x;
        let oldy = self.objs[ob].y;
        let mut pushed = false;
        let push = push_override.unwrap_or_else(|| self.objs[ob].state.map(|s| s.pushtofloor).unwrap_or(false));
        if push {
            if self.objs[ob].hitnorth == 25 {
                self.ph.ytry = 145;
            } else {
                self.ph.ytry = self.ph.xtry.abs() + 16;
                pushed = true;
            }
        }
        self.ph.xtry = self.ph.xtry.clamp(-239, 239);
        self.ph.ytry = self.ph.ytry.clamp(-239, 255);
        self.objs[ob].x += self.ph.xtry;
        self.objs[ob].y += self.ph.ytry;
        self.objs[ob].needtoreact = true;
        if self.objs[ob].shapenum == 0 {
            return;
        }
        self.save_old(ob);
        self.calc_bounds(ob);
        {
            let o = &mut self.objs[ob];
            o.hitnorth = 0;
            o.hiteast = 0;
            o.hitsouth = 0;
            o.hitwest = 0;
        }
        if self.objs[ob].needtoclip != 0 {
            self.calc_moved(ob);
            self.clip_to_ends(ob);
            if ob == self.player && !self.ph.nowall {
                if self.objs[ob].hitnorth == 0 && self.ph.bottommoved > 0 {
                    self.player_bottom_kludge(ob);
                }
                if self.objs[ob].hitsouth == 0 && self.ph.topmoved < 0 {
                    self.player_top_kludge(ob);
                }
            }
            self.clip_to_sides(ob);
        }
        if pushed && self.objs[ob].hitnorth == 0 {
            // didn't land on anything, so no push
            self.objs[ob].y = oldy;
            self.objs[ob].x = oldx + self.ph.xtry;
            self.calc_bounds(ob);
        }
        let o = &mut self.objs[ob];
        o.xmove += o.x - oldx;
        o.ymove += o.y - oldy;
    }

    /// Used by objects that need a full 2x2 tile box (satellite/rocket).
    pub fn full_clip_to_walls(&mut self, ob: usize) {
        let oldx = self.objs[ob].x;
        let oldy = self.objs[ob].y;
        self.ph.xtry = self.ph.xtry.clamp(-239, 239);
        self.ph.ytry = self.ph.ytry.clamp(-239, 239);
        self.objs[ob].x += self.ph.xtry;
        self.objs[ob].y += self.ph.ytry;
        self.objs[ob].needtoreact = true;
        let (w, h) = match super::ep::fullclip_box(self.objs[ob].obclass) {
            Some(b) => b,
            None => {
                self.quit("FullClipToWalls: Bad obclass");
                (0x200, 0x200)
            }
        };
        {
            let o = &mut self.objs[ob];
            o.right = o.x + w;
            o.left = o.x;
            o.top = o.y;
            o.bottom = o.y + h;
            o.tileleft = u16shr(o.left, 8);
            o.tileright = u16shr(o.right, 8);
            o.tiletop = u16shr(o.top, 8);
            o.tilebottom = u16shr(o.bottom, 8);
            o.hitnorth = 0;
            o.hiteast = 0;
            o.hitsouth = 0;
            o.hitwest = 0;
        }
        if !self.check_position(ob) {
            let xt = self.ph.xtry;
            let yt = self.ph.ytry;
            self.move_obj_horz(ob, -xt);
            if !self.check_position(ob) {
                if yt > 0 {
                    self.objs[ob].hitnorth = 1;
                } else {
                    self.objs[ob].hitsouth = 1;
                }
                self.move_obj_horz(ob, xt);
                self.move_obj_vert(ob, -yt);
                if !self.check_position(ob) {
                    self.move_obj_horz(ob, -xt);
                    if xt > 0 {
                        self.objs[ob].hitwest = 1;
                    } else {
                        self.objs[ob].hiteast = 1;
                    }
                }
            } else if xt > 0 {
                self.objs[ob].hitwest = 1;
            } else {
                self.objs[ob].hiteast = 1;
            }
        }
        let s = self.sprite_info(self.objs[ob].shapenum);
        let o = &mut self.objs[ob];
        o.xmove += o.x - oldx;
        o.ymove += o.y - oldy;
        o.left = o.x + s.xl as i32;
        o.right = o.x + s.xh as i32;
        o.top = o.y + s.yl as i32;
        o.bottom = o.y + s.yh as i32;
        o.midx = o.left + u16shr(o.right - o.left, 1);
    }

    /// Moves an object without wall clipping when it was pushed by another.
    pub fn push_obj(&mut self, ob: usize) {
        let oldx = self.objs[ob].x;
        let oldy = self.objs[ob].y;
        self.objs[ob].x += self.ph.xtry;
        self.objs[ob].y += self.ph.ytry;
        self.objs[ob].needtoreact = true;
        if self.objs[ob].shapenum != 0 {
            self.save_old(ob);
            self.calc_bounds(ob);
            if self.objs[ob].needtoclip != 0 {
                self.calc_moved(ob);
                self.clip_to_ends(ob);
                self.clip_to_sides(ob);
            }
        }
        let o = &mut self.objs[ob];
        o.xmove += o.x - oldx;
        o.ymove += o.y - oldy;
    }

    pub fn clip_to_sprite_side(&mut self, push: usize, solid: usize) {
        let xmove = self.objs[solid].xmove - self.objs[push].xmove;
        let leftinto = self.objs[solid].right - self.objs[push].left;
        let rightinto = self.objs[push].right - self.objs[solid].left;
        let pushtofloor = self.objs[push].state.map(|s| s.pushtofloor).unwrap_or(false);
        if leftinto > 0 && leftinto <= xmove {
            self.ph.xtry = leftinto;
            if pushtofloor {
                self.ph.ytry = leftinto + 16;
            }
            self.clip_to_walls(push);
            self.objs[push].hiteast = 1;
            return;
        }
        if rightinto > 0 && rightinto <= -xmove {
            self.ph.xtry = -rightinto;
            if pushtofloor {
                self.ph.ytry = rightinto + 16;
            }
            self.clip_to_walls(push);
            self.objs[push].hitwest = 1;
        }
    }

    pub fn clip_to_sprite_top(&mut self, push: usize, solid: usize) {
        let ymove = self.objs[push].ymove - self.objs[solid].ymove;
        let bottominto = self.objs[push].bottom - self.objs[solid].top;
        if bottominto >= 0 && bottominto <= ymove {
            if push == self.player {
                self.gs.riding = Some(solid);
            }
            self.ph.ytry = -bottominto;
            self.clip_to_walls_push(push, Some(false));
            if self.objs[push].hitsouth == 0 {
                self.objs[push].hitnorth = 25;
            }
        }
    }

    pub fn clip_to_sprite(&mut self, push: usize, solid: usize, squish: bool) {
        let xmove = self.objs[solid].xmove - self.objs[push].xmove;
        self.ph.xtry = 0;
        self.ph.ytry = 0;
        let leftinto = self.objs[solid].right - self.objs[push].left;
        let rightinto = self.objs[push].right - self.objs[solid].left;
        if leftinto > 0 && leftinto <= xmove + 1 {
            self.ph.xtry = leftinto;
            self.objs[push].xspeed = 0;
            self.push_obj(push);
            if squish && self.objs[push].hitwest != 0 {
                self.kill_keen();
            }
            self.objs[push].hiteast = 1;
            return;
        }
        if rightinto > 0 && rightinto <= 1 - xmove {
            self.ph.xtry = -rightinto;
            self.objs[push].xspeed = 0;
            self.push_obj(push);
            if squish && self.objs[push].hiteast != 0 {
                self.kill_keen();
            }
            self.objs[push].hitwest = 1;
            return;
        }
        let ymove = self.objs[push].ymove - self.objs[solid].ymove;
        let topinto = self.objs[solid].bottom - self.objs[push].top;
        let bottominto = self.objs[push].bottom - self.objs[solid].top;
        if bottominto >= 0 && bottominto <= ymove {
            if push == self.player {
                self.gs.riding = Some(solid);
            }
            self.ph.ytry = -bottominto;
            self.push_obj(push);
            if squish && self.objs[push].hitsouth != 0 {
                self.kill_keen();
            }
            if self.objs[push].hitsouth == 0 {
                self.objs[push].hitnorth = 25;
            }
            return;
        }
        if topinto >= 0 && topinto <= ymove {
            self.ph.ytry = topinto;
            self.clip_to_walls(push);
            if squish && self.objs[push].hitnorth != 0 {
                self.kill_keen();
            }
            self.objs[push].hitsouth = 25;
        }
    }

    /// Returns the excess tics after a state change (0 if still in state).
    fn do_actor(&mut self, ob: usize, numtics: i32) -> i32 {
        let Some(state) = self.objs[ob].state else {
            // a NULL state (see change_state_null): the original reads the
            // Borland copyright at DS:0 as the state record; with its
            // progress and tictime (8292) it only counts tics
            self.objs[ob].ticcount += numtics;
            return 0;
        };
        if state.progress == Progress::Think {
            if let Some(think) = state.think {
                if self.objs[ob].nothink != 0 {
                    self.objs[ob].nothink -= 1;
                } else {
                    think(self, ob);
                }
            }
            return 0;
        }
        let ticcount = self.objs[ob].ticcount + numtics;
        if state.tictime > ticcount || state.tictime == 0 {
            self.objs[ob].ticcount = ticcount;
            if state.progress == Progress::Slide || state.progress == Progress::SlideThink {
                self.apply_state_move(ob, state, numtics);
            }
            if state.progress == Progress::StepThink || state.progress == Progress::SlideThink {
                if let Some(think) = state.think {
                    if self.objs[ob].nothink != 0 {
                        self.objs[ob].nothink -= 1;
                    } else {
                        think(self, ob);
                    }
                }
            }
            return 0;
        }
        // finished the state
        let usedtics = state.tictime - self.objs[ob].ticcount;
        let excess = ticcount - state.tictime;
        self.objs[ob].ticcount = 0;
        if state.progress == Progress::Slide || state.progress == Progress::SlideThink {
            self.apply_state_move(ob, state, usedtics);
        } else {
            let o = &self.objs[ob];
            if o.xdir != 0 {
                self.ph.xtry += if o.xdir == 1 { state.xmove } else { -state.xmove };
            }
            if o.ydir != 0 {
                self.ph.ytry += if o.ydir == 1 { state.ymove } else { -state.ymove };
            }
        }
        if let Some(think) = state.think {
            if self.objs[ob].nothink != 0 {
                self.objs[ob].nothink -= 1;
            } else {
                think(self, ob);
            }
        }
        if same(self.objs[ob].state, state) {
            self.objs[ob].state = state.next;
        } else if self.objs[ob].state.is_none() {
            return 0;
        }
        excess
    }

    fn apply_state_move(&mut self, ob: usize, state: StateRef, tics: i32) {
        let o = &self.objs[ob];
        if o.xdir != 0 {
            self.ph.xtry += if o.xdir == 1 { state.xmove * tics } else { -tics * state.xmove };
        }
        if o.ydir != 0 {
            self.ph.ytry += if o.ydir == 1 { state.ymove * tics } else { -tics * state.ymove };
        }
    }

    pub fn state_machine(&mut self, ob: usize) {
        self.ph.xtry = 0;
        self.ph.ytry = 0;
        self.objs[ob].xmove = 0;
        self.objs[ob].ymove = 0;
        let oldshapenum = self.objs[ob].shapenum;
        let mut state: Option<StateRef> = self.objs[ob].state;
        let mut excesstics = self.do_actor(ob, self.tics);
        if !opt_same(self.objs[ob].state, state) {
            self.objs[ob].ticcount = 0;
            state = self.objs[ob].state;
        }
        while excesstics != 0 {
            // passed through to the next state
            let Some(st) = state else { break };
            if !st.skippable && st.tictime <= excesstics {
                excesstics = self.do_actor(ob, st.tictime - 1);
            } else {
                excesstics = self.do_actor(ob, excesstics);
            }
            if !opt_same(self.objs[ob].state, state) {
                self.objs[ob].ticcount = 0;
                state = self.objs[ob].state;
            }
        }
        let Some(state) = self.objs[ob].state else {
            self.remove_obj(ob);
            return;
        };
        if state.right != 0 {
            self.objs[ob].shapenum = if self.objs[ob].xdir > 0 { state.right } else { state.left };
        }
        if self.objs[ob].shapenum == -1 {
            self.objs[ob].shapenum = 0;
        }
        if self.ph.xtry != 0 || self.ph.ytry != 0 || self.objs[ob].shapenum != oldshapenum || self.objs[ob].hitnorth == 25
        {
            if self.objs[ob].needtoclip == CL_FULLCLIP {
                self.full_clip_to_walls(ob);
            } else {
                self.clip_to_walls(ob);
            }
        }
    }

    /// ChangeState(ob, NULL), which the original does e.g. in R_Bounce when
    /// the state has no next state: the "state" is then the Borland
    /// copyright at DS:0 (no shapes, pushtofloor set), and StateMachine
    /// removes the object in the next frame.
    pub fn change_state_null(&mut self, ob: usize) {
        self.objs[ob].state = None;
        self.objs[ob].ticcount = 0;
        if self.objs[ob].shapenum == -1 {
            self.objs[ob].shapenum = 0;
        }
        self.objs[ob].needtoreact = true;
        self.ph.xtry = 0;
        self.ph.ytry = 0;
        if self.objs[ob].hitnorth != 25 {
            self.clip_to_walls_push(ob, Some(true));
        }
    }

    pub fn new_state(&mut self, ob: usize, st: StateRef) {
        self.objs[ob].state = Some(st);
        if st.right != 0 {
            self.objs[ob].shapenum = if self.objs[ob].xdir > 0 { st.right } else { st.left };
        }
        if self.objs[ob].shapenum == -1 {
            self.objs[ob].shapenum = 0;
        }
        let temp = self.objs[ob].needtoclip;
        self.objs[ob].needtoclip = CL_NOCLIP;
        self.ph.xtry = 0;
        self.ph.ytry = 0;
        self.clip_to_walls(ob);
        self.objs[ob].needtoclip = temp;
        if temp == CL_FULLCLIP {
            self.full_clip_to_walls(ob);
        } else if temp == CL_MIDCLIP {
            self.clip_to_walls(ob);
        }
    }

    pub fn change_state(&mut self, ob: usize, st: StateRef) {
        self.objs[ob].state = Some(st);
        self.objs[ob].ticcount = 0;
        if st.right != 0 {
            self.objs[ob].shapenum = if self.objs[ob].xdir > 0 { st.right } else { st.left };
        }
        if self.objs[ob].shapenum == -1 {
            self.objs[ob].shapenum = 0;
        }
        self.objs[ob].needtoreact = true;
        self.ph.xtry = 0;
        self.ph.ytry = 0;
        if self.objs[ob].hitnorth != 25 {
            self.clip_to_walls(ob);
        }
    }

    pub fn on_screen(&self, ob: usize) -> bool {
        let o = &self.objs[ob];
        let c = &self.cam;
        !((o.tileright as u16) < c.originxtile as u16
            || (o.tilebottom as u16) < c.originytile as u16
            || (o.tileleft as u16) > c.originxtilemax as u16
            || (o.tiletop as u16) > c.originytilemax as u16)
    }

    /// Iterates the tics of this frame like `for (i=lasttimecount-tics; i<lasttimecount; i++)`
    /// (32-bit TimeCount values).
    fn tic_range(&self) -> std::ops::Range<i64> {
        let last = self.lasttimecount as i64;
        (last - self.tics as i64)..last
    }

    pub fn do_gravity(&mut self, ob: usize) {
        for i in self.tic_range() {
            let o = &mut self.objs[ob];
            if i & 1 != 0 {
                if o.yspeed < 0 && o.yspeed >= -4 {
                    self.ph.ytry += o.yspeed;
                    o.yspeed = 0;
                    return;
                }
                o.yspeed += 4;
                if o.yspeed > 70 {
                    o.yspeed = 70;
                }
            }
            self.ph.ytry += o.yspeed;
        }
    }

    pub fn do_weak_gravity(&mut self, ob: usize) {
        for i in self.tic_range() {
            let o = &mut self.objs[ob];
            if i & 1 != 0 {
                if o.yspeed < 0 && o.yspeed >= -3 {
                    self.ph.ytry += o.yspeed;
                    o.yspeed = 0;
                    return;
                }
                o.yspeed += 3;
                if o.yspeed > 70 {
                    o.yspeed = 70;
                }
            }
            self.ph.ytry += o.yspeed;
        }
    }

    /// The original tests `!i%4`, which is only true for i == 0 (kept as is).
    pub fn do_tiny_gravity(&mut self, ob: usize) {
        for i in self.tic_range() {
            let o = &mut self.objs[ob];
            if i as u32 == 0 {
                o.yspeed += 1;
                if o.yspeed > 70 {
                    o.yspeed = 70;
                }
            }
            self.ph.ytry += o.yspeed;
        }
    }

    pub fn accelerate_x(&mut self, ob: usize, dir: i32, max: i32) {
        let mut oldsign = self.objs[ob].xspeed & 0x8000;
        for i in self.tic_range() {
            let o = &mut self.objs[ob];
            if i & 1 != 0 {
                o.xspeed += dir;
                if (o.xspeed & 0x8000) != oldsign {
                    oldsign = o.xspeed & 0x8000;
                    o.xdir = if oldsign == 0 { 1 } else { -1 };
                }
                if o.xspeed > max {
                    o.xspeed = max;
                } else if o.xspeed < -max {
                    o.xspeed = -max;
                }
            }
            self.ph.xtry += o.xspeed;
        }
    }

    pub fn accelerate_xv(&mut self, ob: usize, dir: i32, max: i32) {
        for i in self.tic_range() {
            let o = &mut self.objs[ob];
            if i & 1 != 0 {
                o.xspeed += dir;
                if o.xspeed > max {
                    o.xspeed = max;
                } else if o.xspeed < -max {
                    o.xspeed = -max;
                }
            }
            self.ph.xtry += o.xspeed;
        }
    }

    pub fn accelerate_y(&mut self, ob: usize, dir: i32, max: i32) {
        for i in self.tic_range() {
            let o = &mut self.objs[ob];
            if i & 1 != 0 {
                o.yspeed += dir;
                if o.yspeed > max {
                    o.yspeed = max;
                } else if o.yspeed < -max {
                    o.yspeed = -max;
                }
            }
            self.ph.ytry += o.yspeed;
        }
    }

    pub fn friction_x(&mut self, ob: usize) {
        let oldsign = self.objs[ob].xspeed & 0x8000;
        let xs = self.objs[ob].xspeed;
        let d = if xs > 0 { -1 } else if xs < 0 { 1 } else { 0 };
        for i in self.tic_range() {
            let o = &mut self.objs[ob];
            if i & 1 != 0 {
                o.xspeed += d;
                if (o.xspeed & 0x8000) != oldsign {
                    o.xspeed = 0;
                }
            }
            self.ph.xtry += o.xspeed;
        }
    }

    pub fn friction_y(&mut self, ob: usize) {
        let oldsign = self.objs[ob].yspeed & 0x8000;
        let ys = self.objs[ob].yspeed;
        let d = if ys > 0 { -1 } else if ys < 0 { 1 } else { 0 };
        for i in self.tic_range() {
            let o = &mut self.objs[ob];
            if i & 1 != 0 {
                o.yspeed += d;
                if (o.yspeed & 0x8000) != oldsign {
                    o.yspeed = 0;
                }
            }
            self.ph.ytry += o.yspeed;
        }
    }

    /// StunObj: switch an enemy to its stunned state.
    pub fn stun_obj(&mut self, ob: usize, shot: usize, stunstate: StateRef) {
        self.explode_shot(shot);
        {
            let o = &mut self.objs[ob];
            o.temp1 = 0;
            o.temp2 = 0;
            o.temp3 = 0;
            o.temp4 = o.obclass;
        }
        self.change_state(ob, stunstate);
        let o = &mut self.objs[ob];
        o.obclass = STUNNEDOBJ;
        if super::ep::STUN_HOP {
            o.yspeed -= 24;
            if o.yspeed < -48 {
                o.yspeed = -48;
            }
        }
    }

    pub fn quit(&mut self, msg: &str) {
        eprintln!("{}", msg);
        self.request_quit = true;
    }
}

#[inline]
pub fn opt_same(a: Option<StateRef>, b: Option<StateRef>) -> bool {
    match (a, b) {
        (Some(x), Some(y)) => std::ptr::eq(x, y),
        (None, None) => true,
        _ => false,
    }
}

// ---- generic think / contact / react functions ---------------------------

pub fn t_projectile(g: &mut Game, ob: usize) {
    g.do_gravity(ob);
    let xs = g.objs[ob].xspeed * g.tics;
    g.ph.xtry = xs;
}

pub fn t_weak_projectile(g: &mut Game, ob: usize) {
    g.do_weak_gravity(ob);
    let xs = g.objs[ob].xspeed * g.tics;
    g.ph.xtry = xs;
}

pub fn t_velocity(g: &mut Game, ob: usize) {
    g.ph.xtry = g.objs[ob].xspeed * g.tics;
    g.ph.ytry = g.objs[ob].yspeed * g.tics;
}

pub fn set_react_think(g: &mut Game, ob: usize) {
    g.objs[ob].needtoreact = true;
}

/// Cycles the stun stars (temp2 = frame 0..2).
pub fn t_stunned(g: &mut Game, ob: usize) {
    let o = &mut g.objs[ob];
    o.temp1 = 0;
    o.needtoreact = true;
    o.temp2 += 1;
    if o.temp2 == 3 {
        o.temp2 = 0;
    }
}

pub fn c_lethal(g: &mut Game, _ob: usize, hit: usize) {
    if g.objs[hit].obclass == KEENOBJ {
        g.kill_keen();
    }
}

pub fn r_draw(g: &mut Game, ob: usize) {
    g.place_sprite(ob);
}

fn walk_turn(g: &mut Game, ob: usize, normal: bool) {
    let o = &g.objs[ob];
    if o.xdir == 1 && o.hitwest != 0 {
        g.objs[ob].x -= g.objs[ob].xmove;
        g.objs[ob].xdir = -1;
    } else if o.xdir == -1 && o.hiteast != 0 {
        g.objs[ob].x -= g.objs[ob].xmove;
        g.objs[ob].xdir = 1;
    } else if (!normal && o.hitnorth == 0) || (normal && (o.hitnorth == 0 || (o.hitnorth & !7) != 0)) {
        g.objs[ob].x -= g.objs[ob].xmove * 2;
        g.objs[ob].xdir = -g.objs[ob].xdir;
    } else {
        g.place_sprite(ob);
        return;
    }
    g.objs[ob].nothink = (g.rnd() >> 5) as i32;
    let st = g.objs[ob].st();
    g.change_state(ob, st);
    g.place_sprite(ob);
}

pub fn r_walk(g: &mut Game, ob: usize) {
    walk_turn(g, ob, false);
}

pub fn r_walk_normal(g: &mut Game, ob: usize) {
    walk_turn(g, ob, true);
}

/// R_Stunned (0x1a1c1): stunned enemies fall and show rotating stars.
pub fn r_stunned(g: &mut Game, ob: usize) {
    {
        let o = &mut g.objs[ob];
        if o.hitwest != 0 || o.hiteast != 0 {
            o.xspeed = 0;
        }
        if o.hitsouth != 0 {
            o.yspeed = 0;
        }
    }
    if g.objs[ob].hitnorth != 0 {
        g.objs[ob].xspeed = 0;
        g.objs[ob].yspeed = 0;
        if let Some(next) = g.objs[ob].st().next {
            g.change_state(ob, next);
        }
    }
    g.place_sprite(ob);
    // star position depends on the original class of the object
    let (dx, dy) = match super::ep::stun_star_offset(g.objs[ob].temp4) {
        Some(d) => d,
        None => {
            g.quit("No star spec for object!");
            (0, 0)
        }
    };
    let o = &mut g.objs[ob];
    o.temp1 += g.tics;
    if o.temp1 > 10 {
        o.temp1 -= 10;
        o.temp2 += 1;
        if o.temp2 == 3 {
            o.temp2 = 0;
        }
    }
    let (x, y, shape) = (o.x + dx, o.y + dy, o.temp2 + super::ep::STARSPR);
    g.place_stars(ob, x, y, shape, 3);
}
