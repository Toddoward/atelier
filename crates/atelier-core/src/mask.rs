//! Single-channel 8-bit coverage mask in sparse 256² tiles — selections
//! (spec 0007). 0 = unselected, 255 = fully selected; absent tiles read 0.

use crate::tile::{TileCoord, TILE_SIZE};
use std::collections::BTreeMap;

const TILE_PX: usize = TILE_SIZE * TILE_SIZE;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CombineOp {
    Replace,
    /// Union: max(a, b)
    Add,
    /// a ∧ ¬b: min(a, 255 − b)
    Subtract,
    /// min(a, b)
    Intersect,
}

#[derive(Clone, PartialEq, Default)]
pub struct Mask {
    tiles: BTreeMap<TileCoord, Vec<u8>>,
}

impl std::fmt::Debug for Mask {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Mask({} tiles)", self.tiles.len())
    }
}

impl Mask {
    pub fn new() -> Self {
        Self::default()
    }

    fn split(x: i32, y: i32) -> (TileCoord, usize) {
        let tx = x.div_euclid(TILE_SIZE as i32);
        let ty = y.div_euclid(TILE_SIZE as i32);
        let ix = x.rem_euclid(TILE_SIZE as i32) as usize;
        let iy = y.rem_euclid(TILE_SIZE as i32) as usize;
        ((tx, ty), iy * TILE_SIZE + ix)
    }

    pub fn get(&self, x: i32, y: i32) -> u8 {
        let (coord, i) = Self::split(x, y);
        self.tiles.get(&coord).map_or(0, |t| t[i])
    }

    pub fn set(&mut self, x: i32, y: i32, v: u8) {
        let (coord, i) = Self::split(x, y);
        if v == 0 && !self.tiles.contains_key(&coord) {
            return;
        }
        self.tiles.entry(coord).or_insert_with(|| vec![0; TILE_PX])[i] = v;
    }

    pub fn is_empty(&self) -> bool {
        self.tiles.values().all(|t| t.iter().all(|&v| v == 0))
    }

    /// Content bounds in doc px (tile granularity), None when empty.
    pub fn bounds(&self) -> Option<[i32; 4]> {
        let mut it = self.tiles.keys();
        let &(tx, ty) = it.next()?;
        let (mut x0, mut y0, mut x1, mut y1) = (tx, ty, tx, ty);
        for &(tx, ty) in it {
            x0 = x0.min(tx);
            y0 = y0.min(ty);
            x1 = x1.max(tx);
            y1 = y1.max(ty);
        }
        let t = TILE_SIZE as i32;
        Some([x0 * t, y0 * t, (x1 + 1) * t, (y1 + 1) * t])
    }

    pub fn prune_blank(&mut self) {
        self.tiles.retain(|_, t| t.iter().any(|&v| v != 0));
    }

    /// Pixel-exact content bounds `[x0, y0, x1, y1)` (half-open) over set
    /// pixels, None when empty. Slower than `bounds` (per-pixel scan) — use for
    /// crop where tile granularity would be wrong.
    pub fn pixel_bounds(&self) -> Option<[i32; 4]> {
        let t = TILE_SIZE as i32;
        let (mut x0, mut y0, mut x1, mut y1) = (i32::MAX, i32::MAX, i32::MIN, i32::MIN);
        for (&(tx, ty), tile) in &self.tiles {
            for iy in 0..TILE_SIZE {
                for ix in 0..TILE_SIZE {
                    if tile[iy * TILE_SIZE + ix] != 0 {
                        let (px, py) = (tx * t + ix as i32, ty * t + iy as i32);
                        x0 = x0.min(px);
                        y0 = y0.min(py);
                        x1 = x1.max(px + 1);
                        y1 = y1.max(py + 1);
                    }
                }
            }
        }
        (x1 > x0).then_some([x0, y0, x1, y1])
    }

    /// Fully-selected mask covering the document rect `[0,0,size)`.
    pub fn select_all(size: [u32; 2]) -> Mask {
        let mut m = Mask::new();
        for y in 0..size[1] as i32 {
            for x in 0..size[0] as i32 {
                m.set(x, y, 255);
            }
        }
        m
    }

    /// Inverse coverage within the document rect (outside the canvas stays 0).
    pub fn inverted(&self, size: [u32; 2]) -> Mask {
        let mut m = Mask::new();
        for y in 0..size[1] as i32 {
            for x in 0..size[0] as i32 {
                let v = 255 - self.get(x, y);
                if v != 0 {
                    m.set(x, y, v);
                }
            }
        }
        m
    }

    /// Combine `other` into self (self = existing selection, other = new shape).
    pub fn combine(&mut self, other: &Mask, op: CombineOp) {
        match op {
            CombineOp::Replace => {
                *self = other.clone();
            }
            CombineOp::Add => {
                for (coord, src) in &other.tiles {
                    let dst = self.tiles.entry(*coord).or_insert_with(|| vec![0; TILE_PX]);
                    for (d, s) in dst.iter_mut().zip(src) {
                        *d = (*d).max(*s);
                    }
                }
            }
            CombineOp::Subtract => {
                for (coord, src) in &other.tiles {
                    if let Some(dst) = self.tiles.get_mut(coord) {
                        for (d, s) in dst.iter_mut().zip(src) {
                            *d = (*d).min(255 - *s);
                        }
                    }
                }
                self.prune_blank();
            }
            CombineOp::Intersect => {
                let keys: Vec<TileCoord> = self.tiles.keys().copied().collect();
                for coord in keys {
                    match other.tiles.get(&coord) {
                        Some(src) => {
                            let dst = self.tiles.get_mut(&coord).expect("key from self");
                            for (d, s) in dst.iter_mut().zip(src) {
                                *d = (*d).min(*s);
                            }
                        }
                        None => {
                            self.tiles.remove(&coord);
                        }
                    }
                }
                self.prune_blank();
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rect_mask(x0: i32, y0: i32, x1: i32, y1: i32) -> Mask {
        let mut m = Mask::new();
        for y in y0..y1 {
            for x in x0..x1 {
                m.set(x, y, 255);
            }
        }
        m
    }

    #[test]
    fn get_set_and_default_zero() {
        let mut m = Mask::new();
        assert_eq!(m.get(5, -3), 0);
        m.set(300, 300, 128);
        assert_eq!(m.get(300, 300), 128);
        assert!(!m.is_empty());
    }

    #[test]
    fn combine_semantics() {
        let a = rect_mask(0, 0, 10, 10);
        let b = rect_mask(5, 0, 15, 10);

        let mut add = a.clone();
        add.combine(&b, CombineOp::Add);
        assert_eq!(add.get(2, 2), 255);
        assert_eq!(add.get(12, 2), 255);

        let mut sub = a.clone();
        sub.combine(&b, CombineOp::Subtract);
        assert_eq!(sub.get(2, 2), 255, "left half kept");
        assert_eq!(sub.get(7, 2), 0, "overlap removed");
        assert_eq!(sub.get(12, 2), 0);

        let mut inter = a.clone();
        inter.combine(&b, CombineOp::Intersect);
        assert_eq!(inter.get(2, 2), 0);
        assert_eq!(inter.get(7, 2), 255, "only overlap");

        let mut rep = a.clone();
        rep.combine(&b, CombineOp::Replace);
        assert_eq!(rep.get(2, 2), 0);
        assert_eq!(rep.get(12, 2), 255);
    }

    #[test]
    fn select_all_and_inverted_respect_doc_rect() {
        let all = Mask::select_all([4, 3]);
        assert_eq!(all.get(0, 0), 255);
        assert_eq!(all.get(3, 2), 255);
        assert_eq!(all.get(4, 0), 0, "outside doc rect");

        let mut sel = Mask::new();
        sel.set(1, 1, 255);
        let inv = sel.inverted([4, 3]);
        assert_eq!(inv.get(1, 1), 0, "selected becomes unselected");
        assert_eq!(inv.get(0, 0), 255, "unselected becomes selected");
        assert_eq!(inv.get(10, 10), 0, "outside doc rect stays 0");
    }

    #[test]
    fn pixel_bounds_is_exact_not_tile_granular() {
        let m = rect_mask(10, 12, 25, 30);
        assert_eq!(m.pixel_bounds(), Some([10, 12, 25, 30]));
        assert_eq!(Mask::new().pixel_bounds(), None);
    }

    #[test]
    fn subtract_all_becomes_empty() {
        let a = rect_mask(0, 0, 8, 8);
        let mut s = a.clone();
        s.combine(&a, CombineOp::Subtract);
        assert!(s.is_empty());
    }
}
