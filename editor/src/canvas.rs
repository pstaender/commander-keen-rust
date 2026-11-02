//! The map view: drawing of the three layers and the tool interaction.

use crate::edit::{self, Brush, TileRect};
use crate::info;
use crate::render::{Atlas, TILE};
use crate::{intile_name, App, Tool};
use eframe::egui::{self, Align2, Color32, FontId, Mesh, PointerButton, Pos2, Rect, Sense, Shape, Stroke, StrokeKind, Vec2};

const WALL_N: Color32 = Color32::from_rgb(255, 255, 85);
const WALL_S: Color32 = Color32::from_rgb(255, 85, 85);
const WALL_E: Color32 = Color32::from_rgb(85, 255, 85);
const WALL_W: Color32 = Color32::from_rgb(85, 255, 255);
const SLOPE: Color32 = Color32::from_rgb(255, 85, 255);

/// Screen mapping of the view.
#[derive(Clone, Copy)]
struct View {
    origin: Pos2,
    offset: Vec2,
    zoom: f32,
}

impl View {
    fn tile_rect(&self, x: usize, y: usize) -> Rect {
        let s = TILE as f32 * self.zoom;
        let min = self.origin + (Vec2::new(x as f32, y as f32) * TILE as f32 - self.offset) * self.zoom;
        Rect::from_min_size(min, Vec2::splat(s))
    }
    fn px(&self, x: f32, y: f32) -> Pos2 {
        self.origin + (Vec2::new(x, y) - self.offset) * self.zoom
    }
    /// Map pixel under a screen position.
    fn map_px(&self, p: Pos2) -> Vec2 {
        (p - self.origin) / self.zoom + self.offset
    }
}

fn add_tile(mesh: &mut Mesh, atlas: &Atlas, t: usize, r: Rect, tint: Color32) {
    if atlas.has(t) {
        mesh.add_rect_with_uv(r, atlas.uv(t), tint);
    }
}

impl App {
    pub fn canvas(&mut self, ui: &mut egui::Ui) {
        let (resp, painter) = ui.allocate_painter(ui.available_size(), Sense::click_and_drag());
        let rect = resp.rect;
        let painter = painter.with_clip_rect(rect);
        painter.rect_filled(rect, 0.0, Color32::from_gray(32));
        let Some((mw, mh)) = self.map().map(|m| (m.width, m.height)) else {
            painter.text(
                rect.center(),
                Align2::CENTER_CENTER,
                "This map slot is empty. Map → New map… creates a map here.",
                FontId::proportional(18.0),
                Color32::GRAY,
            );
            self.hover = None;
            return;
        };
        let full = Vec2::new((mw * TILE) as f32, (mh * TILE) as f32);
        if self.recenter {
            self.recenter = false;
            // fit the map (or show its top left) at a whole zoom step
            let fit = (rect.width() / full.x).min(rect.height() / full.y);
            self.zoom = [0.25, 0.5, 1.0, 2.0, 3.0, 4.0].into_iter().filter(|&z| z <= fit.max(1.0)).last().unwrap_or(1.0);
            self.offset = Vec2::ZERO;
        }

        // scrolling and zooming
        let (scroll, zoom_delta, space) = ui.input(|i| (i.smooth_scroll_delta(), i.zoom_delta(), i.key_down(egui::Key::Space)));
        if resp.hovered() {
            if zoom_delta != 1.0 {
                if let Some(p) = resp.hover_pos() {
                    let before = (p - rect.min) / self.zoom + self.offset;
                    self.zoom = (self.zoom * zoom_delta).clamp(0.125, 8.0);
                    self.offset = before - (p - rect.min) / self.zoom;
                }
            } else if scroll != Vec2::ZERO {
                self.offset -= scroll / self.zoom;
            }
        }
        if resp.dragged_by(PointerButton::Middle) || (space && resp.dragged_by(PointerButton::Primary)) {
            self.offset -= resp.drag_delta() / self.zoom;
        }
        // keep some of the map in view
        let vis = rect.size() / self.zoom;
        self.offset.x = self.offset.x.clamp(-vis.x + 32.0, full.x - 32.0);
        self.offset.y = self.offset.y.clamp(-vis.y + 32.0, full.y - 32.0);
        let view = View { origin: rect.min, offset: self.offset, zoom: self.zoom };

        // tile under the mouse
        self.hover = resp.hover_pos().and_then(|p| {
            let m = view.map_px(p);
            let (x, y) = ((m.x / TILE as f32).floor(), (m.y / TILE as f32).floor());
            (x >= 0.0 && y >= 0.0 && (x as usize) < mw && (y as usize) < mh).then_some((x as usize, y as usize))
        });

        self.draw_map(&painter, view, rect);
        if !space {
            self.interact(ui, &resp);
        }
        self.draw_overlay(&painter, view);
    }

    fn draw_map(&mut self, painter: &egui::Painter, view: View, rect: Rect) {
        let Some(tex) = self.tex.as_mut() else { return };
        let m = self.project.maps[self.cur].as_ref().unwrap();
        let ts = TILE as f32 * view.zoom;
        let x0 = (view.offset.x / TILE as f32).floor().max(0.0) as usize;
        let y0 = (view.offset.y / TILE as f32).floor().max(0.0) as usize;
        let x1 = (((view.offset.x + rect.width() / view.zoom) / TILE as f32).ceil().max(0.0) as usize).min(m.width);
        let y1 = (((view.offset.y + rect.height() / view.zoom) / TILE as f32).ceil().max(0.0) as usize).min(m.height);
        let map_rect = Rect::from_min_max(view.px(0.0, 0.0), view.px((m.width * TILE) as f32, (m.height * TILE) as f32));
        painter.rect_filled(map_rect, 0.0, Color32::BLACK);

        if self.show[0] {
            let mut mesh = Mesh::with_texture(tex.bg.tex.id());
            for y in y0..y1 {
                for x in x0..x1 {
                    add_tile(&mut mesh, &tex.bg, m.planes[0][y * m.width + x] as usize, view.tile_rect(x, y), Color32::WHITE);
                }
            }
            painter.add(Shape::mesh(mesh));
        }
        if self.show[1] {
            let mut mesh = Mesh::with_texture(tex.fg.tex.id());
            for y in y0..y1 {
                for x in x0..x1 {
                    let t = m.planes[1][y * m.width + x] as usize;
                    if t != 0 {
                        add_tile(&mut mesh, &tex.fg, t, view.tile_rect(x, y), Color32::WHITE);
                    }
                }
            }
            painter.add(Shape::mesh(mesh));
        }
        if self.show_walls {
            let ti = &self.project.data.tileinfo;
            let w = (view.zoom * 1.5).clamp(1.0, 4.0);
            for y in y0..y1 {
                for x in x0..x1 {
                    let t = m.planes[1][y * m.width + x] as usize;
                    if t == 0 || t >= ti.north.len() {
                        continue;
                    }
                    let r = view.tile_rect(x, y).shrink(w / 2.0);
                    let (n, e, s, wv) = (ti.north[t], ti.east[t], ti.south[t], ti.west[t]);
                    if n != 0 {
                        painter.line_segment([r.left_top(), r.right_top()], Stroke::new(w, if n > 1 { SLOPE } else { WALL_N }));
                    }
                    if s != 0 {
                        painter.line_segment([r.left_bottom(), r.right_bottom()], Stroke::new(w, if s > 1 { SLOPE } else { WALL_S }));
                    }
                    if e != 0 {
                        painter.line_segment([r.right_top(), r.right_bottom()], Stroke::new(w, WALL_E));
                    }
                    if wv != 0 {
                        painter.line_segment([r.left_top(), r.left_bottom()], Stroke::new(w, WALL_W));
                    }
                    if view.zoom >= 1.0 {
                        let code = match intile_name(ti.intile[t]) {
                            Some("pole") => "P",
                            Some("door") => "D",
                            Some("deadly") => "X",
                            Some(_) => "•",
                            None => "",
                        };
                        if !code.is_empty() {
                            painter.text(r.center(), Align2::CENTER_CENTER, code, FontId::monospace(8.0 * view.zoom.min(2.0)), Color32::WHITE);
                        }
                    }
                }
            }
        }
        if self.show[2] {
            // sprites can reach a few tiles beyond their spot
            let (sx0, sy0) = (x0.saturating_sub(4), y0.saturating_sub(4));
            let (sx1, sy1) = ((x1 + 4).min(m.width), (y1 + 4).min(m.height));
            let world = self.cur == 0;
            for y in sy0..sy1 {
                for x in sx0..sx1 {
                    let v = m.planes[2][y * m.width + x];
                    if v == 0 {
                        continue;
                    }
                    let r = view.tile_rect(x, y);
                    let mut drawn = false;
                    if self.show_sprites {
                        if let Some(p) = self.previews.get(&v).copied() {
                            if let Some((t, size)) = tex.sprite(painter.ctx(), &self.project.data.gfx, p.sprite) {
                                let min = r.min + Vec2::new(p.dx as f32, p.dy as f32) * view.zoom;
                                let sr = Rect::from_min_size(min, Vec2::new(size[0] as f32, size[1] as f32) * view.zoom);
                                painter.image(t.id(), sr, Rect::from_min_max(Pos2::ZERO, Pos2::new(1.0, 1.0)), Color32::WHITE);
                                drawn = true;
                            }
                        }
                    }
                    if x < x0 || y < y0 || x >= x1 || y >= y1 {
                        continue;
                    }
                    info_badge(painter, r, v, world, drawn, ts);
                }
            }
            if self.show_targets {
                for y in y0..y1 {
                    for x in x0..x1 {
                        let v = m.planes[2][y * m.width + x];
                        if let Some((_, info::Category::Target, _)) = info::describe(v, self.cur == 0) {
                            let (tx, ty) = ((v >> 8) as usize, (v & 0xff) as usize);
                            target_arrow(painter, view.tile_rect(x, y).center(), view.tile_rect(tx, ty), Color32::from_rgba_unmultiplied(60, 220, 220, 160));
                        }
                    }
                }
            }
        }
        if self.show_grid && view.zoom >= 0.5 {
            let stroke = Stroke::new(1.0, Color32::from_white_alpha(28));
            for x in x0..=x1 {
                let px = view.px((x * TILE) as f32, 0.0).x;
                painter.line_segment([Pos2::new(px, map_rect.top()), Pos2::new(px, map_rect.bottom())], stroke);
            }
            for y in y0..=y1 {
                let py = view.px(0.0, (y * TILE) as f32).y;
                painter.line_segment([Pos2::new(map_rect.left(), py), Pos2::new(map_rect.right(), py)], stroke);
            }
        }
        painter.rect_stroke(map_rect, 0.0, Stroke::new(1.0, Color32::from_gray(120)), StrokeKind::Outside);
    }

    fn draw_overlay(&mut self, painter: &egui::Painter, view: View) {
        let Some(m) = self.map() else { return };
        let world = self.cur == 0;
        if let Some(r) = self.selection {
            let sr = view.tile_rect(r.x0, r.y0).union(view.tile_rect(r.x1, r.y1));
            painter.rect_filled(sr, 0.0, Color32::from_rgba_unmultiplied(80, 140, 255, 40));
            painter.rect_stroke(sr, 0.0, Stroke::new(1.5, Color32::from_rgb(120, 170, 255)), StrokeKind::Inside);
        }
        if let Some((sx, sy)) = self.target_src {
            painter.rect_stroke(view.tile_rect(sx, sy), 0.0, Stroke::new(2.0, Color32::from_rgb(60, 220, 220)), StrokeKind::Inside);
            if let Some(h) = self.hover {
                target_arrow(painter, view.tile_rect(sx, sy).center(), view.tile_rect(h.0, h.1), Color32::from_rgb(60, 220, 220));
            }
        }
        let Some((hx, hy)) = self.hover else { return };
        // highlight the target of the hovered tile
        let v = m.planes[2][hy * m.width + hx];
        if let Some((_, info::Category::Target, _)) = info::describe(v, world) {
            let t = view.tile_rect((v >> 8) as usize, (v & 0xff) as usize);
            target_arrow(painter, view.tile_rect(hx, hy).center(), t, Color32::from_rgb(60, 255, 255));
        }
        match self.tool {
            Tool::Draw if self.right_start.is_none() => self.draw_brush_preview(painter, view, self.brush_pos((hx, hy))),
            Tool::Rect | Tool::Select => {
                let r = match self.drag_start {
                    Some(s) if self.tool == Tool::Rect => TileRect::span(s, (hx, hy)),
                    _ => TileRect::span((hx, hy), (hx, hy)),
                };
                let sr = view.tile_rect(r.x0, r.y0).union(view.tile_rect(r.x1, r.y1));
                painter.rect_stroke(sr, 0.0, Stroke::new(1.5, Color32::WHITE), StrokeKind::Inside);
            }
            Tool::TestFrom => {
                painter.rect_stroke(view.tile_rect(hx, hy), 0.0, Stroke::new(2.0, Color32::from_rgb(80, 220, 80)), StrokeKind::Inside);
            }
            _ => {
                painter.rect_stroke(view.tile_rect(hx, hy), 0.0, Stroke::new(1.0, Color32::WHITE), StrokeKind::Inside);
            }
        }
        if let Some(s) = self.right_start {
            let r = TileRect::span(s, (hx, hy));
            let sr = view.tile_rect(r.x0, r.y0).union(view.tile_rect(r.x1, r.y1));
            painter.rect_stroke(sr, 0.0, Stroke::new(1.5, Color32::YELLOW), StrokeKind::Inside);
        }
    }

    fn draw_brush_preview(&mut self, painter: &egui::Painter, view: View, (bx, by): (usize, usize)) {
        let Some(tex) = self.tex.as_ref() else { return };
        let tint = Color32::from_white_alpha(170);
        let b = &self.brush;
        let mut bg = Mesh::with_texture(tex.bg.tex.id());
        let mut fg = Mesh::with_texture(tex.fg.tex.id());
        for y in 0..b.h {
            for x in 0..b.w {
                let c = b.cell(x, y);
                let r = view.tile_rect(bx + x, by + y);
                if let Some(t) = c[0] {
                    add_tile(&mut bg, &tex.bg, t as usize, r, tint);
                }
                if let Some(t) = c[1] {
                    add_tile(&mut fg, &tex.fg, t as usize, r, tint);
                }
            }
        }
        painter.add(Shape::mesh(bg));
        painter.add(Shape::mesh(fg));
        for y in 0..b.h {
            for x in 0..b.w {
                if let Some(v) = b.cell(x, y)[2] {
                    let r = view.tile_rect(bx + x, by + y);
                    if v != 0 {
                        info_badge(painter, r, v, self.cur == 0, false, TILE as f32 * view.zoom);
                    }
                }
            }
        }
        let r = view.tile_rect(bx, by).union(view.tile_rect(bx + b.w - 1, by + b.h - 1));
        painter.rect_stroke(r, 0.0, Stroke::new(1.0, Color32::WHITE), StrokeKind::Inside);
    }

    /// Where the brush is painted for the mouse tile: aligned to the brush
    /// size relative to the start of the stroke, so blocks tile seamlessly.
    fn brush_pos(&self, (x, y): (usize, usize)) -> (usize, usize) {
        let (bw, bh) = (self.brush.w as isize, self.brush.h as isize);
        match self.drag_start {
            Some((sx, sy)) if self.tool == Tool::Draw => {
                let (dx, dy) = (x as isize - sx as isize, y as isize - sy as isize);
                let ax = sx as isize + dx.div_euclid(bw) * bw;
                let ay = sy as isize + dy.div_euclid(bh) * bh;
                (ax.max(0) as usize, ay.max(0) as usize)
            }
            _ => (x, y),
        }
    }

    fn interact(&mut self, ui: &egui::Ui, resp: &egui::Response) {
        let (pressed, down, released, rpressed, rdown) = ui.input(|i| {
            (
                i.pointer.button_pressed(PointerButton::Primary),
                i.pointer.button_down(PointerButton::Primary),
                i.pointer.button_released(PointerButton::Primary),
                i.pointer.button_pressed(PointerButton::Secondary),
                i.pointer.button_down(PointerButton::Secondary),
            )
        });
        let hover = self.hover;
        let world = self.cur == 0;

        // right button: pick tiles into the brush
        if rpressed && resp.hovered() {
            self.right_start = hover;
        }
        if self.right_start.is_some() && !rdown {
            if let (Some(s), Some(h), Some(m)) = (self.right_start, hover, self.map()) {
                let r = TileRect::span(s, h);
                let planes = self.pick_planes();
                let v = m.planes[self.active][h.1 * m.width + h.0];
                self.brush = Brush::from_map(m, r, planes);
                self.pal_sel = None;
                if r.w() == 1 && r.h() == 1 {
                    self.status = format!("Picked {}", if self.active == 2 { crate::info_text(v, world) } else { v.to_string() });
                } else {
                    self.status = format!("Picked a {}x{} block", r.w(), r.h());
                }
                if !matches!(self.tool, Tool::Draw | Tool::Rect | Tool::Fill) {
                    self.tool = Tool::Draw;
                }
            }
            self.right_start = None;
        }

        // left button: the tool
        if pressed && resp.hovered() {
            if let Some(h) = hover {
                self.drag_start = Some(h);
                self.last_stamp = None;
                match self.tool {
                    Tool::Draw | Tool::Rect | Tool::Fill => self.begin_change(),
                    Tool::Select => self.selection = Some(TileRect::span(h, h)),
                    _ => {}
                }
                if self.tool == Tool::Fill {
                    let (b, p) = (self.brush.clone(), self.active);
                    if b.cells.iter().any(|c| c[p].is_some()) {
                        if let Some(m) = self.project.maps[self.cur].as_mut() {
                            edit::flood_fill(m, p, &b, h.0, h.1);
                        }
                    } else {
                        self.status = "The brush has nothing for the edited layer".into();
                    }
                }
            }
        }
        let Some(start) = self.drag_start else { return };
        if down {
            if let Some(h) = hover {
                match self.tool {
                    Tool::Draw => {
                        let pos = self.brush_pos(h);
                        if self.last_stamp != Some(pos) {
                            self.last_stamp = Some(pos);
                            let b = self.brush.clone();
                            if let Some(m) = self.project.maps[self.cur].as_mut() {
                                edit::stamp(m, &b, pos.0, pos.1);
                            }
                        }
                    }
                    Tool::Select => self.selection = Some(TileRect::span(start, h)),
                    _ => {}
                }
            }
        }
        if released || !down {
            let end = hover.unwrap_or(start);
            match self.tool {
                Tool::Rect => {
                    let b = self.brush.clone();
                    if let Some(m) = self.project.maps[self.cur].as_mut() {
                        edit::fill_rect(m, &b, TileRect::span(start, end));
                    }
                }
                Tool::Select => {
                    let r = TileRect::span(start, end);
                    self.selection = Some(r);
                    self.status = format!("Selected {}x{} at {},{}", r.w(), r.h(), r.x0, r.y0);
                }
                Tool::Target => match self.target_src {
                    None => {
                        self.target_src = Some(end);
                        self.status = "Now click the destination (the tile where Keen will stand)".into();
                    }
                    Some(src) => {
                        let v = ((end.0 as u16) << 8) | end.1 as u16;
                        self.begin_change();
                        if let Some(m) = self.project.maps[self.cur].as_mut() {
                            m.planes[2][src.1 * m.width + src.0] = v;
                        }
                        self.status = format!("Target of {},{} set to {},{} (info 0x{:04x})", src.0, src.1, end.0, end.1, v);
                        self.target_src = None;
                    }
                },
                Tool::TestFrom => {
                    self.tool = Tool::Draw;
                    self.play_test(Some(end));
                }
                _ => {}
            }
            self.drag_start = None;
        }
    }
}

/// Info value marker: a colored box with a short label (only an outline
/// when the sprite preview is drawn).
fn info_badge(painter: &egui::Painter, r: Rect, v: u16, world: bool, sprite: bool, ts: f32) {
    let (cat, label) = match info::describe(v, world) {
        Some((_, c, l)) => (Some(c), l),
        None => (None, format!("{:x}", v)),
    };
    let [cr, cg, cb] = cat.map(|c| c.color()).unwrap_or([200, 200, 200]);
    let col = Color32::from_rgb(cr, cg, cb);
    if sprite {
        painter.rect_stroke(r, 0.0, Stroke::new(1.0, col.gamma_multiply(0.8)), StrokeKind::Inside);
        return;
    }
    painter.rect_filled(r.shrink(ts * 0.06), 2.0, Color32::from_rgba_unmultiplied(cr, cg, cb, 150));
    if ts >= 12.0 {
        let size = (ts * 0.42).min(16.0) * if label.chars().count() > 2 { 0.8 } else { 1.0 };
        painter.text(r.center(), Align2::CENTER_CENTER, label, FontId::proportional(size), Color32::BLACK);
    }
}

fn target_arrow(painter: &egui::Painter, from: Pos2, to: Rect, col: Color32) {
    painter.rect_stroke(to, 0.0, Stroke::new(1.5, col), StrokeKind::Inside);
    let tip = to.center();
    if (tip - from).length() > 2.0 {
        painter.arrow(from, tip - from, Stroke::new(1.5, col));
    }
}
