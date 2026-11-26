# Animated volume flipbooks

`VolumeFlipbookPart` plays baked scalar density on a genuine 3D voxel grid. It uses
the same recipe → definition → layer → part stack as the capture-plane
`FlipbookPart`. This is spatial density integration, with internal parallax and
opaque-scene intersections; it does not simulate fluid motion or deform around
obstacles.

## Authoring and switching renderers

```rust
use maybraid_vfx::EffectLayer;

let card = EffectLayer::flipbook_path("vfx/dust/rolling_wave.flipbook.json");
let volume = EffectLayer::volume_flipbook_path(
    "vfx/fixtures/volume_puffs/puffs.volume_flipbook.json",
)
.with_name("animated density")
.with_translation(bevy::prelude::Vec3::Y * 0.2)
.with_scale(0.8);
```

Put either layer in an ordinary recipe or `EffectLayerGroup`, and spawn the typed
recipe with `Effect::new(recipe)`. Root placement remains an ECS `Transform`.
`VolumeFlipbookPart::new(VolumeFlipbookRef::path(path))` is the explicit equivalent.
No recipe needs an asset server, renderer mesh, texture handles, or copied duration.
The two dust styles should compose this shared part with their own exported
sequences. The fixture above is generated test data, **not a converted rolling-wave
asset or a production dust style**. Captured angular views cannot be renamed or
reinterpreted as density time samples.

`VfxSpawn::playback` changes time; intensity multiplies extinction; tint supplies
linear RGB and final alpha. Scalar density is otherwise neutral white. Seed remains
on the instance for recipe composition but does not alter baked voxel data. Palette
is available to other parts in the recipe; this unlit density renderer uses tint.

## Export contract

The manifest extension is `.volume_flipbook.json`. See the
[complete fixture manifest](../assets/vfx/fixtures/volume_puffs/puffs.volume_flipbook.json).
All fields are required; unknown fields are rejected.

| Field | Meaning |
| --- | --- |
| `schema_version` | `1` |
| `frames` | 2–64 manifest-relative `.ktx2` paths in chronological order; no traversal |
| `dimensions` | `[width, height, depth]`, each 2–128; identical for every frame |
| `bounds` | Local `min`/`max` AABB enclosing the transformed voxel edges |
| `format` | `"r16_float"`: one linear scalar density channel |
| `density_scale` | Positive extinction multiplier per authored local-space unit |
| `index_to_local` | Column-major affine 4×4 matrix taking integer indices to voxel centers |
| `fps` | Positive finite, uniformly spaced sample rate |
| `end_behavior` | `{"mode":"hold"}` or `{"mode":"fade","seconds":0.5}` within clip duration |

Export finite, nonnegative little-endian float16 density, X fastest, then Y, then Z.
Use a common grid and transform for every frame. Leave density zero around the grid
boundary when a soft edge is wanted. The voxel-edge domain is `[-0.5, dimensions -
0.5]`; sampling coordinates are `(index + 0.5) / dimensions`. Rotated/sheared grid
transforms use the same `StaticVolume` validation and conventions.

Package each frame offline as a **3D R16_SFLOAT KTX2**, linear transfer function,
one mip, no array layers, and either no supercompression or Zstandard. The existing
`encode_volume` / `decode_volume` functions in
[`bin/vfx/export_volumes.py`](../../bin/vfx/export_volumes.py) provide an offline
KTX-Software packing and byte-verification path for raw grids. Do not run the pinned
static-delivery export command on unrelated new artwork. Commit the new manifest,
KTX2 frames, and provenance together; archive large authoring projects separately.

The loader checks each frame's header, dimensions, format, mip count, and decoded
byte count before delegating pixel decoding to Bevy. Total decoded density is
limited to **128 MiB per sequence**, conservatively counting every frame entry.
The entire sequence is resident while referenced. There is no streaming or runtime
repacking. Bevy shares images between instances, and each material binds just the
current two 3D textures—no unsupported 3D texture array is required.

The clip lasts `(frames.len() - 1) / fps`, ending at its final timestamp. Density is
trilinearly filtered spatially, then interpolated between adjacent time samples
before opacity integration. `hold` clamps to the last frame; ordinary instance
cleanup still ends the effect. It does not make the instance immortal.

## Rendering and lifecycle

Add `VfxPlugin` and `DepthPrepass` to the viewing camera. Opaque depth clips each ray
before density accumulation. Without a depth prepass the volume still renders, but
**opaque intersection/occlusion is unavailable**: the projected proxy intentionally
performs no surface-depth test or depth write.

The vertex shader projects the transformed volume AABB to a screen rectangle. It
uses a viewport-sized fallback when the bounds cross the camera plane. A robust
ray/grid-box test rejects empty pixels and supports cameras inside the volume,
perspective and orthographic projection, negative/nonuniform scale, and nested
transforms. Singular transforms are hidden. No root or layer scale is applied twice.

Integration takes at most **96 midpoint steps**, two trilinear density samples per
step, with early termination at 99% accumulated opacity. Step lengths are measured
in authored local units: uniformly enlarging an effect preserves its optical
thickness. There is no stochastic jitter, so a fixed time has stable output.

This first renderer is unlit tinted extinction with alpha compositing. It has no
self-shadowing, scene-light scattering, collision response, or fluid simulation.
Transparent objects and overlapping volumes use ordinary object-level transparency
sorting, not globally interleaved ray integration. Thin features can be undersampled
at the fixed step budget. Large on-screen volumes remain expensive even with tiny
textures.

Loading waits for the complete frame dependency set and image availability before
starting the layer clock. Delayed layers use their actual ready time. Materials and
clocks are per instance; manifests, images, and the proxy mesh are shared. Failure
reports the recipe/layer/path/dependency error and cancels the root. Cancelling a
loading or delayed effect drops its handles without resurrecting entities. A cold
GPU pipeline can still need compilation after CPU asset readiness; warm it once and
reset before inspecting the opening frame.

## Playground and repeatable probe

```sh
cargo run -p vfx-playground -- show volume-demo --seed 7
cargo run -p vfx-playground --example volume_probe -- /tmp/volume-probe
```

The regular playground supports `orbit`, `view 0.1 4.8 0.2` (above),
`view 0.8 1.0 0.6` (inside), repeated `show` or `spread` for concurrent instances,
`pause`, `step 0.25`, `reset`, `cancel`, `solo`, and `mute`. The inspector reports
volume frame interpolation and decoded sequence size. `reload` requests all frame
images and the manifest; reset after loading to rebuild materials. Rust recipe
edits still require recompilation. `screenshot /tmp/volume.png` uses GPU readback.

The `volume_probe` example adds an opaque pillar and captures baseline, outside,
side, above, and inside views. It pauses while loading, advances exactly 0.75 seconds
once ready, hides the UI, and emits median/p95 wall-clock frame intervals after
warmup. Its captures use the same in-engine screenshot command. Output goes outside
the repository. These intervals include CPU scheduling, rendering, and presentation;
they are **not isolated GPU timestamp measurements**.

Regenerate the eight 24³ fixture frames and provenance with:

```sh
python3 bin/vfx/generate_volume_fixture.py
```

The deterministic fixture requires no network, artwork, or KTX command-line install.
It occupies **221,184 bytes (216 KiB) decoded**, plus small container headers, shared
across all instances. Two 3D density samples × 96 steps bounds sampling work at 192
trilinear samples per covered pixel, before early termination.

### Measured development run

2026-11-26: debug build, 960×540, one 24³ × 8 sequence, 96 steps, UI hidden,
VSync disabled, paused at 0.75 s. Vulkan llvmpipe (LLVM 19.1.7), Mesa 25.0.7,
Linux container with five AMD EPYC 9V74 CPU cores and 17.6 GiB reported RAM.
The probe samples a two-second window after four seconds of warmup, before capture.

| View | Sampled frames | Median frame interval | p95 |
| --- | ---: | ---: | ---: |
| Empty playground baseline | 27 | 55.8 ms | 87.1 ms |
| Outside | 9 | 255.8 ms | 275.7 ms |
| Side | 8 | 251.1 ms | 262.3 ms |
| Above | 6 | 296.1 ms | 326.0 ms |
| Inside | 7 | 297.1 ms | 313.5 ms |

The outside view adds about 200 ms over the matching empty-camera baseline on this
**software renderer**. The samples are short and noisy; these numbers establish
that ray marching has substantial cost, not a hardware frame-rate expectation.
Hardware GPU profiling and a production quality/performance budget remain necessary
before widespread use. The probe exited successfully; GPU-readback captures were
reviewed for parallax, opaque intersection, top views, and inside-volume rendering.
