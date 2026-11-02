//! GAMEMAPS.CKn loader (MAPHEAD offsets + Carmack + RLEW planes) and the
//! tile info tables appended to MAPHEAD.

use super::carmack::{carmack_compress, carmack_expand, rlew_compress, rlew_expand};
use super::episode::Layout;

pub const NUMMAPS: usize = 100;

/// Map name length in the map header (including the terminating zero).
pub const MAPNAME_LEN: usize = 16;

#[derive(Clone, Default)]
pub struct Map {
    pub width: usize,
    pub height: usize,
    pub name: String,
    /// plane 0: background tiles, 1: foreground (masked) tiles, 2: info
    pub planes: [Vec<u16>; 3],
}

/// Per-tile attributes (ID_RF tinf[] arrays).
#[derive(Clone)]
pub struct TileInfo {
    pub speed: Vec<u8>,    // background animation speed (tics)
    pub anim: Vec<i8>,     // background animation offset
    pub north: Vec<u8>,    // foreground: top wall/slope type
    pub east: Vec<u8>,
    pub south: Vec<u8>,
    pub west: Vec<u8>,
    pub manim: Vec<i8>,    // foreground animation offset
    pub intile: Vec<u8>,   // foreground misc/special (bit 7 = draw in front)
    pub mspeed: Vec<u8>,   // foreground animation speed
}

impl TileInfo {
    pub fn from_maphead(lay: &Layout, mh: &[u8]) -> TileInfo {
        let mut o = 402;
        let mut take = |n: usize| {
            let v = mh[o..o + n].to_vec();
            o += n;
            v
        };
        let speed = take(lay.numtile16);
        let anim = take(lay.numtile16).into_iter().map(|b| b as i8).collect();
        let north = take(lay.numtile16m);
        let east = take(lay.numtile16m);
        let south = take(lay.numtile16m);
        let west = take(lay.numtile16m);
        let manim = take(lay.numtile16m).into_iter().map(|b| b as i8).collect();
        let intile = take(lay.numtile16m);
        let mspeed = take(lay.numtile16m);
        TileInfo { speed, anim, north, east, south, west, manim, intile, mspeed }
    }
}

fn u32le(d: &[u8], o: usize) -> u32 {
    u32::from_le_bytes(d[o..o + 4].try_into().unwrap())
}
fn u16le(d: &[u8], o: usize) -> u16 {
    u16::from_le_bytes([d[o], d[o + 1]])
}

pub struct Maps {
    pub rlew_tag: u16,
    pub maps: Vec<Option<Map>>,
}

impl Maps {
    pub fn load(file: &[u8], maphead: &[u8]) -> Maps {
        let rlew_tag = u16le(maphead, 0);
        let mut maps = Vec::with_capacity(NUMMAPS);
        for i in 0..NUMMAPS {
            let pos = u32le(maphead, 2 + i * 4) as usize;
            if pos == 0 || pos + 38 > file.len() {
                maps.push(None);
                continue;
            }
            let h = &file[pos..pos + 38];
            let width = u16le(h, 18) as usize;
            let height = u16le(h, 20) as usize;
            let name: String = h[22..38].iter().take_while(|&&c| c != 0).map(|&c| c as char).collect();
            let mut planes: [Vec<u16>; 3] = Default::default();
            for p in 0..3 {
                let start = u32le(h, p * 4) as usize;
                let len = u16le(h, 12 + p * 2) as usize;
                if start == 0 || start + len > file.len() {
                    planes[p] = vec![0; width * height];
                    continue;
                }
                let src = &file[start..start + len];
                let exp = u16le(src, 0) as usize;
                let carm = carmack_expand(&src[2..], exp);
                let mut plane = rlew_expand(&carm, rlew_tag);
                plane.resize(width * height, 0);
                planes[p] = plane;
            }
            maps.push(Some(Map { width, height, name, planes }));
        }
        Maps { rlew_tag, maps }
    }
}

impl Map {
    /// An empty map (all planes zero).
    pub fn new(width: usize, height: usize, name: &str) -> Map {
        let n = width * height;
        Map { width, height, name: name.to_string(), planes: [vec![0; n], vec![0; n], vec![0; n]] }
    }
}

impl Maps {
    /// Writes the maps in the TED5 format: returns the new GAMEMAPS file
    /// and MAPHEAD (header offsets; the tile info is copied from `maphead`).
    /// Planes are RLEW + Carmack compressed like the originals.
    pub fn encode(&self, maphead: &[u8]) -> (Vec<u8>, Vec<u8>) {
        let mut file = b"TED5v1.0".to_vec();
        let mut head = maphead.to_vec();
        head[0..2].copy_from_slice(&self.rlew_tag.to_le_bytes());
        for i in 0..NUMMAPS {
            let mut pos = 0u32;
            if let Some(Some(m)) = self.maps.get(i) {
                let mut starts = [0u32; 3];
                let mut lens = [0u16; 3];
                for p in 0..3 {
                    let rlew = rlew_compress(&m.planes[p], self.rlew_tag);
                    let carm = carmack_compress(&rlew);
                    starts[p] = file.len() as u32;
                    file.extend_from_slice(&((rlew.len() * 2) as u16).to_le_bytes());
                    file.extend_from_slice(&carm);
                    lens[p] = (carm.len() + 2) as u16;
                }
                pos = file.len() as u32;
                for s in starts {
                    file.extend_from_slice(&s.to_le_bytes());
                }
                for l in lens {
                    file.extend_from_slice(&l.to_le_bytes());
                }
                file.extend_from_slice(&(m.width as u16).to_le_bytes());
                file.extend_from_slice(&(m.height as u16).to_le_bytes());
                let mut name = [0u8; MAPNAME_LEN];
                for (d, c) in name.iter_mut().zip(m.name.bytes().take(MAPNAME_LEN - 1)) {
                    *d = c;
                }
                file.extend_from_slice(&name);
                file.extend_from_slice(b"!ID!");
            }
            head[2 + i * 4..6 + i * 4].copy_from_slice(&pos.to_le_bytes());
        }
        (file, head)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn encode_roundtrip() {
        let mut maps = Maps { rlew_tag: 0xabcd, maps: vec![None; NUMMAPS] };
        let mut m = Map::new(40, 30, "Test level name too long");
        for (i, v) in m.planes[0].iter_mut().enumerate() {
            *v = (i / 7 % 13) as u16;
        }
        m.planes[1][5] = 0xabcd;
        m.planes[2][100] = 0xc001;
        maps.maps[3] = Some(m.clone());
        let head = vec![0u8; super::super::episode::K6.maphead_len()];
        let (file, mh) = maps.encode(&head);
        let back = Maps::load(&file, &mh);
        let b = back.maps[3].as_ref().unwrap();
        assert_eq!((b.width, b.height), (40, 30));
        assert_eq!(b.name, "Test level name");
        assert_eq!(b.planes, m.planes);
        assert!(back.maps[0].is_none());
    }
}
