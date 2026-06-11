# Atelier (photo-illustration-shop)

A cross-platform, GPU-accelerated desktop image editor unifying raster editing
(Photoshop-class) and vector illustration (Illustrator-class) in one document model.
PSD/AI interchange, ICC color management, local (ONNX) or user-configured cloud CV-AI
assist tools — no generative AI.

- **Vision & goals:** [docs/VISION.md](docs/VISION.md)
- **Requirements:** [docs/REQUIREMENTS.md](docs/REQUIREMENTS.md)
- **Architecture:** [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md)
- **Roadmap & status:** [docs/ROADMAP.md](docs/ROADMAP.md)
- **Current state / resume point:** [docs/SESSION-STATE.md](docs/SESSION-STATE.md)

## Build

Rust stable (rustup) + platform GPU drivers. On Windows: MSVC Build Tools.

```sh
cargo run -p atelier-app
```

Status: **Phase 0 (bootstrap)** — see the roadmap.
