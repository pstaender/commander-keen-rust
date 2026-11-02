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
  tile info (from MAPHEAD), AUDIO (PC speaker / AdLib / IMF). `episode.rs`
  holds the per-episode layout (file names, chunk counts, header offsets in
  the executables); the episode is detected from the data directory.
  `exe.rs` locates the headers linked into the executable.
- `src/engine/` – platform layer (ID_* managers): `sys.rs` window/time
  (winit + softbuffer, events pumped from the game loop, letterboxed
  scaling, F11 fullscreen, 70 Hz TimeCount), `video.rs` scalable 32-bit surface in logical
  320x200 coordinates + EGA palette fades, `gfx.rs` texture sets and
  high-res replacement packs, `sound.rs` ID_SD emulation (560/140 Hz timing,
  OPL2 via `oplon`, PC speaker) with replacement audio (rodio, rustysynth),
  `input.rs` scan codes / ControlInfo.
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
  `keen5.rs`, `keen6.rs`.
- `editor/` – Commander Keen Editor (`keen-editor`, workspace member, all
  three episodes,
  eframe/egui + rfd): `project.rs` map set load/save/play test (runs the
  `keen6` binary next to it with `--maps`), `canvas.rs` map view and tools,
  `palette.rs`, `edit.rs` brushes/fill/resize/undo, `info.rs` info plane
  values per episode and sprite previews (runs `scan_info_plane` on a tiny
  map in a headless `Game` of the episode), `render.rs` tile atlases. Testing aid:
  `KEEN6_EDITOR_SHOT=file.png [KEEN6_EDITOR_SETUP=layer=2,zoom=2,…]` saves
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
  unchanged so the demos keep in sync.
- Original globals live in `Game` (src/game/mod.rs); module-private globals
  go in the module's `ModState`.
- Don't commit game data or anything derived from the executable's code.

## Build / run
- `cargo build --release` (game), `cargo build --release --workspace`
  (game + editor)
- `./target/release/keen4|keen5|keen6 [--data DIR]` (options: `--help`)
- Testing without a window: `--headless`, env `KEEN6_SHOT_DIR=dir
  KEEN6_SHOT_EVERY=n KEEN6_MAX_FRAMES=n` saves PNG frames; `--demo N` replays
  original demo N; `--level N` starts a map directly.
- `--dump DIR` exports all graphics as PNG (base for high-res packs).
- Keen 6 must stay identical: the four original demos are compared with
  reference traces (`KEEN6_TRACE=file keen6 --demo N`, see
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
  `music/NN.ogg|...|mid` replaces song NN (`--soundfont FILE.sf2` for MIDI).

## Work log
Major steps and decisions are recorded in `claude_history.log`.
