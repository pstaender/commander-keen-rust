//! Locates the data headers that were linked into the executable
//! (AUDIOHED, AUDIODCT, EGAHEAD, EGADICT, MAPHEAD + tileinfo).
//!
//! Offsets were found by analysing the unpacked v1.4 executables with REA
//! (see claude_history.log and `episode.rs`); a validating search is used
//! as fallback so other builds have a chance to work as well.

use super::episode::Layout;
use super::lzexe;

pub struct ExeData {
    pub image: Vec<u8>,
    pub audiohed: Vec<u32>,
    pub audiodict: Vec<u8>,
    pub egahead: Vec<u32>,
    pub egadict: Vec<u8>,
    pub maphead: Vec<u8>,
}


fn valid_dict(img: &[u8], o: usize) -> bool {
    if o + 1020 > img.len() {
        return false;
    }
    let mut seen = vec![false; 512];
    for i in 0..510 {
        let v = u16::from_le_bytes([img[o + i * 2], img[o + i * 2 + 1]]) as usize;
        if v > 0x1fd || seen[v] {
            return false;
        }
        seen[v] = true;
    }
    true
}

fn read3(img: &[u8], o: usize, n: usize) -> Vec<u32> {
    (0..n)
        .map(|i| {
            let p = o + i * 3;
            img[p] as u32 | (img[p + 1] as u32) << 8 | (img[p + 2] as u32) << 16
        })
        .collect()
}

fn read4(img: &[u8], o: usize, n: usize) -> Vec<u32> {
    (0..n)
        .map(|i| u32::from_le_bytes(img[o + i * 4..o + i * 4 + 4].try_into().unwrap()))
        .collect()
}

impl ExeData {
    pub fn load(lay: &Layout, exe: &[u8], egagraph_len: u32, audio_len: u32) -> Result<ExeData, String> {
        let image = lzexe::load_image(exe).map_err(|e| e.to_string())?;
        let nchunks = lay.numchunks;
        let naudio = lay.numaudiochunks();
        let (audiohed_ofs, egahead_ofs, maphead_ofs) = (lay.audiohed_ofs, lay.egahead_ofs, lay.maphead_ofs);
        let (audiodict_ofs, egadict_ofs) = (lay.audiodict_ofs, lay.egadict_ofs);
        let maphead_len = lay.maphead_len();

        // dictionaries
        let mut dicts: Vec<usize> = Vec::new();
        if valid_dict(&image, audiodict_ofs) && valid_dict(&image, egadict_ofs) {
            dicts.push(audiodict_ofs);
            dicts.push(egadict_ofs);
        } else {
            let mut o = 0;
            while o + 1020 < image.len() {
                if valid_dict(&image, o) {
                    dicts.push(o);
                    o += 1020;
                } else {
                    o += 2;
                }
            }
        }
        if dicts.len() < 2 {
            return Err("could not find Huffman dictionaries in executable".into());
        }

        // EGAHEAD: chunk NUMCHUNKS offset equals file length
        let find_ega = |o: usize| {
            o + (nchunks + 1) * 3 <= image.len() && {
                let h = read3(&image, o, nchunks + 1);
                h[0] == 0 && h[nchunks] == egagraph_len
            }
        };
        let ega_ofs = if find_ega(egahead_ofs) {
            egahead_ofs
        } else {
            (0..image.len()).find(|&o| find_ega(o)).ok_or("EGAHEAD not found")?
        };
        let find_audio = |o: usize| {
            o + (naudio + 1) * 4 <= image.len() && {
                let h = read4(&image, o, naudio + 1);
                h[0] == 0 && h[naudio] == audio_len
            }
        };
        let audio_ofs = if find_audio(audiohed_ofs) {
            audiohed_ofs
        } else {
            (0..image.len()).step_by(2).find(|&o| find_audio(o)).ok_or("AUDIOHED not found")?
        };
        let find_map = |o: usize| o + maphead_len <= image.len() && image[o] == 0xcd && image[o + 1] == 0xab;
        let map_ofs = if find_map(maphead_ofs) {
            maphead_ofs
        } else {
            (0..image.len()).find(|&o| find_map(o)).ok_or("MAPHEAD not found")?
        };

        // Decide which dictionary is which: the EGA one decodes chunk 0
        // (the picture table) to sane values; we simply keep the order seen
        // in all v1.4 builds (audio first) unless offsets say otherwise.
        let (ad, ed) = (dicts[0], dicts[1]);
        Ok(ExeData {
            audiohed: read4(&image, audio_ofs, naudio + 1),
            audiodict: image[ad..ad + 1020].to_vec(),
            egahead: read3(&image, ega_ofs, nchunks + 1),
            egadict: image[ed..ed + 1020].to_vec(),
            maphead: image[map_ofs..map_ofs + maphead_len].to_vec(),
            image,
        })
    }
}
