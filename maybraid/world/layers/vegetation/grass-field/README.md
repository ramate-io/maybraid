# GPU-driven grass fields

Working field renderer for moderate-height grass plus the reusable WESL
vegetation library in [`vegetation-shaders`](../shaders/README.md). Tracks
[#1637](https://github.com/ramate-io/maybraid/issues/1637).

Existing terrain POM turf (~9.5 cm) and authored frond/tuft plants stay in
place. This crate fills the gap with GPU-selected tapered ribbons.

## Architecture

```text
CPU GrassPatch (bounds, seed, species, surface samples)
        │ compact BladeDescriptor[]  (no per-blade ECS, no expanded mesh)
        ▼
persistent GPU storage  ──►  compute select (frustum, nested density, LOD)
                            ──►  compute finalize (draw_indexed_indirect)
                            ──►  Opaque3d ribbons (near / mid / card templates)
```

| Piece | Role |
| --- | --- |
| `GrassPatch` | CPU ownership. SDF/height samples, conservative AABB including wind. |
| `GrassField` | Compact descriptor + patch-header snapshot. Generation bumps on change. |
| `GrassSelector` | CPU mirror of `select.wesl`. Tests drive this path end to end. |
| `select.wesl` | Per-view compaction into three LOD lists. Rank overflow is deterministic. |
| `finalize.wesl` | Writes 20-byte `DrawIndexedIndirect` args. No CPU readback. |
| `blade.wesl` | Vertex-generated ribbons importing `curve` / `wind` / `shading`. |
| `GrassPatchLod` | Thin adapter onto existing `LodSceneLevel`. UltraLow emits no blades. |

Shadow policy follows [`CanopyShadowMode`](../components/src/shadow_proxy.rs):
default **proxy** does not put grass in shadow views; playground **full** does.
Camera-visible lists are not reused for off-camera casters.

Adapter fallback: `GpuDriven` when compute shaders exist, otherwise `CpuSelect`
(same math, `draw_indexed`). `Unsupported` is reserved.

## WESL modules (reusable, including [#1638](https://github.com/ramate-io/maybraid/issues/1638))

Imported as `vegetation_shaders::wesl::<stem>`:

| Module | Grass | Bush follow-up |
| --- | --- | --- |
| `hash` | plant id, density rank | same |
| `curve` | rest curve, ribbon, `stable_frame` | `attachment_frame` at branch `t` |
| `wind` | stiffness ≈ 0 | higher stiffness for wood |
| `lod` | projected height at actual resolution | same budgets |
| `shading` | painterly wrap + root darkening | wood/foliage palettes |
| `layout` | `BladeDescriptor`, view uniform, indirect | extra branch fields in the bush crate |

Do not copy these functions into grass/bush entry points.

## Playground

```bash
nix develop --command cargo run -p vegetation-grass-field-playground --release --locked
```

In-game `/` commands: `spawn meadow|clump|slope|crest`, `camera orbit|overhead|ground`,
`wind still|gentle|set`, `lod host high|medium|low|ultra-low`, `shadows proxy|full|off`,
`compare turf on|off`.

Captures (BotW meadow lighting):

```bash
MAYBRAID_CAPTURE_DIR=maybraid/world/layers/vegetation/grass-field-playground/captures \
  nix develop --command cargo run -p vegetation-grass-field-playground --release --locked
```

See the [playground README](../grass-field-playground/README.md).

## Tests

```bash
PRE_COMMIT_DISABLE=true nix develop --command cargo test -p vegetation-shaders --release --locked
PRE_COMMIT_DISABLE=true nix develop --command cargo test -p vegetation-grass-field --release --locked
PRE_COMMIT_DISABLE=true nix develop --command cargo test -p vegetation-grass-field-playground --release --locked
```
