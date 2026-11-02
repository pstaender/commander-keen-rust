//! Commander Keen 4: Secret of the Oracle (KEEN4E.EXE v1.4).
//! The shared CK_* modules from src/game/ plus the CK4 specific ones.

pub const EPISODE: i32 = 4;
pub const LAYOUT: &crate::assets::Layout = &crate::assets::episode::K4;

// Code that only exists in some episodes (#if GAMEVER_... in the original).
#[allow(unused_macros)]
macro_rules! ep4 { ($($t:tt)*) => { $($t)* } }
#[allow(unused_macros)]
macro_rules! ep5 { ($($t:tt)*) => {} }
#[allow(unused_macros)]
macro_rules! ep6 { ($($t:tt)*) => {} }
/// `ep_sel!(k4, k5, k6)`: the expression for this episode
#[allow(unused_macros)]
macro_rules! ep_sel { ($k4:expr, $k5:expr, $k6:expr) => { $k4 } }

pub mod consts;
pub mod ep;
pub mod states;
pub mod spec;
pub mod spec_tiles;
pub mod act1;
pub mod act2;
pub mod act3;

include!("../game/modules.rs");
include!("../game/root.rs");
