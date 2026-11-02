//! ID_VW helpers: palette fades (VW_FadeOut / VW_FadeIn / VW_FadeUp/Down).

use super::Game;
use crate::engine::video::FADE_STEPS;

impl Game {
    fn set_palette_step(&mut self, step: usize) {
        let mut regs = FADE_STEPS[step];
        regs[16] = self.sys.border;
        self.sys.palette.regs = regs;
    }

    /// VW_FadeOut: four steps to black, 6 VBLs each.
    pub fn fade_out(&mut self) {
        for i in (0..=3).rev() {
            self.set_palette_step(i);
            self.sys.wait_vbl(6);
        }
        self.screenfaded = true;
    }

    /// VW_FadeIn
    pub fn fade_in(&mut self) {
        for i in 0..4 {
            self.set_palette_step(i);
            self.sys.wait_vbl(6);
        }
        self.screenfaded = false;
    }

    /// VW_FadeUp: to white
    pub fn fade_up(&mut self) {
        for i in 3..6 {
            self.set_palette_step(i);
            self.sys.wait_vbl(6);
        }
        self.screenfaded = true;
    }

    /// VW_FadeDown: from white
    pub fn fade_down(&mut self) {
        for i in (3..6).rev() {
            self.set_palette_step(i);
            self.sys.wait_vbl(6);
        }
        self.screenfaded = false;
    }

    pub fn set_default_palette(&mut self) {
        self.set_palette_step(3);
        self.screenfaded = false;
    }

    /// Draws a picture chunk (VWB_DrawPic), `chunk` is the EGAGRAPH chunk.
    pub fn draw_pic(&mut self, x: i32, y: i32, chunk: i32) {
        let i = chunk - super::consts::STARTPICS as i32;
        if i >= 0 {
            if let Some(t) = self.gfx.pics.get(i as usize) {
                self.sys.surface.blit(t, x, y);
            }
        }
    }

    /// Draws a masked picture chunk (VWB_DrawMPic).
    pub fn draw_mpic(&mut self, x: i32, y: i32, chunk: i32) {
        let i = chunk - super::consts::STARTPICM as i32;
        if i >= 0 {
            if let Some(t) = self.gfx.picms.get(i as usize) {
                self.sys.surface.blit(t, x, y);
            }
        }
    }
}
