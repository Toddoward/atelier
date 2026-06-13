//! Drawable vector shape: a path plus fill and/or stroke (VEC-4).

use crate::path::Path;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum LineCap {
    #[default]
    Butt,
    Round,
    Square,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum LineJoin {
    #[default]
    Miter,
    Round,
    Bevel,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Stroke {
    pub color: [f32; 4],
    pub width: f32,
    pub cap: LineCap,
    pub join: LineJoin,
}

impl Default for Stroke {
    fn default() -> Self {
        Self { color: [0.0, 0.0, 0.0, 1.0], width: 1.0, cap: LineCap::default(), join: LineJoin::default() }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Shape {
    pub path: Path,
    pub fill: Option<[f32; 4]>,
    pub stroke: Option<Stroke>,
}

impl Shape {
    pub fn filled(path: Path, color: [f32; 4]) -> Self {
        Self { path, fill: Some(color), stroke: None }
    }

    pub fn stroked(path: Path, stroke: Stroke) -> Self {
        Self { path, fill: None, stroke: Some(stroke) }
    }
}

/// A vector layer's content: an ordered list of shapes (bottom-first).
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct VectorContent {
    pub shapes: Vec<Shape>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::path::Path;

    #[test]
    fn shape_serde_round_trip() {
        let s = Shape {
            path: Path::rect(0.0, 0.0, 10.0, 10.0),
            fill: Some([1.0, 0.0, 0.0, 1.0]),
            stroke: Some(Stroke { color: [0.0; 4], width: 2.0, ..Default::default() }),
        };
        let json = serde_json::to_string(&s).unwrap();
        assert_eq!(serde_json::from_str::<Shape>(&json).unwrap(), s);
    }
}
