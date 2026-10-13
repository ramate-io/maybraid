# Epiphant / Bikanwarda tattoo masks

Placeholder UV0 (or documented provisional unwrap) masks for [\#1128](https://github.com/ramate-io/maybraid/issues/1128). Liam chose **art A + mapping 1** on Oct 12, 2026: ritual linework on existing UV0 — no second UV channel. [#1231](https://github.com/ramate-io/maybraid/pull/1231) review asked for **denser** strokes so marks read from front, side, and three-quarter, including flanks, belly, back, legs, ears, trunk, and crown.

These PNGs are **placeholders only**. Replace them in place for final art. Do not add `TEXCOORD_1`.

| File | Part | Shader recipe |
|------|------|----------------|
| `epiphant_body_uv0_placeholder.png` | Body `characters/bodies/quadruped/epiphant.glb` | `skin_tattoo_body` |
| `epiphant_head_uv0_placeholder.png` | Head `characters/heads/meerkat_head_pronograde.glb` | `skin_tattoo_head` |
| `epiphant_ear_uv0_placeholder.png` | Ears `characters/ears/epiphant_ear_left.glb` (mirrored right) | `skin_tattoo_ear` |
| `epiphant_trunk_uv0_placeholder.png` | Trunk `characters/noses/trunkish_nose.glb` | `skin_tattoo_trunk` |
| `*_layout.png` | Occupancy + placeholder strokes | documentation |

Ears and trunk are separate meshes with **no `TEXCOORD_0`**. They stay on mapping 1 (one mask per mesh, no second UV channel) via the documented provisional unwraps below — same contract as the body until Liam authors UV0.

## Texture spec

- **Resolution:** 1024×1024 (placeholder). Final art may go to 2048×2048; keep power-of-two square.
- **Format:** PNG, RGBA8. White / light = emission. Black + alpha 0 = unmarked hide.
- **Filter:** linear. Author as a thin mask (no color); the shader tints with `MaterialRef` palette[1].
- **Color space:** mask is treated as data (shader uses max(rgb)×alpha).

## Mapping 1 — UV0 / provisional regions

### Head (authored UV0)

`meerkat_head_pronograde.glb` already has `TEXCOORD_0`. Used islands sit approximately in **u 0.00–0.70, v 0.60–1.00**. Crown, forehead, and brow paint belong in that island.

### Body (GLB requirement)

`epiphant.glb` currently has **no `TEXCOORD_0`**. Placeholder body mask uses this **provisional cylindrical unwrap** of bind-pose local space (must match `skin_material.wgsl`):

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

### Ear (provisional planar XY)

```
u = saturate((x - x_min) / x_span)
v = saturate((y - y_min) / y_span)
x_min = -0.29522276   x_span = 2.3301609
y_min = -1.6527351    y_span = 2.9819443
```

### Trunk (provisional cylindrical about +Y)

```
u = atan2(x, z) / (2π) + 0.5
v = saturate((y - y_min) / y_span)
y_min = -1.932509     y_span = 2.4697547
```

**Final GLB work for Liam:** add a single `TEXCOORD_0` to `epiphant.glb`, `epiphant_ear_left.glb`, and `trunkish_nose.glb` (no `TEXCOORD_1`). Keep existing skinning and joint names. Re-export the same paths. Then replace the placeholder PNGs so strokes sit on the authored islands. Once UV0 exists, the shader samples mesh UV0 and the provisional unwrap for that mesh is unused.

Do **not** edit the `.blend` in this lane; this folder is the contract for the later art pass.

## Recolor / reuse

Stamp `SkinTattoo { glow, intensity, pulse_rate, pulse_amp }` onto the **body, head, ear, and trunk part hosts** (`PartNode::with_skin_tattoo` / `MaterialRefRoot`). Ward and Enraged reuse the same recipes and swap `glow` (and optionally intensity). Tail stays ordinary hide tint.

Open knobs (config defaults, not approved):

- Glow color: warm gold/amber `srgb(1.0, 0.72, 0.28)`
- Intensity: `4.0`
- Pulse: off (`pulse_rate = 0`)
