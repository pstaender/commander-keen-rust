//! Multiplayer mode (not in the original): up to three more Keens in the
//! levels.
//!
//! Each player plays with his own gamepad (player 1 with the first one);
//! the first player without one plays with the keyboard, so 2 players need
//! a gamepad, 3 two and 4 three. Without a gamepad there is no
//! multiplayer mode. The leading Keen (slot 0, `player`, the first object)
//! is the one the camera follows; the others (slots 1-3) are brought back
//! to him when they stay off screen, and come along through his doors. On
//! the world map only the leading player walks, the others follow him.
//!
//! The ported Keen code works on the globals of "the" player (`player`,
//! `c`, the button states, `gs.score`, `gs.ammo`, `gs.riding`, ...). For
//! everything another Keen does (his thinking, his controls, his contacts
//! and tiles) `with_slot` swaps his set of these globals in, so the
//! original code runs unchanged for every Keen. Keys, gems, lives, drops
//! and the special items stay shared; score and shots are per player (all
//! get the shots found). Stunner shots stun the other Keens: they are dizzy
//! for two seconds.
//!
//! When the leading Keen dies while another one is still alive, that one
//! takes the lead (`mp_handover`): the two Keen objects exchange their
//! contents and their globals, so the living Keen is `player` again (the
//! camera follows him) and the dying one comes back for a shared life.
//! `MpState::person` tells which player plays each slot (controls, colors
//! and score boxes follow the people, not the objects) until the level
//! ends.
//!
//! Never active in demos or with --classic, so the original code paths
//! (and the demo playback) stay as they were.

use super::defs::*;
use super::states::*;
use super::Game;
use crate::engine::input::ControlInfo;

/// most players (Keens)
pub const MAXPLAYERS: usize = 4;
/// tics a Keen hit by another one's stunner is dizzy (2 seconds)
pub const DIZZYTICS: i32 = 140;
/// tics another Keen may stay off screen before he is brought back
const AWAYTICS: i32 = 140;
/// tics after another Keen died before he comes back
const RESPAWNTICS: i32 = 70;
/// a Keen is invincible for a moment after coming back
const RESPAWNINVINCIBLE: i32 = 70;
/// the Keens that don't lead are drawn this much higher (2 pixels), so
/// they are told apart even on the same spot
pub const ALTYOFFSET: i32 = 0x20;
/// world map: distance (global units) between the Keens walking behind
/// the leading one
const WORLDFOLLOW: i32 = 0x180;
/// world map: Keen 1 moved further than this in one frame (teleporter,
/// elevator): the others jump along
const WORLDJUMP: i32 = 0x300;

/// The per player globals swapped by `Game::mp_swap`.
#[derive(Clone, Default)]
pub struct PlayerVars {
    pub player: usize,
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
    pub invincible: i32,
    pub lastpoletime: i64,
    pub score: i32,
    pub nextextra: i32,
    pub ammo: i32,
    pub riding: Option<usize>,
}

/// One of the Keens that don't lead (slots 1-3).
#[derive(Clone, Default)]
pub struct Slot {
    /// his Keen object in the current level
    pub obj: Option<usize>,
    /// the globals of the player of this slot while they are not swapped
    /// in (inside `with_slot` the leading player's)
    pub vars: PlayerVars,
    /// he died: tics until he comes back (0: not waiting)
    pub respawn: i32,
    /// tics he has been off screen
    pub away: i32,
    /// he lost the last life: he is out until the next level
    pub out: bool,
}

#[derive(Clone)]
pub struct MpState {
    /// number of players chosen (1: off; EXTENDED OPTIONS > PLAYERS or
    /// CONFIGURE > USE GAMEPAD, config.txt `players`)
    pub players: usize,
    /// the slots; [0] is unused (the leading Keen's globals are the
    /// game's own)
    pub slots: [Slot; MAXPLAYERS],
    /// the slot whose globals are swapped in (0: the leading Keen's)
    pub cur: usize,
    /// tics the Keen of each slot stays dizzy
    pub dizzy: [i32; MAXPLAYERS],
    /// the player (0 = player 1) of each slot: the same as the slot until
    /// a Keen takes the lead (until the level ends); a permutation
    pub person: [usize; MAXPLAYERS],
    /// world map: the path of the leading Keen (x, y, shape), newest last;
    /// the others (only sprites there) walk along it
    pub trail: Vec<(i32, i32, i32)>,
    /// world map: the direction (dir_* id) of each follower, for his
    /// standing shape
    pub world_dir: [i32; MAXPLAYERS],
}

impl Default for MpState {
    fn default() -> Self {
        MpState {
            players: 1,
            slots: Default::default(),
            cur: 0,
            dizzy: [0; MAXPLAYERS],
            person: [0, 1, 2, 3],
            trail: Vec::new(),
            world_dir: [4; MAXPLAYERS],
        }
    }
}

impl MpState {
    /// More than one player chosen.
    pub fn enabled(&self) -> bool {
        self.players >= 2
    }

    /// A Keen took the lead: the slots are played by other people.
    pub fn flipped(&self) -> bool {
        self.person != [0, 1, 2, 3]
    }
}

impl Game {
    /// Multiplayer mode can be played: chosen, a gamepad is connected, not
    /// in demos and not with --classic.
    pub fn mp_available(&self) -> bool {
        self.mp.enabled() && !self.opts.classic && self.demomode == 0 && !self.storedemo && self.sys.input.pad_connected
    }

    /// Number of players playing: as chosen, but at most one more than
    /// there are gamepads (the keyboard); 1 when not available.
    pub fn mp_count(&self) -> usize {
        if !self.mp_available() {
            return 1;
        }
        self.mp.players.min(self.sys.input.pad_count.max(1) + 1).clamp(2, MAXPLAYERS)
    }

    /// The next number of players in the menus: 1, 2, ... as many as the
    /// gamepads allow, then 1 again.
    pub fn mp_next_players(&mut self) {
        let max = self.mp_max_players();
        self.mp.players = if self.mp.players >= max { 1 } else { (self.mp.players + 1).max(2) };
    }

    /// Multiplayer: the controls are split between the players (each his
    /// gamepad, the last one the keyboard if there are fewer gamepads); on
    /// the world map too, where only the leading player walks.
    pub fn mp_split_input(&mut self) {
        let world = self.ca_mapon == 0;
        self.sys.input.players = if self.mp_in_level() || self.mp.flipped() || world { self.mp_count() } else { 1 };
    }

    /// The most players the connected gamepads allow (the keyboard plays
    /// too): 1 without a gamepad.
    pub fn mp_max_players(&self) -> usize {
        if !self.sys.input.pad_connected || self.opts.classic {
            1
        } else {
            (self.sys.input.pad_count.max(1) + 1).min(MAXPLAYERS)
        }
    }

    /// The Keens that don't lead and take part in the current level (alive,
    /// dying or waiting to come back).
    fn mp_slot_active(&self, k: usize) -> bool {
        k >= 1 && k < MAXPLAYERS && (self.mp.slots[k].obj.is_some() || self.mp.slots[k].respawn > 0)
    }

    /// Another Keen takes part in the current level.
    pub fn mp_in_level(&self) -> bool {
        (1..MAXPLAYERS).any(|k| self.mp_slot_active(k))
    }

    /// The Keen object of slot `k`, whatever is swapped in.
    pub fn mp_slot_obj(&self, k: usize) -> Option<usize> {
        let cur = self.mp.cur;
        if k == 0 {
            Some(if cur == 0 { self.player } else { self.mp.slots[cur].vars.player })
        } else if k == cur {
            Some(self.player)
        } else {
            self.mp.slots[k].obj
        }
    }

    /// Slot of a Keen object, None for other objects.
    pub fn mp_player_of(&self, ob: usize) -> Option<usize> {
        (0..MAXPLAYERS).find(|&k| self.mp_slot_obj(k) == Some(ob))
    }

    /// The first object of the list (the leading Keen's), also inside
    /// `with_slot`.
    pub fn mp_first_obj(&self) -> usize {
        self.mp_slot_obj(0).unwrap_or(self.player)
    }

    /// The player (0 = player 1) of the Keen object `ob`, for his colors;
    /// None for other objects.
    pub fn mp_person_of(&self, ob: usize) -> Option<usize> {
        self.mp_player_of(ob).map(|k| self.mp.person[k])
    }

    /// New game: the other players start like player 1 (NewGame).
    pub fn mp_new_game(&mut self) {
        self.mp.person = [0, 1, 2, 3];
        for s in self.mp.slots.iter_mut() {
            s.vars.score = 0;
            s.vars.nextextra = 20000;
            s.vars.ammo = 5;
            s.out = false;
        }
    }

    /// Two gamepads at launch: multiplayer mode on, with 2 players (not
    /// with --classic).
    pub fn mp_auto_enable(&mut self) {
        if std::mem::take(&mut self.sys.input.two_pads_at_start) && !self.opts.classic && !self.mp.enabled() {
            self.mp.players = 2;
        }
    }

    /// Exchanges the leading player's globals with those of slot `k`.
    fn mp_swap_vars(&mut self, k: usize) {
        let o = &mut self.mp.slots[k].vars;
        std::mem::swap(&mut self.player, &mut o.player);
        std::mem::swap(&mut self.c, &mut o.c);
        std::mem::swap(&mut self.jumpbutton, &mut o.jumpbutton);
        std::mem::swap(&mut self.jumpheld, &mut o.jumpheld);
        std::mem::swap(&mut self.pogobutton, &mut o.pogobutton);
        std::mem::swap(&mut self.pogoheld, &mut o.pogoheld);
        std::mem::swap(&mut self.firebutton, &mut o.firebutton);
        std::mem::swap(&mut self.fireheld, &mut o.fireheld);
        std::mem::swap(&mut self.upheld, &mut o.upheld);
        std::mem::swap(&mut self.jumptime, &mut o.jumptime);
        std::mem::swap(&mut self.pogodelay, &mut o.pogodelay);
        std::mem::swap(&mut self.invincible, &mut o.invincible);
        std::mem::swap(&mut self.lastpoletime, &mut o.lastpoletime);
        std::mem::swap(&mut self.gs.score, &mut o.score);
        std::mem::swap(&mut self.gs.nextextra, &mut o.nextextra);
        std::mem::swap(&mut self.gs.ammo, &mut o.ammo);
        std::mem::swap(&mut self.gs.riding, &mut o.riding);
    }

    /// Exchanges the per player globals (score, shots, buttons, ...) of
    /// the leading Keen and slot `k`, but not the objects.
    fn mp_exchange_vars(&mut self, k: usize) {
        let (p, op) = (self.player, self.mp.slots[k].vars.player);
        self.mp_swap_vars(k);
        self.player = p;
        self.mp.slots[k].vars.player = op;
        self.mp.person.swap(0, k);
    }

    /// Back to every player in his own slot (a new level, the world map):
    /// the globals go back to their players, the objects stay.
    fn mp_unflip(&mut self) {
        while let Some(k) = (1..MAXPLAYERS).find(|&k| self.mp.person[k] != k) {
            // the slot holding this slot's player
            let j = self.mp.person.iter().position(|&p| p == k).unwrap_or(k);
            if j == 0 {
                self.mp_exchange_vars(k);
            } else {
                let (a, b) = (self.mp.slots[j].vars.clone(), self.mp.slots[k].vars.clone());
                self.mp.slots[j].vars = b;
                self.mp.slots[k].vars = a;
                self.mp.person.swap(j, k);
            }
        }
        if self.mp.person[0] != 0 {
            // (only slot 0 left: nothing to do, the permutation is whole)
            self.mp.person = [0, 1, 2, 3];
        }
    }

    /// The leading Keen died (or fell out of the map with `force`) while
    /// another one is alive: the first of them takes the lead. The two
    /// Keen objects exchange everything but their place in the object list
    /// (the first one stays `player`, which the original code relies on),
    /// the per player globals and dizziness. The slot that got the dying
    /// Keen, if done.
    pub fn mp_handover(&mut self, force: bool) -> Option<usize> {
        if self.mp.cur != 0 {
            return None;
        }
        let p1 = self.player;
        let s = self.objs[p1].state;
        if !force && !(same(s, &S_KEENDIE1) || same(s, &S_KEENDIE2) || same(s, &S_KEENDEAD)) {
            return None;
        }
        let k = (1..MAXPLAYERS).find(|&k| {
            self.mp.slots[k].obj.is_some_and(|o| !self.keen_obj_dying(o) && self.objs[o].obclass == KEENOBJ)
        })?;
        self.mp_handover_to(k);
        Some(k)
    }

    /// The Keen of slot `k` takes the lead (see `mp_handover`).
    fn mp_handover_to(&mut self, k: usize) {
        let (p1, pk) = (self.player, self.mp.slots[k].obj.unwrap());
        let (a, b) = (self.objs[p1].clone(), self.objs[pk].clone());
        self.objs[p1] = super::defs::Obj { prev: a.prev, next: a.next, ..b };
        self.objs[pk] = super::defs::Obj { prev: b.prev, next: b.next, ..a };
        self.rf.sprites.swap(p1, pk);
        self.rf.stars.swap(p1, pk);
        self.mp_exchange_vars(k);
        self.mp.dizzy.swap(0, k);
        self.mp.slots[k].away = 0;
        // (KillKeen locked the camera on the dying Keen)
        self.scrolllock = false;
    }

    /// From the play loop: the first Keen fell out of the map. With
    /// another Keen alive he takes the lead and the fallen one costs a
    /// life like the others do; false if the level is lost.
    pub fn mp_leader_fell(&mut self) -> bool {
        let Some(k) = self.mp_handover(true) else { return false };
        self.mp_slot_died(k);
        true
    }

    /// Swaps the globals of slot `k` in or out (`cur` 0 <-> k).
    fn mp_swap(&mut self, k: usize) {
        self.mp_swap_vars(k);
        self.mp.cur = if self.mp.cur == k { 0 } else { k };
    }

    /// Runs `f` as the Keen of slot `k` (his globals swapped in). If he died
    /// in it (KeenDieThink sets EX_DIED) the level goes on without him.
    pub fn with_slot<R>(&mut self, k: usize, f: impl FnOnce(&mut Game) -> R) -> R {
        let prev = self.mp.cur;
        if prev == k {
            return f(self);
        }
        if prev != 0 {
            self.mp_swap(prev);
        }
        let ps = self.playstate;
        if k != 0 {
            // (his object: `player` while his globals are swapped in)
            if let Some(o) = self.mp.slots[k].obj {
                self.mp.slots[k].vars.player = o;
            }
            self.mp_swap(k);
        }
        let r = f(self);
        if k != 0 {
            self.mp_swap(k);
            if ps != EX_DIED && self.playstate == EX_DIED {
                self.playstate = ps;
                self.mp_slot_died(k);
            }
        }
        if prev != 0 {
            if let Some(o) = self.mp.slots[prev].obj {
                self.mp.slots[prev].vars.player = o;
            }
            self.mp_swap(prev);
        }
        r
    }

    /// The slot (1-3) of the Keen object `ob` if it is one of the Keens that
    /// don't lead and nothing is swapped in.
    pub fn mp_extra_slot(&self, ob: usize) -> Option<usize> {
        if self.mp.cur != 0 {
            return None;
        }
        (1..MAXPLAYERS).find(|&k| self.mp.slots[k].obj == Some(ob))
    }

    /// The Keens that don't lead, with their slots.
    pub fn mp_extra_objs(&self) -> Vec<(usize, usize)> {
        (1..MAXPLAYERS).filter_map(|k| self.mp.slots[k].obj.map(|o| (k, o))).collect()
    }

    /// Calls a contact function, as another Keen if it is about him.
    pub fn mp_contact(&mut self, c: ContactFn, ob: usize, hit: usize) {
        if let Some(k) = self.mp_extra_slot(ob) {
            self.with_slot(k, |g| c(g, ob, hit))
        } else if let Some(k) = self.mp_extra_slot(hit).filter(|_| ob != self.player) {
            self.with_slot(k, |g| c(g, ob, hit))
        } else {
            c(self, ob, hit)
        }
    }

    /// The current Keen is dizzy: his controls do nothing.
    pub fn mp_dizzy(&self) -> bool {
        (self.mp_in_level() || self.mp.cur != 0) && self.mp.dizzy[self.mp.cur] > 0
    }

    /// Shots found: every player gets them.
    pub fn give_ammo(&mut self, n: i32) {
        self.gs.ammo += n;
        if self.mp_in_level() || self.mp.cur != 0 {
            for k in 1..MAXPLAYERS {
                if k == self.mp.cur || self.mp_slot_active(k) || self.mp.slots[k].out {
                    self.mp.slots[k].vars.ammo += n;
                }
            }
        }
    }

    /// Owner of a new stunner shot (the player number 1-4; 0 = single
    /// player).
    pub fn mp_shot_owner(&self) -> u8 {
        if self.mp_in_level() || self.mp.cur != 0 {
            self.mp.person[self.mp.cur] as u8 + 1
        } else {
            0
        }
    }

    /// From KeenContact: a stunner shot of another player makes Keen dizzy.
    /// True if the shot was used up.
    pub fn mp_shot_hits_keen(&mut self, ob: usize, hit: usize) -> bool {
        if self.objs[hit].obclass != STUNSHOTOBJ || !(self.mp_in_level() || self.mp.cur != 0) {
            return false;
        }
        let owner = self.objs[hit].owner as usize;
        let Some(k) = self.mp_player_of(ob) else { return false };
        if owner == 0 || owner == self.mp.person[k] + 1 {
            return false;
        }
        self.explode_shot(hit);
        self.mp.dizzy[k] = DIZZYTICS;
        self.play_sound(super::keen::SND_HELMETHIT);
        self.mp_knock_off_pogo(k, ob);
        true
    }

    // ---- levels ---------------------------------------------------------

    fn mp_reset_slots(&mut self) {
        for s in self.mp.slots.iter_mut() {
            s.obj = None;
            s.respawn = 0;
            s.away = 0;
            s.out = false;
            s.vars.riding = None;
            s.vars.invincible = 0;
        }
        self.mp.dizzy = [0; MAXPLAYERS];
        self.mp.trail.clear();
    }

    /// After ScanInfoPlane: the other players start where player 1 does
    /// (not on the world map and not where Keen doesn't start on his feet,
    /// like Keen 4's underwater level).
    pub fn mp_setup_level(&mut self) {
        self.mp_auto_enable();
        self.mp_unflip();
        self.mp_reset_slots();
        if !self.mp_available() || self.gs.mapon == 0 {
            return;
        }
        let p = self.player;
        if self.objs[p].obclass != KEENOBJ || !same(self.objs[p].state, &S_KEENSTAND) {
            return;
        }
        for k in 1..self.mp_count() {
            self.mp_spawn(k, &S_KEENSTAND);
        }
    }

    /// A loaded game: the other Keen objects from the saved game (new
    /// ones next to player 1 for the players it doesn't have), none in one
    /// player mode.
    pub fn mp_after_load(&mut self, objs: [Option<usize>; MAXPLAYERS]) {
        let outs: Vec<bool> = self.mp.slots.iter().map(|s| s.out).collect();
        self.mp_reset_slots();
        let n = self.mp_count();
        let mut valid = [None; MAXPLAYERS];
        for k in 1..MAXPLAYERS {
            valid[k] = objs[k].filter(|&o| o != self.player && !self.obj_free[o] && self.objs[o].obclass == KEENOBJ);
        }
        if !self.mp_available() || self.gs.mapon == 0 {
            for o in valid.iter().flatten() {
                self.remove_obj(*o);
            }
            self.mp_unflip();
            return;
        }
        // (players beyond the number playing now leave)
        for k in 1..MAXPLAYERS {
            if self.mp.person[k] >= n {
                if let Some(o) = valid[k].take() {
                    self.remove_obj(o);
                }
            }
        }
        if self.mp.person[0] >= n {
            self.mp_unflip();
        }
        let p = self.player;
        for k in 1..MAXPLAYERS {
            if self.mp.person[k] >= n {
                continue;
            }
            self.mp.slots[k].out = outs[k];
            match valid[k] {
                Some(o) => self.mp.slots[k].obj = Some(o),
                None if !outs[k] && self.objs[p].obclass == KEENOBJ && self.objs[p].needtoclip != CL_NOCLIP => {
                    self.mp_spawn(k, &S_KEENFALL1);
                }
                None => {}
            }
        }
    }

    /// A new Keen object for slot `k` at the leading Keen's position.
    fn mp_spawn(&mut self, k: usize, st: StateRef) -> bool {
        let n = self.get_new_obj(true);
        if n == DUMMYOBJ {
            return false;
        }
        let p = self.objs[self.player].clone();
        {
            let o = &mut self.objs[n];
            o.obclass = KEENOBJ;
            o.active = AC_ALLWAYS;
            o.priority = 1;
            o.x = p.x;
            o.y = p.y;
            o.xdir = if p.xdir == 0 { 1 } else { p.xdir };
            o.ydir = 1;
        }
        let s = &mut self.mp.slots[k];
        s.obj = Some(n);
        s.away = 0;
        s.respawn = 0;
        self.mp.dizzy[k] = 0;
        self.with_slot(k, |g| {
            g.jumptime = 0;
            g.gs.riding = None;
            g.new_state(n, st);
        });
        true
    }

    /// The number of players changed in the middle of a level (in the
    /// control panel): the players beyond it leave at once (if one of them
    /// leads, a remaining player gets the lead back); new ones join next to
    /// the leading Keen.
    fn mp_apply_mode(&mut self) {
        if self.mp.cur != 0 || self.ca_mapon == 0 {
            return;
        }
        let n = self.mp_count();
        if self.mp.person[0] >= n {
            // a remaining player's Keen alive: he leads; otherwise only the
            // globals go back and player 1 plays on with the leading Keen
            let alive = (1..MAXPLAYERS).find(|&k| {
                self.mp.person[k] < n
                    && self.mp.slots[k].obj.is_some_and(|o| !self.keen_obj_dying(o) && self.objs[o].obclass == KEENOBJ)
            });
            match alive {
                Some(k) => self.mp_handover_to(k),
                None => self.mp_unflip(),
            }
        }
        for k in 1..MAXPLAYERS {
            if self.mp.person[k] < n {
                continue;
            }
            if let Some(o) = self.mp.slots[k].obj.take() {
                self.remove_obj(o);
            }
            let s = &mut self.mp.slots[k];
            s.respawn = 0;
            s.away = 0;
            s.out = false;
            s.vars.riding = None;
            s.vars.invincible = 0;
            self.mp.dizzy[k] = 0;
        }
        // joining: like after loading a game without them (not in Keen 4's
        // underwater level, where Keen swims)
        let p = self.player;
        if n < 2 || (super::EPISODE == 4 && self.ca_mapon == 0x11) || self.objs[p].obclass != KEENOBJ || self.objs[p].needtoclip == CL_NOCLIP || self.keen_dying() {
            return;
        }
        for k in 1..MAXPLAYERS {
            if self.mp.person[k] < n && !self.mp_slot_active(k) && !self.mp.slots[k].out && self.mp_spawn(k, &S_KEENFALL1) {
                self.mp.slots[k].vars.invincible = RESPAWNINVINCIBLE;
            }
        }
    }

    /// The Keen of slot `k` died (fell off the screen dying, or out of the
    /// map): he comes back next to the leading Keen for one of the shared
    /// lives.
    fn mp_slot_died(&mut self, k: usize) {
        let Some(n) = self.mp.slots[k].obj.take() else { return };
        self.remove_obj(n);
        self.mp.dizzy[k] = 0;
        let unlimited = self.unlimited_lives();
        let s = &mut self.mp.slots[k];
        s.vars.riding = None;
        if unlimited {
            s.respawn = RESPAWNTICS;
        } else if self.gs.lives > 0 {
            self.gs.lives -= 1;
            s.respawn = RESPAWNTICS;
        } else {
            s.out = true;
        }
    }

    /// Brings the Keen of slot `k` back to the leading Keen (falling, so he
    /// lands safely): when he was off screen too long, and through the
    /// leading Keen's doors.
    fn mp_warp(&mut self, k: usize) {
        let Some(ob) = self.mp.slots[k].obj else { return };
        let p = self.objs[self.player].clone();
        self.with_slot(k, |g| {
            let o = &mut g.objs[ob];
            o.x = p.x;
            o.y = p.y;
            o.xspeed = 0;
            o.yspeed = 0;
            o.priority = 1;
            g.gs.riding = None;
            g.jumptime = 0;
            // without clipping, like KeenEnterThink: ChangeState clips the
            // move from the old position, which would stop him on the
            // first floor between there and the leading Keen
            g.objs[ob].needtoclip = CL_NOCLIP;
            g.change_state(ob, &S_KEENFALL1);
            g.objs[ob].needtoclip = CL_MIDCLIP;
        });
        self.mp.slots[k].away = 0;
    }

    /// The leading Keen went through a door (KeenEnterThink, also to
    /// another spot of the same map): the others come along. (Their own
    /// doors take only them; when they stay off screen they are brought
    /// back.)
    pub fn mp_follow_door(&mut self) {
        if self.mp.cur != 0 {
            return;
        }
        for (k, o) in self.mp_extra_objs() {
            if !self.keen_obj_dying(o) {
                self.mp_warp(k);
            }
        }
    }

    /// From the play loop, after the reactions: the reaction of another
    /// Keen (or his end when he fell out of the map).
    pub fn mp_react_extra(&mut self, k: usize, ob: usize) {
        if (self.objs[ob].tilebottom as u16) < (self.mapheight - 1) as u16 {
            if self.objs[ob].needtoreact {
                if let Some(r) = self.objs[ob].state.and_then(|s| s.react) {
                    self.objs[ob].needtoreact = false;
                    self.with_slot(k, |g| r(g, ob));
                }
            }
        } else {
            self.mp_slot_died(k);
        }
    }

    /// Once per frame after scrolling: what ScrollScreen does for the
    /// leading Keen (falling out of the map, leaving it at the sides) for
    /// the others, bringing them back when they are off screen too long,
    /// coming back after dying, dizziness.
    pub fn mp_tick(&mut self) {
        self.mp_apply_mode();
        // the leading Keen dies: another one goes on
        if self.mp.cur == 0 && !self.mp_extra_objs().is_empty() && self.keen_dying() {
            self.mp_handover(false);
        }
        if !self.mp_in_level() {
            return;
        }
        let tics = self.tics;
        for k in 1..MAXPLAYERS {
            if !self.mp_slot_active(k) {
                continue;
            }
            let s = &mut self.mp.slots[k];
            s.vars.invincible = (s.vars.invincible - tics).max(0);
            if s.respawn > 0 {
                s.respawn -= tics;
                let p = self.player;
                if s.respawn <= 0 {
                    s.respawn = 1;
                    if !self.keen_dying() && self.objs[p].needtoclip != CL_NOCLIP && self.mp_spawn(k, &S_KEENFALL1) {
                        self.mp.slots[k].vars.invincible = RESPAWNINVINCIBLE;
                    }
                }
            }
            let Some(ob) = self.mp.slots[k].obj else { continue };
            let o = self.objs[ob].clone();
            let c = self.cam.clone();
            let dying = self.keen_obj_dying(ob);
            let right = if c.wide() { (self.mapwidth - 2) * 256 } else { c.originxmax + 0x1400 };
            if !dying && ((o.left as u16) < c.originxmin as u16 || (right as u16) < o.right as u16) {
                self.playstate = EX_COMPLETED;
            } else if !dying && (self.fall_line() as u16) < o.bottom as u16 {
                self.objs[ob].y -= o.bottom - self.fall_line();
                self.play_sound(super::keen::SND_PLUMMET);
                self.with_slot(k, |g| {
                    let god = std::mem::replace(&mut g.godmode, false);
                    g.invincible = 0;
                    g.kill_keen();
                    g.godmode = god;
                });
            } else if !dying && !self.on_screen(ob) && !self.scrolllock {
                self.mp.slots[k].away += tics;
                let p = self.player;
                if self.mp.slots[k].away > AWAYTICS && !self.keen_dying() && self.objs[p].needtoclip != CL_NOCLIP {
                    self.mp_warp(k);
                }
            } else {
                self.mp.slots[k].away = 0;
            }
        }
        // the stun stars over a dizzy Keen's head
        let frame = (self.lasttimecount / 10) % 3;
        for k in 0..MAXPLAYERS {
            let Some(ob) = (if k == 0 { Some(self.player) } else { self.mp.slots[k].obj }) else { continue };
            if self.mp.dizzy[k] > 0 {
                self.mp.dizzy[k] = (self.mp.dizzy[k] - tics).max(0);
                self.mp_knock_off_pogo(k, ob);
                // on the ground he sits dazed, the stars lower
                let down = if self.mp_sit_dizzy(ob) { 0x80 } else { 0 };
                let (x, y) = (self.objs[ob].x + 0x20, self.objs[ob].y - 0x90 + down);
                self.place_stars(ob, x, y, super::ep::STARSPR + frame, 3);
            } else {
                self.remove_stars(ob);
            }
        }
    }

    /// World map (multiplayer mode): the other Keens follow the leading
    /// one's path, one behind the other. Only sprites: they do nothing
    /// there, and saved games, demos and the map logic don't know them.
    pub fn mp_world_tick(&mut self) {
        self.mp_auto_enable();
        let n = self.mp_count();
        if self.ca_mapon != 0 || n < 2 {
            self.rf.extra.clear();
            return;
        }
        let p1 = self.rf.sprites.get(self.player).copied().unwrap_or_default();
        if !p1.visible || p1.shape == 0 {
            self.rf.extra.clear();
            return;
        }
        let dist = |a: (i32, i32), b: (i32, i32)| (a.0 - b.0).abs().max((a.1 - b.1).abs());
        let t = &mut self.mp.trail;
        let moved = t.last().is_none_or(|l| (l.0, l.1) != (p1.x, p1.y));
        if t.last().is_some_and(|l| dist((l.0, l.1), (p1.x, p1.y)) > WORLDJUMP) {
            t.clear();
        }
        if moved {
            t.push((p1.x, p1.y, p1.shape));
            if t.len() > 1024 {
                t.drain(..512);
            }
        }
        let st = self.objs[self.player].state;
        let walking = same(st, &S_WORLDKEENWALK);
        let mut extra = Vec::new();
        for k in 1..n {
            // the point k * WORLDFOLLOW back along the path (the oldest
            // one while the path is shorter)
            let t = &self.mp.trail;
            let mut i = t.len() - 1;
            let mut walked = 0;
            while i > 0 && walked < WORLDFOLLOW * k as i32 {
                walked += dist((t[i].0, t[i].1), (t[i - 1].0, t[i - 1].1));
                i -= 1;
            }
            let (x, y, mut shape) = t[i];
            // walking Keen: the others look where they go, and stand when
            // he stops
            if walking || same(st, &S_WORLDKEEN) {
                if let Some(&(nx, ny, _)) = t.get(i + 1) {
                    let sx = if nx - x > 0x10 { 1 } else if x - nx > 0x10 { -1 } else { 0 };
                    let sy = if ny - y > 0x10 { 1 } else if y - ny > 0x10 { -1 } else { 0 };
                    let d = match (sx, sy) {
                        (0, -1) => 0,
                        (1, -1) => 1,
                        (1, 0) => 2,
                        (1, 1) => 3,
                        (0, 1) => 4,
                        (-1, 1) => 5,
                        (-1, 0) => 6,
                        (-1, -1) => 7,
                        _ => self.mp.world_dir[k],
                    };
                    self.mp.world_dir[k] = d;
                }
                if !walking || !moved {
                    shape = super::keen2::WORLDSHAPES[self.mp.world_dir[k].clamp(0, 7) as usize] + 3;
                }
            }
            // below the leading Keen (drawn before him), the last one first
            extra.push(super::rf::SpriteSlot {
                visible: true,
                x,
                y: y - ALTYOFFSET * k as i32,
                shape,
                priority: p1.priority,
                order: p1.order - k as i64,
                white: false,
                alt: k as u8,
                stunned: false,
                depth: k as u8,
            });
        }
        self.rf.extra = extra;
    }

    /// A dizzy Keen falls off his pogo stick (and sits dazed once he
    /// lands). `k`: the slot of the object `ob`.
    fn mp_knock_off_pogo(&mut self, k: usize, ob: usize) {
        let Some(st) = self.objs[ob].state else { return };
        if !(same(Some(st), &S_KEENPOGO) || same(Some(st), &S_KEENPOGO2) || same(Some(st), &S_KEENPOGODOWN)) {
            return;
        }
        self.with_slot(k, move |g| {
            g.jumptime = 0;
            g.objs[ob].xspeed = 0;
            g.change_state(ob, &S_KEENFALL1);
        });
    }

    /// A dizzy Keen standing on the ground (or idling: the idle
    /// animations start over) sits dazed: Keen 6's stunned Keen sprite
    /// (Keen 4 and 5 borrow it from the Keen 6 files if they are there).
    fn mp_sit_dizzy(&mut self, ob: usize) -> bool {
        let Some(st) = self.objs[ob].state else { return false };
        // standing or any of the idle animations (looking around, waiting,
        // sitting down, reading, Keen 4's mooning; they differ per episode)
        let idle = st.name == "s_keenstand"
            || ["s_keenpause", "s_keenwait", "s_keensit", "s_keenread", "s_keenmoon"].iter().any(|p| st.name.starts_with(p));
        if !idle || self.objs[ob].hitnorth == 0 || !self.mp_load_stunned() {
            return false;
        }
        if !std::ptr::eq(st, &S_KEENSTAND) {
            self.change_state(ob, &S_KEENSTAND);
        }
        if let Some(s) = self.rf.sprites.get_mut(ob) {
            s.stunned = true;
        }
        true
    }

    /// Makes `Gfx::stunned` (once): Keen 6's own sprite, or for Keen 4
    /// and 5 the one of the Keen 6 files in a KEEN6 / Keen6 / keen6
    /// directory next to the data directory. False if there is none.
    fn mp_load_stunned(&mut self) -> bool {
        use crate::engine::gfx::{alt_tex, StunSprite};
        use crate::engine::video::Tex;
        if let Some(s) = &self.gfx.stunned {
            return s.is_some();
        }
        let idx = (crate::ck6::states::S_KEENSTUNNED.left - crate::ck6::consts::STARTSPRITES as i32) as usize;
        let scale = self.gfx.scale;
        let found = if super::EPISODE == 6 {
            let t = &self.gfx.sprites[idx];
            let info = self.data.gfx.spritetable[idx];
            let tex = Tex { w: t.w, h: t.h, px: t.px.clone(), has_alpha: t.has_alpha };
            Some((tex, info))
        } else {
            let base = self.data.dir.parent().map(|p| p.to_path_buf()).unwrap_or_default();
            ["KEEN6", "Keen6", "keen6"].iter().find_map(|n| {
                let d = crate::assets::GameData::load(&base.join(n)).ok()?;
                if d.episode != crate::assets::Episode::K6 {
                    return None;
                }
                let img = d.gfx.sprites.get(idx)?;
                Some((Tex::from_image(img, scale), *d.gfx.spritetable.get(idx)?))
            })
        };
        self.gfx.stunned = Some(found.map(|(tex, info)| StunSprite {
            alt: [alt_tex(&tex, 2), alt_tex(&tex, 3), alt_tex(&tex, 4)],
            tex,
            orgx: info.orgx as i32,
            orgy: info.orgy as i32,
        }));
        self.gfx.stunned.as_ref().is_some_and(|s| s.is_some())
    }

    /// The score and shots of player `n` (0 = player 1) for his score
    /// box: None if he isn't in this level (on the world map every player
    /// playing is shown).
    pub fn mp_status(&self, n: usize) -> Option<(i32, i32)> {
        let k = self.mp.person.iter().position(|&p| p == n)?;
        if k == 0 {
            return Some((self.gs.score, self.gs.ammo));
        }
        let world = self.ca_mapon == 0 && n < self.mp_count();
        if !self.mp_slot_active(k) && !self.mp.slots[k].out && !world {
            return None;
        }
        let o = &self.mp.slots[k].vars;
        Some((o.score, o.ammo))
    }
}

#[cfg(test)]
mod tests {
    use super::super::defs::*;
    use super::super::states::*;
    use super::super::Game;
    use crate::assets::GameData;
    use crate::engine::{gfx::Gfx, sound::SoundManager, sys::Sys};

    /// A headless game of this episode in level 1 with two players, its
    /// saves in a temporary directory; None without the game data.
    fn two_player_game(tmp: &std::path::Path) -> Option<Game> {
        let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join(format!("Keen{}", super::super::EPISODE));
        let Ok(data) = GameData::load(&dir) else {
            eprintln!("{}: no game data, skipped", dir.display());
            return None;
        };
        let gfx = Gfx::new(&data.gfx, 1, None);
        let sd = SoundManager::new(&data.audio, false, None, None);
        let sys = Sys::new(1, 1, true, true, false);
        // (extended mode is on unless --classic)
        let opts = crate::options::Options { cheatmode: true, ..Default::default() };
        let mut g = Game::new(sys, data, gfx, sd, opts);
        g.data.dir = tmp.to_path_buf();
        g.data.write_dir = tmp.to_path_buf();
        g.sys.input.pad_connected = true;
        g.mp.players = 2;
        g.new_game();
        g.gs.mapon = 1;
        assert!(g.setup_game_level(false));
        Some(g)
    }

    /// A headless game in level 1 with `n` players and `pads` gamepads.
    fn n_player_game(tmp: &std::path::Path, n: usize, pads: usize) -> Option<Game> {
        let mut g = two_player_game(tmp)?;
        g.sys.input.pad_count = pads;
        g.mp.players = n;
        g.new_game();
        g.gs.mapon = 1;
        assert!(g.setup_game_level(false));
        Some(g)
    }

    /// Four players (three gamepads and the keyboard): three more Keens in
    /// their own colors and controls, the lead going to the next living
    /// one, saved and loaded, fewer players in the middle of a level.
    #[test]
    fn four_players() {
        let tmp = std::env::temp_dir().join(format!("keen-mp4-{}-{}", super::super::EPISODE, std::process::id()));
        let _ = std::fs::remove_dir_all(&tmp);
        std::fs::create_dir_all(&tmp).unwrap();
        let Some(mut g) = n_player_game(&tmp, 4, 3) else { return };
        assert_eq!(g.mp_count(), 4);
        assert_eq!(g.gs.lives, 12);
        let objs: Vec<usize> = (1..4).map(|k| g.mp.slots[k].obj.expect("Keen spawned")).collect();
        assert_eq!(g.mp_extra_objs().len(), 3);
        for (k, &o) in objs.iter().enumerate() {
            g.place_sprite(o);
            assert_eq!(g.rf.sprites[o].alt as usize, k + 1);
            assert_eq!(g.rf.sprites[o].depth as usize, k + 1);
            assert_eq!(g.rf.sprites[o].y, g.objs[o].y - super::ALTYOFFSET * (k as i32 + 1));
        }
        // controls: gamepads 1-3, the keyboard plays the fourth Keen
        g.sys.input.players = 4;
        assert_eq!(g.sys.input.keyboard_player(), 3);
        g.sys.input.keyboard[g.sys.input.key_def.button0 as usize] = true;
        g.sys.input.pads[1].pogo = true;
        g.poll_controls();
        assert!(!g.jumpbutton && !g.pogobutton);
        assert!(g.mp.slots[1].vars.pogobutton && !g.mp.slots[1].vars.jumpbutton);
        assert!(!g.mp.slots[2].vars.jumpbutton);
        assert!(g.mp.slots[3].vars.jumpbutton);
        g.sys.input.keyboard = [false; 128];
        g.sys.input.pads[1].pogo = false;
        // every Keen gets the shots found, each keeps his own score
        let ammo = g.gs.ammo;
        g.give_ammo(5);
        assert!((1..4).all(|k| g.mp.slots[k].vars.ammo == ammo + 5));
        g.mp.slots[3].vars.score = 400;
        // the leading Keen dies: the first living one (slot 1) takes over
        let x2 = g.objs[objs[0]].x;
        g.kill_keen();
        g.mp_tick();
        assert_eq!(g.mp.person, [1, 0, 2, 3]);
        assert_eq!(g.objs[g.player].x, x2);
        // saved and loaded: the same players in the same slots
        g.save_state();
        g.mp.person = [0, 1, 2, 3];
        g.load_state();
        assert_eq!(g.mp.person, [1, 0, 2, 3]);
        assert_eq!(g.mp.slots[3].vars.score, 400);
        assert!((1..4).all(|k| g.mp.slots[k].obj.is_some() || g.mp.slots[k].respawn > 0));
        // two players now: players 3 and 4 leave, player 2 keeps leading
        g.mp.players = 2;
        g.mp_tick();
        assert!(g.mp.slots[2].obj.is_none() && g.mp.slots[3].obj.is_none());
        assert_eq!(g.mp.person[0], 1);
        // the next level: everybody in his own slot again
        g.mp.players = 4;
        g.mp_setup_level();
        assert_eq!(g.mp.person, [0, 1, 2, 3]);
        assert_eq!(g.mp.slots[3].vars.score, 400);
        let _ = std::fs::remove_dir_all(&tmp);
    }

    /// Three players chosen in the middle of a level with two gamepads: the
    /// third Keen joins and the controls are split at once (gamepads 1 and
    /// 2, the keyboard plays the third Keen).
    #[test]
    fn more_players_in_a_level() {
        let tmp = std::env::temp_dir().join(format!("keen-mp3-{}-{}", super::super::EPISODE, std::process::id()));
        let _ = std::fs::create_dir_all(&tmp);
        let Some(mut g) = n_player_game(&tmp, 1, 2) else { return };
        g.mp_split_input();
        assert_eq!(g.sys.input.players, 1);
        g.mp.players = 3;
        g.mp_tick();
        g.mp_split_input();
        assert_eq!(g.mp_extra_objs().len(), 2);
        assert_eq!(g.sys.input.players, 3);
        assert_eq!(g.sys.input.keyboard_player(), 2);
        g.sys.input.pads[1].jump = true;
        g.sys.input.keyboard[g.sys.input.key_def.button1 as usize] = true;
        g.poll_controls();
        assert!(!g.jumpbutton && !g.pogobutton);
        assert!(g.mp.slots[1].vars.jumpbutton && !g.mp.slots[1].vars.pogobutton);
        assert!(g.mp.slots[2].vars.pogobutton && !g.mp.slots[2].vars.jumpbutton);
        let _ = std::fs::remove_dir_all(&tmp);
    }

    /// The number of players offered: as many as the gamepads allow (one
    /// more than there are, the keyboard), 4 at most.
    #[test]
    fn players_offered_by_gamepads() {
        let tmp = std::env::temp_dir().join(format!("keen-mpn-{}-{}", super::super::EPISODE, std::process::id()));
        let _ = std::fs::create_dir_all(&tmp);
        let Some(mut g) = two_player_game(&tmp) else { return };
        for (pads, seq) in [(1, vec![1, 2, 1]), (2, vec![1, 2, 3, 1]), (3, vec![1, 2, 3, 4, 1]), (4, vec![1, 2, 3, 4, 1])] {
            g.sys.input.pad_count = pads;
            g.mp.players = 1;
            let mut got = vec![1];
            for _ in 1..seq.len() {
                g.mp_next_players();
                got.push(g.mp.players);
            }
            assert_eq!(got, seq, "{} gamepads", pads);
        }
        // 4 chosen but only two gamepads: three play
        g.sys.input.pad_count = 2;
        g.mp.players = 4;
        assert_eq!(g.mp_count(), 3);
        let _ = std::fs::remove_dir_all(&tmp);
    }

    /// World map: only the leading player walks (the other players'
    /// controls do nothing), the others follow him one behind the other.
    #[test]
    fn world_map_followers() {
        let tmp = std::env::temp_dir().join(format!("keen-mpw-{}-{}", super::super::EPISODE, std::process::id()));
        let _ = std::fs::create_dir_all(&tmp);
        let Some(mut g) = n_player_game(&tmp, 3, 2) else { return };
        g.gs.mapon = 0;
        assert!(g.setup_game_level(false));
        assert!(g.mp_extra_objs().is_empty());
        let p = g.player;
        g.place_sprite(p);
        g.mp_world_tick();
        assert_eq!(g.rf.extra.len(), 2);
        assert_eq!((g.rf.extra[0].alt, g.rf.extra[1].alt), (1, 2));
        // the keyboard is player 3's: it doesn't move Keen
        g.sys.input.players = g.mp_count();
        g.sys.input.keyboard[g.sys.input.key_def.right as usize] = true;
        g.poll_controls();
        assert_eq!(g.c.xaxis, 0);
        g.sys.input.pad_x = 1;
        g.poll_controls();
        assert_eq!(g.c.xaxis, 1);
        let _ = std::fs::remove_dir_all(&tmp);
    }

    #[test]
    fn two_players_spawn_swap_save_load() {
        let tmp = std::env::temp_dir().join(format!("keen-mp-test-{}-{}", super::super::EPISODE, std::process::id()));
        let _ = std::fs::remove_dir_all(&tmp);
        std::fs::create_dir_all(&tmp).unwrap();
        let Some(mut g) = two_player_game(&tmp) else { return };
        let p1 = g.player;
        let p2 = g.mp.slots[1].obj.expect("player 2 spawned");
        assert_ne!(p1, p2);
        assert_eq!(g.objs[p2].obclass, KEENOBJ);
        assert_eq!((g.objs[p2].x, g.objs[p2].y), (g.objs[p1].x, g.objs[p1].y));

        // the swap: player 2's globals inside as_p2 only
        g.gs.score = 100;
        g.mp.slots[1].vars.score = 1234;
        g.with_slot(1, |g| {
            assert_eq!(g.player, p2);
            assert_eq!(g.gs.score, 1234);
            g.give_points(10);
        });
        assert_eq!((g.player, g.gs.score, g.mp.slots[1].vars.score), (p1, 100, 1244));

        // shots found: both get them
        let (a1, a2) = (g.gs.ammo, g.mp.slots[1].vars.ammo);
        g.with_slot(1, |g| g.give_ammo(5));
        assert_eq!((g.gs.ammo, g.mp.slots[1].vars.ammo), (a1 + 5, a2 + 5));

        // save and load keep player 2
        g.save_state();
        g.mp.slots[1].vars.score = 0;
        g.load_state();
        assert_eq!(g.mp.slots[1].obj, Some(p2));
        assert_eq!(g.mp.slots[1].vars.score, 1244);
        assert_eq!(g.mp.slots[1].vars.ammo, a2 + 5);

        // loaded in one player mode: player 2 is gone
        g.mp.players = 1;
        g.load_state();
        assert_eq!(g.mp.slots[1].obj, None);
        assert!(g.obj_free[p2]);
        // and comes back (fresh) when two player mode is on again
        g.mp.players = 2;
        g.load_state();
        assert!(g.mp.slots[1].obj.is_some());
        let _ = std::fs::remove_dir_all(&tmp);
    }

    #[test]
    fn unlimited_lives_cheat() {
        let tmp = std::env::temp_dir().join(format!("keen-mp-lives-{}-{}", super::super::EPISODE, std::process::id()));
        let _ = std::fs::create_dir_all(&tmp);
        let Some(mut g) = two_player_game(&tmp) else { return };
        // a new two player game starts with twice the lives
        assert_eq!(g.gs.lives, 6);
        g.opts.unlimited_lives = true;
        g.mp_slot_died(1);
        assert_eq!(g.gs.lives, 6);
        assert!(g.mp.slots[1].respawn > 0);
        g.opts.unlimited_lives = false;
        g.mp.slots[1].respawn = 0;
        g.mp_spawn(1, &S_KEENSTAND);
        g.mp_slot_died(1);
        assert_eq!(g.gs.lives, 5);
        // (HandleDeath uses the same rule for player 1; never in demos)
        g.opts.unlimited_lives = true;
        assert!(g.unlimited_lives());
        g.demomode = 1;
        assert!(!g.unlimited_lives());
        let _ = std::fs::remove_dir_all(&tmp);
    }

    #[test]
    fn shots_stun_the_other_player() {
        let tmp = std::env::temp_dir().join(format!("keen-mp-shot-{}-{}", super::super::EPISODE, std::process::id()));
        let _ = std::fs::create_dir_all(&tmp);
        let Some(mut g) = two_player_game(&tmp) else { return };
        let (p1, p2) = (g.player, g.mp.slots[1].obj.unwrap());
        let shot = g.get_new_obj(false);
        g.objs[shot].obclass = STUNSHOTOBJ;
        g.objs[shot].state = Some(&super::super::states::S_STUNRAY1);
        // own shots don't stun
        g.objs[shot].owner = 1;
        assert!(!g.mp_shot_hits_keen(p1, shot));
        g.objs[shot].owner = 2;
        assert!(g.mp_shot_hits_keen(p1, shot));
        assert_eq!(g.mp.dizzy, [super::DIZZYTICS, 0, 0, 0]);
        assert_eq!(g.objs[shot].obclass, INERTOBJ);
        // a dizzy player's controls do nothing
        g.sys.input.keyboard[g.sys.input.key_def.button0 as usize] = true;
        g.sys.input.pad_jump = true;
        g.sys.input.players = 2;
        g.poll_controls();
        assert!(!g.jumpbutton);
        // player 2 (keyboard with one gamepad) is not dizzy
        assert!(g.mp.slots[1].vars.jumpbutton);
        let _ = p2;
        let _ = std::fs::remove_dir_all(&tmp);
    }

    #[test]
    fn player2_takes_over_when_player1_dies() {
        let tmp = std::env::temp_dir().join(format!("keen-mp-lead-{}-{}", super::super::EPISODE, std::process::id()));
        let _ = std::fs::create_dir_all(&tmp);
        let Some(mut g) = two_player_game(&tmp) else { return };
        let (p1, p2) = (g.player, g.mp.slots[1].obj.unwrap());
        g.gs.score = 100;
        g.mp.slots[1].vars.score = 200;
        g.objs[p2].x += 0x100;
        let x2 = g.objs[p2].x;
        let lives = g.gs.lives;
        g.kill_keen();
        assert!(g.keen_dying());
        g.mp_tick();
        // player 2's Keen is the first object now, alive, with his globals
        assert!(g.mp.flipped());
        assert_eq!((g.player, g.mp.slots[1].obj), (p1, Some(p2)));
        assert!(!g.keen_dying() && !g.scrolllock);
        assert_eq!(g.objs[p1].x, x2);
        assert_eq!((g.gs.score, g.mp.slots[1].vars.score), (200, 100));
        assert!((g.mp_person_of(p1) == Some(1)) && !(g.mp_person_of(p2) == Some(1)));
        assert!(g.keen_obj_dying(p2));
        // the leading Keen (in player 2's colors) is drawn in front
        g.place_sprite(p1);
        g.place_sprite(p2);
        assert!(g.rf.sprites[p1].alt == 1 && g.rf.sprites[p1].depth == 0);
        assert!(g.rf.sprites[p2].alt == 0 && g.rf.sprites[p2].depth == 1);
        // and the one behind is the one drawn 2 pixels higher
        assert_eq!(g.rf.sprites[p1].y, g.objs[p1].y);
        assert_eq!(g.rf.sprites[p2].y, g.objs[p2].y - super::ALTYOFFSET);
        // the dead player 1 costs a life and comes back as the second Keen
        g.with_slot(1, |g| g.playstate = EX_DIED);
        assert_eq!(g.playstate, EX_STILLPLAYING);
        assert_eq!(g.gs.lives, lives - 1);
        assert!(g.mp.slots[1].respawn > 0);
        // saved and loaded: still player 2 leading
        g.save_state();
        g.mp.person = [0, 1, 2, 3];
        g.load_state();
        assert!(g.mp.flipped());
        // the next level: player 1 leads again, with his own score
        g.mp_setup_level();
        assert!(!g.mp.flipped());
        assert_eq!((g.gs.score, g.mp.slots[1].vars.score), (100, 200));
        let _ = std::fs::remove_dir_all(&tmp);
    }

    #[test]
    fn shot_off_the_pogo_stick() {
        let tmp = std::env::temp_dir().join(format!("keen-mp-pogo-{}-{}", super::super::EPISODE, std::process::id()));
        let _ = std::fs::create_dir_all(&tmp);
        let Some(mut g) = two_player_game(&tmp) else { return };
        let p2 = g.mp.slots[1].obj.unwrap();
        g.with_slot(1, |g| g.change_state(p2, &S_KEENPOGO));
        let shot = g.get_new_obj(false);
        g.objs[shot].obclass = STUNSHOTOBJ;
        g.objs[shot].state = Some(&super::super::states::S_STUNRAY1);
        g.objs[shot].owner = 1;
        g.with_slot(1, |g| assert!(g.mp_shot_hits_keen(p2, shot)));
        assert!(same(g.objs[p2].state, &S_KEENFALL1));
        let _ = std::fs::remove_dir_all(&tmp);
    }

    #[test]
    fn unlimited_ammo_and_two_pads_at_start() {
        let tmp = std::env::temp_dir().join(format!("keen-mp-ammo-{}-{}", super::super::EPISODE, std::process::id()));
        let _ = std::fs::create_dir_all(&tmp);
        let Some(mut g) = two_player_game(&tmp) else { return };
        g.gs.ammo = 0;
        let (x, y) = (g.objs[g.player].x, g.objs[g.player].y);
        let used = |g: &Game| g.obj_free.iter().filter(|&&f| !f).count();
        let n = used(&g);
        g.spawn_shot(x, y, 1);
        assert_eq!((g.gs.ammo, used(&g)), (0, n));
        // (the shot may hit something at once and explode, still an object)
        g.opts.unlimited_ammo = true;
        g.spawn_shot(x, y, 1);
        assert_eq!((g.gs.ammo, used(&g)), (0, n + 1));
        // never with --classic (no extended mode)
        g.opts.cheatmode = false;
        assert!(!g.unlimited_ammo() && !g.unlimited_lives());
        // remembered in config.txt
        g.opts.cheatmode = true;
        g.opts.unlimited_lives = true;
        g.opts.allow_reenter = true;
        g.opts.hide_inventory = true;
        g.save_config();
        g.opts = crate::options::Options { cheatmode: true, ..Default::default() };
        g.load_config();
        assert!(g.opts.unlimited_ammo && g.opts.unlimited_lives && g.opts.allow_reenter && g.opts.hide_inventory);
        // two gamepads at the start: two player mode on (once)
        g.mp.players = 1;
        g.sys.input.two_pads_at_start = true;
        g.mp_auto_enable();
        assert!(g.mp.enabled() && !g.sys.input.two_pads_at_start);
        let _ = std::fs::remove_dir_all(&tmp);
    }

    #[test]
    fn the_other_keen_follows_the_leaders_door() {
        let tmp = std::env::temp_dir().join(format!("keen-mp-door-{}-{}", super::super::EPISODE, std::process::id()));
        let _ = std::fs::create_dir_all(&tmp);
        let Some(mut g) = two_player_game(&tmp) else { return };
        let (p1, p2) = (g.player, g.mp.slots[1].obj.unwrap());
        // player 2 elsewhere on the map, on a pole (no clipping there,
        // but not dying)
        g.objs[p2].x += 0x2000;
        g.with_slot(1, |g| {
            g.objs[p2].needtoclip = CL_NOCLIP;
            g.change_state(p2, &S_KEENPOLE);
        });
        // player 1 went through a door to another spot: player 2 follows
        let x = g.objs[p1].x + 0x500;
        g.objs[p1].x = x;
        g.mp_follow_door();
        assert!((g.objs[p2].x - x).abs() < 0x100);
        assert!(same(g.objs[p2].state, &S_KEENFALL1));
        // player 2's door takes only him
        let x1 = g.objs[p1].x;
        g.with_slot(1, |g| {
            g.objs[p2].x += 0x1000;
            g.mp_follow_door();
        });
        g.mp_tick();
        assert!(!g.mp.flipped());
        assert_eq!(g.objs[p1].x, x1);
        let _ = std::fs::remove_dir_all(&tmp);
    }

    #[test]
    fn brought_back_through_floors() {
        let tmp = std::env::temp_dir().join(format!("keen-mp-warp-{}-{}", super::super::EPISODE, std::process::id()));
        let _ = std::fs::create_dir_all(&tmp);
        let Some(mut g) = two_player_game(&tmp) else { return };
        let (p1, p2) = (g.player, g.mp.slots[1].obj.unwrap());
        // player 2 far above player 1 (floors in between in any level)
        g.objs[p2].y = 0x300;
        g.with_slot(1, |g| {
            g.objs[p2].needtoclip = CL_NOCLIP;
            g.change_state(p2, &S_KEENFALL1);
            g.objs[p2].needtoclip = CL_MIDCLIP;
        });
        assert!(g.objs[p1].y > 0x800);
        g.mp_warp(1);
        // he arrives at player 1, not on a floor on the way
        assert!((g.objs[p2].y - g.objs[p1].y).abs() < 0x100, "{:#x} {:#x}", g.objs[p2].y, g.objs[p1].y);
        let _ = std::fs::remove_dir_all(&tmp);
    }

    #[test]
    fn dizzy_keen_does_not_read() {
        let tmp = std::env::temp_dir().join(format!("keen-mp-read-{}-{}", super::super::EPISODE, std::process::id()));
        let _ = std::fs::create_dir_all(&tmp);
        let Some(mut g) = two_player_game(&tmp) else { return };
        if !g.mp_load_stunned() {
            return; // (Keen 4/5 without Keen 6 files: no sitting sprite)
        }
        let p1 = g.player;
        g.objs[p1].hitnorth = 1;
        for st in [&S_KEENREAD2, &S_KEENWAIT3, &S_KEENSIT2] {
            g.change_state(p1, st);
            assert!(g.mp_sit_dizzy(p1), "{}", st.name);
            assert!(same(g.objs[p1].state, &S_KEENSTAND));
        }
        let _ = std::fs::remove_dir_all(&tmp);
    }

    /// TWO PLAYERS switched off in a level: player 2's Keen leaves at the
    /// next frame, also when he leads (player 1 gets the lead back);
    /// switched on again he joins.
    #[test]
    fn switching_two_players_off_and_on_in_a_level() {
        let tmp = std::env::temp_dir().join(format!("keen-mp-off-{}-{}", super::super::EPISODE, std::process::id()));
        let _ = std::fs::create_dir_all(&tmp);
        let Some(mut g) = two_player_game(&tmp) else { return };
        let (p1, p2) = (g.player, g.mp.slots[1].obj.unwrap());
        g.mp.players = 1;
        g.mp_tick();
        assert_eq!(g.mp.slots[1].obj, None);
        assert!(g.obj_free[p2] && !g.rf.sprites[p2].visible);
        assert!(!g.mp_in_level() && !g.mp.flipped());
        // on again: he is back next to player 1
        g.mp.players = 2;
        g.mp_tick();
        let p2 = g.mp.slots[1].obj.expect("player 2 joined");
        assert!((g.objs[p2].x - g.objs[p1].x).abs() < 0x100);
        // player 2 leads (player 1 died), then off: player 1 leads again
        // with his own score, the dead Keen is gone
        g.gs.score = 100;
        g.mp.slots[1].vars.score = 200;
        g.objs[p2].x += 0x100;
        g.kill_keen();
        g.mp_tick();
        assert!(g.mp.flipped());
        g.mp.players = 1;
        g.mp_tick();
        assert!(!g.mp.flipped() && g.mp.slots[1].obj.is_none());
        assert_eq!(g.player, p1);
        assert_eq!(g.gs.score, 100);
        assert!(g.obj_free[p2]);
        let _ = std::fs::remove_dir_all(&tmp);
    }

    /// The last frame of a level (faded out) while Keen walks out past the
    /// map's edge still shows the level, not the strip beyond the edge;
    /// Keen 5's world map keeps its black (not yellow) border while a
    /// level is being entered (gamestate.mapon already set).
    #[test]
    fn last_frames_before_fading_out() {
        let tmp = std::env::temp_dir().join(format!("keen-fade-{}-{}", super::super::EPISODE, std::process::id()));
        let _ = std::fs::create_dir_all(&tmp);
        let Some(mut g) = two_player_game(&tmp) else { return };
        g.mp.players = 1;
        g.mp_tick();
        g.wide_ok = true;
        g.opts.zoom = 50;
        g.update_view();
        let p = g.player;
        // at the left edge of the map, in a room narrower than the view
        // (like Keen 6's Megarocket level), so the view reaches beyond
        // the edge
        g.objs[p].x = 0x280;
        g.cam.vblocks.push(20);
        g.center_actor(p);
        let lit = |g: &Game| g.sys.surface.px.iter().filter(|&&c| c & 0xffffff != 0).count();
        g.draw_playfield();
        let before = lit(&g);
        // walked out of the left edge (left of the scroll block in column 1)
        g.objs[p].x = 0x80;
        g.draw_playfield();
        assert!(lit(&g) * 2 > before, "the level stays visible");
        if super::super::EPISODE == 5 {
            g.gs.mapon = 0;
            assert!(g.setup_game_level(false));
            g.update_view();
            let p = g.player;
            g.center_actor(p);
            g.draw_playfield();
            let yellow = |g: &Game| g.sys.surface.px.iter().filter(|&&c| c & 0xffffff == 0xffff55).count();
            let y0 = yellow(&g);
            g.gs.mapon = 1;
            g.draw_playfield();
            assert_eq!(yellow(&g), y0);
        }
        let _ = std::fs::remove_dir_all(&tmp);
    }

    /// Zoomed out to 50 percent the view is 400 lines high, Keen stays in
    /// it, falling out of the map happens where it does in the original
    /// view, and 100 percent restores the original view.
    #[test]
    fn zoomed_out_view() {
        let tmp = std::env::temp_dir().join(format!("keen-zoom-test-{}-{}", super::super::EPISODE, std::process::id()));
        let _ = std::fs::remove_dir_all(&tmp);
        std::fs::create_dir_all(&tmp).unwrap();
        let Some(mut g) = two_player_game(&tmp) else { return };
        let fall = g.fall_line();
        g.wide_ok = true;
        g.opts.zoom = 50;
        g.update_view();
        assert_eq!(g.cam.view_hpx(), 400);
        assert_eq!(g.cam.view_w(), 640);
        assert_eq!((g.sys.surface.logical_w(), g.sys.surface.logical_h()), (640, 400));
        assert_eq!((g.sys.surface.xoff, g.sys.surface.yoff), (160, 100));
        assert_eq!(g.fall_line(), fall);
        let p = g.player;
        // a small room near the top left corner: the centered view starts
        // left of and above the map, objects in it must still wake up
        // (the activation check of PlayLoop, unsigned like the original)
        let (sx, sy, sb, sv) = (g.objs[p].x, g.objs[p].y, g.cam.hblocks.clone(), g.cam.vblocks.clone());
        g.objs[p].x = 0x380;
        g.objs[p].y = 0x380;
        g.cam.vblocks.push(6);
        g.cam.hblocks.push(6);
        g.rf_new_position(0x200, 0x200);
        g.update_active_bounds();
        assert!(g.cam.originxglobal < 0 && g.cam.originyglobal < 0);
        let c = &g.cam;
        let (tl, tt) = (3u16, 3u16);
        assert!(((c.originxtile - 1) as u16) <= tl && tl <= (c.originxtilemax + 1) as u16);
        assert!(((c.originytile - 1) as u16) <= tt && tt <= (c.originytilemax + 1) as u16);
        assert!(c.originxtilemax < 40 && c.originytilemax < 25);
        (g.objs[p].x, g.objs[p].y, g.cam.hblocks, g.cam.vblocks) = (sx, sy, sb, sv);
        g.center_actor(p);
        assert_eq!(g.cam.originytilemax - (g.cam.originyglobal >> 8), 25);
        let (o, c) = (&g.objs[p], &g.cam);
        assert!(c.originyglobal <= o.top && o.bottom <= c.originyglobal + 400 * 16);
        assert!(c.originxglobal <= o.left && o.right <= c.originxglobal + 640 * 16);
        g.step_zoom(false);
        g.update_view();
        assert_eq!(g.cam.view_hpx(), 266);
        g.opts.zoom = 100;
        g.update_view();
        assert_eq!((g.cam.view_w(), g.cam.view_hpx()), (320, 200));
        assert_eq!(g.fall_line(), fall);
        let _ = std::fs::remove_dir_all(&tmp);
    }
}
