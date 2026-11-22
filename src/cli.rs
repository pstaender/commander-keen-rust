//! Command line front end shared by the keen4/keen5/keen6 executables.

use crate::assets::{Episode, GameData};
use crate::engine::gfx::{pack_scale, Gfx};
use crate::engine::sound::SoundManager;
use crate::engine::sys::Sys;
use crate::options::Options;
use std::path::PathBuf;

const USAGE: &str = "keenN [options]
  --data DIR         directory with the original Keen 4, 5 or 6 files (the
                     episode is detected from the files; default: KEENN)
  --gfx-pack DIR     high resolution graphics replacement pack
  --audio-pack DIR   replacement sound effects (sfx/NN.ogg) and music (music/NN.ogg|.mid)
  --soundfont FILE   SoundFont (.sf2) for the MIDI music (CONFIGURE >
                     MUSIC > MIDI MUSIC) and .mid files of an audio pack;
                     default: the first .sf2 in the data directory, next
                     to the executable or in the current directory
  --export-midi DIR  write the songs as DIR/music/NN.mid (DIR can be used
                     as --audio-pack after editing them) and exit
  --window-scale N   window size multiplier (default 3)
  --no-fullscreen    start in a window (default: fullscreen; F11 or
                     Cmd/Win+F toggles)
  --no-aspect        do not stretch to 4:3
  --classic          the game as it was: no widescreen, no MIDI music, no
                     volume settings, no autosave and no quick save
                     (gamepad save/load state, load menu page 2, F2/F3)
                     and no extended options
  --classic-view     keep the original 320x200 play field (otherwise it
                     widens to the window's shape, e.g. 16:9)
  --system-shortcuts keep the macOS keyboard shortcuts (Ctrl+arrows switch
                     Spaces) while the game window has focus
  --nosound          disable audio output
  --no-auto-save     do not save the running game every 5 seconds (by
                     default it is saved to saves/AUTOSAVE.CKN and offered
                     to resume at the next start)
  --level N          start directly in map N
  --difficulty D     difficulty for --level: easy, normal (default), hard
  --maps DIR         load GAMEMAPS.CKn (and MAPHEAD.CKn) from DIR, e.g.
                     levels made with the Commander Keen Editor
  --demo N|FILE      play the original demo N (0-3), or a demo file
                     recorded with F10+D (DEMOn.CKn), and exit
  --debug-shortcuts  enable the F10 debug keys (alias: --debug)
  --dump DIR         export the original graphics as PNG files and exit
  --export-sfx DIR   write the original sound effects as DIR/NN-adlib.wav
                     and DIR/NN-pc.wav and exit
  --headless         run without a window (testing)";

/// `main` of the executables: `default_dir` is the data directory used
/// without --data (KEEN4, KEEN5 or KEEN6).
pub fn main(default_dir: &str) {
    let args: Vec<String> = std::env::args().collect();
    let mut data_dir = PathBuf::from(default_dir);
    let mut dump_dir: Option<PathBuf> = None;
    let mut midi_dir: Option<PathBuf> = None;
    let mut sfx_dir: Option<PathBuf> = None;
    let mut gfx_pack: Option<PathBuf> = None;
    let mut audio_pack: Option<PathBuf> = None;
    let mut soundfont: Option<PathBuf> = None;
    let mut window_scale = 3usize;
    let mut aspect = true;
    let mut sound = true;
    let mut system_shortcuts = false;
    let mut maps_dir: Option<PathBuf> = None;
    let mut headless = false;
    let mut fullscreen = true;
    let mut opts = Options::default();
    let mut i = 1;
    let next = |i: &mut usize| -> String {
        *i += 1;
        args.get(*i).cloned().unwrap_or_default()
    };
    while i < args.len() {
        match args[i].as_str() {
            "--data" => data_dir = PathBuf::from(next(&mut i)),
            "--dump" => dump_dir = Some(PathBuf::from(next(&mut i))),
            "--gfx-pack" => gfx_pack = Some(PathBuf::from(next(&mut i))),
            "--audio-pack" => audio_pack = Some(PathBuf::from(next(&mut i))),
            "--soundfont" => soundfont = Some(PathBuf::from(next(&mut i))),
            "--export-midi" => midi_dir = Some(PathBuf::from(next(&mut i))),
            "--export-sfx" => sfx_dir = Some(PathBuf::from(next(&mut i))),
            "--window-scale" => window_scale = next(&mut i).parse().unwrap_or(3),
            "--no-aspect" => aspect = false,
            "--nosound" => sound = false,
            "--system-shortcuts" => system_shortcuts = true,
            "--headless" => headless = true,
            "--debug" | "--debug-shortcuts" => opts.debug = true,
            "--no-fullscreen" => fullscreen = false,
            "--classic-view" => opts.classic_view = true,
            "--classic" => opts.classic = true,
            "--no-auto-save" => opts.no_auto_save = true,
            "--difficulty" => {
                opts.start_difficulty = match next(&mut i).to_ascii_lowercase().as_str() {
                    "easy" | "1" => Some(1),
                    "normal" | "2" => Some(2),
                    "hard" | "3" => Some(3),
                    v => {
                        eprintln!("unknown difficulty {} (easy, normal or hard)", v);
                        None
                    }
                }
            }
            "--maps" => maps_dir = Some(PathBuf::from(next(&mut i))),
            "--level" => opts.start_level = next(&mut i).parse().ok(),
            "--demo" => {
                let v = next(&mut i);
                match v.parse() {
                    Ok(n) => opts.play_demo = Some(n),
                    Err(_) => opts.demo_file = Some(PathBuf::from(v)),
                }
            }
            "-h" | "--help" => {
                println!("{}", USAGE);
                return;
            }
            other => eprintln!("unknown option {}", other),
        }
        i += 1;
    }
    if opts.classic {
        opts.classic_view = true;
        opts.no_auto_save = true;
    }
    // extended mode (EXTENDED OPTIONS in the main menu, F2/F3 quick save/load,
    // inventory box) is on unless --classic
    opts.cheatmode = !opts.classic;
    let data = match GameData::load_with_maps(&data_dir, maps_dir.as_deref()) {
        Ok(d) => d,
        Err(e) => {
            eprintln!("error: {}", e);
            std::process::exit(1);
        }
    };
    if let Ok(spec) = std::env::var("KEEN_RENDER_WAV") {
        // "song:N:file.wav", "midi:N:file.wav" (MIDI music) or
        // "pc:N:file.wav" / "adlib:N:file.wav"
        let p: Vec<&str> = spec.split(':').collect();
        let n: usize = p.get(1).and_then(|v| v.parse().ok()).unwrap_or(0);
        let path = std::path::Path::new(p.get(2).copied().unwrap_or("out.wav"));
        let r = match p[0] {
            "song" => crate::engine::sound::render_to_wav(&data.audio, Some(n), None, 20.0, path),
            "midi" => {
                let sf = soundfont.clone().or_else(|| crate::engine::sound::find_soundfont(&data_dir)).expect("MIDI music needs a SoundFont (--soundfont FILE)");
                let table = load_instrument_table(audio_pack.as_deref());
                crate::engine::sound::render_midi_to_wav(&data.audio, n, &sf, &table, 20.0, path)
            }
            "pc" => crate::engine::sound::render_to_wav(&data.audio, None, Some((n, crate::engine::sound::SdMode::PC)), 2.0, path),
            _ => crate::engine::sound::render_to_wav(&data.audio, None, Some((n, crate::engine::sound::SdMode::AdLib)), 2.0, path),
        };
        r.expect("render failed");
        return;
    }
    if let Some(d) = sfx_dir {
        let n = crate::engine::sound::export_sfx(&data.audio, &d).expect("sound export failed");
        println!("exported {} sound effects to {}", n, d.display());
        return;
    }
    if let Some(d) = midi_dir {
        export_midi(&data, &d, audio_pack.as_deref()).expect("MIDI export failed");
        return;
    }
    if let Some(d) = dump_dir {
        crate::dump::dump(&data, &d).expect("dump failed");
        println!("dumped to {}", d.display());
        return;
    }
    let scale = gfx_pack.as_deref().map(pack_scale).unwrap_or(1);
    let gfx = Gfx::new(&data.gfx, scale, gfx_pack.as_deref());
    if !opts.classic && soundfont.is_none() {
        soundfont = crate::engine::sound::find_soundfont(&data_dir);
    }
    let mut sd = SoundManager::new(&data.audio, sound && !headless, audio_pack.as_deref(), soundfont.as_deref());
    sd.set_episode(match data.episode {
        Episode::K4 => 4,
        Episode::K5 => 5,
        Episode::K6 => 6,
    });
    if !opts.classic {
        sd.allow_midi(load_instrument_table(audio_pack.as_deref()));
    }
    let mut sys = Sys::new(scale, window_scale.max(1), headless, aspect, fullscreen);
    sys.set_grab_hotkeys(!system_shortcuts);
    match data.episode {
        Episode::K4 => crate::ck4::run(sys, data, gfx, sd, opts),
        Episode::K5 => crate::ck5::run(sys, data, gfx, sd, opts),
        Episode::K6 => crate::ck6::run(sys, data, gfx, sd, opts),
    }
}

/// `--export-midi DIR`: every song as DIR/music/NN.mid plus a report of the
/// instruments and the General MIDI programs chosen for them.
fn export_midi(data: &crate::assets::GameData, dir: &std::path::Path, audio_pack: Option<&std::path::Path>) -> std::io::Result<()> {
    use crate::assets::midi;
    let music = dir.join("music");
    std::fs::create_dir_all(&music)?;
    let mut report = String::from(
        "# Instruments of the songs and the General MIDI voices the MIDI music uses for them.\n\
         # Copy lines to instruments.txt of an --audio-pack and change the voice:\n\
         # [NN:]<patch> <program 0-127>  or  [NN:]<patch> drum <key>   (NN: only in song NN)\n\n",
    );
    let table = load_instrument_table(audio_pack);
    for (n, song) in data.audio.songs.iter().enumerate() {
        if song.commands.is_empty() {
            continue;
        }
        std::fs::write(music.join(format!("{:02}.mid", n)), midi::song_to_midi(song, n, &table))?;
        report.push_str(&midi::song_report(song, n, &table));
    }
    std::fs::write(dir.join("instruments.txt"), report)?;
    println!("exported {} songs to {}", data.audio.songs.len(), music.display());
    Ok(())
}

/// The instrument overrides of the MIDI music: `instruments.txt` of the
/// audio pack.
pub fn load_instrument_table(audio_pack: Option<&std::path::Path>) -> crate::assets::midi::InstrumentTable {
    audio_pack
        .and_then(|d| std::fs::read_to_string(d.join("instruments.txt")).ok())
        .map(|t| crate::assets::midi::parse_instrument_table(&t))
        .unwrap_or_default()
}
