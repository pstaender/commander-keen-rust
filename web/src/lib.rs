//! Commander Keen 4, 5 and 6 in the web browser.
//!
//! `wasm-pack build --target web` (see web/build.sh) makes `pkg/`, loaded by
//! the web worker in `www/worker.js`. The page (`www/main.js`) shares
//! memory with the worker for the picture, the keyboard, the gamepads and
//! the sound (see `keen::engine::sys`, src/engine/sys_web.rs) and keeps the
//! data files, saved games and config.txt (IndexedDB).
#![cfg(target_arch = "wasm32")]

use js_sys::{Array, Function, Object, Reflect, SharedArrayBuffer, Uint8Array};
use keen::assets::{Episode, GameData};
use keen::engine::gfx::Gfx;
use keen::engine::sound::SoundManager;
use keen::engine::sys::{self, Sys};
use keen::options::Options;
use std::cell::RefCell;
use std::path::Path;
use wasm_bindgen::prelude::*;

/// where the page's files are put (the data directory; saves/ and
/// config.txt go there too)
const DATA_DIR: &str = "/data";

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = console, js_name = error)]
    fn console_error(s: &str);
}

thread_local! {
    /// the page's callback: host(kind, path, data)
    static HOST: RefCell<Option<Function>> = const { RefCell::new(None) };
}

fn host(kind: &str, path: &str, data: &JsValue) {
    HOST.with(|h| {
        if let Some(f) = &*h.borrow() {
            let _ = f.call3(&JsValue::NULL, &kind.into(), &path.into(), data);
        }
    });
}

fn set(o: &Object, k: &str, v: impl Into<JsValue>) {
    let _ = Reflect::set(o, &k.into(), &v.into());
}

/// The sizes and indexes of the shared buffers, and the scan code of each
/// key `code` (the page turns key events into scan codes).
#[wasm_bindgen]
pub fn layout() -> Object {
    let o = Object::new();
    for (k, v) in [
        ("CONTROL_BYTES", sys::CONTROL_BYTES),
        ("FRAME_BYTES", sys::FRAME_BYTES),
        ("AUDIO_BYTES", sys::AUDIO_BYTES),
        ("C_AREA_W", sys::C_AREA_W),
        ("C_AREA_H", sys::C_AREA_H),
        ("C_KEY_WRITE", sys::C_KEY_WRITE),
        ("C_FULLSCREEN", sys::C_FULLSCREEN),
        ("C_PAUSE", sys::C_PAUSE),
        ("C_KEYS", sys::C_KEYS),
        ("KEY_RING", sys::KEY_RING),
        ("C_PADS", sys::C_PADS),
        ("PAD_STRIDE", sys::PAD_STRIDE),
        ("PAD_SLOTS", sys::PAD_SLOTS as u32),
        ("PAD_AXES_AT", sys::PAD_AXES_AT),
        ("PAD_MAX_AXES", sys::PAD_MAX_AXES as u32),
        ("F_SEQ", sys::F_SEQ),
        ("F_W", sys::F_W),
        ("F_H", sys::F_H),
        ("F_DISP_W", sys::F_DISP_W),
        ("F_DISP_H", sys::F_DISP_H),
        ("F_BORDER", sys::F_BORDER),
        ("F_PIXELS", sys::F_PIXELS),
        ("A_WRITE", sys::A_WRITE),
        ("A_READ", sys::A_READ),
        ("A_SAMPLES", sys::A_SAMPLES),
        ("A_FRAMES", sys::A_FRAMES),
        ("A_AHEAD", sys::A_AHEAD as u32),
        ("SAMPLE_RATE", keen::engine::sound::SAMPLE_RATE),
    ] {
        set(&o, k, v);
    }
    let keys = Object::new();
    for &(code, sc) in keen::engine::input::KEY_CODES {
        set(&keys, code, sc);
    }
    set(&o, "keys", keys);
    o
}

/// Runs the game (until it quits; the worker blocks meanwhile).
///
/// `files`: [name, Uint8Array] pairs relative to the data directory
/// (`saves/...` and `config.txt` included); `args`: the options, like the
/// command line (`--classic`, `--zoom 50`, `--level 3` ...); `host_fn`:
/// called as host(kind, path, data) with kind "write" (data: Uint8Array),
/// "remove", "quit" or "error" (path: the message).
#[wasm_bindgen]
pub fn start(ctrl: SharedArrayBuffer, frame: SharedArrayBuffer, audio: SharedArrayBuffer, files: Array, args: Array, host_fn: Function) {
    HOST.with(|h| *h.borrow_mut() = Some(host_fn));
    std::panic::set_hook(Box::new(|info| {
        let msg = info.to_string();
        console_error(&msg);
        host("error", &msg, &JsValue::NULL);
    }));
    for pair in files.iter() {
        let pair: Array = pair.into();
        let (Some(name), data) = (pair.get(0).as_string(), Uint8Array::new(&pair.get(1))) else { continue };
        keen::vfs::insert(&Path::new(DATA_DIR).join(name), data.to_vec());
    }
    keen::vfs::set_write_hook(Box::new(|path, data| {
        let rel = path.strip_prefix(DATA_DIR).unwrap_or(path).trim_start_matches('/');
        match data {
            Some(d) => host("write", rel, &Uint8Array::from(d).into()),
            None => host("remove", rel, &JsValue::NULL),
        }
    }));
    sys::init_shared(&ctrl, &frame, &audio);

    let args: Vec<String> = args.iter().filter_map(|v| v.as_string()).collect();
    let (opts, sound, aspect) = parse_args(&args);
    let data = match GameData::load(Path::new(DATA_DIR)) {
        Ok(d) => d,
        Err(e) => {
            host("error", &e, &JsValue::NULL);
            return;
        }
    };
    let gfx = Gfx::new(&data.gfx, 1, None);
    let mut sd = SoundManager::new(&data.audio, sound, None, None);
    sd.set_episode(match data.episode {
        Episode::K4 => 4,
        Episode::K5 => 5,
        Episode::K6 => 6,
    });
    let sys = Sys::new(1, 1, false, aspect, false);
    match data.episode {
        Episode::K4 => keen::ck4::run(sys, data, gfx, sd, opts),
        Episode::K5 => keen::ck5::run(sys, data, gfx, sd, opts),
        Episode::K6 => keen::ck6::run(sys, data, gfx, sd, opts),
    }
    host("quit", "", &JsValue::NULL);
}

/// The options of the command line that make sense in the browser (from
/// the page's URL, e.g. `?classic&zoom=50`): returns the options, sound
/// on, 4:3 aspect.
fn parse_args(args: &[String]) -> (Options, bool, bool) {
    let mut opts = Options { zoom: 75, zoom_config: 75, ..Options::default() };
    let (mut sound, mut aspect) = (true, true);
    let mut it = args.iter();
    while let Some(a) = it.next() {
        match a.as_str() {
            "--classic" => opts.classic = true,
            "--classic-view" => opts.classic_view = true,
            "--no-auto-save" => opts.no_auto_save = true,
            "--nosound" => sound = false,
            "--no-aspect" => aspect = false,
            "--debug" | "--debug-shortcuts" => opts.debug = true,
            "--zoom" => {
                if let Some(z) = it.next().and_then(|v| v.trim_end_matches('%').parse::<u8>().ok()) {
                    if keen::options::ZOOMS.contains(&z) {
                        opts.zoom = z;
                        opts.zoom_given = true;
                    }
                }
            }
            "--level" => opts.start_level = it.next().and_then(|v| v.parse().ok()),
            "--difficulty" => {
                opts.start_difficulty = match it.next().map(|v| v.to_ascii_lowercase()).as_deref() {
                    Some("easy" | "1") => Some(1),
                    Some("normal" | "2") => Some(2),
                    Some("hard" | "3") => Some(3),
                    _ => None,
                }
            }
            "--demo" => opts.play_demo = it.next().and_then(|v| v.parse().ok()),
            _ => {}
        }
    }
    if opts.classic {
        opts.classic_view = true;
        opts.no_auto_save = true;
    }
    opts.cheatmode = !opts.classic;
    (opts, sound, aspect)
}
