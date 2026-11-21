//! IMF (AdLib register stream) to General MIDI conversion, used by
//! the MIDI MUSIC mode (played through a SoundFont) and `--export-midi`.
//!
//! The song is replayed against a model of the OPL2 registers: a key-on
//! becomes a note-on with the pitch of the channel's F-number/block, a
//! key-off a note-off, and the instrument loaded into the channel's two
//! operators picks a General MIDI program (`gm_program`). Timing is kept
//! exactly: 560 Hz ticks are written as MIDI ticks (280 per quarter at
//! 120 bpm) and the song ends where the original loops.

use super::audio::Song;
use std::collections::BTreeMap;

/// Ticks per second of the music interrupt.
const IMF_HZ: u32 = 560;
/// MIDI resolution; with the default 120 bpm one MIDI tick is one IMF tick.
const PPQ: u16 = (IMF_HZ / 2) as u16;
const DRUM_CH: u8 = 9;

/// Modulator operator offset of each melodic channel (carrier = +3).
const OP_OFF: [u8; 9] = [0x00, 0x01, 0x02, 0x08, 0x09, 0x0a, 0x10, 0x11, 0x12];

/// The registers that make up an OPL2 instrument: modulator and carrier
/// 0x20 (AM/VIB/EG/KSR/MULT), 0x40 (KSL/TL), 0x60 (AR/DR), 0x80 (SL/RR),
/// 0xe0 (waveform) and the channel's 0xc0 (feedback/connection).
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug)]
pub struct Patch {
    pub m: [u8; 5],
    pub c: [u8; 5],
    pub fb: u8,
}

impl Patch {
    fn from_regs(r: &[u8; 256], ch: usize) -> Patch {
        let o = OP_OFF[ch] as usize;
        let op = |o: usize| [r[0x20 + o], r[0x40 + o] & 0x3f, r[0x60 + o], r[0x80 + o], r[0xe0 + o] & 3];
        // the carrier's total level is the note volume, not part of the sound
        let mut c = op(o + 3);
        c[1] = 0;
        Patch { m: op(o), c, fb: r[0xc0 + ch] & 0x0f }
    }

    /// Hex form used in the instrument table and `instruments.txt`.
    pub fn hex(&self) -> String {
        let mut s = String::new();
        for b in self.m.iter().chain(self.c.iter()).chain(std::iter::once(&self.fb)) {
            s.push_str(&format!("{:02x}", b));
        }
        s
    }
}

/// How an instrument sounds, measured by playing one note of it on the
/// emulated OPL2 (key on for 0.6 s, then released).
#[derive(Clone, Copy, Debug, Default)]
pub struct Sound {
    /// time until the note is within 3 dB of its loudest point (s)
    pub attack: f32,
    /// time from the peak until the note fell by 20 dB while held (s;
    /// 9 = it doesn't)
    pub decay: f32,
    /// level 0.5 s into the held note relative to the peak
    pub sustain: f32,
    /// 1 = clean periodic tone at the played pitch, 0 = noise
    pub tonal: f32,
    /// spectral centroid in harmonics of the played pitch (1 = sine)
    pub centroid: f32,
    /// rough main frequency from the zero crossings (Hz); for noise its
    /// "color"
    pub zcr_hz: f32,
}

const FEAT_RATE: u32 = 22050;

/// Plays `p` at MIDI note `key` and measures it (`Sound`).
pub fn measure(p: &Patch, key: u8) -> Sound {
    let mut opl = oplon::Opl2::new(FEAT_RATE);
    opl.write_reg(0x01, 0x20);
    for (i, r) in [0x20u8, 0x40, 0x60, 0x80, 0xe0].iter().enumerate() {
        opl.write_reg(r + 0x00, p.m[i]);
        opl.write_reg(r + 0x03, p.c[i]);
    }
    opl.write_reg(0xc0, p.fb);
    // the F-number/block closest to the key
    let f = 440.0 * 2f64.powf((key as f64 - 69.0) / 12.0);
    let mut block = 0u8;
    let mut fnum = (f * (1u32 << 20) as f64 / 49716.0).round();
    while fnum > 1023.0 && block < 7 {
        block += 1;
        fnum = (f * (1u32 << (20 - block as u32)) as f64 / 49716.0).round();
    }
    let fnum = fnum.min(1023.0) as u16;
    opl.write_reg(0xa0, fnum as u8);
    opl.write_reg(0xb0, 0x20 | (block << 2) | (fnum >> 8) as u8);
    let held = (FEAT_RATE as f32 * 0.6) as usize;
    let mut x = Vec::with_capacity(held);
    for _ in 0..held {
        let (l, _) = opl.render_frame();
        x.push(l as f32);
    }
    let f0 = fnum as f64 * 49716.0 / (1u32 << (20 - block as u32)) as f64;
    // envelope in 5 ms frames
    let fr = (FEAT_RATE / 200) as usize;
    let env: Vec<f32> = x.chunks(fr).map(|c| (c.iter().map(|v| v * v).sum::<f32>() / c.len() as f32).sqrt()).collect();
    let (pk_i, pk) = env.iter().enumerate().fold((0, 0f32), |a, (i, &v)| if v > a.1 { (i, v) } else { a });
    if pk <= 1.0 {
        return Sound::default();
    }
    // (tremolo moves the loudest point around: use the time to -3 dB)
    let attack = env.iter().position(|&v| v >= pk * 0.7).unwrap_or(pk_i) as f32 * 0.005;
    let decay = env[pk_i..].iter().position(|&v| v < pk * 0.1).map(|i| i as f32 * 0.005).unwrap_or(9.0);
    let sustain = env[env.len() - 10..].iter().sum::<f32>() / 10.0 / pk;
    // analysis window: around the peak, at least 2 periods and 40 ms
    let period = FEAT_RATE as f64 / f0;
    let wlen = ((period * 4.0) as usize).max(FEAT_RATE as usize / 25).min(x.len() / 2);
    let w0 = (pk_i * fr).min(x.len().saturating_sub(wlen + 2 * period.ceil() as usize + 2));
    let w = &x[w0..w0 + wlen];
    // normalized autocorrelation one (or, for a modulator at half the
    // frequency, two) periods later, with linear interpolation
    let corr = |lag: f64| -> f32 {
        let li = lag.floor() as usize;
        let fr = (lag - li as f64) as f32;
        let (mut num, mut e0, mut e1) = (0f32, 0f32, 0f32);
        for i in 0..wlen {
            let j = w0 + i + li;
            if j + 1 >= x.len() {
                break;
            }
            let a = x[w0 + i];
            let b = x[j] * (1.0 - fr) + x[j + 1] * fr;
            num += a * b;
            e0 += a * a;
            e1 += b * b;
        }
        if e0 > 0.0 && e1 > 0.0 { (num / (e0 * e1).sqrt()).max(0.0) } else { 0.0 }
    };
    let tonal = corr(period).max(corr(period * 2.0));
    // harmonic energies (Goertzel) for the centroid
    let mut sum = 0f64;
    let mut wsum = 0f64;
    for h in 1..=24 {
        let fh = f0 * h as f64;
        if fh >= FEAT_RATE as f64 / 2.0 {
            break;
        }
        let k = 2.0 * (2.0 * std::f64::consts::PI * fh / FEAT_RATE as f64).cos();
        let (mut s1, mut s2) = (0f64, 0f64);
        for &v in w {
            let s0 = v as f64 + k * s1 - s2;
            s2 = s1;
            s1 = s0;
        }
        let e = s1 * s1 + s2 * s2 - k * s1 * s2;
        sum += e;
        wsum += e * h as f64;
    }
    let centroid = if sum > 0.0 { (wsum / sum) as f32 } else { 1.0 };
    let cross = w.windows(2).filter(|p| (p[0] < 0.0) != (p[1] < 0.0)).count();
    let zcr_hz = cross as f32 * FEAT_RATE as f32 / 2.0 / w.len() as f32;
    Sound { attack, decay, sustain, tonal, centroid, zcr_hz }
}

/// General MIDI voice of an instrument: a program, or a drum (channel 10)
/// with the given key.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Voice {
    Program(u8),
    Drum(u8),
}

/// Picks a General MIDI voice for an instrument from how it sounds
/// (`Sound`) and the keys it is played with (`low`..`high`).
pub fn gm_guess(snd: &Sound, low: u8, high: u8) -> Voice {
    let mid = (low as i32 + high as i32) / 2;
    let noisy = snd.tonal < 0.5;
    let short = snd.decay < 0.35 && snd.sustain < 0.08;
    // drums: short hits that are noise, or always played at one pitch
    if short && (noisy || high - low <= 2) {
        return Voice::Drum(if noisy {
            match snd.zcr_hz {
                f if f >= 4500.0 => if snd.decay < 0.2 { 42 } else { 46 }, // closed / open hi-hat
                f if f >= 1000.0 => 38,                                     // snare
                _ => 36,                                                    // bass drum
            }
        } else {
            match mid {
                ..=40 => 36,   // bass drum
                41..=47 => 45, // low tom
                48..=55 => 47, // mid tom
                56..=67 => 50, // high tom
                68..=83 => 76, // wood block
                _ => 75,       // claves
            }
        });
    }
    let plucked = snd.decay < 2.0 || snd.sustain < 0.3;
    if mid < 48 {
        return Voice::Program(match () {
            _ if plucked && snd.centroid > 3.0 => 38, // synth bass 1
            _ if plucked => 33,                       // finger bass
            _ if snd.centroid > 3.0 || noisy => 39,   // synth bass 2
            _ => 32,                                  // acoustic bass
        });
    }
    Voice::Program(if plucked {
        match () {
            _ if mid >= 79 && snd.centroid < 2.0 => 9, // glockenspiel
            _ if snd.centroid < 1.4 && snd.decay < 0.3 => 12, // marimba
            _ if snd.centroid < 1.4 => 11,             // vibraphone
            _ if snd.centroid < 2.5 => 0,              // piano
            _ if snd.centroid < 5.0 => 4,              // electric piano
            _ => 6,                                    // harpsichord
        }
    } else if snd.attack > 0.15 {
        // slowly fading in
        if snd.centroid < 1.4 { 89 } else if snd.centroid < 3.0 { 49 } else { 50 } // warm pad / strings / synth strings
    } else if noisy {
        81 // sawtooth lead
    } else {
        match () {
            _ if snd.centroid < 1.3 => if mid >= 60 { 73 } else { 16 }, // flute / drawbar organ
            _ if snd.centroid < 2.5 => if mid >= 72 { 68 } else { 71 }, // oboe / clarinet
            _ if snd.centroid < 5.0 => if mid >= 60 { 56 } else { 61 }, // trumpet / brass section
            _ => if mid >= 60 { 81 } else { 30 },                       // sawtooth lead / distortion guitar
        }
    })
}

/// An instrument of a song: the keys and number of notes it plays, how it
/// sounds and the voice chosen for it.
pub struct Instrument {
    pub low: u8,
    pub high: u8,
    pub notes: usize,
    pub sound: Sound,
    pub voice: Voice,
}

/// Voices for the instruments of song `song`: the override table first
/// (`NN:patch`, then `patch`), then `gm_guess`. White noise hits are told
/// apart by how often they play: more often than the bass drum makes them
/// a hi-hat, otherwise a snare.
pub fn song_voices(notes: &[Note], song: usize, table: &InstrumentTable) -> BTreeMap<Patch, Instrument> {
    let mut m = BTreeMap::new();
    let mut fixed = Vec::new();
    for (p, (low, high, cnt)) in patch_ranges(notes) {
        let sound = measure(&p, ((low as u16 + high as u16) / 2) as u8);
        let h = p.hex();
        let voice = match table.get(&format!("{:02}:{}", song, h)).or_else(|| table.get(&h)) {
            Some(v) => {
                fixed.push(p);
                *v
            }
            None => gm_guess(&sound, low, high),
        };
        m.insert(p, Instrument { low, high, notes: cnt, sound, voice });
    }
    let kick = m.values().filter(|i| i.voice == Voice::Drum(36)).map(|i| i.notes).max().unwrap_or(0);
    for (p, i) in m.iter_mut() {
        if !fixed.contains(p) && matches!(i.voice, Voice::Drum(42 | 46)) && i.notes <= kick {
            i.voice = Voice::Drum(38);
        }
    }
    m
}

/// Overrides `patch hex → voice`, read from a text file: one instrument per
/// line, `[NN:]<patch hex> <program 0-127>` or `[NN:]<patch hex> drum <key>`
/// (`NN:` limits it to song NN, `#` starts a comment). `--export-midi`
/// writes the instruments of all songs in this format (`instruments.txt`).
pub type InstrumentTable = std::collections::HashMap<String, Voice>;

pub fn parse_instrument_table(text: &str) -> InstrumentTable {
    let mut t = InstrumentTable::new();
    for line in text.lines() {
        let line = line.split('#').next().unwrap_or("").trim();
        let w: Vec<&str> = line.split_whitespace().collect();
        let v = match w.as_slice() {
            [p, n] => n.parse::<u8>().ok().filter(|&n| n < 128).map(|n| (p, Voice::Program(n))),
            [p, d, k] if d.eq_ignore_ascii_case("drum") => k.parse::<u8>().ok().filter(|&k| k < 128).map(|k| (p, Voice::Drum(k))),
            _ => None,
        };
        if let Some((p, v)) = v {
            t.insert(p.to_ascii_lowercase(), v);
        }
    }
    t
}

/// MIDI note of an OPL F-number/block.
fn midi_note(fnum: u16, block: u8) -> Option<u8> {
    if fnum == 0 {
        return None;
    }
    let f = fnum as f64 * 49716.0 / (1u32 << (20 - block as u32)) as f64;
    let n = (69.0 + 12.0 * (f / 440.0).log2()).round();
    if (0.0..=127.0).contains(&n) {
        Some(n as u8)
    } else {
        None
    }
}

/// MIDI velocity of a carrier total level (0.75 dB per step).
fn velocity(tl: u8) -> u8 {
    let db = -0.75 * (tl & 0x3f) as f64;
    // General MIDI: dB = 40 log10(v / 127)
    (127.0 * 10f64.powf(db / 40.0)).round().clamp(1.0, 127.0) as u8
}

/// A note as the OPL played it.
#[derive(Clone, Copy, Debug)]
pub struct Note {
    pub start: u32,
    pub end: u32,
    pub ch: u8,
    pub key: u8,
    pub vel: u8,
    pub patch: Patch,
}

/// Rhythm mode notes (0xbd bits): bass drum, snare, tom, cymbal, hi-hat.
const RHYTHM_KEYS: [u8; 5] = [42, 49, 45, 38, 36];

/// Replays the song against the OPL registers and collects its notes
/// (`rhythm` gets the rhythm-mode hits as (start, end, GM key)). Returns
/// the song's length in ticks (where it loops).
pub fn song_notes(song: &Song) -> (Vec<Note>, Vec<(u32, u32, u8)>, u32) {
    let mut r = [0u8; 256];
    let mut on: [Option<Note>; 9] = [None; 9];
    let mut notes = Vec::new();
    let mut rhythm = Vec::new();
    let mut rhythm_on: [Option<u32>; 5] = [None; 5];
    let mut t = 0u32;
    let n = song.commands.len();
    for (i, &(reg, val, delay)) in song.commands.iter().enumerate() {
        let old = r[reg as usize];
        r[reg as usize] = val;
        match reg {
            0xa0..=0xa8 | 0xb0..=0xb8 => {
                let ch = (reg & 0x0f) as usize;
                let key_on = r[0xb0 + ch] & 0x20 != 0;
                let fnum = r[0xa0 + ch] as u16 | ((r[0xb0 + ch] as u16 & 3) << 8);
                let block = (r[0xb0 + ch] >> 2) & 7;
                let key = midi_note(fnum, block);
                let was_on = reg & 0xf0 == 0xb0 && old & 0x20 != 0 || reg & 0xf0 == 0xa0 && on[ch].is_some();
                // a new key-on, or a pitch change of the sounding note
                let retrigger = key_on && (!was_on || on[ch].map(|o| Some(o.key) != key).unwrap_or(true));
                if (!key_on || retrigger) && on[ch].is_some() {
                    let mut o = on[ch].take().unwrap();
                    o.end = t;
                    if o.end > o.start {
                        notes.push(o);
                    }
                }
                if retrigger {
                    if let Some(k) = key {
                        let o = OP_OFF[ch] as usize + 3;
                        on[ch] = Some(Note { start: t, end: t, ch: ch as u8, key: k, vel: velocity(r[0x40 + o]), patch: Patch::from_regs(&r, ch) });
                    }
                }
            }
            0xbd => {
                let enabled = val & 0x20 != 0;
                for b in 0..5 {
                    let hit = enabled && val & (1 << b) != 0;
                    let was = old & 0x20 != 0 && old & (1 << b) != 0;
                    if hit && !was {
                        rhythm_on[b] = Some(t);
                    } else if !hit {
                        if let Some(s) = rhythm_on[b].take() {
                            rhythm.push((s, t.max(s + 1), RHYTHM_KEYS[b]));
                        }
                    }
                }
            }
            _ => {}
        }
        if i + 1 < n {
            t += delay as u32;
        }
    }
    // the original restarts the tick after the last command
    let len = t + 1;
    for o in on.iter_mut() {
        if let Some(mut o) = o.take() {
            o.end = len;
            notes.push(o);
        }
    }
    for (b, s) in rhythm_on.iter().enumerate() {
        if let Some(s) = s {
            rhythm.push((*s, len, RHYTHM_KEYS[b]));
        }
    }
    (notes, rhythm, len)
}

/// Range of keys each instrument is played with.
fn patch_ranges(notes: &[Note]) -> BTreeMap<Patch, (u8, u8, usize)> {
    let mut m: BTreeMap<Patch, (u8, u8, usize)> = BTreeMap::new();
    for n in notes {
        let e = m.entry(n.patch).or_insert((127, 0, 0));
        e.0 = e.0.min(n.key);
        e.1 = e.1.max(n.key);
        e.2 += 1;
    }
    m
}

fn vlq(out: &mut Vec<u8>, mut v: u32) {
    let mut buf = [0u8; 5];
    let mut i = 4;
    buf[i] = (v & 0x7f) as u8;
    v >>= 7;
    while v > 0 {
        i -= 1;
        buf[i] = (v & 0x7f) as u8 | 0x80;
        v >>= 7;
    }
    out.extend_from_slice(&buf[i..]);
}

/// Converts song number `n` to a Standard MIDI File (format 0).
pub fn song_to_midi(song: &Song, n: usize, table: &InstrumentTable) -> Vec<u8> {
    let (notes, rhythm, len) = song_notes(song);
    let voices = song_voices(&notes, n, table);
    // events: (time, order, bytes); note-offs sort before program changes
    // and note-ons
    let mut ev: Vec<(u32, u8, Vec<u8>)> = Vec::new();
    let mut program: [Option<u8>; 16] = [None; 16];
    let mut sorted = notes.clone();
    sorted.sort_by_key(|n| (n.start, n.ch));
    for n in &sorted {
        let (ch, key) = match voices[&n.patch].voice {
            Voice::Drum(k) => (DRUM_CH, k),
            Voice::Program(p) => {
                let ch = n.ch;
                if program[ch as usize] != Some(p) {
                    program[ch as usize] = Some(p);
                    ev.push((n.start, 1, vec![0xc0 | ch, p]));
                }
                (ch, n.key)
            }
        };
        ev.push((n.start, 2, vec![0x90 | ch, key, n.vel]));
        ev.push((n.end, 0, vec![0x80 | ch, key, 0]));
    }
    for &(s, e, key) in &rhythm {
        ev.push((s, 2, vec![0x90 | DRUM_CH, key, 100]));
        ev.push((e, 0, vec![0x80 | DRUM_CH, key, 0]));
    }
    ev.sort_by_key(|e| (e.0, e.1));

    let mut trk = Vec::new();
    // tempo 500000 us per quarter: 120 bpm, PPQ ticks per half second
    trk.extend_from_slice(&[0x00, 0xff, 0x51, 0x03, 0x07, 0xa1, 0x20]);
    for ch in 0..16u8 {
        // a little reverb, no chorus
        trk.extend_from_slice(&[0x00, 0xb0 | ch, 91, 30, 0x00, 0xb0 | ch, 93, 0]);
    }
    let mut now = 0u32;
    for (t, _, b) in &ev {
        vlq(&mut trk, t - now);
        now = *t;
        trk.extend_from_slice(b);
    }
    vlq(&mut trk, len.saturating_sub(now));
    trk.extend_from_slice(&[0xff, 0x2f, 0x00]);

    let mut out = Vec::new();
    out.extend_from_slice(b"MThd");
    out.extend_from_slice(&6u32.to_be_bytes());
    out.extend_from_slice(&0u16.to_be_bytes());
    out.extend_from_slice(&1u16.to_be_bytes());
    out.extend_from_slice(&PPQ.to_be_bytes());
    out.extend_from_slice(b"MTrk");
    out.extend_from_slice(&(trk.len() as u32).to_be_bytes());
    out.extend_from_slice(&trk);
    out
}

const GM_NAMES: [&str; 128] = [
    "Acoustic Grand Piano", "Bright Acoustic Piano", "Electric Grand Piano", "Honky-tonk Piano", "Electric Piano 1", "Electric Piano 2", "Harpsichord", "Clavinet",
    "Celesta", "Glockenspiel", "Music Box", "Vibraphone", "Marimba", "Xylophone", "Tubular Bells", "Dulcimer",
    "Drawbar Organ", "Percussive Organ", "Rock Organ", "Church Organ", "Reed Organ", "Accordion", "Harmonica", "Tango Accordion",
    "Nylon Guitar", "Steel Guitar", "Jazz Guitar", "Clean Guitar", "Muted Guitar", "Overdriven Guitar", "Distortion Guitar", "Guitar Harmonics",
    "Acoustic Bass", "Finger Bass", "Pick Bass", "Fretless Bass", "Slap Bass 1", "Slap Bass 2", "Synth Bass 1", "Synth Bass 2",
    "Violin", "Viola", "Cello", "Contrabass", "Tremolo Strings", "Pizzicato Strings", "Orchestral Harp", "Timpani",
    "String Ensemble 1", "String Ensemble 2", "Synth Strings 1", "Synth Strings 2", "Choir Aahs", "Voice Oohs", "Synth Voice", "Orchestra Hit",
    "Trumpet", "Trombone", "Tuba", "Muted Trumpet", "French Horn", "Brass Section", "Synth Brass 1", "Synth Brass 2",
    "Soprano Sax", "Alto Sax", "Tenor Sax", "Baritone Sax", "Oboe", "English Horn", "Bassoon", "Clarinet",
    "Piccolo", "Flute", "Recorder", "Pan Flute", "Blown Bottle", "Shakuhachi", "Whistle", "Ocarina",
    "Square Lead", "Sawtooth Lead", "Calliope Lead", "Chiff Lead", "Charang Lead", "Voice Lead", "Fifths Lead", "Bass + Lead",
    "New Age Pad", "Warm Pad", "Polysynth Pad", "Choir Pad", "Bowed Pad", "Metallic Pad", "Halo Pad", "Sweep Pad",
    "Rain", "Soundtrack", "Crystal", "Atmosphere", "Brightness", "Goblins", "Echoes", "Sci-fi",
    "Sitar", "Banjo", "Shamisen", "Koto", "Kalimba", "Bagpipe", "Fiddle", "Shanai",
    "Tinkle Bell", "Agogo", "Steel Drums", "Woodblock", "Taiko Drum", "Melodic Tom", "Synth Drum", "Reverse Cymbal",
    "Guitar Fret Noise", "Breath Noise", "Seashore", "Bird Tweet", "Telephone Ring", "Helicopter", "Applause", "Gunshot",
];

fn voice_text(v: Voice) -> String {
    match v {
        Voice::Program(p) => format!("{:<9}# {}", p, GM_NAMES[p as usize]),
        Voice::Drum(k) => format!("drum {:<4}# {}", k, match k {
            35 | 36 => "bass drum",
            37 => "side stick",
            38 | 40 => "snare",
            39 => "hand clap",
            41 | 43 | 45 | 47 | 48 | 50 => "tom",
            42 | 44 => "closed hi-hat",
            46 => "open hi-hat",
            49 | 52 | 55 | 57 => "crash cymbal",
            51 | 53 | 59 => "ride cymbal",
            56 => "cowbell",
            75 => "claves",
            76 | 77 => "wood block",
            _ => "percussion",
        }),
    }
}

/// The instruments of song `n` with the voices chosen for them, in the
/// format of the override table (`instruments.txt` of `--export-midi`).
pub fn song_report(song: &Song, n: usize, table: &InstrumentTable) -> String {
    let (notes, rhythm, len) = song_notes(song);
    let mut s = format!("# song {:02}: {:.1} s, {} notes", n, len as f64 / IMF_HZ as f64, notes.len());
    if !rhythm.is_empty() {
        s.push_str(&format!(", {} rhythm mode hits (drums)", rhythm.len()));
    }
    s.push('\n');
    for (p, i) in song_voices(&notes, n, table) {
        let mut chans: Vec<u8> = notes.iter().filter(|x| x.patch == p).map(|x| x.ch).collect();
        chans.sort();
        chans.dedup();
        let m = i.sound;
        s.push_str(&format!(
            "{:02}:{} {}  (channel {:?}, keys {}-{}, {} notes; attack {:.2} s, decay {} s, sustain {:.2}, tonal {:.2}, harmonics {:.1}, {:.0} Hz)\n",
            n,
            p.hex(),
            voice_text(i.voice),
            chans,
            i.low,
            i.high,
            i.notes,
            m.attack,
            if m.decay >= 9.0 { "-".to_string() } else { format!("{:.2}", m.decay) },
            m.sustain,
            m.tonal,
            m.centroid,
            m.zcr_hz
        ));
    }
    s.push('\n');
    s
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn notes_and_timing() {
        // channel 0: A4 (fnum 0x244 block 4 ~ 440 Hz) for 100 ticks, then 20 ticks pause
        let song = Song { commands: vec![(0x40 + 3, 0, 0), (0xa0, 0x44, 0), (0xb0, 0x32, 100), (0xb0, 0x12, 20), (0x00, 0x00, 0)] };
        let (notes, _, len) = song_notes(&song);
        assert_eq!(notes.len(), 1);
        assert_eq!((notes[0].start, notes[0].end, notes[0].key), (0, 100, 69));
        assert_eq!(len, 121);
        let mid = song_to_midi(&song, 0, &InstrumentTable::new());
        assert_eq!(&mid[..4], b"MThd");
        assert!(rustysynth::MidiFile::new(&mut std::io::Cursor::new(mid)).is_ok());
    }

    #[test]
    fn vlq_encoding() {
        let mut v = Vec::new();
        vlq(&mut v, 0x3fff);
        assert_eq!(v, [0xff, 0x7f]);
    }
}
