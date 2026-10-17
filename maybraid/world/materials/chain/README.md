# Mythical chain material

Glowing iron-and-energy surface for Bikanwarda’s restraint chains
([#1124](https://github.com/ramate-io/maybraid/issues/1124)). Durability,
glow, seed, and break/fade are [`MaterialRef`](../../material-ref) bags.
Deterioration is shaded, not remeshed.

```text
cargo test -p chain-shaders
cargo run -p chain-playground
cargo run -p chain-playground -- show pair
cargo run -p chain-playground -- lighting night
```

Runtime skinned rope/chain geometry is [A04 / #1125](https://github.com/ramate-io/maybraid/issues/1125)
and is not in this crate. Playground previews use
[`placeholder::placeholder_chain_mesh`](src/placeholder.rs).

## Recipe

| Name | Kind |
|------|------|
| `mythical_chain` | Stylized metal + energy veins + deterioration |

Helpers: `mythical_chain_ref`, `mythical_chain_ref_for`, `MythicalChainState`.

Two chains with different scalars or seeds resolve to different
[`MaterialRefKey`](../../material-ref/src/key.rs) handles. Updating one
`MaterialRefRoot` does not mutate the other instance.

## Parameter ranges

Palette (linear-authored sRGB):

| Slot | Role | Default | Notes |
|------|------|---------|-------|
| 0 | Intact metal | `(0.22, 0.16, 0.12)` | Readability under key + fill |
| 1 | Energy glow | `(0.62, 0.90, 1.0)` | Intensity is a scalar, not alpha |
| 2 | Damage / ash | `(0.20, 0.08, 0.05)` | Cracks and rust |
| 3 | Ember / near-break | `(1.0, 0.42, 0.10)` | Hot only when damaged |

Scalars (index meaning is this shader’s contract):

| Index | Name | Range | Default | Intact / damaged / near-break / released |
|-------|------|-------|---------|------------------------------------------|
| 0 | `intact` | `[0, 1]` | `1.0` | `1.0` / `0.55` / `0.18` / `0.0` |
| 1 | `glow_intensity` | `[0, 8]` | `5.2` | authored gain |
| 2 | `fade` (break/release) | `[0, 1]` | `0.0` | `0.0` / `0.0` / `0.15` / `1.0` |
| 3 | `damage` | `[0, 1]` | `0.0` | `0` means derive `1 - intact` |
| 4 | `link_count` hint | `[1, 256]` | `14` | UV.y → link fallback |
| 5 | `crack_frequency` | `[0.5, 24]` | `6.0` | UV-space crack scale |
| 6 | `localized_blend` | `[0, 1]` | `0.0` | `0` = global durability only |
| 7 | `pulse_rate` | `[0, 8]` | `1.4` | glow / ember flicker |

[`NoiseParams`](../../../procedural/common/src/noise.rs):

| Field | Role | Authored |
|-------|------|----------|
| `seed` | Per-chain variation and which links weaken first | `11` |
| `frequency` | Spatial scale of cracks / ember boil | `2.4` |
| `amplitude` | Noise strength on cracks | `1.0` |
| `octaves` | FBm depth | `2` |

Global durability is the supported path (`intact` / derived `damage`).
Localized damage is an opt-in extension: write per-link or per-vertex
values into **vertex color G** and raise `localized_blend` above `0`.

## Geometry contract (A04 / #1125)

The shader is UV-locked and skin-aware. World-space position is only used
for lighting. A04’s skinned mesh must meet:

### UV 0

- **U (`uv.x`)**: around the tube / link cross-section, `[0, 1]`, seamless.
- **V (`uv.y`)**: **along the chain**, `[0, 1]`, monotonically increasing
  from start anchor to end anchor. Distance convention: `0` at the first
  joint, `1` at the last. Within link `i` of `N` links, `uv.y` should
  occupy `[i/N, (i+1)/N]` (continuous along a link is better than a
  constant).
- Do not mirror V. Glow flow, crack travel, and fade read V as a
  directed length parameter.
- Seams belong at link contacts, not mid-link.

### Vertex color (`Mesh::ATTRIBUTE_COLOR`, float4)

| Channel | Meaning | Encoding |
|---------|---------|----------|
| R | Per-link id | `link_index as f32` (0, 1, 2, …). All vertices of one link share the same value so interpolation stays flat. Shader uses `floor(color.r + 0.5)`. |
| G | Localized damage | `[0, 1]`, `0` intact. Ignored while `localized_blend == 0`. |
| B | Twist phase | `0` even / `1` odd (alternating link orientation). |
| A | Reserved | `1` |

If color is missing, the shader falls back to
`floor(uv.y * link_count)` using scalar 4. That fallback cannot express
true interlocking ids or localized damage — A04 should write color.

### Skinning

The vertex stage uses Bevy `#ifdef SKINNED` (same path as clothing).
Joint indices/weights and inverse bind poses are A04’s. This material
does not generate them. Bounds must keep deformed links visible.

### Damage parameter input

Gameplay (A06) should restamp a per-chain [`MaterialRef`](../../material-ref)
rather than mutate a shared handle:

1. `intact` = remaining durability in `[0, 1]`.
2. `fade` = release / dissolve progress (`0` still bound, `1` fully
   released). Near-breaking authored fade is `0.15`, not `1`.
3. Optional `damage` override; leave `0` to derive `1 - intact`.
4. Keep `seed` stable for a given chain so weakening stays on the same
   links as durability drops.
5. Do not rebuild the mesh on hit. Cracks, ember, glow drop, and
   dissolve are fragment tests hashed by link id + seed.

Weak links (higher `hash13(vec3(id * 17.13, seed * 0.013, 4.2))`)
deteriorate first, so the chain reads as unevenly failing along its
length without a second damage texture. The CPU model in
[`hash.rs`](src/hash.rs) is a lockstep port of [`hash.wgsl`](src/hash.wgsl);
tests compare both function bodies and golden vectors computed from the
WGSL literals and the Rust port.

### Out of scope here

- Armature, bind pose, capsule collision (A04 / A05).
- Hit routing and the broken signal (A06).
- Final Blender hero mesh. Placeholder tori are preview-only.

## Authored states

| State | `intact` | `fade` | Look |
|-------|----------|--------|------|
| Intact | `1.00` | `0.00` | Clean metal, full veins |
| Damaged | `0.55` | `0.00` | Cracks, rust, dimmer glow |
| Near-breaking | `0.18` | `0.15` | Ember, flicker, holes on weak links |
| Released | `0.00` | `1.00` | Dissolve / leftover sparks |

## Playground

`chain-playground` shows two independently parameterized chains, a
durability sweep, CPU sine-bend (stand-in for A04 skinning), and
day/night lighting.
