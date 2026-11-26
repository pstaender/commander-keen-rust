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

The [release downloads](https://github.com/pstaender/commander-keen-rust/releases)
(Linux, Windows, macOS) include the shareware episode Keen 4 in `data/KEEN4`
(on macOS inside `Keen 4.app`), so Keen 4 is playable right away. For Keen 5
and 6 put your files in `data/KEEN5` / `data/KEEN6` (or use `--data DIR`).
The downloads also include the SoundFont
[GeneralUser GS](https://www.schristiancollins.com/generaluser) by S.
Christian Collins (its license is in `data/`, on macOS in `Keen 4.app`) for
MIDI MUSIC. On macOS the apps are not notarized: open them the first time with right
click > Open. Saves of the bundled Keen 4 go to
`~/Library/Application Support/Commander Keen/KEEN4`.

## Additonial features

Play in widescreen mode:

<img width="1840" height="864" alt="widescreen" src="https://github.com/user-attachments/assets/7a850433-dee2-4e36-bdd2-6334ec218613" />

Adding additional lives and armor, unlimited lives and ammo (the score box then leaves those counters empty), zoom out the play field (see [Zoom](#zoom)), choose the number of players (see [Multiplayer](#multiplayer)) and replay completed levels with the extended options (EXTENDED OPTIONS in the main menu, two pages of the OPTIONS MENU turned with the arrow keys like LOAD GAME, the first with SHOW INVENTORY, ZOOM, PLAYERS, UNLIMITED LIVES, UNLIMITED AMMO and ALLOW RE-ENTER; they also hold THE KEENEST – the game's high score screen, also in the middle of a game, shown without Keen and the monsters until a key is pressed, like the results after a battle – and END GAME; the options are remembered in `config.txt`; off with `--classic`):

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

Put the game files in a directory (default `KEEN4/`, `KEEN5/` or `KEEN6/`
in the current directory, next to the executable or in `data/` next to it)
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
| `--data DIR` | directory with the original game files (default `KEEN4`, `KEEN5` or `KEEN6`, after the executable's name; also looked up next to the executable and in `data/` there) |
| `--gfx-pack DIR` | high-resolution graphics replacement pack (see below) |
| `--audio-pack DIR` | replacement sound effects and music (see below) |
| `--soundfont FILE` | SoundFont (`.sf2`) for the MIDI music and `.mid` replacement music (default: the first `.sf2` found, see below) |
| `--export-midi DIR` | write all songs as `DIR/music/NN.mid` plus `DIR/instruments.txt` and exit |
| `--window-scale N` | window size multiplier (default 3) |
| `--no-fullscreen` | start in a window; by default the game starts fullscreen, also with `--classic` |
| `--no-aspect` | don't stretch 320×200 to 4:3 |
| `--classic` | the game as it was: no widescreen (play field, intro, menu), no MIDI music, no volume settings, no autosave and no quick save (gamepad save/load state, load menu page 2, F2/F3), no extended options |
| `--classic-view` | keep the original 320×200 play field instead of widening it to the screen (and no zoom) |
| `--zoom PERCENT` | play field zoom: `100`, `75` (default) or `50` (see [Zoom](#zoom)) |
| `--no-auto-save` | turn off the autosave (every 5 s to `saves/AUTOSAVE.CKn`; after a crash, kill or quit the game offers to resume it at the next start) |
| `--system-shortcuts` | macOS: keep the system keyboard shortcuts active while the game has focus (see below) |
| `--nosound` | disable audio output |
| `--level N` | start directly in map N (0 = world map) |
| `--difficulty D` | difficulty for `--level`: `easy`, `normal` (default) or `hard` |
| `--maps DIR` | load `GAMEMAPS.CKn` (and `MAPHEAD.CKn`) from DIR, e.g. levels made with the Commander Keen Editor |
| `--demo N\|FILE` | play the original demo N (0–3, 4 = high score screen), or a demo file recorded with F10+D, and exit |
| `--debug-shortcuts` | enable the F10 debug keys (alias `--debug`) |
| `--dump DIR` | export all original graphics as PNG files and exit |
| `--export-sfx DIR` | export all original sound effects as `DIR/NN-adlib.wav` and `DIR/NN-pc.wav` and exit (e.g. to compare with an audio pack) |
| `--headless` | run without a window (automated testing) |

### Controls

The defaults are the original ones and can be changed in the in-game menu
(F1/Esc → Configure → Keyboard):

| Key | Action |
|---|---|
| Arrow keys (Home/PgUp/End/PgDn for diagonals) | move, look up/down, enter doors |
| Enter | status window |
| Esc / F4–F7 | control panel (new game, load/save, options, …); in Keen 6 also F1, with `--classic` also F2/F3 |
| F1 (Keen 4/5) | help screens; move the hand up onto the HELP! title and press Enter for a help section about everything new in this version (not with `--classic`) |
| `-` / `+` | zoom the play field out / in (100, 75, 50 %; not with `--classic`) |
| Pause | pause |
| Ctrl+Q | quit |
| F11 or Cmd+F (macOS) / Win+F | toggle fullscreen |

On the world map, press jump (or fire) on a level entrance to enter it.

macOS: Ctrl+arrow keys are system shortcuts (switch Spaces, Mission
Control), so with Ctrl as the jump key the arrow presses would be swallowed
while jumping. The game turns the system shortcuts off while its window has
focus. They come back when you switch to another app, and while Cmd is held,
so Cmd+Tab and Cmd+Space keep working. `--system-shortcuts` disables this.

### Gamepad

Gamepads (USB or Bluetooth) work without calibration. Default layout
(Xbox style names; on a SNES style pad A/B/X/Y are the buttons at those
positions):

| Button | In the game | In the menus |
|---|---|---|
| D-pad (the left stick always works too) | move | move the cursor |
| A | jump | select (Enter) |
| B | pogo | back (Esc) |
| X | fire | Y (yes) in the Y/N questions |
| Start | open the control panel | back |
| Select | – | back |
| LB | quick save | – |
| RB | load the quick save | – |

**Configure:** with a gamepad connected, CONFIGURE in the control panel
shows USE GAMEPAD (instead of the joystick entries) with

- CONFIGURE MOVEMENT: up, down, left, right
- CONFIGURE BUTTONS: jump, pogo, fire, menu, save state, load state
- DEFAULT LAYOUT: back to the table above

Select an entry and press the button (or push the stick / d-pad) for it.
Esc cancels. Space, Backspace or pressing the button that is already bound
there clears the entry. A button that another action uses is swapped: the
other action gets the old button of this one. The layout is stored in
`config.txt`.

Pads the gamepad library doesn't know (many cheap SNES style pads) work as
well: their d-pad/stick moves, in the menus the four face buttons select
and the others go back, and every button can be bound as above. If a pad
does not react, `KEEN_PAD_DEBUG=1` prints what it sends.

Nintendo Switch Pro Controller on macOS: macOS puts it into a report mode
the gamepad library misreads (buttons seemingly pressed all the time), so
the game reads it with its own driver (`src/engine/procon.rs`, USB or
Bluetooth). Its buttons count by position like on the other pads: B is A
(bottom), A is B (right), Y is X (left), X is Y (top).

### Multiplayer

Up to four players, each with his own Keen in the levels. Choose the number
under EXTENDED OPTIONS > PLAYERS or CONFIGURE > USE GAMEPAD > PLAYERS
(remembered in `config.txt`); it is set to 2 by itself when two gamepads are
plugged in at the start. Every player plays with his own gamepad, the first
one without a gamepad with the keyboard:

| Players | Needs | Player 1 | Player 2 | Player 3 | Player 4 |
|---|---|---|---|---|---|
| 2 | 1 gamepad | gamepad 1 | gamepad 2 or keyboard | | |
| 3 | 2 gamepads | gamepad 1 | gamepad 2 | gamepad 3 or keyboard | |
| 4 | 3 gamepads | gamepad 1 | gamepad 2 | gamepad 3 | gamepad 4 or keyboard |

- Player 2 wears a blue helmet with light pink stripes and a green shirt, player 3
  a red helmet with dark red stripes and a cyan shirt, player 4 a light
  green helmet with dark green stripes and a red shirt; all of them blue jeans. Each
  has a score box in his colors.
- The view follows the leading Keen (player 1). The others are brought back
  to him when they stay off screen, and come along through his doors.
- When the leading Keen dies, the next living one takes the lead; the fallen
  Keen comes back for one of the shared lives (a new game starts with three
  lives per player).
- Keys, gems and lives are shared, shots found go to everybody, each Keen
  keeps his own score. A stunner shot makes another Keen dizzy; when it
  hits the leading Keen, the one who fired takes the lead (the view
  follows him).
- On the world map only the leading player walks; the others follow him.
- Changing the number in the middle of a level applies at once. Not in
  demos and not with `--classic`.

### Battle

NEW GAME > BATTLE: two to four players play one level against each other
for a set time (the gamepads as for multiplayer). The menu takes you through
the steps: the number of players, the difficulty, the level (RANDOM or one
of the levels, left / right arrow keys) and the time (1 to 30 minutes, 5 by
default, left / right).

- **Points** for bonus items (they come back 10 seconds after they were
  taken) and for stunning monsters (200). Shots found are only yours.
- **Easy:** no monsters. **Normal:** monsters you stun stay stunned.
  **Hard:** stunned monsters wake up again after 5 seconds.
- **Knock-outs:** a stunner shot that hits another Keen makes him dizzy;
  his score box counts it where the lives are shown (0, -1, -2, ...).
  While he is dizzy, more hits don't count. If
  the screen was following him, it follows the one who fired now.
- Nobody leaves the level: doors and exits are closed, the objects that
  would end the level are gone. A Keen who dies comes back at once.
- The time left is shown at the top (MM:SS, the last 10 seconds blink).
- When the time is up, the high score screen shows the results: each
  Keen's helmet in his colors, how often he was knocked out ("3x knocked
  out") and his score, the best first.
- Always played at 50 % zoom, can't be saved. Not with `--classic`.

### Saves and settings

- **Saved games:** LOAD GAME / SAVE GAME in the control panel, six slots in
  `saves/SAVEGAMn.CKn` inside the data directory. This port uses its own
  format, so saves from the DOS version can't be loaded.
- **Quick save:** the gamepad's save state button (LB) saves to
  `saves/STATE.CKn` without asking, load state (RB) goes back to it. It is
  separate from the six slots.
- **Autosave:** while you play, the game is saved every 5 seconds to
  `saves/AUTOSAVE.CKn`, and once more when you quit in the middle of a game
  (not while Keen is dying, so it always holds a moment before). After a
  crash, kill or quit, the next start asks "Resume your last game?" (Y /
  N, on a gamepad A / B). The autosave is removed when the game ends (game
  over, won, END GAME) or a new game starts. `--no-auto-save` turns it off.
- **Load menu:** the second page of LOAD GAME has the quick save and the
  autosave. Go down past the last slot or up past the first one (or press
  left/right) to turn the page.
- F2 saves to and F3 loads the slot last saved or loaded, without asking;
  before a slot was used they save / load the quick save (the gamepad's
  SAVE STATE) (not with `--classic`).
- Settings (sound mode, key bindings, gamepad) and high scores are stored in
  `config.txt` in the data directory, so each episode has its own.

## Widescreen

The play field adapts to the shape of the window or screen. The height stays
at the original 200 lines, and the width grows from 320 up to 864 pixels
(32:9), so a 16:9 screen shows about 426 pixels: more of the level to the
left and right. Resizing the window or toggling fullscreen updates the view
immediately.

- The status window and dialogs stay 320×200 and are centered. The score
  box stays in the top left corner.
- The ID Software intro (COMMANDER KEEN flying in and zooming out) and the
  control panel fill the whole width; the arm around the menu computer is
  continued to the edges. The Keen 4/5 help screens fill the width with
  their background, the pages stay centered. The title, story and the
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

## Zoom

The play field is zoomed out to 75 % by default, which shows more of the
level in every direction (at 50 % on a 16:9 screen about 852×400 pixels
instead of 426×200). Changing the number of players sets the zoom to 50 %
for two or more players and back to 75 % for one. Press `-` and `+` while
playing, or change ZOOM in EXTENDED OPTIONS (100, 75, 50 %); the setting is
remembered in `config.txt`. `--zoom 50` sets it for one session only.

- It works like the widescreen view in both directions: enemies wake up
  when they come into the larger view, and rooms smaller than the view are
  centered with black around them. Falling out of the map happens at the
  same place as at 100 %.
- The marker tiles the level designers left where the original view never
  looks ("BLOCK", "EDGE OF MAP", "BOTTOM OF MAP") are drawn black in the
  wide and zoomed out views.
- The score box, the inventory, dialogs and the status window keep their
  pixel size, so they look smaller on screen. The status window hangs from
  the top of the view.
- Demos, `--classic` and `--classic-view` always use 100 %.

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
- files in `DIR/sfx/k4/`, `k5/`, `k6/` (and `music/kN/`) are used only by
  that episode and win over `DIR/sfx/`: the sound numbers mean different
  sounds in each episode

Anything not replaced plays through the emulated AdLib/PC speaker.

### MIDI music and volumes

In the control panel, CONFIGURE > MUSIC offers **MIDI MUSIC** after
ADLIB/SOUNDBLASTER. The original songs are AdLib register streams; MIDI
MUSIC converts them to General MIDI and plays them through a SoundFont.
Timing and loop points match the original exactly. The entry is disabled
when no SoundFont is found.

CONFIGURE > SOUND and CONFIGURE > MUSIC each have a **VOLUME** bar (default
100 %): select it and press left / right to change it in 10 % steps. The
choice and both volumes are saved in `config.txt`. With `--classic` the
cards are the original ones (no MIDI, no volume bars).

- **SoundFont.** Pass it with `--soundfont FILE.sf2`. Without that option the
  first `.sf2` in the data directory, next to the executable or in the
  current directory is used, then a system one (e.g.
  `/usr/share/sounds/sf2/FluidR3_GM.sf2`). The free
  [GeneralUser GS](https://github.com/mrbumpy409/GeneralUser-GS) works well.
  Without a SoundFont the game plays the AdLib music.
- **Instruments.** Each AdLib instrument is played once on the emulated OPL2
  and measured: attack, decay, sustain, tone or noise, brightness. That,
  together with the keys it is played in, picks a General MIDI instrument or
  drum. The instruments are guessed, so some will sound wrong. Override them
  in `instruments.txt` of the `--audio-pack` directory, one per line:

  ```
  # [NN:]<patch> <program 0-127>   or   [NN:]<patch> drum <key>
  03:090eda25000100f115000a 29          # song 03 only: overdriven guitar
  000ba84c000000d64f0000 drum 36        # this instrument in every song: bass drum
  ```

  `--export-midi DIR` writes the `instruments.txt` of all songs in this
  format. For each instrument it lists the chosen voice and what was
  measured. Copy the lines you want to change.
- **Editing songs.** The exported `.mid` files can be edited in any
  sequencer. Use DIR as `--audio-pack` with a SoundFont, and the edited
  `music/NN.mid` files are played instead of the converted ones.

## Development and testing aids

Environment variables (mostly for use with `--headless`):

| Variable | Effect |
|---|---|
| `KEEN_SHOT_DIR=dir` | save presented frames as PNG |
| `KEEN_SHOT_EVERY=n` | …every n-th frame (default 35) |
| `KEEN_MAX_FRAMES=n` | quit after n frames |
| `KEEN_KEYS="frame:scancode[+frames],..."` | scripted key presses |
| `KEEN_PAD_DEBUG=1` | print the detected gamepads (name, mapping) and all their events |
| `KEEN_TRACE=file` | log the player's map/x/y/sprite per game frame |
| `KEEN_RENDER_WAV="song:N:file.wav"` | render music (or `midi:N:…` for the MIDI version, `pc:N:…`, `adlib:N:…` for sounds) to WAV and exit |

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
