//! Boolean path operations (VEC-5, spec 0031) via `i_overlay`. Cubic segments
//! are flattened to polylines, the overlay runs on the resulting polygons, and
//! the result is rebuilt as a line-only `Path` (which may be a compound path —
//! multiple subpaths for holes / disjoint regions).

use crate::path::{Path, PathBuilder, Seg};
use i_overlay::core::fill_rule::FillRule;
use i_overlay::core::overlay_rule::OverlayRule;
use i_overlay::float::single::SingleFloatOverlay;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BoolOp {
    Union,
    Intersect,
    Difference,
    Exclude,
}

impl BoolOp {
    fn rule(self) -> OverlayRule {
        match self {
            BoolOp::Union => OverlayRule::Union,
            BoolOp::Intersect => OverlayRule::Intersect,
            BoolOp::Difference => OverlayRule::Difference,
            BoolOp::Exclude => OverlayRule::Xor,
        }
    }
}

/// Steps used to flatten each cubic segment into line segments.
const FLATTEN_STEPS: usize = 24;

fn flatten_cubic(p0: [f32; 2], c1: [f32; 2], c2: [f32; 2], p1: [f32; 2], out: &mut Vec<[f32; 2]>) {
    for i in 1..=FLATTEN_STEPS {
        let t = i as f32 / FLATTEN_STEPS as f32;
        let mt = 1.0 - t;
        let a = mt * mt * mt;
        let b = 3.0 * mt * mt * t;
        let c = 3.0 * mt * t * t;
        let d = t * t * t;
        out.push([
            a * p0[0] + b * c1[0] + c * c2[0] + d * p1[0],
            a * p0[1] + b * c1[1] + c * c2[1] + d * p1[1],
        ]);
    }
}

/// A path → one polygon contour (Vec of points) per subpath (implicitly closed).
fn path_to_contours(path: &Path) -> Vec<Vec<[f32; 2]>> {
    let mut contours = Vec::new();
    for sp in &path.subpaths {
        let mut pts = vec![sp.start];
        let mut cur = sp.start;
        for seg in &sp.segs {
            match seg {
                Seg::Line(p) => {
                    pts.push(*p);
                    cur = *p;
                }
                Seg::Cubic(c1, c2, p) => {
                    flatten_cubic(cur, *c1, *c2, *p, &mut pts);
                    cur = *p;
                }
            }
        }
        if pts.len() >= 3 {
            contours.push(pts);
        }
    }
    contours
}

/// Boolean of two paths. Empty result → an empty `Path` (no subpaths).
pub fn boolean(subj: &Path, clip: &Path, op: BoolOp) -> Path {
    let s = path_to_contours(subj);
    let c = path_to_contours(clip);
    if s.is_empty() {
        return if op == BoolOp::Union { clip.clone() } else { Path::default() };
    }
    if c.is_empty() {
        return match op {
            BoolOp::Intersect => Path::default(),
            _ => subj.clone(),
        };
    }
    // Shapes<[f32;2]> = Vec<shape: Vec<contour: Vec<[f32;2]>>>.
    let shapes = s.overlay(&c, op.rule(), FillRule::NonZero);
    let mut b = PathBuilder::new();
    for shape in &shapes {
        for contour in shape {
            if contour.len() < 3 {
                continue;
            }
            b.move_to(contour[0]);
            for p in &contour[1..] {
                b.line_to(*p);
            }
            b.close();
        }
    }
    b.build()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn area_bounds(p: &Path) -> Option<[f32; 4]> {
        p.bounds()
    }

    #[test]
    fn union_of_overlapping_rects_spans_both() {
        let a = Path::rect(0.0, 0.0, 10.0, 10.0);
        let b = Path::rect(5.0, 0.0, 10.0, 10.0);
        let u = boolean(&a, &b, BoolOp::Union);
        let bb = area_bounds(&u).expect("non-empty");
        assert!(bb[0] <= 0.5 && bb[2] >= 14.5, "spans x 0..15: {bb:?}");
    }

    #[test]
    fn intersection_is_the_overlap() {
        let a = Path::rect(0.0, 0.0, 10.0, 10.0);
        let b = Path::rect(6.0, 0.0, 10.0, 10.0);
        let i = boolean(&a, &b, BoolOp::Intersect);
        let bb = area_bounds(&i).expect("non-empty");
        assert!(bb[0] >= 5.5 && bb[2] <= 10.5, "overlap x 6..10: {bb:?}");
    }

    #[test]
    fn difference_removes_clip() {
        let a = Path::rect(0.0, 0.0, 10.0, 10.0);
        let b = Path::rect(6.0, 0.0, 10.0, 10.0);
        let d = boolean(&a, &b, BoolOp::Difference);
        let bb = area_bounds(&d).expect("non-empty");
        assert!(bb[2] <= 6.5, "right part removed: {bb:?}");
    }

    #[test]
    fn disjoint_intersection_is_empty() {
        let a = Path::rect(0.0, 0.0, 10.0, 10.0);
        let b = Path::rect(50.0, 0.0, 10.0, 10.0);
        let i = boolean(&a, &b, BoolOp::Intersect);
        assert!(i.subpaths.is_empty(), "no overlap → empty path");
    }

    #[test]
    fn union_of_disjoint_keeps_two_contours() {
        let a = Path::rect(0.0, 0.0, 10.0, 10.0);
        let b = Path::rect(50.0, 0.0, 10.0, 10.0);
        let u = boolean(&a, &b, BoolOp::Union);
        assert_eq!(u.subpaths.len(), 2, "two disjoint regions");
    }
}
