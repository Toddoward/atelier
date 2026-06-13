//! Tessellate shapes to flat-color triangle meshes via lyon (VEC-7 prep).
//! The GPU renderer (spec 0013) consumes `Mesh` directly.

use crate::path::{FillRule, Path, Seg};
use crate::shape::{LineCap, LineJoin, Shape, Stroke};
use lyon::math::point;
use lyon::path::Path as LyonPath;
use lyon::tessellation::{
    BuffersBuilder, FillOptions, FillTessellator, FillVertex, StrokeOptions, StrokeTessellator,
    StrokeVertex, VertexBuffers,
};

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Vertex {
    pub pos: [f32; 2],
    pub color: [f32; 4],
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct Mesh {
    pub vertices: Vec<Vertex>,
    pub indices: Vec<u32>,
}

impl Mesh {
    pub fn is_empty(&self) -> bool {
        self.indices.is_empty()
    }
}

fn to_lyon(path: &Path) -> LyonPath {
    let mut b = LyonPath::builder();
    for sp in &path.subpaths {
        b.begin(point(sp.start[0], sp.start[1]));
        for seg in &sp.segs {
            match seg {
                Seg::Line(p) => {
                    b.line_to(point(p[0], p[1]));
                }
                Seg::Cubic(c1, c2, p) => {
                    b.cubic_bezier_to(
                        point(c1[0], c1[1]),
                        point(c2[0], c2[1]),
                        point(p[0], p[1]),
                    );
                }
            }
        }
        b.end(sp.closed);
    }
    b.build()
}

fn lyon_cap(c: LineCap) -> lyon::tessellation::LineCap {
    match c {
        LineCap::Butt => lyon::tessellation::LineCap::Butt,
        LineCap::Round => lyon::tessellation::LineCap::Round,
        LineCap::Square => lyon::tessellation::LineCap::Square,
    }
}

fn lyon_join(j: LineJoin) -> lyon::tessellation::LineJoin {
    match j {
        LineJoin::Miter => lyon::tessellation::LineJoin::Miter,
        LineJoin::Round => lyon::tessellation::LineJoin::Round,
        LineJoin::Bevel => lyon::tessellation::LineJoin::Bevel,
    }
}

/// Tessellate a shape: fill first (under), then stroke (over). Returns one mesh
/// with per-vertex color so the GPU shader stays a flat-color triangle pass.
pub fn tessellate(shape: &Shape) -> Mesh {
    let lyon_path = to_lyon(&shape.path);
    let mut buffers: VertexBuffers<Vertex, u32> = VertexBuffers::new();

    if let Some(color) = shape.fill {
        let mut tess = FillTessellator::new();
        let opts = FillOptions::tolerance(0.1).with_fill_rule(match shape.path.fill_rule {
            FillRule::NonZero => lyon::path::FillRule::NonZero,
            FillRule::EvenOdd => lyon::path::FillRule::EvenOdd,
        });
        let _ = tess.tessellate_path(
            &lyon_path,
            &opts,
            &mut BuffersBuilder::new(&mut buffers, move |v: FillVertex| Vertex {
                pos: v.position().to_array(),
                color,
            }),
        );
    }

    if let Some(Stroke { color, width, cap, join }) = shape.stroke {
        let mut tess = StrokeTessellator::new();
        let opts = StrokeOptions::tolerance(0.1)
            .with_line_width(width)
            .with_line_cap(lyon_cap(cap))
            .with_line_join(lyon_join(join));
        let _ = tess.tessellate_path(
            &lyon_path,
            &opts,
            &mut BuffersBuilder::new(&mut buffers, move |v: StrokeVertex| Vertex {
                pos: v.position().to_array(),
                color,
            }),
        );
    }

    Mesh { vertices: buffers.vertices, indices: buffers.indices }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::path::{Path, PathBuilder};
    use crate::shape::Stroke;

    fn area_covered(mesh: &Mesh) -> f32 {
        // Sum unsigned triangle areas — sanity that we tessellated real geometry.
        let mut a = 0.0;
        for tri in mesh.indices.chunks_exact(3) {
            let p0 = mesh.vertices[tri[0] as usize].pos;
            let p1 = mesh.vertices[tri[1] as usize].pos;
            let p2 = mesh.vertices[tri[2] as usize].pos;
            a += ((p1[0] - p0[0]) * (p2[1] - p0[1]) - (p2[0] - p0[0]) * (p1[1] - p0[1])).abs()
                * 0.5;
        }
        a
    }

    #[test]
    fn fill_square_covers_its_area() {
        let m = tessellate(&Shape::filled(Path::rect(0.0, 0.0, 10.0, 10.0), [1.0, 0.0, 0.0, 1.0]));
        assert!(!m.is_empty(), "produced triangles");
        assert!(m.indices.len().is_multiple_of(3));
        assert!((area_covered(&m) - 100.0).abs() < 1.0, "≈100 px² covered");
        assert_eq!(m.vertices[0].color, [1.0, 0.0, 0.0, 1.0]);
    }

    #[test]
    fn stroke_produces_geometry() {
        let m = tessellate(&Shape::stroked(
            Path::rect(0.0, 0.0, 10.0, 10.0),
            Stroke { color: [0.0, 0.0, 0.0, 1.0], width: 2.0, ..Default::default() },
        ));
        assert!(!m.is_empty(), "stroke tessellated");
        assert!(area_covered(&m) > 0.0);
    }

    #[test]
    fn even_odd_vs_nonzero_differ_on_overlap() {
        // A path that winds twice over a central region (figure-with-hole intent).
        let outer = |fr| {
            let mut b = PathBuilder::new();
            b.move_to([0.0, 0.0])
                .line_to([30.0, 0.0])
                .line_to([30.0, 30.0])
                .line_to([0.0, 30.0])
                .close()
                // inner loop, SAME winding as outer: non-zero fills it (count 2),
                // even-odd carves it as a hole (count even).
                .move_to([10.0, 10.0])
                .line_to([20.0, 10.0])
                .line_to([20.0, 20.0])
                .line_to([10.0, 20.0])
                .close();
            b.fill_rule(fr);
            Shape::filled(b.build(), [1.0; 4])
        };
        let nz = area_covered(&tessellate(&outer(FillRule::NonZero)));
        let eo = area_covered(&tessellate(&outer(FillRule::EvenOdd)));
        // Even-odd carves the inner square as a hole; non-zero (same winding) fills it.
        assert!(nz > eo + 50.0, "non-zero {nz} should exceed even-odd {eo}");
    }
}
