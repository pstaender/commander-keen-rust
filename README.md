# Commander Keen 4, 5 and 6 – Rust reimplementation

A faithful Rust port of the three "Goodbye Galaxy" episodes of Commander Keen
(id Software 1991):

| Episode | Original executable | Data directory |
|---|---|---|
| *Commander Keen 4: Secret of the Oracle* | `KEEN4E.EXE` v1.4 | `KEEN4/` |
| *Commander Keen 5: The Armageddon Machine* | `KEEN5E.EXE` v1.4 | `KEEN5/` |
| *Commander Keen 6: Aliens Ate My Baby Sitter!* | `KEEN6C.EXE` v1.4 | `KEEN6/` |

The code was rebuilt function by function from the original executables. The
three games share one engine, as in id's source code: the shared modules are
compiled once per episode, and each episode adds its own actors and
specials. The port draws the original 320×200 EGA graphics, plays the
original AdLib and PC speaker sound, and replays the original demos. It also
has hooks for high-resolution graphics and modern audio.

**The game data is not included.** You need the original v1.4 files of the
episodes you want to play: `AUDIO.CKn`, `EGAGRAPH.CKn`, `GAMEMAPS.CKn` and
the executable (`KEEN4E.EXE`, `KEEN5E.EXE` or `KEEN6C.EXE`; `CONFIG.CKn` is
optional). Keen 4 was shareware, Keen 5 and 6 were commercial releases, so
use your own copy. The versions found on archive.org could work as well.

## Additonial features

Play in widescreen mode:

<img width="1840" height="864" alt="widescreen" src="https://github.com/user-attachments/assets/7a850433-dee2-4e36-bdd2-6334ec218613" />

Adding additional lives and armor and replay completed levels using the `--cheatmode`-flag:

<img width="1072" height="864" alt="cheat_options" src="https://github.com/user-attachments/assets/ac6dac4f-fc06-4eba-93f7-c414dfdfd283" />

View all maps and levels, edit existing levels or create new ones with the editor:

<img width="1462" height="1102" alt="level_editor" src="https://github.com/user-attachments/assets/06a9abd9-c6aa-4a7b-b3d4-63f7cde11d32" />

## Build

Requires a recent stable Rust toolchain (edition 2021).

```sh
cargo build --release
```

Dependencies (fetched by cargo): `winit` + `softbuffer` (window, fullscreen), `rodio` (audio output and
ogg/wav/flac/mp3 decoding), `oplon` (OPL2/AdLib emulation), `rustysynth`
(MIDI via SoundFont), `gilrs` (gamepads), `png`.

## Run

Put the game files in a directory (default `KEEN4/`, `KEEN5/` or `KEEN6/`)
and start the matching executable:

```sh
./target/release/keen4            # Keen 4 from KEEN4/
./target/release/keen5 --data DIR # Keen 5 from DIR
./target/release/keen6
```

The episode is detected from the files in the data directory, so every
executable can start every episode with `--data`. File names are matched
case-insensitively.

### Command-line options

| Option | Description |
|---|---|
| `--data DIR` | directory with the original game files (default `KEEN4`, `KEEN5` or `KEEN6`, after the executable's name) |
| `--gfx-pack DIR` | high-resolution graphics replacement pack (see below) |
| `--audio-pack DIR` | replacement sound effects and music (see below) |
| `--soundfont FILE` | SoundFont (`.sf2`) used for `.mid` replacement music |
| `--window-scale N` | window size multiplier (default 3) |
| `--fullscreen` / `--windowed` | start fullscreen or in a window (otherwise the last used mode from `config.txt`) |
| `--no-aspect` | don't stretch 320×200 to 4:3 |
| `--classic-view` | keep the original 320×200 play field instead of widening it to the screen |
| `--system-shortcuts` | macOS: keep the system keyboard shortcuts active while the game has focus (see below) |
| `--nosound` | disable audio output |
| `--level N` | start directly in map N (0 = world map) |
| `--difficulty D` | difficulty for `--level`: `easy`, `normal` (default) or `hard` |
| `--maps DIR` | load `GAMEMAPS.CKn` (and `MAPHEAD.CKn`) from DIR, e.g. levels made with the Commander Keen Editor |
| `--demo N\|FILE` | play the original demo N (0–3, 4 = high score screen), or a demo file recorded with F10+D, and exit |
| `--debug-shortcuts` | enable the F10 debug keys (alias `--debug`) |
| `--cheatmode` | CHEAT OPTIONS replaces END GAME in the main menu: add lives and shots, allow re-entering finished levels |
| `--dump DIR` | export all original graphics as PNG files and exit |
| `--headless` | run without a window (automated testing) |

### Controls

The defaults are the original ones and can be changed in the in-game menu
(F1/Esc → Configure → Keyboard):

| Key | Action |
|---|---|
| Arrow keys (Home/PgUp/End/PgDn for diagonals) | move, look up/down, enter doors |
| Enter | status window |
| Esc / F2–F7 | control panel (new game, load/save, options, …); in Keen 6 also F1 |
| F1 (Keen 4/5) | help screens |
| Pause | pause |
| Ctrl+Q | quit |
| F11 or Cmd+F (macOS) / Win+F | toggle fullscreen |

On the world map, press jump (or fire) on a level entrance to enter it.

macOS: Ctrl+arrow keys are system shortcuts (switch Spaces, Mission
Control), so with Ctrl as the jump key the arrow presses would be swallowed
while jumping. The game turns the system shortcuts off while its window has
focus. They come back when you switch to another app, and while Cmd is held,
so Cmd+Tab and Cmd+Space keep working. `--system-shortcuts` disables this.

Gamepads work without calibration: stick or d-pad to move, A = jump,
B = pogo, X/Y = fire, Start = control panel.

### Debug keys (`--debug-shortcuts`)

With `--debug-shortcuts` (or `--debug`) the original's debug keys are
available: hold **F10** and press a key. In KEEN6C.EXE v1.4 these keys are
always active; the port enables them only on request. In Keen 4 and 5 they
can also be switched on like in the original: hold **A** and **2** while
opening the status window with Enter ("Debug keys active"). Keen 4 and 5
also have the **B+A+T** cheat (all keys, 99 shots, an extra Keen; Keen 5
also the keycard), F10+I gives the wetsuit (Keen 4) or the keycard (Keen 5).

| Key | Function |
|---|---|
| F10+G | god mode |
| F10+I | free items (keys, 99 ammo, sandwich, rope, passcard, 3000 points) |
| F10+J | jump cheat (you can't jump down while it's on) |
| F10+N | no clipping (falling off the bottom of a level still kills you) |
| F10+Y | reveal walls: marks every wall side on the tiles, so fake walls and hidden passages show up (see note) |
| F10+B | border color (0–15) |
| F10+C | number of active/inactive objects |
| F10+D | record a demo: asks for a level, plays it in demo mode, F10+D again stops and saves `DEMOn.CKn` in the data directory (play it with `--demo DEMOn.CKn`) |
| F10+E | end (complete) the current level |
| F10+M | memory usage (the port shows the size of the loaded data) |
| F10+P | save a screenshot to `screenshots/` in the data directory (the original froze the screen and exited to DOS for a screen grab) |
| F10+S | slow motion |
| F10+T | sprite test: Up/Down/PgUp/PgDn select a sprite, Left/Right the shift, Esc leaves |
| F10+V | add 0–8 extra VBLs per frame |
| F10+W | warp to a level (1–18) |
| F10+Z | kill Keen and lose all lives |

Note on F10+Y: the original draws the wall icons from masked tiles
0x6c–0x7b, which Keen 4 contains but Keen 6 ships empty (the original game
would draw garbage). The port draws colored bars instead: north yellow, south
red, east green, west cyan, slopes magenta. They stay until the game exits.

Holding **E+N+D** together during play ends the game with the finale, in
the original too (Keen 4: as if the last council member was rescued; Keen 5:
the QED is destroyed). This doesn't need `--debug-shortcuts`.

### Saves and settings

- Saved games go to `saves/SAVEGAMn.CKn` inside the data directory. This port uses its own format, so
  saves from the DOS version can't be loaded.
- Settings (sound mode, key bindings, gamepad, fullscreen) and high scores are stored in
  `config.txt` in the data directory, so each episode has its own.

## Widescreen

The play field adapts to the shape of the window or screen. The height stays
at the original 200 lines, and the width grows from 320 up to 864 pixels
(32:9), so a 16:9 screen shows about 426 pixels: more of the level to the
left and right. Resizing the window or toggling fullscreen updates the view
immediately.

- The status window, menus and dialogs stay 320×200 and are centered. The
  score box stays in the top left corner. The title, intro, story and the
  other full-screen pictures are shown 4:3 with borders.
- Keen stays in the middle of the screen. Enemies wake up when they come into
  the wider view, so the game plays slightly differently than in the
  original.
- Where a room between scroll blocks is narrower than the view, the room is
  centered and the rest is black, so the wider view doesn't reveal hidden
  areas.
- Recorded demos (attract mode, `--demo`, F10+D) always use the original 320
  width so they stay in sync.
- `--classic-view` keeps the original 320 view everywhere.

## Commander Keen Editor

`editor/` contains a level and world map editor (`keen-editor`) for all three
episodes. You can use it to
change the original levels, make new ones and play test them in the game.

```sh
cargo build --release --workspace
./target/release/keen-editor --data KEEN4|KEEN5|KEEN6 [--maps DIR] [MAP]
```

- **Layers:** background tiles, foreground tiles and the info layer, each
  with its own palette. Enemies, items and platforms show their real
  sprites, taken from the game's own spawn code. Door, teleporter and
  switch targets are drawn as arrows. World map markers (level entrances,
  flags, blockers) have labels.
- **Tools:** draw, rectangle, flood fill, select (copy, cut, clear) and door
  target. Right click picks a tile, right drag picks a block, and dragging
  over a palette takes a block of tiles as the brush. Undo and redo go
  back up to 200 steps.
- **Overlays:** tile properties (walls, slopes, poles, doors, deadly tiles),
  grid, and zoom from 12.5% to 800%.
- **Maps:** create, duplicate, resize (with an anchor), rename and delete
  maps (slots 0–99). Export a map as PNG.
- **Play test:** F5 starts the game on the current map with the unsaved
  changes. F6 starts it with Keen at the tile under the mouse. The game
  exits when the level is done. The difficulty (Easy/Normal/Hard) is
  chosen next to the Play button or in the Test menu and is remembered
  between sessions.
- **Saving:** writes `GAMEMAPS.CKn` and `MAPHEAD.CKn` (RLEW and Carmack
  compressed like the originals) into a folder. Play them with
  `keen6 --data KEEN6 --maps FOLDER` (`keen4`/`keen5` likewise). If you save into the game folder
  itself, the port loads the new maps there by default, and the originals
  are kept as `*.bak`. The DOS executable has its map header built in, so it
  only uses the new `MAPHEAD.CKn` through a patcher, as with other Keen
  modding tools.

Press F1 in the editor for all mouse and keyboard controls.

## Remastering hooks

### High-resolution graphics: `--gfx-pack DIR`

1. `keen6 --data KEEN6 --dump pack` (or `keen4`, `keen5`) exports every original image (pictures,
   sprites, 8×8 and 16×16 tiles, masked tiles) as PNG, plus a `pack.txt`
   template.
2. Upscale or repaint the PNGs you want (32-bit RGBA, any colors). Keep the
   file names and scale each image by the same integer factor, and set it in
   `pack.txt`, e.g. `scale=4`.
3. `keen6 --data KEEN6 --gfx-pack pack`

Missing files fall back to the original graphics. The game logic still runs at
320×200 logical coordinates. Rendering happens at the pack scale, so a
`scale=4` pack renders at 1280×800.

### Audio replacement: `--audio-pack DIR`

- `DIR/sfx/NN.ogg` (or `.wav`, `.flac`, `.mp3`) replaces sound effect NN
- `DIR/music/NN.ogg` (or `.wav`, `.flac`, `.mp3`, `.mid`) replaces song NN
- `.mid` files need `--soundfont FILE.sf2`

Anything not replaced plays through the emulated AdLib/PC speaker.

## Development and testing aids

Environment variables (mostly for use with `--headless`):

| Variable | Effect |
|---|---|
| `KEEN6_SHOT_DIR=dir` | save presented frames as PNG |
| `KEEN6_SHOT_EVERY=n` | …every n-th frame (default 35) |
| `KEEN6_MAX_FRAMES=n` | quit after n frames |
| `KEEN6_KEYS="frame:scancode[+frames],..."` | scripted key presses |
| `KEEN6_TRACE=file` | log the player's map/x/y/sprite per game frame |
| `KEEN6_RENDER_WAV="song:N:file.wav"` | render music (or `pc:N:…`, `adlib:N:…` for sounds) to WAV and exit |

`tools/dosbox_trace.py` (`EP=4|5|6`) and `tools/compare_trace.py` record the
same trace from the original game running in DOSBox Staging
(`--set webserver_enabled=true`) and compare the two, to check that the demos
stay in sync. All original demos of the three episodes (including the high
score screens) replay in sync with the DOS originals.

See `CLAUDE.md` for the code layout and porting conventions, and
`claude_history.log` for the project history.

## Legal

Commander Keen is a trademark of id Software. This project contains no
original game data or code. It only reads the files from a copy you own.

This project is for educational purpose only.
