//! Platform/engine layer (the ID_* managers).

pub mod gfx;
pub mod input;
#[cfg(target_os = "macos")]
pub mod procon;
pub mod sound;
pub mod sys;
pub mod video;
