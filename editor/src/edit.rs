//! Editing operations on maps: brushes, flood fill, resizing and undo.

use keen::assets::maps::Map;

/// A rectangle of cells; `None` leaves that plane unchanged when painting.
#[derive(Clone)]
pub struct Brush {
    pub w: usize,
    pub h: usize,
    pub cells: Vec<[Option<u16>; 3]>,
}

impl Brush {
    pub fn single(plane: usize, v: u16) -> Brush {
        let mut c = [None; 3];
        c[plane] = Some(v);
        Brush { w: 1, h: 1, cells: vec![c] }
    }

    /// A tile block from a tile set: tiles `first` .. in rows of `cols`.
    pub fn block(plane: usize, first: usize, w: usize, h: usize, cols: usize) -> Brush {
        let mut cells = Vec::with_capacity(w * h);
        for y in 0..h {
            for x in 0..w {
                let mut c = [None; 3];
                c[plane] = Some((first + y * cols + x) as u16);
                cells.push(c);
            }
        }
        Brush { w, h, cells }
    }

    /// Copies a rectangle of the map, taking the planes in `planes`.
    pub fn from_map(m: &Map, r: TileRect, planes: [bool; 3]) -> Brush {
        let mut cells = Vec::with_capacity(r.w() * r.h());
        for y in r.y0..=r.y1 {
            for x in r.x0..=r.x1 {
                let mut c = [None; 3];
                for p in 0..3 {
                    if planes[p] {
                        c[p] = Some(m.planes[p][y * m.width + x]);
                    }
                }
                cells.push(c);
            }
        }
        Brush { w: r.w(), h: r.h(), cells }
    }

    pub fn cell(&self, x: usize, y: usize) -> [Option<u16>; 3] {
        self.cells[(y % self.h) * self.w + x % self.w]
    }

    /// The single value of `plane`, if the brush is one cell.
    pub fn value(&self, plane: usize) -> Option<u16> {
        (self.w == 1 && self.h == 1).then(|| self.cells[0][plane]).flatten()
    }
}

/// Inclusive tile rectangle.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct TileRect {
    pub x0: usize,
    pub y0: usize,
    pub x1: usize,
    pub y1: usize,
}

impl TileRect {
    pub fn span(a: (usize, usize), b: (usize, usize)) -> TileRect {
        TileRect { x0: a.0.min(b.0), y0: a.1.min(b.1), x1: a.0.max(b.0), y1: a.1.max(b.1) }
    }
    pub fn w(&self) -> usize {
        self.x1 - self.x0 + 1
    }
    pub fn h(&self) -> usize {
        self.y1 - self.y0 + 1
    }
}

/// Paints the brush with its top left corner at (x, y).
pub fn stamp(m: &mut Map, b: &Brush, x: usize, y: usize) {
    for by in 0..b.h {
        for bx in 0..b.w {
            put(m, x + bx, y + by, b.cell(bx, by));
        }
    }
}

/// Fills the rectangle with the brush repeated (aligned to the rectangle).
pub fn fill_rect(m: &mut Map, b: &Brush, r: TileRect) {
    for y in r.y0..=r.y1 {
        for x in r.x0..=r.x1 {
            put(m, x, y, b.cell(x - r.x0, y - r.y0));
        }
    }
}

fn put(m: &mut Map, x: usize, y: usize, c: [Option<u16>; 3]) {
    if x >= m.width || y >= m.height {
        return;
    }
    for (p, v) in c.iter().enumerate() {
        if let Some(v) = v {
            m.planes[p][y * m.width + x] = *v;
        }
    }
}

/// Sets the planes in `planes` to 0 inside the rectangle.
pub fn clear_rect(m: &mut Map, r: TileRect, planes: [bool; 3]) {
    let c = [planes[0].then_some(0), planes[1].then_some(0), planes[2].then_some(0)];
    for y in r.y0..=r.y1 {
        for x in r.x0..=r.x1 {
            put(m, x, y, c);
        }
    }
}

/// Flood fills the 4-connected area of equal values in `plane`, painting
/// the brush pattern (aligned to the map).
pub fn flood_fill(m: &mut Map, plane: usize, b: &Brush, x: usize, y: usize) {
    let (w, h) = (m.width, m.height);
    let old = m.planes[plane][y * w + x];
    let mut seen = vec![false; w * h];
    let mut stack = vec![(x, y)];
    while let Some((x, y)) = stack.pop() {
        let i = y * w + x;
        if seen[i] || m.planes[plane][i] != old {
            continue;
        }
        seen[i] = true;
        if x > 0 {
            stack.push((x - 1, y));
        }
        if x + 1 < w {
            stack.push((x + 1, y));
        }
        if y > 0 {
            stack.push((x, y - 1));
        }
        if y + 1 < h {
            stack.push((x, y + 1));
        }
    }
    for (i, s) in seen.iter().enumerate() {
        if *s {
            put(m, i % w, i / w, b.cell(i % w, i / w));
        }
    }
}

/// Resizes the map; the old contents are placed at (dx, dy).
pub fn resize(m: &Map, w: usize, h: usize, dx: isize, dy: isize) -> Map {
    let mut n = Map::new(w, h, &m.name);
    for y in 0..m.height {
        for x in 0..m.width {
            let (nx, ny) = (x as isize + dx, y as isize + dy);
            if nx < 0 || ny < 0 || nx as usize >= w || ny as usize >= h {
                continue;
            }
            for p in 0..3 {
                n.planes[p][ny as usize * w + nx as usize] = m.planes[p][y * m.width + x];
            }
        }
    }
    n
}

/// Undo/redo of whole map slots (maps are small, snapshots are simple).
#[derive(Default)]
pub struct History {
    undo: Vec<(usize, Option<Map>)>,
    redo: Vec<(usize, Option<Map>)>,
}

const MAX_UNDO: usize = 200;

impl History {
    /// Records the state of slot `n` before a change.
    pub fn push(&mut self, n: usize, before: Option<Map>) {
        self.undo.push((n, before));
        if self.undo.len() > MAX_UNDO {
            self.undo.remove(0);
        }
        self.redo.clear();
    }

    /// Undoes the last change; returns the slot that changed.
    pub fn undo(&mut self, maps: &mut [Option<Map>]) -> Option<usize> {
        let (n, m) = self.undo.pop()?;
        self.redo.push((n, std::mem::replace(&mut maps[n], m)));
        Some(n)
    }

    pub fn redo(&mut self, maps: &mut [Option<Map>]) -> Option<usize> {
        let (n, m) = self.redo.pop()?;
        self.undo.push((n, std::mem::replace(&mut maps[n], m)));
        Some(n)
    }

    pub fn can_undo(&self) -> bool {
        !self.undo.is_empty()
    }
    pub fn can_redo(&self) -> bool {
        !self.redo.is_empty()
    }
    pub fn clear(&mut self) {
        self.undo.clear();
        self.redo.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fill_and_undo() {
        let mut maps = vec![Some(Map::new(10, 8, "t"))];
        let mut h = History::default();
        h.push(0, maps[0].clone());
        let m = maps[0].as_mut().unwrap();
        fill_rect(m, &Brush::single(1, 5), TileRect::span((2, 2), (4, 3)));
        flood_fill(m, 1, &Brush::single(1, 7), 0, 0);
        assert_eq!(m.planes[1][2 * 10 + 3], 5);
        assert_eq!(m.planes[1][0], 7);
        assert_eq!(m.planes[0][0], 0);
        h.undo(&mut maps);
        assert!(maps[0].as_ref().unwrap().planes[1].iter().all(|&v| v == 0));
        h.redo(&mut maps);
        assert_eq!(maps[0].as_ref().unwrap().planes[1][0], 7);
        let r = resize(maps[0].as_ref().unwrap(), 12, 9, 1, 1);
        assert_eq!(r.planes[1][3 * 12 + 4], 5);
    }
}
