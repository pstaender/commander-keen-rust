//! The edited map set: original game data plus the maps being edited,
//! saving (GAMEMAPS.CKn + MAPHEAD.CKn) and play testing with the game.

use keen::assets::maps::{Map, Maps, NUMMAPS};
use keen::assets::{find_file, GameData};
use std::path::{Path, PathBuf};

pub struct Project {
    pub data_dir: PathBuf,
    pub data: GameData,
    /// MAPHEAD with the tile info (header offsets are rewritten on save)
    pub maphead: Vec<u8>,
    pub rlew_tag: u16,
    pub maps: Vec<Option<Map>>,
    /// where the map set is loaded from / saved to (None: the original data)
    pub maps_dir: Option<PathBuf>,
    pub dirty: bool,
}

impl Project {
    pub fn open(data_dir: &Path, maps_dir: Option<&Path>) -> Result<Project, String> {
        let data = GameData::load_with_maps(data_dir, maps_dir)?;
        crate::info::set_episode(data.episode.number());
        let maphead = data.exe.maphead.clone();
        let rlew_tag = data.maps.rlew_tag;
        let mut maps = data.maps.maps.clone();
        maps.resize(NUMMAPS, None);
        Ok(Project {
            data_dir: data_dir.to_path_buf(),
            data,
            maphead,
            rlew_tag,
            maps,
            maps_dir: maps_dir.map(Path::to_path_buf),
            dirty: false,
        })
    }

    /// Replaces the maps with the map set in `dir` (GAMEMAPS.CKn and an
    /// optional MAPHEAD.CKn), keeping the loaded graphics.
    pub fn load_maps(&mut self, dir: &Path) -> Result<(), String> {
        let read = |name: &str| -> Result<Vec<u8>, String> {
            let p = find_file(dir, name).ok_or_else(|| format!("{} not found in {}", name, dir.display()))?;
            std::fs::read(&p).map_err(|e| format!("{}: {}", p.display(), e))
        };
        let lay = self.data.layout;
        let gm = read(&lay.file("GAMEMAPS"))?;
        let mh = match read(&lay.file("MAPHEAD")) {
            Ok(mh) if mh.len() >= self.maphead.len() => mh[..self.maphead.len()].to_vec(),
            Ok(_) => return Err(format!("{} is too short", lay.file("MAPHEAD"))),
            Err(_) => self.data.exe.maphead.clone(),
        };
        let maps = Maps::load(&gm, &mh);
        self.rlew_tag = maps.rlew_tag;
        self.maps = maps.maps;
        self.maps.resize(NUMMAPS, None);
        self.maphead = mh;
        self.maps_dir = Some(dir.to_path_buf());
        self.dirty = false;
        Ok(())
    }

    fn encode(&self, maps: Vec<Option<Map>>) -> (Vec<u8>, Vec<u8>) {
        Maps { rlew_tag: self.rlew_tag, maps }.encode(&self.maphead)
    }

    /// Writes GAMEMAPS.CKn and MAPHEAD.CKn into `dir`. Existing files are
    /// kept as *.bak the first time.
    pub fn save_to(&mut self, dir: &Path) -> Result<(), String> {
        std::fs::create_dir_all(dir).map_err(|e| format!("{}: {}", dir.display(), e))?;
        let (gm, mh) = self.encode(self.maps.clone());
        let lay = self.data.layout;
        for (name, bytes) in [(lay.file("GAMEMAPS"), &gm), (lay.file("MAPHEAD"), &mh)] {
            let path = find_file(dir, &name).unwrap_or_else(|| dir.join(&name));
            let bak = path.with_extension(format!("{}.bak", lay.ext));
            if path.exists() && !bak.exists() {
                std::fs::copy(&path, &bak).map_err(|e| format!("{}: {}", bak.display(), e))?;
            }
            std::fs::write(&path, bytes).map_err(|e| format!("{}: {}", path.display(), e))?;
        }
        self.maps_dir = Some(dir.to_path_buf());
        self.dirty = false;
        Ok(())
    }

    /// Starts the game with the current (unsaved) maps on map `n` at
    /// `difficulty` (1 easy, 2 normal, 3 hard). With `start`, Keen starts at
    /// that tile instead of the map's start point.
    pub fn play_test(&self, n: usize, start: Option<(usize, usize)>, difficulty: i32) -> Result<(), String> {
        let mut maps = self.maps.clone();
        let Some(Some(m)) = maps.get_mut(n) else { return Err("this map slot is empty".into()) };
        if let Some((x, y)) = start {
            let keen = if n == 0 { 3 } else { 1 };
            for v in m.planes[2].iter_mut() {
                if (1..=3).contains(v) {
                    *v = 0;
                }
            }
            m.planes[2][y * m.width + x] = keen;
        } else if !m.planes[2].iter().any(|&v| (1..=3).contains(&v)) {
            return Err("the map has no Keen start point (info values 1-3)".into());
        }
        let dir = std::env::temp_dir().join(format!("keen-editor-test-{}", std::process::id()));
        std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
        let (gm, mh) = self.encode(maps);
        let lay = self.data.layout;
        std::fs::write(dir.join(lay.file("GAMEMAPS")), gm).map_err(|e| e.to_string())?;
        std::fs::write(dir.join(lay.file("MAPHEAD")), mh).map_err(|e| e.to_string())?;
        let exe = format!("keen{}", lay.number);
        let game = game_binary(&exe).ok_or(format!("the game executable ({}) was not found next to the editor", exe))?;
        std::process::Command::new(&game)
            .arg("--data")
            .arg(&self.data_dir)
            .arg("--maps")
            .arg(&dir)
            .arg("--level")
            .arg(n.to_string())
            .arg("--difficulty")
            .arg(difficulty.to_string())
            .spawn()
            .map_err(|e| format!("{}: {}", game.display(), e))?;
        Ok(())
    }

    /// Name shown for map slot `n`.
    pub fn map_title(&self, n: usize) -> String {
        match &self.maps[n] {
            Some(m) if n == 0 => format!("{:2}  {} (world map)", n, m.name),
            Some(m) => format!("{:2}  {}", n, m.name),
            None => format!("{:2}  (empty)", n),
        }
    }
}

/// The game executable (keen4/keen5/keen6): next to the editor (same cargo
/// target directory), otherwise from the PATH.
fn game_binary(exe: &str) -> Option<PathBuf> {
    let name = if cfg!(windows) { format!("{}.exe", exe) } else { exe.to_string() };
    if let Ok(exe) = std::env::current_exe() {
        let p = exe.with_file_name(&name);
        if p.exists() {
            return Some(p);
        }
    }
    std::env::var_os("PATH").and_then(|paths| std::env::split_paths(&paths).map(|d| d.join(&name)).find(|p| p.exists()))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Builds a small test level in a new slot and saves it; with
    /// KEEN6_EDITOR_TEST_OUT=dir the map set is kept there for the game.
    #[test]
    fn new_small_level() {
        let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../KEEN6");
        let Ok(mut p) = Project::open(&dir, None) else { return };
        let ti = &p.data.tileinfo;
        // a foreground tile that is solid on all sides
        let solid = (1..ti.north.len())
            .find(|&t| ti.north[t] == 1 && ti.south[t] == 1 && ti.east[t] == 1 && ti.west[t] == 1 && ti.intile[t] == 0)
            .unwrap() as u16;
        let (w, h) = (crate::MIN_MAP_W, crate::MIN_MAP_H);
        let mut m = Map::new(w, h, "Small test");
        for y in 0..h {
            for x in 0..w {
                if x < 2 || y < 2 || x >= w - 2 || y >= h - 2 {
                    m.planes[1][y * w + x] = solid;
                }
            }
        }
        m.planes[2][(h - 4) * w + 4] = 1;
        p.maps[19] = Some(m);
        let out = std::env::var("KEEN6_EDITOR_TEST_OUT")
            .map(PathBuf::from)
            .unwrap_or_else(|_| std::env::temp_dir().join(format!("keen-editor-unit-{}", std::process::id())));
        p.save_to(&out).unwrap();
        let back = Project::open(&dir, Some(&out)).unwrap();
        assert_eq!(back.maps[19].as_ref().unwrap().planes, p.maps[19].as_ref().unwrap().planes);
        assert_eq!(back.maps[3].as_ref().unwrap().planes, p.maps[3].as_ref().unwrap().planes);
        if std::env::var("KEEN6_EDITOR_TEST_OUT").is_err() {
            std::fs::remove_dir_all(&out).ok();
        }
    }
}
