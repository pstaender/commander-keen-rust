# MJSFX sound effects

Replacement sound effects for Keen 4, 5 and 6, designed with
[MJSFX](https://mjsfx.app) as one chip-sound family (square and triangle
voices, filtered noise, a little room on pickups and fanfares). Each
replacement keeps roughly the length and pitch direction of the original,
because the game waits for some sounds to finish.

    ./target/release/keen6 --audio-pack audio-packs/mjsfx

- `sfx/NN.wav`: sounds all three episodes share (Keen's footsteps, jump,
  landing, shot, pogo, pickups, death, level done, status window, flags …)
- `sfx/k4/`, `sfx/k5/`, `sfx/k6/`: the episode's own sounds (enemies,
  doors, explosions, Paddle War …); the numbers mean different sounds in
  each episode

Sounds without a file play as before. The sounds are made by
`tools/mjsfx/design.py`: one function per sound. It needs the MJSFX app,
renders every sound, saves it to the MJSFX library under `Keen/` and
writes `rendered.json` (run it from `tools/mjsfx/`, then copy the files as
listed in `rendered.json` into `sfx/`).
