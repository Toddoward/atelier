//! Sparse 256² RGBA8 tile storage (RAS-1). Pure data — compositing and brush
//! operations live in `atelier-raster`; GPU upload in `atelier-gpu`.
//!
//! Pixels are straight (unassociated) alpha, sRGB-component space, document
//! coordinates. Absent tiles are fully transparent. Tile bytes are skipped by
//! serde (the `.atl` container stores them as binary parts, see
//! docs/FORMAT-ATL.md) but participate in `PartialEq`.

use std::collections::BTreeMap;

pub const TILE_SIZE: usize = 256;
const TILE_BYTES: usize = TILE_SIZE * TILE_SIZE * 4;

/// Tile coordinate in units of whole tiles (doc pixel x = tx * 256 + in-tile x).
pub type TileCoord = (i32, i32);

#[derive(Clone, PartialEq)]
pub struct Tile {
    data: Vec<u8>,
}

impl std::fmt::Debug for Tile {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Tile({} bytes)", self.data.len())
    }
}

impl Default for Tile {
    fn default() -> Self {
        Self { data: vec![0; TILE_BYTES] }
    }
}

impl Tile {
    /// Wrap raw RGBA8 bytes; must be exactly 256·256·4 bytes.
    pub fn from_bytes(data: Vec<u8>) -> Result<Self, TileError> {
        if data.len() != TILE_BYTES {
            return Err(TileError::BadLength(data.len()));
        }
        Ok(Self { data })
    }

    pub fn bytes(&self) -> &[u8] {
        &self.data
    }

    #[inline]
    pub fn pixel(&self, x: usize, y: usize) -> [u8; 4] {
        let i = (y * TILE_SIZE + x) * 4;
        [self.data[i], self.data[i + 1], self.data[i + 2], self.data[i + 3]]
    }

    #[inline]
    pub fn set_pixel(&mut self, x: usize, y: usize, rgba: [u8; 4]) {
        let i = (y * TILE_SIZE + x) * 4;
        self.data[i..i + 4].copy_from_slice(&rgba);
    }

    pub fn is_blank(&self) -> bool {
        // Alpha channel all zero ⇒ tile contributes nothing.
        self.data.chunks_exact(4).all(|px| px[3] == 0)
    }
}

#[derive(Debug, thiserror::Error, PartialEq)]
pub enum TileError {
    #[error("tile byte length {0} != 256*256*4")]
    BadLength(usize),
}

#[derive(Clone, PartialEq, Debug, Default)]
pub struct TileMap {
    tiles: BTreeMap<TileCoord, Tile>,
}

impl TileMap {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn is_empty(&self) -> bool {
        self.tiles.is_empty()
    }

    pub fn tile_count(&self) -> usize {
        self.tiles.len()
    }

    pub fn tiles(&self) -> impl Iterator<Item = (&TileCoord, &Tile)> {
        self.tiles.iter()
    }

    pub fn tile_at(&self, coord: TileCoord) -> Option<&Tile> {
        self.tiles.get(&coord)
    }

    /// Used by the `.atl` loader to reattach deserialized tile parts.
    pub fn insert_tile(&mut self, coord: TileCoord, tile: Tile) {
        self.tiles.insert(coord, tile);
    }

    fn split(doc_x: i32, doc_y: i32) -> (TileCoord, usize, usize) {
        let tx = doc_x.div_euclid(TILE_SIZE as i32);
        let ty = doc_y.div_euclid(TILE_SIZE as i32);
        let ix = doc_x.rem_euclid(TILE_SIZE as i32) as usize;
        let iy = doc_y.rem_euclid(TILE_SIZE as i32) as usize;
        ((tx, ty), ix, iy)
    }

    /// Transparent black where no tile exists.
    pub fn pixel(&self, doc_x: i32, doc_y: i32) -> [u8; 4] {
        let (coord, ix, iy) = Self::split(doc_x, doc_y);
        self.tiles.get(&coord).map(|t| t.pixel(ix, iy)).unwrap_or([0; 4])
    }

    /// Creates the touched tile on demand.
    pub fn set_pixel(&mut self, doc_x: i32, doc_y: i32, rgba: [u8; 4]) {
        let (coord, ix, iy) = Self::split(doc_x, doc_y);
        self.tiles.entry(coord).or_default().set_pixel(ix, iy, rgba);
    }

    /// Fill an axis-aligned rect (doc coords, half-open) with one color.
    pub fn fill_rect(&mut self, x0: i32, y0: i32, x1: i32, y1: i32, rgba: [u8; 4]) {
        for y in y0..y1 {
            for x in x0..x1 {
                self.set_pixel(x, y, rgba);
            }
        }
    }

    /// Drop tiles that are fully transparent (sparseness maintenance).
    pub fn prune_blank(&mut self) {
        self.tiles.retain(|_, t| !t.is_blank());
    }

    /// Coarse content bounds in doc pixels `[x0, y0, x1, y1)` — tile
    /// granularity (selection outlines, invalidation), None when empty.
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
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn absent_tiles_read_transparent() {
        let map = TileMap::new();
        assert_eq!(map.pixel(0, 0), [0; 4]);
        assert_eq!(map.pixel(-1000, 12345), [0; 4]);
        assert!(map.is_empty());
    }

    #[test]
    fn set_then_get_round_trips_across_tile_borders() {
        let mut map = TileMap::new();
        for &(x, y) in &[(0, 0), (255, 255), (256, 256), (-1, -1), (511, 0)] {
            map.set_pixel(x, y, [1, 2, 3, 4]);
            assert_eq!(map.pixel(x, y), [1, 2, 3, 4], "at ({x},{y})");
        }
        // (-1,-1) lands in tile (-1,-1), not (0,0).
        assert!(map.tile_at((-1, -1)).is_some());
        assert_eq!(map.tile_count(), 4); // (0,0) holds both (0,0) and (255,255)
    }

    #[test]
    fn fill_rect_spans_tiles_and_prune_drops_blank() {
        let mut map = TileMap::new();
        map.fill_rect(250, 250, 260, 260, [9, 9, 9, 255]);
        assert_eq!(map.tile_count(), 4); // rect crosses the 256 boundary both axes
        assert_eq!(map.pixel(259, 259), [9, 9, 9, 255]);
        assert_eq!(map.pixel(260, 260), [0; 4]);

        map.fill_rect(250, 250, 260, 260, [0, 0, 0, 0]);
        map.prune_blank();
        assert!(map.is_empty());
    }

    #[test]
    fn from_bytes_validates_length() {
        assert!(Tile::from_bytes(vec![0; TILE_BYTES]).is_ok());
        assert_eq!(Tile::from_bytes(vec![0; 3]).unwrap_err(), TileError::BadLength(3));
    }
}
