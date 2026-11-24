//! Sound manager (ID_SD equivalent).
//!
//! The original timer interrupt runs at 560 Hz while music plays: IMF music
//! commands are processed every tick, sound effects (PC speaker and AdLib)
//! every 4th tick (140 Hz). We emulate exactly that inside an audio source:
//! the OPL2 is emulated by the `oplon` crate, the PC speaker is a square
//! wave generator.
//!
//! Every sound effect and song can be replaced by a modern file (wav, ogg,
//! flac, mp3; songs may also be .mid when a SoundFont is given). See
//! `AudioPack`. In the MIDI music mode the songs are converted to General MIDI
//! (`assets::midi`) and played through the SoundFont.

use crate::assets::audio::{AdlibSound, Audio, PcSound, Song};
use oplon::Opl2;
use rodio::{OutputStreamBuilder, Sink, Source};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::Duration;

pub const SAMPLE_RATE: u32 = 44100;
const PIT_HZ: f64 = 1_193_181.0;

#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum SdMode {
    Off,
    PC,
    #[default]
    AdLib,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum SmMode {
    Off,
    #[default]
    AdLib,
    /// the songs converted to General MIDI, played through a SoundFont
    /// (not in the original)
    Midi,
}

struct Synth {
    /// the music; sound effects play on their own chip (`opl_fx`, channel
    /// 0 like the original) so both can have their own volume
    opl: Opl2,
    opl_fx: Opl2,
    music_vol: f32,
    sfx_vol: f32,
    pcs: Vec<PcSound>,
    adl: Vec<AdlibSound>,
    // sound effects
    sound_mode: SdMode,
    quiet_adlib: bool,
    pc_sound: Option<(usize, usize)>, // (sound, pos)
    pc_last: u8,
    pc_freq: f64,
    pc_phase: f64,
    al_sound: Option<(usize, usize)>,
    al_block: u8,
    sound_number: i32,
    sound_priority: u16,
    // music
    music: Option<Song>,
    music_pos: usize,
    music_time: u32,
    al_time_count: u32,
    music_on: bool,
    // timing
    tick: u32,
    sample_acc: f64,
    samples_per_tick: f64,
    dc_x: f32,
    dc_y: f32,
}

impl Synth {
    fn new(audio: &Audio) -> Synth {
        let new_opl = || {
            let mut opl = Opl2::new(SAMPLE_RATE);
            // waveform select enable
            opl.write_reg(0x01, 0x20);
            opl.write_reg(0x08, 0x00);
            opl
        };
        Synth {
            opl: new_opl(),
            opl_fx: new_opl(),
            music_vol: 1.0,
            sfx_vol: 1.0,
            pcs: audio.pc.clone(),
            adl: audio.adlib.clone(),
            sound_mode: SdMode::AdLib,
            quiet_adlib: false,
            pc_sound: None,
            pc_last: 0,
            pc_freq: 0.0,
            pc_phase: 0.0,
            al_sound: None,
            al_block: 0,
            sound_number: 0,
            sound_priority: 0,
            music: None,
            music_pos: 0,
            music_time: 0,
            al_time_count: 0,
            music_on: false,
            tick: 0,
            sample_acc: 0.0,
            samples_per_tick: SAMPLE_RATE as f64 / 560.0,
            dc_x: 0.0,
            dc_y: 0.0,
        }
    }

    fn sound_finished(&mut self) {
        self.sound_number = 0;
        self.sound_priority = 0;
    }

    fn pc_stop(&mut self) {
        self.pc_sound = None;
        self.pc_last = 0;
        self.pc_freq = 0.0;
    }

    fn al_stop(&mut self) {
        self.al_sound = None;
        self.opl_fx.write_reg(0xb0, 0);
    }

    fn stop_sound(&mut self) {
        self.pc_stop();
        self.al_stop();
        self.sound_finished();
    }

    fn al_set_fx_inst(&mut self, s: &AdlibSound) {
        let m = 0u8;
        let c = 3u8;
        let i = s.inst;
        self.opl_fx.write_reg(m + 0x20, i.m_char);
        self.opl_fx.write_reg(m + 0x40, i.m_scale);
        self.opl_fx.write_reg(m + 0x60, i.m_attack);
        self.opl_fx.write_reg(m + 0x80, i.m_sus);
        self.opl_fx.write_reg(m + 0xe0, i.m_wave);
        self.opl_fx.write_reg(c + 0x20, i.c_char);
        let mut cs = i.c_scale;
        if self.quiet_adlib {
            let a = 0x3f - (cs & 0x3f);
            cs = 0x3f - ((a >> 1) + (a >> 2));
        }
        self.opl_fx.write_reg(c + 0x40, cs);
        self.opl_fx.write_reg(c + 0x60, i.c_attack);
        self.opl_fx.write_reg(c + 0x80, i.c_sus);
        self.opl_fx.write_reg(c + 0xe0, i.c_wave);
    }

    fn play(&mut self, n: usize) -> bool {
        let prio = match self.sound_mode {
            SdMode::Off => return false,
            SdMode::PC => self.pcs.get(n).map(|s| s.priority),
            SdMode::AdLib => self.adl.get(n).map(|s| s.priority),
        };
        let Some(prio) = prio else { return false };
        if prio < self.sound_priority {
            return false;
        }
        match self.sound_mode {
            SdMode::PC => {
                self.pc_stop();
                if !self.pcs[n].data.is_empty() {
                    self.pc_sound = Some((n, 0));
                }
            }
            SdMode::AdLib => {
                self.al_stop();
                let s = self.adl[n].clone();
                self.al_set_fx_inst(&s);
                self.al_block = ((s.block & 7) << 2) | 0x20;
                if !s.data.is_empty() {
                    self.al_sound = Some((n, 0));
                }
            }
            SdMode::Off => {}
        }
        self.sound_number = n as i32;
        self.sound_priority = prio;
        true
    }

    fn pc_service(&mut self) {
        if let Some((n, pos)) = self.pc_sound {
            let data = &self.pcs[n].data;
            let s = data[pos];
            if s != self.pc_last {
                self.pc_last = s;
                self.pc_freq = if s != 0 { PIT_HZ / (s as f64 * 60.0) } else { 0.0 };
            }
            if pos + 1 >= data.len() {
                self.pc_stop();
                self.sound_finished();
            } else {
                self.pc_sound = Some((n, pos + 1));
            }
        }
    }

    fn al_service(&mut self) {
        if let Some((n, pos)) = self.al_sound {
            let len = self.adl[n].data.len();
            let s = self.adl[n].data[pos];
            if s == 0 {
                self.opl_fx.write_reg(0xb0, 0);
            } else {
                self.opl_fx.write_reg(0xa0, s);
                self.opl_fx.write_reg(0xb0, self.al_block);
            }
            if pos + 1 >= len {
                self.al_sound = None;
                self.opl_fx.write_reg(0xb0, 0);
                self.sound_finished();
            } else {
                self.al_sound = Some((n, pos + 1));
            }
        }
    }

    fn music_service(&mut self) {
        if !self.music_on {
            return;
        }
        let Some(song) = &self.music else { return };
        let mut writes: Vec<(u8, u8)> = Vec::new();
        while self.music_pos < song.commands.len() && self.music_time <= self.al_time_count {
            let (reg, val, delay) = song.commands[self.music_pos];
            self.music_pos += 1;
            self.music_time = self.al_time_count + delay as u32;
            writes.push((reg, val));
        }
        self.al_time_count += 1;
        if self.music_pos >= song.commands.len() {
            self.music_pos = 0;
            self.music_time = 0;
            self.al_time_count = 0;
        }
        for (r, v) in writes {
            self.opl.write_reg(r, v);
        }
    }

    /// One 560 Hz interrupt.
    fn t0_tick(&mut self) {
        self.music_service();
        self.tick = self.tick.wrapping_add(1);
        if self.tick & 3 == 0 {
            match self.sound_mode {
                SdMode::PC => self.pc_service(),
                SdMode::AdLib => self.al_service(),
                SdMode::Off => {}
            }
        }
    }

    fn render(&mut self, out: &mut [f32]) {
        for o in out.iter_mut() {
            self.sample_acc += 1.0;
            while self.sample_acc >= self.samples_per_tick {
                self.sample_acc -= self.samples_per_tick;
                self.t0_tick();
            }
            let (l, r) = self.opl.render_frame();
            let (fl, fr) = self.opl_fx.render_frame();
            // OPL2 is mono (l == r); one operator peaks at about +-4084
            let mut v = (l + r) as f32 / 16384.0 * self.music_vol + (fl + fr) as f32 / 16384.0 * self.sfx_vol;
            if self.pc_freq > 0.0 {
                self.pc_phase += self.pc_freq / SAMPLE_RATE as f64;
                if self.pc_phase >= 1.0 {
                    self.pc_phase -= self.pc_phase.floor();
                }
                v += if self.pc_phase < 0.5 { 0.15 } else { -0.15 } * self.sfx_vol;
            }
            // DC blocker
            let y = v - self.dc_x + 0.995 * self.dc_y;
            self.dc_x = v;
            self.dc_y = y;
            *o = y.clamp(-1.0, 1.0);
        }
    }
}

struct SynthSource {
    synth: Arc<Mutex<Synth>>,
    buf: Vec<f32>,
    pos: usize,
}

impl Iterator for SynthSource {
    type Item = f32;
    fn next(&mut self) -> Option<f32> {
        if self.pos >= self.buf.len() {
            self.buf.resize(256, 0.0);
            if let Ok(mut s) = self.synth.lock() {
                s.render(&mut self.buf);
            }
            self.pos = 0;
        }
        let v = self.buf[self.pos];
        self.pos += 1;
        Some(v)
    }
}

impl Source for SynthSource {
    fn current_span_len(&self) -> Option<usize> {
        None
    }
    fn channels(&self) -> u16 {
        1
    }
    fn sample_rate(&self) -> u32 {
        SAMPLE_RATE
    }
    fn total_duration(&self) -> Option<Duration> {
        None
    }
}

/// Replacement audio files. Layout of the pack directory:
/// `sfx/NN.ext` for sound effect NN and `music/NN.ext` for song NN; files
/// in `sfx/kN/` and `music/kN/` (N = 4, 5, 6) are only used by that episode
/// and come first (the numbers mean different sounds per episode).
pub struct AudioPack {
    pub dir: Option<PathBuf>,
    pub soundfont: Option<PathBuf>,
    /// "k4", "k5" or "k6"
    pub episode: Option<String>,
}

impl AudioPack {
    fn find(&self, sub: &str, n: usize, exts: &[&str]) -> Option<PathBuf> {
        let base = self.dir.as_ref()?.join(sub);
        let mut dirs = Vec::new();
        if let Some(ep) = &self.episode {
            dirs.push(base.join(ep));
        }
        dirs.push(base);
        for d in dirs {
            for e in exts {
                for name in [format!("{:02}.{}", n, e), format!("{}.{}", n, e)] {
                    let p = d.join(name);
                    if p.exists() {
                        return Some(p);
                    }
                }
            }
        }
        None
    }
}

/// MIDI playback through rustysynth.
struct MidiSource {
    seq: rustysynth::MidiFileSequencer,
    l: Vec<f32>,
    r: Vec<f32>,
    pos: usize,
    right: bool,
}

impl Iterator for MidiSource {
    type Item = f32;
    fn next(&mut self) -> Option<f32> {
        if self.pos >= self.l.len() {
            self.l.resize(512, 0.0);
            self.r.resize(512, 0.0);
            self.seq.render(&mut self.l, &mut self.r);
            self.pos = 0;
        }
        let v = if self.right { self.r[self.pos] } else { self.l[self.pos] };
        if self.right {
            self.pos += 1;
        }
        self.right = !self.right;
        Some(v)
    }
}

impl Source for MidiSource {
    fn current_span_len(&self) -> Option<usize> {
        None
    }
    fn channels(&self) -> u16 {
        2
    }
    fn sample_rate(&self) -> u32 {
        SAMPLE_RATE
    }
    fn total_duration(&self) -> Option<Duration> {
        None
    }
}

/// how long the audio output may take to start before the game goes on
/// without sound
const AUDIO_OPEN_TIMEOUT: Duration = Duration::from_secs(3);

/// The audio output, opened and kept on a thread of its own: a device that
/// never finishes starting (CoreAudio waiting for an HDMI screen's audio,
/// e.g.) only costs the sound, not the game. Dropping it closes the output.
struct AudioOutput {
    mixer: rodio::mixer::Mixer,
    _stop: std::sync::mpsc::Sender<()>,
}

impl AudioOutput {
    fn open() -> Result<AudioOutput, String> {
        let (tx, rx) = std::sync::mpsc::channel();
        let (stop_tx, stop_rx) = std::sync::mpsc::channel::<()>();
        std::thread::Builder::new()
            .name("audio output".into())
            .spawn(move || match OutputStreamBuilder::open_default_stream() {
                Ok(mut stream) => {
                    stream.log_on_drop(false);
                    if tx.send(Ok(stream.mixer().clone())).is_ok() {
                        // (keeps the stream until the sender is dropped)
                        let _ = stop_rx.recv();
                    }
                }
                Err(e) => {
                    let _ = tx.send(Err(e.to_string()));
                }
            })
            .map_err(|e| e.to_string())?;
        match rx.recv_timeout(AUDIO_OPEN_TIMEOUT) {
            Ok(Ok(mixer)) => Ok(AudioOutput { mixer, _stop: stop_tx }),
            Ok(Err(e)) => Err(e),
            Err(_) => Err(format!(
                "the audio output did not start within {} seconds (check the sound output device)",
                AUDIO_OPEN_TIMEOUT.as_secs()
            )),
        }
    }
}

pub struct SoundManager {
    synth: Option<Arc<Mutex<Synth>>>,
    output: Option<AudioOutput>,
    sfx_sink: Option<Sink>,
    music_sink: Option<Sink>,
    pack: AudioPack,
    songs: Vec<Song>,
    pcs_prio: Vec<u16>,
    pub sound_mode: SdMode,
    pub music_mode: SmMode,
    pub quiet_adlib: bool,
    ext_sound: i32,
    ext_priority: u16,
    current_song: Option<usize>,
    /// loaded from `pack.soundfont` when first needed
    soundfont: Option<Arc<rustysynth::SoundFont>>,
    soundfont_tried: bool,
    /// MIDI music can be chosen (not with --classic, needs a SoundFont)
    midi_allowed: bool,
    /// instrument overrides for the converted songs, and the songs
    midi_table: crate::assets::midi::InstrumentTable,
    midi_cache: std::collections::HashMap<usize, Arc<rustysynth::MidiFile>>,
    /// volumes in percent (not in the original)
    pub sfx_volume: u8,
    pub music_volume: u8,
    /// extra gain of what the music sink plays (MIDI is quieter)
    music_sink_gain: f32,
    /// MIDI music was chosen but is not available (--classic): the
    /// config keeps the choice
    pub midi_requested: bool,
}

/// Loudness of a volume setting in percent (squared: closer to how loud it
/// sounds).
fn vol_gain(v: u8) -> f32 {
    let f = v.min(100) as f32 / 100.0;
    f * f
}

/// Volume of SoundFont music relative to the emulated AdLib.
const MIDI_GAIN: f32 = 3.5;

fn midi_source(sf: &Arc<rustysynth::SoundFont>, midi: &Arc<rustysynth::MidiFile>) -> Option<MidiSource> {
    let settings = rustysynth::SynthesizerSettings::new(SAMPLE_RATE as i32);
    let syn = rustysynth::Synthesizer::new(sf, &settings).ok()?;
    let mut seq = rustysynth::MidiFileSequencer::new(syn);
    seq.play(midi, true);
    Some(MidiSource { seq, l: Vec::new(), r: Vec::new(), pos: 0, right: false })
}

fn load_soundfont(path: &Path) -> Option<Arc<rustysynth::SoundFont>> {
    let mut f = match std::fs::File::open(path) {
        Ok(f) => f,
        Err(e) => {
            eprintln!("soundfont {}: {}", path.display(), e);
            return None;
        }
    };
    match rustysynth::SoundFont::new(&mut f) {
        Ok(s) => Some(Arc::new(s)),
        Err(e) => {
            eprintln!("soundfont {}: {:?}", path.display(), e);
            None
        }
    }
}

/// A SoundFont for the MIDI music when none is given: the first .sf2 in the
/// data directory or its parent (data/ of the release archives), next to
/// the executable (also in data/ there), in a macOS app bundle's Resources
/// (also of the other Keen apps beside it), in the current directory, or a
/// system-wide General MIDI SoundFont.
pub fn find_soundfont(data_dir: &Path) -> Option<PathBuf> {
    let mut dirs = vec![data_dir.to_path_buf()];
    if let Some(p) = data_dir.parent() {
        dirs.push(p.to_path_buf());
    }
    if let Some(d) = std::env::current_exe().ok().and_then(|e| e.parent().map(|p| p.to_path_buf())) {
        dirs.push(d.join("data"));
        dirs.push(d.join("../Resources"));
        // beside the app bundle: data/ and the other apps (the release
        // ships the SoundFont once, inside Keen 4.app)
        let beside = d.join("../../..");
        dirs.push(beside.join("data"));
        if let Ok(rd) = std::fs::read_dir(&beside) {
            let mut apps: Vec<PathBuf> = rd.flatten().map(|e| e.path()).filter(|p| p.extension().map(|e| e == "app").unwrap_or(false)).collect();
            apps.sort();
            dirs.extend(apps.into_iter().map(|a| a.join("Contents/Resources")));
        }
        dirs.insert(1, d);
    }
    dirs.push(PathBuf::from("."));
    for d in dirs {
        if let Ok(rd) = std::fs::read_dir(&d) {
            let mut v: Vec<PathBuf> = rd
                .flatten()
                .map(|e| e.path())
                .filter(|p| p.extension().map(|e| e.eq_ignore_ascii_case("sf2")).unwrap_or(false))
                .collect();
            v.sort();
            if let Some(p) = v.into_iter().next() {
                return Some(p);
            }
        }
    }
    [
        "/usr/share/sounds/sf2/FluidR3_GM.sf2",
        "/usr/share/soundfonts/FluidR3_GM.sf2",
        "/usr/share/soundfonts/default.sf2",
        "/usr/local/share/soundfonts/default.sf2",
        "/opt/homebrew/share/soundfonts/default.sf2",
    ]
    .iter()
    .map(PathBuf::from)
    .find(|p| p.exists())
}

impl SoundManager {
    pub fn new(audio: &Audio, enable: bool, pack_dir: Option<&Path>, soundfont: Option<&Path>) -> SoundManager {
        let pack = AudioPack { dir: pack_dir.map(|p| p.to_path_buf()), soundfont: soundfont.map(|p| p.to_path_buf()), episode: None };
        let mut sm = SoundManager {
            synth: None,
            output: None,
            sfx_sink: None,
            music_sink: None,
            pack,
            songs: audio.songs.clone(),
            pcs_prio: audio.adlib.iter().map(|s| s.priority).collect(),
            sound_mode: SdMode::AdLib,
            music_mode: SmMode::AdLib,
            quiet_adlib: false,
            ext_sound: 0,
            ext_priority: 0,
            current_song: None,
            soundfont: None,
            soundfont_tried: false,
            midi_allowed: false,
            midi_table: Default::default(),
            midi_cache: std::collections::HashMap::new(),
            sfx_volume: 100,
            music_volume: 100,
            music_sink_gain: 1.0,
            midi_requested: false,
        };
        let synth = Arc::new(Mutex::new(Synth::new(audio)));
        if enable {
            match AudioOutput::open() {
                Ok(out) => {
                    out.mixer.add(SynthSource { synth: synth.clone(), buf: Vec::new(), pos: 0 });
                    sm.sfx_sink = Some(Sink::connect_new(&out.mixer));
                    sm.music_sink = Some(Sink::connect_new(&out.mixer));
                    sm.output = Some(out);
                }
                Err(e) => eprintln!("audio disabled: {}", e),
            }
        }
        sm.synth = Some(synth);
        sm
    }

    /// Selects the episode subfolders (`sfx/k4/` …) of the audio pack.
    pub fn set_episode(&mut self, episode: u8) {
        self.pack.episode = Some(format!("k{}", episode));
    }

    /// Allows the MIDI music mode (`table`: instrument overrides); it
    /// needs a SoundFont (`--soundfont` or one found by `find_soundfont`).
    pub fn allow_midi(&mut self, table: crate::assets::midi::InstrumentTable) {
        self.midi_table = table;
        self.midi_allowed = true;
    }

    /// The MIDI music mode can be chosen.
    pub fn midi_available(&self) -> bool {
        self.midi_allowed && self.pack.soundfont.is_some()
    }

    /// The SoundFont, loaded on first use.
    fn soundfont(&mut self) -> Option<Arc<rustysynth::SoundFont>> {
        if self.soundfont.is_none() && !self.soundfont_tried {
            self.soundfont_tried = true;
            self.soundfont = self.pack.soundfont.as_deref().and_then(load_soundfont);
        }
        self.soundfont.clone()
    }

    /// Sound effect volume in percent.
    pub fn set_sfx_volume(&mut self, v: u8) {
        self.sfx_volume = v.min(100);
        let g = vol_gain(self.sfx_volume);
        self.with(|s| s.sfx_vol = g);
        if let Some(s) = &self.sfx_sink {
            s.set_volume(g);
        }
    }

    /// Music volume in percent.
    pub fn set_music_volume(&mut self, v: u8) {
        self.music_volume = v.min(100);
        let g = vol_gain(self.music_volume);
        self.with(|s| s.music_vol = g);
        if let Some(s) = &self.music_sink {
            s.set_volume(g * self.music_sink_gain);
        }
    }

    /// The converted song `n` (cached).
    fn midi_song(&mut self, n: usize) -> Option<Arc<rustysynth::MidiFile>> {
        let table = &self.midi_table;
        if let Some(m) = self.midi_cache.get(&n) {
            return Some(m.clone());
        }
        let song = self.songs.get(n).filter(|s| !s.commands.is_empty())?;
        let bytes = crate::assets::midi::song_to_midi(song, n, table);
        let m = Arc::new(rustysynth::MidiFile::new(&mut std::io::Cursor::new(bytes)).ok()?);
        self.midi_cache.insert(n, m.clone());
        Some(m)
    }

    fn with<R>(&self, f: impl FnOnce(&mut Synth) -> R) -> Option<R> {
        let s = self.synth.as_ref()?;
        let mut g = s.lock().ok()?;
        Some(f(&mut g))
    }

    pub fn set_sound_mode(&mut self, m: SdMode) {
        self.sound_mode = m;
        self.stop_sound();
        self.with(|s| s.sound_mode = m);
    }

    pub fn set_quiet(&mut self, q: bool) {
        self.quiet_adlib = q;
        self.with(|s| s.quiet_adlib = q);
    }

    pub fn set_music_mode(&mut self, m: SmMode) {
        self.midi_requested = m == SmMode::Midi && !self.midi_available();
        let m = if self.midi_requested { SmMode::AdLib } else { m };
        self.music_mode = m;
        if m == SmMode::Off {
            self.music_off();
        }
    }

    /// SD_PlaySound
    pub fn play_sound(&mut self, n: usize) {
        if self.sound_mode == SdMode::Off {
            return;
        }
        let ext = self.pack.find("sfx", n, &["ogg", "wav", "flac", "mp3"]);
        if let (Some(p), Some(sink)) = (ext, &self.sfx_sink) {
            let prio = self.pcs_prio.get(n).copied().unwrap_or(0);
            if !sink.empty() && prio < self.ext_priority {
                return;
            }
            if let Ok(f) = std::fs::File::open(&p) {
                if let Ok(dec) = rodio::Decoder::try_from(f) {
                    sink.clear();
                    sink.append(dec);
                    sink.play();
                    self.ext_sound = n as i32;
                    self.ext_priority = prio;
                    return;
                }
            }
        }
        self.with(|s| s.play(n));
    }

    pub fn stop_sound(&mut self) {
        if let Some(s) = &self.sfx_sink {
            s.clear();
        }
        self.ext_sound = 0;
        self.ext_priority = 0;
        self.with(|s| s.stop_sound());
    }

    /// SD_SoundPlaying: number of the current sound or 0.
    pub fn sound_playing(&self) -> i32 {
        // without an output stream nothing advances the synth
        if self.output.is_none() {
            return 0;
        }
        if let Some(s) = &self.sfx_sink {
            if !s.empty() {
                return self.ext_sound;
            }
        }
        self.with(|s| s.sound_number).unwrap_or(0)
    }

    pub fn wait_sound_done(&self, sys: &mut super::sys::Sys) {
        let start = sys.time_count();
        while self.sound_playing() != 0 && !sys.quit {
            sys.wait_vbl(1);
            if sys.time_count() - start > 70 * 10 {
                break;
            }
        }
    }

    /// StartMusic(song)
    pub fn start_music(&mut self, n: usize) {
        self.music_off();
        if self.music_mode == SmMode::Off {
            return;
        }
        self.current_song = Some(n);
        if let Some(p) = self.pack.find("music", n, &["ogg", "flac", "mp3", "wav"]) {
            if let (Ok(f), Some(sink)) = (std::fs::File::open(&p), &self.music_sink) {
                if let Ok(dec) = rodio::Decoder::new_looped(f) {
                    self.music_sink_gain = 1.0;
                    sink.set_volume(vol_gain(self.music_volume));
                    sink.append(dec);
                    sink.play();
                    return;
                }
            }
        }
        if self.music_sink.is_some() {
            // a .mid of the audio pack (with a SoundFont), else in MIDI mode
            // the converted song
            let mut midi = None;
            if self.pack.soundfont.is_some() {
                if let Some(p) = self.pack.find("music", n, &["mid"]) {
                    if let Ok(mut f) = std::fs::File::open(&p) {
                        midi = rustysynth::MidiFile::new(&mut f).ok().map(Arc::new);
                    }
                }
            }
            if midi.is_none() && self.music_mode == SmMode::Midi {
                midi = self.midi_song(n);
            }
            if let (Some(m), Some(sf)) = (midi, self.soundfont()) {
                if let (Some(src), Some(sink)) = (midi_source(&sf, &m), &self.music_sink) {
                    self.music_sink_gain = MIDI_GAIN;
                    sink.set_volume(vol_gain(self.music_volume) * MIDI_GAIN);
                    sink.append(src);
                    sink.play();
                    return;
                }
            }
        }
        let song = self.songs.get(n).cloned();
        self.with(|s| {
            s.music = song;
            s.music_pos = 0;
            s.music_time = 0;
            s.al_time_count = 0;
            s.music_on = true;
        });
    }

    pub fn music_off(&mut self) {
        if let Some(s) = &self.music_sink {
            s.clear();
        }
        // current_song is kept: music_on (after PAUSED) restarts it
        self.with(|s| {
            s.music_on = false;
            s.music = None;
            // SDL_ALStopMusic: key off all music voices
            for i in 1..9u8 {
                s.opl.write_reg(0xb0 + i, 0);
            }
            s.opl.write_reg(0xbd, 0);
        });
    }

    pub fn music_on(&mut self) {
        if let Some(n) = self.current_song {
            self.start_music(n);
        }
    }
}

/// Renders a song (`song`) or a sound effect (`sfx`) offline to a WAV file
/// (debugging / comparing the emulation).
pub fn render_to_wav(audio: &Audio, song: Option<usize>, sfx: Option<(usize, SdMode)>, seconds: f32, path: &Path) -> std::io::Result<()> {
    let mut s = Synth::new(audio);
    if let Some(n) = song {
        s.music = audio.songs.get(n).cloned();
        s.music_on = true;
    }
    if let Some((n, mode)) = sfx {
        s.sound_mode = mode;
        s.play(n);
    }
    let total = (SAMPLE_RATE as f32 * seconds) as usize;
    let mut buf = vec![0f32; total];
    s.render(&mut buf);
    write_wav(&buf, path)
}

/// Renders song `n` converted to MIDI through `soundfont` (mono mix) to a
/// WAV file, like `render_to_wav` (comparing levels and instruments).
pub fn render_midi_to_wav(audio: &Audio, n: usize, soundfont: &Path, table: &crate::assets::midi::InstrumentTable, seconds: f32, path: &Path) -> std::io::Result<()> {
    let err = |m: &str| std::io::Error::other(m.to_string());
    let sf = load_soundfont(soundfont).ok_or_else(|| err("no soundfont"))?;
    let song = audio.songs.get(n).ok_or_else(|| err("no such song"))?;
    let bytes = crate::assets::midi::song_to_midi(song, n, table);
    let midi = Arc::new(rustysynth::MidiFile::new(&mut std::io::Cursor::new(bytes)).map_err(|e| err(&format!("{:?}", e)))?);
    let mut src = midi_source(&sf, &midi).ok_or_else(|| err("synth"))?;
    let total = (SAMPLE_RATE as f32 * seconds) as usize;
    let buf: Vec<f32> = (0..total).map(|_| (src.next().unwrap_or(0.0) + src.next().unwrap_or(0.0)) * 0.5 * MIDI_GAIN).collect();
    write_wav(&buf, path)
}

/// `--export-sfx DIR`: every sound effect as DIR/NN-adlib.wav and
/// DIR/NN-pc.wav, rendered like in the game and cut after the sound (and
/// the AdLib release) has died away, plus DIR/sfx.json with the raw data
/// (AdLib: block, instrument registers and one F-number byte per 140 Hz
/// tick, 0 = key off; PC speaker: one PIT divisor/60 byte per tick).
pub fn export_sfx(audio: &Audio, dir: &Path) -> std::io::Result<usize> {
    std::fs::create_dir_all(dir)?;
    let mut json = String::from("[\n");
    for n in 0..audio.adlib.len().max(audio.pc.len()) {
        let list = |d: &[u8]| d.iter().map(|b| b.to_string()).collect::<Vec<_>>().join(",");
        let (al, pc) = (audio.adlib.get(n), audio.pc.get(n));
        let inst = al.map(|s| {
            let i = &s.inst;
            [i.m_char, i.c_char, i.m_scale, i.c_scale, i.m_attack, i.c_attack, i.m_sus, i.c_sus, i.m_wave, i.c_wave, i.n_conn]
        });
        json.push_str(&format!(
            "{{\"n\":{},\"priority\":{},\"block\":{},\"inst\":[{}],\"adlib\":[{}],\"pc\":[{}]}}{}\n",
            n,
            al.map(|s| s.priority).unwrap_or(0),
            al.map(|s| s.block).unwrap_or(0),
            inst.map(|i| list(&i)).unwrap_or_default(),
            al.map(|s| list(&s.data)).unwrap_or_default(),
            pc.map(|s| list(&s.data)).unwrap_or_default(),
            if n + 1 < audio.adlib.len().max(audio.pc.len()) { "," } else { "" }
        ));
    }
    json.push_str("]\n");
    std::fs::write(dir.join("sfx.json"), json)?;
    let mut count = 0;
    for n in 0..audio.adlib.len().max(audio.pc.len()) {
        for (mode, tag, len) in [
            (SdMode::AdLib, "adlib", audio.adlib.get(n).map(|s| s.data.len())),
            (SdMode::PC, "pc", audio.pc.get(n).map(|s| s.data.len())),
        ] {
            let Some(len) = len.filter(|&l| l > 0) else { continue };
            let mut s = Synth::new(audio);
            s.sound_mode = mode;
            if !s.play(n) {
                continue;
            }
            // the sound itself (140 Hz ticks) plus up to 2 s of release
            let total = ((len as f32 / 140.0 + 2.0) * SAMPLE_RATE as f32) as usize;
            let mut buf = vec![0f32; total];
            s.render(&mut buf);
            let end = buf.iter().rposition(|v| v.abs() > 0.002).map(|i| i + 1).unwrap_or(0);
            let keep = (end + SAMPLE_RATE as usize / 50).min(buf.len());
            write_wav(&buf[..keep], &dir.join(format!("{:02}-{}.wav", n, tag)))?;
            count += 1;
        }
    }
    Ok(count)
}

fn write_wav(buf: &[f32], path: &Path) -> std::io::Result<()> {
    let total = buf.len();
    let mut data = Vec::with_capacity(total * 2);
    for v in buf {
        data.extend_from_slice(&((v.clamp(-1.0, 1.0) * 32767.0) as i16).to_le_bytes());
    }
    let mut out = Vec::new();
    out.extend_from_slice(b"RIFF");
    out.extend_from_slice(&(36 + data.len() as u32).to_le_bytes());
    out.extend_from_slice(b"WAVEfmt ");
    out.extend_from_slice(&16u32.to_le_bytes());
    out.extend_from_slice(&1u16.to_le_bytes());
    out.extend_from_slice(&1u16.to_le_bytes());
    out.extend_from_slice(&SAMPLE_RATE.to_le_bytes());
    out.extend_from_slice(&(SAMPLE_RATE * 2).to_le_bytes());
    out.extend_from_slice(&2u16.to_le_bytes());
    out.extend_from_slice(&16u16.to_le_bytes());
    out.extend_from_slice(b"data");
    out.extend_from_slice(&(data.len() as u32).to_le_bytes());
    out.extend_from_slice(&data);
    std::fs::write(path, out)
}
