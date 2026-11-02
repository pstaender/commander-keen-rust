//! Commander Keen Editor: a level and world map editor for Keen 4, 5 and 6
//! (GAMEMAPS.CKn), with play testing in the Rust reimplementation.

mod canvas;
mod edit;
mod info;
mod palette;
mod project;
mod render;

use eframe::egui::{self, Key, KeyboardShortcut, Modifiers, ViewportCommand};
use edit::{Brush, History, TileRect};
use info::Preview;
use keen::assets::maps::{Map, MAPNAME_LEN, NUMMAPS};
use project::Project;
use render::Textures;
use std::collections::HashMap;
use std::path::PathBuf;

const USAGE: &str = "keen-editor [options] [MAP]
  --data DIR   directory with the original Keen 4, 5 or 6 files (default:
               KEEN6, KEEN5 or KEEN4, whichever exists)
  --maps DIR   edit the map set (GAMEMAPS.CKn, MAPHEAD.CKn) in DIR
  MAP          map number to open (0 = world map)";

/// Largest map: the planes are 16-bit segments in the original (and the
/// RLEW length word is 16 bit), door targets address 256x256 tiles.
pub const MAX_MAP_SIDE: usize = 255;
pub const MAX_MAP_AREA: usize = 0x7fff;
/// Smallest map the game's refresh manager handles (21x14 tile buffer).
pub const MIN_MAP_W: usize = 21;
pub const MIN_MAP_H: usize = 14;

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Tool {
    Draw,
    Rect,
    Fill,
    Select,
    /// click a door/teleporter tile, then its destination
    Target,
    /// click where Keen starts the play test
    TestFrom,
}

impl Tool {
    fn label(self) -> &'static str {
        match self {
            Tool::Draw => "✏ Draw",
            Tool::Rect => "▭ Rectangle",
            Tool::Fill => "🪣 Fill",
            Tool::Select => "⬚ Select",
            Tool::Target => "➡ Door target",
            Tool::TestFrom => "▶ Test from…",
        }
    }
    fn help(self) -> &'static str {
        match self {
            Tool::Draw => "Paint the brush (D). Right click picks a tile, right drag picks a block.",
            Tool::Rect => "Drag a rectangle and fill it with the brush (R).",
            Tool::Fill => "Flood fill the clicked area of the active layer (F).",
            Tool::Select => "Drag to select. Cmd/Ctrl+C copies into the brush, Delete clears, Cmd/Ctrl+X cuts (S).",
            Tool::Target => "Click a door, teleporter or switch, then click its destination (T).",
            Tool::TestFrom => "Click the tile where Keen should start.",
        }
    }
}

pub enum Dialog {
    NewMap { slot: usize, name: String, w: usize, h: usize, copy_of: Option<usize> },
    Properties { name: String, w: usize, h: usize, anchor: (usize, usize) },
    ConfirmDelete(usize),
    ConfirmClose,
    ConfirmOpen,
    Message(String),
    Help,
}

pub struct App {
    pub project: Project,
    pub tex: Option<Textures>,
    pub previews: HashMap<u16, Preview>,
    /// current map slot
    pub cur: usize,
    pub zoom: f32,
    /// top left of the view, in map pixels
    pub offset: egui::Vec2,
    pub show: [bool; 3],
    pub show_grid: bool,
    pub show_walls: bool,
    pub show_sprites: bool,
    pub show_targets: bool,
    /// layer edited by the tools and picked from (0 bg, 1 fg, 2 info)
    pub active: usize,
    /// picking from the map copies all visible layers
    pub pick_all: bool,
    pub tool: Tool,
    pub brush: Brush,
    /// last brush used on each layer, restored when switching layers
    saved_brush: [Option<Brush>; 3],
    prev_active: usize,
    /// tile block selected in a tile palette (plane, block)
    pub pal_sel: Option<(usize, TileRect)>,
    pub pal_drag: Option<usize>,
    pub pal_zoom: f32,
    pub info_cat: info::Category,
    pub info_raw: String,
    pub selection: Option<TileRect>,
    /// tile where the current left/right drag started
    pub drag_start: Option<(usize, usize)>,
    pub right_start: Option<(usize, usize)>,
    pub last_stamp: Option<(usize, usize)>,
    pub target_src: Option<(usize, usize)>,
    pub hover: Option<(usize, usize)>,
    pub history: History,
    pub dialog: Option<Dialog>,
    pub status: String,
    pub allow_close: bool,
    title: String,
    /// map to center the view on after switching maps
    pub recenter: bool,
    /// difficulty for play tests (1 easy, 2 normal, 3 hard), remembered
    pub difficulty: i32,
    /// testing: KEEN6_EDITOR_SHOT=file.png saves a screenshot and exits
    shot: Option<(PathBuf, u32)>,
}

impl App {
    fn new(project: Project, map: usize, storage: Option<&dyn eframe::Storage>) -> App {
        let previews = info::sprite_previews(&project.data_dir);
        let mut app = App {
            project,
            tex: None,
            previews,
            cur: 0,
            zoom: 2.0,
            offset: egui::Vec2::ZERO,
            show: [true; 3],
            show_grid: false,
            show_walls: false,
            show_sprites: true,
            show_targets: true,
            active: 1,
            pick_all: false,
            tool: Tool::Draw,
            brush: Brush::single(1, 0),
            saved_brush: [None, None, None],
            prev_active: 1,
            pal_sel: None,
            pal_drag: None,
            pal_zoom: 1.0,
            info_cat: info::Category::Keen,
            info_raw: String::new(),
            selection: None,
            drag_start: None,
            right_start: None,
            last_stamp: None,
            target_src: None,
            hover: None,
            history: History::default(),
            dialog: None,
            status: "Welcome to the Commander Keen Editor. Press F1 for help.".into(),
            allow_close: false,
            title: String::new(),
            recenter: true,
            difficulty: storage
                .and_then(|s| s.get_string(DIFFICULTY_KEY))
                .and_then(|v| v.parse().ok())
                .filter(|d| (1..=3).contains(d))
                .unwrap_or(2),
            shot: std::env::var_os("KEEN6_EDITOR_SHOT").map(|p| (PathBuf::from(p), 0)),
        };
        app.select_map(map.min(NUMMAPS - 1));
        if let Ok(setup) = std::env::var("KEEN6_EDITOR_SETUP") {
            app.debug_setup(&setup);
        }
        app
    }

    /// Testing: applies "key=value,…" settings (layer, zoom, x, y, walls,
    /// grid, cat, help, pick) after startup.
    fn debug_setup(&mut self, s: &str) {
        for item in s.split(',') {
            let (k, v) = item.split_once('=').unwrap_or((item, ""));
            let f: f32 = v.parse().unwrap_or(0.0);
            match k {
                "layer" => self.active = f as usize % 3,
                "zoom" => {
                    self.zoom = f;
                    self.recenter = false;
                }
                "x" => self.offset.x = f * 16.0,
                "y" => self.offset.y = f * 16.0,
                "walls" => self.show_walls = true,
                "grid" => self.show_grid = true,
                "nosprites" => self.show_sprites = false,
                "help" => self.dialog = Some(Dialog::Help),
                "cat" => self.info_cat = info::Category::ALL[f as usize % 6],
                "tool" => self.tool = [Tool::Draw, Tool::Rect, Tool::Fill, Tool::Select, Tool::Target][f as usize % 5],
                _ => eprintln!("unknown setup item {}", k),
            }
        }
    }

    fn debug_shot(&mut self, ctx: &egui::Context) {
        let Some((path, frames)) = &mut self.shot else { return };
        *frames += 1;
        if *frames == 15 {
            ctx.send_viewport_cmd(ViewportCommand::Screenshot(Default::default()));
        }
        let image = ctx.input(|i| {
            i.events.iter().find_map(|e| match e {
                egui::Event::Screenshot { image, .. } => Some(image.clone()),
                _ => None,
            })
        });
        if let Some(img) = image {
            let rgba: Vec<u8> = img.pixels.iter().flat_map(|c| c.to_array()).collect();
            if let Err(e) = keen::dump::write_png(path, img.size[0], img.size[1], &rgba) {
                eprintln!("{}: {}", path.display(), e);
            }
            self.allow_close = true;
            ctx.send_viewport_cmd(ViewportCommand::Close);
        }
        ctx.request_repaint();
    }

    pub fn map(&self) -> Option<&Map> {
        self.project.maps[self.cur].as_ref()
    }

    /// Records the current map for undo before it is changed.
    pub fn begin_change(&mut self) {
        self.history.push(self.cur, self.project.maps[self.cur].clone());
        self.project.dirty = true;
    }

    pub fn select_map(&mut self, n: usize) {
        self.cur = n;
        self.selection = None;
        self.target_src = None;
        self.recenter = true;
        if n == 0 {
            self.info_cat = info::Category::WorldMap;
        } else if self.info_cat == info::Category::WorldMap {
            self.info_cat = info::Category::Keen;
        }
    }

    fn undo(&mut self) {
        if let Some(n) = self.history.undo(&mut self.project.maps) {
            self.project.dirty = true;
            if n != self.cur {
                self.select_map(n);
            }
            self.status = "Undo".into();
        }
    }

    fn redo(&mut self) {
        if let Some(n) = self.history.redo(&mut self.project.maps) {
            self.project.dirty = true;
            if n != self.cur {
                self.select_map(n);
            }
            self.status = "Redo".into();
        }
    }

    fn save(&mut self) {
        match self.project.maps_dir.clone() {
            Some(dir) => self.save_to(dir),
            None => self.save_as(),
        }
    }

    fn save_as(&mut self) {
        let mut d = rfd::FileDialog::new().set_title("Save map set (GAMEMAPS + MAPHEAD) into folder");
        if let Some(dir) = &self.project.maps_dir {
            d = d.set_directory(dir);
        }
        if let Some(dir) = d.pick_folder() {
            self.save_to(dir);
        }
    }

    fn save_to(&mut self, dir: PathBuf) {
        match self.project.save_to(&dir) {
            Ok(()) => {
                let same = dir.canonicalize().ok() == self.project.data_dir.canonicalize().ok();
                let lay = self.project.data.layout;
                self.status = if same {
                    format!(
                        "Saved into the game directory {}. The Rust port uses these maps now; the original \
                         backups are {}.bak / {}.bak.",
                        dir.display(),
                        lay.file("GAMEMAPS"),
                        lay.file("MAPHEAD")
                    )
                } else {
                    format!("Saved to {}. Play with: keen{} --data … --maps {}", dir.display(), lay.number, dir.display())
                };
            }
            Err(e) => self.dialog = Some(Dialog::Message(format!("Saving failed: {}", e))),
        }
    }

    fn open_maps(&mut self) {
        if self.project.dirty && !matches!(self.dialog, Some(Dialog::ConfirmOpen)) {
            self.dialog = Some(Dialog::ConfirmOpen);
            return;
        }
        if let Some(dir) = rfd::FileDialog::new().set_title("Open map set folder (GAMEMAPS.CKn)").pick_folder() {
            match self.project.load_maps(&dir) {
                Ok(()) => {
                    self.history.clear();
                    let n = if self.project.maps[self.cur].is_some() { self.cur } else { 0 };
                    self.select_map(n);
                    self.status = format!("Opened {}", dir.display());
                }
                Err(e) => self.dialog = Some(Dialog::Message(e)),
            }
        }
    }

    pub fn play_test(&mut self, start: Option<(usize, usize)>) {
        match self.project.play_test(self.cur, start, self.difficulty) {
            Ok(()) => {
                self.status = format!(
                    "Started the game on map {} ({}; the game exits when the level is done)",
                    self.cur,
                    DIFFICULTIES[self.difficulty as usize - 1]
                )
            }
            Err(e) => self.dialog = Some(Dialog::Message(format!("Can't test: {}", e))),
        }
    }

    /// Exports the current map as a PNG (background + foreground).
    fn export_png(&mut self) {
        let Some(m) = self.map() else { return };
        let Some(path) = rfd::FileDialog::new()
            .add_filter("PNG image", &["png"])
            .set_file_name(format!("map{:02}.png", self.cur))
            .save_file()
        else {
            return;
        };
        let g = &self.project.data.gfx;
        let (w, h) = (m.width * 16, m.height * 16);
        let mut rgba = vec![0u8; w * h * 4];
        for ty in 0..m.height {
            for tx in 0..m.width {
                let i = ty * m.width + tx;
                let layers = [g.tile16.get(m.planes[0][i] as usize), g.tile16m.get(m.planes[1][i] as usize)];
                for (li, t) in layers.into_iter().enumerate() {
                    let Some(Some(img)) = t else { continue };
                    if li == 1 && m.planes[1][i] == 0 {
                        continue;
                    }
                    for y in 0..16 {
                        for x in 0..16 {
                            let c = render::ega(img.get(x, y));
                            if c[3] == 0 {
                                continue;
                            }
                            let o = ((ty * 16 + y) * w + tx * 16 + x) * 4;
                            rgba[o..o + 4].copy_from_slice(&c);
                        }
                    }
                }
            }
        }
        self.status = match keen::dump::write_png(&path, w, h, &rgba) {
            Ok(()) => format!("Exported {}", path.display()),
            Err(e) => format!("Export failed: {}", e),
        };
    }

    fn copy_selection(&mut self, cut: bool) {
        let (Some(r), Some(m)) = (self.selection, self.map()) else { return };
        let planes = self.pick_planes();
        self.brush = Brush::from_map(m, r, planes);
        self.pal_sel = None;
        if cut {
            self.begin_change();
            if let Some(m) = self.project.maps[self.cur].as_mut() {
                edit::clear_rect(m, r, planes);
            }
        }
        self.tool = Tool::Draw;
        self.status = format!("Copied {}x{} tiles into the brush", r.w(), r.h());
    }

    fn delete_selection(&mut self) {
        let Some(r) = self.selection else { return };
        let planes = self.pick_planes();
        self.begin_change();
        if let Some(m) = self.project.maps[self.cur].as_mut() {
            edit::clear_rect(m, r, planes);
        }
    }

    /// Keeps a brush per layer: switching the edited layer brings back the
    /// last brush used there (an eraser at first).
    fn switch_brush(&mut self) {
        if self.active == self.prev_active {
            return;
        }
        let touches = |b: &Brush, p: usize| b.cells.iter().any(|c| c[p].is_some());
        if touches(&self.brush, self.prev_active) {
            self.saved_brush[self.prev_active] = Some(self.brush.clone());
        }
        if !touches(&self.brush, self.active) {
            self.brush = self.saved_brush[self.active].clone().unwrap_or_else(|| Brush::single(self.active, 0));
            self.pal_sel = None;
        }
        self.prev_active = self.active;
    }

    /// Layers that picking/copying from the map takes.
    pub fn pick_planes(&self) -> [bool; 3] {
        if self.pick_all {
            self.show
        } else {
            let mut p = [false; 3];
            p[self.active] = true;
            p
        }
    }

    fn shortcuts(&mut self, ctx: &egui::Context) {
        let cmd = |k| KeyboardShortcut::new(Modifiers::COMMAND, k);
        let cmd_shift = |k| KeyboardShortcut::new(Modifiers::COMMAND | Modifiers::SHIFT, k);
        let pressed = |s: KeyboardShortcut| ctx.input_mut(|i| i.consume_shortcut(&s));
        if pressed(cmd_shift(Key::Z)) || pressed(cmd(Key::Y)) {
            self.redo();
        }
        if pressed(cmd(Key::Z)) {
            self.undo();
        }
        if pressed(cmd_shift(Key::S)) {
            self.save_as();
        }
        if pressed(cmd(Key::S)) {
            self.save();
        }
        if pressed(cmd(Key::O)) {
            self.open_maps();
        }
        if ctx.egui_wants_keyboard_input() {
            return;
        }
        if pressed(cmd(Key::C)) {
            self.copy_selection(false);
        }
        if pressed(cmd(Key::X)) {
            self.copy_selection(true);
        }
        let key = |k| ctx.input(|i| i.key_pressed(k) && i.modifiers.is_none());
        if key(Key::F1) {
            self.dialog = Some(Dialog::Help);
        }
        if key(Key::F5) {
            self.play_test(None);
        }
        if key(Key::F6) {
            let h = self.hover;
            if h.is_some() {
                self.play_test(h);
            }
        }
        if key(Key::Delete) || key(Key::Backspace) {
            self.delete_selection();
        }
        if key(Key::Escape) {
            self.selection = None;
            self.target_src = None;
            if self.tool == Tool::TestFrom {
                self.tool = Tool::Draw;
            }
        }
        for (k, t) in [(Key::D, Tool::Draw), (Key::R, Tool::Rect), (Key::F, Tool::Fill), (Key::S, Tool::Select), (Key::T, Tool::Target)] {
            if key(k) {
                self.tool = t;
                self.target_src = None;
            }
        }
        for (k, p) in [(Key::Num1, 0), (Key::Num2, 1), (Key::Num3, 2)] {
            if key(k) {
                self.active = p;
            }
        }
        if key(Key::G) {
            self.show_grid = !self.show_grid;
        }
        if key(Key::W) {
            self.show_walls = !self.show_walls;
        }
        if key(Key::Plus) || key(Key::Equals) {
            self.zoom = (self.zoom * 2.0).min(8.0);
        }
        if key(Key::Minus) {
            self.zoom = (self.zoom / 2.0).max(0.125);
        }
        if key(Key::PageUp) && self.cur > 0 {
            self.select_map(self.cur - 1);
        }
        if key(Key::PageDown) && self.cur + 1 < NUMMAPS {
            self.select_map(self.cur + 1);
        }
    }

    fn menu_bar(&mut self, ui: &mut egui::Ui) {
        egui::MenuBar::new().ui(ui, |ui| {
            ui.menu_button("File", |ui| {
                if ui.button("Open map set…  (Cmd/Ctrl+O)").clicked() {
                    ui.close();
                    self.open_maps();
                }
                if ui.button("Save  (Cmd/Ctrl+S)").clicked() {
                    ui.close();
                    self.save();
                }
                if ui.button("Save as…  (Cmd/Ctrl+Shift+S)").clicked() {
                    ui.close();
                    self.save_as();
                }
                ui.separator();
                if ui.button("Export map as PNG…").clicked() {
                    ui.close();
                    self.export_png();
                }
                ui.separator();
                if ui.button("Quit").clicked() {
                    ui.close();
                    ui.ctx().send_viewport_cmd(ViewportCommand::Close);
                }
            });
            ui.menu_button("Edit", |ui| {
                if ui.add_enabled(self.history.can_undo(), egui::Button::new("Undo  (Cmd/Ctrl+Z)")).clicked() {
                    self.undo();
                }
                if ui.add_enabled(self.history.can_redo(), egui::Button::new("Redo  (Cmd/Ctrl+Shift+Z)")).clicked() {
                    self.redo();
                }
                ui.separator();
                let sel = self.selection.is_some();
                if ui.add_enabled(sel, egui::Button::new("Copy to brush  (Cmd/Ctrl+C)")).clicked() {
                    ui.close();
                    self.copy_selection(false);
                }
                if ui.add_enabled(sel, egui::Button::new("Cut  (Cmd/Ctrl+X)")).clicked() {
                    ui.close();
                    self.copy_selection(true);
                }
                if ui.add_enabled(sel, egui::Button::new("Clear selection  (Delete)")).clicked() {
                    ui.close();
                    self.delete_selection();
                }
            });
            ui.menu_button("Map", |ui| {
                if ui.button("New map…").clicked() {
                    ui.close();
                    self.new_map_dialog(None);
                }
                if ui.add_enabled(self.map().is_some(), egui::Button::new("Duplicate…")).clicked() {
                    ui.close();
                    self.new_map_dialog(Some(self.cur));
                }
                if ui.add_enabled(self.map().is_some(), egui::Button::new("Properties / resize…")).clicked() {
                    ui.close();
                    self.properties_dialog();
                }
                if ui.add_enabled(self.map().is_some(), egui::Button::new("Delete map…")).clicked() {
                    ui.close();
                    self.dialog = Some(Dialog::ConfirmDelete(self.cur));
                }
            });
            ui.menu_button("Test", |ui| {
                if ui.button("Play this map  (F5)").clicked() {
                    ui.close();
                    self.play_test(None);
                }
                if ui.button("Play from a tile…  (F6 at the mouse)").clicked() {
                    ui.close();
                    self.tool = Tool::TestFrom;
                }
                ui.separator();
                ui.label("Difficulty:");
                for (d, name) in (1..).zip(DIFFICULTIES) {
                    ui.radio_value(&mut self.difficulty, d, name);
                }
            });
            ui.menu_button("Help", |ui| {
                if ui.button("Help  (F1)").clicked() {
                    ui.close();
                    self.dialog = Some(Dialog::Help);
                }
            });
        });
    }

    fn toolbar(&mut self, ui: &mut egui::Ui) {
        ui.horizontal_wrapped(|ui| {
            for t in [Tool::Draw, Tool::Rect, Tool::Fill, Tool::Select, Tool::Target, Tool::TestFrom] {
                if ui.selectable_label(self.tool == t, t.label()).on_hover_text(t.help()).clicked() {
                    self.tool = t;
                    self.target_src = None;
                }
            }
            ui.separator();
            ui.label("Edit:");
            for (p, name, key) in [(0, "Background", "1"), (1, "Foreground", "2"), (2, "Info", "3")] {
                ui.selectable_value(&mut self.active, p, name).on_hover_text(format!("Edit this layer ({})", key));
            }
            ui.separator();
            ui.label("Show:");
            ui.checkbox(&mut self.show[0], "BG");
            ui.checkbox(&mut self.show[1], "FG");
            ui.checkbox(&mut self.show[2], "Info");
            ui.checkbox(&mut self.show_sprites, "Sprites");
            ui.checkbox(&mut self.show_targets, "Targets");
            ui.checkbox(&mut self.show_walls, "Walls").on_hover_text(
                "Tile properties (W): top yellow, bottom red, right green, left cyan, slopes magenta; \
                 P pole, D door, X deadly",
            );
            ui.checkbox(&mut self.show_grid, "Grid");
            ui.separator();
            ui.checkbox(&mut self.pick_all, "Pick all layers").on_hover_text(
                "Right click picking and Copy take all visible layers instead of only the edited one",
            );
            ui.separator();
            if ui.button("−").clicked() {
                self.zoom = (self.zoom / 2.0).max(0.125);
            }
            ui.label(format!("{:.0}%", self.zoom * 100.0));
            if ui.button("+").clicked() {
                self.zoom = (self.zoom * 2.0).min(8.0);
            }
            ui.separator();
            if ui.button("▶ Play (F5)").clicked() {
                self.play_test(None);
            }
            egui::ComboBox::from_id_salt("difficulty")
                .selected_text(DIFFICULTIES[self.difficulty as usize - 1])
                .show_ui(ui, |ui| {
                    for (d, name) in (1..).zip(DIFFICULTIES) {
                        ui.selectable_value(&mut self.difficulty, d, name);
                    }
                })
                .response
                .on_hover_text("Difficulty for play tests (remembered)");
        });
    }

    fn map_list(&mut self, ui: &mut egui::Ui) {
        ui.heading("Maps");
        ui.horizontal(|ui| {
            if ui.button("New…").clicked() {
                self.new_map_dialog(None);
            }
            if ui.add_enabled(self.map().is_some(), egui::Button::new("Properties…")).clicked() {
                self.properties_dialog();
            }
        });
        ui.separator();
        egui::ScrollArea::vertical().show(ui, |ui| {
            for n in 0..NUMMAPS {
                let exists = self.project.maps[n].is_some();
                if !exists && n > 0 && self.project.maps[n - 1].is_none() && n != self.cur {
                    continue;
                }
                let text = egui::RichText::new(self.project.map_title(n)).monospace();
                let text = if exists { text } else { text.weak() };
                let r = ui.selectable_label(self.cur == n, text);
                if r.clicked() {
                    self.select_map(n);
                }
                r.context_menu(|ui| {
                    if exists {
                        if ui.button("Duplicate…").clicked() {
                            ui.close();
                            self.new_map_dialog(Some(n));
                        }
                        if ui.button("Delete…").clicked() {
                            ui.close();
                            self.dialog = Some(Dialog::ConfirmDelete(n));
                        }
                    } else if ui.button("New map here…").clicked() {
                        ui.close();
                        self.cur = n;
                        self.new_map_dialog(None);
                    }
                });
            }
        });
    }

    fn first_free_slot(&self) -> usize {
        if self.project.maps[self.cur].is_none() {
            return self.cur;
        }
        (1..NUMMAPS).find(|&n| self.project.maps[n].is_none()).unwrap_or(NUMMAPS - 1)
    }

    fn new_map_dialog(&mut self, copy_of: Option<usize>) {
        let (name, w, h) = match copy_of.and_then(|n| self.project.maps[n].as_ref()) {
            Some(m) => (format!("{} copy", m.name).chars().take(MAPNAME_LEN - 1).collect(), m.width, m.height),
            None => ("New level".to_string(), 64, 40),
        };
        self.dialog = Some(Dialog::NewMap { slot: self.first_free_slot(), name, w, h, copy_of });
    }

    fn properties_dialog(&mut self) {
        if let Some(m) = self.map() {
            self.dialog = Some(Dialog::Properties { name: m.name.clone(), w: m.width, h: m.height, anchor: (0, 0) });
        }
    }

    fn dialogs(&mut self, ctx: &egui::Context) {
        let Some(mut dialog) = self.dialog.take() else { return };
        let mut open = true;
        let mut keep = true;
        let title = match &dialog {
            Dialog::NewMap { .. } => "New map",
            Dialog::Properties { .. } => "Map properties",
            Dialog::ConfirmDelete(_) => "Delete map",
            Dialog::ConfirmClose | Dialog::ConfirmOpen => "Unsaved changes",
            Dialog::Message(_) => "Commander Keen Editor",
            Dialog::Help => "Help",
        };
        egui::Window::new(title)
            .collapsible(false)
            .resizable(false)
            .open(&mut open)
            .anchor(egui::Align2::CENTER_CENTER, egui::Vec2::ZERO)
            .show(ctx, |ui| match &mut dialog {
                Dialog::NewMap { slot, name, w, h, copy_of } => {
                    egui::Grid::new("newmap").num_columns(2).show(ui, |ui| {
                        ui.label("Slot");
                        ui.add(egui::DragValue::new(slot).range(0..=NUMMAPS - 1));
                        ui.end_row();
                        ui.label("Name");
                        ui.add(egui::TextEdit::singleline(name).char_limit(MAPNAME_LEN - 1));
                        ui.end_row();
                        if copy_of.is_none() {
                            size_fields(ui, w, h);
                        } else {
                            ui.label("Copy of");
                            ui.label(self.project.map_title(copy_of.unwrap_or(0)));
                            ui.end_row();
                        }
                    });
                    if self.project.maps[*slot].is_some() {
                        ui.colored_label(egui::Color32::YELLOW, "This slot is used; its map will be replaced.");
                    }
                    if *slot > 18 {
                        ui.label("Maps above 18 can't be entered from the world map; test them with F5.");
                    }
                    ui.horizontal(|ui| {
                        if ui.button("Create").clicked() {
                            let mut m = match copy_of.and_then(|n| self.project.maps[n].clone()) {
                                Some(m) => m,
                                None => Map::new(*w, *h, name),
                            };
                            m.name = name.clone();
                            self.history.push(*slot, self.project.maps[*slot].clone());
                            self.project.maps[*slot] = Some(m);
                            self.project.dirty = true;
                            self.select_map(*slot);
                            keep = false;
                        }
                        if ui.button("Cancel").clicked() {
                            keep = false;
                        }
                    });
                }
                Dialog::Properties { name, w, h, anchor } => {
                    egui::Grid::new("props").num_columns(2).show(ui, |ui| {
                        ui.label("Name");
                        ui.add(egui::TextEdit::singleline(name).char_limit(MAPNAME_LEN - 1));
                        ui.end_row();
                        size_fields(ui, w, h);
                        ui.label("Keep contents at");
                        ui.horizontal(|ui| {
                            egui::Grid::new("anchor").spacing([2.0, 2.0]).show(ui, |ui| {
                                for ay in 0..3 {
                                    for ax in 0..3 {
                                        let sym = if *anchor == (ax, ay) { "●" } else { "○" };
                                        if ui.small_button(sym).clicked() {
                                            *anchor = (ax, ay);
                                        }
                                    }
                                    ui.end_row();
                                }
                            });
                        });
                        ui.end_row();
                    });
                    ui.horizontal(|ui| {
                        if ui.button("Apply").clicked() {
                            if let Some(m) = self.map().cloned() {
                                self.begin_change();
                                let dx = (*w as isize - m.width as isize) * anchor.0 as isize / 2;
                                let dy = (*h as isize - m.height as isize) * anchor.1 as isize / 2;
                                let mut n = if (*w, *h) != (m.width, m.height) { edit::resize(&m, *w, *h, dx, dy) } else { m };
                                n.name = name.clone();
                                self.project.maps[self.cur] = Some(n);
                                self.selection = None;
                            }
                            keep = false;
                        }
                        if ui.button("Cancel").clicked() {
                            keep = false;
                        }
                    });
                }
                Dialog::ConfirmDelete(n) => {
                    ui.label(format!("Delete map {}? (Undo brings it back.)", self.project.map_title(*n).trim()));
                    ui.horizontal(|ui| {
                        if ui.button("Delete").clicked() {
                            self.history.push(*n, self.project.maps[*n].take());
                            self.project.dirty = true;
                            keep = false;
                        }
                        if ui.button("Cancel").clicked() {
                            keep = false;
                        }
                    });
                }
                Dialog::ConfirmClose | Dialog::ConfirmOpen => {
                    let closing = matches!(dialog, Dialog::ConfirmClose);
                    ui.label("The map set has unsaved changes.");
                    ui.horizontal(|ui| {
                        if ui.button("Save").clicked() {
                            self.save();
                            keep = false;
                            if !self.project.dirty && closing {
                                self.allow_close = true;
                                ctx.send_viewport_cmd(ViewportCommand::Close);
                            }
                        }
                        if ui.button("Discard changes").clicked() {
                            keep = false;
                            if closing {
                                self.allow_close = true;
                                ctx.send_viewport_cmd(ViewportCommand::Close);
                            } else {
                                self.project.dirty = false;
                                self.open_maps();
                            }
                        }
                        if ui.button("Cancel").clicked() {
                            keep = false;
                        }
                    });
                }
                Dialog::Message(m) => {
                    ui.label(m.as_str());
                    if ui.button("OK").clicked() {
                        keep = false;
                    }
                }
                Dialog::Help => {
                    ui.add(egui::Label::new(egui::RichText::new(HELP).monospace()).extend());
                    if ui.button("Close").clicked() {
                        keep = false;
                    }
                }
            });
        if open && keep && self.dialog.is_none() {
            self.dialog = Some(dialog);
        }
    }

    fn status_bar(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            if let (Some((x, y)), Some(m)) = (self.hover, self.map()) {
                let i = y * m.width + x;
                let (bg, fg, inf) = (m.planes[0][i], m.planes[1][i], m.planes[2][i]);
                let ti = &self.project.data.tileinfo;
                let f = fg as usize;
                let mut s = format!("{:3},{:3}   BG {:4}   FG {:4}", x, y, bg, fg);
                if fg != 0 && f < ti.north.len() {
                    s += &format!(
                        " (walls T{} R{} B{} L{}{})",
                        ti.north[f],
                        ti.east[f],
                        ti.south[f],
                        ti.west[f],
                        intile_name(ti.intile[f]).map(|n| format!(", {}", n)).unwrap_or_default()
                    );
                }
                s += &format!("   Info {}", info_text(inf, self.cur == 0));
                ui.monospace(s);
                ui.separator();
            }
            ui.label(&self.status);
        });
    }
}

fn size_fields(ui: &mut egui::Ui, w: &mut usize, h: &mut usize) {
    ui.label("Width");
    ui.add(egui::DragValue::new(w).range(MIN_MAP_W..=MAX_MAP_SIDE));
    ui.end_row();
    ui.label("Height");
    ui.add(egui::DragValue::new(h).range(MIN_MAP_H..=MAX_MAP_SIDE));
    ui.end_row();
    if *w * *h > MAX_MAP_AREA {
        *h = MAX_MAP_AREA / *w;
    }
}

/// Short description of an info value for the status bar.
pub fn info_text(v: u16, world: bool) -> String {
    match info::describe(v, world) {
        _ if v == 0 => "0".into(),
        Some((name, _, _)) => format!("{} ({})", v_text(v), name),
        None => format!("{} (unknown)", v_text(v)),
    }
}

pub fn v_text(v: u16) -> String {
    if v >= 0x100 {
        format!("0x{:04x}", v)
    } else {
        v.to_string()
    }
}

/// Names of the special foreground tile types (tinf INTILE, low 7 bits).
pub fn intile_name(v: u8) -> Option<&'static str> {
    Some(match v & 0x7f {
        1 => "pole",
        2 => "door",
        3 => "deadly",
        4 => "drop",
        5 | 6 => "switch",
        7..=10 => "gem holder",
        15 => "bridge switch",
        0x12 | 0x13 => "switched tiles",
        21..=28 => "bonus item",
        _ => return None,
    })
}

const HELP: &str = "Commander Keen Editor – levels and the world maps of Keen 4, 5 and 6

Layers: background tiles, foreground (masked) tiles and the info layer
(Keen's start, enemies, items, platforms, door targets). The tools edit the
layer selected under \"Edit\" (keys 1, 2, 3).

Mouse
  left button        use the tool (draw, rectangle, fill, select, …)
  right click        pick the tile under the mouse into the brush
  right drag         pick a block of tiles into the brush
  middle drag, or Space + left drag   scroll
  wheel / trackpad   scroll; Cmd/Ctrl + wheel or pinch: zoom

Keys
  D R F S T          draw, rectangle, fill, select, door target tool
  1 2 3              edit background / foreground / info layer
  G, W               grid, tile properties (walls)
  + −                zoom, PageUp/PageDown: previous/next map
  Cmd/Ctrl+Z, Cmd/Ctrl+Shift+Z   undo, redo
  Cmd/Ctrl+C, X, Delete          copy selection to brush, cut, clear
  Cmd/Ctrl+S                     save the map set
  F5                 play the current map in the game
  F6                 play it starting at the tile under the mouse
  Test menu          Easy, Normal or Hard for play tests (remembered)

Palettes: drag over the tile set to take a block of tiles as brush.
Doors, teleporters and switches store their destination in the info
layer (x * 256 + y): use the door target tool, first click the door,
then the destination tile (where Keen will stand).

Saving writes GAMEMAPS.CKn and MAPHEAD.CKn into a folder. Play them with
  keen6 --data KEEN6 --maps FOLDER (keen4/keen5 for the other episodes)
Saving into the game folder itself keeps backups (*.bak).";

const DIFFICULTIES: [&str; 3] = ["Easy", "Normal", "Hard"];
const DIFFICULTY_KEY: &str = "test_difficulty";

impl eframe::App for App {
    fn save(&mut self, storage: &mut dyn eframe::Storage) {
        storage.set_string(DIFFICULTY_KEY, self.difficulty.to_string());
    }

    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let ctx = ui.ctx().clone();
        if self.tex.is_none() {
            self.tex = Some(Textures::new(&ctx, &self.project.data.gfx));
        }
        if ctx.input(|i| i.viewport().close_requested()) && self.project.dirty && !self.allow_close {
            ctx.send_viewport_cmd(ViewportCommand::CancelClose);
            self.dialog = Some(Dialog::ConfirmClose);
        }
        let title = format!(
            "{}{} – Commander Keen {} Editor",
            if self.project.dirty { "• " } else { "" },
            self.project.maps_dir.as_ref().unwrap_or(&self.project.data_dir).display(),
            self.project.data.layout.number
        );
        if title != self.title {
            ctx.send_viewport_cmd(ViewportCommand::Title(title.clone()));
            self.title = title;
        }
        if self.dialog.is_none() {
            self.shortcuts(&ctx);
        }
        self.switch_brush();

        egui::Panel::top("menu").show(ui, |ui| {
            self.menu_bar(ui);
            self.toolbar(ui);
        });
        egui::Panel::bottom("status").show(ui, |ui| self.status_bar(ui));
        egui::Panel::left("maps").default_size(200.0).resizable(true).show(ui, |ui| self.map_list(ui));
        egui::Panel::right("palette").default_size(330.0).resizable(true).show(ui, |ui| self.palette(ui));
        egui::CentralPanel::no_frame().show(ui, |ui| self.canvas(ui));
        self.dialogs(&ctx);
        self.debug_shot(&ctx);
    }
}

fn main() -> eframe::Result {
    let mut data_dir = ["KEEN6", "KEEN5", "KEEN4"]
        .iter()
        .map(PathBuf::from)
        .find(|d| d.is_dir())
        .unwrap_or_else(|| PathBuf::from("KEEN6"));
    let mut maps_dir: Option<PathBuf> = None;
    let mut map = 0;
    let args: Vec<String> = std::env::args().skip(1).collect();
    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--data" if i + 1 < args.len() => {
                i += 1;
                data_dir = PathBuf::from(&args[i]);
            }
            "--maps" if i + 1 < args.len() => {
                i += 1;
                maps_dir = Some(PathBuf::from(&args[i]));
            }
            "-h" | "--help" => {
                println!("{}", USAGE);
                return Ok(());
            }
            a => match a.parse() {
                Ok(n) => map = n,
                Err(_) => eprintln!("unknown option {}", a),
            },
        }
        i += 1;
    }
    let project = match Project::open(&data_dir, maps_dir.as_deref()) {
        Ok(p) => p,
        Err(e) => {
            // ask for the game directory
            eprintln!("{}", e);
            let Some(dir) = rfd::FileDialog::new().set_title("Select the Keen 4, 5 or 6 game folder (KEENnE.EXE / KEEN6C.EXE, *.CKn)").pick_folder()
            else {
                return Ok(());
            };
            match Project::open(&dir, maps_dir.as_deref()) {
                Ok(p) => p,
                Err(e) => {
                    rfd::MessageDialog::new().set_title("Commander Keen Editor").set_description(&e).show();
                    return Ok(());
                }
            }
        }
    };
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default().with_inner_size([1400.0, 900.0]).with_title("Commander Keen Editor"),
        ..Default::default()
    };
    eframe::run_native("Commander Keen Editor", options, Box::new(move |cc| Ok(Box::new(App::new(project, map, cc.storage)))))
}
