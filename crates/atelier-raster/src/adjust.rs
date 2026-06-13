//! Tile-level application of adjustments (spec 0008). The `Adjustment` value
//! type and the per-pixel math live in `atelier-core::adjust`; this module
//! applies them to tiles with an optional selection clip.

pub use atelier_core::adjust::Adjustment;
use atelier_core::{Mask, Tile, TileMap, TILE_SIZE};

/// Apply `adj` to one tile at doc-tile `(tx,ty)`, layer drawn at `offset`,
/// optionally clipped by `mask` (selection coverage). `coverage/255` blends the
/// adjusted color toward the original so partial selections apply partially.
pub fn apply_tile(
    tile: &mut Tile,
    adj: Adjustment,
    tx: i32,
    ty: i32,
    offset: [i32; 2],
    mask: Option<&Mask>,
) {
    let t = TILE_SIZE as i32;
    for y in 0..TILE_SIZE {
        for x in 0..TILE_SIZE {
            let orig = tile.pixel(x, y);
            if orig[3] == 0 {
                continue;
            }
            let cov = match mask {
                None => 255u8,
                Some(m) => {
                    let dx = tx * t + x as i32 + offset[0];
                    let dy = ty * t + y as i32 + offset[1];
                    m.get(dx, dy)
                }
            };
            if cov == 0 {
                continue;
            }
            let out = adj.map_pixel_amount(orig, cov as f32 / 255.0);
            tile.set_pixel(x, y, out);
        }
    }
}

/// Tiles to apply over: those intersecting `bounds` (doc px) if given, else all.
pub fn target_tiles(tiles: &TileMap, bounds: Option<[i32; 4]>, offset: [i32; 2]) -> Vec<(i32, i32)> {
    let t = TILE_SIZE as i32;
    tiles
        .tiles()
        .map(|(c, _)| *c)
        .filter(|&(tx, ty)| match bounds {
            None => true,
            Some([bx0, by0, bx1, by1]) => {
                let x0 = tx * t + offset[0];
                let y0 = ty * t + offset[1];
                x0 < bx1 && x0 + t > bx0 && y0 < by1 && y0 + t > by0
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn apply_tile_clips_to_mask() {
        let mut tile = Tile::default();
        for y in 0..TILE_SIZE {
            for x in 0..TILE_SIZE {
                tile.set_pixel(x, y, [10, 20, 30, 255]);
            }
        }
        let mut mask = Mask::new();
        for y in 0..TILE_SIZE as i32 {
            for x in 0..128 {
                mask.set(x, y, 255);
            }
        }
        apply_tile(&mut tile, Adjustment::Invert, 0, 0, [0, 0], Some(&mask));
        assert_eq!(tile.pixel(10, 10), [245, 235, 225, 255], "inside selection inverted");
        assert_eq!(tile.pixel(200, 10), [10, 20, 30, 255], "outside selection untouched");
    }

    #[test]
    fn target_tiles_filters_by_bounds() {
        let mut tiles = TileMap::new();
        tiles.set_pixel(10, 10, [1, 2, 3, 255]); // tile (0,0)
        tiles.set_pixel(300, 300, [1, 2, 3, 255]); // tile (1,1)
        let all = target_tiles(&tiles, None, [0, 0]);
        assert_eq!(all.len(), 2);
        let clipped = target_tiles(&tiles, Some([0, 0, 64, 64]), [0, 0]);
        assert_eq!(clipped, vec![(0, 0)]);
    }
}
