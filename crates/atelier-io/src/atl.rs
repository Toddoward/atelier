//! `.atl` v0: ZIP container with a single `manifest.json` holding the
//! schema-versioned document. See docs/FORMAT-ATL.md.

use atelier_core::Document;
use serde::{Deserialize, Serialize};
use std::fs::File;
use std::io::{Read, Write};
use std::path::Path;
use zip::write::SimpleFileOptions;
use zip::{ZipArchive, ZipWriter};

pub const SCHEMA_VERSION: u32 = 0;
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
    zip.start_file(MANIFEST, SimpleFileOptions::default())?;
    zip.write_all(serde_json::to_string_pretty(&manifest)?.as_bytes())?;
    zip.finish()?;
    Ok(())
}

pub fn load_atl(path: &Path) -> Result<Document, AtlError> {
    let mut zip = ZipArchive::new(File::open(path)?)?;
    let mut raw = String::new();
    zip.by_name(MANIFEST)?.read_to_string(&mut raw)?;
    let manifest: Manifest = serde_json::from_str(&raw)?;
    if manifest.schema_version > SCHEMA_VERSION {
        return Err(AtlError::VersionTooNew(manifest.schema_version));
    }
    Ok(serde_json::from_value(manifest.document)?)
}

#[cfg(test)]
mod tests {
    use super::*;
    use atelier_core::command::AddNode;
    use atelier_core::{Command, LayerProps, Node, NodeKind, PlaceholderArt, ProjectFocus};

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
            NodeKind::Raster(PlaceholderArt { bounds: [1.0, 2.0, 30.0, 40.0], color: [0.5; 4] }),
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
}
