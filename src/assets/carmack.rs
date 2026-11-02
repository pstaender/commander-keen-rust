//! Carmack (near/far pointer LZ) and RLEW decompression used by GAMEMAPS.

pub const NEARTAG: u8 = 0xa7;
pub const FARTAG: u8 = 0xa8;

fn w(src: &[u8], i: usize) -> u16 {
    if i + 1 < src.len() {
        u16::from_le_bytes([src[i], src[i + 1]])
    } else {
        0
    }
}

/// Expands Carmack-compressed data. `src` starts after the length word.
/// `len` is the expanded length in bytes.
pub fn carmack_expand(src: &[u8], len: usize) -> Vec<u16> {
    let words = len / 2;
    let mut out: Vec<u16> = Vec::with_capacity(words);
    let mut i = 0usize;
    while out.len() < words && i + 1 < src.len() {
        let ch = w(src, i);
        i += 2;
        let count = (ch & 0xff) as usize;
        let tag = (ch >> 8) as u8;
        if tag == NEARTAG || tag == FARTAG {
            if count == 0 {
                // literal word: high byte is tag, low byte follows
                let lo = src.get(i).copied().unwrap_or(0) as u16;
                i += 1;
                out.push(lo | ((tag as u16) << 8));
                continue;
            }
            let start = if tag == NEARTAG {
                let off = src.get(i).copied().unwrap_or(0) as usize;
                i += 1;
                out.len().saturating_sub(off)
            } else {
                let off = w(src, i) as usize;
                i += 2;
                off
            };
            for k in 0..count {
                let v = out.get(start + k).copied().unwrap_or(0);
                out.push(v);
            }
        } else {
            out.push(ch);
        }
    }
    out.resize(words, 0);
    out
}

/// Expands RLEW data (words). First word of `src` is the expanded byte length.
pub fn rlew_expand(src: &[u16], tag: u16) -> Vec<u16> {
    if src.is_empty() {
        return Vec::new();
    }
    let words = src[0] as usize / 2;
    let mut out = Vec::with_capacity(words);
    let mut i = 1;
    while out.len() < words && i < src.len() {
        let v = src[i];
        i += 1;
        if v == tag {
            let count = src.get(i).copied().unwrap_or(0) as usize;
            let val = src.get(i + 1).copied().unwrap_or(0);
            i += 2;
            for _ in 0..count {
                out.push(val);
            }
        } else {
            out.push(v);
        }
    }
    out.resize(words, 0);
    out
}

/// RLEW compression (the inverse of `rlew_expand`): the first output word is
/// the expanded byte length, runs of more than 3 words (and every word equal
/// to the tag) become `tag, count, value`.
pub fn rlew_compress(src: &[u16], tag: u16) -> Vec<u16> {
    let mut out = vec![(src.len() * 2) as u16];
    let mut i = 0;
    while i < src.len() {
        let v = src[i];
        let mut n = 1;
        while i + n < src.len() && src[i + n] == v && n < 0xffff {
            n += 1;
        }
        if n > 3 || v == tag {
            out.extend_from_slice(&[tag, n as u16, v]);
        } else {
            out.extend(std::iter::repeat_n(v, n));
        }
        i += n;
    }
    out
}

/// Carmack compression (the inverse of `carmack_expand`): repeated word
/// sequences become near (up to 255 words back) or far (absolute word
/// offset) pointers; literal words with a tag as high byte are escaped.
pub fn carmack_compress(src: &[u16]) -> Vec<u8> {
    use std::collections::HashMap;
    let mut out = Vec::with_capacity(src.len() * 2);
    // positions of each word pair, most recent last
    let mut pairs: HashMap<(u16, u16), Vec<usize>> = HashMap::new();
    let add = |pairs: &mut HashMap<(u16, u16), Vec<usize>>, j: usize| {
        if j + 1 < src.len() {
            pairs.entry((src[j], src[j + 1])).or_default().push(j);
        }
    };
    let mut i = 0;
    while i < src.len() {
        let (mut best_len, mut best_pos) = (0usize, 0usize);
        let (mut near_len, mut near_pos) = (0usize, 0usize);
        if i + 1 < src.len() {
            if let Some(cands) = pairs.get(&(src[i], src[i + 1])) {
                for &j in cands.iter().rev().take(64) {
                    // no overlap with the words being produced
                    let max = (i - j).min(src.len() - i).min(255);
                    let mut n = 0;
                    while n < max && src[j + n] == src[i + n] {
                        n += 1;
                    }
                    if i - j <= 255 && n > near_len {
                        near_len = n;
                        near_pos = j;
                    }
                    if n > best_len && j <= 0xffff {
                        best_len = n;
                        best_pos = j;
                    }
                }
            }
        }
        let step;
        if near_len >= 2 && near_len + 1 >= best_len {
            out.extend_from_slice(&[near_len as u8, NEARTAG, (i - near_pos) as u8]);
            step = near_len;
        } else if best_len >= 3 {
            out.extend_from_slice(&[best_len as u8, FARTAG]);
            out.extend_from_slice(&(best_pos as u16).to_le_bytes());
            step = best_len;
        } else {
            let v = src[i];
            let hi = (v >> 8) as u8;
            if hi == NEARTAG || hi == FARTAG {
                out.extend_from_slice(&[0, hi, v as u8]);
            } else {
                out.extend_from_slice(&v.to_le_bytes());
            }
            step = 1;
        }
        for j in i..i + step {
            add(&mut pairs, j);
        }
        i += step;
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn compress_roundtrip() {
        let mut data: Vec<u16> = Vec::new();
        for i in 0..5000u32 {
            data.push(match i % 97 {
                0..=40 => 0,
                41..=60 => (i % 7) as u16,
                61 => 0xa7ff,
                62 => 0xa812,
                63 => 0xabcd,
                _ => ((i * 2654435761) >> 20) as u16,
            });
        }
        let tag = 0xabcd;
        let rlew = rlew_compress(&data, tag);
        assert_eq!(rlew_expand(&rlew, tag), data);
        let carm = carmack_compress(&rlew);
        assert_eq!(carmack_expand(&carm, rlew.len() * 2), rlew);
    }
}
