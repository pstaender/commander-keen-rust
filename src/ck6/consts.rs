//! Keen 6 constants: EGAGRAPH chunk numbers (GFXE_CK6.H equivalent).

const L: &crate::assets::Layout = super::LAYOUT;
pub const NUMCHUNKS: usize = L.numchunks;
pub const NUMPICS: usize = L.numpics;
pub const NUMSPRITES: usize = L.numsprites;
pub const NUMTILE16: usize = L.numtile16;
pub const NUMTILE16M: usize = L.numtile16m;

pub const STARTPICS: usize = L.startpics();
pub const STARTPICM: usize = L.startpicm();
pub const STARTSPRITES: usize = L.startsprites();
pub const STARTTILE16: usize = L.starttile16();
pub const STARTTILE16M: usize = L.starttile16m();
pub const STARTEXTERNS: usize = L.startexterns();

// Externs (texts, screens, demos), as cached by the executable
pub const T_ENDART: usize = STARTEXTERNS; // 5550
pub const ORDERSCREEN: usize = 5551;
pub const BIGCOMMANDER: usize = 5552;
pub const BIGKEEN: usize = 5553;
pub const OUTOFMEM: usize = 5554;
pub const DEMO0: usize = 5555;
pub const DEMO1: usize = 5556;
pub const DEMO2: usize = 5557;
pub const DEMO3: usize = 5558;
pub const DEMO4: usize = 5559;
