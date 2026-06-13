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
    fn builder_round_trips_through_serde() {
        let p = Path::rect(0.0, 0.0, 5.0, 5.0);
        let json = serde_json::to_string(&p).unwrap();
        let back: Path = serde_json::from_str(&json).unwrap();
        assert_eq!(p, back);
    }
}
