# Maybraid VFX Playground

Isolated host for [`maybraid-vfx`](../lib). Spawn the composed fiery explosion or any single layer.

```text
cargo run -p vfx-playground -- fiery-explosion
cargo run -p vfx-playground -- show fireball
cargo run -p vfx-playground -- show dust --playback 0.5
cargo run -p vfx-playground -- fiery-explosion --scale 1.5 --intensity 1.2 --seed 4 --playback 0.5
```

In-game (press `/`):

- `fiery-explosion` — all four layers (flash, fireball, smoke, sparks)
- `show` — same as `fiery-explosion`
- `show flash|fireball|smoke|sparks|dust` — one layer
- `--distance` — offset along +Z; `spread` places near / mid / far
- `--seed` — reproduce an instance; omit to generate a new one
- `--playback` — 1.0 is authored tempo; `0.5` is half speed, `2` is twice as fast
- `orbit` — orbit the camera around the burst
- `freeze 0.4` — pause once the instance reaches that age; `play` resumes
- `pause` — freeze all current instances immediately; `play` resumes
- `reset` — cancel current instances and replay the last request from frame zero
- `cancel` — remove all instances and pending/repeating playback
- `noloop` — stop repeating the last show
- The last effect loops so a one-shot burst can be judged

The scene includes a warm reference ground, an occluding wall, and a box so orbiting can show depth and occlusion.

`show dust` selects the baked rolling-wave dust preset. It spawns at the ground pivot;
its 0.55 m capture-center offset is part of the asset. Use `freeze 0.8` and `orbit`
to inspect its shape, or `spread` / repeated `show dust` commands to compare concurrent
instances. Each instance has its own clock and material uniforms; the eight-view
KTX2 texture and standard proxy mesh are shared. `--scale` supplies the root
`Transform`; `--playback` is an effect setting. The preset constructs its recipe
without runtime resources and resolves its manifest path only when realized. Playback ends at the last authored timestamp, 3.2 seconds.

The dust interpolates adjacent times and neighboring azimuth captures in premultiplied
linear color. Its views were authored at 20° elevation: orbiting horizontally is the
intended inspection path, and steep overhead/underside views remain an approximation.
The renderer projects two fixed capture planes, not a simulated volume. The playground
camera uses Bevy's `DepthPrepass`: each plane is rejected against opaque scene depth
before angular blending, with the original 0.18 m contact fade. Other hosts should add
`DepthPrepass` to their camera for this behavior. Without it, the material falls back to
one alpha-weighted hardware depth for the blended pair; intersecting geometry can then
occlude the pair incorrectly. The shared camera-facing proxy can clip at very close
camera distances. It casts no shadow and does not react to scene lighting or obstacles.

A new instance waits for its manifest and image dependencies before advancing. GPU
pipeline compilation is asynchronous; a cold first show can still miss visible opening
frames while its pipeline warms up. Replay with `reset` after the first load when
comparing a particular frame; no guessed startup delay is applied.

Repeating playback waits for actual instance completion, including asset readiness
and delayed layers, then pauses 0.45 seconds before replaying. No duration metadata
is duplicated in the recipe or playground.

## Add a recipe and inspect its layers

See the [migration guide](../MIGRATION.md) for a complete typed recipe example.
Register its factory on your playground app (before or after adding the plugin):

```rust,ignore
use vfx_playground::VfxPreviewAppExt;
app.register_vfx_preview("impact", || Impact { size: 0.8 });
```

Then use `/show impact`. Custom names match exactly and take precedence over builtin
aliases. Factories run once per spawn/rebuild, and the spawned root keeps the typed
recipe component. Registration is only for this preview host; game spawning still
uses `Effect::new(recipe)` directly.

The scrollable layer panel inspects the most recently spawned preview, including
delayed layers. Use the mouse wheel over it to scroll; F1 hides both panels.
It reports pending/loading/playing/finished and the flipbook's two interpolated
frames. Completed state remains visible until the next preview. Load errors include
recipe, layer index/label, manifest path, and dependency error, and stop replay.

- `solo 1` — rebuild with only authored layer 1; clears mute selections.
- `mute 0` — rebuild without layer 0. Indices stay those in the full definition.
- `unsolo` / `unmute` — clear the corresponding selection and rebuild.
- `pause`, then `step [seconds]` — advance once while staying paused (default 1/60 s).
  Time steps are in simulation seconds; the effect's playback multiplier still applies.
  This is forward simulation, not arbitrary seeking or GPU particle rewind.
- `reset` — call the factory again with the last request's seed and current selection.
- `show ...` — clear selections and preview the requested effect.
- `reload` — request reloads of the preview's flipbook manifests and images. After
  loading finishes, `reset` recreates the recipe and materials. Rust edits require
  recompilation. No filesystem watcher or Rust hot-code loader is enabled.

Solo and mute exclude layers before realization, including their lights and particles.
They do not simply hide already-running entities. All preview controls use the shared
VFX runtime; they introduce no game/world dependencies.

## Animated volume preview

`show volume-demo` loads a generated eight-frame 24³ density sequence. It uses the
shared `VolumeFlipbookPart`, not a special dust renderer. See the
[volume guide](../VOLUME_FLIPBOOK.md) for export details and the reproducible
`volume_probe` example, including frame cost and GPU-readback captures.

Use `view 0.1 4.8 0.2` to look down, `view 0.8 1.0 0.6` to look from inside the
volume, and `view 2.4 1.6 3.2` to restore the default viewpoint. These commands stop
automatic orbit. `screenshot /tmp/volume.png` captures through the renderer.
Volume layers participate in the existing selection, playback, reload, and failure
workflows; the inspector also reports their interpolated frame pair and decoded size.

## SurrealistDust

```sh
cargo run --locked -p vfx-playground -- show surrealist-dust --seed 13
```

`effects::surrealist_dust::SurrealistDust` combines a 4.7-second animated density
clip with 28 sparse grain tracks. Compile its shared grain assets once using
`SurrealistDust::compile(&mut World)`, retain the recipe, and clone it when spawning
through `Effect::new(recipe)`. Root ECS `Transform` controls placement. Add
`DepthPrepass` to the viewing camera for opaque-scene occlusion.

Use `noloop` for one playback. Normal camera, pause, reset, selection and cancellation
controls apply. `view 0.8 0.6 0.6` moves inside the volume.

The clip contains 48 R16Float frames at 128×64×128 and 10 samples per second,
occupying 96 MiB decoded and about 101 MB on disk. Images are shared across
instances; clocks and materials are independent. Seed 13 is baked. The recipe
uses sand tint and optional directional density probes for contrast.

This integration demo uses spatial/time interpolation, simplified ballistic grains,
object-level transparency sorting and a 0.1-second terminal fade.

### Offline export

Use a reference checkout at `070e97eed4d94c8c06377c2286ce89895d38d696` with its
pinned Rust toolchain and a working native GPU/window environment. NumPy is needed
only by the offline packaging and validation tools.

```sh
REFERENCE="/path/to/reference checkout"
python3 bin/vfx/run_reference_export.py --reference-checkout "$REFERENCE" --output target/surrealist-export/sequence
python3 bin/vfx/run_reference_export.py --reference-checkout "$REFERENCE" --compare --output target/surrealist-export/comparison
python3 -m venv target/surrealist-export/env
target/surrealist-export/env/bin/pip install numpy==2.0.2
target/surrealist-export/env/bin/python bin/vfx/package_surrealist.py target/surrealist-export/sequence --output-dir target/surrealist-export/assets --grain-output target/surrealist-export/grains.rs
target/surrealist-export/env/bin/python bin/vfx/check_surrealist.py --assets-dir target/surrealist-export/assets --readbacks target/surrealist-export/sequence --comparison-readbacks target/surrealist-export/comparison --report target/surrealist-export/validation.json
rustfmt --edition 2024 --config-path rustfmt.toml target/surrealist-export/grains.rs
```

The wrapper validates the reference commit and resolves artifacts from Cargo build
output. `--validate-only` checks reference identity; `--target-dir PATH` overrides
the build directory. The reader samples the composed GPU density field. Packaging
resamples it onto a common grid and writes relative frame paths and source hashes.

Native capture and instance-cleanup examples accept an output directory:

```sh
cargo run --locked -p vfx-playground --example surrealist_probe -- /tmp/surrealist-views
cargo run --locked -p vfx-playground --example surrealist_instances -- /tmp/surrealist-instances
```

The examples use inspection clocks and provide no performance benchmark.
