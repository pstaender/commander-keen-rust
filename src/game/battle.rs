//! Battle mode (not in the original): 2-4 players play one level against
//! each other for a set time (NEW GAME > BATTLE).
//!
//! It runs on top of the multiplayer mode (`mp.rs`): the same Keens,
//! controls, colors and score boxes. What is different in a battle:
//! - the players collect points (bonus items, stunning monsters); the
//!   items come back 10 seconds after they were taken,
//! - easy: no monsters; normal: the monsters stay stunned like in the
//!   game; hard: they wake up again after 5 seconds,
//! - nobody can leave the level (doors, exits and the objects that end a
//!   level are blocked or removed), a Keen who dies comes back at once,
//! - a stunner shot that hits another Keen knocks him out (dizzy); his
//!   score box counts it (0, -1, -2, ... where it shows the lives),
//! - the time left is shown at the top (MM:SS); when it runs
//!   out the results are shown on the high score
//!   screen: each Keen's helmet in his colors, how often he was knocked
//!   out and his score.
//!
//! Always played at 50 % zoom; no saving, no autosave. Never in demos or
//! with --classic.

use super::defs::*;
use super::mp::MAXPLAYERS;
use super::states::*;
use super::Game;

/// the battle's time is up (play loop -> game loop)
pub const EX_BATTLEOVER: i32 = 100;
/// tics until a bonus item that was taken comes back (10 seconds)
const ITEMTICS: i32 = 10 * 70;
/// hard: tics a stunned monster stays stunned (5 seconds)
const WAKETICS: i32 = 5 * 70;
/// points for stunning a monster
pub const STUNPOINTS: i32 = 200;
/// a Keen coming back is invincible for a moment
const COMEBACKINVINCIBLE: i32 = 70;
/// longest battle in minutes
pub const MAXMINUTES: i32 = 30;

/// Something taken that comes back.
#[derive(Clone)]
enum Item {
    /// a bonus object as it was before it was taken
    Obj(Obj),
    /// a bonus or drop tile (x, y, foreground tile)
    Tile(i32, i32, u16),
}

/// One line of the results.
#[derive(Clone, Debug)]
pub struct BattleResult {
    /// the player (0 = player 1)
    pub person: usize,
    pub score: i32,
    /// how often he was knocked out by another Keen's shot
    pub knocked: i32,
}

#[derive(Clone)]
pub struct BattleState {
    /// chosen in the control panel: the next new game is a battle
    pub pending: bool,
    /// a battle is being played
    pub active: bool,
    /// the choices of the control panel
    pub players: usize,
    pub difficulty: i32,
    /// map number, 0 = random
    pub level: i32,
    pub minutes: i32,
    /// tics left
    pub time_left: i32,
    /// per player (0 = player 1): how often another Keen's shot knocked
    /// him out
    pub knocked: [i32; MAXPLAYERS],
    /// where the leading Keen started (he comes back there)
    start: (i32, i32),
    /// taken items: (tics until they come back, item)
    items: Vec<(i32, Item)>,
    /// hard: the monsters as spawned, by object number
    monsters: Vec<Option<Obj>>,
    /// hard: stunned monsters (object, tics until they wake up)
    stunned: Vec<(usize, i32)>,
    /// the battle ended with the time up: the results, best first (the
    /// high score screen shows them instead of the high scores)
    pub results: Option<Vec<BattleResult>>,
}

impl Default for BattleState {
    fn default() -> Self {
        BattleState {
            pending: false,
            active: false,
            players: 2,
            difficulty: GD_NORMAL,
            level: 0,
            minutes: 5,
            time_left: 0,
            knocked: [0; MAXPLAYERS],
            start: (0, 0),
            items: Vec::new(),
            monsters: Vec::new(),
            stunned: Vec::new(),
            results: None,
        }
    }
}

impl Game {
    /// A battle is being played (not in demos).
    pub fn battle_on(&self) -> bool {
        self.battle.active && self.demomode == 0 && !self.storedemo && !self.opts.classic
    }

    /// The levels a battle can be played in: the episode's levels except
    /// Keen 4's underwater level, the ending and the high score map.
    pub fn battle_levels(&self) -> Vec<i32> {
        let names = super::play::LEVELNAMES;
        (1..names.len() as i32)
            .filter(|&n| {
                let name = names[n as usize];
                !(super::EPISODE == 4 && n == 0x11)
                    && !name.contains("Megarocket")
                    && !name.contains("High Scores")
                    && matches!(self.data.maps.maps.get(n as usize), Some(Some(_)))
            })
            .collect()
    }

    /// The name of map `n` on one line ("RANDOM" for 0).
    pub fn battle_level_name(n: i32) -> String {
        if n == 0 {
            return "RANDOM".into();
        }
        super::play::LEVELNAMES.get(n as usize).copied().unwrap_or("").replace('\n', " ")
    }

    /// NewGame: a battle chosen in the control panel starts (its level,
    /// time, players).
    pub fn battle_begin(&mut self) {
        if !std::mem::take(&mut self.battle.pending) {
            return;
        }
        let levels = self.battle_levels();
        if levels.is_empty() {
            return;
        }
        let b = &mut self.battle;
        b.active = true;
        b.knocked = [0; MAXPLAYERS];
        b.results = None;
        b.time_left = b.minutes.clamp(1, MAXMINUTES) * 60 * 70;
        let level = if levels.contains(&b.level) {
            b.level
        } else {
            let r = crate::vfs::now_since_epoch().subsec_nanos();
            levels[(r as usize / 1000) % levels.len()]
        };
        self.gs.mapon = level;
        self.gs.difficulty = self.battle.difficulty;
    }

    /// The battle is over (or another game was started or loaded).
    pub fn battle_stop(&mut self) {
        self.battle.active = false;
        self.battle.pending = false;
        self.battle.items.clear();
        self.battle.stunned.clear();
        self.battle.monsters.clear();
    }

    /// Objects that are monsters (removed in an easy battle): everything
    /// but Keen, items, platforms and the like.
    fn battle_monster(&self, ob: usize) -> bool {
        let c = self.objs[ob].obclass;
        if [NOTHING, INERTOBJ, KEENOBJ, STUNSHOTOBJ, BONUSOBJ, PLATFORMOBJ, STUNNEDOBJ, MAPFLAGOBJ].contains(&c) {
            return false;
        }
        !super::ep::BATTLE_KEEP.contains(&c)
    }

    /// After the level was set up: objects that would end the level go,
    /// easy battles have no monsters, hard ones remember them so they can
    /// wake up.
    pub fn battle_setup_level(&mut self) {
        if !self.battle_on() || self.gs.mapon == 0 {
            return;
        }
        let easy = self.gs.difficulty == GD_EASY;
        let hard = self.gs.difficulty == GD_HARD;
        self.battle.items.clear();
        self.battle.stunned.clear();
        self.battle.monsters = vec![None; self.objs.len()];
        let p = self.player;
        self.battle.start = (self.objs[p].x, self.objs[p].y);
        let mut cur = self.objs[p].next;
        while let Some(ob) = cur {
            cur = self.objs[ob].next;
            let c = self.objs[ob].obclass;
            if super::ep::BATTLE_REMOVE.contains(&c) || (easy && self.battle_monster(ob)) {
                self.remove_obj(ob);
            } else if hard && self.battle_monster(ob) {
                self.battle.monsters[ob] = Some(self.objs[ob].clone());
            }
        }
    }

    /// Once per frame (play loop): the clock, items coming back, monsters
    /// waking up, a new leading Keen.
    pub fn battle_tick(&mut self) {
        if !self.battle_on() || self.ca_mapon == 0 {
            return;
        }
        let tics = self.tics;
        // items
        let mut back = Vec::new();
        self.battle.items.retain_mut(|(t, it)| {
            *t -= tics;
            if *t <= 0 {
                back.push(it.clone());
                false
            } else {
                true
            }
        });
        for it in back {
            match it {
                Item::Tile(x, y, tile) => self.rf_mem_to_map(&[tile], 1, x, y, 1, 1),
                Item::Obj(o) => self.battle_respawn_obj(o),
            }
        }
        // monsters
        let mut wake = Vec::new();
        self.battle.stunned.retain_mut(|(ob, t)| {
            *t -= tics;
            if *t <= 0 {
                wake.push(*ob);
                false
            } else {
                true
            }
        });
        for ob in wake {
            self.battle_wake(ob);
        }
        // the clock (it ends the battle when the time is up)
        self.battle.time_left = (self.battle.time_left - tics).max(0);
        if self.battle.time_left == 0 && self.playstate == EX_STILLPLAYING {
            self.playstate = EX_BATTLEOVER;
        }
    }

    /// The end of a frame of the play loop: nothing may end the level but
    /// the time, and the leading Keen comes back when he died with nobody
    /// to take the lead.
    pub fn battle_guard(&mut self) {
        if !self.battle_on() || self.ca_mapon == 0 {
            return;
        }
        match self.playstate {
            EX_COMPLETED | EX_RESCUED | EX_WARPED | EX_FOOT | EX_SANDWICH | EX_HOOK | EX_CARD | EX_MOLLY | EX_PORTOUT | EX_FUSEBROKE
            | EX_QEDBROKE => self.playstate = EX_STILLPLAYING,
            EX_DIED => {
                self.playstate = EX_STILLPLAYING;
                if self.mp_battle_handover() {
                    return;
                }
                self.battle_leader_comes_back();
            }
            _ => {}
        }
    }

    /// A Keen walking off the map's left or right edge (which would end the
    /// level) is stopped at it.
    pub fn battle_keep_in(&mut self, ob: usize, right: i32) {
        let minx = self.cam.originxmin;
        let o = &mut self.objs[ob];
        if (o.left as u16) < minx as u16 {
            o.x += minx - o.left;
        } else if (right as u16) < o.right as u16 {
            o.x -= o.right - right;
        }
        o.xspeed = 0;
    }

    /// The leading Keen comes back where the level started.
    fn battle_leader_comes_back(&mut self) {
        let p = self.player;
        let (x, y) = self.battle.start;
        self.remove_stars(p);
        {
            let o = &mut self.objs[p];
            o.obclass = KEENOBJ;
            o.active = AC_ALLWAYS;
            o.priority = 1;
            o.x = x;
            o.y = y;
            o.xspeed = 0;
            o.yspeed = 0;
            o.xdir = if o.xdir == 0 { 1 } else { o.xdir };
            o.needtoclip = CL_MIDCLIP;
        }
        self.gs.riding = None;
        self.jumptime = 0;
        self.new_state(p, &S_KEENSTAND);
        self.mp.dizzy[0] = 0;
        self.invincible = COMEBACKINVINCIBLE;
        self.scrolllock = false;
        self.center_actor(p);
    }

    /// A bonus object was taken (KeenContact; `o` as it was before): it
    /// comes back later.
    pub fn battle_bonus_taken(&mut self, o: Obj) {
        if self.battle_on() && o.obclass == BONUSOBJ {
            self.battle.items.push((ITEMTICS, Item::Obj(o)));
        }
    }

    /// A bonus or drop tile is taken: it comes back later.
    pub fn battle_tile_taken(&mut self, x: i32, y: i32) {
        if self.battle_on() {
            let tile = self.fg_tile(x, y) as u16;
            self.battle.items.push((ITEMTICS, Item::Tile(x, y, tile)));
        }
    }

    /// A bonus object comes back as it was.
    fn battle_respawn_obj(&mut self, saved: Obj) {
        let n = self.get_new_obj(true);
        if n == DUMMYOBJ {
            return;
        }
        let (prev, next) = (self.objs[n].prev, self.objs[n].next);
        self.objs[n] = Obj { prev, next, sprite: false, needtoreact: true, active: AC_YES, ..saved };
        if let Some(st) = saved.state {
            self.new_state(n, st);
        }
    }

    /// StunObj: a monster was stunned by `shot`: points for the player who
    /// fired; in a hard battle the monster wakes up again.
    pub fn battle_stunned(&mut self, ob: usize, shot: usize) {
        if !self.battle_on() {
            return;
        }
        let owner = self.objs[shot].owner as usize;
        if owner > 0 {
            self.battle_points(owner - 1, STUNPOINTS);
        }
        if self.battle.monsters.get(ob).is_some_and(|m| m.is_some()) {
            self.battle.stunned.retain(|&(o, _)| o != ob);
            self.battle.stunned.push((ob, WAKETICS));
        }
    }

    /// A stunned monster wakes up: as spawned, where it is now.
    fn battle_wake(&mut self, ob: usize) {
        let Some(Some(snap)) = self.battle.monsters.get(ob).cloned() else { return };
        let cur = self.objs[ob].clone();
        if self.obj_free[ob] || cur.obclass != STUNNEDOBJ || cur.temp4 != snap.obclass {
            return;
        }
        let Some(st) = snap.state else { return };
        self.remove_stars(ob);
        let height = snap.bottom - snap.y;
        self.objs[ob] = Obj {
            prev: cur.prev,
            next: cur.next,
            x: cur.x,
            y: cur.bottom - height,
            active: AC_YES,
            needtoreact: true,
            sprite: cur.sprite,
            xspeed: 0,
            yspeed: 0,
            ..snap
        };
        self.new_state(ob, st);
    }

    /// Points for player `person` (0 = player 1).
    fn battle_points(&mut self, person: usize, points: i32) {
        if let Some(k) = self.mp.person.iter().position(|&p| p == person) {
            self.with_slot(k, |g| g.give_points(points));
        }
    }

    /// Another Keen's stunner shot hit the Keen of slot `k`: he is knocked
    /// out once more.
    pub fn battle_strike(&mut self, k: usize) {
        if !self.battle_on() {
            return;
        }
        if let Some(n) = self.battle.knocked.get_mut(self.mp.person[k]) {
            *n += 1;
        }
    }

    /// The game loop: the time is up. The results, best score first.
    pub fn battle_finish(&mut self) {
        let n = self.mp_count();
        let mut r: Vec<BattleResult> = (0..n)
            .map(|p| BattleResult {
                person: p,
                score: self.mp_status(p).map_or(0, |s| s.0),
                knocked: self.battle.knocked[p],
            })
            .collect();
        r.sort_by(|a, b| b.score.cmp(&a.score).then(a.knocked.cmp(&b.knocked)).then(a.person.cmp(&b.person)));
        // "Time's up!"
        self.draw_playfield();
        self.us_center_window(16, 3);
        self.us_print_centered("Time's up!");
        self.sys.present();
        self.sys.input.clear_keys_down();
        self.user_input(3 * 70);
        self.battle.results = Some(r);
    }

    /// The time left at the top of the play field: MM:SS in a box framed
    /// like the score box.
    pub fn draw_battle_timer(&mut self) {
        if !self.battle_on() || self.ca_mapon == 0 {
            return;
        }
        let secs = (self.battle.time_left + 69) / 70;
        let text = format!("{:02}{:02}", (secs / 60).min(99), secs % 60);
        // 4 digits of 8 pixels and a colon of 4
        let (pw, ph) = (4 * 8 + 4 + 4, 12);
        let (w, h) = (pw + 6, ph + 4);
        let x = (self.cam.view_w() - w) / 2;
        let y = 4;
        let s = &mut self.sys.surface;
        s.bar(x, y, w, h, 0);
        s.bar(x + 1, y, w - 1, 1, 15);
        s.bar(x + w - 2, y, 2, h - 1, 15);
        s.bar(x + w - 1, y + h - 1, 1, 1, 15);
        s.bar(x + 2, y + 1, w - 4, h - 2, 7);
        let (px, py) = (x + 3, y + 2);
        s.bar(px, py, pw, ph, 0);
        for (cx, cy) in [(px, py), (px + pw - 1, py), (px, py + ph - 1), (px + pw - 1, py + ph - 1)] {
            s.bar(cx, cy, 1, 1, 7);
        }
        // the last 10 seconds blink
        if secs <= 10 && (self.battle.time_left / 18) % 2 == 1 {
            return;
        }
        let mut cx = px + 2;
        for (i, c) in text.bytes().enumerate() {
            if i == 2 {
                // the colon in the digits' red
                self.sys.surface.bar(cx + 1, py + 4, 2, 1, 12);
                self.sys.surface.bar(cx + 1, py + 8, 2, 1, 12);
                cx += 4;
            }
            self.draw_tile8(cx, py + 2, c as i32 - 6);
            cx += 8;
        }
    }

    /// The helmet of player `person` (0 = player 1) in his colors, from the
    /// score box (its lives icon), black made transparent.
    pub fn battle_helmet(&mut self, person: usize) -> Option<crate::engine::video::Tex> {
        let idx = (super::keen2::SCOREBOXSPR - super::consts::STARTSPRITES as i32) as usize;
        if idx >= self.gfx.sprites.len() {
            return None;
        }
        let t = if person == 0 { &self.gfx.sprites[idx] } else { self.gfx.alt_sprite(person + 1, idx) };
        // (the icon at 9..22, 17..28 of the score box)
        let s = (t.w / 88).max(1);
        let (x0, y0, w, h) = (9 * s, 17 * s, 13 * s, 11 * s);
        if x0 + w > t.w || y0 + h > t.h {
            return None;
        }
        let mut px = Vec::with_capacity(w * h);
        for y in y0..y0 + h {
            for x in x0..x0 + w {
                let p = t.px[y * t.w + x];
                px.push(if p & 0xffffff == 0 { 0 } else { p });
            }
        }
        Some(crate::engine::video::Tex { w, h, px, has_alpha: true })
    }
}

#[cfg(test)]
mod tests {
    use super::super::defs::*;
    use super::super::Game;
    use crate::assets::GameData;
    use crate::engine::{gfx::Gfx, sound::SoundManager, sys::Sys};

    /// A headless battle in level 1: `players` Keens (three gamepads),
    /// difficulty `diff`; None without the game data.
    fn battle(tmp: &std::path::Path, players: usize, diff: i32) -> Option<Game> {
        let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join(format!("Keen{}", super::super::EPISODE));
        let Ok(data) = GameData::load(&dir) else {
            eprintln!("{}: no game data, skipped", dir.display());
            return None;
        };
        let gfx = Gfx::new(&data.gfx, 1, None);
        let sd = SoundManager::new(&data.audio, false, None, None);
        let sys = Sys::new(1, 1, true, true, false);
        let opts = crate::options::Options { cheatmode: true, zoom: 100, ..Default::default() };
        let mut g = Game::new(sys, data, gfx, sd, opts);
        g.data.dir = tmp.to_path_buf();
        g.data.write_dir = tmp.to_path_buf();
        g.sys.input.pad_connected = true;
        g.sys.input.pad_count = 3;
        g.battle.pending = true;
        g.battle.players = players;
        g.battle.difficulty = diff;
        g.battle.level = 1;
        g.battle.minutes = 2;
        g.new_game();
        assert!(g.battle.active && !g.battle.pending);
        assert_eq!(g.gs.mapon, 1);
        assert!(g.setup_game_level(false));
        Some(g)
    }

    fn tmpdir(name: &str) -> std::path::PathBuf {
        let tmp = std::env::temp_dir().join(format!("keen-battle-{}-{}-{}", name, super::super::EPISODE, std::process::id()));
        let _ = std::fs::create_dir_all(&tmp);
        tmp
    }

    fn monsters(g: &Game) -> usize {
        let mut n = 0;
        let mut cur = Some(g.player);
        while let Some(ob) = cur {
            if g.battle_monster(ob) {
                n += 1;
            }
            cur = g.objs[ob].next;
        }
        n
    }

    /// The players, time, zoom of a battle; the config's number of players
    /// stays; the battle ends with the time.
    #[test]
    fn a_battle_and_its_clock() {
        let tmp = tmpdir("clock");
        let Some(mut g) = battle(&tmp, 3, GD_NORMAL) else { return };
        assert_eq!(g.mp.players, 1);
        assert_eq!(g.mp_count(), 3);
        assert_eq!(g.zoom(), 50);
        assert!((1..3).all(|k| g.mp.slots[k].obj.is_some()));
        assert_eq!(g.battle.time_left, 2 * 60 * 70);
        g.playstate = EX_STILLPLAYING;
        g.tics = 3;
        g.battle_tick();
        assert_eq!(g.battle.time_left, 2 * 60 * 70 - 3);
        // level ends other than by the time don't count
        g.playstate = EX_COMPLETED;
        g.battle_guard();
        assert_eq!(g.playstate, EX_STILLPLAYING);
        g.battle.time_left = 2;
        g.battle_tick();
        assert_eq!(g.playstate, super::EX_BATTLEOVER);
        g.battle_stop();
        assert_eq!(g.mp_count(), 1);
        let _ = std::fs::remove_dir_all(&tmp);
    }

    /// A shot of player 2 hits the leading Keen: player 1 is knocked out,
    /// player 2 takes the lead (the camera); the leading Keen comes back
    /// where he started when he dies with nobody else alive.
    #[test]
    fn knocked_out_lead_and_coming_back() {
        let tmp = tmpdir("strike");
        let Some(mut g) = battle(&tmp, 2, GD_NORMAL) else { return };
        let start = (g.objs[g.player].x, g.objs[g.player].y);
        let shot = g.get_new_obj(false);
        g.objs[shot].obclass = STUNSHOTOBJ;
        g.objs[shot].state = Some(&super::super::states::S_STUNRAY1);
        g.objs[shot].owner = 2;
        let p1 = g.player;
        assert!(g.mp_shot_hits_keen(p1, shot));
        assert_eq!(g.battle.knocked, [1, 0, 0, 0]);
        g.tics = 1;
        g.mp_tick();
        assert_eq!(g.mp.person, [1, 0, 2, 3]);
        // player 1 (now slot 1) is the dizzy one
        assert!(g.mp.dizzy[1] > 0 && g.mp.dizzy[0] == 0);
        // hit again while he is dizzy: that doesn't count (and he keeps
        // the lead he took)
        let p1obj = g.mp.slots[1].obj.unwrap();
        let shot2 = g.get_new_obj(false);
        g.objs[shot2].obclass = STUNSHOTOBJ;
        g.objs[shot2].state = Some(&super::super::states::S_STUNRAY1);
        g.objs[shot2].owner = 2;
        g.mp_contact(super::super::keen::keen_contact, p1obj, shot2);
        assert_eq!(g.battle.knocked, [1, 0, 0, 0]);
        assert_ne!(g.objs[shot2].obclass, STUNSHOTOBJ);
        g.mp_tick();
        assert_eq!(g.mp.person, [1, 0, 2, 3]);
        // the leading Keen (player 2) dies
        g.invincible = 0;
        g.kill_keen();
        // shots found are only his
        let a = g.mp.slots[1].vars.ammo;
        g.give_ammo(5);
        assert_eq!(g.mp.slots[1].vars.ammo, a);
        // nobody else alive: he comes back at the start
        let k = 1;
        if let Some(o) = g.mp.slots[k].obj.take() {
            g.remove_obj(o);
        }
        g.playstate = EX_DIED;
        g.battle_guard();
        assert_eq!(g.playstate, EX_STILLPLAYING);
        assert_eq!((g.objs[g.player].x, g.objs[g.player].y), start);
        assert!(!g.keen_dying());
        let _ = std::fs::remove_dir_all(&tmp);
    }

    /// Bonus items come back after 10 seconds.
    #[test]
    fn items_come_back() {
        let tmp = tmpdir("items");
        let Some(mut g) = battle(&tmp, 2, GD_NORMAL) else { return };
        let Some(b) = (0..g.objs.len()).find(|&o| !g.obj_free[o] && g.objs[o].obclass == BONUSOBJ) else { return };
        let (x, y, kind) = (g.objs[b].x, g.objs[b].y, g.objs[b].temp1);
        let p = g.player;
        let score = g.gs.score;
        super::super::keen::keen_contact(&mut g, p, b);
        assert_ne!(g.objs[b].obclass, BONUSOBJ);
        assert!(g.gs.score >= score);
        let count = |g: &Game| (0..g.objs.len()).filter(|&o| !g.obj_free[o] && g.objs[o].obclass == BONUSOBJ && g.objs[o].x == x && g.objs[o].y == y).count();
        g.tics = 70;
        for _ in 0..9 {
            g.battle_tick();
        }
        assert_eq!(count(&g), 0);
        g.battle_tick();
        assert_eq!(count(&g), 1);
        let n = (0..g.objs.len()).find(|&o| !g.obj_free[o] && g.objs[o].obclass == BONUSOBJ && g.objs[o].x == x && g.objs[o].y == y).unwrap();
        assert_eq!(g.objs[n].temp1, kind);
        let _ = std::fs::remove_dir_all(&tmp);
    }

    /// Easy: no monsters. Hard: stunned monsters wake up after 5 seconds,
    /// and the player who stunned one gets points.
    #[test]
    fn monsters_by_difficulty() {
        let tmp = tmpdir("monsters");
        let Some(easy) = battle(&tmp, 2, GD_EASY) else { return };
        assert_eq!(monsters(&easy), 0);
        drop(easy);
        let Some(mut g) = battle(&tmp, 2, GD_HARD) else { return };
        if monsters(&g) == 0 {
            return;
        }
        let m = (0..g.objs.len()).find(|&o| !g.obj_free[o] && g.battle.monsters.get(o).is_some_and(|s| s.is_some())).unwrap();
        let class = g.objs[m].obclass;
        let shot = g.get_new_obj(false);
        g.objs[shot].obclass = STUNSHOTOBJ;
        g.objs[shot].state = Some(&super::super::states::S_STUNRAY1);
        g.objs[shot].owner = 2;
        let score2 = g.mp.slots[1].vars.score;
        // (stunned like StunObj does, in the monster's own state)
        g.battle_stunned(m, shot);
        g.objs[m].temp4 = class;
        g.objs[m].obclass = STUNNEDOBJ;
        assert_eq!(g.mp.slots[1].vars.score, score2 + super::STUNPOINTS);
        g.tics = 70;
        for _ in 0..4 {
            g.battle_tick();
        }
        assert_eq!(g.objs[m].obclass, STUNNEDOBJ);
        g.battle_tick();
        assert_eq!(g.objs[m].obclass, class);
        let _ = std::fs::remove_dir_all(&tmp);
    }

    /// The high score screen from the control panel in the middle of a
    /// game: the game goes on where it was.
    #[test]
    fn high_scores_keep_the_game() {
        let tmp = tmpdir("hs");
        let Some(mut g) = battle(&tmp, 2, GD_NORMAL) else { return };
        g.battle_stop();
        let p = g.player;
        g.objs[p].x += 0x100;
        let (x, score, map) = (g.objs[p].x, g.gs.score + 100, g.gs.mapon);
        g.gs.score = score;
        // (the screen stays until a key is pressed)
        g.sys.script_key(200, 0x39);
        g.high_scores_keeping_game(true).unwrap();
        assert!(g.alive());
        assert!(g.loadedgame);
        assert_eq!((g.objs[g.player].x, g.gs.score, g.gs.mapon), (x, score, map));
        let _ = std::fs::remove_dir_all(&tmp);
    }
}
