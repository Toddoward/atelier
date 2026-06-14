//! `.atl` container: `manifest.json` (schema-versioned document JSON) plus,
//! since schema v1, lz4-compressed binary tile parts
//! `tiles/<node-id>/<tx>_<ty>.bin`. See docs/FORMAT-ATL.md.
//!
//! v0 files (manifest only, no pixels) still load.

use atelier_core::{Document, NodeId, NodeKind, Tile};
use serde::{Deserialize, Serialize};
use std::fs::File;
use std::io::{Read, Write};
use std::path::Path;
use zip::write::SimpleFileOptions;
use zip::{ZipArchive, ZipWriter};

pub const SCHEMA_VERSION: u32 = 3;
const MANIFEST: &str = "manifest.json";

#[derive(Debug, thiserror::Error)]
pub enum AtlError {
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("not a valid .atl container: {0}")]
    Container(#[from] zip::result::ZipError),
    #[error("manifest is not valid JSON: {0}")]
    Manifest(#[from] serde_json::Error),
    #[error("file uses schema v{0}, this build reads up to v{SCHEMA_VERSION} — update Atelier")]
    VersionTooNew(u32),
    #[error("malformed tile part \"{0}\"")]
    BadTilePart(String),
}

#[derive(Serialize, Deserialize)]
struct Manifest {
    schema_version: u32,
    document: serde_json::Value,
}

pub fn save_atl(doc: &Document, path: &Path) -> Result<(), AtlError> {
    let manifest = Manifest {
        schema_version: SCHEMA_VERSION,
        document: serde_json::to_value(doc)?,
    };
    let mut zip = ZipWriter::new(File::create(path)?);
    // Tile bytes are already lz4-compressed; don't deflate them again.
    let stored = SimpleFileOptions::default().compression_method(zip::CompressionMethod::Stored);
    zip.start_file(MANIFEST, SimpleFileOptions::default())?;
    zip.write_all(serde_json::to_string_pretty(&manifest)?.as_bytes())?;

    write_parts(&mut zip, doc, "", stored)?;
    zip.finish()?;
    Ok(())
}

/// Recursively write tile/mask parts. `prefix` is the dotted node-id chain of
/// the smart-object ancestors (empty at the top level). Raster nodes write their
/// pixels/mask under `<prefix>.<id>` (just `<id>` at the top); Smart nodes recurse
/// into their embedded document (schema v3, spec 0053).
fn write_parts(
    zip: &mut ZipWriter<File>,
    doc: &Document,
    prefix: &str,
    stored: SimpleFileOptions,
) -> Result<(), AtlError> {
    for (id, _) in doc.iter_tree() {
        let Some(node) = doc.node(id) else { continue };
        let key = if prefix.is_empty() {
            id.0.to_string()
        } else {
            format!("{prefix}.{}", id.0)
        };
        match &node.kind {
            NodeKind::Raster(content) => {
                for (&(tx, ty), tile) in content.tiles.tiles() {
                    zip.start_file(format!("tiles/{key}/{tx}_{ty}.bin"), stored)?;
                    zip.write_all(&lz4_flex::compress_prepend_size(tile.bytes()))?;
                }
                // Layer mask (schema v2): header (x0,y0,w,h i32 LE) + coverage bytes.
                if let Some((x0, y0, w, h, cov)) =
                    content.mask.as_ref().and_then(|m| m.to_region_bytes())
                {
                    let mut buf = Vec::with_capacity(16 + cov.len());
                    buf.extend_from_slice(&x0.to_le_bytes());
                    buf.extend_from_slice(&y0.to_le_bytes());
                    buf.extend_from_slice(&w.to_le_bytes());
                    buf.extend_from_slice(&h.to_le_bytes());
                    buf.extend_from_slice(&cov);
                    zip.start_file(format!("masks/{key}.bin"), stored)?;
                    zip.write_all(&lz4_flex::compress_prepend_size(&buf))?;
                }
            }
            NodeKind::Smart(content) => {
                write_parts(zip, &content.doc, &key, stored)?;
            }
            _ => {}
        }
    }
    Ok(())
}

/// Parse a dotted node-id chain (e.g. `"3.1"`) into ids. Returns `None` on any
/// non-numeric or empty segment.
fn parse_chain(s: &str) -> Option<Vec<u64>> {
    let chain: Option<Vec<u64>> = s.split('.').map(|seg| seg.parse::<u64>().ok()).collect();
    chain.filter(|c| !c.is_empty())
}

/// Resolve a node-id chain to the target raster content, descending through
/// nested smart-object documents for each non-final id (spec 0053).
fn resolve_raster_mut<'a>(
    doc: &'a mut Document,
    chain: &[u64],
) -> Option<&'a mut atelier_core::RasterContent> {
    let (&leaf, ancestors) = chain.split_last()?;
    let mut cur = doc;
    for &nid in ancestors {
        let NodeKind::Smart(content) = &mut cur.node_mut(NodeId(nid))?.kind else {
            return None;
        };
        cur = content.doc.as_mut();
    }
    match &mut cur.node_mut(NodeId(leaf))?.kind {
        NodeKind::Raster(content) => Some(content),
        _ => None,
    }
}

pub fn load_atl(path: &Path) -> Result<Document, AtlError> {
    let mut zip = ZipArchive::new(File::open(path)?)?;
    let mut raw = String::new();
    zip.by_name(MANIFEST)?.read_to_string(&mut raw)?;
    let manifest: Manifest = serde_json::from_str(&raw)?;
    if manifest.schema_version > SCHEMA_VERSION {
        return Err(AtlError::VersionTooNew(manifest.schema_version));
    }
    let mut doc_json = manifest.document;
    if manifest.schema_version == 0 {
        migrate_v0(&mut doc_json);
    }
    migrate_vector_placeholder(&mut doc_json);
    let mut doc: Document = serde_json::from_value(doc_json)?;

    // Reattach tile parts (absent in v0 files — manifest alone is a valid doc).
    // Keys are a dotted node-id chain through nested smart objects (schema v3);
    // v1/v2 single-id keys resolve as a one-element chain.
    let names: Vec<String> = zip.file_names().map(String::from).collect();
    for name in names {
        let Some(rest) = name.strip_prefix("tiles/") else { continue };
        let parse = || -> Option<(Vec<u64>, i32, i32)> {
            let (key, coords) = rest.split_once('/')?;
            let (tx, ty) = coords.strip_suffix(".bin")?.split_once('_')?;
            Some((parse_chain(key)?, tx.parse().ok()?, ty.parse().ok()?))
        };
        let (chain, tx, ty) = parse().ok_or_else(|| AtlError::BadTilePart(name.clone()))?;

        let mut compressed = Vec::new();
        zip.by_name(&name)?.read_to_end(&mut compressed)?;
        let bytes = lz4_flex::decompress_size_prepended(&compressed)
            .map_err(|_| AtlError::BadTilePart(name.clone()))?;
        let tile = Tile::from_bytes(bytes).map_err(|_| AtlError::BadTilePart(name.clone()))?;

        match resolve_raster_mut(&mut doc, &chain) {
            Some(content) => content.tiles.insert_tile((tx, ty), tile),
            None => return Err(AtlError::BadTilePart(name)),
        }
    }

    // Reattach layer mask parts (schema v2; v3 dotted chains for nested docs).
    let mask_names: Vec<String> =
        zip.file_names().filter(|n| n.starts_with("masks/")).map(String::from).collect();
    for name in mask_names {
        let chain = name
            .strip_prefix("masks/")
            .and_then(|r| r.strip_suffix(".bin"))
            .and_then(parse_chain)
            .ok_or_else(|| AtlError::BadTilePart(name.clone()))?;
        let mut compressed = Vec::new();
        zip.by_name(&name)?.read_to_end(&mut compressed)?;
        let buf = lz4_flex::decompress_size_prepended(&compressed)
            .map_err(|_| AtlError::BadTilePart(name.clone()))?;
        if buf.len() < 16 {
            return Err(AtlError::BadTilePart(name));
        }
        let rd = |o: usize| i32::from_le_bytes([buf[o], buf[o + 1], buf[o + 2], buf[o + 3]]);
        let (x0, y0, w, h) = (rd(0), rd(4), rd(8) as u32, rd(12) as u32);
        let cov = &buf[16..];
        if cov.len() != (w as usize * h as usize) {
            return Err(AtlError::BadTilePart(name));
        }
        let mask = atelier_core::Mask::from_region_bytes(x0, y0, w, h, cov);
        if let Some(content) = resolve_raster_mut(&mut doc, &chain) {
            content.mask = Some(mask);
        }
    }
    Ok(doc)
}

/// v0 → v1: raster payload was a bare `PlaceholderArt`; v1 wraps it as
/// `RasterContent { art, tiles }` (tiles live in binary parts, absent in v0).
fn migrate_v0(doc_json: &mut serde_json::Value) {
    let Some(nodes) = doc_json.get_mut("nodes").and_then(|n| n.as_object_mut()) else {
        return;
    };
    for node in nodes.values_mut() {
        let Some(raster) = node.get_mut("kind").and_then(|k| k.get_mut("Raster")) else {
            continue;
        };
        if raster.get("art").is_none() {
            let old = raster.take();
            *raster = serde_json::json!({ "art": old });
        }
    }
}

/// Migrate legacy `Vector(PlaceholderArt{bounds,color})` nodes (spec 0012) to a
/// `VectorContent` holding one filled-rectangle shape.
fn migrate_vector_placeholder(doc_json: &mut serde_json::Value) {
    use atelier_core::atelier_vector::{Path, Shape};
    let Some(nodes) = doc_json.get_mut("nodes").and_then(|n| n.as_object_mut()) else {
        return;
    };
    for node in nodes.values_mut() {
        let Some(vec_val) = node.get_mut("kind").and_then(|k| k.get_mut("Vector")) else {
            continue;
        };
        // New form already has "shapes"; only migrate the old {bounds,color}.
        let (Some(bounds), Some(color)) = (
            vec_val.get("bounds").and_then(|b| b.as_array()).cloned(),
            vec_val.get("color").and_then(|c| c.as_array()).cloned(),
        ) else {
            continue;
        };
        let f = |v: &serde_json::Value| v.as_f64().unwrap_or(0.0) as f32;
        let b: Vec<f32> = bounds.iter().map(f).collect();
        let c: Vec<f32> = color.iter().map(f).collect();
        if b.len() == 4 && c.len() == 4 {
            let shape = Shape::filled(
                Path::rect(b[0], b[1], b[2], b[3]),
                [c[0], c[1], c[2], c[3]],
            );
            let content = atelier_core::VectorContent { shapes: vec![shape] };
            *vec_val = serde_json::to_value(content).expect("serialize vector content");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use atelier_core::command::AddNode;
    use atelier_core::{Command, LayerProps, Node, NodeKind, PlaceholderArt, ProjectFocus, RasterContent};

    fn temp(name: &str) -> std::path::PathBuf {
        std::env::temp_dir().join(format!("atelier-test-{}-{name}.atl", std::process::id()))
    }

    fn sample_doc() -> Document {
        let mut doc = Document::new([320, 240], ProjectFocus::Vector);
        let root = doc.root();
        let mut add_g = AddNode::new(&mut doc, Node::group("g"), root, 0);
        add_g.apply(&mut doc);
        let leaf = Node::new(
            LayerProps::named("layer 1"),
            NodeKind::Raster(RasterContent::from_placeholder(PlaceholderArt {
                bounds: [1.0, 2.0, 30.0, 40.0],
                color: [0.5; 4],
            })),
        );
        let mut add_l = AddNode::new(&mut doc, leaf, add_g.id, 0);
        add_l.apply(&mut doc);
        doc
    }

    #[test]
    fn save_load_round_trips_deep_equal() {
        let doc = sample_doc();
        let path = temp("roundtrip");
        save_atl(&doc, &path).unwrap();
        let loaded = load_atl(&path).unwrap();
        std::fs::remove_file(&path).ok();
        assert_eq!(doc, loaded);
    }

    #[test]
    fn round_trip_preserves_pixels() {
        let doc = sample_doc();
        let path = temp("pixels");
        save_atl(&doc, &path).unwrap();
        let loaded = load_atl(&path).unwrap();
        std::fs::remove_file(&path).ok();
        // Find the raster layer and check a pixel inside its filled rect.
        let (id, _) = doc.iter_tree().into_iter().nth(1).expect("layer present");
        let NodeKind::Raster(content) = &loaded.node(id).unwrap().kind else {
            panic!("raster node expected")
        };
        assert_eq!(content.tiles.pixel(5, 10), [128, 128, 128, 128]);
        assert!(!content.tiles.is_empty());
    }

    /// A v0 file (manifest only, bare-PlaceholderArt raster payloads) still loads.
    #[test]
    fn loads_v0_schema_files() {
        let doc = sample_doc();
        // Build the v0 JSON shape from the current model: unwrap Raster.art.
        let mut doc_json = serde_json::to_value(&doc).unwrap();
        for node in doc_json["nodes"].as_object_mut().unwrap().values_mut() {
            if let Some(raster) = node.get_mut("kind").and_then(|k| k.get_mut("Raster")) {
                let art = raster["art"].take();
                *raster = art;
            }
        }
        let path = temp("v0");
        let mut zip = ZipWriter::new(File::create(&path).unwrap());
        zip.start_file(MANIFEST, SimpleFileOptions::default()).unwrap();
        let manifest =
            serde_json::json!({ "schema_version": 0, "document": doc_json }).to_string();
        zip.write_all(manifest.as_bytes()).unwrap();
        zip.finish().unwrap();

        let loaded = load_atl(&path).unwrap();
        std::fs::remove_file(&path).ok();
        assert_eq!(loaded.node_count(), doc.node_count());
        let (id, _) = doc.iter_tree().into_iter().nth(1).expect("layer present");
        let NodeKind::Raster(content) = &loaded.node(id).unwrap().kind else {
            panic!("raster node expected")
        };
        assert!(content.art.is_some(), "placeholder art migrated");
        assert!(content.tiles.is_empty(), "v0 carries no pixels");
    }

    #[test]
    fn rejects_malformed_tile_part() {
        let doc = sample_doc();
        let path = temp("badtile");
        // Valid v1 manifest + a garbage tile part.
        let mut zip = ZipWriter::new(File::create(&path).unwrap());
        zip.start_file(MANIFEST, SimpleFileOptions::default()).unwrap();
        let manifest = serde_json::json!({
            "schema_version": 1,
            "document": serde_json::to_value(&doc).unwrap()
        })
        .to_string();
        zip.write_all(manifest.as_bytes()).unwrap();
        zip.start_file("tiles/not-a-node.bin", SimpleFileOptions::default()).unwrap();
        zip.write_all(b"junk").unwrap();
        zip.finish().unwrap();

        let err = load_atl(&path).unwrap_err();
        std::fs::remove_file(&path).ok();
        assert!(matches!(err, AtlError::BadTilePart(_)));
    }

    #[test]
    fn round_trips_vector_shape_and_migrates_old_placeholder() {
        use atelier_core::atelier_vector::{Path, Shape};
        use atelier_core::VectorContent;
        // New-form vector layer round-trips.
        let mut doc = Document::new([16, 16], ProjectFocus::Vector);
        let root = doc.root();
        let content = VectorContent {
            shapes: vec![Shape::filled(Path::rect(1.0, 2.0, 8.0, 9.0), [0.2, 0.4, 0.6, 1.0])],
        };
        let mut add = AddNode::new(
            &mut doc,
            Node::new(LayerProps::named("vec"), NodeKind::Vector(content.clone())),
            root,
            0,
        );
        add.apply(&mut doc);
        let path = temp("vector");
        save_atl(&doc, &path).unwrap();
        let loaded = load_atl(&path).unwrap();
        std::fs::remove_file(&path).ok();
        assert_eq!(doc, loaded);

        // Legacy {bounds,color} vector payload migrates to one filled rect.
        let mut json = serde_json::to_value(&doc).unwrap();
        for node in json["nodes"].as_object_mut().unwrap().values_mut() {
            if let Some(v) = node.get_mut("kind").and_then(|k| k.get_mut("Vector")) {
                *v = serde_json::json!({ "bounds": [1.0, 2.0, 8.0, 9.0], "color": [0.2, 0.4, 0.6, 1.0] });
            }
        }
        let p2 = temp("vector-old");
        let mut zip = ZipWriter::new(File::create(&p2).unwrap());
        zip.start_file(MANIFEST, SimpleFileOptions::default()).unwrap();
        let m = serde_json::json!({ "schema_version": 1, "document": json }).to_string();
        zip.write_all(m.as_bytes()).unwrap();
        zip.finish().unwrap();
        let migrated = load_atl(&p2).unwrap();
        std::fs::remove_file(&p2).ok();
        match &migrated.node(add.id).unwrap().kind {
            NodeKind::Vector(c) => {
                assert_eq!(c.shapes.len(), 1, "migrated to one shape");
                assert_eq!(c.shapes[0].fill, Some([0.2, 0.4, 0.6, 1.0]));
            }
            _ => panic!("vector expected"),
        }
    }

    #[test]
    fn round_trips_layer_mask() {
        use atelier_core::{LayerProps, Mask, NodeKind, RasterContent};
        let mut doc = Document::new([16, 16], ProjectFocus::Raster);
        let root = doc.root();
        let mut content = RasterContent::default();
        content.tiles.fill_rect(0, 0, 16, 16, [10, 20, 30, 255]);
        let mut m = Mask::new();
        for y in 0..16 {
            for x in 0..8 {
                m.set(x, y, 200);
            }
        }
        content.mask = Some(m);
        let mut add = AddNode::new(
            &mut doc,
            Node::new(LayerProps::named("masked"), NodeKind::Raster(content)),
            root,
            0,
        );
        add.apply(&mut doc);
        let path = temp("mask");
        save_atl(&doc, &path).unwrap();
        let loaded = load_atl(&path).unwrap();
        std::fs::remove_file(&path).ok();
        match &loaded.node(add.id).unwrap().kind {
            NodeKind::Raster(c) => {
                let mask = c.mask.as_ref().expect("mask restored");
                assert_eq!(mask.get(3, 3), 200, "masked-in coverage preserved");
                assert_eq!(mask.get(12, 3), 0, "outside mask stays 0");
            }
            _ => panic!("raster expected"),
        }
    }

    #[test]
    fn round_trips_adjustment_layer() {
        use atelier_core::{Adjustment, LayerProps};
        let mut doc = Document::new([16, 16], ProjectFocus::Raster);
        let root = doc.root();
        let adj = Adjustment::Levels { black: 0.1, white: 0.9, gamma: 1.5 };
        let mut add = AddNode::new(
            &mut doc,
            Node::new(LayerProps::named("levels"), NodeKind::Adjustment(adj)),
            root,
            0,
        );
        add.apply(&mut doc);
        let path = temp("adjlayer");
        save_atl(&doc, &path).unwrap();
        let loaded = load_atl(&path).unwrap();
        std::fs::remove_file(&path).ok();
        assert_eq!(doc, loaded);
        match &loaded.node(add.id).unwrap().kind {
            NodeKind::Adjustment(a) => assert_eq!(*a, adj),
            _ => panic!("adjustment layer expected"),
        }
    }

    #[test]
    fn rejects_future_schema_version() {
        let path = temp("future");
        let mut zip = ZipWriter::new(File::create(&path).unwrap());
        zip.start_file(MANIFEST, SimpleFileOptions::default()).unwrap();
        zip.write_all(br#"{"schema_version": 99, "document": {}}"#).unwrap();
        zip.finish().unwrap();
        let err = load_atl(&path).unwrap_err();
        std::fs::remove_file(&path).ok();
        assert!(matches!(err, AtlError::VersionTooNew(99)));
    }

    #[test]
    fn rejects_non_zip_garbage() {
        let path = temp("garbage");
        std::fs::write(&path, b"definitely not a zip").unwrap();
        let err = load_atl(&path).unwrap_err();
        std::fs::remove_file(&path).ok();
        assert!(matches!(err, AtlError::Container(_)));
    }

    /// Spec 0053: a smart object's embedded pixels AND embedded layer mask
    /// survive save→load (schema v3).
    #[test]
    fn round_trips_embedded_smart_object() {
        use atelier_core::{LayerProps, Mask, SmartContent};
        let mut inner = Document::new([32, 32], ProjectFocus::Raster);
        let iroot = inner.root();
        let mut content = RasterContent::default();
        content.tiles.fill_rect(0, 0, 20, 20, [11, 22, 33, 255]);
        let mut m = Mask::new();
        for y in 0..16 {
            for x in 0..10 {
                m.set(x, y, 180);
            }
        }
        content.mask = Some(m);
        let mut add_inner = AddNode::new(
            &mut inner,
            Node::new(LayerProps::named("inner-raster"), NodeKind::Raster(content)),
            iroot,
            0,
        );
        add_inner.apply(&mut inner);

        let mut doc = Document::new([32, 32], ProjectFocus::Raster);
        let root = doc.root();
        let mut add_s = AddNode::new(
            &mut doc,
            Node::new(
                LayerProps::named("smart"),
                NodeKind::Smart(SmartContent { doc: Box::new(inner), offset: [4, 6] }),
            ),
            root,
            0,
        );
        add_s.apply(&mut doc);

        let path = temp("smart");
        save_atl(&doc, &path).unwrap();
        let loaded = load_atl(&path).unwrap();
        std::fs::remove_file(&path).ok();
        assert_eq!(doc, loaded, "embedded pixels+mask round-trip deep-equal");

        let NodeKind::Smart(sc) = &loaded.node(add_s.id).unwrap().kind else {
            panic!("smart expected");
        };
        let (iid, _) = sc.doc.iter_tree().into_iter().next().expect("inner layer");
        let NodeKind::Raster(c) = &sc.doc.node(iid).unwrap().kind else {
            panic!("inner raster expected");
        };
        assert_eq!(c.tiles.pixel(5, 5), [11, 22, 33, 255], "embedded pixel survived");
        assert_eq!(c.mask.as_ref().unwrap().get(3, 3), 180, "embedded mask survived");
    }

    /// Spec 0053: pixels nested two smart-objects deep survive (the dotted-chain
    /// key disambiguates per-doc id spaces).
    #[test]
    fn round_trips_nested_smart_object() {
        use atelier_core::{LayerProps, SmartContent};
        let mut deep = Document::new([16, 16], ProjectFocus::Raster);
        let droot = deep.root();
        let mut content = RasterContent::default();
        content.tiles.fill_rect(0, 0, 8, 8, [200, 100, 50, 255]);
        let mut a = AddNode::new(
            &mut deep,
            Node::new(LayerProps::named("deep-raster"), NodeKind::Raster(content)),
            droot,
            0,
        );
        a.apply(&mut deep);

        let mut mid = Document::new([16, 16], ProjectFocus::Raster);
        let mroot = mid.root();
        let mut b = AddNode::new(
            &mut mid,
            Node::new(
                LayerProps::named("mid-smart"),
                NodeKind::Smart(SmartContent { doc: Box::new(deep), offset: [1, 1] }),
            ),
            mroot,
            0,
        );
        b.apply(&mut mid);

        let mut doc = Document::new([16, 16], ProjectFocus::Raster);
        let root = doc.root();
        let mut c = AddNode::new(
            &mut doc,
            Node::new(
                LayerProps::named("top-smart"),
                NodeKind::Smart(SmartContent { doc: Box::new(mid), offset: [2, 2] }),
            ),
            root,
            0,
        );
        c.apply(&mut doc);

        let path = temp("nested");
        save_atl(&doc, &path).unwrap();
        let loaded = load_atl(&path).unwrap();
        std::fs::remove_file(&path).ok();
        assert_eq!(doc, loaded, "nested smart-in-smart pixels round-trip deep-equal");
    }
}
