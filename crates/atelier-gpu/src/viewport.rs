//! Canvas viewport: maps document coordinates to canvas-local screen points.
//!
//! `screen = doc * zoom + pan`, where `pan` is in canvas-local logical points and
//! `zoom` is points-per-document-pixel. UI-framework agnostic on purpose.

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Viewport {
    pub pan: [f32; 2],
    pub zoom: f32,
}

impl Default for Viewport {
    fn default() -> Self {
        Self { pan: [0.0, 0.0], zoom: 1.0 }
    }
}

impl Viewport {
    pub const MIN_ZOOM: f32 = 1.0 / 64.0;
    pub const MAX_ZOOM: f32 = 64.0;

    pub fn doc_to_screen(&self, doc: [f32; 2]) -> [f32; 2] {
        [doc[0] * self.zoom + self.pan[0], doc[1] * self.zoom + self.pan[1]]
    }

    pub fn screen_to_doc(&self, screen: [f32; 2]) -> [f32; 2] {
        [(screen[0] - self.pan[0]) / self.zoom, (screen[1] - self.pan[1]) / self.zoom]
    }

    /// Translate the view by a screen-space delta (drag pan).
    pub fn pan_by(&mut self, delta: [f32; 2]) {
        self.pan[0] += delta[0];
        self.pan[1] += delta[1];
    }

    /// Multiply zoom by `factor`, keeping the document point under `anchor`
    /// (canvas-local screen point) fixed on screen.
    pub fn zoom_about(&mut self, anchor: [f32; 2], factor: f32) {
        let new_zoom = (self.zoom * factor).clamp(Self::MIN_ZOOM, Self::MAX_ZOOM);
        let applied = new_zoom / self.zoom;
        self.pan[0] = anchor[0] - (anchor[0] - self.pan[0]) * applied;
        self.pan[1] = anchor[1] - (anchor[1] - self.pan[1]) * applied;
        self.zoom = new_zoom;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn assert_close(a: [f32; 2], b: [f32; 2]) {
        assert!((a[0] - b[0]).abs() < 1e-4 && (a[1] - b[1]).abs() < 1e-4, "{a:?} != {b:?}");
    }

    #[test]
    fn round_trip() {
        let vp = Viewport { pan: [10.0, -3.0], zoom: 2.5 };
        let doc = [123.0, 45.0];
        assert_close(vp.screen_to_doc(vp.doc_to_screen(doc)), doc);
    }

    #[test]
    fn zoom_about_keeps_anchor_fixed() {
        let mut vp = Viewport { pan: [50.0, 20.0], zoom: 1.0 };
        let anchor = [200.0, 150.0];
        let doc_under_cursor = vp.screen_to_doc(anchor);
        vp.zoom_about(anchor, 2.0);
        assert_close(vp.doc_to_screen(doc_under_cursor), anchor);
        assert!((vp.zoom - 2.0).abs() < 1e-6);
    }

    #[test]
    fn zoom_clamps() {
        let mut vp = Viewport::default();
        vp.zoom_about([0.0, 0.0], 1e9);
        assert_eq!(vp.zoom, Viewport::MAX_ZOOM);
        vp.zoom_about([0.0, 0.0], 0.0);
        assert_eq!(vp.zoom, Viewport::MIN_ZOOM);
    }

    #[test]
    fn pan_accumulates() {
        let mut vp = Viewport::default();
        vp.pan_by([5.0, 7.0]);
        vp.pan_by([-2.0, 3.0]);
        assert_close(vp.pan, [3.0, 10.0]);
    }
}
