# Atelier (photo-illustration-shop)

A cross-platform, GPU-accelerated desktop image editor that unifies raster editing
(Photoshop-class) and vector illustration (Illustrator-class) in one document model:
PSD/AI interchange, ICC color management, and computer-vision assist tools that run locally
(ONNX Runtime) or against a user-configured cloud endpoint.

> **On "no generative AI":** this is a scope decision, not an ideological one. Generative
> fill / text-to-image would be a large subsystem (model weights, prompt UX, safety,
> licensing) that doesn't serve the app's core job — precise raster+vector editing — so it's
> left out as over-engineering for the goals here. The AI that *is* included is classic CV
> assist (select-subject, background removal, mask refine), which directly speeds up editing.

- **Vision & goals:** [docs/VISION.md](docs/VISION.md)
- **Requirements:** [docs/REQUIREMENTS.md](docs/REQUIREMENTS.md)
- **Architecture:** [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md)
- **Roadmap & status:** [docs/ROADMAP.md](docs/ROADMAP.md)
- **Current state / resume point:** [docs/SESSION-STATE.md](docs/SESSION-STATE.md)

## Status

Active development. Working today: app shell with dockable panels, GPU-composited tiled
canvas (full Photoshop blend-mode set, CPU/GPU bit-exact), layers + groups + undo, brush /
eraser / move / selections / adjustments / adjustment-layers, and a vector engine with
shape tools, pen, and full anchor + bezier-handle editing. See
[docs/ROADMAP.md](docs/ROADMAP.md) for the phase table.

## Prerequisites

You need the **Rust toolchain (stable)**, a **C/C++ linker**, and **working GPU drivers**.

### 1. Rust toolchain (all platforms)

Install via [rustup](https://rustup.rs):

```sh
# macOS / Linux
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# Windows (PowerShell) — or download & run rustup-init.exe from https://rustup.rs
winget install Rustlang.Rustup
```

Then restart your shell and confirm:

```sh
rustc --version   # 1.75+ (stable)
cargo --version
```

`rustup` reads `rust-toolchain.toml` in this repo and pins the stable channel automatically.

### 2. Platform build dependencies

- **Windows:** the MSVC toolchain — install **Visual Studio Build Tools** with the
  "Desktop development with C++" workload (provides `link.exe`). rustup defaults to the
  `x86_64-pc-windows-msvc` target.
- **macOS:** Xcode Command Line Tools — `xcode-select --install`.
- **Linux (Debian/Ubuntu):** build essentials + windowing/GPU headers:

  ```sh
  sudo apt-get update && sudo apt-get install -y \
      build-essential pkg-config \
      libxkbcommon-dev libwayland-dev libgtk-3-dev
  ```

  (Other distros: install the equivalent `gcc`, `pkg-config`, `wayland`, `libxkbcommon`,
  and GTK-3 development packages.)

### 3. GPU drivers

Rendering uses **wgpu** (Vulkan / DX12 / Metal). Install up-to-date GPU drivers:

- Windows: vendor drivers (NVIDIA / AMD / Intel) — DX12 works out of the box.
- Linux: Mesa (Intel/AMD) or the proprietary NVIDIA driver, with Vulkan support installed.
- macOS: Metal ships with the OS.

The app prints the selected adapter to the log on launch; a hardware adapter (not a software
rasterizer) is needed for the GPU golden-parity tests.

## Build, run, test

```sh
cargo run -p atelier-app                       # launch the app
cargo build --workspace                        # build everything
cargo test  --workspace                        # run all tests
cargo clippy --workspace --all-targets -- -D warnings   # lint gate (CI-enforced)

# set RUST_LOG for logs, e.g.:
#   macOS/Linux:  RUST_LOG=info cargo run -p atelier-app
#   Windows PS:   $env:RUST_LOG='info'; cargo run -p atelier-app
```

First build compiles the wgpu/egui stack and takes a few minutes; later builds are
incremental. GPU golden-parity tests skip automatically on machines without a hardware
adapter (e.g. CI software rasterizers).

## Platform support

Tier 1 (release gate): **Windows x86-64**. Tier 2 (built + tested in CI): **macOS Apple
Silicon**, **Linux x86-64**. CI runs the full build/test/clippy on Windows, macOS, and Linux.
See [docs/VISION.md](docs/VISION.md) for the full platform tier table.

## License

Apache-2.0.
