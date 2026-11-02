//! The palettes: background and foreground tile sets (18 tiles per row,
//! drag to take a block) and the info values with their sprites.

use crate::edit::{Brush, TileRect};
use crate::info::{self, Category};
use crate::render::{ATLAS_COLS, TILE};
use crate::{intile_name, v_text, App};
use eframe::egui::{self, Color32, Pos2, Rect, Sense, Stroke, StrokeKind, Vec2};

impl App {
    pub fn palette(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            for (p, name) in [(0, "Background"), (1, "Foreground"), (2, "Info")] {
                ui.selectable_value(&mut self.active, p, name);
            }
        });
        ui.separator();
        self.brush_info(ui);
        ui.separator();
        if self.active == 2 {
            self.info_palette(ui);
        } else {
            ui.horizontal(|ui| {
                ui.label("Zoom");
                for z in [1.0, 1.5, 2.0] {
                    ui.selectable_value(&mut self.pal_zoom, z, format!("{}x", z));
                }
            });
            self.tile_palette(ui, self.active);
        }
    }

    fn brush_info(&mut self, ui: &mut egui::Ui) {
        let b = &self.brush;
        let world = self.cur == 0;
        let text = if b.w == 1 && b.h == 1 {
            let c = b.cells[0];
            let parts: Vec<String> = [("BG", c[0]), ("FG", c[1]), ("Info", c[2])]
                .iter()
                .filter_map(|(n, v)| v.map(|v| if *n == "Info" { format!("{} {}", n, crate::info_text(v, world)) } else { format!("{} {}", n, v) }))
                .collect();
            parts.join(", ")
        } else {
            format!("{}x{} block", b.w, b.h)
        };
        ui.label(format!("Brush: {}", text));
        if !b.cells.iter().any(|c| c[self.active].is_some()) {
            ui.colored_label(Color32::YELLOW, "The brush doesn't change the edited layer.");
        }
        ui.horizontal(|ui| {
            if ui.button("Eraser").on_hover_text("Brush that clears the edited layer").clicked() {
                self.brush = Brush::single(self.active, 0);
                self.pal_sel = None;
            }
        });
    }

    fn tile_palette(&mut self, ui: &mut egui::Ui, plane: usize) {
        let Some(tex) = self.tex.as_ref() else { return };
        let atlas = if plane == 0 { &tex.bg } else { &tex.fg };
        let (count, rows, id) = (atlas.count, atlas.rows, atlas.tex.id());
        let ts = TILE as f32 * self.pal_zoom;
        let ti = &self.project.data.tileinfo;
        egui::ScrollArea::vertical().id_salt(("pal", plane)).show(ui, |ui| {
            let size = Vec2::new(ATLAS_COLS as f32 * ts, rows as f32 * ts);
            let (resp, painter) = ui.allocate_painter(size, Sense::click_and_drag());
            let r = resp.rect;
            painter.rect_filled(r, 0.0, Color32::from_gray(20));
            painter.image(id, r, Rect::from_min_max(Pos2::ZERO, Pos2::new(1.0, 1.0)), Color32::WHITE);
            let tile_at = |p: Pos2| -> Option<(usize, usize)> {
                let v = (p - r.min) / ts;
                let (x, y) = (v.x.floor() as isize, v.y.floor() as isize);
                (x >= 0 && y >= 0 && (x as usize) < ATLAS_COLS && (y as usize) < rows && (y as usize * ATLAS_COLS + x as usize) < count)
                    .then_some((x as usize, y as usize))
            };
            let trect = |t: TileRect| {
                Rect::from_min_size(r.min + Vec2::new(t.x0 as f32, t.y0 as f32) * ts, Vec2::new(t.w() as f32, t.h() as f32) * ts)
            };
            // selection by click or drag
            if resp.drag_started() || resp.clicked() {
                if let Some(t) = resp.interact_pointer_pos().and_then(tile_at) {
                    self.pal_drag = Some(t.1 * ATLAS_COLS + t.0);
                }
            }
            if let (Some(start), Some(p)) = (self.pal_drag, resp.interact_pointer_pos()) {
                if let Some(t) = tile_at(p) {
                    let s = (start % ATLAS_COLS, start / ATLAS_COLS);
                    let sel = TileRect::span(s, t);
                    self.pal_sel = Some((plane, sel));
                    self.brush = Brush::block(plane, sel.y0 * ATLAS_COLS + sel.x0, sel.w(), sel.h(), ATLAS_COLS);
                }
            }
            if resp.drag_stopped() || resp.clicked() {
                self.pal_drag = None;
                if !matches!(self.tool, crate::Tool::Draw | crate::Tool::Rect | crate::Tool::Fill) {
                    self.tool = crate::Tool::Draw;
                }
            }
            if let Some((p, sel)) = self.pal_sel {
                if p == plane {
                    painter.rect_stroke(trect(sel), 0.0, Stroke::new(2.0, Color32::WHITE), StrokeKind::Outside);
                }
            }
            if let Some(t) = resp.hover_pos().and_then(tile_at) {
                let n = t.1 * ATLAS_COLS + t.0;
                painter.rect_stroke(trect(TileRect::span(t, t)), 0.0, Stroke::new(1.0, Color32::YELLOW), StrokeKind::Inside);
                let mut tip = format!("Tile {}", n);
                if plane == 1 && n < ti.north.len() {
                    tip += &format!("\nwalls: top {} right {} bottom {} left {}", ti.north[n], ti.east[n], ti.south[n], ti.west[n]);
                    if let Some(s) = intile_name(ti.intile[n]) {
                        tip += &format!("\n{}", s);
                    }
                    if ti.intile[n] & 0x80 != 0 {
                        tip += "\ndrawn in front of sprites";
                    }
                    if ti.manim[n] != 0 {
                        tip += "\nanimated";
                    }
                } else if plane == 0 && n < ti.anim.len() && ti.anim[n] != 0 {
                    tip += "\nanimated";
                }
                resp.on_hover_text(tip);
            }
        });
    }

    fn info_palette(&mut self, ui: &mut egui::Ui) {
        ui.horizontal_wrapped(|ui| {
            for c in Category::ALL {
                ui.selectable_value(&mut self.info_cat, c, c.name());
            }
        });
        ui.horizontal(|ui| {
            ui.label("Value:");
            let r = ui.add(egui::TextEdit::singleline(&mut self.info_raw).desired_width(70.0).hint_text("e.g. 0x2a10"));
            let set = ui.button("Use").clicked() || (r.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)));
            if set {
                let s = self.info_raw.trim();
                let v = s.strip_prefix("0x").map(|h| u16::from_str_radix(h, 16).ok()).unwrap_or_else(|| s.parse().ok());
                match v {
                    Some(v) => self.brush = Brush::single(2, v),
                    None => self.status = "Enter a number (decimal, or hex with 0x)".into(),
                }
            }
        });
        ui.label("Door targets: use the door target tool (T).");
        ui.separator();
        let world = self.cur == 0;
        let values = info::palette_values(self.info_cat);
        let current = self.brush.value(2);
        egui::ScrollArea::vertical().id_salt("infopal").show(ui, |ui| {
            for v in values {
                let Some((name, cat, label)) = info::describe(v, world || cat_is_world(self.info_cat)) else { continue };
                let selected = current == Some(v);
                let r = ui
                    .horizontal(|ui| {
                        let (rect, _) = ui.allocate_exact_size(Vec2::splat(36.0), Sense::hover());
                        self.info_icon(ui, rect, v, cat, &label);
                        ui.selectable_label(selected, format!("{:>6}  {}", v_text(v), name))
                    })
                    .inner;
                if r.clicked() {
                    self.brush = Brush::single(2, v);
                    if !matches!(self.tool, crate::Tool::Draw | crate::Tool::Rect | crate::Tool::Fill) {
                        self.tool = crate::Tool::Draw;
                    }
                }
            }
        });
    }

    /// Sprite preview (scaled to fit) or a colored badge.
    fn info_icon(&mut self, ui: &egui::Ui, rect: Rect, v: u16, cat: Category, label: &str) {
        let painter = ui.painter_at(rect);
        painter.rect_filled(rect, 3.0, Color32::from_gray(24));
        if let (Some(p), Some(tex)) = (self.previews.get(&v).copied(), self.tex.as_mut()) {
            if let Some((t, size)) = tex.sprite(ui.ctx(), &self.project.data.gfx, p.sprite) {
                let s = Vec2::new(size[0] as f32, size[1] as f32);
                let k = (rect.width() / s.x).min(rect.height() / s.y).min(1.0);
                let r = Rect::from_center_size(rect.center(), s * k);
                painter.image(t.id(), r, Rect::from_min_max(Pos2::ZERO, Pos2::new(1.0, 1.0)), Color32::WHITE);
                return;
            }
        }
        let [r, g, b] = cat.color();
        painter.rect_filled(rect.shrink(6.0), 3.0, Color32::from_rgb(r, g, b));
        painter.text(rect.center(), egui::Align2::CENTER_CENTER, label, egui::FontId::proportional(13.0), Color32::BLACK);
    }
}

fn cat_is_world(c: Category) -> bool {
    c == Category::WorldMap
}
