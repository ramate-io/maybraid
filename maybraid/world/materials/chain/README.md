# Mythical chain material

Painterly iron-and-energy surface for Bikanwarda’s restraint chains
([#1124](https://github.com/ramate-io/maybraid/issues/1124)). Durability,
glow, seed, flow, and break/fade are [`MaterialRef`](../../material-ref) bags.
Structural loss is **pre-partitioned chunk removal**, not remeshing. Per-link
hits update an **owned** runtime uniform; they do not restamp the shared
[`MaterialRefKey`](../../material-ref/src/key.rs) cache.

```text
cargo test -p chain-shaders --all-targets
cargo run -p chain-playground
cargo run -p chain-playground -- show pair
cargo run -p chain-playground -- lighting night
cargo run -p chain-playground -- hit 6
```

Runtime skinned rope/chain geometry is [A04 / #1125](https://github.com/ramate-io/maybraid/issues/1125)
and is not in this crate. Playground previews use
[`placeholder::placeholder_chain_mesh`](src/placeholder.rs) and
[`string::continuous_tube_mesh`](src/string.rs).

## Recipe

| Name | Kind |
|------|------|
| `mythical_chain` | Quiet metal + traveling turquoise energy + staged chunk loss |

Helpers: `mythical_chain_ref`, `mythical_chain_ref_for`, `MythicalChainState`,
`ChainFlowStyle`, `OwnedChainMaterial`, `apply_chain_hit`.

Two chains with different scalars or seeds resolve to different
[`MaterialRefKey`](../../material-ref/src/key.rs) handles. Independently
damaged chains **must** take [`OwnedChainMaterial`](src/runtime.rs): fulfill
still caches the authored look, then clones a unique handle whose
`link_damage[32]` / `impact` uniforms are mutated in place. Never write
runtime damage into a cached handle shared by unrelated chains.

## Visual language

The look follows the storyboard’s painterly, Breath-of-the-Wild-inspired
register ([`maybraid/storyboard/README.md`](../../../storyboard/README.md)):
broad quiet masses, a few readable tone regions, rounded highlights. Starting
PBR (tuned in scene lighting, not as acceptance numbers):

| Term | Authored |
|------|----------|
| Roughness | `0.68` (higher on broken lips) |
| Metallic | `0.22` |
| Reflectance | `0.16` |
| Bump | `0.10 * amplitude`, applied **before** writing `N` / `world_normal` |
| Metal | blue-gray bronze |
| Energy | saturated turquoise body + a narrower pale highlight + a subdued halo |

Shaping uses the scene’s PBR lights. There is no private sun and no unlit
toon pass. Orange heat is an optional lip accent at late severity.

## Structural damage

Decorative energy conduits are **not** fractures.

Each link is pre-partitioned into six irregular Voronoi regions in
link-local rest UV (`3` around the tube × `2` along the link; U wraps).
A region has a stable damage threshold from `hash13(chunk, seed, link)`:

| Rank | Threshold | Stage |
|------|-----------|-------|
| 0–1 | `0.20` / `0.28` | Early chips |
| 2–3 | `0.46` / `0.55` | Mid bites (obvious by 50%) |
| 4 | `0.76` | Late gap / broken end |
| 5 | `0` | Reserved until release |

**Cost:** six site hashes plus a wrap-aware nearest / second-nearest search
per fragment. No atlas, no per-hit mesh upload, no debris. Region ids are
computed from UV so they never interpolate. Distance-to-edge is filtered
only for broken-lip shading.

Severity is a bounded union:

```text
severity = max(global_severity(effective_damage, weakness), local_persistent)
```

Flash and transport energy never enter severity. Weakness may shift global
onset, but a local `1.0` is terminal for every seed. Increasing damage never
restores a removed region. Pattern is fixed in rest space while the mesh
bends. Full `fade = 1` discards body and halo (discard creates no solid
interior faces — backsides are double-sided and darkened).

The superseded crack-distance atlas was deleted. It stored only nearest
distance, fattened lines, and hashed Voronoi identities that interpolated.

## Traveling energy

`s` is rest-space arc length in **metres** along the meshed string, stored on
[`Mesh::ATTRIBUTE_UV_1`](https://docs.rs/bevy/0.19.1/bevy/mesh/struct.Mesh.html#associatedconstant.ATTRIBUTE_UV_1)
(`.x = flow_s_m`, `.y = total rest length`). Link-local `uv.y` is **not**
distance (it is link index + angle). World Y is wrong for a bent string.

```text
phase     = TAU * (s - speed_mps * time) / wavelength_m
broad     = 0.5 + 0.5 * cos(phase)
envelope  = smoothstep(0.05, 0.95, broad)
energy    = mix(baseline, max(peak, baseline), envelope)
```

The same `energy` lights conduits, broken lips, and the halo (once — no
repeated envelope multiply). Missing material stays absent in the halo.

| `speed_mps` | Behaviour |
|-------------|-----------|
| `> 0` | Travels toward `+s` |
| `< 0` | Reverse |
| `0` | Standing spatial wave; time does not move the crest |

Defaults: `1.25 m/s`, `2.2 m` wavelength (~2.5 s per transit on the 3.1 m
playground chain). Hit flash is a short smooth rise/decay; it does not reset
flow or enter severity.

### Chain vs tube `s`

- **Continuous tube:** every vertex of a ring shares the ring’s cumulative
  rest arc length. Flow is continuous.
- **Placeholder chain:** `s` is the rest-space Y of the vertex (projection
  onto the hanging centerline) *before* bend. Within a link this spans the
  torus extent along Y — not a single value per link, and not a geodesic
  around the torus tube.

## Halo / renderer

Authored `halo_width_m` (default `0.012 m`) is independent of incidental
scene units. Expansion is nearly fixed (`width * (0.94 + 0.08 * energy)`).
The halo fragment is an early branch: visibility, flow, Fresnel. Depth
tested, no depth writes, additive, no full-screen bloom.

Sync is driven by parent handle / mesh / skin changes and
`AssetEvent::Modified` on the **parent** material (`apply_chain_hit` mutates
the `Assets` entry in place, so `Changed<MeshMaterial3d>` is not enough).
Writes to the halo asset are ignored. Settled chains stop modifying
materials. The child copies the parent’s [`SkinnedMesh`](https://docs.rs/bevy/0.19.1/bevy/mesh/skinning/struct.SkinnedMesh.html)
(joints + inverse bindposes). Removing `ChainHalo` despawns the child.
Culling AABB is expanded by the authored width.

The opaque pass does **not** call `tone_mapping` so HDR emission can share
the camera’s atmosphere / a later bloom. Fog comes from the view’s PBR path.

## Localized damage

| Path | Role |
|------|------|
| `impact` vec4 | Temporary hotspot: along-chain pos, radius, strength, hit time. Shader time decays the flash. |
| `link_damage[32]` | Persistent per-link values, packed as eight `vec4`s, indexed by vertex-color R. |
| Vertex color G + `localized_blend` | Legacy mesh-color blend. Not the supported A06 path. |

On a hit, update the affected link and the hotspot on the **owned**
material. The shader does a direct lookup. No vertex walk, no mesh upload.

## Parameter ranges

Palette (linear-authored sRGB):

| Slot | Role | Default |
|------|------|---------|
| 0 | Intact metal | `(0.40, 0.36, 0.32)` |
| 1 | Energy glow | `(0.06, 0.76, 0.70)` |
| 2 | Ash / broken lip | `(0.26, 0.22, 0.20)` |
| 3 | Ember accent | `(0.70, 0.30, 0.10)` |

Scalars (`MythicalChainScalars::to_array` writes 14 floats; `from_slice`
keeps flow defaults when the slice is the old 8-float recipe):

| Index | Name | Default |
|-------|------|---------|
| 0 | `intact` | `1.0` / `0.50` / `0.22` / `0.0` |
| 1 | `glow_intensity` | `2.2` |
| 2 | `fade` | `0` except released `1` |
| 3 | `damage` | `0` → derive `1 - intact` |
| 4 | `link_count` | `14` |
| 5 | `conduit_frequency` | `2.2` |
| 6 | `localized_blend` | `0` |
| 7 | reserved | `0` (old pulse rate is ignored) |
| 8 | `flow.speed_mps` | `1.25` (signed) |
| 9 | `flow.wavelength_m` | `2.2` |
| 10 | `flow.baseline` | `0.28` |
| 11 | `flow.peak` | `1.0` |
| 12 | `flow.halo_width_m` | `0.012` |
| 13 | `flow.halo_gain` | `0.48` |

Hit timestamps and `link_damage` are runtime-only.

## Geometry contract (A04 / #1125)

The shader is UV-locked and skin-aware. World-space position is used for
lighting and the surface-gradient bump. A04’s skinned mesh must meet:

### UV 0

- **U (`uv.x`)**: around the tube, `[0, 1]`. Duplicate the seam.
- **V (`uv.y`)**: along the chain, `[0, 1]`. Link `i` of `N` occupies
  `[i/N, (i+1)/N]`. Duplicate the ring seam.

### UV 1 (`Mesh::ATTRIBUTE_UV_1`)

- **`.x`**: rest-space arc length in metres (`flow_s_m`).
- **`.y`**: total rest length in metres.
- Continuous strings: one `s` per ring, shared by every vertex of that ring.
- Links: project onto the rest centerline (see above). Do not claim
  continuous flow if only one value is written per link.

### Vertex color (`Mesh::ATTRIBUTE_COLOR`, float4)

| Channel | Meaning |
|---------|---------|
| R | Per-link id (`link_index as f32`). `floor(color.r + 0.5)` indexes `link_damage`. |
| G | Localized damage (legacy). Ignored while `localized_blend == 0`. |
| B | Twist phase. |
| A | Reserved (`1`). |

### Skinning contract (for A04)

The vertex stage uses Bevy `#ifdef SKINNED`. A halo child **must** receive a
copy of the parent’s `SkinnedMesh` (same `joints` and `inverse_bindposes`
handle) or the shell will not follow the bind pose. Halo systems already
copy and propagate `Mesh3d` / `SkinnedMesh` changes, expand the AABB, and
despawn the child when `ChainHalo` is removed. This crate does not generate
runtime chain meshes.

### Damage parameter input

1. Insert [`OwnedChainMaterial`](src/runtime.rs).
2. After fulfill, write `apply_chain_hit` on that entity’s unique handle.
3. Authored restamps (`intact` / `fade` / `flow`) go through `MaterialRef`
   and `apply_authored`; runtime `link_damage` / `impact` persist.

### Out of scope here

- Armature, bind pose, capsule collision (A04 / A05).
- Hit routing and the broken signal (A06).
- Geometry debris, particles, detached-piece animation, runtime mesh cutting.

## Authored states

| State | `intact` | `fade` | Look |
|-------|----------|--------|------|
| Intact | `1.00` | `0.00` | Quiet metal, traveling energy, no missing chunks |
| Damaged | `0.50` | `0.00` | Obvious bites, energy on remaining material |
| Near-breaking | `0.22` | `0.00` | Substantial gaps; reserved plates remain |
| Released | `0.00` | `1.00` | Body and halo discarded |

## Playground

`chain-playground` shows two independently parameterized chains (left is
owned so `/hit` is in-place), a continuous bent tube, a posed skinned tube
that proves halo skinning, a mundane wood cube, day/night lighting, and the
real Native / Sharp / Retro / Antediluvian offscreen present path
(`FollowCamera` + [`VisualStylePlugin`](../../../lib/src/render_scale.rs)).
