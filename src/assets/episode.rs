//! The three Keen "Goodbye Galaxy" episodes share one engine; this holds
//! what differs in their data files: file names, the EGAGRAPH and AUDIO
//! chunk layouts (GFXE_CKn.H / AUDIOCKn.H equivalents) and where the data
//! headers are linked into the executables (found with REA, see
//! claude_history.log).

#[derive(Clone, Copy, PartialEq, Eq, Debug, Hash)]
pub enum Episode {
    K4,
    K5,
    K6,
}

pub struct Layout {
    pub episode: Episode,
    pub number: i32,
    pub title: &'static str,
    /// data file extension, e.g. "CK6"
    pub ext: &'static str,
    /// executable names, the first one is the version this port follows
    pub exes: &'static [&'static str],
    // EGAGRAPH
    pub numchunks: usize,
    pub numfont: usize,
    pub numpics: usize,
    pub numpicm: usize,
    pub numsprites: usize,
    pub numtile8: usize,
    pub numtile8m: usize,
    pub numtile16: usize,
    pub numtile16m: usize,
    pub numexterns: usize,
    // AUDIO
    pub numsounds: usize,
    pub numsongs: usize,
    // image offsets (linear address - 0x10000) of the linked data headers
    pub audiohed_ofs: usize,
    pub egahead_ofs: usize,
    pub maphead_ofs: usize,
    pub audiodict_ofs: usize,
    pub egadict_ofs: usize,
}

impl Layout {
    pub const fn startfont(&self) -> usize {
        3
    }
    pub const fn startpics(&self) -> usize {
        3 + self.numfont
    }
    pub const fn startpicm(&self) -> usize {
        self.startpics() + self.numpics
    }
    pub const fn startsprites(&self) -> usize {
        self.startpicm() + self.numpicm
    }
    pub const fn starttile8(&self) -> usize {
        self.startsprites() + self.numsprites
    }
    pub const fn starttile8m(&self) -> usize {
        self.starttile8() + 1
    }
    pub const fn starttile16(&self) -> usize {
        self.starttile8m() + 1
    }
    pub const fn starttile16m(&self) -> usize {
        self.starttile16() + self.numtile16
    }
    pub const fn startexterns(&self) -> usize {
        self.starttile16m() + self.numtile16m
    }
    pub const fn startadlibsounds(&self) -> usize {
        self.numsounds
    }
    pub const fn startmusic(&self) -> usize {
        self.numsounds * 3
    }
    pub const fn numaudiochunks(&self) -> usize {
        self.startmusic() + self.numsongs
    }
    /// MAPHEAD: RLEW tag, 100 map offsets, then the tile info arrays
    pub const fn maphead_len(&self) -> usize {
        402 + 2 * self.numtile16 + 7 * self.numtile16m
    }
    /// file name with the episode's extension, e.g. "GAMEMAPS" -> "GAMEMAPS.CK6"
    pub fn file(&self, base: &str) -> String {
        format!("{}.{}", base, self.ext)
    }
}

/// Keen 4: Secret of the Oracle, KEEN4E.EXE v1.4
pub const K4: Layout = Layout {
    episode: Episode::K4,
    number: 4,
    title: "Commander Keen 4: Secret of the Oracle",
    ext: "CK4",
    exes: &["KEEN4E.EXE", "KEEN4.EXE"],
    numchunks: 4743,
    numfont: 3,
    numpics: 107,
    numpicm: 3,
    numsprites: 397,
    numtile8: 108,
    numtile8m: 36,
    numtile16: 1296,
    numtile16m: 2916,
    numexterns: 16,
    numsounds: 52,
    numsongs: 6,
    audiohed_ofs: 0x1fb80,
    egahead_ofs: 0x1fe10,
    maphead_ofs: 0x235b0,
    audiodict_ofs: 0x33ff6,
    egadict_ofs: 0x343f6,
};

/// Keen 5: The Armageddon Machine, KEEN5E.EXE v1.4
pub const K5: Layout = Layout {
    episode: Episode::K5,
    number: 5,
    title: "Commander Keen 5: The Armageddon Machine",
    ext: "CK5",
    exes: &["KEEN5E.EXE", "KEEN5.EXE"],
    numchunks: 4931,
    numfont: 3,
    numpics: 93,
    numpicm: 3,
    numsprites: 346,
    numtile8: 108,
    numtile8m: 36,
    numtile16: 1512,
    numtile16m: 2952,
    numexterns: 17,
    numsounds: 64,
    numsongs: 14,
    audiohed_ofs: 0x21c84,
    egahead_ofs: 0x21fc0,
    maphead_ofs: 0x25990,
    audiodict_ofs: 0x35ec4,
    egadict_ofs: 0x362c4,
};

/// Keen 6: Aliens Ate My Baby Sitter!, KEEN6C.EXE v1.4
pub const K6: Layout = Layout {
    episode: Episode::K6,
    number: 6,
    title: "Commander Keen 6: Aliens Ate My Baby Sitter!",
    ext: "CK6",
    exes: &["KEEN6C.EXE", "KEEN6.EXE", "KEEN6E.EXE"],
    numchunks: 5560,
    numfont: 3,
    numpics: 37,
    numpicm: 3,
    numsprites: 390,
    numtile8: 104,
    numtile8m: 20,
    numtile16: 2376,
    numtile16m: 2736,
    numexterns: 10,
    numsounds: 60,
    numsongs: 9,
    audiohed_ofs: 0x1f820,
    egahead_ofs: 0x1fb20,
    maphead_ofs: 0x23c50,
    audiodict_ofs: 0x35912,
    egadict_ofs: 0x35d12,
};

impl Episode {
    pub const ALL: [Episode; 3] = [Episode::K4, Episode::K5, Episode::K6];

    pub fn layout(self) -> &'static Layout {
        match self {
            Episode::K4 => &K4,
            Episode::K5 => &K5,
            Episode::K6 => &K6,
        }
    }

    pub fn number(self) -> i32 {
        self.layout().number
    }

    pub fn from_number(n: i32) -> Option<Episode> {
        Episode::ALL.into_iter().find(|e| e.number() == n)
    }

    /// The episode whose data files are in `dir` (by the EGAGRAPH.CKn file).
    pub fn detect(dir: &std::path::Path) -> Option<Episode> {
        Episode::ALL.into_iter().rev().find(|e| super::find_file(dir, &e.layout().file("EGAGRAPH")).is_some())
    }
}
