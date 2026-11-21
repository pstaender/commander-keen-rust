//! Platform layer: window, presentation, keyboard and the 70 Hz TimeCount.

use super::input::{key_to_scan, scan_to_ascii, Input, SC_LSHIFT, SC_RSHIFT};
use super::video::{Palette, Surface, EGA_RGB, SCREEN_W};
use std::num::NonZeroU32;
use std::rc::Rc;
use std::time::{Duration, Instant};
use winit::application::ApplicationHandler;
use winit::dpi::LogicalSize;
use winit::event::{ElementState, WindowEvent};
use winit::event_loop::{ActiveEventLoop, EventLoop};
use winit::keyboard::{KeyCode, ModifiersState, PhysicalKey};
use winit::platform::pump_events::EventLoopExtPumpEvents;
use winit::window::{Fullscreen, Window, WindowId};

pub const TICRATE: u64 = 70;

/// widest play field view (32:9 with the aspect correction)
pub const MAX_VIEW_W: usize = 864;

const TITLE: &str = "Commander Keen 6 - Aliens Ate My Baby Sitter!";

/// The window (winit) and its pixel buffer (softbuffer). Events are pumped
/// from the game's own loop, like the original polled its interrupt state.
struct Win {
    size: LogicalSize<u32>,
    fullscreen: bool,
    window: Option<Rc<Window>>,
    context: Option<softbuffer::Context<Rc<Window>>>,
    surface: Option<softbuffer::Surface<Rc<Window>, Rc<Window>>>,
    /// keys currently held, by scan code
    down: [bool; 128],
    /// key presses since the last poll (scan code, auto repeat)
    pressed: Vec<(u8, bool)>,
    pause: bool,
    closed: bool,
    mods: ModifiersState,
    /// turn off the system keyboard shortcuts while the window has focus
    grab_hotkeys: bool,
    focused: bool,
    hotkeys_off: bool,
}

/// macOS: Ctrl+arrow keys are system shortcuts (switch Spaces, Mission
/// Control), so with Ctrl as the jump key the arrow presses never reach the
/// game. Like remote desktop clients, the window turns the global hotkeys
/// off while it has focus (back on while Cmd is held, for Cmd+Tab).
#[cfg(target_os = "macos")]
mod hotkeys {
    #[link(name = "CoreGraphics", kind = "framework")]
    extern "C" {
        fn CGSMainConnectionID() -> i32;
        fn CGSSetGlobalHotKeyOperatingMode(cid: i32, mode: i32) -> i32;
    }

    pub fn set_enabled(on: bool) {
        // kCGSGlobalHotKeyEnable = 0, kCGSGlobalHotKeyDisable = 1
        unsafe {
            CGSSetGlobalHotKeyOperatingMode(CGSMainConnectionID(), if on { 0 } else { 1 });
        }
    }
}

#[cfg(not(target_os = "macos"))]
mod hotkeys {
    pub fn set_enabled(_on: bool) {}
}

impl Drop for Win {
    fn drop(&mut self) {
        if self.hotkeys_off {
            hotkeys::set_enabled(true);
        }
    }
}

impl Win {
    fn update_hotkeys(&mut self) {
        let off = self.grab_hotkeys && self.focused && !self.mods.super_key();
        if off != self.hotkeys_off {
            self.hotkeys_off = off;
            hotkeys::set_enabled(!off);
        }
    }

    fn set_fullscreen(&mut self, on: bool) {
        self.fullscreen = on;
        if let Some(w) = &self.window {
            w.set_fullscreen(on.then_some(Fullscreen::Borderless(None)));
            w.set_cursor_visible(false);
        }
    }
}

impl ApplicationHandler for Win {
    fn resumed(&mut self, el: &ActiveEventLoop) {
        if self.window.is_some() {
            return;
        }
        let attrs = Window::default_attributes()
            .with_title(TITLE)
            .with_inner_size(self.size)
            .with_fullscreen(self.fullscreen.then_some(Fullscreen::Borderless(None)));
        let window = match el.create_window(attrs) {
            Ok(w) => Rc::new(w),
            Err(e) => {
                eprintln!("could not open window: {}", e);
                self.closed = true;
                return;
            }
        };
        window.set_cursor_visible(false);
        let context = softbuffer::Context::new(window.clone()).ok();
        self.surface = context.as_ref().and_then(|c| softbuffer::Surface::new(c, window.clone()).ok());
        self.context = context;
        self.window = Some(window);
    }

    fn window_event(&mut self, _el: &ActiveEventLoop, _id: WindowId, event: WindowEvent) {
        match event {
            WindowEvent::CloseRequested => self.closed = true,
            WindowEvent::ModifiersChanged(m) => {
                self.mods = m.state();
                self.update_hotkeys();
            }
            WindowEvent::Focused(f) => {
                if !f {
                    self.down = [false; 128];
                }
                self.focused = f;
                self.update_hotkeys();
            }
            WindowEvent::KeyboardInput { event, .. } => {
                let PhysicalKey::Code(code) = event.physical_key else { return };
                let pressed = event.state == ElementState::Pressed;
                // F11 (or Cmd/Win+F) toggles fullscreen and is not passed on
                if code == KeyCode::F11 || (code == KeyCode::KeyF && self.mods.super_key()) {
                    if pressed && !event.repeat {
                        self.set_fullscreen(!self.fullscreen);
                    }
                    return;
                }
                if code == KeyCode::Pause && pressed {
                    self.pause = true;
                }
                let sc = key_to_scan(code);
                if sc == 0 || sc as usize >= 128 {
                    return;
                }
                self.down[sc as usize] = pressed;
                if pressed {
                    self.pressed.push((sc, event.repeat));
                }
            }
            _ => {}
        }
    }
}

pub struct Sys {
    win: Option<(EventLoop<()>, Win)>,
    /// fullscreen setting, kept without a window (headless) for the config
    fullscreen_pref: bool,
    pad: Option<gilrs::Gilrs>,
    /// gamepad actions held in the last poll (for presses)
    pad_prev: [bool; super::input::PAD_ACTIONS],
    /// last value of each axis by code (to detect an axis being moved)
    pad_axis_prev: std::collections::HashMap<u32, f32>,
    /// KEEN_PAD_DEBUG=1: print the gamepad events
    pad_debug: bool,
    /// d-pads that are an axis pair mapped as buttons (see `dpad_axis`):
    /// the codes of those axes and the (x, y) direction per pad
    pad_dpad_codes: std::collections::HashSet<u32>,
    pad_dpad: std::collections::HashMap<usize, [i8; 2]>,
    pub surface: Surface,
    pub palette: Palette,
    pub border: u8,
    pub input: Input,
    start: Instant,
    time_offset: i64,
    /// window was closed / quit requested
    pub quit: bool,
    present_buf: Vec<u32>,
    /// force a fixed TimeCount progression (headless/testing)
    pub headless_time: Option<i64>,
    pub fullscreen_aspect: bool,
    /// debugging: save every Nth presented frame to KEEN_SHOT_DIR
    frame: u64,
    shot_dir: Option<std::path::PathBuf>,
    shot_every: u64,
    max_frames: u64,
    /// debugging: scripted key presses (frame, scancode, duration in frames)
    script: Vec<(u64, u8, u64)>,
}

impl Sys {
    pub fn new(scale: usize, window_scale: usize, headless: bool, aspect: bool, fullscreen: bool) -> Sys {
        let surface = Surface::new(scale);
        let win = if headless {
            None
        } else {
            match EventLoop::new() {
                Ok(mut el) => {
                    let wh = if aspect { 240 } else { 200 };
                    let mut w = Win {
                        size: LogicalSize::new(320 * window_scale as u32, wh * window_scale as u32),
                        fullscreen,
                        window: None,
                        context: None,
                        surface: None,
                        down: [false; 128],
                        pressed: Vec::new(),
                        pause: false,
                        closed: false,
                        mods: ModifiersState::empty(),
                        grab_hotkeys: true,
                        focused: false,
                        hotkeys_off: false,
                    };
                    // the window is created on the first `resumed` event
                    for _ in 0..100 {
                        el.pump_app_events(Some(Duration::from_millis(10)), &mut w);
                        if w.window.is_some() || w.closed {
                            break;
                        }
                    }
                    if w.window.is_some() {
                        Some((el, w))
                    } else {
                        None
                    }
                }
                Err(e) => {
                    eprintln!("could not open window: {}", e);
                    None
                }
            }
        };
        Sys {
            win,
            fullscreen_pref: fullscreen,
            pad: if headless { None } else { open_pads() },
            pad_prev: [false; super::input::PAD_ACTIONS],
            pad_axis_prev: Default::default(),
            pad_debug: std::env::var("KEEN_PAD_DEBUG").is_ok_and(|v| v == "1"),
            pad_dpad_codes: Default::default(),
            pad_dpad: Default::default(),
            surface,
            palette: Palette::default(),
            border: 0,
            // headless testing: KEEN_PAD=1 pretends a gamepad is connected
            input: Input { pad_connected: headless && std::env::var("KEEN_PAD").is_ok_and(|v| v == "1"), ..Input::default() },
            start: Instant::now(),
            time_offset: 0,
            quit: false,
            present_buf: Vec::new(),
            headless_time: if headless { Some(0) } else { None },
            fullscreen_aspect: aspect,
            frame: 0,
            shot_dir: std::env::var("KEEN_SHOT_DIR").ok().map(std::path::PathBuf::from),
            shot_every: std::env::var("KEEN_SHOT_EVERY").ok().and_then(|v| v.parse().ok()).unwrap_or(35),
            max_frames: std::env::var("KEEN_MAX_FRAMES").ok().and_then(|v| v.parse().ok()).unwrap_or(0),
            script: std::env::var("KEEN_KEYS")
                .map(|v| {
                    // "frame:scancode[+frames],..." with hex or decimal scan codes
                    v.split(',')
                        .filter_map(|e| {
                            let (f, rest) = e.split_once(':')?;
                            let (sc, dur) = rest.split_once('+').unwrap_or((rest, "3"));
                            let sc = if let Some(h) = sc.strip_prefix("0x") {
                                u8::from_str_radix(h, 16).ok()?
                            } else {
                                sc.parse().ok()?
                            };
                            Some((f.parse().ok()?, sc, dur.parse().ok()?))
                        })
                        .collect()
                })
                .unwrap_or_default(),
        }
    }

    /// The global TimeCount (70 Hz).
    pub fn time_count(&self) -> i32 {
        if let Some(t) = self.headless_time {
            return t as i32;
        }
        let el = self.start.elapsed();
        let ticks = (el.as_micros() as i64 * TICRATE as i64) / 1_000_000;
        (ticks + self.time_offset) as i32
    }

    pub fn set_time_count(&mut self, v: i32) {
        if let Some(t) = &mut self.headless_time {
            *t = v as i64;
            return;
        }
        let el = self.start.elapsed();
        let ticks = (el.as_micros() as i64 * TICRATE as i64) / 1_000_000;
        self.time_offset = v as i64 - ticks;
    }

    /// Sleeps until TimeCount reaches `t` (keeps the window responsive).
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
            std::thread::sleep(Duration::from_micros((remaining * 1_000_000 / TICRATE).min(4000)));
            self.poll();
        }
    }

    /// Waits `n` vertical blanks (70 Hz).
    pub fn wait_vbl(&mut self, n: i32) {
        let t = self.time_count() + n;
        self.present();
        self.wait_until(t);
    }

    /// Advances fake time in headless mode by one tic so busy loops progress.
    fn headless_tick(&mut self) {
        if let Some(h) = &mut self.headless_time {
            *h += 1;
        }
    }

    /// Pumps window events and updates keyboard state.
    pub fn poll(&mut self) {
        let Some((el, w)) = &mut self.win else {
            self.headless_tick();
            return;
        };
        el.pump_app_events(Some(Duration::ZERO), w);
        if w.closed {
            self.quit = true;
            return;
        }
        self.sync_keys();
    }

    /// Logical width of a play field view that fills the window: 200 lines
    /// high (displayed 240 high with the 4:3 aspect correction), as wide as
    /// the window's shape allows, at least the original 320 and at most
    /// `MAX_VIEW_W`. 320 without a window.
    pub fn wide_width(&self) -> usize {
        let Some(window) = self.win.as_ref().and_then(|(_, w)| w.window.as_ref()) else {
            // headless testing: KEEN_VIEW_W=n simulates a wide window
            let w = std::env::var("KEEN_VIEW_W").ok().and_then(|v| v.parse().ok()).unwrap_or(SCREEN_W);
            return (w & !1).clamp(SCREEN_W, MAX_VIEW_W);
        };
        let size = window.inner_size();
        if size.width == 0 || size.height == 0 {
            return SCREEN_W;
        }
        let disp_h = if self.fullscreen_aspect { 240 } else { 200 };
        let w = size.width as usize * disp_h / size.height as usize;
        (w & !1).clamp(SCREEN_W, MAX_VIEW_W)
    }

    /// Whether the window is in fullscreen mode.
    pub fn fullscreen(&self) -> bool {
        self.win.as_ref().map_or(self.fullscreen_pref, |(_, w)| w.fullscreen)
    }

    /// Whether the system keyboard shortcuts are turned off while the
    /// window has focus (macOS; `--system-shortcuts` keeps them).
    pub fn set_grab_hotkeys(&mut self, on: bool) {
        if let Some((_, w)) = &mut self.win {
            w.grab_hotkeys = on;
            w.update_hotkeys();
        }
    }

    pub fn set_fullscreen(&mut self, on: bool) {
        self.fullscreen_pref = on;
        if let Some((_, w)) = &mut self.win {
            w.set_fullscreen(on);
        }
    }

    /// Reads gamepads through the bindings in `input.pad_map` (the left
    /// stick always moves as well): movement and the jump / pogo / fire
    /// buttons are held states, MENU sets Esc, jump Space (for "press a
    /// key"), the state actions `pad_save` / `pad_load`. While the control
    /// panel is open the pad navigates it, while binding an action the next
    /// input goes to `pad_captured`. In the control panel fire answers Y
    /// (yes) in the Y/N questions.
    ///
    /// Pads gilrs has no mapping for (e.g. many SNES style USB pads) report
    /// raw codes: their X/Y axes and hat switch move as well, and in the
    /// control panel face buttons (HID buttons 1-4) select, other buttons
    /// back out, unless raw buttons are bound. KEEN_PAD_DEBUG=1 prints the
    /// pads and their events.
    fn sync_pad(&mut self) {
        use super::input::*;
        use gilrs::{Axis, Button, EventType, MappingSource};
        let Some(g) = &mut self.pad else { return };
        let i = &mut self.input;
        let mut captured = None;
        // raw buttons of unmapped pads pressed in this poll
        let (mut raw_face, mut raw_other) = (false, false);
        let mut select = false;
        while let Some(ev) = g.next_event() {
            if self.pad_debug {
                let pad = g.gamepad(ev.id);
                eprintln!("pad {} \"{}\" ({:?} mapping): {:?}", usize::from(ev.id), pad.name(), pad.mapping_source(), ev.event);
            }
            // a d-pad axis mapped as buttons: one button name for both
            // directions, the value gives the direction
            let btn_ev = match ev.event {
                EventType::ButtonChanged(b, v, code) => Some((b, Some(v), code.into_u32())),
                EventType::ButtonPressed(b, code) | EventType::ButtonReleased(b, code) => Some((b, None, code.into_u32())),
                _ => None,
            };
            if let Some((b, v, c)) = btn_ev {
                let vertical = matches!(b, Button::DPadUp | Button::DPadDown);
                if (vertical || matches!(b, Button::DPadLeft | Button::DPadRight))
                    && (dpad_axis(c) || self.pad_dpad_codes.contains(&c) || v.is_some_and(|v| v > 0.1 && v < 0.9))
                {
                    self.pad_dpad_codes.insert(c);
                    if let Some(v) = v {
                        // HID axes: the maximum is right / down
                        let dir = if v > 0.75 { 1 } else if v < 0.25 { -1 } else { 0 };
                        let d = self.pad_dpad.entry(usize::from(ev.id)).or_default();
                        let old = std::mem::replace(&mut d[vertical as usize], dir);
                        if dir != 0 && old != dir {
                            let btn = match (vertical, dir > 0) {
                                (false, false) => 15,
                                (false, true) => 16,
                                (true, false) => 13,
                                (true, true) => 14,
                            };
                            captured = captured.or(Some(PadInput::Button(btn)));
                        }
                    }
                    // the pressed / released events name the wrong direction
                    continue;
                }
            }
            match ev.event {
                EventType::ButtonPressed(Button::Select, _) => {
                    select = true;
                    captured = captured.or(Some(PadInput::Button(8)));
                }
                EventType::ButtonPressed(Button::Unknown, code) => {
                    let c = code.into_u32();
                    captured = captured.or(Some(PadInput::RawButton(c)));
                    if c >> 16 == HID_BUTTON_PAGE && (1..=4).contains(&(c & 0xffff)) {
                        raw_face = true;
                    } else {
                        raw_other = true;
                    }
                }
                EventType::ButtonPressed(b, _) => {
                    if let Some(c) = PAD_BUTTONS.iter().position(|&x| x == b) {
                        captured = captured.or(Some(PadInput::Button(c as u8)));
                    }
                }
                EventType::AxisChanged(a, v, code) => {
                    let prev = self.pad_axis_prev.insert(code.into_u32(), v).unwrap_or(0.0);
                    if v.abs() >= 0.7 && prev.abs() < 0.7 {
                        let p = if a == Axis::Unknown {
                            Some(PadInput::RawAxis(code.into_u32(), v > 0.0))
                        } else {
                            PAD_AXES.iter().position(|&x| x == a).map(|c| PadInput::Axis(c as u8, v > 0.0))
                        };
                        captured = captured.or(p);
                    }
                }
                _ => {}
            }
        }
        i.pad_connected = g.gamepads().next().is_some();
        let mut act = [false; PAD_ACTIONS];
        for (id, pad) in g.gamepads() {
            let dpad = self.pad_dpad.get(&usize::from(id)).copied().unwrap_or_default();
            for (k, &b) in i.pad_map.iter().enumerate() {
                act[k] |= pad_active(&pad, b, dpad, &self.pad_dpad_codes);
            }
            let (ax, ay) = if pad.mapping_source() == MappingSource::None {
                // unmapped: the raw X/Y axes or the hat switch (HID: down
                // is positive)
                let raw = |c: u32| raw_axis(&pad, c);
                let x = if raw(HID_X) != 0.0 { raw(HID_X) } else { raw(HID_HAT_X) };
                let y = if raw(HID_Y) != 0.0 { raw(HID_Y) } else { raw(HID_HAT_Y) };
                (x, -y)
            } else {
                (pad.value(Axis::LeftStickX), pad.value(Axis::LeftStickY))
            };
            act[PAD_LEFT] |= ax < -0.5;
            act[PAD_RIGHT] |= ax > 0.5;
            act[PAD_UP] |= ay > 0.5;
            act[PAD_DOWN] |= ay < -0.5;
        }
        let prev = std::mem::replace(&mut self.pad_prev, act);
        let pressed = |k: usize| act[k] && !prev[k];
        i.pad_x = 0;
        i.pad_y = 0;
        i.pad_jump = false;
        i.pad_pogo = false;
        i.pad_fire = false;
        if i.pad_capture {
            if captured.is_some() {
                i.pad_captured = captured;
            }
            return;
        }
        if i.pad_menu_nav {
            for (k, sc) in [
                (PAD_UP, SC_UPARROW),
                (PAD_DOWN, SC_DOWNARROW),
                (PAD_LEFT, SC_LEFTARROW),
                (PAD_RIGHT, SC_RIGHTARROW),
                (PAD_JUMP, SC_RETURN),
                // "yes" in the Y/N questions (quit, end game, resume)
                (PAD_FIRE, SC_Y),
                (PAD_POGO, SC_ESCAPE),
                (PAD_MENU, SC_ESCAPE),
            ] {
                if pressed(k) {
                    i.last_scan = sc;
                }
            }
            // SELECT backs out as well (unless bound to an action)
            if select && !i.pad_map.contains(&PadInput::Button(8)) {
                i.last_scan = SC_ESCAPE;
            }
            // unmapped pad whose buttons are not configured yet
            if !i.pad_map.iter().any(|b| matches!(b, PadInput::RawButton(_))) {
                if raw_face {
                    i.last_scan = SC_RETURN;
                } else if raw_other {
                    i.last_scan = SC_ESCAPE;
                }
            }
            return;
        }
        i.pad_x = if act[PAD_LEFT] { -1 } else if act[PAD_RIGHT] { 1 } else { 0 };
        i.pad_y = if act[PAD_UP] { -1 } else if act[PAD_DOWN] { 1 } else { 0 };
        i.pad_jump = act[PAD_JUMP];
        i.pad_pogo = act[PAD_POGO];
        i.pad_fire = act[PAD_FIRE];
        if pressed(PAD_JUMP) {
            i.last_scan = SC_SPACE;
        }
        if pressed(PAD_MENU) {
            i.last_scan = SC_ESCAPE;
        }
        i.pad_save |= pressed(PAD_SAVESTATE);
        i.pad_load |= pressed(PAD_LOADSTATE);
    }

    fn sync_keys(&mut self) {
        self.sync_pad();
        let Some((_, w)) = &mut self.win else { return };
        let now = w.down;
        let shift = now[SC_LSHIFT as usize] || now[SC_RSHIFT as usize];
        let pressed = std::mem::take(&mut w.pressed);
        if std::mem::take(&mut w.pause) {
            self.input.paused = true;
        }
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

    /// Saves the current screen (palette applied) as a PNG file.
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

    /// Saves debugging screenshots (of what the window would show).
    fn debug_frame(&mut self, identity: bool) {
        self.frame += 1;
        if !self.script.is_empty() {
            let f = self.frame;
            for &(start, sc, dur) in &self.script {
                if f == start {
                    self.input.last_scan = sc;
                    let a = super::input::scan_to_ascii(sc, false);
                    if a != 0 {
                        self.input.last_ascii = a;
                    }
                }
                if (sc as usize) < 128 && f >= start && f < start + dur {
                    self.input.keyboard[sc as usize] = true;
                } else if (sc as usize) < 128 && f == start + dur {
                    self.input.keyboard[sc as usize] = false;
                }
            }
        }
        if let Some(d) = &self.shot_dir {
            if self.frame % self.shot_every == 0 {
                let s = &self.surface;
                let src: &[u32] = if identity { &s.px } else { &self.present_buf };
                let mut rgba = Vec::with_capacity(src.len() * 4);
                for &p in src {
                    rgba.extend_from_slice(&[(p >> 16) as u8, (p >> 8) as u8, p as u8, 255]);
                }
                let _ = std::fs::create_dir_all(d);
                let _ = crate::dump::write_png(&d.join(format!("f{:06}.png", self.frame)), s.w, s.h, &rgba);
            }
        }
        if self.max_frames != 0 && self.frame >= self.max_frames {
            self.quit = true;
        }
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

    /// Presents the surface to the window applying the palette.
    pub fn present(&mut self) {
        let want_shot = self.shot_dir.is_some();
        let identity = if self.win.is_some() || want_shot { self.apply_palette() } else { true };
        self.debug_frame(identity);
        let Some((el, w)) = &mut self.win else {
            self.headless_tick();
            return;
        };
        el.pump_app_events(Some(Duration::ZERO), w);
        if w.closed {
            self.quit = true;
            return;
        }
        let s = &self.surface;
        let src: &[u32] = if identity { &s.px } else { &self.present_buf };
        let border = EGA_RGB[(self.border & 15) as usize];
        if let (Some(window), Some(surf)) = (&w.window, &mut w.surface) {
            let size = window.inner_size();
            if let (Some(ww), Some(wh)) = (NonZeroU32::new(size.width), NonZeroU32::new(size.height)) {
                if surf.resize(ww, wh).is_ok() {
                    if let Ok(mut buf) = surf.buffer_mut() {
                        let disp_w = s.logical_w();
                        let disp_h = if self.fullscreen_aspect { 240 } else { 200 };
                        scale_into(&mut buf, size.width as usize, size.height as usize, src, s.w, s.h, disp_w, disp_h, border);
                        let _ = buf.present();
                    }
                }
            }
        }
        self.sync_keys();
    }
}

/// Gamepad buttons by binding code (names in `input::PAD_BUTTON_NAMES`).
const PAD_BUTTONS: [gilrs::Button; 19] = {
    use gilrs::Button::*;
    [
        South, East, West, North, LeftTrigger, LeftTrigger2, RightTrigger, RightTrigger2, Select, Start, Mode, LeftThumb,
        RightThumb, DPadUp, DPadDown, DPadLeft, DPadRight, C, Z,
    ]
};

/// Gamepad axes by binding code (names in `input::PAD_AXIS_NAMES`).
const PAD_AXES: [gilrs::Axis; 8] = {
    use gilrs::Axis::*;
    [LeftStickX, LeftStickY, RightStickX, RightStickY, LeftZ, RightZ, DPadX, DPadY]
};

/// Whether a code is a HID axis (generic desktop page, not one of the
/// d-pad button usages 0x90..0x93), i.e. a d-pad "button" on it is half
/// of an axis (macOS codes; elsewhere detected by fractional values).
fn dpad_axis(c: u32) -> bool {
    cfg!(target_os = "macos") && c >> 16 == 1 && !(0x90..=0x93).contains(&(c & 0xffff))
}

/// Whether a bound gamepad input is held. A d-pad button also reads the
/// d-pad axes (some pads report the d-pad as a hat) or, for a d-pad that
/// is an axis pair mapped as buttons, the (x, y) direction `dpad` tracked
/// from its events.
fn pad_active(
    pad: &gilrs::Gamepad,
    b: super::input::PadInput,
    dpad: [i8; 2],
    dpad_codes: &std::collections::HashSet<u32>,
) -> bool {
    use super::input::PadInput;
    match b {
        PadInput::None => false,
        PadInput::Button(c) => {
            let Some(&btn) = PAD_BUTTONS.get(c as usize) else { return false };
            // the mapping may name only one button of the axis (the
            // Retrolink SNES pad reports up and down both as DPadUp)
            let pair = if c <= 14 { [gilrs::Button::DPadUp, gilrs::Button::DPadDown] } else { [gilrs::Button::DPadLeft, gilrs::Button::DPadRight] };
            if (13..=16).contains(&c)
                && pair.iter().any(|&p| pad.button_code(p).is_some_and(|code| dpad_codes.contains(&code.into_u32())))
            {
                return match c {
                    13 => dpad[1] < 0,
                    14 => dpad[1] > 0,
                    15 => dpad[0] < 0,
                    _ => dpad[0] > 0,
                };
            }
            let hat = match c {
                13 => pad.value(gilrs::Axis::DPadY) > 0.5,
                14 => pad.value(gilrs::Axis::DPadY) < -0.5,
                15 => pad.value(gilrs::Axis::DPadX) < -0.5,
                16 => pad.value(gilrs::Axis::DPadX) > 0.5,
                _ => false,
            };
            pad.is_pressed(btn) || hat
        }
        PadInput::Axis(c, pos) => {
            let Some(&axis) = PAD_AXES.get(c as usize) else { return false };
            let v = pad.value(axis);
            if pos {
                v > 0.5
            } else {
                v < -0.5
            }
        }
        PadInput::RawButton(c) => pad.state().buttons().any(|(code, d)| code.into_u32() == c && d.is_pressed()),
        PadInput::RawAxis(c, pos) => {
            let v = raw_axis(pad, c);
            if pos {
                v > 0.5
            } else {
                v < -0.5
            }
        }
    }
}

/// The value of an axis by its platform code (0 if the pad has none).
fn raw_axis(pad: &gilrs::Gamepad, c: u32) -> f32 {
    pad.state().axes().find(|(code, _)| code.into_u32() == c).map_or(0.0, |(_, d)| d.value())
}

/// Opens the gamepad library; with KEEN_PAD_DEBUG=1 lists the pads.
///
/// Without the default filters: gilrs' `axis_dpad_to_button` only turns a
/// d-pad axis into button presses at exactly +-1.0 and drops the axis
/// event otherwise, so pads whose d-pad is an axis pair 0..255 (e.g. the
/// Retrolink SNES controller, -0.992..1.0) lost up and left. `pad_active`
/// reads the d-pad axes itself; the thresholds replace the deadzone.
fn open_pads() -> Option<gilrs::Gilrs> {
    let g = match gilrs::GilrsBuilder::new().with_default_filters(false).build() {
        Ok(g) => g,
        Err(e) => {
            eprintln!("gamepads not available: {}", e);
            return None;
        }
    };
    if std::env::var("KEEN_PAD_DEBUG").is_ok_and(|v| v == "1") {
        for (id, pad) in g.gamepads() {
            eprintln!(
                "pad {} \"{}\" ({:?} mapping, uuid {:02x?})",
                usize::from(id),
                pad.name(),
                pad.mapping_source(),
                pad.uuid()
            );
        }
    }
    Some(g)
}

/// Scales the logical picture (`src`, any multiple of its logical size)
/// into the window buffer, keeping the display aspect of `disp_w` x
/// `disp_h` and filling the rest with the EGA border (overscan) color.
#[allow(clippy::too_many_arguments)]
fn scale_into(
    dst: &mut [u32],
    dw: usize,
    dh: usize,
    src: &[u32],
    sw: usize,
    sh: usize,
    disp_w: usize,
    disp_h: usize,
    border: u32,
) {
    let fit_w = dw * disp_h <= dh * disp_w;
    let (w, h) = if fit_w { (dw, (dw * disp_h / disp_w).max(1)) } else { ((dh * disp_w / disp_h).max(1), dh) };
    let (ox, oy) = ((dw - w) / 2, (dh - h) / 2);
    let border = border & 0xffffff;
    dst[..oy * dw].fill(border);
    dst[(oy + h) * dw..].fill(border);
    let xmap: Vec<usize> = (0..w).map(|x| x * sw / w).collect();
    for y in 0..h {
        let row = &mut dst[(oy + y) * dw..(oy + y + 1) * dw];
        row[..ox].fill(border);
        row[ox + w..].fill(border);
        let srow = &src[(y * sh / h) * sw..][..sw];
        for (d, &sx) in row[ox..ox + w].iter_mut().zip(&xmap) {
            *d = srow[sx] & 0xffffff;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::scale_into;

    #[test]
    fn letterbox_keeps_4_3() {
        // 320x200 shown as 4:3 in a 16:9 window: pillarbox left and right
        let src: Vec<u32> = (0..320 * 200).map(|i| 0xff000000 | (i as u32 % 320 + 1)).collect();
        let (dw, dh) = (1600, 900);
        let mut dst = vec![0u32; dw * dh];
        scale_into(&mut dst, dw, dh, &src, 320, 200, 320, 240, 0x123456);
        let w = 900 * 320 / 240;
        let ox = (dw - w) / 2;
        assert_eq!(dst[0], 0x123456);
        assert_eq!(dst[ox - 1], 0x123456);
        assert_eq!(dst[ox], 1);
        assert_eq!(dst[ox + w - 1], 320);
        assert_eq!(dst[ox + w], 0x123456);
        // a wide-and-short window letterboxes top and bottom
        let (dw, dh) = (640, 1000);
        let mut dst = vec![0u32; dw * dh];
        scale_into(&mut dst, dw, dh, &src, 320, 200, 320, 200, 0);
        assert_eq!(dst[((1000 - 400) / 2) * dw], 1);
        assert_eq!(dst[((1000 - 400) / 2 - 1) * dw], 0);
    }
}
