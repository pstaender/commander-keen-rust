// Root of an episode module (included by src/ckN/mod.rs): the Game struct
// holding the original globals. The CK_* modules in src/game/ are compiled
// once per episode, like the original source with its #ifdefs.

use crate::assets::GameData;
use crate::engine::gfx::Gfx;
use crate::engine::input::ControlInfo;
use crate::engine::sound::SoundManager;
use crate::engine::sys::Sys;
use defs::*;
use phys::PhysTemps;
use rf::{Camera, RfState};

pub use crate::options::Options;

pub struct Game {
    pub sys: Sys,
    pub data: GameData,
    pub gfx: Gfx,
    pub sd: SoundManager,
    pub opts: Options,
    pub request_quit: bool,

    // objects
    pub objs: Vec<Obj>,
    pub obj_free: Vec<bool>,
    pub objfreelist: Option<usize>,
    pub lastobj: Option<usize>,
    pub objectcount: i32,
    pub player: usize,
    pub scoreobj: usize,
    pub newobj: usize,
    pub rndindex: usize,

    // physics temporaries (xtry, ytry, old*, *moved)
    pub ph: PhysTemps,

    // game state
    pub gs: GameState,
    pub playstate: i32,
    pub tics: i32,
    pub lasttimecount: i32,
    pub demomode: i32,
    pub loadedgame: bool,
    pub abortgame: bool,
    pub restartgame: bool,
    pub godmode: bool,
    pub jumpcheat: bool,
    pub invincible: i32,
    pub scrolllock: bool,
    pub centerlevel: i32,
    pub extravbls: i32,
    pub slowmotion: bool,
    pub showscorebox: bool,
    pub twobutton: bool,
    pub screenfaded: bool,
    pub inlevel: bool,

    // controls
    pub c: ControlInfo,
    pub jumpbutton: bool,
    pub jumpheld: bool,
    pub pogobutton: bool,
    pub pogoheld: bool,
    pub firebutton: bool,
    pub fireheld: bool,
    pub upheld: bool,
    pub jumptime: i32,
    pub pogodelay: i32,

    // map
    pub mapplanes: [Vec<u16>; 3],
    pub mapwidth: i32,
    pub mapheight: i32,
    pub mapname: String,
    /// ID_CA mapon (0xa7ae): the map currently cached, -1 if none
    pub ca_mapon: i32,
    /// in the game loop: the play field may use a wide view
    pub wide_ok: bool,

    pub cam: Camera,
    pub rf: RfState,

    // misc globals
    pub lastpoletime: i64,
    /// moonok: Keen touches tile type 16 (Keen 4: 2 = mooned)
    pub moonok: i32,
    pub storedemo: bool,
    pub crand: u32,
    pub flagtarget: (i32, i32),
    pub flagpath: [(i32, i32); 30],
    pub firescan: u8,
    pub scrolllock_rf: bool,
    pub shaketime: i32,
    pub scroll_target_last: i32,
    pub tw: us::TextWin,
    pub lastlevel: i32,
    pub restartdifficulty: i32,
    pub demo: demo::DemoState,

    // per-module state of the ported CK6 modules
    pub act1: act1::ModState,
    pub act2: act2::ModState,
    pub act3: act3::ModState,
    pub text: text::ModState,
    pub menu: menu::ModState,
    pub spec_tiles: spec_tiles::ModState,
}

impl Game {
    pub fn new(sys: Sys, data: GameData, gfx: Gfx, sd: SoundManager, opts: Options) -> Game {
        let mut g = Game {
            sys,
            data,
            gfx,
            sd,
            opts,
            request_quit: false,
            objs: vec![Obj::default(); MAXACTORS + 1],
            obj_free: vec![true; MAXACTORS + 1],
            objfreelist: None,
            lastobj: None,
            objectcount: 0,
            player: 0,
            scoreobj: 1,
            newobj: 0,
            rndindex: 0,
            ph: PhysTemps::default(),
            gs: GameState::default(),
            playstate: 0,
            tics: 1,
            lasttimecount: 0,
            demomode: 0,
            loadedgame: false,
            abortgame: false,
            restartgame: false,
            godmode: false,
            jumpcheat: false,
            invincible: 0,
            scrolllock: false,
            centerlevel: 0x8c,
            extravbls: 0,
            slowmotion: false,
            showscorebox: true,
            twobutton: false,
            screenfaded: false,
            inlevel: false,
            c: ControlInfo::default(),
            jumpbutton: false,
            jumpheld: false,
            pogobutton: false,
            pogoheld: false,
            firebutton: false,
            fireheld: false,
            upheld: false,
            jumptime: 0,
            pogodelay: 0,
            mapplanes: Default::default(),
            mapwidth: 0,
            mapheight: 0,
            mapname: String::new(),
            ca_mapon: -1,
            wide_ok: false,
            cam: Camera::default(),
            rf: RfState::default(),
            lastpoletime: 0,
            moonok: 0,
            storedemo: false,
            crand: 1,
            flagtarget: (0, 0),
            flagpath: [(0, 0); 30],
            firescan: crate::engine::input::SC_SPACE,
            scrolllock_rf: false,
            shaketime: 0,
            scroll_target_last: 0,
            tw: us::TextWin { fontcolor: 15, ..Default::default() },
            lastlevel: -1,
            restartdifficulty: 0,
            demo: demo::DemoState::default(),
            act1: Default::default(),
            act2: Default::default(),
            act3: Default::default(),
            text: Default::default(),
            menu: Default::default(),
            spec_tiles: Default::default(),
        };
        g.init_obj_array();
        g
    }

    pub fn alive(&self) -> bool {
        !self.sys.quit && !self.request_quit
    }

    /// SD_PlaySound shortcut
    pub fn play_sound(&mut self, n: i32) {
        if n >= 0 {
            self.sd.play_sound(n as usize);
        }
    }
}

/// Runs the game (attract loop, or the demo given in the options) until
/// the player quits.
pub fn run(sys: Sys, data: GameData, gfx: Gfx, sd: SoundManager, opts: Options) {
    let mut game = Game::new(sys, data, gfx, sd, opts);
    game.load_config();
    if let Some(n) = game.opts.play_demo {
        game.run_demo(n);
        return;
    }
    if let Some(p) = game.opts.demo_file.clone() {
        match std::fs::read(&p) {
            Ok(d) => game.run_demo_data(&d),
            Err(e) => eprintln!("{}: {}", p.display(), e),
        }
        return;
    }
    game.demo_loop();
    game.save_config();
}
