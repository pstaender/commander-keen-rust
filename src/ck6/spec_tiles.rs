//! HitSpecialTile (0x1dff1, segment 0x1da0): switch blocks that Keen hits
//! from below or lands on (foreground tiles with wall value 0x21).
//!
//! The switch itself is a 2x3 tile group; the info plane holds a pointer
//! `(x << 8) | y` to the target one tile right of / below the switch's
//! top-left corner. The target is either a platform arrow in the info plane
//! (rotated via the table at DS:0x1824), or a foreground tile whose INTILE
//! value selects the action (toggle info value, animate a row of tiles, or
//! switch a vertical column of tiles).

use super::Game;

/// SD_PlaySound number used when a switch is hit.
const SND_SWITCH: i32 = 0x15;

/// DS:0x1824: new arrow direction (minus 0x5b) for each platform arrow
/// info value 0x5b..0x62 (reverses the direction).
const ARROWFLIP: [u16; 8] = [2, 3, 0, 1, 6, 7, 4, 5];

/// Raw word of map plane `plane` at linear index `i` (like the far pointer
/// arithmetic of the original; 0 outside the plane).
fn plane_at(g: &Game, plane: usize, i: i32) -> u16 {
    if i < 0 {
        return 0;
    }
    g.mapplanes[plane].get(i as usize).copied().unwrap_or(0)
}

fn manim(g: &Game, t: u16) -> i8 {
    g.data.tileinfo.manim.get(t as usize).copied().unwrap_or(0)
}

fn intile(g: &Game, t: u16) -> u8 {
    g.data.tileinfo.intile.get(t as usize).copied().unwrap_or(0)
}

/// HitSpecialTile (0x1dff1)
pub fn hit_special_tile(g: &mut Game, ob: usize, top: bool) {
    let w = g.mapwidth;
    let y = if top { g.objs[ob].tilebottom } else { g.objs[ob].tiletop - 2 };
    let mut x = g.objs[ob].tileleft - 1;

    // find the switch: the info value sits at (x+1, y+1)
    let mut i = (y + 1) * w + x + 1;
    while plane_at(g, 2, i) == 0 {
        if i < 0 || i as usize >= g.mapplanes[2].len() {
            return; // the original would run off the plane here
        }
        i += 1;
        x += 1;
    }

    // animate the 2x3 switch tiles
    let mut tiles = [0u16; 6];
    let mut p = y * w + x;
    let mut n = 0;
    for _ in 0..3 {
        for c in 0..2 {
            let t = plane_at(g, 1, p + c);
            tiles[n] = t.wrapping_add(manim(g, t) as i16 as u16);
            n += 1;
        }
        p += w;
    }
    g.rf_mem_to_map(&tiles, 1, x, y, 2, 3);

    let info = plane_at(g, 2, (y + 1) * w + x + 1);
    let tx = (info >> 8) as i32;
    let ty = (info & 0xff) as i32;
    g.play_sound(SND_SWITCH);

    let target = ty * w + tx;
    let v = plane_at(g, 2, target);
    if (0x5b..0x63).contains(&v) {
        // platform arrow: reverse it
        if target >= 0 && (target as usize) < g.mapplanes[2].len() {
            g.mapplanes[2][target as usize] = ARROWFLIP[(v - 0x5b) as usize] + 0x5b;
        }
        return;
    }

    let t = plane_at(g, 1, target);
    match intile(g, t) & 0x7f {
        0 => {
            // toggle the info value (e.g. platform stop/go)
            if target >= 0 && (target as usize) < g.mapplanes[2].len() {
                g.mapplanes[2][target as usize] ^= 0x1f;
            }
        }
        0x12 => {
            // animate two rows of tiles (the second one starting one tile left)
            for row in ty..ty + 2 {
                let back = (row != ty) as i32;
                let mut p = row * w + tx - back;
                let mut col = tx - back;
                while (col as u16) < (w as u16) {
                    let t = plane_at(g, 1, p);
                    p += 1;
                    let m = manim(g, t);
                    if m == 0 {
                        break;
                    }
                    let nt = t.wrapping_add(m as i16 as u16);
                    g.rf_mem_to_map(&[nt], 1, col, row, 1, 1);
                    col += 1;
                }
            }
        }
        0x13 => {
            // vertical tile column (door/bridge) on: tiles 0, 1, 2 of the
            // foreground plane hold the start, middle and end tiles; the
            // middle tile replaces every tile below with INTILE 3
            let (a, b, c) = (plane_at(g, 1, 0), plane_at(g, 1, 1), plane_at(g, 1, 2));
            bridge(g, tx, ty, a, b, c, |it| it == 3);
        }
        0x1e => {
            // the reverse: tiles 3, 4, 5 of the foreground plane, the middle
            // one written down to the tile with INTILE 0x1e
            let (a, b, c) = (plane_at(g, 1, 3), plane_at(g, 1, 4), plane_at(g, 1, 5));
            bridge(g, tx, ty, a, b, c, |it| it != 0x1e);
        }
        _ => {}
    }
}

/// The bridge loop shared by the INTILE 0x13 and 0x1e cases of
/// HitSpecialTile: write `a` at (tx, ty), then `b` going down while
/// `cont(intile)` holds for the tile below, then `c`.
fn bridge(g: &mut Game, tx: i32, ty: i32, a: u16, b: u16, c: u16, cont: impl Fn(u8) -> bool) {
    let w = g.mapwidth;
    let mut y = ty;
    let mut p = (ty + 1) * w + tx;
    g.rf_mem_to_map(&[a], 1, tx, y, 1, 1);
    y += 1;
    loop {
        let t = plane_at(g, 1, p);
        if !cont(intile(g, t)) {
            break;
        }
        if p < 0 || p as usize >= g.mapplanes[1].len() {
            break; // guard against running off the plane
        }
        g.rf_mem_to_map(&[b], 1, tx, y, 1, 1);
        y += 1;
        p += w;
    }
    g.rf_mem_to_map(&[c], 1, tx, y, 1, 1);
}

/// Module-private state stored in `Game::spec_tiles` (none needed).
#[derive(Default, Clone)]
pub struct ModState {}
