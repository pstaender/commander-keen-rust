//! Keyboard state using PC scan codes, like ID_IN. The game logic and the
//! control panel work with scan codes, so key configuration stays compatible
//! with CONFIG.CK6.

use winit::keyboard::KeyCode;

pub type ScanCode = u8;

pub const SC_NONE: ScanCode = 0;
pub const SC_BAD: ScanCode = 0xff;
pub const SC_RETURN: ScanCode = 0x1c;
pub const SC_ENTER: ScanCode = SC_RETURN;
pub const SC_ESCAPE: ScanCode = 0x01;
pub const SC_SPACE: ScanCode = 0x39;
pub const SC_BACKSPACE: ScanCode = 0x0e;
pub const SC_TAB: ScanCode = 0x0f;
pub const SC_ALT: ScanCode = 0x38;
pub const SC_CONTROL: ScanCode = 0x1d;
pub const SC_CAPSLOCK: ScanCode = 0x3a;
pub const SC_LSHIFT: ScanCode = 0x2a;
pub const SC_RSHIFT: ScanCode = 0x36;
pub const SC_UPARROW: ScanCode = 0x48;
pub const SC_DOWNARROW: ScanCode = 0x50;
pub const SC_LEFTARROW: ScanCode = 0x4b;
pub const SC_RIGHTARROW: ScanCode = 0x4d;
pub const SC_INSERT: ScanCode = 0x52;
pub const SC_DELETE: ScanCode = 0x53;
pub const SC_HOME: ScanCode = 0x47;
pub const SC_END: ScanCode = 0x4f;
pub const SC_PGUP: ScanCode = 0x49;
pub const SC_PGDN: ScanCode = 0x51;
pub const SC_F1: ScanCode = 0x3b;
pub const SC_F2: ScanCode = 0x3c;
pub const SC_F3: ScanCode = 0x3d;
pub const SC_F4: ScanCode = 0x3e;
pub const SC_F5: ScanCode = 0x3f;
pub const SC_F6: ScanCode = 0x40;
pub const SC_F7: ScanCode = 0x41;
pub const SC_F8: ScanCode = 0x42;
pub const SC_F9: ScanCode = 0x43;
pub const SC_F10: ScanCode = 0x44;
pub const SC_F11: ScanCode = 0x57;
pub const SC_F12: ScanCode = 0x59;
pub const SC_A: ScanCode = 0x1e;
pub const SC_B: ScanCode = 0x30;
pub const SC_C: ScanCode = 0x2e;
pub const SC_D: ScanCode = 0x20;
pub const SC_E: ScanCode = 0x12;
pub const SC_F: ScanCode = 0x21;
pub const SC_G: ScanCode = 0x22;
pub const SC_H: ScanCode = 0x23;
pub const SC_I: ScanCode = 0x17;
pub const SC_J: ScanCode = 0x24;
pub const SC_K: ScanCode = 0x25;
pub const SC_L: ScanCode = 0x26;
pub const SC_M: ScanCode = 0x32;
pub const SC_N: ScanCode = 0x31;
pub const SC_O: ScanCode = 0x18;
pub const SC_P: ScanCode = 0x19;
pub const SC_Q: ScanCode = 0x10;
pub const SC_R: ScanCode = 0x13;
pub const SC_S: ScanCode = 0x1f;
pub const SC_T: ScanCode = 0x14;
pub const SC_U: ScanCode = 0x16;
pub const SC_V: ScanCode = 0x2f;
pub const SC_W: ScanCode = 0x11;
pub const SC_X: ScanCode = 0x2d;
pub const SC_Y: ScanCode = 0x15;
pub const SC_Z: ScanCode = 0x2c;

/// Maps a physical key to its PC/XT scan code (0 = not mapped).
pub fn key_to_scan(k: KeyCode) -> ScanCode {
    use KeyCode::*;
    match k {
        Escape => 0x01,
        Digit1 => 0x02,
        Digit2 => 0x03,
        Digit3 => 0x04,
        Digit4 => 0x05,
        Digit5 => 0x06,
        Digit6 => 0x07,
        Digit7 => 0x08,
        Digit8 => 0x09,
        Digit9 => 0x0a,
        Digit0 => 0x0b,
        Minus => 0x0c,
        Equal => 0x0d,
        Backspace => 0x0e,
        Tab => 0x0f,
        KeyQ => 0x10,
        KeyW => 0x11,
        KeyE => 0x12,
        KeyR => 0x13,
        KeyT => 0x14,
        KeyY => 0x15,
        KeyU => 0x16,
        KeyI => 0x17,
        KeyO => 0x18,
        KeyP => 0x19,
        BracketLeft => 0x1a,
        BracketRight => 0x1b,
        Enter | NumpadEnter => 0x1c,
        ControlLeft | ControlRight => 0x1d,
        KeyA => 0x1e,
        KeyS => 0x1f,
        KeyD => 0x20,
        KeyF => 0x21,
        KeyG => 0x22,
        KeyH => 0x23,
        KeyJ => 0x24,
        KeyK => 0x25,
        KeyL => 0x26,
        Semicolon => 0x27,
        Quote => 0x28,
        Backquote => 0x29,
        ShiftLeft => 0x2a,
        Backslash | IntlBackslash => 0x2b,
        KeyZ => 0x2c,
        KeyX => 0x2d,
        KeyC => 0x2e,
        KeyV => 0x2f,
        KeyB => 0x30,
        KeyN => 0x31,
        KeyM => 0x32,
        Comma => 0x33,
        Period => 0x34,
        Slash => 0x35,
        ShiftRight => 0x36,
        NumpadMultiply => 0x37,
        AltLeft | AltRight => 0x38,
        Space => 0x39,
        CapsLock => 0x3a,
        F1 => 0x3b,
        F2 => 0x3c,
        F3 => 0x3d,
        F4 => 0x3e,
        F5 => 0x3f,
        F6 => 0x40,
        F7 => 0x41,
        F8 => 0x42,
        F9 => 0x43,
        F10 => 0x44,
        NumLock => 0x45,
        ScrollLock => 0x46,
        Home | Numpad7 => 0x47,
        ArrowUp | Numpad8 => 0x48,
        PageUp | Numpad9 => 0x49,
        NumpadSubtract => 0x4a,
        ArrowLeft | Numpad4 => 0x4b,
        Numpad5 => 0x4c,
        ArrowRight | Numpad6 => 0x4d,
        NumpadAdd => 0x4e,
        End | Numpad1 => 0x4f,
        ArrowDown | Numpad2 => 0x50,
        PageDown | Numpad3 => 0x51,
        Insert | Numpad0 => 0x52,
        Delete | NumpadDecimal => 0x53,
        F11 => 0x57,
        F12 => 0x58,
        _ => 0,
    }
}

/// Scan code to ASCII tables (ID_IN ASCIINames / ShiftNames).
pub fn scan_to_ascii(sc: ScanCode, shift: bool) -> u8 {
    const NORMAL: &[u8; 0x3a] = b"\0\x1b1234567890-=\x08\tqwertyuiop[]\r\0asdfghjkl;'`\0\\zxcvbnm,./\0*\0 ";
    const SHIFTED: &[u8; 0x3a] = b"\0\x1b!@#$%^&*()_+\x08\tQWERTYUIOP{}\r\0ASDFGHJKL:\"~\0|ZXCVBNM<>?\0*\0 ";
    let t = if shift { SHIFTED } else { NORMAL };
    t.get(sc as usize).copied().unwrap_or(0)
}

/// Names for scan codes used by the control panel (ID_US ScanNames).
pub fn scan_name(sc: ScanCode) -> String {
    let named = match sc {
        0x01 => "Esc",
        0x0e => "BkSp",
        0x0f => "Tab",
        0x1d => "Ctrl",
        0x2a => "LShft",
        0x39 => "Space",
        0x3a => "CapsLk",
        0x3b => "F1",
        0x3c => "F2",
        0x3d => "F3",
        0x3e => "F4",
        0x3f => "F5",
        0x40 => "F6",
        0x41 => "F7",
        0x42 => "F8",
        0x43 => "F9",
        0x44 => "F10",
        0x46 => "ScrlLk",
        0x1c => "Enter",
        0x36 => "RShft",
        0x37 => "PrtSc",
        0x38 => "Alt",
        0x47 => "Home",
        0x49 => "PgUp",
        0x4f => "End",
        0x51 => "PgDn",
        0x52 => "Ins",
        0x53 => "Del",
        0x45 => "NumLk",
        0x48 => "Up",
        0x50 => "Down",
        0x4b => "Left",
        0x4d => "Right",
        _ => "",
    };
    if !named.is_empty() {
        return named.to_string();
    }
    let c = scan_to_ascii(sc, true);
    if c > 32 {
        (c as char).to_string()
    } else {
        "?".to_string()
    }
}

#[derive(Default, Clone, Copy, PartialEq, Eq, Debug)]
pub enum Direction {
    NorthWest,
    North,
    NorthEast,
    West,
    #[default]
    None,
    East,
    SouthWest,
    South,
    SouthEast,
}

impl Direction {
    /// Value as used by the original code (dir_North=0 ... dir_None=8).
    pub fn id(self) -> i32 {
        match self {
            Direction::North => 0,
            Direction::NorthEast => 1,
            Direction::East => 2,
            Direction::SouthEast => 3,
            Direction::South => 4,
            Direction::SouthWest => 5,
            Direction::West => 6,
            Direction::NorthWest => 7,
            Direction::None => 8,
        }
    }
}

/// ControlInfo as returned by IN_ReadControl.
#[derive(Default, Clone, Copy, Debug)]
pub struct ControlInfo {
    pub button0: bool,
    pub button1: bool,
    pub button2: bool,
    pub xaxis: i32,
    pub yaxis: i32,
    pub dir: Direction,
}

/// KeyboardDef: the configurable keys.
#[derive(Clone, Copy, Debug)]
pub struct KeyboardDef {
    pub button0: ScanCode,
    pub button1: ScanCode,
    pub upleft: ScanCode,
    pub up: ScanCode,
    pub upright: ScanCode,
    pub left: ScanCode,
    pub right: ScanCode,
    pub downleft: ScanCode,
    pub down: ScanCode,
    pub downright: ScanCode,
}

impl Default for KeyboardDef {
    fn default() -> Self {
        KeyboardDef {
            button0: SC_CONTROL,
            button1: SC_ALT,
            upleft: SC_HOME,
            up: SC_UPARROW,
            upright: SC_PGUP,
            left: SC_LEFTARROW,
            right: SC_RIGHTARROW,
            downleft: SC_END,
            down: SC_DOWNARROW,
            downright: SC_PGDN,
        }
    }
}

pub struct Input {
    /// gamepad state (merged into the keyboard controls)
    pub pad_x: i32,
    pub pad_y: i32,
    pub pad_jump: bool,
    pub pad_pogo: bool,
    pub pad_fire: bool,
    pub keyboard: [bool; 128],
    pub last_scan: ScanCode,
    /// `last_scan` came from keyboard auto repeat, not a new press
    pub last_repeat: bool,
    /// keys cleared by `clear_keys_down` while held: they read as up until
    /// released or repeated (the DOS keyboard interrupt only set a key down
    /// again on its next make code)
    pub masked: [bool; 128],
    pub last_ascii: u8,
    pub paused: bool,
    pub key_def: KeyboardDef,
}

impl Default for Input {
    fn default() -> Self {
        Input {
            pad_x: 0,
            pad_y: 0,
            pad_jump: false,
            pad_pogo: false,
            pad_fire: false,
            keyboard: [false; 128],
            last_scan: 0, last_repeat: false, masked: [false; 128], last_ascii: 0, paused: false, key_def: KeyboardDef::default() }
    }
}

impl Input {
    pub fn key(&self, sc: ScanCode) -> bool {
        self.keyboard.get(sc as usize).copied().unwrap_or(false)
    }

    pub fn clear_keys_down(&mut self) {
        for (m, k) in self.masked.iter_mut().zip(&self.keyboard) {
            *m |= *k;
        }
        self.keyboard = [false; 128];
        self.last_scan = SC_NONE;
        self.last_ascii = 0;
    }

    /// Reads the keyboard control scheme (IN_ReadControl, keyboard part).
    pub fn read_control(&self) -> ControlInfo {
        let d = &self.key_def;
        let k = |s| self.key(s);
        let mut mx = 0i32;
        let mut my = 0i32;
        if k(d.upleft) {
            mx = -1;
            my = -1;
        } else if k(d.upright) {
            mx = 1;
            my = -1;
        } else if k(d.downleft) {
            mx = -1;
            my = 1;
        } else if k(d.downright) {
            mx = 1;
            my = 1;
        }
        if k(d.up) {
            my = -1;
        } else if k(d.down) {
            my = 1;
        }
        if k(d.left) {
            mx = -1;
        } else if k(d.right) {
            mx = 1;
        }
        if self.pad_x != 0 {
            mx = self.pad_x;
        }
        if self.pad_y != 0 {
            my = self.pad_y;
        }
        let dir = match (mx, my) {
            (-1, -1) => Direction::NorthWest,
            (0, -1) => Direction::North,
            (1, -1) => Direction::NorthEast,
            (-1, 0) => Direction::West,
            (1, 0) => Direction::East,
            (-1, 1) => Direction::SouthWest,
            (0, 1) => Direction::South,
            (1, 1) => Direction::SouthEast,
            _ => Direction::None,
        };
        ControlInfo {
            button0: k(d.button0) || self.pad_jump,
            button1: k(d.button1) || self.pad_pogo,
            button2: false, xaxis: mx, yaxis: my, dir }
    }
}
