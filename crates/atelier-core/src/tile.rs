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

    pub fn remove_tile(&mut self, coord: TileCoord) {
        self.tiles.remove(&coord);
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

    /// Extract the 256² region that lands on doc-tile `(tx, ty)` when this map
    /// is drawn at `offset` — i.e. source pixels `doc - offset`. None when the
    /// region touches no stored tile (fully transparent).
    pub fn extract_shifted(&self, tx: i32, ty: i32, offset: [i32; 2]) -> Option<Tile> {
        let t = TILE_SIZE as i32;
        let (dx0, dy0) = (tx * t, ty * t);
        // Which source tiles can contribute?
        let sx0 = (dx0 - offset[0]).div_euclid(t);
        let sy0 = (dy0 - offset[1]).div_euclid(t);
        let any = (sx0..=sx0 + 1)
            .flat_map(|x| (sy0..=sy0 + 1).map(move |y| (x, y)))
            .any(|c| self.tiles.contains_key(&c));
        if !any {
            return None;
        }
        let mut out = Tile::default();
        for y in 0..TILE_SIZE {
            for x in 0..TILE_SIZE {
                let px = self.pixel(dx0 + x as i32 - offset[0], dy0 + y as i32 - offset[1]);
                if px[3] != 0 {
                    out.set_pixel(x, y, px);
                }
            }
        }
        Some(out)
    }

    /// Pixel-exact content bounds `[x0, y0, x1, y1)` over non-transparent
    /// pixels, None when empty. Per-pixel scan — use when tile granularity is
    /// wrong (transform pivot, trim). Cf. [`TileMap::bounds`] (tile-granular).
    pub fn pixel_bounds(&self) -> Option<[i32; 4]> {
        let t = TILE_SIZE as i32;
        let (mut x0, mut y0, mut x1, mut y1) = (i32::MAX, i32::MAX, i32::MIN, i32::MIN);
        for (&(tx, ty), tile) in &self.tiles {
            for iy in 0..TILE_SIZE {
                for ix in 0..TILE_SIZE {
                    if tile.pixel(ix, iy)[3] != 0 {
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
    fn pixel_bounds_is_exact_vs_tile_granular() {
        let mut map = TileMap::new();
        map.fill_rect(10, 20, 35, 50, [1, 2, 3, 255]);
        assert_eq!(map.pixel_bounds(), Some([10, 20, 35, 50]), "exact content extent");
        assert_eq!(map.bounds(), Some([0, 0, 256, 256]), "tile-granular differs");
        assert_eq!(TileMap::new().pixel_bounds(), None);
    }

    #[test]
    fn from_bytes_validates_length() {
        assert!(Tile::from_bytes(vec![0; TILE_BYTES]).is_ok());
        assert_eq!(Tile::from_bytes(vec![0; 3]).unwrap_err(), TileError::BadLength(3));
    }
}
