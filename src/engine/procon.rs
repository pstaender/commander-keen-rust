//! Nintendo Switch Pro Controller on macOS (not in the original).
//!
//! macOS (its GameController framework) switches the Pro Controller into
//! Nintendo's full report mode (input report 0x30), which does not match
//! the controller's HID descriptor: gilrs reads the report's timer byte as
//! buttons 1-8, so buttons seem pressed and released all the time. This
//! reads the controller with hidapi instead (opened non-exclusively, gilrs
//! keeps it too) and decodes the 0x30 report; `sys::sync_pad` leaves the
//! gilrs pad of a Pro Controller out while it is read here.
//!
//! Report 0x30: [1] timer, [2] battery, [3..6] buttons (right, shared,
//! left), [6..9] left stick, [9..12] right stick (two 12-bit values each).

/// USB vendor / product of the Pro Controller
pub const VENDOR: u16 = 0x057e;
pub const PRODUCT: u16 = 0x2009;

/// Button bits (report byte, mask) by binding code (the order of
/// `sys::PAD_BUTTONS`, positional like the SDL mapping: B is South, A
/// East, Y West, X North).
const BUTTON_BITS: [(usize, u8); 19] = [
    (3, 0x04), // A: B
    (3, 0x08), // B: A
    (3, 0x01), // X: Y
    (3, 0x02), // Y: X
    (5, 0x40), // LB: L
    (5, 0x80), // LT: ZL
    (3, 0x40), // RB: R
    (3, 0x80), // RT: ZR
    (4, 0x01), // SELECT: -
    (4, 0x02), // START: +
    (4, 0x10), // GUIDE: HOME
    (4, 0x08), // L3
    (4, 0x04), // R3
    (5, 0x02), // D-UP
    (5, 0x01), // D-DOWN
    (5, 0x08), // D-LEFT
    (5, 0x04), // D-RIGHT
    (4, 0x20), // C: CAPTURE
    (0, 0),    // Z: none
];

/// The decoded state of one controller.
#[derive(Clone, Copy, Default, PartialEq, Debug)]
pub struct ProState {
    /// by binding code (`input::PAD_BUTTON_NAMES`)
    pub buttons: [bool; 19],
    /// by binding code (`input::PAD_AXIS_NAMES`; Y positive upwards like
    /// gilrs)
    pub axes: [f32; 8],
}

impl ProState {
    fn decode(r: &[u8]) -> Option<ProState> {
        if r.len() < 12 || r[0] != 0x30 {
            return None;
        }
        let mut s = ProState::default();
        for (b, &(i, m)) in s.buttons.iter_mut().zip(&BUTTON_BITS) {
            *b = m != 0 && r[i] & m != 0;
        }
        // 12-bit values, centered around 2048 (about +-1500 at the rim)
        let stick = |o: usize| {
            let x = r[o] as i32 | (r[o + 1] as i32 & 0xf) << 8;
            let y = (r[o + 1] as i32) >> 4 | (r[o + 2] as i32) << 4;
            let f = |v: i32| ((v - 2048) as f32 / 1500.0).clamp(-1.0, 1.0);
            (f(x), f(y))
        };
        (s.axes[0], s.axes[1]) = stick(6);
        (s.axes[2], s.axes[3]) = stick(9);
        Some(s)
    }
}

pub struct ProPad {
    dev: hidapi::HidDevice,
    usb: bool,
    pub state: ProState,
    /// when it was opened (reports expected), whether the mode was set
    opened: std::time::Instant,
    init_sent: bool,
    counter: u8,
}

impl ProPad {
    /// Reads all pending reports. Returns the bindings pressed / moved in
    /// them (for binding an action), `None` when the controller is gone.
    pub fn poll(&mut self, debug: bool) -> Option<Vec<super::input::PadInput>> {
        use super::input::PadInput;
        let mut buf = [0u8; 64];
        let mut got = false;
        let mut events = Vec::new();
        loop {
            let n = self.dev.read_timeout(&mut buf, 0).ok()?;
            if n == 0 {
                break;
            }
            let Some(s) = ProState::decode(&buf[..n]) else { continue };
            got = true;
            let old = std::mem::replace(&mut self.state, s);
            for c in 0..s.buttons.len() {
                if s.buttons[c] && !old.buttons[c] {
                    events.push(PadInput::Button(c as u8));
                }
            }
            for c in 0..4 {
                let (v, p) = (s.axes[c], old.axes[c]);
                if v.abs() >= 0.7 && p.abs() < 0.7 {
                    events.push(PadInput::Axis(c as u8, v > 0.0));
                }
            }
            if debug && (s.buttons != old.buttons || s.axes.iter().zip(&old.axes).any(|(a, b)| (a - b).abs() > 0.1)) {
                let held: Vec<_> = (0..19).filter(|&c| s.buttons[c]).map(|c| super::input::PAD_BUTTON_NAMES[c]).collect();
                eprintln!("pro controller: {:?} sticks {:.2?}", held, &s.axes[..4]);
            }
        }
        // not in full report mode yet (no one else switched it): ask for it
        if !got && !self.init_sent && self.opened.elapsed() > std::time::Duration::from_millis(500) {
            self.init_sent = true;
            self.set_full_mode();
        }
        Some(events)
    }

    /// Switches the controller to input report 0x30 (as SDL's HIDAPI
    /// driver does: over USB the handshake first and subcommands wrapped
    /// in a 0x80 0x92 packet).
    fn set_full_mode(&mut self) {
        if self.usb {
            for cmd in [[0x80, 0x02], [0x80, 0x03], [0x80, 0x02], [0x80, 0x04]] {
                let _ = self.dev.write(&cmd);
                std::thread::sleep(std::time::Duration::from_millis(20));
            }
        }
        self.counter = self.counter.wrapping_add(1) & 0xf;
        // output report 0x01: counter, neutral rumble, subcommand 0x03
        // (set input report mode) 0x30
        let sub = [0x01, self.counter, 0x00, 0x01, 0x40, 0x40, 0x00, 0x01, 0x40, 0x40, 0x03, 0x30];
        let mut pkt = Vec::new();
        if self.usb {
            pkt.extend_from_slice(&[0x80, 0x92, 0x00, sub.len() as u8, 0, 0, 0, 0]);
        }
        pkt.extend_from_slice(&sub);
        let _ = self.dev.write(&pkt);
    }
}

/// The Pro Controllers read through hidapi.
#[derive(Default)]
pub struct ProPads {
    api: Option<hidapi::HidApi>,
    pub pads: Vec<ProPad>,
    last_scan: Option<std::time::Instant>,
}

impl ProPads {
    /// (Re)opens the Pro Controllers when gilrs sees `wanted` of them and
    /// a different number is open (at most once a second; hidapi has no
    /// path on an open device, so all are opened again).
    pub fn update(&mut self, wanted: usize, debug: bool) {
        if wanted == self.pads.len() || self.last_scan.is_some_and(|t| t.elapsed().as_secs() < 1) {
            return;
        }
        self.pads.clear();
        self.last_scan = Some(std::time::Instant::now());
        if self.api.is_none() {
            match hidapi::HidApi::new() {
                Ok(a) => {
                    // gilrs has the device open as well
                    a.set_open_exclusive(false);
                    self.api = Some(a);
                }
                Err(e) => {
                    eprintln!("Pro Controller: hidapi not available: {}", e);
                    return;
                }
            }
        }
        let Some(api) = &mut self.api else { return };
        let _ = api.refresh_devices();
        let mut seen = std::collections::HashSet::new();
        for d in api.device_list() {
            // one entry per top-level collection: the game pad one
            if d.vendor_id() != VENDOR || d.product_id() != PRODUCT || d.usage_page() != 1 {
                continue;
            }
            if !seen.insert(d.path().to_owned()) {
                continue;
            }
            if self.pads.len() >= wanted {
                break;
            }
            match d.open_device(api) {
                Ok(dev) => {
                    if debug {
                        eprintln!("pro controller opened ({:?})", d.bus_type());
                    }
                    self.pads.push(ProPad {
                        dev,
                        usb: matches!(d.bus_type(), hidapi::BusType::Usb),
                        state: ProState::default(),
                        opened: std::time::Instant::now(),
                        init_sent: false,
                        counter: 0,
                    });
                }
                Err(e) => eprintln!("Pro Controller: could not open: {}", e),
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decode_report() {
        // a recorded report: nothing pressed, sticks near the center
        let r = [0x30, 0x94, 0x91, 0x00, 0x80, 0x00, 0x87, 0x38, 0x77, 0x21, 0xa8, 0x7e];
        let s = ProState::decode(&r).unwrap();
        assert!(s.buttons.iter().all(|&b| !b));
        assert!(s.axes.iter().all(|a| a.abs() < 0.2));
        // B (South), d-pad up, + (START), left stick fully right
        let r = [0x30, 0, 0, 0x04, 0x02, 0x02, 0xff, 0x0f, 0x80, 0, 0x08, 0x80];
        let s = ProState::decode(&r).unwrap();
        assert!(s.buttons[0] && s.buttons[13] && s.buttons[9]);
        assert_eq!(s.buttons.iter().filter(|&&b| b).count(), 3);
        assert_eq!(s.axes[0], 1.0);
        assert!(s.axes[1].abs() < 0.01);
        assert_eq!(ProState::decode(&[0x3f; 12]), None);
    }
}
