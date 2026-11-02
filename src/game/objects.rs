//! Object list (CK_PLAY InitObjArray / GetNewObj / RemoveObj) and US_RndT.
//! The original keeps a doubly linked list inside a fixed array; we keep
//! the very same semantics (free list order, append at end) so that update
//! order and therefore demo playback stay identical.

use super::defs::*;
use super::Game;

pub const RNDTABLE: [u8; 256] = [
    0, 8, 109, 220, 222, 241, 149, 107, 75, 248, 254, 140, 16, 66, 74, 21,
    211, 47, 80, 242, 154, 27, 205, 128, 161, 89, 77, 36, 95, 110, 85, 48,
    212, 140, 211, 249, 22, 79, 200, 50, 28, 188, 52, 140, 202, 120, 68, 145,
    62, 70, 184, 190, 91, 197, 152, 224, 149, 104, 25, 178, 252, 182, 202, 182,
    141, 197, 4, 81, 181, 242, 145, 42, 39, 227, 156, 198, 225, 193, 219, 93,
    122, 175, 249, 0, 175, 143, 70, 239, 46, 246, 163, 53, 163, 109, 168, 135,
    2, 235, 25, 92, 20, 145, 138, 77, 69, 166, 78, 176, 173, 212, 166, 113,
    94, 161, 41, 50, 239, 49, 111, 164, 70, 60, 2, 37, 171, 75, 136, 156,
    11, 56, 42, 146, 138, 229, 73, 146, 77, 61, 98, 196, 135, 106, 63, 197,
    195, 86, 96, 203, 113, 101, 170, 247, 181, 113, 80, 250, 108, 7, 255, 237,
    129, 226, 79, 107, 112, 166, 103, 241, 24, 223, 239, 120, 198, 58, 60, 82,
    128, 3, 184, 66, 143, 224, 145, 224, 81, 206, 163, 45, 63, 90, 168, 114,
    59, 33, 159, 95, 28, 139, 123, 98, 125, 196, 15, 70, 194, 253, 54, 14,
    109, 226, 71, 17, 161, 93, 186, 87, 244, 138, 20, 52, 123, 251, 26, 36,
    17, 46, 52, 231, 232, 76, 31, 221, 84, 37, 216, 165, 212, 106, 197, 242,
    98, 43, 39, 175, 254, 145, 190, 84, 118, 222, 187, 136, 120, 163, 236, 249,
];

impl Game {
    pub fn init_obj_array(&mut self) {
        self.objs = vec![Obj::default(); MAXACTORS + 1];
        self.obj_free = vec![false; MAXACTORS + 1];
        // free list linked through prev
        for i in 0..MAXACTORS {
            self.objs[i].prev = if i + 1 < MAXACTORS { Some(i + 1) } else { None };
            self.objs[i].next = None;
            self.obj_free[i] = true;
        }
        self.objfreelist = Some(0);
        self.lastobj = None;
        self.objectcount = 0;
        self.player = self.get_new_obj(false);
        self.scoreobj = self.get_new_obj(false);
    }

    /// Returns the index of a fresh object appended to the list.
    pub fn get_new_obj(&mut self, usedummy: bool) -> usize {
        let Some(n) = self.objfreelist else {
            if usedummy {
                self.objs[DUMMYOBJ] = Obj::default();
                self.newobj = DUMMYOBJ;
                return DUMMYOBJ;
            }
            self.quit("GetNewObj: No free spots in objarray!");
            self.objs[DUMMYOBJ] = Obj::default();
            self.newobj = DUMMYOBJ;
            return DUMMYOBJ;
        };
        self.objfreelist = self.objs[n].prev;
        self.objs[n] = Obj::default();
        self.obj_free[n] = false;
        if let Some(l) = self.lastobj {
            self.objs[l].next = Some(n);
        }
        self.objs[n].prev = self.lastobj;
        self.objs[n].active = AC_YES;
        self.objs[n].needtoclip = CL_MIDCLIP;
        self.lastobj = Some(n);
        self.objectcount += 1;
        self.newobj = n;
        n
    }

    pub fn remove_obj(&mut self, gone: usize) {
        if gone == DUMMYOBJ || self.obj_free[gone] {
            return;
        }
        if gone == self.player {
            self.quit("RemoveObj: Tried to remove the player!");
            return;
        }
        // RF_RemoveSprite(&gone->sprite), and the stars of stunned objects
        self.remove_sprite(gone);
        self.objs[gone].state = None;
        let next = self.objs[gone].next;
        let prev = self.objs[gone].prev;
        if Some(gone) == self.lastobj {
            self.lastobj = prev;
        } else if let Some(nx) = next {
            self.objs[nx].prev = prev;
        }
        if let Some(pv) = prev {
            self.objs[pv].next = next;
        }
        self.objs[gone].prev = self.objfreelist;
        self.objfreelist = Some(gone);
        self.obj_free[gone] = true;
        self.objectcount -= 1;
    }

    pub fn init_rnd(&mut self, randomize: bool) {
        self.rndindex = if randomize {
            (std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.subsec_millis() / 10).unwrap_or(0)
                & 0xff) as usize
        } else {
            0
        };
    }

    /// US_RndT
    pub fn rnd(&mut self) -> u8 {
        self.rndindex = (self.rndindex + 1) & 0xff;
        RNDTABLE[self.rndindex]
    }
}
