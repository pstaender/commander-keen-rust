//! AUDIO.CKn: PC speaker sounds, AdLib sounds and IMF music.

use super::episode::Layout;
use super::huffman::Huffman;

#[derive(Clone, Default)]
pub struct PcSound {
    pub priority: u16,
    /// one byte per 140 Hz tick, PIT divisor = value * 60, 0 = silence
    pub data: Vec<u8>,
}

#[derive(Clone, Copy, Default, Debug)]
pub struct Instrument {
    pub m_char: u8,
    pub c_char: u8,
    pub m_scale: u8,
    pub c_scale: u8,
    pub m_attack: u8,
    pub c_attack: u8,
    pub m_sus: u8,
    pub c_sus: u8,
    pub m_wave: u8,
    pub c_wave: u8,
    pub n_conn: u8,
}

#[derive(Clone, Default)]
pub struct AdlibSound {
    pub priority: u16,
    pub inst: Instrument,
    pub block: u8,
    /// one F-number low byte per 140 Hz tick, 0 = key off
    pub data: Vec<u8>,
}

#[derive(Clone, Default)]
pub struct Song {
    /// (register, value, delay in 560 Hz ticks)
    pub commands: Vec<(u8, u8, u16)>,
}

pub struct Audio {
    pub pc: Vec<PcSound>,
    pub adlib: Vec<AdlibSound>,
    pub songs: Vec<Song>,
}

impl Audio {
    pub fn load(lay: &Layout, file: &[u8], head: &[u32], dict: &[u8]) -> Audio {
        let huff = Huffman::new(dict);
        let chunk = |c: usize| -> Vec<u8> {
            let o = head[c] as usize;
            let next = head.get(c + 1).copied().unwrap_or(0) as usize;
            if o + 4 > file.len() || next == o {
                return Vec::new();
            }
            let len = u32::from_le_bytes(file[o..o + 4].try_into().unwrap()) as usize;
            if len > 0x10000 {
                return Vec::new();
            }
            huff.expand(&file[o + 4..], len)
        };
        let pc = (0..lay.numsounds)
            .map(|i| {
                let d = chunk(i);
                if d.len() < 6 {
                    return PcSound::default();
                }
                let len = u32::from_le_bytes(d[0..4].try_into().unwrap()) as usize;
                let priority = u16::from_le_bytes([d[4], d[5]]);
                let end = (6 + len).min(d.len());
                PcSound { priority, data: d[6..end].to_vec() }
            })
            .collect();
        let adlib = (0..lay.numsounds)
            .map(|i| {
                let d = chunk(lay.startadlibsounds() + i);
                if d.len() < 23 {
                    return AdlibSound::default();
                }
                let len = u32::from_le_bytes(d[0..4].try_into().unwrap()) as usize;
                let priority = u16::from_le_bytes([d[4], d[5]]);
                let b = &d[6..];
                let inst = Instrument {
                    m_char: b[0],
                    c_char: b[1],
                    m_scale: b[2],
                    c_scale: b[3],
                    m_attack: b[4],
                    c_attack: b[5],
                    m_sus: b[6],
                    c_sus: b[7],
                    m_wave: b[8],
                    c_wave: b[9],
                    n_conn: b[10],
                };
                let block = d[22];
                let end = (23 + len).min(d.len());
                AdlibSound { priority, inst, block, data: d[23..end].to_vec() }
            })
            .collect();
        let songs = (0..lay.numsongs)
            .map(|i| {
                let d = chunk(lay.startmusic() + i);
                if d.len() < 2 {
                    return Song::default();
                }
                let len = (u16::from_le_bytes([d[0], d[1]]) as usize).min(d.len() - 2);
                let mut commands = Vec::with_capacity(len / 4);
                let mut p = 2;
                while p + 4 <= 2 + len {
                    commands.push((d[p], d[p + 1], u16::from_le_bytes([d[p + 2], d[p + 3]])));
                    p += 4;
                }
                Song { commands }
            })
            .collect();
        Audio { pc, adlib, songs }
    }
}
