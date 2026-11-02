//! CK5 spec_tiles: Keen 5 has no HitSpecialTile (see spec.rs); this module
//! only keeps Keen 5 game state that GameState does not have yet.

#[allow(unused_imports)]
use super::*;

#[derive(Default)]
pub struct ModState {
    /// gamestate.numfuses (DS:0x6f5e): fuses left on the current map, set
    /// by ScanInfoPlane (info value 41). Belongs into GameState.
    pub numfuses: i32,
}
