# Commander Keen 4, 5 and 6 – Rust reimplementation

Rust port of the "Goodbye Galaxy" episodes *Commander Keen 4: Secret of the
Oracle* (KEEN4E.EXE v1.4), *5: The Armageddon Machine* (KEEN5E.EXE v1.4) and
*6: Aliens Ate My Baby Sitter!* (KEEN6C.EXE v1.4, id Software 1991), rebuilt
from the original executables with REA (reverse engineering MCP, Ghidra
provider). The original data files (KEEN4/, KEEN5/, KEEN6/) are required at
runtime and are never committed.

## Layout
- `src/assets/` – original file formats: LZEXE unpacker (the EXEs are packed),
  Huffman, Carmack+RLEW, EGAGRAPH (EGA planar → indexed images), GAMEMAPS,
  tile info (from MAPHEAD), AUDIO (PC speaker / AdLib / IMF), `midi.rs`
  IMF → General MIDI (music mode MIDI MUSIC, `--export-midi`; instruments are
  classified by rendering each OPL patch once, overrides in the audio
  pack's `instruments.txt`). `episode.rs`
  holds the per-episode layout (file names, chunk counts, header offsets in
  the executables); the episode is detected from the data directory.
  `exe.rs` locates the headers linked into the executable.
- `src/engine/` – platform layer (ID_* managers): `sys.rs` window/time
  (winit + softbuffer, events pumped from the game loop, letterboxed
  scaling, fullscreen by default (--no-fullscreen, F11 toggles), 70 Hz TimeCount), `video.rs` scalable 32-bit surface in logical
  320x200 coordinates + EGA palette fades, `gfx.rs` texture sets and
  high-res replacement packs, `sound.rs` ID_SD emulation (560/140 Hz timing,
  OPL2 via `oplon`, PC speaker) with replacement audio (rodio, rustysynth),
  `input.rs` scan codes / ControlInfo, gamepad bindings (`PadInput`,
  `pad_map`; read in `Sys::sync_pad`, set up in the menu's PADCARD).
  Headless: `KEEN_PAD=1` pretends a gamepad is connected;
  `KEEN_PAD_DEBUG=1` prints the pads and their events. gilrs runs without
  its default filters (they drop d-pad axes that don't reach exactly ±1).
- `src/game/` – the shared CK_* modules, ported function by function from
  the decompilation. Like the original source (one set of files with
  `#ifdef`s) they are compiled once per episode module (`src/ckN/mod.rs`
  includes `src/game/modules.rs` and `root.rs`); inside them `super::` is the
  episode module. Episode differences: `ep4! {}` / `ep5! {}` / `ep6! {}`
  (code only in that episode), `ep_sel!(k4, k5, k6)` (value per episode),
  or a function in `src/ckN/ep.rs` (KeenContact, status window, object
  classes used by shared code). `defs.rs` objects/constants, `phys.rs`
  CK_PHYS, `play.rs` CK_PLAY, `keen.rs`/`keen2.rs` CK_KEEN/CK_KEEN2,
  `game.rs` CK_GAME, `rf.rs` refresh manager, `us.rs` text windows,
  `menu.rs` control panel, `demo.rs` CK_DEMO, `text.rs` CK_TEXT, `vw.rs`
  fades, `tools.rs` helpers for the editor.
- `src/ck4/`, `src/ck5/`, `src/ck6/` – per episode: `consts.rs` (chunk
  numbers, GFXE_CKn.H), `ep.rs`, `spec.rs` (ScanInfoPlane, specials),
  `act1-3.rs` (the episode's actors), `states.rs` (GENERATED with
  `EP=N python3 -I tools/gen_states.py` from the state records in the
  executable; names in `tools/states_kN.py` and `tools/states_kN_<module>.py`).
- `src/cli.rs` – the command line, shared by `src/bin/keen4.rs`,
  `keen5.rs`, `keen6.rs`. Without `--data` the data directory is also
  looked up next to the executable, in `data/` there and in a macOS app
  bundle (`GameData::write_dir`: saves/config then go to Application
  Support).
- Release builds: `.github/workflows/release.yml` (on a published
  release; original data from archive.org, cached per half year, for the
  tests/demo checks; only Keen 4 – shareware – is bundled, plus the
  GeneralUser GS SoundFont, pinned commit + sha256, found by
  `find_soundfont`). `icon.png` is
  the window icon, the Windows exe icon (`build.rs`) and the macOS app icon
  (`tools/make_icns.py`, `tools/make_app.sh`).
- `editor/` – Commander Keen Editor (`keen-editor`, workspace member, all
  three episodes,
  eframe/egui + rfd): `project.rs` map set load/save/play test (runs the
  `keen6` binary next to it with `--maps`), `canvas.rs` map view and tools,
  `palette.rs`, `edit.rs` brushes/fill/resize/undo, `info.rs` info plane
  values per episode and sprite previews (runs `scan_info_plane` on a tiny
  map in a headless `Game` of the episode), `render.rs` tile atlases. Testing aid:
  `KEEN_EDITOR_SHOT=file.png [KEEN_EDITOR_SETUP=layer=2,zoom=2,…]` saves
  the editor's own screenshot and exits.
- `tools/` – Python helpers (run with `python3 -I`, `EP=4|5|6` selects the
  episode, default 6): `unlzexe.py`, `gen_states.py`, `dis.py` (ndisasm
  wrapper), `dg.py` (DGROUP dump), `jt.py` (jump tables), `find_states.py`,
  `name_states.py`, `match_funcs.py` (K4/K5 functions vs K6), `xlate.py`
  (K6 names for K4/K5 globals/functions → `re/kN/xdecomp`), `xdiff.py` and
  `immdiff.py` (how a K6 function differs in K4/K5), `explore.py`.
- `re/` – analysis notes: `symbols.md` (K6 globals/functions),
  `PORTING_GUIDE.md` (how to port a function), `PORTING_GUIDE_EP.md`
  (Keen 4/5 specifics), `states.txt`. Git-ignored: `re/KEEN6C_unpacked.exe`,
  `re/decomp/*.c` (REA pseudo-C, one file per function, named by linear
  address), `re/modules/`, `re/k4/`, `re/k5/` (the same for Keen 4/5).

## Conventions
- Faithfulness first: keep the original order of operations, 16-bit
  semantics (`as u16` for unsigned compares, shifts as in the asm), integer
  constants and object list order, so the original demos replay identically.
- Every ported function gets a doc comment with its original name and linear
  address, e.g. `/// KeenWalkThink (0x1a893)`.
- Coordinates are global units (1/16 pixel, 256 per tile) like the original.
- Rendering always uses logical 320x200 coordinates; never assume scale 1.
  Widescreen: during the game loop (not in demos) the surface can be wider
  (`Camera::view`, `Game::update_view`); logical drawing is offset by
  `Surface::xoff` so 320-wide screens/windows stay centered, while the play
  field and score box draw at xoff 0. The original 320 code paths stay
  unchanged so the demos keep in sync. Outside the game loop only the ID
  intro (Terminator, `ega_render_wide` paints beyond the 320 EGA view) and
  the control panel (`Surface::extend_edges` continues the arm) and the
  Keen 4/5 help screens (background only) go wide
  (`Game::set_screen_width`); title, story etc. stay 320. Zoom (extended
  option ZOOM, `-`/`+`, `--zoom`, `Options::zoom`, `Game::zoom()`): the
  view is also taller (`Camera::view_h`, `tall()`, `extra_y()`; 400 lines
  at 50 %), 320x200 drawing is centered vertically with `Surface::yoff`;
  vertical scroll blocks/centering follow the wide rules, `fall_line()`
  keeps the original death line. `MARKER_TILES` (rf.rs: BLOCK, EDGE /
  BOTTOM OF MAP foreground tiles) are drawn black in wide/zoomed views.
  The room around the player (masking, centering) uses `room_pos()`,
  kept inside the map's edge blocks; "on the world map" checks for
  drawing use `ca_mapon` (gamestate.mapon changes a frame early).
  The help menu's
  hand can point at the HELP! title: `NEW_HELP` in text.rs, the pages about
  this version (keep it current when adding features).
- Multiplayer mode, up to 4 players (`src/game/mp.rs`, EXTENDED OPTIONS >
  PLAYERS or CONFIGURE > USE GAMEPAD > PLAYERS, `mp_next_players`: 1..as
  many as the pads allow; config `players` (old `twoplayers`); changed in
  a level it applies at once, `mp_apply_mode`; 2 by itself when two pads
  show up in the first seconds, `Input::two_pads_at_start`,
  `mp_auto_enable`). Not in demos, not with --classic, needs a gamepad;
  `mp_count()` = players actually playing (at most pads + 1). Slots:
  0 = the leading Keen (`player`, list head, camera), 1-3 = the others
  (`MpState::slots[k].obj`, their globals in `slots[k].vars`);
  `MpState::person[slot]` = the player playing it (a permutation, saved;
  identity until a handover, restored by `mp_unflip` at level setup).
  `Game::with_slot(k, f)` swaps slot k's globals (`player`, `c`, buttons,
  `gs.score`/`ammo`/`riding`, `invincible`, ...) in (`mp.cur`), so the
  ported code runs unchanged for him; the play loop calls the others'
  think/contact/tiles/react through it. Input: person n = pad n, the
  first person without a pad = keyboard (`Input::players`,
  `keyboard_player`, `read_control_of`); on the world map the input is
  split too, so only the leading player walks; the others are sprites
  following his path (`mp_world_tick`, `RfState::extra`). When the leader
  dies, `mp_handover` gives the lead to the first living slot (exchanging
  the Keen objects' contents, globals, persons). The leader's doors bring
  the others along. Shots carry `Obj::owner` (person + 1); another
  player's shot makes Keen dizzy (knocked off the pogo stick; on the
  ground drawn sitting with Keen 6's stunned sprite, `Gfx::stunned`,
  which Keen 4/5 borrow from a KEEN6 directory next to theirs). The Keens
  that don't lead are drawn behind and 2 pixels higher
  (`SpriteSlot::behind`, `ALTYOFFSET`). Colors per person
  (`Gfx::alt_sprite(n, idx)`, `ALT_COLORS`, `SpriteSlot::alt`), score
  boxes per person (`mp_status`). A new game starts with 3 lives per
  player. Saves get an optional `MP4` trailer (old `MP2` still loads).
  In extended mode the inventory (gems, K5 keycard, K6 special items) is
  shown at the bottom right (`draw_items_box`, extended option SHOW
  INVENTORY, on by default; on the world map only the K6 special items).
- Battle mode (`src/game/battle.rs`, NEW GAME > BATTLE, menu cards
  BATTLECARD..BATTLETIMECARD): on top of the multiplayer mode
  (`Game::battle_on()`, `BattleState`; `mp_count` uses `battle.players`,
  the config's `players` stays). Started by `UC_BATTLE` -> `new_game` ->
  `battle_begin`; `battle_setup_level` (monsters removed / remembered,
  per episode `BATTLE_KEEP`/`BATTLE_REMOVE` in ep.rs), `battle_tick`
  (clock, items back after 10 s, hard monsters wake up), `battle_guard`
  (only the time ends the level, `EX_BATTLEOVER`), results on the high
  score screen (`battle.results`, `draw_battle_results`). Doors blocked in
  `check_enter_house`; no saving, zoom 50.
- `--classic` (`Options::classic`) switches the additions off: it implies
  `classic_view` and `no_auto_save`, hides MIDI MUSIC and the volume bars
  of the sound/music cards and disables
  quick save (gamepad save/load state, load menu page 2, F2/F3)
  and extended mode (`Options::cheatmode`, otherwise always on: EXTENDED
  OPTIONS card in place of END GAME, which moves into that card, F2/F3,
  inventory box).
  New non-original features should respect it.
- In-game help: every new or changed feature also goes into the help
  pages about this version (`NEW_HELP` in src/game/text.rs, F1 in Keen 4/5,
  hand on the HELP! title), together with README.md. Check the pages for
  outdated information when features change, and that each page still
  fits on the screen (headless screenshots of the help).
- Original globals live in `Game` (src/game/mod.rs); module-private globals
  go in the module's `ModState`.
- Don't commit game data or anything derived from the executable's code.

## Build / run
- `cargo build --release` (game), `cargo build --release --workspace`
  (game + editor)
- `./target/release/keen4|keen5|keen6 [--data DIR]` (options: `--help`)
- Testing without a window: `--headless`, env `KEEN_SHOT_DIR=dir
  KEEN_SHOT_EVERY=n KEEN_MAX_FRAMES=n` saves PNG frames; `--demo N` replays
  original demo N; `--level N` starts a map directly.
- `--dump DIR` exports all graphics as PNG (base for high-res packs).
- Keen 6 must stay identical: the four original demos are compared with
  reference traces (`KEEN_TRACE=file keen6 --demo N`, see
  claude_history.log).
- Re-analysis: `python3 -I tools/unlzexe.py KEEN6/KEEN6C.EXE re/KEEN6C_unpacked.exe`
  (K4: `re/k4/KEEN4E_unpacked.exe`, K5: `re/k5/KEEN5E_unpacked.exe`), then
  open it with REA (`open_binary`) and `batch_decompile`;
  `tools/split_decomp.py OUTDIR result.json` splits the results.

## Remastering hooks
- `--gfx-pack DIR`: same file names as `--dump` output, each image scaled by
  the integer factor in `DIR/pack.txt` (`scale=N`); missing files fall back
  to the original graphics.
- `--audio-pack DIR`: `sfx/NN.ogg|wav|flac|mp3` replaces sound NN,
  `music/NN.ogg|...|mid` replaces song NN (`--soundfont FILE.sf2` for MIDI);
  `sfx/kN/`, `music/kN/` (N = 4, 5, 6) are per episode and come first.
- MIDI MUSIC (CONFIGURE > MUSIC, `SmMode::Midi`): songs converted to
  General MIDI and played through the SoundFont (`--soundfont` or the first
  `.sf2` found, loaded on first use); `--export-midi DIR` writes them as an
  editable audio pack. Music and sound effects run on separate emulated
  OPL chips so each has its own volume (VOLUME bars, `config.txt`).

## Work log
Major steps and decisions are recorded in `claude_history.log`.
