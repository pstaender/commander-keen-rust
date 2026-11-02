//! Commander Keen 4, 5 and 6 ("Goodbye Galaxy" engine) reimplementation in Rust.

pub mod assets;
pub mod dump;
pub mod cli;
pub mod engine;
pub mod options;

pub mod ck4;
pub mod ck5;
pub mod ck6;

/// Info plane sprite previews for the editor, see `ckN::tools`.
pub fn sprite_previews(data: assets::GameData, values: std::ops::RangeInclusive<u16>) -> Vec<(u16, usize, i32, i32)> {
    match data.episode {
        assets::Episode::K4 => ck4::tools::sprite_previews(data, values),
        assets::Episode::K5 => ck5::tools::sprite_previews(data, values),
        assets::Episode::K6 => ck6::tools::sprite_previews(data, values),
    }
}
