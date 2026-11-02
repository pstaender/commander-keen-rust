//! Meaning of the info plane (plane 2) values for Keen 4, 5 and 6, as read
//! by ScanInfoPlane, the world map code and the door/teleport logic, plus sprite
//! previews made by running the game's own spawn functions.

use keen::assets::GameData;
use std::collections::HashMap;
use std::path::Path;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Category {
    Keen,
    Enemy,
    Item,
    Platform,
    Special,
    WorldMap,
    Target,
}

impl Category {
    pub const ALL: [Category; 6] =
        [Category::Keen, Category::Enemy, Category::Item, Category::Platform, Category::Special, Category::WorldMap];

    pub fn name(self) -> &'static str {
        match self {
            Category::Keen => "Keen",
            Category::Enemy => "Enemies",
            Category::Item => "Items",
            Category::Platform => "Platforms",
            Category::Special => "Special",
            Category::WorldMap => "World map",
            Category::Target => "Door / teleport target",
        }
    }

    /// Badge color (RGB) for values without a sprite.
    pub fn color(self) -> [u8; 3] {
        match self {
            Category::Keen => [80, 220, 80],
            Category::Enemy => [230, 70, 70],
            Category::Item => [240, 200, 60],
            Category::Platform => [90, 160, 255],
            Category::Special => [200, 120, 255],
            Category::WorldMap => [255, 140, 40],
            Category::Target => [60, 220, 220],
        }
    }
}

const DIRS: [&str; 8] = ["up", "right", "down", "left", "up-right", "down-right", "down-left", "up-left"];
const ARROWS: [&str; 8] = ["↑", "→", "↓", "←", "↗", "↘", "↙", "↖"];
const DIFF: [&str; 3] = ["easy+", "normal+", "hard"];
const COLORS: [&str; 4] = ["red", "yellow", "blue", "green"];
const BONUS: [&str; 12] = [
    "red gem",
    "yellow gem",
    "blue gem",
    "green gem",
    "100 points",
    "200 points",
    "500 points",
    "1000 points",
    "2000 points",
    "5000 points",
    "extra life",
    "stunner ammo",
];

use std::sync::atomic::{AtomicI32, Ordering};

/// The episode of the loaded data (4, 5 or 6), set by the project.
static EPISODE: AtomicI32 = AtomicI32::new(6);

pub fn set_episode(ep: i32) {
    EPISODE.store(ep, Ordering::Relaxed);
}

pub fn episode() -> i32 {
    EPISODE.load(Ordering::Relaxed)
}

/// Highest info value with a meaning in ScanInfoPlane.
pub fn max_value() -> u16 {
    match episode() {
        4 => 88,
        5 => 125,
        _ => 106,
    }
}

/// Name, category and short label of an info value; `None` for unknown values.
pub fn describe(v: u16, world: bool) -> Option<(String, Category, String)> {
    match episode() {
        4 => describe4(v, world),
        5 => describe5(v, world),
        _ => describe6(v, world),
    }
}

/// World map markers and door/teleport targets (all episodes).
fn common(v: u16, world: bool) -> Option<(String, Category, String)> {
    use Category::*;
    match v {
        0xc001..=0xc012 if world => Some((format!("Level entrance: level {}", v & 0xff), WorldMap, format!("L{}", v & 0xff))),
        0xd001..=0xd012 if world => {
            Some((format!("Removed when level {} is done", v & 0xff), WorldMap, format!("D{}", v & 0xff)))
        }
        0xf001..=0xf012 if world => Some((format!("Flag of level {}", v & 0xff), WorldMap, format!("F{}", v & 0xff))),
        0x100.. => {
            let (x, y) = (v >> 8, v & 0xff);
            Some((format!("Target {},{} (door, teleporter, switch)", x, y), Target, "→".into()))
        }
        _ => None,
    }
}

fn d3(base: u16, v: u16) -> &'static str {
    DIFF[(v - base) as usize]
}

/// Keen 4 values (ScanInfoPlane 0x1de89).
fn describe4(v: u16, world: bool) -> Option<(String, Category, String)> {
    use Category::*;
    let s = |n: &str, c: Category, l: &str| Some((n.to_string(), c, l.to_string()));
    let e = |n: &str, base: u16, l: &str| Some((format!("{} ({})", n, d3(base, v)), Enemy, l.to_string()));
    match v {
        0 => None,
        1 => s("Keen start (facing right)", Keen, "K→"),
        2 => s("Keen start (facing left)", Keen, "←K"),
        3 => s("World map Keen start", Keen, "K"),
        42 => s("Swimming Keen start", Keen, "Ks"),
        4 => s("Council member", Special, "Cm"),
        5 => e("Berkeloid", 4, "Bk"),
        49 | 50 => e("Berkeloid", 48, "Bk"),
        6 => s("Princess Lindsey", Special, "Li"),
        7 => e("Wormouth", 6, "Wm"),
        51 | 52 => e("Wormouth", 50, "Wm"),
        8 => e("Skypest", 7, "Sp"),
        45 | 46 => e("Skypest", 44, "Sp"),
        9 => s("Thundercloud", Enemy, "Th"),
        10 => s("Giant foot (to the secret level)", Special, "Ft"),
        11 => s("Inchworm", Special, "In"),
        12 => s("Bounder", Enemy, "Bd"),
        13 => s("Egg (eggbird)", Enemy, "Eg"),
        14 => e("Lick", 13, "Lk"),
        47 | 48 => e("Lick", 46, "Lk"),
        15 => e("Dopefish", 14, "Df"),
        87 | 88 => e("Dopefish", 86, "Df"),
        16 => s("Schoolfish", Enemy, "Sf"),
        17 => e("Sprite", 16, "Pi"),
        23 | 24 => e("Sprite", 22, "Pi"),
        18 => s("Treasure eater", Enemy, "Te"),
        19 => s("Mimrock", Enemy, "Mr"),
        20 => e("Arachnut", 19, "Ar"),
        73 | 74 => e("Arachnut", 72, "Ar"),
        21 => s("Mad mushroom", Enemy, "Mm"),
        22 => e("Poison slug", 21, "Sl"),
        43 | 44 => e("Poison slug", 42, "Sl"),
        25 => s("Scroll block (horizontal)", Special, "|"),
        26 => s("Scroll block (vertical)", Special, "—"),
        27..=30 => Some((format!("Platform, moving {}", DIRS[(v - 27) as usize]), Platform, ARROWS[(v - 27) as usize].into())),
        32 => s("Dropping platform", Platform, "Dr"),
        33 => s("Miragia (mirage village)", Special, "Mi"),
        34 => s("Stunner ammo (only if Keen has less than 5 shots)", Item, "A?"),
        35 => s("Scuba gear (wetsuit)", Item, "Sc"),
        53..=56 => Some((format!("Dart gun, shooting {} (easy+)", DIRS[(v - 53) as usize]), Enemy, "Dg".into())),
        79..=82 => Some((format!("Dart gun, shooting {} (normal+)", DIRS[(v - 79) as usize]), Enemy, "Dg".into())),
        83..=86 => Some((format!("Dart gun, shooting {} (hard)", DIRS[(v - 83) as usize]), Enemy, "Dg".into())),
        57..=68 => Some((BONUS[(v - 57) as usize].to_string(), Item, "*".into())),
        69..=72 => Some((format!("Mine, moving {}", DIRS[(v - 69) as usize]), Enemy, "Mn".into())),
        76 => s("Eggbird (easy+)", Enemy, "Eb"),
        77 => s("Eggbird (normal+)", Enemy, "Eb"),
        78 => s("Eggbird (hard)", Enemy, "Eb"),
        _ => common(v, world),
    }
}

/// Keen 5 values (ScanInfoPlane 0x1ee32).
fn describe5(v: u16, world: bool) -> Option<(String, Category, String)> {
    use Category::*;
    let s = |n: &str, c: Category, l: &str| Some((n.to_string(), c, l.to_string()));
    let e = |n: &str, base: u16, l: &str| Some((format!("{} ({})", n, d3(base, v)), Enemy, l.to_string()));
    match v {
        0 => None,
        1 => s("Keen start (facing right)", Keen, "K→"),
        2 => s("Keen start (facing left)", Keen, "←K"),
        3 => s("World map Keen start", Keen, "K"),
        26 => s("World map Keen arriving through a teleporter", WorldMap, "Kt"),
        4..=6 => e("Sparky", 4, "Sk"),
        7..=9 => e("Shikadi mine", 7, "Mn"),
        10..=12 => e("Slicestar, sliding up/down", 10, "Ss"),
        13..=15 => e("Robo Red", 13, "RR"),
        16..=18 => e("Spirogrip", 16, "Sg"),
        19..=21 => e("Slicestar, bouncing diagonally", 19, "Ss"),
        22..=24 => e("Slicestar, sliding left/right", 22, "Ss"),
        25 => s("Scroll block (horizontal)", Special, "|"),
        27..=30 => Some((format!("Platform, moving {}", DIRS[(v - 27) as usize]), Platform, ARROWS[(v - 27) as usize].into())),
        32 => s("Dropping platform", Platform, "Dr"),
        33 => s("Static platform (easy only)", Platform, "P1"),
        34 => s("Static platform (easy and normal)", Platform, "P2"),
        35 => s("Static platform", Platform, "P"),
        36..=39 => Some((format!("Arrow-guided platform, starting {}", DIRS[(v - 36) as usize]), Platform, "G".into())),
        40 => s("Sneaky platform", Platform, "Sn"),
        41 => s("Fuse (map 12: the QED)", Special, "Fu"),
        42..=44 => e("Little Ampton", 42, "Am"),
        45..=48 => Some((format!("Laser turret, firing {} (easy+)", DIRS[(v - 45) as usize]), Enemy, "C".into())),
        49..=52 => Some((format!("Laser turret, firing {} (normal+)", DIRS[(v - 49) as usize]), Enemy, "C".into())),
        53..=56 => Some((format!("Laser turret, firing {} (hard)", DIRS[(v - 53) as usize]), Enemy, "C".into())),
        57..=68 => Some((BONUS[(v - 57) as usize].to_string(), Item, "*".into())),
        69 => s("Stunner ammo (only if Keen has less than 5 shots)", Item, "A?"),
        70 => s("Keycard", Item, "Kc"),
        71..=73 => e("Volte-face", 71, "Vf"),
        74..=76 => e("Shelley", 74, "Sh"),
        77..=79 => e("Spindred", 77, "Sd"),
        80..=83 => Some((format!("Arrow-guided platform (slot), starting {}", DIRS[(v - 80) as usize]), Platform, "G".into())),
        84..=87 => Some((format!("Platform (slot), moving {}", DIRS[(v - 84) as usize]), Platform, ARROWS[(v - 84) as usize].into())),
        88..=90 => e("Shikadi master", 88, "SM"),
        91..=98 => Some((format!("Platform arrow {}", DIRS[(v - 91) as usize]), Platform, ARROWS[(v - 91) as usize].into())),
        99..=101 => e("Shikadi", 99, "Sk"),
        102..=104 => e("Shockshund", 102, "Sd"),
        105..=107 => e("Sphereful", 105, "Sf"),
        124 => s("Korath", Enemy, "Ko"),
        _ => common(v, world),
    }
}

/// Keen 6 values (ScanInfoPlane 0x1da07).
fn describe6(v: u16, world: bool) -> Option<(String, Category, String)> {
    use Category::*;
    let s = |n: &str, c: Category, l: &str| Some((n.to_string(), c, l.to_string()));
    let d = |base: u16| DIFF[(v - base) as usize];
    let r = match v {
        0 => None,
        1 => s("Keen start (facing right)", Keen, "K→"),
        2 => s("Keen start (facing left)", Keen, "←K"),
        3 => s("World map Keen start", Keen, "K"),
        4..=6 => Some((format!("Bloog ({})", d(4)), Enemy, "Bl".into())),
        7..=14 => {
            let k = (v - 7) as usize;
            let gem = if k >= 4 { " carrying a gem" } else { "" };
            Some((format!("Blooglet, {}{}", COLORS[k % 4], gem), Enemy, "bl".into()))
        }
        15 => s("Grappling hook spot", Special, "Gr"),
        16 => s("Grappling hook spot (top)", Special, "Gr"),
        18..=20 => Some((format!("Fleex ({})", d(18)), Enemy, "Fx".into())),
        21..=24 => s("Molly", Special, "Mo"),
        25 => s("Scroll block (horizontal)", Special, "|"),
        26 => s("Scroll block (vertical)", Special, "—"),
        27..=30 => Some((format!("Platform, moving {}", DIRS[(v - 27) as usize]), Platform, ARROWS[(v - 27) as usize].into())),
        31 => s("Platform blocker", Platform, "X"),
        32 => s("Dropping platform", Platform, "Dr"),
        33 => s("Static platform (easy only)", Platform, "P1"),
        34 => s("Static platform (easy and normal)", Platform, "P2"),
        35 => s("Static platform", Platform, "P"),
        36..=39 => Some((format!("Arrow-guided platform, starting {}", DIRS[(v - 36) as usize]), Platform, "G".into())),
        40 => s("Sneaky platform", Platform, "Sn"),
        41..=43 => Some((format!("Bobba ({})", d(41)), Enemy, "Bo".into())),
        44 => s("Satellite stop", WorldMap, "S1"),
        45 => s("Satellite stop (landing)", WorldMap, "S2"),
        47..=49 => Some((format!("Nospike ({})", d(47)), Enemy, "Ns".into())),
        50..=52 => Some((format!("Gik ({})", d(50)), Enemy, "Gk".into())),
        53..=56 => Some((format!("Cannon, firing {}", DIRS[(v - 53) as usize]), Enemy, "C".into())),
        57..=68 => Some((BONUS[(v - 57) as usize].to_string(), Item, "*".into())),
        69 => s("Stunner ammo (only if Keen has less than 5 shots)", Item, "A?"),
        70..=72 => Some((format!("Orbatrix ({})", d(70)), Enemy, "Or".into())),
        73..=75 => Some((format!("Bip ship ({})", d(73)), Enemy, "Bp".into())),
        76..=78 => Some((format!("Flect ({})", d(76)), Enemy, "Fl".into())),
        79..=81 => Some((format!("Blorb ({})", d(79)), Enemy, "Bb".into())),
        82..=84 => Some((format!("Ceilick ({})", d(82)), Enemy, "Ce".into())),
        85..=87 => Some((format!("Blooguard ({})", d(85)), Enemy, "Bg".into())),
        88 => s("Grabbiter", WorldMap, "Gb"),
        89 => s("Satellite", WorldMap, "Sa"),
        91..=98 => Some((format!("Platform arrow {}", DIRS[(v - 91) as usize]), Platform, ARROWS[(v - 91) as usize].into())),
        99 => s("Grappling hook (special item)", Item, "Hk"),
        100 => s("Sandwich (special item)", Item, "Sw"),
        101 => s("Passcard (special item)", Item, "Pc"),
        102..=104 => Some((format!("Babobba ({})", d(102)), Enemy, "Ba".into())),
        105 => s("Rocket", WorldMap, "Ro"),
        106 => s("Rocket (destination)", WorldMap, "R2"),
        _ => common(v, world),
    };
    r
}

/// Values of the info palette, grouped by category.
pub fn palette_values(cat: Category) -> Vec<u16> {
    match cat {
        Category::WorldMap => {
            let mut v = match episode() {
                4 => vec![3],
                5 => vec![3, 26],
                _ => vec![3, 44, 45, 88, 89, 105, 106],
            };
            v.extend((1..=18).map(|l| 0xc000 + l));
            v.extend((1..=18).map(|l| 0xd000 + l));
            v.extend((1..=18).map(|l| 0xf000 + l));
            v
        }
        cat => (1..=max_value()).filter(|&v| describe(v, false).is_some_and(|d| d.1 == cat)).collect(),
    }
}

/// A sprite preview: sprite index (into the EGAGRAPH sprites) and the
/// offset of its top left corner from the tile, in pixels.
#[derive(Clone, Copy)]
pub struct Preview {
    pub sprite: usize,
    pub dx: i32,
    pub dy: i32,
}

/// Runs ScanInfoPlane on a tiny map for every actor value and records the
/// sprite the spawned object starts with (on hard, so everything spawns).
pub fn sprite_previews(data_dir: &Path) -> HashMap<u16, Preview> {
    let mut out = HashMap::new();
    let Ok(data) = GameData::load(data_dir) else { return out };
    let ep = data.episode.number();
    for (v, sprite, dx, dy) in keen::sprite_previews(data, 1..=max_value()) {
        out.insert(v, Preview { sprite, dx, dy });
    }
    // static platforms that only spawn on lower difficulties look the same
    for v in if ep == 4 { vec![] } else { vec![33, 34] } {
        if let Some(&p) = out.get(&35) {
            out.insert(v, p);
        }
    }
    out
}
