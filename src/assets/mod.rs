//! Loading of the original Keen 4, 5 and 6 data files.

pub mod audio;
pub mod carmack;
pub mod episode;
pub mod exe;
pub mod graphics;
pub mod huffman;
pub mod lzexe;
pub mod maps;

pub use episode::{Episode, Layout};
use std::path::{Path, PathBuf};

pub struct GameData {
    pub dir: PathBuf,
    pub episode: Episode,
    pub layout: &'static Layout,
    pub exe: exe::ExeData,
    pub gfx: graphics::EgaGraph,
    pub maps: maps::Maps,
    pub tileinfo: maps::TileInfo,
    pub audio: audio::Audio,
}

/// Finds a file case-insensitively inside `dir`.
pub fn find_file(dir: &Path, name: &str) -> Option<PathBuf> {
    let direct = dir.join(name);
    if direct.exists() {
        return Some(direct);
    }
    std::fs::read_dir(dir).ok()?.flatten().map(|e| e.path()).find(|p| {
        p.file_name().and_then(|n| n.to_str()).map(|n| n.eq_ignore_ascii_case(name)).unwrap_or(false)
    })
}

fn read(dir: &Path, name: &str) -> Result<Vec<u8>, String> {
    let p = find_file(dir, name).ok_or_else(|| format!("missing data file {} in {}", name, dir.display()))?;
    std::fs::read(&p).map_err(|e| format!("{}: {}", p.display(), e))
}

impl GameData {
    /// Loads the episode found in `dir`.
    pub fn load(dir: &Path) -> Result<GameData, String> {
        Self::load_with_maps(dir, None)
    }

    /// Loads the data; GAMEMAPS.CKn (and MAPHEAD.CKn, if present) come from
    /// `maps_dir` when given (edited levels, e.g. from the Commander Keen
    /// Editor). A MAPHEAD.CKn file replaces the map header linked into the
    /// executable, like in modding tools.
    pub fn load_with_maps(dir: &Path, maps_dir: Option<&Path>) -> Result<GameData, String> {
        let ep = Episode::detect(dir).ok_or_else(|| {
            format!("no Commander Keen 4, 5 or 6 data (EGAGRAPH.CK4/5/6) found in {}", dir.display())
        })?;
        Self::load_episode(ep, dir, maps_dir)
    }

    pub fn load_episode(ep: Episode, dir: &Path, maps_dir: Option<&Path>) -> Result<GameData, String> {
        let lay = ep.layout();
        let exe_bytes = lay
            .exes
            .iter()
            .find_map(|n| read(dir, n).ok())
            .ok_or_else(|| format!("missing executable {} in {}", lay.exes[0], dir.display()))?;
        let ega = read(dir, &lay.file("EGAGRAPH"))?;
        let audio = read(dir, &lay.file("AUDIO"))?;
        let mdir = maps_dir.unwrap_or(dir);
        let gamemaps = read(mdir, &lay.file("GAMEMAPS"))?;
        let mut exe = exe::ExeData::load(lay, &exe_bytes, ega.len() as u32, audio.len() as u32)?;
        if let Ok(mh) = read(mdir, &lay.file("MAPHEAD")) {
            if mh.len() < lay.maphead_len() {
                return Err(format!("{} in {} is too short", lay.file("MAPHEAD"), mdir.display()));
            }
            exe.maphead = mh[..lay.maphead_len()].to_vec();
        }
        let gfx = graphics::EgaGraph::load(lay, &ega, &exe.egahead, &exe.egadict);
        let maps = maps::Maps::load(&gamemaps, &exe.maphead);
        let tileinfo = maps::TileInfo::from_maphead(lay, &exe.maphead);
        let audio = audio::Audio::load(lay, &audio, &exe.audiohed, &exe.audiodict);
        Ok(GameData { dir: dir.to_path_buf(), episode: ep, layout: lay, exe, gfx, maps, tileinfo, audio })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Re-encodes the original maps and loads them back (needs KEEN4-6/).
    #[test]
    fn reencode_original_maps() {
        for ep in ["KEEN4", "KEEN5", "KEEN6"] {
            reencode(ep);
        }
    }

    fn reencode(name: &str) {
        let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join(name);
        let Ok(data) = GameData::load(&dir) else {
            eprintln!("{} data not found, skipped", name);
            return;
        };
        let lay = data.layout;
        let (gm, mh) = data.maps.encode(&data.exe.maphead);
        let out = std::env::temp_dir().join(format!("keen6-maps-test-{}", std::process::id()));
        std::fs::create_dir_all(&out).unwrap();
        std::fs::write(out.join(lay.file("GAMEMAPS")), &gm).unwrap();
        std::fs::write(out.join(lay.file("MAPHEAD")), &mh).unwrap();
        let back = GameData::load_with_maps(&dir, Some(&out)).unwrap();
        std::fs::remove_dir_all(&out).ok();
        assert_eq!(back.exe.maphead[402..], data.exe.maphead[402..]);
        for (a, b) in data.maps.maps.iter().zip(&back.maps.maps) {
            match (a, b) {
                (Some(a), Some(b)) => {
                    assert_eq!((a.width, a.height, &a.name), (b.width, b.height, &b.name));
                    assert_eq!(a.planes, b.planes);
                }
                (None, None) => {}
                _ => panic!("map presence differs"),
            }
        }
        let orig = std::fs::metadata(dir.join(lay.file("GAMEMAPS"))).map(|m| m.len()).unwrap_or(0);
        eprintln!("{}: GAMEMAPS original {} bytes, re-encoded {}", name, orig, gm.len());
    }
}
