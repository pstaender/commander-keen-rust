//! Two player mode (not in the original): a second Keen in the levels.
//!
//! The first player plays with the first gamepad, the second one with the
//! second gamepad or, if there is only one, the keyboard. Without a
//! gamepad there is no two player mode. Player 1 leads: the camera follows
//! him, and player 2 is brought back to him when he stays off screen.
//!
//! The ported Keen code works on the globals of "the" player (`player`,
//! `c`, the button states, `gs.score`, `gs.ammo`, `gs.riding`, ...). For
//! everything player 2 does (his thinking, his controls, his contacts and
//! tiles) `as_p2` swaps his set of these globals in, so the original code
//! runs unchanged for both Keens. Keys, gems, lives, drops and the special
//! items stay shared; score and shots are per player (both get the shots
//! found). Stunner shots stun the other Keen: he is dizzy for two seconds.
//!
//! When player 1 dies while player 2 is still alive, player 2 takes the
//! lead (`mp_handover`): the two Keen objects exchange their contents and
//! their globals, so the living Keen is `player` again (the camera follows
//! him) and the dying one comes back as the second Keen for a shared life.
//! `MpState::flipped` then tells that the first Keen is played by the
//! second player (controls, colors and score boxes follow the people, not
//! the objects) until the level ends.
//!
//! Never active in demos or with --classic, so the original code paths
//! (and the demo playback) stay as they were.

use super::defs::*;
use super::states::*;
use super::Game;
use crate::engine::input::ControlInfo;

/// tics a Keen hit by the other one's stunner is dizzy (2 seconds)
pub const DIZZYTICS: i32 = 140;
/// tics player 2 may stay off screen before he is brought back
const AWAYTICS: i32 = 140;
/// tics after player 2 died before he comes back
const RESPAWNTICS: i32 = 70;
/// player 2 is invincible for a moment after coming back
const RESPAWNINVINCIBLE: i32 = 70;
/// player 2's sprites are drawn this much higher (2 pixels), so the two
/// Keens are told apart even on the same spot
pub const ALTYOFFSET: i32 = 0x20;
/// world map: distance (global units) Keen 2 walks behind Keen 1
const WORLDFOLLOW: i32 = 0x180;
/// world map: Keen 1 moved further than this in one frame (teleporter,
/// elevator): Keen 2 jumps along
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

#[derive(Clone, Default)]
pub struct MpState {
    /// two player mode chosen (CONFIGURE > USE GAMEPAD > TWO PLAYERS,
    /// config.txt `twoplayers`)
    pub enabled: bool,
    /// player 2's object in the current level
    pub p2: Option<usize>,
    /// the globals of the player not swapped in: player 2's normally,
    /// player 1's inside `as_p2` (they keep player 2's score and shots
    /// between levels as well)
    pub other: PlayerVars,
    /// player 2's globals are swapped in
    pub swapped: bool,
    /// tics each player stays dizzy
    pub dizzy: [i32; 2],
    /// player 2 died: tics until he comes back (0: not waiting)
    pub respawn: i32,
    /// tics player 2 has been off screen
    pub away: i32,
    /// player 2 lost his last life: he is out until the next level
    pub out: bool,
    /// world map: the path of Keen 1 (x, y, shape), newest last; Keen 2
    /// (only a sprite there) walks along it
    pub trail: Vec<(i32, i32, i32)>,
    /// world map: Keen 2's direction (dir_* id) for his standing shape
    pub world_dir: i32,
    /// player 1 died and player 2 took the lead: `player` (the first Keen,
    /// the camera follows him) is played by player 2, the second Keen by
    /// player 1 (until the level ends)
    pub flipped: bool,
}

impl Game {
    /// Two player mode can be played: chosen, a gamepad is connected, not
    /// in demos and not with --classic.
    pub fn mp_available(&self) -> bool {
        self.mp.enabled
            && !self.opts.classic
            && self.demomode == 0
            && !self.storedemo
            && self.sys.input.pad_connected
    }

    /// Player 2 takes part in the current level (alive, dying or waiting
    /// to come back).
    pub fn mp_in_level(&self) -> bool {
        self.mp.p2.is_some() || self.mp.respawn > 0
    }

    /// The current player number (0 or 1) of the swapped in globals.
    fn mp_cur(&self) -> usize {
        self.mp.swapped as usize
    }

    /// Player number of a Keen object, None for other objects.
    pub fn mp_player_of(&self, ob: usize) -> Option<usize> {
        let (p1, p2) = self.mp_objects();
        if ob == p1 {
            Some(0)
        } else if Some(ob) == p2 {
            Some(1)
        } else {
            None
        }
    }

    /// (player 1's object, player 2's object), whatever is swapped in.
    pub fn mp_objects(&self) -> (usize, Option<usize>) {
        if self.mp.swapped {
            (self.mp.other.player, Some(self.player))
        } else {
            (self.player, self.mp.p2)
        }
    }

    /// The first object of the list (player 1's), also inside `as_p2`.
    pub fn mp_first_obj(&self) -> usize {
        self.mp_objects().0
    }

    /// New game: player 2 starts like player 1 (NewGame).
    pub fn mp_new_game(&mut self) {
        let o = &mut self.mp.other;
        o.score = 0;
        o.nextextra = 20000;
        o.ammo = 5;
        self.mp.out = false;
        self.mp.flipped = false;
    }

    /// Two gamepads at launch: two player mode on (not with --classic).
    pub fn mp_auto_enable(&mut self) {
        if std::mem::take(&mut self.sys.input.two_pads_at_start) && !self.opts.classic {
            self.mp.enabled = true;
        }
    }

    /// The Keen object `ob` is played by player 2 (drawn recolored).
    pub fn mp_is_player2(&self, ob: usize) -> bool {
        match self.mp_player_of(ob) {
            Some(n) => (n == 1) != self.mp.flipped,
            None => false,
        }
    }

    /// Exchanges the per player globals (score, shots, buttons, ...) but
    /// not the objects.
    fn mp_exchange_vars(&mut self) {
        let (p, op) = (self.player, self.mp.other.player);
        self.mp_swap();
        self.mp.swapped = !self.mp.swapped;
        self.player = p;
        self.mp.other.player = op;
    }

    /// Back to player 1 leading (a new level or the world map).
    fn mp_unflip(&mut self) {
        if self.mp.flipped {
            self.mp_exchange_vars();
            self.mp.flipped = false;
        }
    }

    /// Player 1 died (or fell out of the map with `force`) while player 2
    /// is alive: player 2 takes the lead. The two Keen objects exchange
    /// everything but their place in the object list (the first one stays
    /// `player`, which the original code relies on), the per player
    /// globals, dizziness and the owners of the flying shots. True if done.
    pub fn mp_handover(&mut self, force: bool) -> bool {
        let Some(p2) = self.mp.p2 else { return false };
        if self.mp.swapped || self.mp_p2_dying(p2) || self.objs[p2].obclass != KEENOBJ {
            return false;
        }
        let p1 = self.player;
        let s = self.objs[p1].state;
        if !force && !(same(s, &S_KEENDIE1) || same(s, &S_KEENDIE2) || same(s, &S_KEENDEAD)) {
            return false;
        }
        let (a, b) = (self.objs[p1].clone(), self.objs[p2].clone());
        self.objs[p1] = super::defs::Obj { prev: a.prev, next: a.next, ..b };
        self.objs[p2] = super::defs::Obj { prev: b.prev, next: b.next, ..a };
        self.rf.sprites.swap(p1, p2);
        self.rf.stars.swap(p1, p2);
        self.mp_exchange_vars();
        self.mp.dizzy.swap(0, 1);
        self.mp.flipped = !self.mp.flipped;
        self.mp.away = 0;
        // (KillKeen locked the camera on the dying Keen)
        self.scrolllock = false;
        for i in 0..self.objs.len() {
            if !self.obj_free[i] && self.objs[i].obclass == STUNSHOTOBJ && matches!(self.objs[i].owner, 1 | 2) {
                self.objs[i].owner = 3 - self.objs[i].owner;
            }
        }
        true
    }

    /// From the play loop: the first Keen fell out of the map. With player
    /// 2 alive he takes the lead and the fallen one costs a life like player
    /// 2 does; false if the level is lost.
    pub fn mp_leader_fell(&mut self) -> bool {
        if !self.mp_handover(true) {
            return false;
        }
        self.mp_p2_died();
        true
    }

    /// Exchanges the globals of the two players.
    fn mp_swap(&mut self) {
        let o = &mut self.mp.other;
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
        self.mp.swapped = !self.mp.swapped;
    }

    /// Runs `f` as player 2 (his globals swapped in). If he died in it
    /// (KeenDieThink sets EX_DIED) the level goes on without him.
    pub fn as_p2<R>(&mut self, f: impl FnOnce(&mut Game) -> R) -> R {
        if self.mp.swapped {
            return f(self);
        }
        let ps = self.playstate;
        // (player 2's object: `player` while his globals are swapped in)
        if let Some(p2) = self.mp.p2 {
            self.mp.other.player = p2;
        }
        self.mp_swap();
        let r = f(self);
        self.mp_swap();
        if ps != EX_DIED && self.playstate == EX_DIED {
            self.playstate = ps;
            self.mp_p2_died();
        }
        r
    }

    /// Calls a contact function, as player 2 if it is about him.
    pub fn mp_contact(&mut self, c: ContactFn, ob: usize, hit: usize) {
        match self.mp.p2 {
            Some(p2) if !self.mp.swapped && (ob == p2 || (hit == p2 && ob != self.player)) => {
                self.as_p2(|g| c(g, ob, hit))
            }
            _ => c(self, ob, hit),
        }
    }

    /// The current player is dizzy: his controls do nothing.
    pub fn mp_dizzy(&self) -> bool {
        self.mp_in_level() && self.mp.dizzy[self.mp_cur()] > 0
    }

    /// Shots found: both players get them.
    pub fn give_ammo(&mut self, n: i32) {
        self.gs.ammo += n;
        if self.mp_in_level() || self.mp.swapped {
            self.mp.other.ammo += n;
        }
    }

    /// Owner of a new stunner shot (1 or 2; 0 = single player).
    pub fn mp_shot_owner(&self) -> u8 {
        if self.mp_in_level() {
            self.mp_cur() as u8 + 1
        } else {
            0
        }
    }

    /// From KeenContact: a stunner shot of the other player makes Keen
    /// dizzy. True if the shot was used up.
    pub fn mp_shot_hits_keen(&mut self, ob: usize, hit: usize) -> bool {
        if self.objs[hit].obclass != STUNSHOTOBJ || !self.mp_in_level() {
            return false;
        }
        let owner = self.objs[hit].owner as usize;
        let Some(n) = self.mp_player_of(ob) else { return false };
        if owner == 0 || owner == n + 1 {
            return false;
        }
        self.explode_shot(hit);
        self.mp.dizzy[n] = DIZZYTICS;
        self.play_sound(super::keen::SND_HELMETHIT);
        self.mp_knock_off_pogo(n, ob);
        true
    }

    // ---- levels ---------------------------------------------------------

    /// After ScanInfoPlane: player 2 starts where player 1 does (not on
    /// the world map and not where Keen doesn't start on his feet, like
    /// Keen 4's underwater level).
    pub fn mp_setup_level(&mut self) {
        self.mp_auto_enable();
        self.mp_unflip();
        self.mp.p2 = None;
        self.mp.respawn = 0;
        self.mp.away = 0;
        self.mp.dizzy = [0; 2];
        self.mp.out = false;
        self.mp.trail.clear();
        self.mp.other.riding = None;
        self.mp.other.invincible = 0;
        if !self.mp_available() || self.gs.mapon == 0 {
            return;
        }
        let p = self.player;
        if self.objs[p].obclass != KEENOBJ || !same(self.objs[p].state, &S_KEENSTAND) {
            return;
        }
        self.mp_spawn_p2(&S_KEENSTAND);
    }

    /// A loaded game: player 2's object from the saved game (or a new one
    /// next to player 1), or none in one player mode.
    pub fn mp_after_load(&mut self, p2: Option<usize>) {
        self.mp.trail.clear();
        self.mp.respawn = 0;
        self.mp.away = 0;
        self.mp.dizzy = [0; 2];
        self.mp.other.riding = None;
        self.mp.other.invincible = 0;
        let p2 = p2.filter(|&n| n != self.player && !self.obj_free[n] && self.objs[n].obclass == KEENOBJ);
        self.mp.p2 = None;
        if !self.mp_available() || self.gs.mapon == 0 {
            if let Some(n) = p2 {
                self.remove_obj(n);
            }
            self.mp_unflip();
            return;
        }
        match p2 {
            Some(n) => self.mp.p2 = Some(n),
            None => {
                let p = self.player;
                if self.objs[p].obclass == KEENOBJ && self.objs[p].needtoclip != CL_NOCLIP {
                    self.mp_spawn_p2(&S_KEENFALL1);
                }
            }
        }
    }

    /// A new Keen object for player 2 at player 1's position.
    fn mp_spawn_p2(&mut self, st: StateRef) -> bool {
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
        self.mp.p2 = Some(n);
        self.mp.away = 0;
        self.mp.dizzy[1] = 0;
        self.as_p2(|g| {
            g.jumptime = 0;
            g.gs.riding = None;
            g.new_state(n, st);
        });
        true
    }

    /// TWO PLAYERS switched in the middle of a level (in the control
    /// panel): off, player 2's Keen leaves at once (player 1 keeps or gets
    /// back the lead); on, he joins next to player 1.
    fn mp_apply_mode(&mut self) {
        if self.mp.swapped {
            return;
        }
        if !self.mp.enabled {
            if !self.mp_in_level() && !self.mp.flipped {
                return;
            }
            if self.mp.flipped {
                // player 1's Keen (the second object) alive: back to him
                // leading; otherwise only player 1's globals come back and
                // he plays on with the remaining Keen
                if !self.mp_handover(true) {
                    self.mp_unflip();
                }
            }
            if let Some(n) = self.mp.p2.take() {
                self.remove_obj(n);
            }
            self.mp.respawn = 0;
            self.mp.away = 0;
            self.mp.dizzy = [0; 2];
            self.mp.out = false;
            self.mp.other.riding = None;
            self.mp.other.invincible = 0;
            return;
        }
        // switched on: join like after loading a one player game
        let p = self.player;
        if self.mp_available()
            && !self.mp_in_level()
            && !self.mp.out
            && self.gs.mapon != 0
            && self.objs[p].obclass == KEENOBJ
            && self.objs[p].needtoclip != CL_NOCLIP
            && !self.keen_dying()
            && self.mp_spawn_p2(&S_KEENFALL1)
        {
            self.mp.other.invincible = RESPAWNINVINCIBLE;
        }
    }

    /// Player 2 died (fell off the screen dying, or out of the map): he
    /// comes back next to player 1 for one of the shared lives.
    fn mp_p2_died(&mut self) {
        let Some(n) = self.mp.p2.take() else { return };
        self.remove_obj(n);
        self.mp.dizzy[1] = 0;
        self.mp.other.riding = None;
        if self.unlimited_lives() {
            self.mp.respawn = RESPAWNTICS;
        } else if self.gs.lives > 0 {
            self.gs.lives -= 1;
            self.mp.respawn = RESPAWNTICS;
        } else {
            self.mp.out = true;
        }
    }

    /// Brings player 2 back to player 1 (falling, so he lands safely):
    /// when he was off screen too long, and through player 1's doors.
    fn mp_warp_p2(&mut self, p2: usize) {
        let p = self.objs[self.player].clone();
        self.as_p2(|g| {
            let o = &mut g.objs[p2];
            o.x = p.x;
            o.y = p.y;
            o.xspeed = 0;
            o.yspeed = 0;
            o.priority = 1;
            g.gs.riding = None;
            g.jumptime = 0;
            // without clipping, like KeenEnterThink: ChangeState clips the
            // move from the old position, which would stop him on the
            // first floor between there and player 1
            g.objs[p2].needtoclip = CL_NOCLIP;
            g.change_state(p2, &S_KEENFALL1);
            g.objs[p2].needtoclip = CL_MIDCLIP;
        });
        self.mp.away = 0;
    }

    /// The leading Keen went through a door (KeenEnterThink, also to
    /// another spot of the same map): the other one comes along. (The
    /// second Keen's doors take only him; when he stays off screen he is
    /// brought back.)
    pub fn mp_follow_door(&mut self) {
        if self.mp.swapped {
            return;
        }
        if let Some(p2) = self.mp.p2 {
            if !self.mp_p2_dying(p2) {
                self.mp_warp_p2(p2);
            }
        }
    }

    /// The Keen object `p2` (either one) is dying. (Only by his state: a
    /// Keen on a pole or climbing onto a ledge has no clipping either, and
    /// must still come along through doors and be brought back.)
    fn mp_p2_dying(&self, p2: usize) -> bool {
        self.keen_obj_dying(p2)
    }

    /// From the play loop, after the reactions: player 2's reaction (or
    /// his end when he fell out of the map).
    pub fn mp_react_p2(&mut self, ob: usize) {
        if (self.objs[ob].tilebottom as u16) < (self.mapheight - 1) as u16 {
            if self.objs[ob].needtoreact {
                if let Some(r) = self.objs[ob].state.and_then(|s| s.react) {
                    self.objs[ob].needtoreact = false;
                    self.as_p2(|g| r(g, ob));
                }
            }
        } else {
            self.mp_p2_died();
        }
    }

    /// Once per frame after scrolling: what ScrollScreen does for player
    /// 1 (falling out of the map, leaving it at the sides), bringing player
    /// 2 back when he is off screen too long, coming back after dying,
    /// dizziness.
    pub fn mp_tick(&mut self) {
        self.mp_apply_mode();
        // player 1 dies: player 2 goes on
        if self.mp.p2.is_some() && !self.mp.swapped && self.keen_dying() {
            self.mp_handover(false);
        }
        if !self.mp_in_level() {
            return;
        }
        let tics = self.tics;
        self.mp.other.invincible = (self.mp.other.invincible - tics).max(0);
        if self.mp.respawn > 0 {
            self.mp.respawn -= tics;
            let p = self.player;
            if self.mp.respawn <= 0 {
                self.mp.respawn = 1;
                if !self.keen_dying() && self.objs[p].needtoclip != CL_NOCLIP && self.mp_spawn_p2(&S_KEENFALL1) {
                    self.mp.respawn = 0;
                    self.mp.other.invincible = RESPAWNINVINCIBLE;
                }
            }
        }
        if let Some(p2) = self.mp.p2 {
            let o = self.objs[p2].clone();
            let c = self.cam.clone();
            let dying = self.mp_p2_dying(p2);
            let right = if c.wide() { (self.mapwidth - 2) * 256 } else { c.originxmax + 0x1400 };
            if !dying && ((o.left as u16) < c.originxmin as u16 || (right as u16) < o.right as u16) {
                self.playstate = EX_COMPLETED;
            } else if !dying && (self.fall_line() as u16) < o.bottom as u16 {
                self.objs[p2].y -= o.bottom - self.fall_line();
                self.play_sound(super::keen::SND_PLUMMET);
                self.as_p2(|g| {
                    let god = std::mem::replace(&mut g.godmode, false);
                    g.invincible = 0;
                    g.kill_keen();
                    g.godmode = god;
                });
            } else if !dying && !self.on_screen(p2) && !self.scrolllock {
                self.mp.away += tics;
                let p = self.player;
                if self.mp.away > AWAYTICS && !self.keen_dying() && self.objs[p].needtoclip != CL_NOCLIP {
                    self.mp_warp_p2(p2);
                }
            } else {
                self.mp.away = 0;
            }
        }
        // the stun stars over a dizzy Keen's head
        let (p1, p2) = self.mp_objects();
        let frame = (self.lasttimecount / 10) % 3;
        for (n, ob) in [(0, Some(p1)), (1, p2)] {
            let Some(ob) = ob else { continue };
            if self.mp.dizzy[n] > 0 {
                self.mp.dizzy[n] = (self.mp.dizzy[n] - tics).max(0);
                self.mp_knock_off_pogo(n, ob);
                // on the ground he sits dazed, the stars lower
                let down = if self.mp_sit_dizzy(ob) { 0x80 } else { 0 };
                let (x, y) = (self.objs[ob].x + 0x20, self.objs[ob].y - 0x90 + down);
                self.place_stars(ob, x, y, super::ep::STARSPR + frame, 3);
            } else {
                self.remove_stars(ob);
            }
        }
    }

    /// World map (two player mode): Keen 2 follows Keen 1's path a
    /// little behind him. Only a sprite: he does nothing there, and saved
    /// games, demos and the map logic don't know him.
    pub fn mp_world_tick(&mut self) {
        self.mp_auto_enable();
        if self.gs.mapon != 0 || !self.mp_available() {
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
            if t.len() > 512 {
                t.drain(..256);
            }
        }
        // the point WORLDFOLLOW back along the path (the oldest one while
        // the path is shorter)
        let mut i = t.len() - 1;
        let mut walked = 0;
        while i > 0 && walked < WORLDFOLLOW {
            walked += dist((t[i].0, t[i].1), (t[i - 1].0, t[i - 1].1));
            i -= 1;
        }
        let (x, y, mut shape) = t[i];
        // walking Keen: Keen 2 looks where he goes, and stands when Keen 1
        // stops
        let st = self.objs[self.player].state;
        let walking = same(st, &S_WORLDKEENWALK);
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
                    _ => self.mp.world_dir,
                };
                self.mp.world_dir = d;
            }
            if !walking || !moved {
                shape = super::keen2::WORLDSHAPES[self.mp.world_dir.clamp(0, 7) as usize] + 3;
            }
        }
        // below Keen 1 (drawn before him)
        let slot = super::rf::SpriteSlot {
            visible: true,
            x,
            y: y - ALTYOFFSET,
            shape,
            priority: p1.priority,
            order: p1.order - 1,
            white: false,
            alt: true,
            stunned: false,
            behind: true,
        };
        self.rf.extra = vec![slot];
    }

    /// A dizzy Keen falls off his pogo stick (and sits dazed once he
    /// lands). `n`: his player number (0, 1) of the object `ob`.
    fn mp_knock_off_pogo(&mut self, n: usize, ob: usize) {
        let Some(st) = self.objs[ob].state else { return };
        if !(same(Some(st), &S_KEENPOGO) || same(Some(st), &S_KEENPOGO2) || same(Some(st), &S_KEENPOGODOWN)) {
            return;
        }
        let knock = move |g: &mut Game| {
            g.jumptime = 0;
            g.objs[ob].xspeed = 0;
            g.change_state(ob, &S_KEENFALL1);
        };
        if n == 1 && !self.mp.swapped {
            self.as_p2(knock);
        } else {
            knock(self);
        }
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
            alt: alt_tex(&tex),
            tex,
            orgx: info.orgx as i32,
            orgy: info.orgy as i32,
        }));
        self.gfx.stunned.as_ref().is_some_and(|s| s.is_some())
    }

    /// Player 2's score, shots and lives for the score box: None if he
    /// isn't in this level (on the world map he is shown in two player
    /// mode).
    pub fn mp_p2_status(&self) -> Option<(i32, i32)> {
        let world = self.gs.mapon == 0 && self.mp_available();
        if !self.mp_in_level() && !self.mp.out && !world {
            return None;
        }
        let o = &self.mp.other;
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
        g.mp.enabled = true;
        g.new_game();
        g.gs.mapon = 1;
        assert!(g.setup_game_level(false));
        Some(g)
    }

    #[test]
    fn two_players_spawn_swap_save_load() {
        let tmp = std::env::temp_dir().join(format!("keen-mp-test-{}-{}", super::super::EPISODE, std::process::id()));
        let _ = std::fs::remove_dir_all(&tmp);
        std::fs::create_dir_all(&tmp).unwrap();
        let Some(mut g) = two_player_game(&tmp) else { return };
        let p1 = g.player;
        let p2 = g.mp.p2.expect("player 2 spawned");
        assert_ne!(p1, p2);
        assert_eq!(g.objs[p2].obclass, KEENOBJ);
        assert_eq!((g.objs[p2].x, g.objs[p2].y), (g.objs[p1].x, g.objs[p1].y));

        // the swap: player 2's globals inside as_p2 only
        g.gs.score = 100;
        g.mp.other.score = 1234;
        g.as_p2(|g| {
            assert_eq!(g.player, p2);
            assert_eq!(g.gs.score, 1234);
            g.give_points(10);
        });
        assert_eq!((g.player, g.gs.score, g.mp.other.score), (p1, 100, 1244));

        // shots found: both get them
        let (a1, a2) = (g.gs.ammo, g.mp.other.ammo);
        g.as_p2(|g| g.give_ammo(5));
        assert_eq!((g.gs.ammo, g.mp.other.ammo), (a1 + 5, a2 + 5));

        // save and load keep player 2
        g.save_state();
        g.mp.other.score = 0;
        g.load_state();
        assert_eq!(g.mp.p2, Some(p2));
        assert_eq!(g.mp.other.score, 1244);
        assert_eq!(g.mp.other.ammo, a2 + 5);

        // loaded in one player mode: player 2 is gone
        g.mp.enabled = false;
        g.load_state();
        assert_eq!(g.mp.p2, None);
        assert!(g.obj_free[p2]);
        // and comes back (fresh) when two player mode is on again
        g.mp.enabled = true;
        g.load_state();
        assert!(g.mp.p2.is_some());
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
        g.mp_p2_died();
        assert_eq!(g.gs.lives, 6);
        assert!(g.mp.respawn > 0);
        g.opts.unlimited_lives = false;
        g.mp.respawn = 0;
        g.mp_spawn_p2(&S_KEENSTAND);
        g.mp_p2_died();
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
        let (p1, p2) = (g.player, g.mp.p2.unwrap());
        let shot = g.get_new_obj(false);
        g.objs[shot].obclass = STUNSHOTOBJ;
        g.objs[shot].state = Some(&super::super::states::S_STUNRAY1);
        // own shots don't stun
        g.objs[shot].owner = 1;
        assert!(!g.mp_shot_hits_keen(p1, shot));
        g.objs[shot].owner = 2;
        assert!(g.mp_shot_hits_keen(p1, shot));
        assert_eq!(g.mp.dizzy, [super::DIZZYTICS, 0]);
        assert_eq!(g.objs[shot].obclass, INERTOBJ);
        // a dizzy player's controls do nothing
        g.sys.input.keyboard[g.sys.input.key_def.button0 as usize] = true;
        g.sys.input.pad_jump = true;
        g.sys.input.split = true;
        g.poll_controls();
        assert!(!g.jumpbutton);
        // player 2 (keyboard with one gamepad) is not dizzy
        assert!(g.mp.other.jumpbutton);
        let _ = p2;
        let _ = std::fs::remove_dir_all(&tmp);
    }

    #[test]
    fn player2_takes_over_when_player1_dies() {
        let tmp = std::env::temp_dir().join(format!("keen-mp-lead-{}-{}", super::super::EPISODE, std::process::id()));
        let _ = std::fs::create_dir_all(&tmp);
        let Some(mut g) = two_player_game(&tmp) else { return };
        let (p1, p2) = (g.player, g.mp.p2.unwrap());
        g.gs.score = 100;
        g.mp.other.score = 200;
        g.objs[p2].x += 0x100;
        let x2 = g.objs[p2].x;
        let lives = g.gs.lives;
        g.kill_keen();
        assert!(g.keen_dying());
        g.mp_tick();
        // player 2's Keen is the first object now, alive, with his globals
        assert!(g.mp.flipped);
        assert_eq!((g.player, g.mp.p2), (p1, Some(p2)));
        assert!(!g.keen_dying() && !g.scrolllock);
        assert_eq!(g.objs[p1].x, x2);
        assert_eq!((g.gs.score, g.mp.other.score), (200, 100));
        assert!(g.mp_is_player2(p1) && !g.mp_is_player2(p2));
        assert!(g.mp_p2_dying(p2));
        // the leading Keen (in player 2's colors) is drawn in front
        g.place_sprite(p1);
        g.place_sprite(p2);
        assert!(g.rf.sprites[p1].alt && !g.rf.sprites[p1].behind);
        assert!(!g.rf.sprites[p2].alt && g.rf.sprites[p2].behind);
        // and the one behind is the one drawn 2 pixels higher
        assert_eq!(g.rf.sprites[p1].y, g.objs[p1].y);
        assert_eq!(g.rf.sprites[p2].y, g.objs[p2].y - super::ALTYOFFSET);
        // the dead player 1 costs a life and comes back as the second Keen
        g.as_p2(|g| g.playstate = EX_DIED);
        assert_eq!(g.playstate, EX_STILLPLAYING);
        assert_eq!(g.gs.lives, lives - 1);
        assert!(g.mp.respawn > 0);
        // saved and loaded: still player 2 leading
        g.save_state();
        g.mp.flipped = false;
        g.load_state();
        assert!(g.mp.flipped);
        // the next level: player 1 leads again, with his own score
        g.mp_setup_level();
        assert!(!g.mp.flipped);
        assert_eq!((g.gs.score, g.mp.other.score), (100, 200));
        let _ = std::fs::remove_dir_all(&tmp);
    }

    #[test]
    fn shot_off_the_pogo_stick() {
        let tmp = std::env::temp_dir().join(format!("keen-mp-pogo-{}-{}", super::super::EPISODE, std::process::id()));
        let _ = std::fs::create_dir_all(&tmp);
        let Some(mut g) = two_player_game(&tmp) else { return };
        let p2 = g.mp.p2.unwrap();
        g.as_p2(|g| g.change_state(p2, &S_KEENPOGO));
        let shot = g.get_new_obj(false);
        g.objs[shot].obclass = STUNSHOTOBJ;
        g.objs[shot].state = Some(&super::super::states::S_STUNRAY1);
        g.objs[shot].owner = 1;
        g.as_p2(|g| assert!(g.mp_shot_hits_keen(p2, shot)));
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
        g.mp.enabled = false;
        g.sys.input.two_pads_at_start = true;
        g.mp_auto_enable();
        assert!(g.mp.enabled && !g.sys.input.two_pads_at_start);
        let _ = std::fs::remove_dir_all(&tmp);
    }

    #[test]
    fn the_other_keen_follows_the_leaders_door() {
        let tmp = std::env::temp_dir().join(format!("keen-mp-door-{}-{}", super::super::EPISODE, std::process::id()));
        let _ = std::fs::create_dir_all(&tmp);
        let Some(mut g) = two_player_game(&tmp) else { return };
        let (p1, p2) = (g.player, g.mp.p2.unwrap());
        // player 2 elsewhere on the map, on a pole (no clipping there,
        // but not dying)
        g.objs[p2].x += 0x2000;
        g.as_p2(|g| {
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
        g.as_p2(|g| {
            g.objs[p2].x += 0x1000;
            g.mp_follow_door();
        });
        g.mp_tick();
        assert!(!g.mp.flipped);
        assert_eq!(g.objs[p1].x, x1);
        let _ = std::fs::remove_dir_all(&tmp);
    }

    #[test]
    fn brought_back_through_floors() {
        let tmp = std::env::temp_dir().join(format!("keen-mp-warp-{}-{}", super::super::EPISODE, std::process::id()));
        let _ = std::fs::create_dir_all(&tmp);
        let Some(mut g) = two_player_game(&tmp) else { return };
        let (p1, p2) = (g.player, g.mp.p2.unwrap());
        // player 2 far above player 1 (floors in between in any level)
        g.objs[p2].y = 0x300;
        g.as_p2(|g| {
            g.objs[p2].needtoclip = CL_NOCLIP;
            g.change_state(p2, &S_KEENFALL1);
            g.objs[p2].needtoclip = CL_MIDCLIP;
        });
        assert!(g.objs[p1].y > 0x800);
        g.mp_warp_p2(p2);
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
        let (p1, p2) = (g.player, g.mp.p2.unwrap());
        g.mp.enabled = false;
        g.mp_tick();
        assert_eq!(g.mp.p2, None);
        assert!(g.obj_free[p2] && !g.rf.sprites[p2].visible);
        assert!(!g.mp_in_level() && !g.mp.flipped);
        // on again: he is back next to player 1
        g.mp.enabled = true;
        g.mp_tick();
        let p2 = g.mp.p2.expect("player 2 joined");
        assert!((g.objs[p2].x - g.objs[p1].x).abs() < 0x100);
        // player 2 leads (player 1 died), then off: player 1 leads again
        // with his own score, the dead Keen is gone
        g.gs.score = 100;
        g.mp.other.score = 200;
        g.objs[p2].x += 0x100;
        g.kill_keen();
        g.mp_tick();
        assert!(g.mp.flipped);
        g.mp.enabled = false;
        g.mp_tick();
        assert!(!g.mp.flipped && g.mp.p2.is_none());
        assert_eq!(g.player, p1);
        assert_eq!(g.gs.score, 100);
        assert!(g.obj_free[p2]);
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
