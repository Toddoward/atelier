//! Golden parity: GPU compositor must match the CPU reference within 1 LSB
//! (8-bit) on identical documents (D-9, spec 0004). Requires a real adapter;
//! skips with a notice when none exists (CI software runners).

use atelier_core::command::AddNode;
use atelier_core::{
    BlendMode, Command, Document, LayerProps, Node, NodeId, NodeKind, ProjectFocus,
    RasterContent,
};
use atelier_gpu::GpuCompositor;

/// Tiny deterministic LCG so fixtures are stable without a rand dep.
struct Lcg(u64);

impl Lcg {
    fn next_u32(&mut self) -> u32 {
        self.0 = self.0.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
        (self.0 >> 33) as u32
    }
    fn unit_f32(&mut self) -> f32 {
        (self.next_u32() % 1000) as f32 / 999.0
    }
    fn range(&mut self, n: u32) -> u32 {
        self.next_u32() % n
    }
}

fn gpu() -> Option<(wgpu::Device, wgpu::Queue)> {
    let instance = wgpu::Instance::default();
    let adapter =
        pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions::default()))?;
    let info = adapter.get_info();
    if info.device_type == wgpu::DeviceType::Cpu {
        // Software rasterizers (WARP/llvmpipe on CI) are not the parity target:
        // FXC chokes on the composite shader and software fp would defeat the
        // bit-exact assertion anyway. Parity is a hardware-truth gate (D-9).
        eprintln!("SKIP: software adapter \"{}\" — parity runs on real GPUs only", info.name);
        return None;
    }
    let (device, queue) = pollster::block_on(
        adapter.request_device(&wgpu::DeviceDescriptor::default(), None),
    )
    .ok()?;
    eprintln!("golden parity on adapter: {}", info.name);
    Some((device, queue))
}

fn add(doc: &mut Document, node: Node, parent: NodeId) -> NodeId {
    let mut cmd = AddNode::new(doc, node, parent, 0);
    cmd.apply(doc);
    cmd.id
}

fn random_layer(rng: &mut Lcg, name: &str, w: u32, h: u32) -> Node {
    let mut content = RasterContent {
        // Random layer offset (move tool, spec 0005) — exercises shifted sampling.
        offset: [rng.range(129) as i32 - 64, rng.range(129) as i32 - 64],
        ..Default::default()
    };
    // 1-3 random filled rects with random colors/alphas.
    for _ in 0..(1 + rng.range(3)) {
        let x0 = rng.range(w) as i32;
        let y0 = rng.range(h) as i32;
        let x1 = (x0 + 1 + rng.range(w) as i32).min(w as i32);
        let y1 = (y0 + 1 + rng.range(h) as i32).min(h as i32);
        let color = [
            (rng.range(256)) as u8,
            (rng.range(256)) as u8,
            (rng.range(256)) as u8,
            (64 + rng.range(192)) as u8,
        ];
        content.tiles.fill_rect(x0, y0, x1, y1, color);
    }
    Node::new(LayerProps::named(name), NodeKind::Raster(content))
}

/// Every mode except PassThrough (group-only), cycled deterministically.
fn mode_for(i: usize) -> BlendMode {
    let modes: Vec<BlendMode> =
        BlendMode::ALL.into_iter().filter(|m| *m != BlendMode::PassThrough).collect();
    modes[i % modes.len()]
}

fn random_doc(seed: u64, w: u32, h: u32) -> Document {
    let mut rng = Lcg(seed);
    let mut doc = Document::new([w, h], ProjectFocus::Raster);
    let root = doc.root();
    let mut mode_i = seed as usize;

    for li in 0..3 {
        let id = add(&mut doc, random_layer(&mut rng, &format!("l{li}"), w, h), root);
        let node = doc.node_mut(id).unwrap();
        node.props.blend = mode_for(mode_i);
        node.props.opacity = 0.25 + 0.75 * rng.unit_f32();
        mode_i += 1;
    }
    // One isolated group with two layers, one nested pass-through group inside.
    let g = add(&mut doc, Node::group("g"), root);
    {
        let node = doc.node_mut(g).unwrap();
        node.props.blend = mode_for(mode_i);
        mode_i += 1;
        node.props.opacity = 0.5 + 0.5 * rng.unit_f32();
    }
    add(&mut doc, random_layer(&mut rng, "g/a", w, h), g);
    let pt = add(&mut doc, Node::group("g/pt"), g); // PassThrough default
    let id = add(&mut doc, random_layer(&mut rng, "g/pt/b", w, h), pt);
    let node = doc.node_mut(id).unwrap();
    node.props.blend = mode_for(mode_i + 1);
    node.props.opacity = 0.25 + 0.75 * rng.unit_f32();
    doc
}

fn assert_parity(label: &str, cpu: &[u8], gpu_out: &[u8]) {
    assert_eq!(cpu.len(), gpu_out.len(), "{label}: length");
    let mut worst = 0u8;
    let mut diffs = 0usize;
    for (i, (a, b)) in cpu.iter().zip(gpu_out).enumerate() {
        let d = a.abs_diff(*b);
        if d > 0 {
            diffs += 1;
            worst = worst.max(d);
            assert!(
                d <= 1,
                "{label}: byte {i} differs by {d} (cpu {a} vs gpu {b})"
            );
        }
    }
    eprintln!("{label}: {diffs} bytes off by 1, worst {worst}");
}

#[test]
fn gpu_matches_cpu_reference_within_1_lsb() {
    let Some((device, queue)) = gpu() else {
        eprintln!("SKIP: no GPU adapter available (expected on CI software runners)");
        return;
    };
    let compositor = GpuCompositor::new(&device);

    // Cover all 27 layer-applicable modes across seeds (4 mode slots per doc,
    // advancing by seed), single-tile and tile-spanning sizes.
    for seed in 0..8u64 {
        let (w, h) = if seed % 2 == 0 { (64, 64) } else { (300, 280) };
        let doc = random_doc(seed.wrapping_mul(0x9E37) + seed, w, h);
        let cpu = atelier_raster::composite_rgba8(&doc, w, h);
        let gpu_out = compositor.composite_rgba8(&device, &queue, &doc, w, h);
        assert_parity(&format!("seed {seed} {w}x{h}"), &cpu, &gpu_out);
    }
}

#[test]
fn dissolve_parity_is_exact() {
    let Some((device, queue)) = gpu() else {
        eprintln!("SKIP: no GPU adapter available");
        return;
    };
    let compositor = GpuCompositor::new(&device);

    let mut doc = Document::new([64, 64], ProjectFocus::Raster);
    let root = doc.root();
    let mut content = RasterContent::default();
    content.tiles.fill_rect(0, 0, 64, 64, [0, 255, 0, 255]);
    let id = add(&mut doc, Node::new(LayerProps::named("d"), NodeKind::Raster(content)), root);
    let node = doc.node_mut(id).unwrap();
    node.props.blend = BlendMode::Dissolve;
    node.props.opacity = 0.5;

    let cpu = atelier_raster::composite_rgba8(&doc, 64, 64);
    let gpu_out = compositor.composite_rgba8(&device, &queue, &doc, 64, 64);
    assert_eq!(cpu, gpu_out, "dissolve hash must match bit-exactly");
}
