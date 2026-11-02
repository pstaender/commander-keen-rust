//! Keen 5 constants: EGAGRAPH chunk numbers (GFXE_CK5.H equivalent).

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
pub const T_HELPART: usize = STARTEXTERNS;
pub const T_CONTRART: usize = STARTEXTERNS + 1;
pub const T_STORYART: usize = STARTEXTERNS + 2;
pub const T_IDART: usize = STARTEXTERNS + 3;
pub const T_ENDART: usize = STARTEXTERNS + 4;
pub const T_SECRETART: usize = STARTEXTERNS + 5;
pub const T_ORDERART: usize = STARTEXTERNS + 6;
pub const ORDERSCREEN: usize = STARTEXTERNS + 7;
pub const BIGCOMMANDER: usize = STARTEXTERNS + 8;
pub const BIGKEEN: usize = STARTEXTERNS + 9;
pub const OUTOFMEM: usize = STARTEXTERNS + 10;
pub const GALAXY: usize = STARTEXTERNS + 11;
pub const DEMO0: usize = STARTEXTERNS + 12;
