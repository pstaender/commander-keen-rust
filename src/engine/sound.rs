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
//! `AudioPack`.

use crate::assets::audio::{AdlibSound, Audio, PcSound, Song};
use oplon::Opl2;
use rodio::{OutputStream, OutputStreamBuilder, Sink, Source};
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
}

struct Synth {
    opl: Opl2,
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
        let mut opl = Opl2::new(SAMPLE_RATE);
        // waveform select enable
        opl.write_reg(0x01, 0x20);
        opl.write_reg(0x08, 0x00);
        Synth {
            opl,
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
        self.opl.write_reg(0xb0, 0);
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
        self.opl.write_reg(m + 0x20, i.m_char);
        self.opl.write_reg(m + 0x40, i.m_scale);
        self.opl.write_reg(m + 0x60, i.m_attack);
        self.opl.write_reg(m + 0x80, i.m_sus);
        self.opl.write_reg(m + 0xe0, i.m_wave);
        self.opl.write_reg(c + 0x20, i.c_char);
        let mut cs = i.c_scale;
        if self.quiet_adlib {
            let a = 0x3f - (cs & 0x3f);
            cs = 0x3f - ((a >> 1) + (a >> 2));
        }
        self.opl.write_reg(c + 0x40, cs);
        self.opl.write_reg(c + 0x60, i.c_attack);
        self.opl.write_reg(c + 0x80, i.c_sus);
        self.opl.write_reg(c + 0xe0, i.c_wave);
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
                self.opl.write_reg(0xb0, 0);
            } else {
                self.opl.write_reg(0xa0, s);
                self.opl.write_reg(0xb0, self.al_block);
            }
            if pos + 1 >= len {
                self.al_sound = None;
                self.opl.write_reg(0xb0, 0);
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
            // OPL2 is mono (l == r); one operator peaks at about +-4084
            let mut v = (l + r) as f32 / 16384.0;
            if self.pc_freq > 0.0 {
                self.pc_phase += self.pc_freq / SAMPLE_RATE as f64;
                if self.pc_phase >= 1.0 {
                    self.pc_phase -= self.pc_phase.floor();
                }
                v += if self.pc_phase < 0.5 { 0.15 } else { -0.15 };
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
/// `sfx/NN.ext` for sound effect NN (0..59) and `music/NN.ext` for song NN.
pub struct AudioPack {
    pub dir: Option<PathBuf>,
    pub soundfont: Option<PathBuf>,
}

impl AudioPack {
    fn find(&self, sub: &str, n: usize, exts: &[&str]) -> Option<PathBuf> {
        let d = self.dir.as_ref()?.join(sub);
        for e in exts {
            for name in [format!("{:02}.{}", n, e), format!("{}.{}", n, e)] {
                let p = d.join(name);
                if p.exists() {
                    return Some(p);
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

pub struct SoundManager {
    synth: Option<Arc<Mutex<Synth>>>,
    _stream: Option<OutputStream>,
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
    soundfont: Option<Arc<rustysynth::SoundFont>>,
}

impl SoundManager {
    pub fn new(audio: &Audio, enable: bool, pack_dir: Option<&Path>, soundfont: Option<&Path>) -> SoundManager {
        let pack = AudioPack { dir: pack_dir.map(|p| p.to_path_buf()), soundfont: soundfont.map(|p| p.to_path_buf()) };
        let mut sm = SoundManager {
            synth: None,
            _stream: None,
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
        };
        let synth = Arc::new(Mutex::new(Synth::new(audio)));
        if enable {
            match OutputStreamBuilder::open_default_stream() {
                Ok(mut stream) => {
                    stream.log_on_drop(false);
                    stream.mixer().add(SynthSource { synth: synth.clone(), buf: Vec::new(), pos: 0 });
                    sm.sfx_sink = Some(Sink::connect_new(stream.mixer()));
                    sm.music_sink = Some(Sink::connect_new(stream.mixer()));
                    sm._stream = Some(stream);
                }
                Err(e) => eprintln!("audio disabled: {}", e),
            }
        }
        if let Some(sf) = &sm.pack.soundfont {
            if let Ok(mut f) = std::fs::File::open(sf) {
                match rustysynth::SoundFont::new(&mut f) {
                    Ok(s) => sm.soundfont = Some(Arc::new(s)),
                    Err(e) => eprintln!("soundfont {}: {:?}", sf.display(), e),
                }
            }
        }
        sm.synth = Some(synth);
        sm
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
        if self._stream.is_none() {
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
        if let Some(sink) = &self.music_sink {
            if let Some(p) = self.pack.find("music", n, &["ogg", "flac", "mp3", "wav"]) {
                if let Ok(f) = std::fs::File::open(&p) {
                    if let Ok(dec) = rodio::Decoder::new_looped(f) {
                        sink.append(dec);
                        sink.play();
                        return;
                    }
                }
            }
            if let (Some(p), Some(sf)) = (self.pack.find("music", n, &["mid"]), &self.soundfont) {
                if let Ok(mut f) = std::fs::File::open(&p) {
                    if let Ok(midi) = rustysynth::MidiFile::new(&mut f) {
                        let settings = rustysynth::SynthesizerSettings::new(SAMPLE_RATE as i32);
                        if let Ok(syn) = rustysynth::Synthesizer::new(sf, &settings) {
                            let mut seq = rustysynth::MidiFileSequencer::new(syn);
                            seq.play(&Arc::new(midi), true);
                            sink.append(MidiSource { seq, l: Vec::new(), r: Vec::new(), pos: 0, right: false });
                            sink.play();
                            return;
                        }
                    }
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
        self.current_song = None;
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
    let mut data = Vec::with_capacity(total * 2);
    for v in &buf {
        data.extend_from_slice(&((v * 32767.0) as i16).to_le_bytes());
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
