//! ID_US_2.C port: the control panel of KEEN6C.EXE (code segment 0x2946,
//! linear 0x29460..0x2ba6b), the parts of ID_US_1 it needs (US_LineInput),
//! DebugKeys from CK_PLAY (0x16e8f) and the copy protection question
//! CheckCreature (0x2b7ca, Keen 6 only).
//!
//! Keen 4 (K4 0x2997f..) and Keen 5 (K5 0x2b833..) use the same code with
//! other picture/sprite/sound numbers; F1 opens the help screens; Keen 5
//! has the "FIX JERKY MOTION" option and a (disabled) gravis gamepad entry.
//!
//! The menu "cards" are the static UserItemGroup/UserItem tables of the
//! original (far data segments 0x2e22..0x2e85), rebuilt once in `ModState`.
//! Saved games use our own format (the original one stored raw near
//! pointers), config persistence uses a text file `config.txt`.

use super::defs::*;
use super::states::ALL_STATES;
use super::Game;
use super::consts::{NUMSPRITES, NUMTILE16M, STARTSPRITES};
use crate::engine::input::*;
use crate::engine::sound::{SdMode, SmMode};
use std::path::PathBuf;

// UserCall
const UIC_SETUPCARD: i32 = 0;
const UIC_DRAWCARD: i32 = 1;
const UIC_TOUCHUPCARD: i32 = 2;
const UIC_DRAWICON: i32 = 3;
const UIC_DRAW: i32 = 4;
const UIC_HIT: i32 = 5;

// UIType
const UII_BUTTON: u8 = 1;
const UII_RADIOBUTTON: u8 = 2;
const UII_FOLDER: u8 = 3;

// UIFlags
const UI_PUSHED: u16 = 1;
const UI_SELECTED: u16 = 2;
const UI_DISABLED: u16 = 4;
const UI_SEPARATED: u16 = 8;

// UComm
const UC_ABORT: i32 = 2;
const UC_QUIT: i32 = 3;
const UC_LOADED: i32 = 4;
const UC_SEASY: i32 = 5;
const UC_SHARD: i32 = 7;

// graphics chunks
const CP_PADDLEWARPIC: i32 = ep_sel!(0x5c, 0x4f, 0x17);
const CP_MENUSCREENPIC: i32 = ep_sel!(0x5f, 0x52, 0x1a);
const CP_MENUMASKPICM: i32 = ep_sel!(0x71, 0x63, 0x2b);
const PADDLESPR: i32 = ep_sel!(0x74, 0x66, 0x2e);
const BALLSPR: i32 = ep_sel!(0x75, 0x67, 0x2f);
const BALL1PIXELTOTHERIGHTSPR: i32 = ep_sel!(0x76, 0x68, 0x30);
/// card title pictures: Keen 6 CP_MAINMENUPIC = 0xb .. 0x16; the other
/// episodes have the same pictures at another offset
const CP_TITLEOFS: i32 = ep_sel!(0x45, 0x38, 0);
/// first tile8 of the item icons
const CP_ICONTILE: i32 = 0x5c;
const CP_KEYCURSORTILE: i32 = 100;

// sounds
const NOWAYSND: i32 = 14;
const KEENPADDLESND: i32 = ep_sel!(0x2f, 0x2e, 0x2e);
const BALLBOUNCESND: i32 = ep_sel!(0x30, 0x2f, 0x2f);
const COMPPADDLESND: i32 = ep_sel!(0x31, 0x30, 0x30);
const COMPSCOREDSND: i32 = ep_sel!(0x32, 0x31, 0x31);
const KEENSCOREDSND: i32 = ep_sel!(0x33, 0x32, 0x32);

const BACKCOLOR: u8 = 8;
const TICKBASE: i32 = 70;
/// cstackptr may go up to 6
const MAXCARDS: usize = 7;
const MAXSTRING: usize = 128;
const MAXGAMENAME: usize = 32;
const MAXSAVEGAMES: usize = 6;

// card numbers (the UserItemGroups of the original)
const ROOTCARD: usize = 0;
const NEWGAMECARD: usize = 1;
const LOADCARD: usize = 2;
const SAVECARD: usize = 3;
const CONFIGCARD: usize = 4;
const SOUNDCARD: usize = 5;
const MUSICCARD: usize = 6;
const OPTIONSCARD: usize = 7;
const SCOREBOXCARD: usize = 8;
const COMPATCARD: usize = 9;
const TWOBUTTONCARD: usize = 10;
const KEYBOARDCARD: usize = 11;
const MOVEMENTCARD: usize = 12;
const BUTTONSCARD: usize = 13;
const JOY1CARD: usize = 14;
const JOY2CARD: usize = 15;
const PADDLECARD: usize = 16;
/// Keen 5 only: the "USE GRAVIS GAMEPAD" folder of the config card
const GAMEPADCARD: usize = 17;
/// Keen 5 only: the "FIX JERKY MOTION" option
const JERKCARD: usize = 18;
/// --cheatmode: the "CHEAT OPTIONS" card (replaces END GAME)
const CHEATCARD: usize = 19;
/// most lives / shots the cheat card adds (the score box shows two digits)
const CHEATMAX: i32 = 99;

/// DS:0x4128 control type names
const CTLNAMES: [&str; 5] = ["KEYBOARD", "KEYBOARD", "JOYSTICK #1", "JOYSTICK #2", "MOUSE"];

/// DS:0x4132 creatures of the copy protection question:
/// (name, sprite chunk, x offset in tile8s, y offset in tile8s)
const CREATURES: [(&str, i32, i32, i32); 12] = [
    ("BIP", 269, -2, 0),
    ("BABOBBA", 288, 0, 0),
    ("BLORB", 399, -2, 0),
    ("GIK", 387, -1, 0),
    ("CEILICK", 246, 0, 0),
    ("BLOOGLET", 351, -2, 0),
    ("BLOOGUARD", 254, -3, -1),
    ("FLECT", 317, -1, 0),
    ("BOBBA", 405, -2, 0),
    ("NOSPIKE", 298, -2, 0),
    ("ORBATRIX", 335, -2, 1),
    ("FLEEX", 239, -2, 0),
];
const CEILICKSPR: i32 = 246;

const SAVEMAGIC: &[u8; 8] = ep_sel!(b"CK4RSAV1", b"CK5RSAV1", b"CK6RSAV1");

#[derive(Clone, Copy, PartialEq, Eq)]
enum Custom {
    None,
    Load,
    Save,
    Config,
    ScoreBox,
    Compat,
    TwoButton,
    Keyboard,
    KeySet,
    Joystick,
    Paddle,
    /// Keen 5 USL_JerkCustom (K5 0x2c18d)
    Jerk,
    /// Keen 5 gamepad card (never reachable: no gamepad support)
    Gamepad,
    /// --cheatmode card (not in the original)
    Cheat,
}

#[derive(Clone)]
struct Item {
    ty: u8,
    flags: u16,
    hotkey: u8,
    text: &'static str,
    comm: i32,
    child: Option<usize>,
    x: i32,
    y: i32,
}

fn item(ty: u8, flags: u16, hotkey: u8, text: &'static str, comm: i32, child: Option<usize>) -> Item {
    Item { ty, flags, hotkey, text, comm, child, x: 0, y: 0 }
}

#[derive(Clone)]
struct Card {
    x: i32,
    y: i32,
    title: i32,
    items: Vec<Item>,
    /// the card uses the item list of another card (load/save share theirs)
    shared: Option<usize>,
    custom: Custom,
    cursor: usize,
}

fn card(x: i32, y: i32, title: i32, items: Vec<Item>, custom: Custom, cursor: usize) -> Card {
    Card { x, y, title, items, shared: None, custom, cursor }
}

/// The static card tables of the executable (0x2e22..0x2e85).
fn build_cards() -> Vec<Card> {
    let b = |hot, text, comm| item(UII_BUTTON, 0, hot, text, comm, None);
    let r = |hot, text| item(UII_RADIOBUTTON, 0, hot, text, 0, None);
    let f = |hot, text, child| item(UII_FOLDER, 0, hot, text, 0, Some(child));
    let mut cards = vec![
        // ROOTCARD 2e85
        card(
            32,
            4,
            0xb,
            vec![
                f(0x31, "NEW GAME", NEWGAMECARD),
                f(0x26, "LOAD GAME", LOADCARD),
                f(0x1f, "SAVE GAME", SAVECARD),
                f(0x2e, "CONFIGURE", CONFIGCARD),
                b(0x13, "", 1),
                b(0x12, "END GAME", 2),
                f(0x19, "PADDLE WAR", PADDLECARD),
                b(0x10, "QUIT", 3),
            ],
            Custom::None,
            0,
        ),
        // NEWGAMECARD 2e35
        card(
            8,
            0,
            0xc,
            vec![b(0x12, "BEGIN EASY GAME", 5), b(0x31, "BEGIN NORMAL GAME", 6), b(0x23, "BEGIN HARD GAME", 7)],
            Custom::None,
            1,
        ),
        // LOADCARD 2e3f
        card(4, 3, 0xd, (2..8).map(|h| b(h, "", 0)).collect(), Custom::Load, 0),
        // SAVECARD 2e41 (shares the items of the load card)
        Card { x: 4, y: 3, title: 0xe, items: Vec::new(), shared: Some(LOADCARD), custom: Custom::Save, cursor: 0 },
        // CONFIGCARD 2e76 (Keen 5: at x 0, with a gamepad entry)
        card(
            ep_sel!(8, 0, 8),
            0,
            0xf,
            {
                let mut v = vec![
                    f(0x1f, "SOUND", SOUNDCARD),
                    f(0x32, "MUSIC", MUSICCARD),
                    f(0x18, "OPTIONS", OPTIONSCARD),
                    item(UII_FOLDER, UI_SEPARATED, 0x25, "USE KEYBOARD", 0, Some(KEYBOARDCARD)),
                    f(0x02, "USE JOYSTICK #1", JOY1CARD),
                    f(0x03, "USE JOYSTICK #2", JOY2CARD),
                ];
                if super::EPISODE == 5 {
                    v.push(f(0x22, "", GAMEPADCARD));
                }
                v
            },
            Custom::Config,
            0,
        ),
        // SOUNDCARD 2e28
        card(
            8,
            0,
            0x10,
            vec![
                r(0x31, "NO SOUND EFFECTS"),
                r(0x19, "PC SPEAKER"),
                r(0x1e, "ADLIB/SOUNDBLASTER"),
                r(0x10, "QUIET ADLIB/SOUNDBLASTER"),
            ],
            Custom::None,
            0,
        ),
        // MUSICCARD 2e2e
        card(8, 0, 0x11, vec![r(0x31, "NO MUSIC"), r(0x1e, "ADLIB/SOUNDBLASTER")], Custom::None, 0),
        // OPTIONSCARD 2e4e (Keen 5: score box, two-button, jerky motion,
        // compatibility)
        card(
            8,
            0,
            0x16,
            if super::EPISODE == 5 {
                vec![f(0x1f, "", SCOREBOXCARD), f(0x14, "", TWOBUTTONCARD), f(0x32, "", JERKCARD), f(0x2e, "", COMPATCARD)]
            } else {
                vec![f(0x1f, "", SCOREBOXCARD), f(0x2e, "", COMPATCARD), f(0x14, "", TWOBUTTONCARD)]
            },
            Custom::None,
            0,
        ),
        // SCOREBOXCARD 2e43, COMPATCARD 2e45, TWOBUTTONCARD 2e47
        card(0, 0, 0, Vec::new(), Custom::ScoreBox, 0),
        card(0, 0, 0, Vec::new(), Custom::Compat, 0),
        card(0, 0, 0, Vec::new(), Custom::TwoButton, 0),
        // KEYBOARDCARD 2e68
        card(8, 0, 0x12, vec![f(0x32, "MOVEMENT", MOVEMENTCARD), f(0x30, "BUTTONS", BUTTONSCARD)], Custom::Keyboard, 0),
        // MOVEMENTCARD 2e5b
        card(
            0,
            0,
            0x13,
            vec![
                b(0, "UP & LEFT", 0),
                b(0, "UP", 0),
                b(0, "UP & RIGHT", 0),
                b(0, "RIGHT", 0),
                b(0, "DOWN & RIGHT", 0),
                b(0, "DOWN", 0),
                b(0, "DOWN & LEFT", 0),
                b(0, "LEFT", 0),
            ],
            Custom::KeySet,
            0,
        ),
        // BUTTONSCARD 2e62
        card(0, 0, 0x14, vec![b(0x24, "JUMP", 0), b(0x19, "POGO", 0), b(0x21, "FIRE", 0)], Custom::KeySet, 0),
        // JOY1CARD 2e6a, JOY2CARD 2e6c
        card(0, 0, 0x15, Vec::new(), Custom::Joystick, 0),
        card(0, 0, 0x15, Vec::new(), Custom::Joystick, 0),
        // PADDLECARD 2e78
        card(0, 0, 0, Vec::new(), Custom::Paddle, 0),
        // Keen 5: GAMEPADCARD, JERKCARD
        card(0, 0, 0x15, Vec::new(), Custom::Gamepad, 0),
        card(0, 0, 0, Vec::new(), Custom::Jerk, 0),
        // CHEATCARD (--cheatmode, not in the original)
        card(
            8,
            0,
            0,
            vec![b(0x26, "ADD EXTRA LIFE", 0), b(0x1f, "ADD EXTRA SHOT", 0), b(0x13, "ALLOW RE-ENTER", 0)],
            Custom::Cheat,
            0,
        ),
    ];
    for c in cards.iter_mut() {
        if c.title != 0 {
            c.title += CP_TITLEOFS;
        }
    }
    cards.shrink_to_fit();
    cards
}

#[derive(Clone, Default)]
struct SaveSlot {
    present: bool,
    name: String,
}

/// Module-private state stored in `Game::menu`.
#[derive(Clone)]
pub struct ModState {
    cards: Vec<Card>,
    /// cardstack[0..=cstackptr]; the last entry is topcard
    stack: Vec<usize>,
    ingame: bool,
    /// CtlPanelDone
    done: bool,
    /// Communication
    comm: i32,
    /// GameIsDirty
    dirty: bool,
    /// QuitToDos
    quit_to_dos: bool,
    /// the three help lines at the bottom of the panel (DS:7262/7264/7266)
    footer: [&'static str; 3],
    /// VW px/py (pen position of USL_DrawString)
    px: i32,
    py: i32,
    games: Vec<SaveSlot>,
    /// --cheatmode quick save/load (F2/F3): the slot last loaded or saved
    quickslot: Option<usize>,
    /// SVGA compatibility (no effect in this port)
    pub compatibility: bool,
    /// Keen 5 "FIX JERKY MOTION" (K5 DS:0x6f12, jerk)
    pub jerk: bool,
    /// Controls[0]: 0 keyboard
    pub controltype: i32,
    creature_ok: bool,
    creature_num: i32,
}

impl Default for ModState {
    fn default() -> Self {
        ModState {
            cards: Vec::new(),
            stack: vec![ROOTCARD],
            ingame: false,
            done: false,
            comm: 0,
            dirty: false,
            quit_to_dos: false,
            footer: ["", "", ""],
            px: 0,
            py: 0,
            games: vec![SaveSlot::default(); MAXSAVEGAMES],
            quickslot: None,
            compatibility: false,
            jerk: false,
            controltype: 0,
            creature_ok: false,
            creature_num: -1,
        }
    }
}

/// Borland atoi
fn atoi(s: &str) -> i32 {
    let s = s.trim_start();
    let (neg, digits) = match s.as_bytes().first() {
        Some(b'-') => (true, &s[1..]),
        Some(b'+') => (false, &s[1..]),
        _ => (false, s),
    };
    let mut v: i32 = 0;
    for c in digits.bytes() {
        if !c.is_ascii_digit() {
            break;
        }
        v = v.wrapping_mul(10).wrapping_add((c - b'0') as i32);
    }
    if neg {
        -v
    } else {
        v
    }
}

// ---- save game serialization ---------------------------------------------

struct Writer(Vec<u8>);

impl Writer {
    fn i32(&mut self, v: i32) {
        self.0.extend_from_slice(&v.to_le_bytes());
    }
    fn u16(&mut self, v: u16) {
        self.0.extend_from_slice(&v.to_le_bytes());
    }
    fn bool(&mut self, v: bool) {
        self.0.push(v as u8);
    }
    fn opt(&mut self, v: Option<usize>) {
        self.i32(v.map(|n| n as i32).unwrap_or(-1));
    }
    fn bytes(&mut self, b: &[u8]) {
        self.i32(b.len() as i32);
        self.0.extend_from_slice(b);
    }
}

struct Reader<'a> {
    d: &'a [u8],
    pos: usize,
}

impl Reader<'_> {
    fn take(&mut self, n: usize) -> Result<&[u8], String> {
        if self.pos + n > self.d.len() {
            return Err("File is Incomplete".into());
        }
        let s = &self.d[self.pos..self.pos + n];
        self.pos += n;
        Ok(s)
    }
    fn i32(&mut self) -> Result<i32, String> {
        let b = self.take(4)?;
        Ok(i32::from_le_bytes([b[0], b[1], b[2], b[3]]))
    }
    fn u16(&mut self) -> Result<u16, String> {
        let b = self.take(2)?;
        Ok(u16::from_le_bytes([b[0], b[1]]))
    }
    fn bool(&mut self) -> Result<bool, String> {
        Ok(self.take(1)?[0] != 0)
    }
    fn len(&mut self, max: usize) -> Result<usize, String> {
        let n = self.i32()?;
        if n < 0 || n as usize > max {
            return Err("Bad saved game".into());
        }
        Ok(n as usize)
    }
    fn opt(&mut self, limit: usize) -> Result<Option<usize>, String> {
        let v = self.i32()?;
        if v < 0 {
            Ok(None)
        } else if (v as usize) < limit {
            Ok(Some(v as usize))
        } else {
            Err("Bad saved game".into())
        }
    }
    fn bytes(&mut self, max: usize) -> Result<Vec<u8>, String> {
        let n = self.len(max)?;
        Ok(self.take(n)?.to_vec())
    }
}

fn state_index(s: Option<StateRef>) -> i32 {
    match s {
        Some(s) => ALL_STATES.iter().position(|&t| std::ptr::eq(t, s)).map(|i| i as i32).unwrap_or(-1),
        None => -1,
    }
}

fn write_obj(w: &mut Writer, o: &Obj) {
    for v in [o.obclass, o.active, o.needtoreact as i32, o.needtoclip, o.nothink, o.x, o.y, o.xdir, o.ydir, o.xmove, o.ymove, o.xspeed, o.yspeed, o.ticcount] {
        w.i32(v);
    }
    w.i32(state_index(o.state));
    for v in [
        o.shapenum, o.priority, o.left, o.top, o.right, o.bottom, o.midx, o.tileleft, o.tiletop, o.tileright, o.tilebottom, o.tilemidx,
        o.hitnorth, o.hiteast, o.hitsouth, o.hitwest, o.temp1, o.temp2, o.temp3, o.temp4,
    ] {
        w.i32(v);
    }
    w.bool(o.sprite);
    w.opt(o.next);
    w.opt(o.prev);
}

fn read_obj(r: &mut Reader, limit: usize) -> Result<Obj, String> {
    let mut v = [0i32; 14];
    for x in v.iter_mut() {
        *x = r.i32()?;
    }
    let si = r.i32()?;
    let state = if si < 0 {
        None
    } else {
        Some(*ALL_STATES.get(si as usize).ok_or_else(|| "Bad saved game".to_string())?)
    };
    let mut u = [0i32; 20];
    for x in u.iter_mut() {
        *x = r.i32()?;
    }
    let sprite = r.bool()?;
    let next = r.opt(limit)?;
    let prev = r.opt(limit)?;
    Ok(Obj {
        obclass: v[0],
        active: v[1],
        needtoreact: v[2] != 0,
        needtoclip: v[3],
        nothink: v[4],
        x: v[5],
        y: v[6],
        xdir: v[7],
        ydir: v[8],
        xmove: v[9],
        ymove: v[10],
        xspeed: v[11],
        yspeed: v[12],
        ticcount: v[13],
        state,
        shapenum: u[0],
        priority: u[1],
        left: u[2],
        top: u[3],
        right: u[4],
        bottom: u[5],
        midx: u[6],
        tileleft: u[7],
        tiletop: u[8],
        tileright: u[9],
        tilebottom: u[10],
        tilemidx: u[11],
        hitnorth: u[12],
        hiteast: u[13],
        hitsouth: u[14],
        hitwest: u[15],
        temp1: u[16],
        temp2: u[17],
        temp3: u[18],
        temp4: u[19],
        sprite,
        next,
        prev,
    })
}

fn io_error_text(e: &std::io::Error) -> String {
    use std::io::ErrorKind::*;
    match e.kind() {
        NotFound => "No such file or directory".into(),
        PermissionDenied => "Permission denied".into(),
        UnexpectedEof => "File is Incomplete".into(),
        WriteZero | StorageFull => "Disk is Full".into(),
        _ => "Unknown".into(),
    }
}

/// The loaded contents of a saved game, applied only when complete.
struct LoadedGame {
    gs: GameState,
    mapwidth: i32,
    mapheight: i32,
    planes: [Vec<u16>; 3],
    objs: Vec<Obj>,
    obj_free: Vec<bool>,
    objfreelist: Option<usize>,
    lastobj: Option<usize>,
    objectcount: i32,
    player: usize,
    scoreobj: usize,
}

impl Game {
    // ---- small helpers ------------------------------------------------------

    /// VW_UpdateScreen + wait one tic (the original busy-waits).
    fn cp_wait(&mut self) {
        self.sys.wait_vbl(1);
    }

    fn cp_top(&self) -> usize {
        *self.menu.stack.last().unwrap_or(&ROOTCARD)
    }

    fn cp_owner(&self, c: usize) -> usize {
        self.menu.cards[c].shared.unwrap_or(c)
    }

    fn cp_nitems(&self, c: usize) -> usize {
        self.menu.cards[self.cp_owner(c)].items.len()
    }

    fn cp_item(&self, c: usize, i: usize) -> &Item {
        &self.menu.cards[self.cp_owner(c)].items[i]
    }

    fn cp_item_mut(&mut self, c: usize, i: usize) -> &mut Item {
        let o = self.cp_owner(c);
        &mut self.menu.cards[o].items[i]
    }

    /// VWB_Bar
    fn cp_bar(&mut self, x: i32, y: i32, w: i32, h: i32, color: u8) {
        self.sys.surface.bar(x, y, w, h, color);
    }

    /// VWB_Hlin
    fn cp_hlin(&mut self, x1: i32, x2: i32, y: i32, color: u8) {
        self.sys.surface.bar(x1, y, x2 - x1 + 1, 1, color);
    }

    /// VWB_Vlin
    fn cp_vlin(&mut self, y1: i32, y2: i32, x: i32, color: u8) {
        self.sys.surface.bar(x, y1, 1, y2 - y1 + 1, color);
    }

    /// USL_MeasureString
    fn cp_measure(&self, s: &str) -> (i32, i32) {
        self.measure_string(s.as_bytes())
    }

    /// USL_DrawString: draws at px/py (XOR) and advances px.
    fn cp_draw_string(&mut self, s: &str) {
        let b = s.as_bytes();
        let (w, _) = self.measure_string(b);
        let (x, y) = (self.menu.px, self.menu.py);
        self.draw_string_at(b, x, y);
        self.menu.px += w;
    }

    /// VWB_DrawSprite: draws a sprite chunk at a screen position.
    fn cp_draw_sprite(&mut self, x: i32, y: i32, chunk: i32) {
        let idx = chunk - super::consts::STARTSPRITES as i32;
        if idx < 0 || idx as usize >= self.gfx.sprites.len() {
            return;
        }
        let info = self.data.gfx.spritetable[idx as usize];
        let x = x + (info.orgx as i32 >> G_P_SHIFT);
        let y = y + (info.orgy as i32 >> G_P_SHIFT);
        // EGA sprites are stored in `shifts` pre-shifted copies
        let step = match info.shifts {
            4 => 2,
            2 => 4,
            1 => 8,
            _ => 1,
        };
        let tex = &self.gfx.sprites[idx as usize];
        self.sys.surface.blit(tex, x & !(step - 1), y);
    }

    fn sound_mode_num(&self) -> i32 {
        match self.sd.sound_mode {
            SdMode::Off => 0,
            SdMode::PC => 1,
            SdMode::AdLib => 2,
        }
    }

    fn music_mode_num(&self) -> i32 {
        match self.sd.music_mode {
            SmMode::Off => 0,
            SmMode::AdLib => 1,
        }
    }

    /// The configurable keys in the order of the table at DS:0x4112.
    fn cp_get_key(&self, i: usize) -> ScanCode {
        let d = &self.sys.input.key_def;
        match i {
            0 => d.button0,
            1 => d.button1,
            2 => self.firescan,
            3 => d.upleft,
            4 => d.up,
            5 => d.upright,
            6 => d.right,
            7 => d.downright,
            8 => d.down,
            9 => d.downleft,
            _ => d.left,
        }
    }

    fn cp_set_key(&mut self, i: usize, sc: ScanCode) {
        let d = &mut self.sys.input.key_def;
        match i {
            0 => d.button0 = sc,
            1 => d.button1 = sc,
            2 => self.firescan = sc,
            3 => d.upleft = sc,
            4 => d.up = sc,
            5 => d.upright = sc,
            6 => d.right = sc,
            7 => d.downright = sc,
            8 => d.down = sc,
            9 => d.downleft = sc,
            _ => d.left = sc,
        }
    }

    // ---- card stack -----------------------------------------------------------

    /// USL_SetupStack (0x29465)
    fn usl_setup_stack(&mut self) {
        self.menu.stack = vec![ROOTCARD];
    }

    /// USL_PopCard (0x29484)
    fn usl_pop_card(&mut self) {
        if self.menu.stack.len() > 1 {
            self.menu.stack.pop();
        }
    }

    /// USL_PushCard (0x294ab)
    fn usl_push_card(&mut self, c: usize) {
        if self.menu.stack.len() < MAXCARDS {
            self.menu.stack.push(c);
        }
    }

    // ---- drawing --------------------------------------------------------------

    /// USL_DrawItemIcon (0x294d8)
    fn usl_draw_item_icon(&mut self, i: usize) {
        let top = self.cp_top();
        if self.usl_custom(top, UIC_DRAWICON, Some(i)) {
            return;
        }
        let it = self.cp_item(top, i);
        let fl = it.flags;
        let icon = if fl & UI_DISABLED != 0 {
            if fl & UI_SELECTED != 0 {
                5
            } else {
                4
            }
        } else if it.ty == UII_RADIOBUTTON && fl & UI_PUSHED == 0 {
            if fl & UI_SELECTED != 0 {
                3
            } else {
                2
            }
        } else if fl & UI_SELECTED != 0 {
            1
        } else {
            0
        };
        let (x, y) = (it.x, it.y);
        self.draw_tile8(x, y, icon + CP_ICONTILE);
    }

    /// USL_DrawItem (0x2956a)
    fn usl_draw_item(&mut self, i: usize) {
        let top = self.cp_top();
        if self.usl_custom(top, UIC_DRAW, Some(i)) {
            return;
        }
        let text = self.cp_item(top, i).text;
        self.usl_draw_item_text(i, text);
    }

    /// USL_DrawItem with the text given by the caller.
    fn usl_draw_item_text(&mut self, i: usize, text: &str) {
        let top = self.cp_top();
        let y = self.cp_item(top, i).y;
        self.cp_bar(0x4b, y, 0x9f, 8, BACKCOLOR);
        self.usl_draw_item_icon(i);
        let it = self.cp_item(top, i);
        let (fl, ix, iy) = (it.flags, it.x, it.y);
        self.tw.fontcolor = if fl & UI_SELECTED == 0 || fl & UI_DISABLED != 0 { 10 } else { 2 };
        self.menu.px = ix + 8;
        self.menu.py = iy + 1;
        self.cp_draw_string(text);
        self.tw.fontcolor = 15;
    }

    /// USL_DrawCtlPanelFooter (0x29600): the three help texts.
    fn usl_draw_footer(&mut self) {
        let f = self.menu.footer;
        self.tw.fontcolor = 10;
        self.menu.px = 0x4e;
        self.menu.py = 0x87;
        self.cp_draw_string(f[0]);
        let (w, _) = self.cp_measure(f[1]);
        self.menu.px = 0xe6 - w;
        self.cp_draw_string(f[1]);
        let (w, h) = self.cp_measure(f[2]);
        self.menu.px = ((0xa0 - w) as u16 >> 1) as i32 + 0x4a;
        self.menu.py += h + 1;
        self.cp_draw_string(f[2]);
        self.tw.fontcolor = 0;
        self.cp_hlin(0x4d, 0xe7, 0x85, 10);
    }

    /// USL_DrawCtlPanelContents (0x296a6)
    fn usl_draw_ctl_panel_contents(&mut self) {
        let top = self.cp_top();
        if self.usl_custom(top, UIC_DRAWCARD, None) {
            return;
        }
        let title = self.menu.cards[top].title;
        if title != 0 {
            self.cp_hlin(0x4d, 0xe7, 0x37, 10);
            self.draw_pic(0x50, 0x30, title);
        }
        self.usl_draw_footer();
        let n = self.cp_nitems(top);
        if n != 0 {
            let mut x = self.menu.cards[top].x + 0x4a;
            if x % 8 != 0 {
                x += 8 - x % 8;
            }
            let mut y = self.menu.cards[top].y + 0x3c;
            for i in 0..n {
                let it = self.cp_item_mut(top, i);
                if it.flags & UI_SEPARATED != 0 {
                    y += 8;
                }
                it.x = x;
                it.y = y;
                self.usl_draw_item(i);
                y += 8;
            }
            self.usl_custom(top, UIC_TOUCHUPCARD, None);
        }
    }

    /// USL_DrawCtlPanel (0x297ba)
    fn usl_draw_ctl_panel(&mut self) {
        let top = self.cp_top();
        if self.cp_nitems(top) != 0 || self.menu.cards[top].title != 0 {
            self.draw_pic(0, 0, CP_MENUSCREENPIC);
            self.usl_draw_ctl_panel_contents();
        }
        self.sys.present();
    }

    /// USL_DialogSetup (0x297ed): the box for a dialog, returns its corner.
    fn usl_dialog_setup(&mut self, w: i32, h: i32) -> (i32, i32) {
        self.draw_mpic(0x4a, 0x30, CP_MENUMASKPICM);
        let x = ((0xa0 - w) as u16 >> 1) as i32 + 0x4a;
        let y = ((0x66 - h) as u16 >> 1) as i32 + 0x30;
        self.cp_bar(x, y, w + 1, h + 1, BACKCOLOR);
        self.cp_hlin(x - 1, x + w + 1, y - 1, 10);
        self.cp_hlin(x - 1, x + w + 1, y + h + 1, 10);
        self.cp_vlin(y - 1, y + h + 1, x - 1, 10);
        self.cp_vlin(y - 1, y + h + 1, x + w + 1, 10);
        (x, y)
    }

    /// USL_ShowLoadSave (0x298b6): "Loading"/"Saving" box with the name.
    fn usl_show_load_save(&mut self, s: &str, name: &str) {
        let q = format!("'{}'", name);
        let (w1, h) = self.cp_measure(s);
        let (w2, _) = self.cp_measure(&q);
        let w = w1.max(w2) + 6;
        let (x, y) = self.usl_dialog_setup(w, h * 2 + 2);
        self.menu.py = y + 2;
        self.menu.px = x + ((w - w1) >> 1);
        self.cp_draw_string(s);
        self.menu.py += h;
        self.menu.px = x + ((w - w2) >> 1);
        self.cp_draw_string(&q);
        self.sys.present();
    }

    /// USL_CtlDialog (0x29990): true if the player pressed Y.
    fn usl_ctl_dialog(&mut self, s1: &str, s2: &str, s3: Option<&str>) -> bool {
        let (w1, h) = self.cp_measure(s1);
        let (w2, _) = self.cp_measure(s2);
        let w3 = s3.map(|s| self.cp_measure(s).0).unwrap_or(0);
        let w = w1.max(w2).max(w3) + 7;
        let hh = h * if s3.is_some() { 5 } else { 4 };
        let (x, y) = self.usl_dialog_setup(w, hh);
        self.tw.fontcolor = 2;
        self.menu.px = x + ((w - w1) >> 1);
        self.menu.py = y + h + 1;
        self.cp_draw_string(s1);
        self.menu.py += h * 2 - 1;
        let py = self.menu.py;
        self.cp_hlin(x + 3, x + w - 3, py, 10);
        self.menu.py += 2;
        self.tw.fontcolor = 10;
        self.menu.px = x + ((w - w2) >> 1);
        self.cp_draw_string(s2);
        self.menu.py += h;
        if let Some(s3) = s3 {
            self.menu.px = x + ((w - w3) >> 1);
            self.cp_draw_string(s3);
        }
        self.sys.present();
        self.sys.input.clear_keys_down();
        // the mouse buttons of the original (button 0 = Y, button 1 = Esc)
        // are not supported
        let c = loop {
            if !self.alive() {
                break SC_ESCAPE;
            }
            let c = self.sys.input.last_scan;
            if c != 0 {
                break c;
            }
            self.cp_wait();
        };
        self.sys.input.clear_keys_down();
        self.usl_draw_ctl_panel();
        c == SC_Y
    }

    /// USL_ConfirmComm (0x29b43)
    fn usl_confirm_comm(&mut self, comm: i32) -> bool {
        if comm == 0 {
            self.quit("USL_ConfirmComm() - empty comm");
            return false;
        }
        let dirty = self.menu.ingame && self.menu.dirty;
        let s3 = "ESC TO BACK OUT";
        let (s1, s2, dialog) = match comm {
            UC_ABORT => ("REALLY END CURRENT GAME?", "PRESS Y TO END IT", dirty),
            UC_QUIT => ("REALLY QUIT?", "PRESS Y TO QUIT", true),
            UC_LOADED => ("YOU'RE IN A GAME", "PRESS Y TO LOAD GAME", dirty),
            UC_SEASY..=UC_SHARD => ("YOU'RE IN A GAME", "PRESS Y FOR NEW GAME", dirty),
            _ => ("", "", false),
        };
        let confirm = if dialog { self.usl_ctl_dialog(s1, s2, Some(s3)) } else { true };
        if confirm {
            self.menu.comm = comm;
            self.menu.done = true;
        }
        confirm
    }

    /// USL_HandleError (0x29c17)
    fn usl_handle_error(&mut self, msg: &str) {
        let buf = format!("Error: {}", msg);
        self.usl_ctl_dialog(&buf, "PRESS ANY KEY", None);
        self.sys.present();
        self.sys.input.clear_keys_down();
        self.wait_ack();
        self.sys.present();
    }

    // ---- custom card routines -------------------------------------------------

    /// Calls the custom routine of a card (group->custom).
    fn usl_custom(&mut self, c: usize, call: i32, i: Option<usize>) -> bool {
        match self.menu.cards[c].custom {
            Custom::None => false,
            Custom::Load => self.usl_load_custom(call, i),
            Custom::Save => self.usl_save_custom(call, i),
            Custom::Config => self.usl_config_custom(call),
            Custom::ScoreBox => self.usl_score_custom(call),
            Custom::Compat => self.usl_comp_custom(call),
            Custom::TwoButton => self.usl_two_custom(call),
            Custom::Keyboard => self.usl_keyboard_custom(call),
            Custom::KeySet => self.usl_keys_custom(call, i),
            Custom::Joystick => self.usl_joy_custom(call),
            Custom::Paddle => self.usl_pong_custom(call),
            Custom::Jerk => self.usl_jerk_custom(call),
            Custom::Gamepad => false,
            Custom::Cheat => self.usl_cheat_custom(call, i),
        }
    }

    /// USL_SetOptionsText (0x29c9a)
    fn usl_set_options_text(&mut self) {
        let showscorebox = self.showscorebox;
        let compat = self.menu.compatibility;
        let two = self.twobutton;
        let jerk = self.menu.jerk;
        let it = &mut self.menu.cards[OPTIONSCARD].items;
        it[0].text = if showscorebox { "SCORE BOX (ON)" } else { "SCORE BOX (OFF)" };
        let (ic, it2) = if super::EPISODE == 5 { (3, 1) } else { (1, 2) };
        it[ic].text = if compat { "SVGA COMPATIBILITY (ON)" } else { "SVGA COMPATIBILITY (OFF)" };
        it[it2].text = if two { "TWO-BUTTON FIRING (ON)" } else { "TWO-BUTTON FIRING (OFF)" };
        if super::EPISODE == 5 {
            // Keen 5 (0x2c099): jerky motion option and the gamepad entry,
            // which is only enabled with a joystick control (never here)
            it[2].text = if jerk { "FIX JERKY MOTION (ON)" } else { "FIX JERKY MOTION (OFF)" };
            let ct = self.menu.controltype;
            let g = &mut self.menu.cards[CONFIGCARD].items[6];
            g.flags |= UI_DISABLED;
            if ct == 2 || ct == 3 {
                g.flags &= !UI_DISABLED;
            }
            g.text = "USE GRAVIS GAMEPAD (OFF)";
        }
        let fire = &mut self.menu.cards[BUTTONSCARD].items[2];
        fire.flags &= !UI_DISABLED;
        if two {
            fire.flags |= UI_DISABLED;
        }
    }

    /// USL_ScoreCustom (0x29d17)
    fn usl_score_custom(&mut self, call: i32) -> bool {
        if call != UIC_SETUPCARD {
            return false;
        }
        self.showscorebox = !self.showscorebox;
        let s = if self.showscorebox { "Score box now on" } else { "Score box now off" };
        self.usl_ctl_dialog(s, "Press any key", None);
        self.usl_set_options_text();
        true
    }

    /// USL_CompCustom (0x29d51): SVGA compatibility (a no-op flag here)
    fn usl_comp_custom(&mut self, call: i32) -> bool {
        if call != UIC_SETUPCARD {
            return false;
        }
        self.menu.compatibility = !self.menu.compatibility;
        let s = if self.menu.compatibility { "SVGA compatibility now on" } else { "SVGA compatibility now off" };
        self.usl_ctl_dialog(s, "Press any key", None);
        self.usl_set_options_text();
        true
    }

    /// USL_JerkCustom (K5 0x2c18d): FIX JERKY MOTION (Keen 5)
    fn usl_jerk_custom(&mut self, call: i32) -> bool {
        if call != UIC_SETUPCARD {
            return false;
        }
        self.menu.jerk = !self.menu.jerk;
        let s = if self.menu.jerk { "Jerky motion fix enabled" } else { "Jerky motion fix disabled" };
        self.usl_ctl_dialog(s, "Press any key", None);
        self.usl_set_options_text();
        true
    }

    /// USL_TwoCustom (0x29d8b)
    fn usl_two_custom(&mut self, call: i32) -> bool {
        if call != UIC_SETUPCARD {
            return false;
        }
        self.twobutton = !self.twobutton;
        let s = if self.twobutton { "Two-button firing now on" } else { "Two-button firing now off" };
        self.usl_ctl_dialog(s, "Press any key", None);
        self.usl_set_options_text();
        true
    }

    /// USL_ConfigCustom (0x29dc5): "CONTROL: KEYBOARD" line
    fn usl_config_custom(&mut self, call: i32) -> bool {
        if call == UIC_TOUCHUPCARD {
            let s = "CONTROL: ";
            let name = CTLNAMES[(self.menu.controltype as usize).min(4)];
            let (w1, _) = self.cp_measure(s);
            let (w2, h) = self.cp_measure(name);
            self.menu.py = 0x84 - h;
            self.menu.px = ((0xa0 - (w1 + w2)) as u16 >> 1) as i32 + 0x4a;
            self.tw.fontcolor = 10;
            self.cp_draw_string(s);
            self.cp_draw_string(name);
        }
        false
    }

    /// USL_CKSetKey (0x29e49): waits for the new key of entry `k`.
    fn usl_ck_set_key(&mut self, i: usize, k: usize) {
        let top = self.cp_top();
        let (ix, iy) = {
            let it = self.cp_item(top, i);
            (it.x, it.y)
        };
        let mut on = false;
        let mut lasttime = 0;
        self.sys.input.last_scan = 0;
        self.tw.fontcolor = 2;
        loop {
            if !self.alive() {
                return;
            }
            if self.sys.time_count() >= lasttime {
                on = !on;
                let c = self.tw.fontcolor ^ 8;
                self.cp_bar(ix + 0x5a, iy, 0x28, 8, c);
                self.cp_bar(ix + 0x5b, iy + 1, 0x26, 6, BACKCOLOR);
                if on {
                    self.draw_tile8(ix + 0x6a, iy, CP_KEYCURSORTILE);
                }
                self.sys.present();
                lasttime = self.sys.time_count() + TICKBASE / 2;
            }
            if self.sys.input.last_scan == SC_LSHIFT {
                self.sys.input.last_scan = 0;
            }
            if self.sys.input.last_scan != 0 {
                break;
            }
            self.cp_wait();
        }
        let sc = self.sys.input.last_scan;
        if sc != SC_ESCAPE {
            let used = (0..11).any(|j| j != k && self.cp_get_key(j) == sc);
            if used {
                self.usl_ctl_dialog("Key already used", "Press any key", None);
            } else {
                self.cp_set_key(k, sc);
            }
        }
        self.sys.input.clear_keys_down();
    }

    /// USL_KeyCustom (0x29fab)
    fn usl_keyboard_custom(&mut self, call: i32) -> bool {
        if call == UIC_SETUPCARD {
            self.menu.controltype = 0;
        }
        false
    }

    /// USL_KeySCustom (0x29fbe): movement and button key entries
    fn usl_keys_custom(&mut self, call: i32, i: Option<usize>) -> bool {
        let top = self.cp_top();
        let i = i.unwrap_or(0);
        let k = if top == MOVEMENTCARD { i + 3 } else { i };
        match call {
            UIC_SETUPCARD => {
                self.menu.controltype = 0;
                false
            }
            UIC_DRAW => {
                let y = self.cp_item(top, i).y;
                self.cp_bar(0x4b, y, 0x9f, 8, BACKCOLOR);
                self.usl_draw_item_icon(i);
                let it = self.cp_item(top, i);
                let (ix, iy, text) = (it.x, it.y, it.text);
                self.tw.fontcolor = if it.flags & UI_SELECTED == 0 { 10 } else { 2 };
                self.menu.px = ix + 8;
                self.menu.py = iy + 1;
                self.cp_draw_string(text);
                let c = self.tw.fontcolor ^ 8;
                self.cp_bar(ix + 0x5a, iy, 0x28, 8, c);
                self.cp_bar(ix + 0x5b, iy + 1, 0x26, 6, BACKCOLOR);
                self.menu.px = ix + 0x60;
                self.menu.py = iy + 1;
                let name = scan_name(self.cp_get_key(k));
                self.cp_draw_string(&name);
                true
            }
            UIC_HIT => {
                self.usl_keys_custom(UIC_DRAW, Some(i));
                self.usl_ck_set_key(i, k);
                self.usl_draw_ctl_panel();
                true
            }
            _ => false,
        }
    }

    /// --cheatmode card: extra lives and shots, re-entering finished levels.
    fn usl_cheat_custom(&mut self, call: i32, i: Option<usize>) -> bool {
        match call {
            UIC_SETUPCARD => {
                // lives and shots only exist in a running game
                for n in 0..2 {
                    let it = &mut self.menu.cards[CHEATCARD].items[n];
                    if self.menu.ingame {
                        it.flags &= !UI_DISABLED;
                    } else {
                        it.flags |= UI_DISABLED;
                    }
                }
                false
            }
            UIC_TOUCHUPCARD => {
                // the card has no title picture
                let s = "CHEAT OPTIONS";
                let (w, _) = self.cp_measure(s);
                self.cp_hlin(0x4d, 0xe7, 0x37, 10);
                self.menu.px = ((0xa0 - w) as u16 >> 1) as i32 + 0x4a;
                self.menu.py = 0x31;
                self.tw.fontcolor = 10;
                self.cp_draw_string(s);
                self.tw.fontcolor = 15;
                false
            }
            UIC_DRAW => {
                let i = i.unwrap_or(0);
                let text = match i {
                    0 => format!("ADD EXTRA LIFE [{}]", self.gs.lives),
                    1 => format!("ADD EXTRA SHOT [{}]", self.gs.ammo),
                    _ => format!("ALLOW RE-ENTER [{}]", if self.opts.allow_reenter { "X" } else { " " }),
                };
                self.usl_draw_item_text(i, &text);
                true
            }
            UIC_HIT => {
                let i = i.unwrap_or(0);
                match i {
                    0 => self.gs.lives = (self.gs.lives + 1).min(CHEATMAX),
                    1 => self.gs.ammo = (self.gs.ammo + 1).min(CHEATMAX),
                    _ => {
                        self.opts.allow_reenter = !self.opts.allow_reenter;
                        // on the world map the level entries change at once
                        if self.menu.ingame && self.ca_mapon == 0 {
                            self.update_reenter_entries();
                        }
                    }
                }
                self.usl_draw_item(i);
                true
            }
            _ => false,
        }
    }

    /// USL_JoyCustom (0x2a3fb / 0x2a432). Joysticks are not supported: the
    /// menu entries are always disabled, so this is never reached.
    fn usl_joy_custom(&mut self, call: i32) -> bool {
        call == UIC_SETUPCARD
    }

    /// USL_DrawFileIcon (0x2a46a)
    fn usl_draw_file_icon(&mut self, i: usize) {
        let top = self.cp_top();
        let y = self.menu.cards[top].y + 0x3c + i as i32 * 11;
        let it = self.cp_item_mut(top, i);
        it.y = y;
        let (x, sel) = (it.x, it.flags & UI_SELECTED != 0);
        self.tw.fontcolor = if sel { 2 } else { 10 };
        let c = self.tw.fontcolor ^ 8;
        self.cp_hlin(x, x + 0x94, y, c);
        self.cp_hlin(x, x + 0x94, y + 9, c);
        self.cp_vlin(y, y + 9, x, c);
        self.cp_vlin(y, y + 9, x + 0x94, c);
    }

    /// USL_DoLoadGame (0x2a547)
    fn usl_do_load_game(&mut self, i: usize) {
        if !self.usl_confirm_comm(UC_LOADED) {
            return;
        }
        let name = self.menu.games[i].name.clone();
        self.usl_show_load_save("Loading", &name);
        match self.load_game_file(i) {
            Ok(()) => {
                self.loadedgame = true;
                self.menu.quickslot = Some(i);
            }
            Err(e) => {
                self.usl_handle_error(&e);
                // the original also sets abortgame here because its state
                // may be half loaded; ours is only applied when complete
                self.menu.comm = 0;
                self.menu.done = false;
            }
        }
        self.menu.games[i].present = true;
        if self.loadedgame {
            // the game is paused after loading (Paused = true)
            self.sys.input.paused = true;
        }
        self.usl_draw_ctl_panel();
    }

    /// USL_LoadCustom (0x2a650)
    fn usl_load_custom(&mut self, call: i32, i: Option<usize>) -> bool {
        match call {
            UIC_SETUPCARD => {
                self.usl_uid_check();
                for n in 0..MAXSAVEGAMES {
                    let present = self.menu.games[n].present;
                    let it = &mut self.menu.cards[LOADCARD].items[n];
                    if present {
                        it.flags &= !UI_DISABLED;
                    } else {
                        it.flags |= UI_DISABLED;
                    }
                }
                false
            }
            UIC_DRAWICON => {
                self.usl_draw_file_icon(i.unwrap_or(0));
                true
            }
            UIC_DRAW => {
                let i = i.unwrap_or(0);
                self.usl_draw_file_icon(i);
                let top = self.cp_top();
                let (ix, iy) = {
                    let it = self.cp_item(top, i);
                    (it.x, it.y)
                };
                self.cp_bar(ix + 1, iy + 2, 0x92, 7, BACKCOLOR);
                let present = self.menu.games[i].present;
                self.menu.px = if present { ix + 2 } else { ix + 0x3c };
                self.menu.py = iy + 2;
                let s = if present { self.menu.games[i].name.clone() } else { "Empty".to_string() };
                self.cp_draw_string(&s);
                true
            }
            UIC_HIT => {
                self.usl_do_load_game(i.unwrap_or(0));
                true
            }
            _ => false,
        }
    }

    /// USL_DoSaveGame (0x2a786)
    fn usl_do_save_game(&mut self, i: usize) {
        self.menu.footer[0] = "Type name";
        self.menu.footer[1] = "Enter accepts";
        self.usl_draw_ctl_panel();
        let top = self.cp_top();
        let (ix, iy) = {
            let it = self.cp_item(top, i);
            (it.x, it.y)
        };
        self.tw.fontcolor = 2;
        self.cp_bar(ix + 1, iy + 2, 0x92, 7, BACKCOLOR);
        let def = if self.menu.games[i].present { Some(self.menu.games[i].name.clone()) } else { None };
        let input = self.us_line_input(ix + 2, iy + 2, def.as_deref(), true, MAXGAMENAME, 0x8a);
        let mut ok = input.is_some();
        if let Some(s) = input {
            self.menu.games[i].name = s;
        }
        if self.menu.games[i].name.is_empty() {
            self.menu.games[i].name = "Untitled".to_string();
        }
        if ok {
            let name = self.menu.games[i].name.clone();
            self.usl_show_load_save("Saving", &name);
            if let Err(e) = self.save_game_file(i) {
                self.usl_handle_error(&e);
                let _ = std::fs::remove_file(self.save_path(i));
                ok = false;
            }
        }
        if !self.menu.games[i].present {
            self.menu.games[i].present = ok;
        }
        if ok {
            self.menu.dirty = false;
            self.menu.quickslot = Some(i);
        }
        self.usl_set_up_card();
    }

    /// USL_SaveCustom (0x2a940)
    /// Keen 4/5 Load/SaveCustom: `if (getenv("UID")) USL_CheckSavedGames();`
    /// (network installations re-read the saved game names).
    fn usl_uid_check(&mut self) {
        if super::EPISODE != 6 && std::env::var_os("UID").is_some() {
            self.usl_check_saved_games();
        }
    }

    fn usl_save_custom(&mut self, call: i32, i: Option<usize>) -> bool {
        match call {
            UIC_SETUPCARD => {
                self.usl_uid_check();
                for it in self.menu.cards[LOADCARD].items.iter_mut() {
                    it.flags &= !UI_DISABLED;
                }
                false
            }
            UIC_HIT => {
                self.usl_do_save_game(i.unwrap_or(0));
                true
            }
            _ => self.usl_load_custom(call, i),
        }
    }

    /// USL_DrawPongScore (0x2a99c)
    fn usl_draw_pong_score(&mut self, k: u16, c: u16) {
        self.tw.fontcolor = 2;
        self.menu.py = 0x34;
        self.tw.printy = 0x34;
        self.menu.px = 0x50;
        self.cp_bar(0x50, 0x34, 0x2a, 6, BACKCOLOR);
        self.cp_draw_string("KEEN:");
        self.tw.printx = self.menu.px;
        self.us_print_unsigned(k as u32);
        self.menu.px = 0xb6;
        let py = self.menu.py;
        self.cp_bar(0xb6, py, 0x32, 6, BACKCOLOR);
        self.cp_draw_string("COMP:");
        self.tw.printx = self.menu.px;
        self.us_print_unsigned(c as u32);
    }

    /// USL_PlayPong (0x2aa33): Paddle War
    fn usl_play_pong(&mut self) {
        const PADDLEMINX: u16 = 0x4e;
        const PADDLEMAXX: u16 = 0xdb;
        const BALLMINX: u16 = 0x4e;
        const BALLMAXX: u16 = 0xe4;
        const BALLMINY: u16 = 0x3e;
        const BALLMAXY: u16 = 0x8a;
        const CPADDLEY: u16 = 0x42;
        const KPADDLEY: u16 = 0x87;

        let mut kx: u16 = 0x94;
        let mut cx: u16 = 0x94;
        let mut oldbx: u16 = PADDLEMINX;
        let mut oldcx: u16 = PADDLEMINX;
        let mut oldkx: u16 = PADDLEMINX;
        let mut oldby: u16 = BALLMINY;
        let mut bx: u16 = 0;
        let mut by: u16 = 0;
        let mut bdx: i16 = 0;
        let mut bdy: i16 = 0;
        let mut kscore: u16 = 0;
        let mut cscore: u16 = 0;
        self.usl_draw_pong_score(0, 0);
        let mut cycle: u16 = 0;
        let mut revdir = false;
        let mut killball = true;
        let mut done = false;
        let mut lastscore = false;
        let mut ball = false;
        let mut balltime: i32 = 0;
        let mut speedup: i32 = 0;
        let mut x: u16 = 0;
        let mut y: u16 = 0;
        let mut rx: u16 = 0;
        let mut lasttime = self.sys.time_count();
        loop {
            let mut waittime;
            loop {
                waittime = self.sys.time_count() - lasttime;
                if waittime != 0 || !self.alive() {
                    break;
                }
                self.sys.wait_until(lasttime + 1);
            }
            lasttime = self.sys.time_count();
            if !(0..5).contains(&waittime) {
                waittime = 4;
            }
            while waittime != 0 && !done && self.sys.input.last_scan != SC_ESCAPE {
                waittime -= 1;
                // IN_ReadCursor: no mouse support, keyboard only
                if self.sys.input.key(SC_LEFTARROW) && kx > PADDLEMINX {
                    kx -= 2;
                } else if self.sys.input.key(SC_RIGHTARROW) && kx < PADDLEMAXX {
                    kx += 2;
                }
                if killball {
                    ball = false;
                    balltime = TICKBASE;
                    speedup = 10;
                    killball = false;
                    self.cp_bar(oldbx as i32, oldby as i32, 5, 5, BACKCOLOR);
                }
                if ball {
                    let c = cycle;
                    cycle = cycle.wrapping_add(1);
                    if c % 3 != 0 {
                        x = bx >> 2;
                        if x & 1 == 0 {
                            x += (self.rnd() & 1) as u16;
                        }
                        if cx + 6 < x && cx < PADDLEMAXX {
                            cx += 1;
                        } else if cx + 6 > x && cx > PADDLEMINX {
                            cx -= 1;
                        }
                    }
                }
                if !ball {
                    balltime -= 1;
                    if balltime != 0 {
                        continue;
                    }
                    ball = true;
                    bdx = 1 - (self.rnd() as i16 % 3);
                    bdy = if lastscore { -3 } else { 3 };
                    bx = (BALLMINX + (BALLMAXX - BALLMINX) / 2) << 2;
                    by = 0x63 << 2;
                }
                let nx = bx.wrapping_add(bdx as u16) >> 2;
                if nx > BALLMAXX || nx < BALLMINX {
                    self.play_sound(BALLBOUNCESND);
                    bdx = -bdx;
                }
                bx = bx.wrapping_add(bdx as u16);
                let ny = by.wrapping_add(bdy as u16) >> 2;
                if ny >= BALLMAXY {
                    killball = true;
                    lastscore = false;
                    cscore += 1;
                    self.play_sound(COMPSCOREDSND);
                    self.usl_draw_pong_score(kscore, cscore);
                    if cscore == 21 {
                        self.usl_ctl_dialog("You lost!", "Press any key", None);
                        done = true;
                        continue;
                    }
                } else if ny < BALLMINY {
                    killball = true;
                    lastscore = true;
                    kscore += 1;
                    self.play_sound(KEENSCOREDSND);
                    self.usl_draw_pong_score(kscore, cscore);
                    if kscore == 21 {
                        self.usl_ctl_dialog("You won!", "Press any key", None);
                        done = true;
                        continue;
                    }
                }
                by = by.wrapping_add(bdy as u16);
                x = bx >> 2;
                y = by >> 2;
                if !killball {
                    if bdy < 0 && y > CPADDLEY - 1 && y < CPADDLEY + 3 && cx.wrapping_sub(5) <= x && x < cx + 11 {
                        rx = cx;
                        revdir = true;
                        self.play_sound(COMPPADDLESND);
                    } else if bdy > 0 && y > KPADDLEY - 4 && y < KPADDLEY && kx.wrapping_sub(5) <= x && x < kx + 11 {
                        if (bdy >> 2) < 3 {
                            speedup -= 1;
                            if speedup == 0 {
                                bdy += 1;
                                speedup = 10;
                            }
                        }
                        rx = kx;
                        revdir = true;
                        self.play_sound(KEENPADDLESND);
                    }
                    if revdir {
                        bdy = -bdy;
                        let t = (x.wrapping_add(5).wrapping_sub(rx) >> 1) as i16;
                        bdx = t - 4;
                        if bdx == 0 {
                            bdx = t - 5;
                        }
                        revdir = false;
                    }
                }
            }
            if ball {
                self.cp_bar(oldbx as i32, oldby as i32, 5, 5, BACKCOLOR);
                oldby = y;
                oldbx = x;
                let spr = if x & 1 != 0 { BALL1PIXELTOTHERIGHTSPR } else { BALLSPR };
                self.cp_draw_sprite(x as i32, y as i32, spr);
            }
            self.cp_bar(oldcx as i32 - 3, CPADDLEY as i32, 0x10, 3, BACKCOLOR);
            oldcx = cx;
            self.cp_draw_sprite(cx as i32, CPADDLEY as i32, PADDLESPR);
            self.cp_bar(oldkx as i32 - 3, KPADDLEY as i32, 0x10, 3, BACKCOLOR);
            oldkx = kx;
            self.cp_draw_sprite(kx as i32, KPADDLEY as i32, PADDLESPR);
            self.sys.present();
            if self.sys.input.last_scan == SC_ESCAPE || done || !self.alive() {
                break;
            }
        }
        self.sys.input.clear_keys_down();
    }

    /// USL_PongCustom (0x2aea1)
    fn usl_pong_custom(&mut self, call: i32) -> bool {
        if call != UIC_SETUPCARD {
            return false;
        }
        self.draw_pic(0, 0, CP_MENUSCREENPIC);
        self.draw_pic(0x82, 0x30, CP_PADDLEWARPIC);
        self.cp_hlin(0x4d, 0xe7, 0x3c, 10);
        self.cp_hlin(0x4d, 0xe7, 0x8f, 10);
        self.usl_play_pong();
        true
    }

    // ---- item handling --------------------------------------------------------

    /// USL_ClearFlags (0x2af0b): clears pushed/selected/disabled recursively.
    fn usl_clear_flags(&mut self, c: usize) {
        let o = self.cp_owner(c);
        let mut kids = Vec::new();
        for it in self.menu.cards[o].items.iter_mut() {
            it.flags &= !(UI_PUSHED | UI_SELECTED | UI_DISABLED);
            if let Some(ch) = it.child {
                kids.push(ch);
            }
        }
        for ch in kids {
            self.usl_clear_flags(ch);
        }
    }

    /// USL_FindPushedItem (0x2af64)
    fn usl_find_pushed_item(&self, c: usize) -> i32 {
        let o = self.cp_owner(c);
        self.menu.cards[o].items.iter().position(|it| it.flags & UI_PUSHED != 0).map(|i| i as i32).unwrap_or(-1)
    }

    /// USL_SelectItem (0x2afa5)
    fn usl_select_item(&mut self, c: usize, i: usize, redraw: bool) {
        if self.cp_nitems(c) == 0 {
            self.menu.cards[c].cursor = i;
            return;
        }
        let cur = self.menu.cards[c].cursor;
        if cur != i && cur < self.cp_nitems(c) {
            self.cp_item_mut(c, cur).flags &= !UI_SELECTED;
            if redraw {
                self.usl_draw_item(cur);
            }
        }
        self.menu.cards[c].cursor = i;
        if i < self.cp_nitems(c) {
            self.cp_item_mut(c, i).flags |= UI_SELECTED;
            if redraw {
                self.usl_draw_item(i);
            }
        }
    }

    /// USL_PushItem (0x2b050): selects and pushes a radio button.
    fn usl_push_item(&mut self, c: usize, i: usize, redraw: bool) {
        self.usl_select_item(c, i, redraw);
        for j in 0..self.cp_nitems(c) {
            let it = self.cp_item_mut(c, j);
            if it.ty != UII_RADIOBUTTON {
                continue;
            }
            if j == i {
                it.flags |= UI_PUSHED;
                if redraw {
                    self.usl_draw_item(j);
                }
            } else if it.flags & UI_PUSHED != 0 {
                it.flags &= !UI_PUSHED;
                if redraw {
                    self.usl_draw_item(j);
                }
            }
        }
    }

    /// USL_NextItem (0x2b0e1)
    fn usl_next_item(&mut self) {
        let top = self.cp_top();
        let cur = self.menu.cards[top].cursor;
        if cur + 1 < self.cp_nitems(top) {
            self.usl_select_item(top, cur + 1, true);
        }
    }

    /// USL_PrevItem (0x2b125)
    fn usl_prev_item(&mut self) {
        let top = self.cp_top();
        let cur = self.menu.cards[top].cursor;
        if cur != 0 {
            self.usl_select_item(top, cur - 1, true);
        }
    }

    /// USL_SetUpCard (0x2b152)
    fn usl_set_up_card(&mut self) {
        self.menu.footer[0] = "Arrows move";
        self.menu.footer[1] = "Enter selects";
        self.menu.footer[2] = if self.menu.stack.len() != 1 {
            "ESC to back out"
        } else if self.opts.cheatmode && self.menu.ingame {
            "ESC to return to game"
        } else {
            "ESC to quit"
        };
        let top = self.cp_top();
        let cur = self.menu.cards[top].cursor;
        self.usl_select_item(top, cur, false);
        self.usl_draw_ctl_panel();
    }

    /// USL_DownLevel (0x2b193)
    fn usl_down_level(&mut self, c: usize) {
        self.usl_push_card(c);
        if self.usl_custom(c, UIC_SETUPCARD, None) {
            self.usl_pop_card();
        }
        self.usl_set_up_card();
    }

    /// USL_UpLevel (0x2b1dd)
    fn usl_up_level(&mut self) {
        if self.menu.stack.len() == 1 {
            // --cheatmode: in a game ESC returns to it (RETURN TO GAME)
            let comm = if self.opts.cheatmode && self.menu.ingame { 1 } else { UC_QUIT };
            self.usl_confirm_comm(comm);
            return;
        }
        let top = self.cp_top();
        let cur = self.menu.cards[top].cursor;
        if cur < self.cp_nitems(top) {
            self.cp_item_mut(top, cur).flags &= !UI_SELECTED;
        }
        self.usl_pop_card();
        self.usl_set_up_card();
    }

    /// USL_DoItem (0x2b228)
    fn usl_do_item(&mut self) {
        let top = self.cp_top();
        let cur = self.menu.cards[top].cursor;
        if cur >= self.cp_nitems(top) {
            return;
        }
        let it = self.cp_item(top, cur).clone();
        if it.flags & UI_DISABLED != 0 {
            self.play_sound(NOWAYSND);
            return;
        }
        match it.ty {
            UII_BUTTON => {
                if !self.usl_custom(top, UIC_HIT, Some(cur)) && it.comm != 0 {
                    self.usl_confirm_comm(it.comm);
                }
            }
            UII_RADIOBUTTON => self.usl_push_item(top, cur, true),
            UII_FOLDER => {
                if let Some(ch) = it.child {
                    self.usl_down_level(ch);
                }
            }
            _ => {}
        }
    }

    /// USL_InitFlags (0x2b2e3): radio buttons, disabled entries, root texts.
    fn usl_init_flags(&mut self) {
        let mut i = self.sound_mode_num();
        if i == 2 && self.sd.quiet_adlib {
            i = 3;
        }
        self.usl_push_item(SOUNDCARD, i as usize, false);
        let m = self.music_mode_num();
        self.usl_push_item(MUSICCARD, m as usize, false);
        // AdLib is always emulated, so its entries stay enabled.
        // No joystick support: both joystick entries are disabled.
        self.menu.cards[CONFIGCARD].items[4].flags |= UI_DISABLED;
        self.menu.cards[CONFIGCARD].items[5].flags |= UI_DISABLED;
        if !self.menu.ingame {
            let root = &mut self.menu.cards[ROOTCARD];
            root.items[4].text = "RETURN TO DEMO";
            root.items[2].flags |= UI_DISABLED;
            if !self.opts.cheatmode {
                root.items[5].flags |= UI_DISABLED;
            }
            root.cursor = 0;
        } else {
            let root = &mut self.menu.cards[ROOTCARD];
            root.items[4].text = "RETURN TO GAME";
            root.cursor = 4;
        }
        self.usl_set_options_text();
    }

    /// USL_SetUpCtlPanel (0x2b3c1)
    fn usl_set_up_ctl_panel(&mut self) {
        if self.menu.cards.is_empty() {
            self.menu.cards = build_cards();
            if self.opts.cheatmode {
                self.menu.cards[ROOTCARD].items[5] = item(UII_FOLDER, 0, 0x23, "CHEAT OPTIONS", 0, Some(CHEATCARD));
            }
        }
        self.usl_check_saved_games();
        // the control panel uses font 1 (fontnumber = 1)
        self.tw.fontnumber = 1;
        self.tw.fontcolor = 15;
        self.sys.surface.clear(0);
        self.menu.comm = 0;
        self.menu.quit_to_dos = false;
        self.usl_clear_flags(ROOTCARD);
        self.usl_init_flags();
        self.usl_setup_stack();
        self.usl_set_up_card();
        if self.menu.ingame {
            self.menu.dirty = true;
        }
        self.sys.input.clear_keys_down();
    }

    /// USL_GetConfig (0x2b4e5): applies the sound and music cards.
    fn usl_get_config(&mut self) {
        let mut i = self.usl_find_pushed_item(SOUNDCARD);
        let quiet = i == 3;
        if quiet {
            i = 2;
        }
        self.sd.set_quiet(quiet);
        if i >= 0 && i != self.sound_mode_num() {
            self.sd.set_sound_mode(match i {
                0 => SdMode::Off,
                1 => SdMode::PC,
                _ => SdMode::AdLib,
            });
        }
        let m = self.usl_find_pushed_item(MUSICCARD);
        if m >= 0 && m != self.music_mode_num() {
            self.sd.set_music_mode(if m == 0 { SmMode::Off } else { SmMode::AdLib });
        }
    }

    /// USL_HandleComm (0x2b48f)
    fn usl_handle_comm(&mut self, comm: i32) {
        match comm {
            1 | UC_LOADED => {}
            UC_ABORT => self.abortgame = true,
            UC_QUIT => self.menu.quit_to_dos = true,
            UC_SEASY..=UC_SHARD => {
                // a new game has no file to quick save to
                self.menu.quickslot = None;
                self.restartgame = true;
                self.restartdifficulty = comm - 4;
            }
            _ => self.quit("USL_HandleComm() - unknown"),
        }
    }

    /// USL_TearDownCtlPanel (0x2b53e)
    fn usl_tear_down_ctl_panel(&mut self) {
        self.usl_get_config();
        if self.menu.comm != 0 {
            let c = self.menu.comm;
            self.usl_handle_comm(c);
        }
        self.tw.fontnumber = 0;
        self.tw.fontcolor = 15;
        if self.restartgame {
            // USL_ResetGame hook = ResetGame (0x15c4e)
            self.new_game();
        } else if self.menu.quit_to_dos {
            self.us_center_window(20, 3);
            self.tw.fontcolor = 3;
            self.us_print_centered("Quitting...");
            self.tw.fontcolor = 15;
            self.sys.present();
            self.request_quit = true;
        }
        self.sys.input.clear_keys_down();
        self.sd.wait_sound_done(&mut self.sys);
        self.sys.surface.clear(0);
        // the original writes the config file in US_Shutdown
        self.save_config();
    }

    /// US_ControlPanel (0x2b5e1)
    pub fn control_panel(&mut self, ingame: bool) {
        let ls = self.sys.input.last_scan;
        if !(SC_F1..=SC_F10).contains(&ls) {
            self.sys.input.clear_keys_down();
        }
        // restartgame is consumed by the caller (GameLoop) before the panel
        // can be opened again; clear it so a new request is detected
        self.restartgame = false;
        self.menu.ingame = ingame;
        self.usl_set_up_ctl_panel();
        self.usl_draw_ctl_panel();
        let mut flash = true;
        let mut resetitem = true;
        let mut flashtime = 0;
        self.menu.done = false;
        while !self.menu.done {
            if !self.alive() {
                break;
            }
            let top = self.cp_top();
            let cur = self.menu.cards[top].cursor;
            let has_item = cur < self.cp_nitems(top);
            if resetitem {
                flashtime = self.sys.time_count() + TICKBASE / 2;
                resetitem = false;
            }
            if self.sys.time_count() >= flashtime && has_item {
                flash = !flash;
                resetitem = true;
                if !flash {
                    self.cp_item_mut(top, cur).flags &= !UI_SELECTED;
                }
                self.usl_draw_item_icon(cur);
                self.cp_item_mut(top, cur).flags |= UI_SELECTED;
            }
            self.cp_wait();
            let ls = self.sys.input.last_scan;
            if ls != 0 {
                match ls {
                    SC_UPARROW => {
                        self.usl_prev_item();
                        resetitem = true;
                    }
                    SC_DOWNARROW => {
                        self.usl_next_item();
                        resetitem = true;
                    }
                    SC_RETURN => {
                        self.usl_do_item();
                        resetitem = true;
                    }
                    SC_ESCAPE => {
                        self.usl_up_level();
                        resetitem = true;
                    }
                    // Keen 4/5: F1 shows the help screens, then the panel
                    // is redrawn
                    SC_F1 if super::EPISODE != 6 => {
                        self.help_screens();
                        self.usl_draw_ctl_panel();
                        resetitem = true;
                    }
                    _ => {}
                }
                let d = self.sys.input.key_def;
                if !resetitem && (ls == d.button0 || ls == d.button1) {
                    self.usl_do_item();
                    resetitem = true;
                }
                if !resetitem {
                    let top = self.cp_top();
                    if let Some(i) = (0..self.cp_nitems(top)).find(|&i| self.cp_item(top, i).hotkey == ls) {
                        self.usl_select_item(top, i, true);
                        resetitem = true;
                    }
                }
                self.sys.input.clear_keys_down();
            }
            // the mouse part of the original loop is not supported
        }
        self.usl_tear_down_ctl_panel();
    }

    // ---- US_LineInput (ID_US_1) -----------------------------------------------

    /// USL_XORICursor (0x29097)
    fn usl_xor_icursor(&mut self, x: i32, y: i32, s: &[u8], cursor: usize) {
        let (w, _) = self.measure_string(&s[..cursor.min(s.len())]);
        self.draw_string_at(b"\x80", x + w - 1, y);
    }

    /// US_LineInput (0x290ed): returns the entered text, None on escape.
    pub fn us_line_input(
        &mut self,
        x: i32,
        y: i32,
        def: Option<&str>,
        escok: bool,
        maxchars: usize,
        maxwidth: i32,
    ) -> Option<String> {
        let mut s: Vec<u8> = def.map(|d| d.as_bytes().to_vec()).unwrap_or_default();
        let mut olds: Vec<u8> = Vec::new();
        let mut cursor = s.len();
        let mut cursormoved = true;
        let mut redraw = true;
        let mut cursorvis = false;
        let mut done = false;
        let mut result = false;
        let mut lasttime = self.sys.time_count();
        self.sys.input.last_ascii = 0;
        self.sys.input.last_scan = 0;
        while !done {
            if !self.alive() {
                break;
            }
            if cursorvis {
                self.usl_xor_icursor(x, y, &s, cursor);
            }
            let sc = self.sys.input.last_scan;
            self.sys.input.last_scan = 0;
            let mut c = self.sys.input.last_ascii;
            self.sys.input.last_ascii = 0;
            match sc {
                SC_LEFTARROW => {
                    cursor = cursor.saturating_sub(1);
                    c = 0;
                    cursormoved = true;
                }
                SC_RIGHTARROW => {
                    if cursor < s.len() {
                        cursor += 1;
                    }
                    c = 0;
                    cursormoved = true;
                }
                SC_HOME => {
                    cursor = 0;
                    c = 0;
                    cursormoved = true;
                }
                SC_END => {
                    cursor = s.len();
                    c = 0;
                    cursormoved = true;
                }
                SC_RETURN => {
                    done = true;
                    result = true;
                    c = 0;
                }
                SC_ESCAPE => {
                    if escok {
                        done = true;
                        result = false;
                    }
                    c = 0;
                }
                SC_BACKSPACE => {
                    if cursor != 0 {
                        s.remove(cursor - 1);
                        cursor -= 1;
                        redraw = true;
                    }
                    c = 0;
                    cursormoved = true;
                }
                SC_DELETE => {
                    if cursor < s.len() {
                        s.remove(cursor);
                        redraw = true;
                    }
                    c = 0;
                    cursormoved = true;
                }
                0x4c | SC_UPARROW | SC_DOWNARROW | SC_PGUP | SC_PGDN | SC_INSERT => c = 0,
                _ => {}
            }
            if c != 0 {
                let len = s.len();
                let (w, _) = self.measure_string(&s);
                if (0x20..0x7f).contains(&c)
                    && len < MAXSTRING - 1
                    && (maxchars == 0 || len < maxchars)
                    && (maxwidth == 0 || w < maxwidth)
                {
                    s.insert(cursor, c);
                    cursor += 1;
                    redraw = true;
                }
            }
            if redraw {
                self.draw_string_at(&olds, x, y);
                olds = s.clone();
                self.draw_string_at(&s, x, y);
                redraw = false;
            }
            if cursormoved {
                cursorvis = false;
                lasttime = self.sys.time_count() - TICKBASE;
                cursormoved = false;
            }
            if self.sys.time_count() - lasttime > TICKBASE / 2 {
                lasttime = self.sys.time_count();
                cursorvis = !cursorvis;
            }
            if cursorvis {
                self.usl_xor_icursor(x, y, &s, cursor);
            }
            self.cp_wait();
        }
        if cursorvis {
            self.usl_xor_icursor(x, y, &s, cursor);
        }
        if !result {
            self.draw_string_at(&olds, x, y);
        }
        self.sys.present();
        self.sys.input.clear_keys_down();
        if result {
            Some(String::from_utf8_lossy(&s).into_owned())
        } else {
            None
        }
    }

    // ---- saved games ----------------------------------------------------------

    fn save_path(&self, n: usize) -> PathBuf {
        self.data.dir.join("saves").join(format!("SAVEGAM{}.{}", n, super::LAYOUT.ext))
    }

    /// USL_CheckSavedGames (0x283b1): reads the names of the saved games.
    fn usl_check_saved_games(&mut self) {
        for n in 0..MAXSAVEGAMES {
            let slot = std::fs::read(self.save_path(n)).ok().and_then(|d| {
                if d.len() < SAVEMAGIC.len() || &d[..SAVEMAGIC.len()] != SAVEMAGIC {
                    return None;
                }
                let mut r = Reader { d: &d, pos: SAVEMAGIC.len() };
                let name = r.bytes(MAXGAMENAME).ok()?;
                Some(SaveSlot { present: true, name: String::from_utf8_lossy(&name).into_owned() })
            });
            self.menu.games[n] = slot.unwrap_or_default();
        }
    }

    /// SaveTheGame (0x158ec) in our own format.
    fn save_game_file(&mut self, n: usize) -> Result<(), String> {
        self.gs.riding = None;
        let mut w = Writer(Vec::new());
        w.0.extend_from_slice(SAVEMAGIC);
        w.bytes(self.menu.games[n].name.as_bytes());
        let gs = self.gs.clone();
        for v in [gs.worldx, gs.worldy] {
            w.i32(v);
        }
        for v in gs.leveldone {
            w.i32(v);
        }
        for v in [gs.score, gs.nextextra, gs.ammo, gs.drops, gs.sandwichstate, gs.hookstate, gs.passcardstate, gs.rocketstate] {
            w.i32(v);
        }
        if super::EPISODE != 6 {
            // Keen 4/5 gametype fields (not in the Keen 6 format)
            for v in [gs.wetsuit, gs.rescued, gs.keycard, gs.destroyed, gs.numfuses] {
                w.i32(v);
            }
        }
        for v in gs.keys {
            w.i32(v);
        }
        for v in [gs.mapon, gs.lives, gs.difficulty] {
            w.i32(v);
        }
        w.i32(self.mapwidth);
        w.i32(self.mapheight);
        for p in &self.mapplanes {
            w.i32(p.len() as i32);
            for &t in p {
                w.u16(t);
            }
        }
        w.i32(self.objs.len() as i32);
        for (o, &free) in self.objs.iter().zip(self.obj_free.iter()) {
            w.bool(free);
            write_obj(&mut w, o);
        }
        w.opt(self.objfreelist);
        w.opt(self.lastobj);
        w.i32(self.objectcount);
        w.i32(self.player as i32);
        w.i32(self.scoreobj as i32);
        let path = self.save_path(n);
        if let Some(d) = path.parent() {
            std::fs::create_dir_all(d).map_err(|e| io_error_text(&e))?;
        }
        std::fs::write(&path, &w.0).map_err(|e| io_error_text(&e))
    }

    fn parse_saved_game(&self, d: &[u8]) -> Result<LoadedGame, String> {
        if d.len() < SAVEMAGIC.len() || &d[..SAVEMAGIC.len()] != SAVEMAGIC {
            return Err(format!("Not a Keen {} saved game", super::EPISODE));
        }
        let mut r = Reader { d, pos: SAVEMAGIC.len() };
        r.bytes(MAXGAMENAME * 4)?;
        let mut gs = GameState { worldx: r.i32()?, worldy: r.i32()?, ..Default::default() };
        for v in gs.leveldone.iter_mut() {
            *v = r.i32()?;
        }
        gs.score = r.i32()?;
        gs.nextextra = r.i32()?;
        gs.ammo = r.i32()?;
        gs.drops = r.i32()?;
        gs.sandwichstate = r.i32()?;
        gs.hookstate = r.i32()?;
        gs.passcardstate = r.i32()?;
        gs.rocketstate = r.i32()?;
        if super::EPISODE != 6 {
            gs.wetsuit = r.i32()?;
            gs.rescued = r.i32()?;
            gs.keycard = r.i32()?;
            gs.destroyed = r.i32()?;
            gs.numfuses = r.i32()?;
        }
        for v in gs.keys.iter_mut() {
            *v = r.i32()?;
        }
        gs.mapon = r.i32()?;
        gs.lives = r.i32()?;
        gs.difficulty = r.i32()?;
        gs.riding = None;
        let mapwidth = r.i32()?;
        let mapheight = r.i32()?;
        if !matches!(self.data.maps.maps.get(gs.mapon.max(0) as usize), Some(Some(_))) || gs.mapon < 0 {
            return Err("Bad saved game".into());
        }
        if mapwidth <= 0 || mapheight <= 0 || mapwidth * mapheight > 0x10000 {
            return Err("Bad saved game".into());
        }
        let size = (mapwidth * mapheight) as usize;
        let mut planes: [Vec<u16>; 3] = Default::default();
        for p in planes.iter_mut() {
            let n = r.len(size)?;
            if n != size {
                return Err("Bad saved game".into());
            }
            for _ in 0..n {
                p.push(r.u16()?);
            }
        }
        let count = r.len(MAXACTORS + 1)?;
        if count != MAXACTORS + 1 {
            return Err("Bad saved game".into());
        }
        let mut objs = Vec::with_capacity(count);
        let mut obj_free = Vec::with_capacity(count);
        for _ in 0..count {
            obj_free.push(r.bool()?);
            objs.push(read_obj(&mut r, count)?);
        }
        let objfreelist = r.opt(count)?;
        let lastobj = r.opt(count)?;
        let objectcount = r.i32()?;
        let player = r.opt(count)?.ok_or_else(|| "Bad saved game".to_string())?;
        let scoreobj = r.opt(count)?.ok_or_else(|| "Bad saved game".to_string())?;
        Ok(LoadedGame { gs, mapwidth, mapheight, planes, objs, obj_free, objfreelist, lastobj, objectcount, player, scoreobj })
    }

    /// LoadTheGame (0x159ea) from our own format.
    fn load_game_file(&mut self, n: usize) -> Result<(), String> {
        let d = std::fs::read(self.save_path(n)).map_err(|e| io_error_text(&e))?;
        let l = self.parse_saved_game(&d)?;
        // Keen 5 keeps the current numfuses (K5 LoadTheGame 0x15db4)
        let numfuses = self.gs.numfuses;
        self.gs = l.gs;
        if super::EPISODE == 5 {
            self.gs.numfuses = numfuses;
        }
        self.cache_map(self.gs.mapon);
        self.mapwidth = l.mapwidth;
        self.mapheight = l.mapheight;
        self.mapplanes = l.planes;
        self.objs = l.objs;
        self.obj_free = l.obj_free;
        self.objfreelist = l.objfreelist;
        self.lastobj = l.lastobj;
        self.objectcount = l.objectcount;
        self.player = l.player;
        self.scoreobj = l.scoreobj;
        for i in 0..self.objs.len() {
            if self.obj_free[i] {
                continue;
            }
            let o = &mut self.objs[i];
            o.needtoreact = true;
            o.sprite = false;
            match super::EPISODE {
                // Keen 4: obclass 0x21 (stunned), 0x14
                4 => {
                    if o.obclass == 0x21 {
                        o.temp3 = 0;
                    } else if o.obclass == 0x14 {
                        o.temp3 = 0;
                        o.temp2 = 0;
                    }
                }
                // Keen 5: obclass 7, 0xa, 0x16
                5 => {
                    if o.obclass == 7 {
                        o.temp3 = 0;
                    } else if o.obclass == 0xa {
                        o.temp4 = 0;
                    } else if o.obclass == 0x16 {
                        o.temp4 = 0;
                        o.temp3 = 0;
                        o.temp2 = 0;
                        o.temp1 = 0;
                    }
                }
                _ => {
                    if o.obclass == STUNNEDOBJ || o.obclass == PLATFORMOBJ {
                        o.temp3 = 0;
                    }
                }
            }
        }
        let s = &mut self.objs[self.scoreobj];
        s.temp1 = -1;
        s.temp2 = -1;
        s.temp3 = -1;
        s.temp4 = -1;
        self.rf_new_map();
        self.rf_mark_tile_graphics_public();
        let p = self.player;
        self.center_actor(p);
        self.loadedgame = true;
        self.playstate = EX_LOADEDGAME;
        Ok(())
    }

    /// --cheatmode F2 (not in the original): saves to the slot last loaded
    /// or saved without asking. False if there is no such slot.
    pub fn quick_save(&mut self) -> bool {
        let Some(i) = self.menu.quickslot else { return false };
        let r = self.save_game_file(i);
        if r.is_ok() {
            self.menu.dirty = false;
        }
        self.quick_message("Saved", r);
        true
    }

    /// --cheatmode F3 (not in the original): loads the slot last loaded or
    /// saved without asking. False if there is no such slot.
    pub fn quick_load(&mut self) -> bool {
        let Some(i) = self.menu.quickslot else { return false };
        let r = self.load_game_file(i);
        if r.is_ok() {
            // load_game_file centered the camera on Keen: show the new spot
            self.rf_refresh();
        }
        self.quick_message("Loaded savegame", r);
        true
    }

    /// Short in-game box after a quick save/load; errors wait for a key.
    fn quick_message(&mut self, msg: &str, r: Result<(), String>) {
        match r {
            Ok(()) => {
                self.us_center_window(18, 3);
                self.us_print_centered(msg);
                self.sys.present();
                self.sys.wait_vbl(35);
            }
            Err(e) => {
                let text = format!("Error: {}\nPress a key", e);
                self.us_center_window(24, 4);
                // center the two lines vertically
                let (_, h) = self.measure_string(b"A");
                self.tw.printy = self.tw.windowy + ((self.tw.windowh - 2 * h) >> 1);
                self.us_cprint(&text);
                self.sys.present();
                self.wait_ack();
            }
        }
    }

    // ---- config file ----------------------------------------------------------

    fn config_path(&self) -> PathBuf {
        self.data.dir.join("config.txt")
    }

    /// USL_ReadConfig (0x28145), as a text file `config.txt`.
    pub fn load_config(&mut self) {
        let Ok(text) = std::fs::read_to_string(self.config_path()) else {
            return;
        };
        let mut quiet = self.sd.quiet_adlib;
        let mut scores = Vec::new();
        for line in text.lines() {
            if let Some(v) = line.strip_prefix("highscore=") {
                let mut p = v.splitn(3, ',');
                if let (Some(a), Some(b), Some(n)) = (p.next(), p.next(), p.next()) {
                    scores.push(super::demo::HighScore {
                        score: a.trim().parse().unwrap_or(0),
                        completed: b.trim().parse().unwrap_or(0),
                        name: n.as_bytes().to_vec(),
                    });
                }
                continue;
            }
            let Some((k, v)) = line.split_once('=') else { continue };
            let (k, v) = (k.trim(), v.trim());
            let n: i32 = v.parse().unwrap_or(-1);
            let sc = if (1..128).contains(&n) { Some(n as ScanCode) } else { None };
            let d = &mut self.sys.input.key_def;
            match k {
                "sound" => match n {
                    0 => self.sd.set_sound_mode(SdMode::Off),
                    1 => self.sd.set_sound_mode(SdMode::PC),
                    2 => self.sd.set_sound_mode(SdMode::AdLib),
                    _ => {}
                },
                "music" => match n {
                    0 => self.sd.set_music_mode(SmMode::Off),
                    1 => self.sd.set_music_mode(SmMode::AdLib),
                    _ => {}
                },
                "quietadlib" => quiet = n == 1,
                "showscorebox" => self.showscorebox = n == 1,
                "fullscreen" => {
                    if self.opts.fullscreen.is_none() {
                        self.sys.set_fullscreen(n == 1);
                    }
                }
                "compatibility" => self.menu.compatibility = n == 1,
                "jerk" => self.menu.jerk = n == 1,
                "twobutton" => self.twobutton = n == 1,
                "controltype" => self.menu.controltype = 0,
                "button0" => d.button0 = sc.unwrap_or(d.button0),
                "button1" => d.button1 = sc.unwrap_or(d.button1),
                "upleft" => d.upleft = sc.unwrap_or(d.upleft),
                "up" => d.up = sc.unwrap_or(d.up),
                "upright" => d.upright = sc.unwrap_or(d.upright),
                "left" => d.left = sc.unwrap_or(d.left),
                "right" => d.right = sc.unwrap_or(d.right),
                "downleft" => d.downleft = sc.unwrap_or(d.downleft),
                "down" => d.down = sc.unwrap_or(d.down),
                "downright" => d.downright = sc.unwrap_or(d.downright),
                "firescan" => self.firescan = sc.unwrap_or(self.firescan),
                _ => {}
            }
        }
        if scores.len() == super::demo::MAXSCORES {
            self.demo.scores = scores;
        }
        self.sd.set_quiet(quiet);
    }

    /// USL_WriteConfig (0x282af), as a text file `config.txt`.
    pub fn save_config(&mut self) {
        let d = self.sys.input.key_def;
        let b = |v: bool| v as i32;
        let mut text = format!(
            "# Commander Keen {} configuration\n\
             sound={}\nmusic={}\nquietadlib={}\nshowscorebox={}\ncompatibility={}\ntwobutton={}\ncontroltype={}\n\
             button0={}\nbutton1={}\nupleft={}\nup={}\nupright={}\nleft={}\nright={}\ndownleft={}\ndown={}\ndownright={}\n\
             firescan={}\nfullscreen={}\n",
            super::EPISODE,
            self.sound_mode_num(),
            self.music_mode_num(),
            b(self.sd.quiet_adlib),
            b(self.showscorebox),
            b(self.menu.compatibility),
            b(self.twobutton),
            self.menu.controltype,
            d.button0,
            d.button1,
            d.upleft,
            d.up,
            d.upright,
            d.left,
            d.right,
            d.downleft,
            d.down,
            d.downright,
            self.firescan,
            b(self.sys.fullscreen())
        );
        if super::EPISODE == 5 {
            text += &format!("jerk={}\n", b(self.menu.jerk));
        }
        for h in &self.demo.scores {
            text += &format!("highscore={},{},{}\n", h.score, h.completed, String::from_utf8_lossy(&h.name));
        }
        if let Err(e) = std::fs::write(self.config_path(), text) {
            eprintln!("could not write config.txt: {}", e);
        }
    }

    // ---- CheckCreature (copy protection) --------------------------------------

    /// CheckCreature (0x2b7ca): "What is the name of this creature?"
    pub fn check_creature(&mut self) -> bool {
        // Keen 4/5 v1.4 have no copy protection question
        if super::EPISODE != 6 || self.menu.creature_ok {
            return true;
        }
        let mut ok = false;
        if self.menu.creature_num == -1 {
            // DOS time: hour + hundredths of a second
            let t = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap_or_default();
            let hour = (t.as_secs() / 3600 % 24) as i32;
            let hund = (t.subsec_millis() / 10) as i32;
            self.menu.creature_num = (hour + hund) % 12;
        }
        let (name, shape, xoff, yoff) = CREATURES[self.menu.creature_num as usize];
        self.cp_bar(0, 0, 320, 200, BACKCOLOR);
        let idx = (shape - super::consts::STARTSPRITES as i32) as usize;
        let info = self.data.gfx.spritetable[idx];
        let (w, h) = (info.width as i32, info.height as i32);
        self.us_center_window(30, ((h + 0x29) as u16 >> 3) as i32 + 1);
        self.tw.printy = self.tw.windowy + 2;
        self.us_cprint("What is the name of this creature?");
        let x = self.tw.windowx + ((self.tw.windoww - w) as u16 >> 1) as i32 + xoff * 8;
        let y = if shape == CEILICKSPR { self.tw.windowy + 0x10 } else { self.tw.windowy + 0xf + yoff * 8 };
        self.cp_draw_sprite(x, y, shape);
        let y = self.tw.windowy + self.tw.windowh;
        let mut w = 100;
        let mut x = self.tw.windowx + ((self.tw.windoww - 100) as u16 >> 1) as i32;
        self.cp_bar(x, y - 0x10, 100, 0xe, 0);
        self.cp_bar(x + 1, y - 0xf, w - 2, 0xc, 15);
        x += 2;
        w -= 8;
        self.sys.present();
        if let Some(buf) = self.us_line_input(x, y - 0xe, None, true, 0x10, w) {
            ok = buf.to_ascii_uppercase() == name;
            if !ok {
                self.cp_bar(0, 0, 320, 200, BACKCOLOR);
                self.us_center_window(0x23, 5);
                self.tw.printy += 0xb;
                self.us_cprint("Sorry, that's not quite right.");
                self.us_cprint("Please check your manual and try again.");
                self.sys.present();
                self.wait_ack();
            }
        }
        self.cp_bar(0, 0, 320, 200, BACKCOLOR);
        self.menu.creature_ok = ok;
        ok
    }

    // ---- DebugKeys (CK_PLAY) --------------------------------------------------

    /// Redraws the play screen under a debug window (VW_FixRefreshBuffer).
    fn debug_window(&mut self, w: i32, h: i32) {
        self.draw_playfield();
        self.draw_score_box();
        self.us_center_window(w, h);
    }

    fn debug_message(&mut self, w: i32, h: i32, s: &str) {
        self.debug_window(w, h);
        self.us_print_centered(s);
        self.sys.present();
        self.wait_ack();
    }

    /// Prompt + two character number input used by B, V and W.
    fn debug_number(&mut self, w: i32, prompt: &str) -> Option<i32> {
        self.debug_input(w, prompt, 2).map(|s| atoi(&s))
    }

    /// Prompt + line input of up to `maxchars` characters.
    pub(super) fn debug_input(&mut self, w: i32, prompt: &str, maxchars: usize) -> Option<String> {
        self.debug_window(w, 3);
        self.tw.printy += 6;
        self.us_print(prompt);
        self.sys.present();
        let (x, y) = (self.tw.printx, self.tw.printy);
        self.us_line_input(x, y, None, true, maxchars, 0)
    }

    /// IN_WaitForKey: waits for a key press and returns its scan code.
    fn wait_for_key(&mut self) -> ScanCode {
        self.sys.input.last_scan = 0;
        loop {
            self.sys.wait_vbl(1);
            if !self.alive() {
                return SC_ESCAPE;
            }
            let sc = self.sys.input.last_scan;
            if sc != 0 {
                self.sys.input.last_scan = 0;
                return sc;
            }
        }
    }

    /// PrintMemory (0x167a4). The port has no memory manager: "Total" is
    /// the size of the loaded game data, the free values are not tracked.
    fn print_memory(&mut self) {
        let d = &self.data;
        let mut total: usize = d.maps.maps.iter().flatten().map(|m| m.planes.iter().map(|p| p.len() * 2).sum::<usize>()).sum();
        total += d.gfx.externs.iter().map(|e| e.len()).sum::<usize>();
        let g = &self.gfx;
        let texsize = |t: &crate::engine::video::Tex| t.px.len() * 4;
        total += g.sprites.iter().chain(&g.pics).chain(&g.picms).chain(&g.tile8).chain(&g.tile8m).map(texsize).sum::<usize>();
        total += g.tile16.iter().chain(&g.tile16m).flatten().map(texsize).sum::<usize>();
        self.debug_window(0x10, 7);
        self.us_cprint("Memory Usage");
        self.us_cprint("------------");
        self.us_print("Total     :");
        self.us_print_unsigned((total / 1024) as u32);
        self.us_print("k\nFree      :-");
        self.us_print("k\nWith purge:-");
        self.us_print("k\n");
        self.sys.present();
        self.wait_ack();
    }

    /// TestSprites (0x16878): sprite viewer with the spritetable values.
    /// Up/Down = next/previous sprite, PgUp/PgDn = +/-10, Left/Right =
    /// shift, Esc = leave. "Mem" is the size of the unshifted EGA data.
    fn test_sprites(&mut self) {
        const FIRST: i32 = STARTSPRITES as i32;
        const LAST: i32 = FIRST + NUMSPRITES as i32 - 1;
        self.debug_window(0x1e, 0x11);
        self.us_cprint("Sprite Test");
        self.us_cprint("-----------");
        let top = self.tw.printy;
        let valx = (self.tw.printx + 0x38) & !7;
        let sprx = valx + 0x28;
        self.us_print("Chunk:\nWidth:\nHeight:\nOrgx:\nOrgy:\nXl:\nYl:\nXh:\nYh:\nShifts:\nMem:\n");
        let bottom = self.tw.printy;
        let mut chunk = FIRST;
        let mut shift = 0;
        loop {
            chunk = chunk.clamp(FIRST, LAST);
            let i = (chunk - FIRST) as usize;
            let info = self.data.gfx.spritetable[i];
            self.sys.surface.bar(valx, top, 0x28, bottom - top, 15);
            self.tw.printy = top;
            self.tw.printx = valx;
            self.us_print_unsigned(chunk as u32);
            for (n, v) in [info.width, info.height, info.orgx, info.orgy, info.xl, info.yl, info.xh, info.yh, info.shifts]
                .into_iter()
                .enumerate()
            {
                self.us_print("\n");
                self.tw.printx = valx;
                if n < 2 {
                    self.us_print_unsigned(v as u16 as u32);
                } else {
                    self.us_print_signed(v as i32);
                }
            }
            self.us_print("\n");
            self.tw.printx = valx;
            let mem = (info.width as u32 * info.height as u32 * 5 + 15) & !15;
            self.us_print_unsigned(mem);
            self.us_print("=");
            let current = chunk;
            while chunk == current {
                self.sys.surface.bar(sprx, top, 0x6e, bottom - top, 15);
                self.tw.printy = top;
                self.tw.printx = sprx;
                self.us_print("Shift:");
                self.us_print_unsigned(shift as u32);
                self.us_print("\n");
                let tex = &self.gfx.sprites[i];
                let x = sprx + shift * 2 + 0x10 + (info.orgx as i32 >> 4);
                let y = self.tw.printy + (info.orgy as i32 >> 4);
                self.sys.surface.blit(tex, x, y);
                self.sys.present();
                match self.wait_for_key() {
                    SC_ESCAPE => return,
                    SC_LEFTARROW => shift = (shift + 3) % 4,
                    SC_RIGHTARROW => shift = (shift + 1) % 4,
                    SC_UPARROW => chunk += 1,
                    SC_DOWNARROW => chunk -= 1,
                    SC_PGUP => chunk = (chunk + 10).min(LAST),
                    SC_PGDN => chunk = (chunk - 10).max(FIRST),
                    _ => {}
                }
            }
        }
    }

    /// F10+P (0x16bd4). The original copies the screen into a single
    /// buffer, shuts down and waits for Esc before exiting to DOS, a way to
    /// grab screenshots; the port saves a PNG to `screenshots/` instead.
    fn debug_screenshot(&mut self) {
        let dir = self.data.dir.join("screenshots");
        let _ = std::fs::create_dir_all(&dir);
        let n = (0..10000).find(|n| !dir.join(format!("SHOT{:04}.png", n)).exists()).unwrap_or(0);
        let path = dir.join(format!("SHOT{:04}.png", n));
        let msg = match self.sys.save_screenshot(&path) {
            Ok(()) => format!("Saved SHOT{:04}.png", n),
            Err(_) => "Screenshot failed!".to_string(),
        };
        self.play_sound(0x11);
        self.debug_message(0x14, 3, &msg);
    }

    /// WallDebug (0x16d2a), F10+Y: marks the wall types on every masked
    /// tile, which reveals fake walls and hidden passages. The original
    /// stamps the masked tiles 0x6c..0x7d over them, but KEEN 6 ships those
    /// chunks empty (it would draw garbage), so the port draws a bar on each
    /// side with a wall instead: north yellow, south red, east green, west
    /// cyan, slopes magenta. Like the original it changes the tile graphics
    /// until the program ends.
    fn wall_debug(&mut self) {
        self.debug_window(0x18, 3);
        self.us_print_centered("WORKING");
        self.sys.present();
        for i in 0..NUMTILE16M {
            if self.gfx.tile16m[i].is_none() {
                continue;
            }
            let ti = &self.data.tileinfo;
            let (north, south, east, west) = (ti.north[i] & 7, ti.south[i] & 7, ti.east[i] & 7, ti.west[i] & 7);
            if east > 1 {
                self.quit(&format!("WallDebug: East wall other than 1:{}", i));
                return;
            }
            if west > 1 {
                self.quit(&format!("WallDebug: West wall other than 1:{}", i));
                return;
            }
            let slope = 0xffff55ff;
            let Some(Some(t)) = self.gfx.tile16m.get_mut(i) else { continue };
            let b = (t.w / 16).max(1) * 2;
            let (w, h) = (t.w, t.h);
            let mut bar = |x0: usize, y0: usize, bw: usize, bh: usize, c: u32| {
                for y in y0..(y0 + bh).min(h) {
                    for x in x0..(x0 + bw).min(w) {
                        t.px[y * w + x] = c;
                    }
                }
            };
            if north != 0 {
                bar(0, 0, w, b, if north == 1 { 0xffffff55 } else { slope });
            }
            if south != 0 {
                bar(0, h - b, w, b, if south == 1 { 0xffff5555 } else { slope });
            }
            if east != 0 {
                bar(w - b, 0, b, h, 0xff55ff55);
            }
            if west != 0 {
                bar(0, 0, b, h, 0xff55ffff);
            }
            t.has_alpha = t.px.iter().any(|&p| p >> 24 < 0x80);
        }
    }

    /// DebugKeys (0x16e8f): F10 + key. Returns true if the screen needs a
    /// refresh. In the original B, C, D, E, G, S and W also require
    /// `inlevel`, which is always set while CheckKeys runs.
    pub fn debug_keys(&mut self) -> bool {
        let k = |g: &Game, sc: ScanCode| g.sys.input.key(sc);
        if k(self, SC_B) {
            if let Some(n) = self.debug_number(0x18, " Border color (0-15):") {
                if (0..16).contains(&n) {
                    self.sys.border = n as u8;
                    self.sys.palette.regs[16] = n as u8;
                }
            }
            return true;
        }
        if k(self, SC_C) {
            let mut active = 0u32;
            let mut inactive = 0u32;
            let mut i = Some(self.player);
            while let Some(n) = i {
                if self.objs[n].active == AC_NO {
                    inactive += 1;
                } else {
                    active += 1;
                }
                i = self.objs[n].next;
            }
            self.debug_window(0x12, 4);
            self.tw.printy += 7;
            self.us_print("Active Objects :");
            self.us_print_unsigned(active);
            self.us_print("\nInactive Objects:");
            self.us_print_unsigned(inactive);
            self.sys.present();
            self.wait_ack();
            return true;
        }
        if k(self, SC_D) {
            if self.demomode == 0 {
                self.start_demo_record();
            } else if self.demomode == 1 {
                self.end_demo_record();
                self.playstate = EX_COMPLETED;
            }
            return true;
        }
        if k(self, SC_E) {
            self.playstate = EX_COMPLETED;
        }
        if k(self, SC_G) {
            let s = if self.godmode { "God mode OFF" } else { "God mode ON" };
            self.debug_message(0xc, 2, s);
            self.godmode = !self.godmode;
            return true;
        }
        if k(self, SC_I) {
            self.debug_window(0xc, 3);
            self.us_print_centered("Free items!");
            self.gs.keys = [99; 4];
            self.gs.ammo = 99;
            match super::EPISODE {
                // Keen 4: the wetsuit, Keen 5: the keycard
                4 => self.gs.wetsuit = 1,
                5 => self.gs.keycard = 1,
                _ => {
                    self.gs.sandwichstate = 1;
                    self.gs.hookstate = 1;
                    self.gs.passcardstate = 1;
                }
            }
            self.sys.present();
            self.wait_ack();
            self.give_points(3000);
            return true;
        }
        if k(self, SC_J) {
            self.jumpcheat = !self.jumpcheat;
            let s = if self.jumpcheat { "Jump cheat ON" } else { "Jump cheat OFF" };
            self.debug_message(0x12, 3, s);
            return true;
        }
        if k(self, SC_M) {
            self.print_memory();
            return true;
        }
        if k(self, SC_N) {
            let p = self.player;
            let s = if self.objs[p].needtoclip == CL_NOCLIP {
                self.objs[p].needtoclip = CL_MIDCLIP;
                "No clipping OFF"
            } else {
                self.objs[p].needtoclip = CL_NOCLIP;
                "No clipping ON"
            };
            self.debug_message(0x12, 3, s);
            return true;
        }
        if k(self, SC_P) {
            self.debug_screenshot();
            return true;
        }
        if k(self, SC_S) {
            self.slowmotion = !self.slowmotion;
            let s = if self.slowmotion { "Slow motion ON" } else { "Slow motion OFF" };
            self.debug_message(0x12, 3, s);
            return true;
        }
        if k(self, SC_T) {
            self.test_sprites();
            return true;
        }
        if k(self, SC_V) {
            if let Some(n) = self.debug_number(0x1e, "  Add how many extra VBLs(0-8):") {
                if (0..9).contains(&n) {
                    self.extravbls = n;
                }
            }
            return true;
        }
        if k(self, SC_W) {
            if let Some(n) = self.debug_number(0x1a, "  Warp to which level(1-18):") {
                if (1..19).contains(&n) {
                    self.playstate = EX_WARPED;
                    self.gs.mapon = n;
                }
            }
            return true;
        }
        if k(self, SC_Y) {
            self.wall_debug();
            return true;
        }
        if k(self, SC_Z) {
            self.gs.lives = 0;
            self.kill_keen();
        }
        false
    }
}
