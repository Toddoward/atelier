//! Vector path model: cubic-Bézier subpaths with a fill rule (VEC-1).
//! Pure data (serde); tessellation lives in `crate::tessellate`.

use serde::{Deserialize, Serialize};

pub type Point = [f32; 2];

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum Seg {
    Line(Point),
    /// Cubic Bézier: two control points then the endpoint.
    Cubic(Point, Point, Point),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SubPath {
    pub start: Point,
    pub segs: Vec<Seg>,
    pub closed: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum FillRule {
    #[default]
    NonZero,
    EvenOdd,
}

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct Path {
    pub subpaths: Vec<SubPath>,
    pub fill_rule: FillRule,
}

/// Incremental builder; `move_to` opens a subpath, `close` finishes it.
#[derive(Default)]
pub struct PathBuilder {
    path: Path,
    current: Option<SubPath>,
}

impl PathBuilder {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn move_to(&mut self, p: Point) -> &mut Self {
        self.flush(false);
        self.current = Some(SubPath { start: p, segs: Vec::new(), closed: false });
        self
    }

    pub fn line_to(&mut self, p: Point) -> &mut Self {
        self.cur().segs.push(Seg::Line(p));
        self
    }

    pub fn cubic_to(&mut self, c1: Point, c2: Point, p: Point) -> &mut Self {
        self.cur().segs.push(Seg::Cubic(c1, c2, p));
        self
    }

    pub fn close(&mut self) -> &mut Self {
        self.flush(true);
        self
    }

    pub fn fill_rule(&mut self, r: FillRule) -> &mut Self {
        self.path.fill_rule = r;
        self
    }

    pub fn build(mut self) -> Path {
        self.flush(false);
        self.path
    }

    fn cur(&mut self) -> &mut SubPath {
        self.current.get_or_insert(SubPath { start: [0.0, 0.0], segs: Vec::new(), closed: false })
    }

    fn flush(&mut self, closed: bool) {
        if let Some(mut sp) = self.current.take() {
            sp.closed = closed;
            self.path.subpaths.push(sp);
        }
    }
}

impl Path {
    /// Axis-aligned rectangle (closed).
    pub fn rect(x: f32, y: f32, w: f32, h: f32) -> Path {
        let mut b = PathBuilder::new();
        b.move_to([x, y])
            .line_to([x + w, y])
            .line_to([x + w, y + h])
            .line_to([x, y + h])
            .close();
        b.build()
    }

    /// Open or closed polyline through `points` (pen tool, spec 0016).
    pub fn polyline(points: &[Point], closed: bool) -> Path {
        let mut b = PathBuilder::new();
        if let Some((first, rest)) = points.split_first() {
            b.move_to(*first);
            for p in rest {
                b.line_to(*p);
            }
            if closed {
                b.close();
            }
        }
        b.build()
    }

    /// Regular `sides`-gon inscribed in radius `r`, first vertex pointing up.
    pub fn polygon(cx: f32, cy: f32, r: f32, sides: u32) -> Path {
        let sides = sides.max(3);
        let mut b = PathBuilder::new();
        for i in 0..sides {
            let a = -std::f32::consts::FRAC_PI_2
                + i as f32 * std::f32::consts::TAU / sides as f32;
            let p = [cx + r * a.cos(), cy + r * a.sin()];
            if i == 0 {
                b.move_to(p);
            } else {
                b.line_to(p);
            }
        }
        b.close();
        b.build()
    }

    /// `points`-pointed star alternating between `r_outer` and `r_inner`.
    pub fn star(cx: f32, cy: f32, r_outer: f32, r_inner: f32, points: u32) -> Path {
        let points = points.max(3);
        let mut b = PathBuilder::new();
        for i in 0..points * 2 {
            let r = if i % 2 == 0 { r_outer } else { r_inner };
            let a = -std::f32::consts::FRAC_PI_2
                + i as f32 * std::f32::consts::PI / points as f32;
            let p = [cx + r * a.cos(), cy + r * a.sin()];
            if i == 0 {
                b.move_to(p);
            } else {
                b.line_to(p);
            }
        }
        b.close();
        b.build()
    }

    /// Ellipse inscribed in the rect, via 4 cubic arcs (kappa approximation).
    pub fn ellipse(cx: f32, cy: f32, rx: f32, ry: f32) -> Path {
        const K: f32 = 0.552_285; // 4/3 * (sqrt(2)-1), kappa
        let (ox, oy) = (rx * K, ry * K);
        let mut b = PathBuilder::new();
        b.move_to([cx, cy - ry])
            .cubic_to([cx + ox, cy - ry], [cx + rx, cy - oy], [cx + rx, cy])
            .cubic_to([cx + rx, cy + oy], [cx + ox, cy + ry], [cx, cy + ry])
            .cubic_to([cx - ox, cy + ry], [cx - rx, cy + oy], [cx - rx, cy])
            .cubic_to([cx - rx, cy - oy], [cx - ox, cy - ry], [cx, cy - ry])
            .close();
        b.build()
    }

    /// On-path anchor points (subpath starts + each segment endpoint), in
    /// traversal order. Bézier control handles are not anchors (spec 0017).
    pub fn anchors(&self) -> Vec<Point> {
        let mut out = Vec::new();
        for sp in &self.subpaths {
            out.push(sp.start);
            for s in &sp.segs {
                out.push(match s {
                    Seg::Line(p) => *p,
                    Seg::Cubic(_, _, p) => *p,
                });
            }
        }
        out
    }

    /// Move the anchor at `index` (order matches [`anchors`]) to `to`.
    /// Translates a cubic segment's endpoint without moving its handles.
    pub fn move_anchor(&mut self, index: usize, to: Point) {
        let mut i = 0;
        for sp in &mut self.subpaths {
            if i == index {
                sp.start = to;
                return;
            }
            i += 1;
            for s in &mut sp.segs {
                if i == index {
                    match s {
                        Seg::Line(p) => *p = to,
                        Seg::Cubic(_, _, p) => *p = to,
                    }
                    return;
                }
                i += 1;
            }
        }
    }

    /// Remove the anchor at `index` (order matches [`anchors`]); the path
    /// reconnects across the gap. Returns false (no-op) if it would leave a
    /// subpath with fewer than 2 anchors. Spec 0018.
    pub fn remove_anchor(&mut self, index: usize) -> bool {
        let mut i = 0;
        for sp in &mut self.subpaths {
            let n = 1 + sp.segs.len();
            if index < i + n {
                if n <= 2 {
                    return false; // keep at least a 2-anchor subpath
                }
                let local = index - i;
                if local == 0 {
                    // Drop the first segment; its endpoint becomes the new start.
                    let first = sp.segs.remove(0);
                    sp.start = match first {
                        Seg::Line(p) | Seg::Cubic(_, _, p) => p,
                    };
                } else {
                    sp.segs.remove(local - 1);
                }
                return true;
            }
            i += n;
        }
        false
    }

    /// Insert a new line anchor at `point` immediately before the anchor at
    /// `index` (so between anchor index-1 and index of the same subpath).
    /// No-op for a subpath start boundary or out-of-range. Spec 0018.
    pub fn insert_anchor(&mut self, index: usize, point: Point) -> bool {
        let mut i = 0;
        for sp in &mut self.subpaths {
            let n = 1 + sp.segs.len();
            if index < i + n {
                let local = index - i;
                if local == 0 {
                    return false; // can't insert before a subpath start
                }
                sp.segs.insert(local - 1, Seg::Line(point));
                return true;
            }
            i += n;
        }
        false
    }

    /// Tight-ish bounds over anchor + control points (control hull, not exact
    /// curve extrema — sufficient for culling/placement).
    pub fn bounds(&self) -> Option<[f32; 4]> {
        let mut it = self.subpaths.iter().flat_map(|sp| {
            std::iter::once(sp.start).chain(sp.segs.iter().flat_map(|s| match s {
                Seg::Line(p) => vec![*p],
                Seg::Cubic(a, b, c) => vec![*a, *b, *c],
            }))
        });
        let first = it.next()?;
        let (mut x0, mut y0, mut x1, mut y1) = (first[0], first[1], first[0], first[1]);
        for p in it {
            x0 = x0.min(p[0]);
            y0 = y0.min(p[1]);
            x1 = x1.max(p[0]);
            y1 = y1.max(p[1]);
        }
        Some([x0, y0, x1, y1])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rect_has_four_corners_and_bounds() {
        let p = Path::rect(10.0, 20.0, 30.0, 40.0);
        assert_eq!(p.subpaths.len(), 1);
        assert!(p.subpaths[0].closed);
        assert_eq!(p.subpaths[0].segs.len(), 3, "3 line segs + implicit close");
        assert_eq!(p.bounds(), Some([10.0, 20.0, 40.0, 60.0]));
    }

    #[test]
    fn ellipse_bounds_match_rect() {
        let p = Path::ellipse(50.0, 50.0, 20.0, 10.0);
        let b = p.bounds().unwrap();
        assert!((b[0] - 30.0).abs() < 1e-3 && (b[2] - 70.0).abs() < 1e-3);
        assert!((b[1] - 40.0).abs() < 1e-3 && (b[3] - 60.0).abs() < 1e-3);
    }

    #[test]
    fn polygon_has_n_vertices_and_fits_radius() {
        let p = Path::polygon(0.0, 0.0, 10.0, 6);
        assert!(p.subpaths[0].closed);
        // move_to start + 5 line segs = 6 vertices for a hexagon.
        assert_eq!(p.subpaths[0].segs.len(), 5);
        let b = p.bounds().unwrap();
        assert!(b[0] >= -10.001 && b[2] <= 10.001, "within radius: {b:?}");
        // First vertex points up.
        assert!((p.subpaths[0].start[1] + 10.0).abs() < 1e-3);
    }

    #[test]
    fn star_alternates_radii() {
        let p = Path::star(0.0, 0.0, 10.0, 4.0, 5);
        assert!(p.subpaths[0].closed);
        // 5-point star = 10 vertices = move_to + 9 line segs.
        assert_eq!(p.subpaths[0].segs.len(), 9);
        let b = p.bounds().unwrap();
        assert!((b[3]).abs() <= 10.001 && (b[1]) >= -10.001);
    }

    #[test]
    fn anchors_and_move_anchor() {
        let mut p = Path::rect(0.0, 0.0, 10.0, 10.0);
        // rect = start + 3 line segs = 4 anchors.
        let a = p.anchors();
        assert_eq!(a.len(), 4);
        assert_eq!(a[0], [0.0, 0.0]);
        assert_eq!(a[2], [10.0, 10.0]);
        // Move the start anchor and a seg endpoint.
        p.move_anchor(0, [-5.0, -5.0]);
        p.move_anchor(2, [20.0, 20.0]);
        let a = p.anchors();
        assert_eq!(a[0], [-5.0, -5.0]);
        assert_eq!(a[2], [20.0, 20.0]);
        // Out-of-range index is a no-op.
        let before = p.clone();
        p.move_anchor(99, [1.0, 1.0]);
        assert_eq!(p, before);
    }

    #[test]
    fn remove_and_insert_anchor() {
        let mut p = Path::rect(0.0, 0.0, 10.0, 10.0); // 4 anchors
        assert!(p.remove_anchor(1));
        assert_eq!(p.anchors().len(), 3, "removed one anchor");
        // Insert before anchor 1 → back to 4.
        assert!(p.insert_anchor(1, [5.0, 0.0]));
        let a = p.anchors();
        assert_eq!(a.len(), 4);
        assert_eq!(a[1], [5.0, 0.0], "new anchor placed");
        // Can't insert before a subpath start, OOB is a no-op.
        assert!(!p.insert_anchor(0, [1.0, 1.0]));
        assert!(!p.remove_anchor(99));
    }

    #[test]
    fn remove_anchor_keeps_minimum_two() {
        // A 2-anchor open path can't lose an anchor.
        let mut p = Path::polyline(&[[0.0, 0.0], [10.0, 0.0]], false);
        assert_eq!(p.anchors().len(), 2);
        assert!(!p.remove_anchor(0));
        assert!(!p.remove_anchor(1));
        assert_eq!(p.anchors().len(), 2);
    }

    #[test]
    fn move_anchor_keeps_cubic_handles() {
        let mut p = Path::ellipse(0.0, 0.0, 10.0, 10.0);
        // anchors = start + 4 cubic endpoints = 5.
        assert_eq!(p.anchors().len(), 5);
        let Seg::Cubic(c1, c2, _) = p.subpaths[0].segs[0] else { panic!("cubic") };
        p.move_anchor(1, [3.0, 3.0]); // first cubic endpoint
        assert_eq!(p.anchors()[1], [3.0, 3.0]);
        let Seg::Cubic(n1, n2, e) = p.subpaths[0].segs[0] else { panic!("cubic") };
        assert_eq!((n1, n2), (c1, c2), "handles unchanged");
        assert_eq!(e, [3.0, 3.0]);
    }

    #[test]
    fn builder_round_trips_through_serde() {
        let p = Path::rect(0.0, 0.0, 5.0, 5.0);
        let json = serde_json::to_string(&p).unwrap();
        let back: Path = serde_json::from_str(&json).unwrap();
        assert_eq!(p, back);
    }
}
