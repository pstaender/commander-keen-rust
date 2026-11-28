//! Platform/engine layer (the ID_* managers).

pub mod gfx;
pub mod input;
#[cfg(target_os = "macos")]
pub mod procon;
pub mod sound;
#[cfg(not(target_arch = "wasm32"))]
pub mod sys;
/// in the browser: a web worker sharing memory with the page
#[cfg(target_arch = "wasm32")]
#[path = "sys_web.rs"]
pub mod sys;
pub mod video;
