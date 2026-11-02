//! LZEXE 0.91 unpacker. KEEN6C.EXE ships compressed with LZEXE; the
//! game data headers (EGAHEAD, AUDIOHED, MAPHEAD, dictionaries) are linked
//! into the executable, so we need the unpacked load image to read them.

use std::fmt;

#[derive(Debug)]
pub struct LzError(pub String);

impl fmt::Display for LzError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "LZEXE: {}", self.0)
    }
}

impl std::error::Error for LzError {}

fn rd16(d: &[u8], o: usize) -> Result<u16, LzError> {
    d.get(o..o + 2)
        .map(|b| u16::from_le_bytes([b[0], b[1]]))
        .ok_or_else(|| LzError("truncated".into()))
}

/// Returns the unpacked load image (without MZ header). If the file is not
/// LZEXE packed, returns the plain load image.
pub fn load_image(exe: &[u8]) -> Result<Vec<u8>, LzError> {
    if exe.len() < 0x20 || &exe[0..2] != b"MZ" {
        return Err(LzError("not an MZ executable".into()));
    }
    let hdr_par = rd16(exe, 8)? as usize;
    if &exe[0x1c..0x20] != b"LZ91" {
        // Uncompressed executable: image is everything after the header.
        let pages = rd16(exe, 4)? as usize;
        let last = rd16(exe, 2)? as usize;
        let mut size = pages * 512;
        if last != 0 {
            size = size - 512 + last;
        }
        let size = size.min(exe.len());
        return Ok(exe[hdr_par * 16..size].to_vec());
    }
    let mut p = hdr_par * 16;
    let mut out: Vec<u8> = Vec::with_capacity(300_000);
    let mut buf = rd16(exe, p)? as u32;
    p += 2;
    let mut cnt = 16u32;
    macro_rules! bit {
        () => {{
            let b = buf & 1;
            cnt -= 1;
            if cnt == 0 {
                buf = rd16(exe, p)? as u32;
                p += 2;
                cnt = 16;
            } else {
                buf >>= 1;
            }
            b
        }};
    }
    macro_rules! byte {
        () => {{
            let b = *exe.get(p).ok_or_else(|| LzError("truncated".into()))?;
            p += 1;
            b as i32
        }};
    }
    loop {
        if bit!() == 1 {
            let b = byte!();
            out.push(b as u8);
            continue;
        }
        let len: i32;
        let span: i32;
        if bit!() == 0 {
            let hi = bit!() as i32;
            let lo = bit!() as i32;
            len = ((hi << 1) | lo) + 2;
            span = byte!() | !0xff;
        } else {
            let lo = byte!();
            let hi = byte!();
            span = (lo | ((hi & 0xf8) << 5) | 0xe000) - 0x10000;
            let l = hi & 7;
            if l == 0 {
                let l2 = byte!();
                if l2 == 0 {
                    break;
                }
                if l2 == 1 {
                    continue;
                }
                len = l2 + 1;
            } else {
                len = l + 2;
            }
        }
        for _ in 0..len {
            let idx = out.len() as i64 + span as i64;
            if idx < 0 {
                return Err(LzError("bad back reference".into()));
            }
            let b = out[idx as usize];
            out.push(b);
        }
    }
    Ok(out)
}
