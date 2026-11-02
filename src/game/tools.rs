//! Helpers for tools like the Commander Keen Editor that need the game's
//! own logic.

use super::consts::STARTSPRITES;
use super::Game;
use crate::assets::GameData;
use crate::engine::{gfx::Gfx, sound::SoundManager, sys::Sys};

/// Runs ScanInfoPlane on a tiny map for every info value in `values` and
/// records the sprite the spawned object starts with (on hard, so
/// everything spawns): (value, sprite index, x and y offset of the
/// sprite's top left corner from the tile in pixels).
pub fn sprite_previews(data: GameData, values: std::ops::RangeInclusive<u16>) -> Vec<(u16, usize, i32, i32)> {
    let mut out = Vec::new();
    let table = data.gfx.spritetable.clone();
    let gfx = Gfx::new(&data.gfx, 1, None);
    let sd = SoundManager::new(&data.audio, false, None, None);
    let sys = Sys::new(1, 1, true, true, false);
    let mut g = Game::new(sys, data, gfx, sd, Default::default());
    let hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(|_| {}));
    const W: usize = 7;
    for v in values {
        let mut planes: [Vec<u16>; 3] = [vec![0; W * W], vec![0; W * W], vec![0; W * W]];
        planes[2][3 * W + 3] = v;
        g.mapwidth = W as i32;
        g.mapheight = W as i32;
        g.mapplanes = planes;
        g.gs.difficulty = 3;
        g.gs.ammo = 0;
        if std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| g.scan_info_plane())).is_err() {
            continue;
        }
        let mut i = Some(g.player);
        while let Some(n) = i {
            let o = &g.objs[n];
            let idx = o.shapenum - STARTSPRITES as i32;
            if n != g.scoreobj && o.state.is_some() && idx >= 0 && (idx as usize) < table.len() {
                let info = table[idx as usize];
                out.push((v, idx as usize, (o.x - 3 * 256 + info.orgx as i32) >> 4, (o.y - 3 * 256 + info.orgy as i32) >> 4));
                break;
            }
            i = o.next;
        }
    }
    std::panic::set_hook(hook);
    out
}
