//! Platform layer in the browser (wasm32), the counterpart of `sys.rs`.
//!
//! The game keeps its own loop like the DOS original, so it runs in a web
//! worker, where it may block (`Atomics.wait`). The page (web/www/main.js)
//! and the worker share three SharedArrayBuffers:
//!
//! * control (`Int32Array`, indexes `C_*`): the size of the game area, the
//!   fullscreen state, a ring of key events (scan code, pressed, repeat)
//!   and the state of up to 4 gamepads, written by the page (only it can
//!   read keyboard events and `navigator.getGamepads()`);
//! * frame: the presented picture as RGBA (header `F_*`), drawn by the page
//!   on each animation frame;
//! * audio: a ring of stereo samples (header `A_*`) played by an
//!   AudioWorklet (web/www/audio.js), filled from the emulated sound chips.
//!
//! Gamepads use the browser's "standard" button layout, which matches the
//! binding codes of `input::PAD_BUTTON_NAMES` (A B X Y LB LT RB RT ...).
//! Pads the browser has no mapping for report raw buttons
//! (`RawButton(HID_BUTTON_PAGE << 16 | n)`, n from 1) and axes
//! (`RawAxis(n)`), like unmapped pads natively; a hat switch reported as
//! one axis (value -1..1 in eighths, > 1 when centered) moves as well.

use super::input::{scan_to_ascii, Input, PadInput, HID_BUTTON_PAGE, PAD_ACTIONS, SC_LSHIFT, SC_RSHIFT};
use super::video::{Palette, Surface, EGA_RGB, SCREEN_W};
use js_sys::{Atomics, Float32Array, Int32Array, SharedArrayBuffer, Uint8Array};
use std::cell::RefCell;
use wasm_bindgen::prelude::*;

pub const TICRATE: u64 = 70;

/// widest play field view (32:9 with the aspect correction)
pub const MAX_VIEW_W: usize = 864;

// control buffer (Int32 indexes)
/// always 0: `Atomics.wait` on it sleeps
pub const C_SLEEP: u32 = 0;
/// size of the game area in CSS pixels
pub const C_AREA_W: u32 = 1;
pub const C_AREA_H: u32 = 2;
/// number of key events written so far
pub const C_KEY_WRITE: u32 = 3;
/// the page is in fullscreen mode
pub const C_FULLSCREEN: u32 = 4;
/// the Pause key was pressed (set by the page, cleared here)
pub const C_PAUSE: u32 = 5;
/// key event ring: scan code | pressed << 8 | repeat << 9
pub const C_KEYS: u32 = 64;
pub const KEY_RING: u32 = 256;
/// gamepads: `PAD_SLOTS` blocks of `PAD_STRIDE` ints at `C_PADS`: 0
/// connected, 1 standard mapping, 2 buttons, 3 axes, 4 buttons pressed
/// (bit mask), 16.. axes (f32)
pub const C_PADS: u32 = 1024;
pub const PAD_STRIDE: u32 = 64;
pub const PAD_SLOTS: usize = 4;
pub const PAD_AXES_AT: u32 = 16;
pub const PAD_MAX_AXES: usize = 16;
pub const CONTROL_BYTES: u32 = 8192;

// frame buffer: Int32 header, RGBA pixels from byte `F_PIXELS`
/// incremented before (odd) and after (even) writing a frame
pub const F_SEQ: u32 = 0;
pub const F_W: u32 = 1;
pub const F_H: u32 = 2;
/// display size (logical, with the aspect correction)
pub const F_DISP_W: u32 = 3;
pub const F_DISP_H: u32 = 4;
/// EGA border (overscan) color, 0xRRGGBB
pub const F_BORDER: u32 = 5;
pub const F_PIXELS: u32 = 64;
pub const FRAME_BYTES: u32 = F_PIXELS + 2048 * 512 * 4;

// audio buffer: Int32 header, interleaved stereo f32 from byte `A_SAMPLES`
/// frames written so far (by the game)
pub const A_WRITE: u32 = 0;
/// frames played so far (by the AudioWorklet)
pub const A_READ: u32 = 1;
pub const A_SAMPLES: u32 = 64;
/// ring size in stereo frames (a power of two)
pub const A_FRAMES: u32 = 16384;
pub const AUDIO_BYTES: u32 = A_SAMPLES + A_FRAMES * 8;
/// how far ahead of the player the samples are rendered (frames, ~70 ms)
pub const A_AHEAD: i32 = 3072;

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = performance, js_name = now)]
    fn perf_now() -> f64;
}

/// The shared buffers, set by the page before the game starts.
struct Shared {
    ctrl: Int32Array,
    ctrl_f: Float32Array,
    frame: Int32Array,
    frame_u8: Uint8Array,
    audio: Int32Array,
    audio_f: Float32Array,
}

/// Produces interleaved stereo samples (the sound manager's synth).
pub type AudioSource = Box<dyn FnMut(&mut [f32])>;

thread_local! {
    static SHARED: RefCell<Option<Shared>> = const { RefCell::new(None) };
    static AUDIO: RefCell<Option<AudioSource>> = const { RefCell::new(None) };
}

/// Hands the shared buffers to the platform layer (before `Sys::new`).
pub fn init_shared(ctrl: &SharedArrayBuffer, frame: &SharedArrayBuffer, audio: &SharedArrayBuffer) {
    let s = Shared {
        ctrl: Int32Array::new(ctrl),
        ctrl_f: Float32Array::new(ctrl),
        frame: Int32Array::new(frame),
        frame_u8: Uint8Array::new(frame),
        audio: Int32Array::new(audio),
        audio_f: Float32Array::new(audio),
    };
    SHARED.with(|x| *x.borrow_mut() = Some(s));
}

/// Whether the page provided the shared buffers (without them the game
/// runs like headless: no picture, input or sound).
pub fn page_connected() -> bool {
    SHARED.with(|s| s.borrow().is_some())
}

/// Sets the source of the sound output (the sound manager).
pub fn set_audio_source(src: AudioSource) {
    AUDIO.with(|a| *a.borrow_mut() = Some(src));
}

fn with_shared<R>(f: impl FnOnce(&Shared) -> R) -> Option<R> {
    SHARED.with(|s| s.borrow().as_ref().map(f))
}

/// The audio ring's state as seen by the producer.
#[derive(Default)]
struct AudioClock {
    last_read: i32,
    read_time: f64,
    prev_pump: f64,
    /// frames the player would have played while it is not running (the
    /// AudioContext waits for a click): the sound chips go on in real time
    virt: i32,
    virt_frac: f64,
    buf: Vec<f32>,
}

pub struct Sys {
    pub surface: Surface,
    pub palette: Palette,
    pub border: u8,
    pub input: Input,
    /// window was closed / quit requested
    pub quit: bool,
    /// force a fixed TimeCount progression (headless/testing)
    pub headless_time: Option<i64>,
    pub fullscreen_aspect: bool,
    fullscreen_pref: bool,
    start: f64,
    time_offset: i64,
    present_buf: Vec<u32>,
    rgba: Vec<u8>,
    /// keyboard state from the events of the page
    key_read: i32,
    down: [bool; 128],
    /// gamepads: actions held in the last poll, buttons / axes of each
    /// slot in the last poll (for presses and binding)
    pad_prev: [bool; PAD_ACTIONS],
    pad_buttons_prev: [u32; PAD_SLOTS],
    pad_axes_prev: [[f32; PAD_MAX_AXES]; PAD_SLOTS],
    /// axes of each slot seen reporting a centered hat switch (> 1)
    pad_hats: [u32; PAD_SLOTS],
    pads_seen_at_start: bool,
    audio: AudioClock,
}

impl Sys {
    pub fn new(scale: usize, _window_scale: usize, headless: bool, aspect: bool, fullscreen: bool) -> Sys {
        let now = perf_now();
        Sys {
            surface: Surface::new(scale),
            palette: Palette::default(),
            border: 0,
            input: Input::default(),
            quit: false,
            headless_time: if headless || !page_connected() { Some(0) } else { None },
            fullscreen_aspect: aspect,
            fullscreen_pref: fullscreen,
            start: now,
            time_offset: 0,
            present_buf: Vec::new(),
            rgba: Vec::new(),
            key_read: with_shared(|s| Atomics::load(&s.ctrl, C_KEY_WRITE).unwrap_or(0)).unwrap_or(0),
            down: [false; 128],
            pad_prev: [false; PAD_ACTIONS],
            pad_buttons_prev: [0; PAD_SLOTS],
            pad_axes_prev: [[0.0; PAD_MAX_AXES]; PAD_SLOTS],
            pad_hats: [0; PAD_SLOTS],
            pads_seen_at_start: false,
            audio: AudioClock { read_time: now, prev_pump: now, ..Default::default() },
        }
    }

    fn ticks(&self) -> i64 {
        ((perf_now() - self.start) * TICRATE as f64 / 1000.0) as i64
    }

    /// The global TimeCount (70 Hz).
    pub fn time_count(&self) -> i32 {
        if let Some(t) = self.headless_time {
            return t as i32;
        }
        (self.ticks() + self.time_offset) as i32
    }

    pub fn set_time_count(&mut self, v: i32) {
        if let Some(t) = &mut self.headless_time {
            *t = v as i64;
            return;
        }
        self.time_offset = v as i64 - self.ticks();
    }

    /// Blocks the worker for `ms` milliseconds.
    pub fn sleep_ms(&mut self, ms: u32) {
        if self.headless_time.is_some() {
            return;
        }
        with_shared(|s| {
            let _ = Atomics::wait_with_timeout(&s.ctrl, C_SLEEP, 0, ms as f64);
        });
    }

    /// Sleeps until TimeCount reaches `t` (keeps the input and sound going).
    pub fn wait_until(&mut self, t: i32) {
        if let Some(h) = &mut self.headless_time {
            if (*h as i32) < t {
                *h = t as i64;
            }
            return;
        }
        loop {
            let now = self.time_count();
            if now >= t || self.quit {
                break;
            }
            let remaining = (t - now) as u64;
            self.sleep_ms(((remaining * 1000 / TICRATE) as u32).clamp(1, 4));
            self.poll();
        }
    }

    /// Waits `n` vertical blanks (70 Hz).
    pub fn wait_vbl(&mut self, n: i32) {
        let t = self.time_count() + n;
        self.present();
        self.wait_until(t);
    }

    /// Reads the keyboard and gamepads and keeps the sound buffer filled.
    pub fn poll(&mut self) {
        if let Some(h) = &mut self.headless_time {
            *h += 1;
            return;
        }
        self.pump_audio();
        self.sync_keys();
    }

    /// Logical width of a play field view that fills the game area (see
    /// the native `Sys::wide_width`).
    pub fn wide_width(&self) -> usize {
        let Some((w, h)) = with_shared(|s| (Atomics::load(&s.ctrl, C_AREA_W).unwrap_or(0), Atomics::load(&s.ctrl, C_AREA_H).unwrap_or(0))) else {
            return SCREEN_W;
        };
        if w <= 0 || h <= 0 {
            return SCREEN_W;
        }
        let disp_h = if self.fullscreen_aspect { 240 } else { 200 };
        let w = w as usize * disp_h / h as usize;
        (w & !1).clamp(SCREEN_W, MAX_VIEW_W)
    }

    /// Logical size of a play field view zoomed out to `zoom` percent.
    pub fn zoomed_size(&self, zoom: u32) -> (usize, usize) {
        let zoom = zoom.clamp(50, 100) as usize;
        let h = (200 * 100 / zoom) & !1;
        let w = (self.wide_width() * h / 200) & !1;
        (w, h)
    }

    /// Whether the page is in fullscreen mode.
    pub fn fullscreen(&self) -> bool {
        with_shared(|s| Atomics::load(&s.ctrl, C_FULLSCREEN).unwrap_or(0) != 0).unwrap_or(self.fullscreen_pref)
    }

    /// (only the page can enter fullscreen, on a click or key press: F11)
    pub fn set_fullscreen(&mut self, on: bool) {
        self.fullscreen_pref = on;
    }

    pub fn set_grab_hotkeys(&mut self, _on: bool) {}

    /// Renders the samples the AudioWorklet will need next. (The frame
    /// counters wrap around: `A_FRAMES` is a power of two.)
    fn pump_audio(&mut self) {
        let a = &mut self.audio;
        let now = perf_now();
        AUDIO.with(|src| {
            let mut src = src.borrow_mut();
            let Some(src) = src.as_mut() else { return };
            with_shared(|s| {
                let read = Atomics::load(&s.audio, A_READ).unwrap_or(0);
                let mut write = Atomics::load(&s.audio, A_WRITE).unwrap_or(0);
                if read != a.last_read {
                    a.last_read = read;
                    a.read_time = now;
                    a.virt = read;
                    a.virt_frac = 0.0;
                } else if now - a.read_time > 250.0 {
                    // not playing (yet): consume in real time
                    a.virt_frac += (now - a.prev_pump) * super::sound::SAMPLE_RATE as f64 / 1000.0;
                    let whole = a.virt_frac.floor();
                    a.virt_frac -= whole;
                    a.virt = a.virt.wrapping_add(whole as i32);
                }
                a.prev_pump = now;
                let consumed = if a.virt.wrapping_sub(read) > 0 { a.virt } else { read };
                if write.wrapping_sub(consumed) < 0 {
                    write = consumed;
                }
                let n = (A_AHEAD - write.wrapping_sub(consumed)).min(A_FRAMES as i32 / 2);
                if n <= 0 {
                    return;
                }
                a.buf.resize(n as usize * 2, 0.0);
                src(&mut a.buf);
                // into the ring (wrapping around)
                let n = n as u32;
                let start = write as u32 & (A_FRAMES - 1);
                let first = (A_FRAMES - start).min(n);
                let base = A_SAMPLES / 4;
                s.audio_f.subarray(base + start * 2, base + (start + first) * 2).copy_from(&a.buf[..first as usize * 2]);
                if first < n {
                    s.audio_f.subarray(base, base + (n - first) * 2).copy_from(&a.buf[first as usize * 2..]);
                }
                Atomics::store(&s.audio, A_WRITE, write.wrapping_add(n as i32)).ok();
            });
        });
    }

    /// Gamepads (see the module comment): the bindings of
    /// `input.pad_map`, the left stick (or an unmapped pad's first axes /
    /// hat switch) always moves too.
    fn sync_pad(&mut self) {
        use super::input::*;
        let Some(pads) = with_shared(|s| {
            (0..PAD_SLOTS as u32)
                .map(|k| {
                    let b = C_PADS + k * PAD_STRIDE;
                    let g = |i: u32| s.ctrl.get_index(b + i);
                    let naxes = (g(3).max(0) as usize).min(PAD_MAX_AXES);
                    let mut axes = [0.0f32; PAD_MAX_AXES];
                    for (n, ax) in axes.iter_mut().enumerate().take(naxes) {
                        *ax = s.ctrl_f.get_index(b + PAD_AXES_AT + n as u32);
                    }
                    (g(0) != 0, g(1) != 0, g(4) as u32, naxes, axes)
                })
                .collect::<Vec<_>>()
        }) else {
            return;
        };
        let i = &mut self.input;
        let mut poll = PadPoll { prev: self.pad_prev, ..Default::default() };
        let mut count = 0;
        for (k, &(connected, standard, buttons, naxes, axes)) in pads.iter().enumerate() {
            let (prev_b, prev_a) = (self.pad_buttons_prev[k], self.pad_axes_prev[k]);
            self.pad_buttons_prev[k] = if connected { buttons } else { 0 };
            self.pad_axes_prev[k] = if connected { axes } else { [0.0; PAD_MAX_AXES] };
            if !connected {
                self.pad_hats[k] = 0;
                continue;
            }
            count += 1;
            for (n, &v) in axes.iter().enumerate().take(naxes) {
                if is_hat(v) {
                    self.pad_hats[k] |= 1 << n;
                }
            }
            let hats = self.pad_hats[k];
            // newly pressed buttons / axes moved past the threshold
            for n in 0..32u32 {
                if buttons & (1 << n) != 0 && prev_b & (1 << n) == 0 {
                    let inp = if standard { standard_button(n) } else { Some(PadInput::RawButton(HID_BUTTON_PAGE << 16 | (n + 1))) };
                    poll.captured = poll.captured.or(inp);
                    if standard {
                        poll.select |= n == 8;
                    } else if (0..4).contains(&n) {
                        poll.raw_face = true;
                    } else {
                        poll.raw_other = true;
                    }
                }
            }
            for n in 0..naxes {
                let (v, p) = (axes[n], prev_a[n]);
                if hats & (1 << n) != 0 {
                    continue;
                }
                if v.abs() >= 0.7 && p.abs() < 0.7 {
                    let inp = if standard {
                        standard_axis(n as u32, v > 0.0)
                    } else {
                        Some(PadInput::RawAxis(n as u32, v > 0.0))
                    };
                    poll.captured = poll.captured.or(inp);
                }
            }
            let mut a = [false; PAD_ACTIONS];
            for (act, &b) in a.iter_mut().zip(i.pad_map.iter()) {
                *act = pad_active(standard, buttons, &axes[..naxes], hats, b);
            }
            // the left stick (unmapped: the first axes or a hat switch)
            let (mut ax, mut ay) = (axes[0], axes[1]);
            if !standard {
                for n in (0..naxes).filter(|n| hats & (1 << n) != 0) {
                    if let Some((hx, hy)) = hat_dir(axes[n]).filter(|&d| d != (0.0, 0.0)) {
                        (ax, ay) = (hx, hy);
                    }
                }
            }
            a[PAD_LEFT] |= ax < -0.5;
            a[PAD_RIGHT] |= ax > 0.5;
            a[PAD_UP] |= ay < -0.5;
            a[PAD_DOWN] |= ay > 0.5;
            for (x, y) in poll.act.iter_mut().zip(a) {
                *x |= y;
            }
            if count <= 4 {
                poll.per_pad[count - 1] = a;
            }
        }
        i.pad_connected = count > 0;
        i.pad_count = count;
        if count >= 2 && !self.pads_seen_at_start && perf_now() - self.start < 5000.0 {
            self.pads_seen_at_start = true;
            i.two_pads_at_start = true;
        }
        self.pad_prev = poll.act;
        i.apply_pad(&poll);
    }

    fn sync_keys(&mut self) {
        self.sync_pad();
        let mut pressed = Vec::new();
        let paused = with_shared(|s| {
            let write = Atomics::load(&s.ctrl, C_KEY_WRITE).unwrap_or(0);
            // (more than a ring full behind: the oldest are lost)
            if write.wrapping_sub(self.key_read) > KEY_RING as i32 {
                self.key_read = write - KEY_RING as i32;
            }
            while self.key_read != write {
                let e = s.ctrl.get_index(C_KEYS + (self.key_read as u32 % KEY_RING));
                self.key_read = self.key_read.wrapping_add(1);
                let (sc, down, repeat) = ((e & 0xff) as u8, e & 0x100 != 0, e & 0x200 != 0);
                if sc == 0 || sc >= 128 {
                    continue;
                }
                self.down[sc as usize] = down;
                if down {
                    pressed.push((sc, repeat));
                }
            }
            Atomics::exchange(&s.ctrl, C_PAUSE, 0).unwrap_or(0) != 0
        })
        .unwrap_or(false);
        if paused {
            self.input.paused = true;
        }
        let now = self.down;
        let shift = now[SC_LSHIFT as usize] || now[SC_RSHIFT as usize];
        for (i, &d) in now.iter().enumerate() {
            if !d {
                self.input.masked[i] = false;
            }
            self.input.keyboard[i] = d && !self.input.masked[i];
        }
        for (sc, repeat) in pressed {
            self.input.masked[sc as usize] = false;
            self.input.last_scan = sc;
            self.input.last_repeat = repeat;
            let a = scan_to_ascii(sc, shift);
            if a != 0 {
                self.input.last_ascii = a;
            }
            // a key pressed and released between two polls still counts
            // as down for one poll, like the interrupt driven original
            self.input.keyboard[sc as usize] = true;
        }
    }

    /// Saves the current screen (palette applied) as a PNG file (in the
    /// browser: offered as a download).
    pub fn save_screenshot(&mut self, path: &std::path::Path) -> std::io::Result<()> {
        let identity = self.apply_palette();
        let s = &self.surface;
        let src: &[u32] = if identity { &s.px } else { &self.present_buf };
        let mut rgba = Vec::with_capacity(src.len() * 4);
        for &p in src {
            rgba.extend_from_slice(&[(p >> 16) as u8, (p >> 8) as u8, p as u8, 255]);
        }
        crate::dump::write_png(path, s.w, s.h, &rgba)
    }

    /// Applies the EGA palette registers to the surface into `present_buf`.
    fn apply_palette(&mut self) -> bool {
        if self.palette.is_default() {
            return true;
        }
        let (map, factor) = self.palette.mapping();
        let s = &self.surface;
        self.present_buf.resize(s.px.len(), 0);
        for (d, &p) in self.present_buf.iter_mut().zip(s.px.iter()) {
            let rgb = p & 0xffffff;
            *d = 0xff000000
                | match EGA_RGB.iter().position(|&c| c == rgb) {
                    Some(i) => map[i],
                    None => {
                        let sc = |c: u32| ((c as i32 * factor) >> 8).clamp(0, 255) as u32;
                        (sc((rgb >> 16) & 255) << 16) | (sc((rgb >> 8) & 255) << 8) | sc(rgb & 255)
                    }
                };
        }
        false
    }

    /// Hands the surface (palette applied) to the page.
    pub fn present(&mut self) {
        if let Some(h) = &mut self.headless_time {
            *h += 1;
            return;
        }
        let identity = self.apply_palette();
        let s = &self.surface;
        let src: &[u32] = if identity { &s.px } else { &self.present_buf };
        let bytes = (src.len() * 4) as u32;
        if F_PIXELS + bytes <= FRAME_BYTES {
            self.rgba.resize(src.len() * 4, 0);
            for (d, &p) in self.rgba.chunks_exact_mut(4).zip(src) {
                d.copy_from_slice(&[(p >> 16) as u8, (p >> 8) as u8, p as u8, 255]);
            }
            let disp_w = s.logical_w();
            let disp_h = if self.fullscreen_aspect { s.logical_h() * 6 / 5 } else { s.logical_h() };
            let border = EGA_RGB[(self.border & 15) as usize] & 0xffffff;
            let (w, h) = (s.w, s.h);
            let rgba = &self.rgba;
            with_shared(|sh| {
                let f = &sh.frame;
                Atomics::add(f, F_SEQ, 1).ok();
                f.set_index(F_W, w as i32);
                f.set_index(F_H, h as i32);
                f.set_index(F_DISP_W, disp_w as i32);
                f.set_index(F_DISP_H, disp_h as i32);
                f.set_index(F_BORDER, border as i32);
                sh.frame_u8.subarray(F_PIXELS, F_PIXELS + bytes).copy_from(rgba);
                Atomics::add(f, F_SEQ, 1).ok();
            });
        }
        self.poll();
    }
}

/// A pad with the browser's standard mapping: button n -> binding code
/// (`input::PAD_BUTTON_NAMES`: A B X Y LB LT RB RT SELECT START HOME LS RS
/// UP DOWN LEFT RIGHT).
fn standard_button(n: u32) -> Option<PadInput> {
    const MAP: [u8; 17] = [0, 1, 2, 3, 4, 6, 5, 7, 8, 9, 11, 12, 13, 14, 15, 16, 10];
    MAP.get(n as usize).map(|&c| PadInput::Button(c))
}

/// Standard mapping: axis n -> binding code (`PAD_AXIS_NAMES`: left
/// stick X / Y (up positive), right stick X / Y). The browser's Y axes
/// are positive downwards.
fn standard_axis(n: u32, pos: bool) -> Option<PadInput> {
    match n {
        0 | 2 => Some(PadInput::Axis(n as u8, pos)),
        1 | 3 => Some(PadInput::Axis(n as u8, !pos)),
        _ => None,
    }
}

/// A hat switch reported as one axis: -1 (up) .. 1 in steps of 2/7
/// clockwise, about 1.29 (or more) when centered.
fn is_hat(v: f32) -> bool {
    v > 1.05
}

/// The (x, y) direction of a hat switch axis (y downwards), None if `v`
/// is not one of its values.
fn hat_dir(v: f32) -> Option<(f32, f32)> {
    if is_hat(v) {
        return Some((0.0, 0.0));
    }
    let step = (v + 1.0) * 3.5;
    let k = step.round();
    if (step - k).abs() > 0.1 || !(0.0..=7.0).contains(&k) {
        return None;
    }
    const DIRS: [(f32, f32); 8] = [(0.0, -1.0), (1.0, -1.0), (1.0, 0.0), (1.0, 1.0), (0.0, 1.0), (-1.0, 1.0), (-1.0, 0.0), (-1.0, -1.0)];
    Some(DIRS[k as usize])
}

/// Whether a bound input is held.
fn pad_active(standard: bool, buttons: u32, axes: &[f32], hats: u32, b: PadInput) -> bool {
    let axis = |n: usize| if hats & (1 << n) != 0 { 0.0 } else { axes.get(n).copied().unwrap_or(0.0) };
    match b {
        PadInput::None => false,
        PadInput::Button(c) if standard => (0..17).any(|n| standard_button(n) == Some(PadInput::Button(c)) && buttons & (1 << n) != 0),
        PadInput::Axis(c, pos) if standard => {
            let v = match c {
                0 | 2 => axis(c as usize),
                1 | 3 => -axis(c as usize),
                _ => 0.0,
            };
            if pos {
                v > 0.5
            } else {
                v < -0.5
            }
        }
        PadInput::RawButton(c) if !standard => {
            let n = c & 0xffff;
            c >> 16 == HID_BUTTON_PAGE && (1..=32).contains(&n) && buttons & (1 << (n - 1)) != 0
        }
        PadInput::RawAxis(n, pos) if !standard => {
            let v = axis(n as usize);
            if pos {
                v > 0.5
            } else {
                v < -0.5
            }
        }
        // bindings made for the other kind of pad: d-pad bindings still
        // work on an unmapped pad's first axes / hat switch (`sync_pad`)
        _ => false,
    }
}
