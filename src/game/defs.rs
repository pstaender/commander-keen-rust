//! Core definitions (CK_DEF.H equivalent): objects, states, constants.

use super::Game;

pub const MAXACTORS: usize = 100;
/// index of the dummy object returned by GetNewObj(true) when full
pub const DUMMYOBJ: usize = MAXACTORS;

pub const GLOBAL1: i32 = 1 << 16;
pub const TILEGLOBAL: i32 = 256;
pub const PIXGLOBAL: i32 = 16;
pub const G_T_SHIFT: i32 = 8;
pub const G_P_SHIFT: i32 = 4;
pub const P_T_SHIFT: i32 = 4;

pub const PORTTILESWIDE: i32 = 21;
pub const PORTTILESHIGH: i32 = 14;

// object classes: per episode, see src/ckN/ep.rs
pub use super::ep::obclass::*;

// active
pub const AC_NO: i32 = 0;
pub const AC_YES: i32 = 1;
pub const AC_ALLWAYS: i32 = 2;
pub const AC_REMOVABLE: i32 = 3;

// clipping
pub const CL_NOCLIP: i32 = 0;
pub const CL_MIDCLIP: i32 = 1;
pub const CL_FULLCLIP: i32 = 2;

// exit types (playstate)
pub const EX_STILLPLAYING: i32 = 0;
pub const EX_DIED: i32 = 1;
pub const EX_COMPLETED: i32 = 2;
pub const EX_RESCUED: i32 = 3;
pub const EX_WARPED: i32 = 4;
pub const EX_RESETGAME: i32 = 5;
pub const EX_LOADEDGAME: i32 = 6;
pub const EX_FOOT: i32 = 7;
pub const EX_ABORTGAME: i32 = 8;
pub const EX_SANDWICH: i32 = 9;
pub const EX_HOOK: i32 = 10;
pub const EX_CARD: i32 = 11;
pub const EX_MOLLY: i32 = 12;
/// Keen 5: Keen teleports on the world map
pub const EX_PORTOUT: i32 = 13;
/// Keen 5: a fuse of the Armageddon Machine was destroyed
pub const EX_FUSEBROKE: i32 = 14;
/// Keen 5: the Quantum Explosion Dynamo was destroyed (map 12)
pub const EX_QEDBROKE: i32 = 15;

// difficulty
pub const GD_EASY: i32 = 1;
pub const GD_NORMAL: i32 = 2;
pub const GD_HARD: i32 = 3;

pub const GAMELEVELS: usize = 25;

/// Original `dir_*` values used by ControlInfo.dir.
pub const DIR_NONE: i32 = 8;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Progress {
    Step,
    Slide,
    Think,
    StepThink,
    SlideThink,
}

pub type ThinkFn = fn(&mut Game, usize);
pub type ContactFn = fn(&mut Game, usize, usize);

pub struct State {
    pub name: &'static str,
    pub left: i32,
    pub right: i32,
    pub progress: Progress,
    pub skippable: bool,
    pub pushtofloor: bool,
    pub tictime: i32,
    pub xmove: i32,
    pub ymove: i32,
    pub think: Option<ThinkFn>,
    pub contact: Option<ContactFn>,
    pub react: Option<ThinkFn>,
    pub next: Option<&'static State>,
}

pub type StateRef = &'static State;

#[inline]
pub fn same(a: Option<StateRef>, b: StateRef) -> bool {
    match a {
        Some(s) => std::ptr::eq(s, b),
        None => false,
    }
}

#[derive(Clone, Default)]
pub struct Obj {
    pub obclass: i32,
    pub active: i32,
    pub needtoreact: bool,
    pub needtoclip: i32,
    pub nothink: i32,
    pub x: i32,
    pub y: i32,
    pub xdir: i32,
    pub ydir: i32,
    pub xmove: i32,
    pub ymove: i32,
    pub xspeed: i32,
    pub yspeed: i32,
    pub ticcount: i32,
    pub state: Option<StateRef>,
    pub shapenum: i32,
    pub priority: i32,
    pub left: i32,
    pub top: i32,
    pub right: i32,
    pub bottom: i32,
    pub midx: i32,
    pub tileleft: i32,
    pub tiletop: i32,
    pub tileright: i32,
    pub tilebottom: i32,
    pub tilemidx: i32,
    pub hitnorth: i32,
    pub hiteast: i32,
    pub hitsouth: i32,
    pub hitwest: i32,
    pub temp1: i32,
    pub temp2: i32,
    pub temp3: i32,
    pub temp4: i32,
    /// sprite is visible (RF sprite allocated)
    pub sprite: bool,
    /// for stunned objects: the stars sprite (shape) and visibility
    pub next: Option<usize>,
    pub prev: Option<usize>,
}

impl Obj {
    pub fn st(&self) -> StateRef {
        self.state.expect("object without state")
    }
}

/// gametype (Keen 6)
#[derive(Clone, Default)]
pub struct GameState {
    pub worldx: i32,
    pub worldy: i32,
    pub leveldone: [i32; GAMELEVELS],
    pub score: i32,
    pub nextextra: i32,
    pub ammo: i32,
    pub drops: i32,
    pub sandwichstate: i32,
    pub hookstate: i32,
    pub passcardstate: i32,
    pub rocketstate: i32,
    /// Keen 4: wetsuit (DS 0x77db) and rescued council members (0x77dd)
    pub wetsuit: i32,
    pub rescued: i32,
    /// Keen 5: keycard (DS 0x6f5a), destroyed fuses (0x6f5c), fuses on
    /// the level (0x6f5e)
    pub keycard: i32,
    pub destroyed: i32,
    pub numfuses: i32,
    pub keys: [i32; 4],
    pub mapon: i32,
    pub lives: i32,
    pub difficulty: i32,
    pub riding: Option<usize>,
}
