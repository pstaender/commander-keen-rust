//! Refresh manager (ID_RF equivalent): camera origin, scroll blocks,
//! animated tiles and drawing of the play field with sprites.
//!
//! The original only redraws changed tiles; we redraw the whole view each
//! refresh, which produces the same picture.

use super::defs::*;
use super::Game;
use super::consts::{NUMTILE16, NUMTILE16M, STARTSPRITES};

pub const PRIORITIES: i32 = 4;
pub const MASKEDTILEPRIORITY: i32 = 3;

#[derive(Default, Clone)]
pub struct Camera {
    pub originxglobal: i32,
    pub originyglobal: i32,
    pub originxtile: i32,
    pub originytile: i32,
    pub originxtilemax: i32,
    pub originytilemax: i32,
    pub originxmin: i32,
    pub originxmax: i32,
    pub originymin: i32,
    pub originymax: i32,
    pub inactivateleft: i32,
    pub inactivateright: i32,
    pub inactivatetop: i32,
    pub inactivatebottom: i32,
    /// x positions of vertical scroll blocks
    pub vblocks: Vec<i32>,
    /// y positions of horizontal scroll blocks
    pub hblocks: Vec<i32>,
    /// logical width of the play field view in pixels (0 = original 320)
    pub view: i32,
}

impl Camera {
    /// view width in pixels
    pub fn view_w(&self) -> i32 {
        if self.view == 0 {
            320
        } else {
            self.view
        }
    }

    /// whole tiles covered by the view (20 for the original view)
    pub fn view_tiles(&self) -> i32 {
        (self.view_w() + 15) / 16
    }

    pub fn wide(&self) -> bool {
        self.view_w() != 320
    }

    /// extra view width beyond the original, in global units
    pub fn extra(&self) -> i32 {
        (self.view_w() - 320) * 16
    }

    /// If the room around the player (the space between the scroll blocks
    /// left and right of `px`, or the map edges) is narrower than the view,
    /// the x origin that centers it.
    pub fn narrow_room_x(&self, px: i32, mapwidth: i32) -> Option<i32> {
        let wg = self.view_w() * 16;
        let left = self.vblocks.iter().copied().filter(|&b| (b + 1) * 256 <= px).max().unwrap_or(-1);
        let right = self.vblocks.iter().copied().filter(|&b| b * 256 > px).min().unwrap_or(mapwidth);
        let (l, r) = ((left + 1) * 256, right * 256);
        (r - l < wg).then(|| (l + r) / 2 - wg / 2)
    }
}

#[derive(Clone, Copy, Default)]
pub struct SpriteSlot {
    pub visible: bool,
    pub x: i32,
    pub y: i32,
    pub shape: i32,
    pub priority: i32,
    /// order of placement (later placed = drawn later within a priority)
    pub order: u32,
    /// RF maskdraw: draw the sprite's shape in white
    pub white: bool,
}

/// One animation group (allanims entry).
#[derive(Clone, Copy)]
pub struct AnimTile {
    /// current tile number; foreground tiles have bit 15 set
    pub current: u32,
    pub countdown: i32,
    /// tile of the cycle that plays `sound` when reached (ID_RF animsounds)
    pub sound_tile: u32,
    pub sound: i32,
    /// number of visible map spots using this animation
    pub visible: i32,
}

/// ID_RF tile animation sounds (far data at 0x2e1b0 in KEEN6C.EXE):
/// foreground tiles 0x868 and 0x8a0 play sounds 58 and 59. Keen 4 and 5
/// have no such table.
const ANIMSOUNDS: &[(u32, i32)] = ep_sel!(&[], &[], &[(0x8868, 58), (0x88a0, 59)]);

pub struct RfState {
    pub anims: Vec<AnimTile>,
    /// anim group of each background / foreground tile number
    pub bg_group: Vec<i32>,
    pub fg_group: Vec<i32>,
    pub sprites: Vec<SpriteSlot>,
    pub stars: Vec<SpriteSlot>,
    pub order: u32,
    /// status window refresh hook active (StatusWindow scrolling)
    pub hook: Option<fn(&mut Game)>,
    /// drawn after the tiles, before the sprites (master screen drawing)
    pub bg_hook: Option<fn(&mut Game)>,
}

impl Default for RfState {
    fn default() -> Self {
        RfState {
            anims: Vec::new(),
            bg_group: vec![-1; NUMTILE16],
            fg_group: vec![-1; NUMTILE16M],
            sprites: vec![SpriteSlot::default(); MAXACTORS + 1],
            stars: vec![SpriteSlot::default(); MAXACTORS + 1],
            order: 0,
            hook: None,
            bg_hook: None,
        }
    }
}

impl Game {
    // ---- camera ---------------------------------------------------------

    fn calc_origin_stuff(&mut self, x: i32, y: i32) {
        let c = &mut self.cam;
        c.originxglobal = x;
        c.originyglobal = y;
        // a wide view centered on a narrow room can start left of the map
        c.originxtile = if x < 0 && c.wide() { 0 } else { (x as u16 >> 8) as i32 };
        c.originytile = (y as u16 >> 8) as i32;
    }

    /// RF_NewPosition
    pub fn rf_new_position(&mut self, x: i32, y: i32) {
        let c = &self.cam;
        let mut x = x;
        let mut y = y;
        if (x as u16) < c.originxmin as u16 || (x as u16) > c.originxmax as u16 {
            x = if (x as u16) < c.originxmin as u16 { c.originxmin } else { c.originxmax };
        }
        if (y as u16) < c.originymin as u16 || (y as u16) > c.originymax as u16 {
            y = if (y as u16) < c.originymin as u16 { c.originymin } else { c.originymax };
        }
        let xt = (x as u16 >> 8) as i32;
        let yt = (y as u16 >> 8) as i32;
        if c.wide() {
            // the same rule for any view width: a block in the left half
            // pushes the view right of it, one in the right half left
            let wg = c.view_w() * 16;
            for &b in &c.vblocks {
                let bx = b * 256;
                if x <= bx && bx <= x + wg / 2 {
                    x = bx + 256;
                    break;
                }
                if x + wg / 2 < bx && bx < x + wg {
                    x = bx - wg;
                    break;
                }
            }
            if let Some(cx) = c.narrow_room_x(self.objs[self.player].x, self.mapwidth) {
                x = cx;
            }
        } else {
            for &b in &c.vblocks {
                if xt <= b && b <= xt + 10 {
                    x = (b + 1) * 256;
                    break;
                }
                if xt + 11 <= b && b <= xt + 20 {
                    x = (b - 20) * 256;
                    break;
                }
            }
        }
        for &b in &c.hblocks {
            if yt <= b && b <= yt + 6 {
                y = (b + 1) * 256;
                break;
            }
            if yt + 7 <= b && b <= yt + 13 {
                y = (b - 13) * 256;
                break;
            }
        }
        self.calc_origin_stuff(x, y);
    }

    /// RF_Scroll (with RFL_BoundScroll)
    pub fn rf_scroll(&mut self, dx: i32, dy: i32) {
        let oldxt = self.cam.originxtile;
        let oldyt = self.cam.originytile;
        let mut x = self.cam.originxglobal + dx;
        let mut y = self.cam.originyglobal + dy;
        if dx > 0 && self.cam.wide() {
            let wg = self.cam.view_w() * 16;
            for &b in &self.cam.vblocks {
                let bx = b * 256;
                if x + wg > bx && x - dx + wg <= bx {
                    x = bx - wg;
                    break;
                }
            }
        } else if dx > 0 {
            for &b in &self.cam.vblocks {
                if b == (x as u16 >> 8) as i32 + 20 {
                    x &= 0xff00;
                    break;
                }
            }
        } else if dx < 0 {
            for &b in &self.cam.vblocks {
                if b == (x as u16 >> 8) as i32 {
                    x = (x & 0xff00) + 0x100;
                    break;
                }
            }
        }
        if dy > 0 {
            for &b in &self.cam.hblocks {
                if b == (y as u16 >> 8) as i32 + 13 {
                    y &= 0xff00;
                    break;
                }
            }
        } else if dy < 0 {
            for &b in &self.cam.hblocks {
                if b == (y as u16 >> 8) as i32 {
                    y = (y & 0xff00) + 0x100;
                    break;
                }
            }
        }
        if self.cam.wide() {
            if let Some(cx) = self.cam.narrow_room_x(self.objs[self.player].x, self.mapwidth) {
                x = cx;
            }
        }
        self.calc_origin_stuff(x, y);
        let mx = (self.cam.originxtile - oldxt).abs();
        let my = (self.cam.originytile - oldyt).abs();
        if mx >= 2 || my >= 2 {
            let (gx, gy) = (self.cam.originxglobal, self.cam.originyglobal);
            self.rf_new_position(gx, gy);
        }
    }

    /// Updates the activation rectangle used by PlayLoop after scrolling.
    pub fn update_active_bounds(&mut self) {
        let c = &mut self.cam;
        let t = c.view_tiles();
        c.originxtilemax = c.originxtile + t;
        c.originytilemax = c.originytile + 13;
        // Keen 5 keeps objects active 6 tiles beyond the view (0x183f1)
        let m = ep_sel!(4, 6, 4);
        c.inactivateleft = (c.originxtile - m).max(0);
        c.inactivateright = (c.originxtile + t + m).max(0);
        c.inactivatetop = (c.originytile - m).max(0);
        c.inactivatebottom = (c.originytile + 13 + m).max(0);
    }

    pub fn rf_set_scroll_block(&mut self, x: i32, y: i32, horizontal: bool) {
        if horizontal {
            self.cam.hblocks.push(y);
        } else {
            self.cam.vblocks.push(x);
        }
    }

    /// RF_NewMap: sets up the camera limits and tile animations.
    pub fn rf_new_map(&mut self) {
        let w = self.mapwidth;
        let h = self.mapheight;
        self.update_view();
        self.cam.originxmin = 0x200;
        self.cam.originymin = 0x200;
        self.cam.originxmax = ((w - 22) * 256 - self.cam.extra()).max(self.cam.originxmin);
        self.cam.originymax = ((h - 15) * 256).max(self.cam.originymin);
        self.cam.vblocks.clear();
        self.cam.hblocks.clear();
        self.rf_set_scroll_block(0, 1, true);
        self.rf_set_scroll_block(0, h - 2, true);
        self.rf_set_scroll_block(1, 0, false);
        self.rf_set_scroll_block(w - 2, 0, false);
        self.lasttimecount = self.sys.time_count();
        self.tics = 1;
        for s in self.rf.sprites.iter_mut().chain(self.rf.stars.iter_mut()) {
            s.visible = false;
        }
    }

    /// Widescreen: picks the play field width for the window's shape. The
    /// original 320 is kept for demos (they only replay in sync with the
    /// original view), outside the game loop and with --classic-view.
    pub fn update_view(&mut self) {
        let want = if self.opts.classic_view || self.demomode != 0 || !self.wide_ok {
            320
        } else {
            self.sys.wide_width() as i32
        };
        if want == self.cam.view_w() && self.sys.surface.logical_w() as i32 == want {
            return;
        }
        self.sys.surface.set_width(want as usize);
        self.cam.view = want;
        if self.mapwidth > 0 {
            self.cam.originxmax = ((self.mapwidth - 22) * 256 - self.cam.extra()).max(self.cam.originxmin);
            let (x, y) = (self.cam.originxglobal, self.cam.originyglobal);
            self.rf_new_position(x, y);
            self.update_active_bounds();
        }
    }

    /// First map column of the view (the camera can be left of the map
    /// only in a wide view).
    fn view_x0(&self) -> i32 {
        if self.cam.wide() {
            self.cam.originxglobal >> 8
        } else {
            self.cam.originxtile
        }
    }

    /// Widescreen: blacks out what lies beyond the scroll blocks around the
    /// player when a room is narrower than the view.
    fn mask_scroll_blocks(&mut self) {
        let p = &self.objs[self.player];
        let (px, py) = (p.x, p.y);
        let (ox, oy) = (self.cam.originxglobal, self.cam.originyglobal);
        let (vw, vh) = (self.cam.view_w(), 200);
        for &b in &self.cam.vblocks {
            let sx = (b * 256 - ox) >> 4;
            if sx + 16 <= 0 || sx >= vw {
                continue;
            }
            if px >= (b + 1) * 256 {
                self.sys.surface.bar(0, 0, sx + 16, vh, 0);
            } else {
                self.sys.surface.bar(sx, 0, vw - sx, vh, 0);
            }
        }
        for &b in &self.cam.hblocks {
            let sy = (b * 256 - oy) >> 4;
            if sy + 16 <= 0 || sy >= vh {
                continue;
            }
            if py >= (b + 1) * 256 {
                self.sys.surface.bar(0, 0, vw, sy + 16, 0);
            } else {
                self.sys.surface.bar(0, sy, vw, vh - sy, 0);
            }
        }
    }

    /// RF_MarkTileGraphics: builds the animation groups used by the map.
    pub fn rf_mark_tile_graphics_public(&mut self) {
        self.rf.anims.clear();
        self.rf.bg_group = vec![-1; NUMTILE16];
        self.rf.fg_group = vec![-1; NUMTILE16M];
        for plane in 0..2 {
            for i in 0..self.mapplanes[plane].len() {
                let t = self.mapplanes[plane][i] as usize;
                self.rf_anim_group(plane, t);
            }
        }
    }

    /// Returns the animation group of a tile, creating it if needed
    /// (-1 if the tile does not animate).
    fn rf_anim_group(&mut self, plane: usize, t: usize) -> i32 {
        let ti = &self.data.tileinfo;
        let (anim, speed, groups, limit) = if plane == 0 {
            (&ti.anim, &ti.speed, &mut self.rf.bg_group, NUMTILE16)
        } else {
            (&ti.manim, &ti.mspeed, &mut self.rf.fg_group, NUMTILE16M)
        };
        if t >= limit || anim[t] == 0 || speed[t] == 0 {
            return -1;
        }
        if groups[t] >= 0 {
            return groups[t];
        }
        let gi = self.rf.anims.len() as i32;
        let mut cur = t;
        let mut n = 0;
        loop {
            groups[cur] = gi;
            let next = (cur as i32 + anim[cur] as i32) as usize;
            n += 1;
            if anim[cur] == 0 || next == t || next >= limit || n > 20 {
                break;
            }
            cur = next;
        }
        let fgbit = if plane == 1 { 0x8000 } else { 0 };
        let mut sound_tile = 0;
        let mut sound = -1;
        let mut cur = t;
        for _ in 0..21 {
            for &(st, sn) in ANIMSOUNDS {
                if cur as u32 | fgbit == st {
                    sound_tile = st;
                    sound = sn;
                }
            }
            let next = (cur as i32 + anim[cur] as i32) as usize;
            if anim[cur] == 0 || next == t || next >= limit {
                break;
            }
            cur = next;
        }
        self.rf.anims.push(AnimTile { current: t as u32 | fgbit, countdown: speed[t] as i32, sound_tile, sound, visible: 0 });
        gi
    }

    /// RFL_AnimateTiles
    fn rf_animate_tiles(&mut self) {
        let ti = &self.data.tileinfo;
        let mut sounds = Vec::new();
        for a in self.rf.anims.iter_mut() {
            a.countdown -= self.tics;
            while a.countdown < 1 {
                if a.current & 0x8000 == 0 {
                    let t = a.current as usize;
                    let n = (t as i32 + ti.anim[t] as i32) as usize;
                    a.countdown += ti.speed.get(n).copied().unwrap_or(1).max(1) as i32;
                    a.current = n as u32;
                } else {
                    let t = (a.current & 0x7fff) as usize;
                    let n = (t as i32 + ti.manim[t] as i32) as usize;
                    a.countdown += ti.mspeed.get(n).copied().unwrap_or(1).max(1) as i32;
                    a.current = n as u32 | 0x8000;
                }
                if a.visible != 0 && a.current == a.sound_tile && a.sound != -1 {
                    sounds.push(a.sound);
                }
            }
        }
        for snd in sounds {
            self.play_sound(snd);
        }
        self.rf_sync_anim_spots();
    }

    /// Writes the current animation frames into the visible map spots.
    fn rf_sync_anim_spots(&mut self) {
        for a in self.rf.anims.iter_mut() {
            a.visible = 0;
        }
        let x0 = self.cam.originxtile;
        let y0 = self.cam.originytile;
        let tiles = self.cam.view_tiles() + PORTTILESWIDE - 20;
        for ty in y0..y0 + PORTTILESHIGH {
            for tx in x0..x0 + tiles {
                if tx < 0 || ty < 0 || tx >= self.mapwidth || ty >= self.mapheight {
                    continue;
                }
                let i = (ty * self.mapwidth + tx) as usize;
                let b = self.mapplanes[0][i] as usize;
                let g = self.rf_anim_group(0, b);
                if g >= 0 {
                    self.rf.anims[g as usize].visible += 1;
                    self.mapplanes[0][i] = (self.rf.anims[g as usize].current & 0x7fff) as u16;
                }
                let f = self.mapplanes[1][i] as usize;
                let g = self.rf_anim_group(1, f);
                if g >= 0 {
                    self.rf.anims[g as usize].visible += 1;
                    self.mapplanes[1][i] = (self.rf.anims[g as usize].current & 0x7fff) as u16;
                }
            }
        }
    }

    // ---- sprites ----------------------------------------------------------

    /// RF_PlaceSprite for an object (R_Draw).
    pub fn place_sprite(&mut self, ob: usize) {
        let o = &self.objs[ob];
        let (x, y, shape, pr) = (o.x, o.y, o.shapenum, o.priority);
        self.place_sprite_at(ob, x, y, shape, pr);
    }

    pub fn place_sprite_at(&mut self, ob: usize, x: i32, y: i32, shape: i32, priority: i32) {
        if ob >= self.rf.sprites.len() {
            return;
        }
        if shape == 0 {
            self.rf.sprites[ob].visible = false;
            self.objs[ob].sprite = false;
            return;
        }
        self.rf.order += 1;
        let order = self.rf.order;
        self.rf.sprites[ob] = SpriteSlot { visible: true, x, y, shape, priority, order, white: false };
        self.objs[ob].sprite = true;
    }

    pub fn place_stars(&mut self, ob: usize, x: i32, y: i32, shape: i32, priority: i32) {
        self.rf.order += 1;
        let order = self.rf.order;
        self.rf.stars[ob] = SpriteSlot { visible: true, x, y, shape, priority, order, white: false };
    }

    /// Switches the placed sprite of `ob` to white (maskdraw) drawing.
    pub fn set_sprite_white(&mut self, ob: usize, white: bool) {
        if ob < self.rf.sprites.len() {
            self.rf.sprites[ob].white = white;
        }
    }

    pub fn remove_sprite(&mut self, ob: usize) {
        if ob < self.rf.sprites.len() {
            self.rf.sprites[ob].visible = false;
            self.rf.stars[ob].visible = false;
        }
        if ob < self.objs.len() {
            self.objs[ob].sprite = false;
        }
    }

    pub fn remove_stars(&mut self, ob: usize) {
        if ob < self.rf.stars.len() {
            self.rf.stars[ob].visible = false;
        }
    }

    // ---- drawing ----------------------------------------------------------

    fn draw_sprite_slot(&mut self, s: SpriteSlot) {
        let idx = s.shape - STARTSPRITES as i32;
        if idx < 0 || idx as usize >= self.gfx.sprites.len() {
            return;
        }
        let info = self.data.gfx.spritetable[idx as usize];
        let scale = self.sys.surface.scale as i32;
        // EGA sprites are pre-shifted in `shifts` steps of 8/shifts pixels
        let step = match info.shifts {
            4 => 2,
            2 => 4,
            1 => 8,
            _ => 1,
        };
        let gpx = (((s.x + info.orgx as i32) as u16 as i32) >> 4) & !(step - 1);
        let gpy = ((s.y + info.orgy as i32) as u16 as i32) >> 4;
        let px = (gpx - (self.cam.originxglobal >> 4)) as i16 as i32;
        let py = (gpy - (self.cam.originyglobal >> 4)) as i16 as i32;
        let (px, py) = (px * scale, py * scale);
        let tex = &self.gfx.sprites[idx as usize];
        if s.white {
            self.sys.surface.blit_phys_solid(tex, px, py, 0xffffffff);
        } else {
            self.sys.surface.blit_phys(tex, px, py);
        }
    }

    /// Draws the play field (tiles + sprites) for the current camera.
    pub fn draw_playfield(&mut self) {
        let scale = self.sys.surface.scale as i32;
        let x0 = self.view_x0();
        let y0 = self.cam.originytile;
        let sx = (self.cam.originxglobal >> 4) & 15;
        let sy = (self.cam.originyglobal >> 4) & 15;
        // the play field fills the whole (possibly wide) surface
        let xoff = std::mem::take(&mut self.sys.surface.xoff);
        self.sys.surface.reset_clip();
        let tiles = self.cam.view_tiles() + PORTTILESWIDE - 20;
        for ty in 0..PORTTILESHIGH + 1 {
            for tx in 0..tiles + 1 {
                let mx = x0 + tx;
                let my = y0 + ty;
                let px = tx * 16 - sx;
                let py = ty * 16 - sy;
                if mx < 0 || my < 0 || mx >= self.mapwidth || my >= self.mapheight {
                    self.sys.surface.bar(px, py, 16, 16, 0);
                    continue;
                }
                let b = self.map_tile(0, mx, my);
                if let Some(Some(t)) = self.gfx.tile16.get(b) {
                    self.sys.surface.blit_phys(t, px * scale, py * scale);
                }
                let f = self.map_tile(1, mx, my);
                if f != 0 {
                    if let Some(Some(t)) = self.gfx.tile16m.get(f) {
                        self.sys.surface.blit_phys(t, px * scale, py * scale);
                    }
                }
            }
        }
        if let Some(h) = self.rf.bg_hook {
            h(self);
        }
        // sprites by priority, masked foreground tiles before priority 3
        let mut list: Vec<SpriteSlot> =
            self.rf.sprites.iter().chain(self.rf.stars.iter()).filter(|s| s.visible).copied().collect();
        list.sort_by_key(|s| (s.priority, s.order));
        let mut i = 0;
        for pr in 0..PRIORITIES {
            if pr == MASKEDTILEPRIORITY {
                self.draw_foreground_tiles();
            }
            while i < list.len() && list[i].priority <= pr {
                self.draw_sprite_slot(list[i]);
                i += 1;
            }
        }
        while i < list.len() {
            self.draw_sprite_slot(list[i]);
            i += 1;
        }
        if self.cam.wide() {
            self.mask_scroll_blocks();
        }
        self.sys.surface.xoff = xoff;
    }

    fn draw_foreground_tiles(&mut self) {
        let scale = self.sys.surface.scale as i32;
        let x0 = self.view_x0();
        let y0 = self.cam.originytile;
        let sx = (self.cam.originxglobal >> 4) & 15;
        let sy = (self.cam.originyglobal >> 4) & 15;
        let tiles = self.cam.view_tiles() + PORTTILESWIDE - 20;
        for ty in 0..PORTTILESHIGH + 1 {
            for tx in 0..tiles + 1 {
                let f = self.map_tile(1, x0 + tx, y0 + ty);
                if f == 0 || self.intile(f) & 0x80 == 0 {
                    continue;
                }
                if let Some(Some(t)) = self.gfx.tile16m.get(f) {
                    self.sys.surface.blit_phys(t, (tx * 16 - sx) * scale, (ty * 16 - sy) * scale);
                }
            }
        }
    }

    /// RFL_CalcTics
    pub fn calc_tics(&mut self) {
        let now = self.sys.time_count();
        if self.lasttimecount > now {
            self.sys.set_time_count(self.lasttimecount);
        }
        if self.demomode != 0 {
            let target = self.lasttimecount + 6;
            self.sys.wait_until(target);
            self.lasttimecount += 3;
            self.sys.set_time_count(self.lasttimecount + 3);
            self.tics = 3;
        } else {
            loop {
                let newtime = self.sys.time_count();
                self.tics = newtime - self.lasttimecount;
                if self.tics >= 2 || self.sys.quit {
                    break;
                }
                std::thread::sleep(std::time::Duration::from_millis(2));
                self.sys.poll();
            }
            let newtime = self.lasttimecount + self.tics;
            self.lasttimecount = newtime;
            if self.tics > 6 {
                let t = self.sys.time_count();
                self.sys.set_time_count(t - (self.tics - 6));
                self.tics = 6;
            }
        }
    }

    /// RF_Refresh: animate, draw, present and calculate tics.
    pub fn rf_refresh(&mut self) {
        self.rf_animate_tiles();
        self.draw_playfield();
        self.draw_score_box();
        if let Some(hook) = self.rf.hook {
            hook(self);
        }
        self.sys.present();
        self.calc_tics();
    }

    /// Redraw without advancing time (RF_ForceRefresh + update).
    pub fn rf_force_refresh(&mut self) {
        self.rf_sync_anim_spots();
        self.draw_playfield();
        self.draw_score_box();
    }
}
