# Mythical chain material

Glowing iron-and-energy surface for Bikanwarda’s restraint chains
([#1124](https://github.com/ramate-io/maybraid/issues/1124)). Durability,
glow, seed, and break/fade are [`MaterialRef`](../../material-ref) bags.
Deterioration is shaded, not remeshed. Per-link hits update an **owned**
runtime uniform; they do not restamp the shared [`MaterialRefKey`](../../material-ref/src/key.rs)
cache.

```text
cargo test -p chain-shaders --all-targets
cargo run -p chain-playground
cargo run -p chain-playground -- show pair
cargo run -p chain-playground -- lighting night
cargo run -p chain-playground -- hit 6
```

Runtime skinned rope/chain geometry is [A04 / #1125](https://github.com/ramate-io/maybraid/issues/1125)
and is not in this crate. Playground previews use
[`placeholder::placeholder_chain_mesh`](src/placeholder.rs).

## Recipe

| Name | Kind |
|------|------|
| `mythical_chain` | Stylized metal + energy veins + deterioration |

Helpers: `mythical_chain_ref`, `mythical_chain_ref_for`, `MythicalChainState`,
`OwnedChainMaterial`, `apply_chain_hit`.

Two chains with different scalars or seeds resolve to different
[`MaterialRefKey`](../../material-ref/src/key.rs) handles. Independently
damaged chains **must** take [`OwnedChainMaterial`](src/runtime.rs): fulfill
still caches the authored look, then clones a unique handle whose
`link_damage[32]` / `impact` uniforms are mutated in place. Never write
runtime damage into a cached handle shared by unrelated chains.

## Fracture atlas

Cracks are a baked, tileable distance field — not FBm ridges.

- Source of truth: [`cracks::bake_atlas_rgba`](src/cracks.rs) (deterministic
  [`hash13`](src/hash.rs) branching walk, four variants).
- Runtime: [`cracks::atlas_image`](src/cracks.rs) uploaded once as
  [`ChainCrackAtlas`](src/cracks.rs).
- Placeholder asset (same bytes): [`assets/crack_atlas.png`](assets/crack_atlas.png).

Regenerate the PNG:

```text
cargo test -p chain-shaders -- cracks::tests::writes_placeholder_png --exact
```

Atlas packing (256×256, 2×2 of 128×128):

| Channel | Meaning |
|---------|---------|
| R | Distance to nearest fissure (`0` on the crack) |
| G | Recessed height for surface-gradient bump |
| B | Cell id for fracture-driven piece removal |
| A | `255` |

Link seed picks a variant. Energy veins occupy that network (not striped
`vein_u`). Damage **widens** the same fissures: hairlines first, then
connected cracks and secondary branches. Each fissure has a dark recessed
center, a narrow bright inner edge, and a wider subdued shoulder. Hits
flash and ember *inside* those cracks; they do not recolor the whole
link. Final holes open along the fissures (and loose cells), not as
hashed speckle.

Sampling uses **link-local** UVs (`around` the tube × `along` the ring,
aspect ≈ 3.05) so chain length does not stretch the network. The
placeholder mesh duplicates U=0/U=1 and ring-seam vertices so interpolation
does not cross the wrap.

Bump mapping uses a Mikkelsen surface-gradient from the height channel
(world-position derivatives), not screen-space noise stuffed into world X/Z.

Actual pieces separating and falling still need geometry / debris VFX
(out of scope).

## Localized damage

| Path | Role |
|------|------|
| `impact` vec4 | Temporary hotspot: along-chain pos, radius, strength, hit time. Shader time decays the flash. |
| `link_damage[32]` | Persistent per-link values, packed as eight `vec4`s, indexed by vertex-color R. |
| Vertex color G + `localized_blend` | Legacy mesh-color blend. Not the supported A06 path. |

On a hit, update the affected link and the hotspot on the **owned**
material. The shader does a direct lookup. No vertex walk, no mesh upload,
no per-fragment hit list.

## Halo / pulse

Gameplay cameras ([`maybraid-world` follow camera](../../../lib/src/camera.rs))
have fog and a depth prepass, **not** bloom. A Fresnel rim on the opaque
surface cannot draw outside the metal silhouette.

- Chain-wide pulse (`chain_pulse`): deep trough, short crest, small
  per-link variation. Drives vein emission, rim, and the halo.
- Additive child shell ([`ChainHalo`](src/halo.rs)): same mesh / skinning,
  expanded along the normal, soft Fresnel, depth-tested, no depth write.
  Opacity and expansion pulse together.
- The opaque pass does **not** call `tone_mapping`, so HDR emission can
  survive if a later camera adds bloom.

## Parameter ranges

Palette (linear-authored sRGB):

| Slot | Role | Default | Notes |
|------|------|---------|-------|
| 0 | Intact metal | `(0.22, 0.16, 0.12)` | Readability under key + fill |
| 1 | Energy glow | `(0.62, 0.90, 1.0)` | Intensity is a scalar, not alpha |
| 2 | Damage / ash | `(0.20, 0.08, 0.05)` | Crack shoulders and rust |
| 3 | Ember / near-break | `(1.0, 0.42, 0.10)` | Hot only when damaged |

Scalars (index meaning is this shader’s contract):

| Index | Name | Range | Default | Intact / damaged / near-break / released |
|-------|------|-------|---------|------------------------------------------|
| 0 | `intact` | `[0, 1]` | `1.0` | `1.0` / `0.55` / `0.18` / `0.0` |
| 1 | `glow_intensity` | `[0, 8]` | `5.2` | authored gain |
| 2 | `fade` (break/release) | `[0, 1]` | `0.0` | `0.0` / `0.0` / `0.15` / `1.0` |
| 3 | `damage` | `[0, 1]` | `0.0` | `0` means derive `1 - intact` |
| 4 | `link_count` hint | `[1, 256]` | `14` | UV.y → link fallback |
| 5 | `crack_frequency` | `[0.5, 24]` | `6.0` | scales link-local crack UVs |
| 6 | `localized_blend` | `[0, 1]` | `0.0` | legacy vertex-color G path |
| 7 | `pulse_rate` | `[0, 8]` | `1.4` | shared pulse frequency |

[`NoiseParams`](../../../procedural/common/src/noise.rs):

| Field | Role | Authored |
|-------|------|----------|
| `seed` | Per-chain identity, variant pick, which links weaken first | `11` |
| `frequency` | Kept for authored identity | `2.4` |
| `amplitude` | Fracture bump strength | `1.0` |
| `octaves` | Unused by the atlas path (FBm rust only) | `2` |

## Geometry contract (A04 / #1125)

The shader is UV-locked and skin-aware. World-space position is used for
lighting and the surface-gradient bump. A04’s skinned mesh must meet:

### UV 0

- **U (`uv.x`)**: around the tube / link cross-section, `[0, 1]`. Duplicate
  the seam (`U=0` and `U=1` on coincident vertices). Do not wrap indices
  on a shared vertex.
- **V (`uv.y`)**: **along the chain**, `[0, 1]`, monotonically increasing
  from start anchor to end anchor. Within link `i` of `N` links, `uv.y`
  occupies `[i/N, (i+1)/N]`. Duplicate the ring seam so the last ring
  parameter is `1` inside the link, not a wrap back to `0`.
- Link-local crack UVs are `vec2(u, v * N - link_id)` with aspect ~3.
- Do not mirror V. Glow flow and fade still read V as a directed length.

### Vertex color (`Mesh::ATTRIBUTE_COLOR`, float4)

| Channel | Meaning | Encoding |
|---------|---------|----------|
| R | Per-link id | `link_index as f32` (0, 1, 2, …). All vertices of one link share the same value so interpolation stays flat. Shader uses `floor(color.r + 0.5)` and as the `link_damage` index. |
| G | Localized damage (legacy) | `[0, 1]`, ignored while `localized_blend == 0`. |
| B | Twist phase | `0` even / `1` odd (alternating link orientation). |
| A | Reserved | `1` |

If color is missing, the shader falls back to
`floor(uv.y * link_count)` using scalar 4.

### Skinning

The vertex stage uses Bevy `#ifdef SKINNED` (same path as clothing). The
halo child uses the same mesh and therefore the same skinning.

### Damage parameter input

Authored look (A06 global durability) still restamps a per-chain
[`MaterialRef`](../../material-ref):

1. `intact` = remaining durability in `[0, 1]`.
2. `fade` = release / dissolve progress.
3. Optional `damage` override; leave `0` to derive `1 - intact`.
4. Keep `seed` stable.
5. Do not rebuild the mesh on hit.

**Localized hits** (the supported path for a chain of tens of links):

1. Insert [`OwnedChainMaterial`](src/runtime.rs) on the chain entity.
2. After fulfill, write `apply_chain_hit` on that entity’s unique
   [`Handle<ChainMaterial>`](src/chain_material.rs).
3. Persistent damage is `link_damage[link_id]`. The temporary flash is
   `impact` + `globals.time`.

Weak links (higher `hash13(vec3(id * 17.13, seed * 0.013, 4.2))`)
still deteriorate first under global durability.

### Out of scope here

- Armature, bind pose, capsule collision (A04 / A05).
- Hit routing and the broken signal (A06).
- Final Blender hero mesh. The PNG atlas is a baked placeholder; bots do
  not produce final Blender assets.
- Geometry debris when pieces separate.

## Authored states

| State | `intact` | `fade` | Look |
|-------|----------|--------|------|
| Intact | `1.00` | `0.00` | Clean metal, full veins, hairline-ready network |
| Damaged | `0.55` | `0.00` | Widened fissures, rust near cracks, dimmer glow |
| Near-breaking | `0.18` | `0.15` | Ember in recesses, pieces opening along fractures |
| Released | `0.00` | `1.00` | Fracture-driven dissolve / leftover sparks |

## Playground

`chain-playground` shows two independently parameterized chains (left is
owned so `/hit` is in-place), a durability sweep, CPU sine-bend, day/night
lighting, and Sharp / Retro / Antediluvian window sizes.
