# Epiphant / Bikanwarda tattoo masks

Placeholder masks and UV contracts for [\#1128](https://github.com/ramate-io/maybraid/issues/1128). Liam chose **art A + mapping 1** on Oct 12, 2026: one mask per mesh on `TEXCOORD_0` when authored — no `TEXCOORD_1`. [#1231](https://github.com/ramate-io/maybraid/pull/1231) tracks engine support and placeholder art.

These PNGs are **placeholders only**. Replace them in place for final art. Do not add `TEXCOORD_1`.

| File | Part | Shader recipe |
|------|------|----------------|
| `epiphant_body_uv0_placeholder.png` | Body `characters/bodies/quadruped/epiphant.glb` | `skin_tattoo_body` |
| `epiphant_head_uv0_placeholder.png` | Head `characters/heads/meerkat_head_pronograde.glb` | `skin_tattoo_head` |
| `epiphant_ear_uv0_placeholder.png` | Ears `characters/ears/epiphant_ear_left.glb` (mirrored right) | `skin_tattoo_ear` |
| `epiphant_trunk_uv0_placeholder.png` | Trunk `characters/noses/trunkish_nose.glb` | `skin_tattoo_trunk` |
| `*_layout.png` | Occupancy + placeholder strokes | documentation |

## Shader material (landable without final art)

Reusable pieces live in `character-shaders`:

- Recipes `skin_tattoo_body` / `head` / `ear` / `trunk` — palette[0] hide, palette[1] glow, mask texture per recipe.
- [`SkinTattoo`](../../world/layers/mobs/characters/shaders/src/skin_material.rs) knobs: glow, intensity, pulse.
- [`SkinTattooUvSource`](../../world/layers/mobs/characters/shaders/src/skin_tattoo_uv.rs): how mask UV is computed. Default **`MeshUv0`** samples mesh `TEXCOORD_0`. Provisional bind-pose unwraps are **opt-in** on the stamped [`MaterialRef`](../../material-ref/src/material_ref.rs) (scalar index [`SKIN_SCALAR_UV_SOURCE`](../../world/layers/mobs/characters/shaders/src/skin_tattoo_uv.rs)); the shader does **not** infer unwrap from body part or mask kind.

Stamp via [`PartNode::with_skin_tattoo`](../../world/layers/mobs/characters/lib/src/nodes/part.rs) with the correct `SkinTattooUvSource`. Ward / Enraged reuse the same recipes and recolor `glow`. Tail stays ordinary hide tint.

Open knobs (config defaults, not approved):

- Glow color: warm gold/amber `srgb(1.0, 0.72, 0.28)`
- Intensity: `4.0`
- Pulse: off (`pulse_rate = 0`)

`MAYBRAID_TATTOO_UV_DEBUG=1` draws an 8×8 UV checker + mask overlay (investigation only).

## Texture spec

- **Resolution:** 1024×1024 (placeholder). Final art may go to 2048×2048; keep power-of-two square.
- **Format:** PNG, RGBA8. White / light = emission. Black + alpha 0 = unmarked hide.
- **Filter:** linear. Author as a thin mask (no color); the shader tints with `MaterialRef` palette[1].
- **Color space:** mask is treated as data (shader uses max(rgb)×alpha).

## Epiphant placeholder mapping (content recipe)

Until Liam authors UV0 on each GLB, the Epiphant recipe stamps provisional sources from [`epiphant_placeholder_uv_source`](../../world/layers/mobs/characters/lib/src/species/epiphant/tattoo_mapping.rs). Constants must match [`skin_material.wgsl`](../../world/layers/mobs/characters/shaders/src/skin_material.wgsl). When final `TEXCOORD_0` lands, switch each part to `SkinTattooUvSource::MeshUv0` only — no shader change.

### Head (provisional cylindrical about +Y — not final UV)

`meerkat_head_pronograde.glb` has `TEXCOORD_0`, but **875 / 956 verts collapse to `(0, 1)`**. The remaining 81 verts are a forehead island at **u 0.31–0.70, v 0.60–1.00**. That island cannot carry distinct linework on the face, cheeks, eyes, sides, or back of the skull.

The placeholder uses **provisional cylindrical about +Y** (Epiphant recipe sets `ProvisionalHeadCylindricalY`). Face at `u = 0.5`:

```
u = atan2(x, z) / (2π) + 0.5
v = saturate((y - y_min) / y_span)
y_min  = -0.9791473
y_span =  2.0418647
```

Dense ritual regions in this unwrap (placeholder mask only):

| Region | Unwrap |
|--------|--------|
| Face / muzzle | u ≈ 0.42–0.58, v 0.36–0.60 |
| Brow | u ≈ 0.34–0.66, v 0.62–0.70 |
| Around the eyes | u ≈ 0.32–0.40 and 0.60–0.68, v 0.52–0.64 |
| Cheeks | u ≈ 0.26–0.34 and 0.66–0.74, v 0.36–0.58 |
| Crown | u ≈ 0.32–0.68, v 0.72–0.94 |
| Skull sides | u ≈ 0.14–0.26 and 0.74–0.86, v 0.38–0.82 |
| Skull back | u ≈ 0.00–0.14 and 0.86–1.00, v 0.40–0.86 |

### Body (provisional cylindrical about +Z)

`epiphant.glb` currently has **no `TEXCOORD_0`**. Placeholder body mask uses **provisional cylindrical about +Z** (`ProvisionalBodyCylindricalZ`):

```
u = atan2(x, y) / (2π) + 0.5     // spine at u = 0.5, seam on the belly
v = saturate((z - z_min) / z_span)
z_min  = -0.9804133
z_span =  2.0035005
```

Z is the long axis (rear → front). Dense ritual regions in this unwrap:

| Region | Unwrap |
|--------|--------|
| Spine / back | u ≈ 0.44–0.56, v 0.10–0.92 |
| Flanks | u ≈ 0.22–0.38 and 0.62–0.78, v 0.18–0.78 |
| Belly | u ≈ 0.00–0.12 and 0.88–1.00, v 0.22–0.74 |
| Hind legs | v ≈ 0.06–0.26 |
| Front legs | v ≈ 0.74–0.94 |
| Chest / trunk-root rings | v ≈ 0.74–0.90, around u 0.50 |
| Hips / rump barrel | v ≈ 0.08–0.32, u 0.18–0.42 and 0.58–0.82 |

**Hips vs forehead (placeholder investigation):** lighting / facing does **not** suppress marks (emission is added after PBR). Body hip stretch matches the flanks (median world/UV ≈ 8). The hip/rump island was a **mask gap** in early placeholders. The forehead dome is the **+Y pole** of the cylindrical-about-Y provisional unwrap: U collapses toward 0.5 until a proper crown cap exists in authored UV0.

### Ear (provisional planar XY)

`ProvisionalEarPlanarXy`:

```
u = saturate((x - x_min) / x_span)
v = saturate((y - y_min) / y_span)
x_min = -0.29522276   x_span = 2.3301609
y_min = -1.6527351    y_span = 2.9819443
```

### Trunk (provisional cylindrical about +Y)

`ProvisionalTrunkCylindricalY`:

```
u = atan2(x, z) / (2π) + 0.5
v = saturate((y - y_min) / y_span)
y_min = -1.932509     y_span = 2.4697547
```

## Final UV requirements (Liam — human / Blender lane)

Do **not** edit the `.blend` in this engine lane. Re-export the same GLB paths. **Keep existing skinning, joint names, and animation clip paths unchanged.**

| Mesh | Requirement |
|------|-------------|
| `epiphant.glb` | Single `TEXCOORD_0` covering body, legs, belly, flanks, spine, and **hips / rump** as intentional islands (not a cylindrical pole at the hindquarters). Hindquarter / rump should have **uniform texel density** comparable to the flanks so ritual linework reads at encounter distance. |
| `meerkat_head_pronograde.glb` | Full-head `TEXCOORD_0` with a dedicated **crown cap island** (planar or dome patch) for forehead and top of skull — **not** a cylindrical pole at +Y. Face, cheeks, eye rings, brow, ears base, and skull sides/back need distinct UV area. |
| `epiphant_ear_left.glb` (+ mirrored right) | `TEXCOORD_0` on the ear mesh. |
| `trunkish_nose.glb` | `TEXCOORD_0` on the trunk mesh. |

After export: replace placeholder PNGs so strokes sit on the authored islands, and set every Epiphant tattoo part to `SkinTattooUvSource::MeshUv0` in [`tattoo_mapping.rs`](../../world/layers/mobs/characters/lib/src/species/epiphant/tattoo_mapping.rs).

No `TEXCOORD_1`.
