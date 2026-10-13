# Epiphant / Bikanwarda tattoo masks

Placeholder UV0 masks for [\#1128](https://github.com/ramate-io/maybraid/issues/1128). Liam chose **art A + mapping 1** on Oct 12, 2026: ritual linework (spine, flanks, forehead, trunk rings) on existing body + head UV0.

These PNGs are **placeholders only**. Replace them in place for final art. Do not add a second UV channel.

| File | Part | Shader recipe |
|------|------|----------------|
| `epiphant_body_uv0_placeholder.png` | Body `characters/bodies/quadruped/epiphant.glb` | `skin_tattoo_body` |
| `epiphant_head_uv0_placeholder.png` | Head `characters/heads/meerkat_head_pronograde.glb` | `skin_tattoo_head` |
| `epiphant_body_uv0_layout.png` | Body occupancy + placeholder strokes | documentation |
| `epiphant_head_uv0_layout.png` | Head UV0 occupancy + placeholder strokes | documentation |

## Texture spec

- **Resolution:** 1024×1024 (placeholder). Final art may go to 2048×2048; keep power-of-two square.
- **Format:** PNG, RGBA8. White / light = emission. Black + alpha 0 = unmarked hide.
- **Filter:** linear. Author as a thin mask (no color); the shader tints with `MaterialRef` palette[1].
- **Color space:** mask is treated as data (shader uses max(rgb)×alpha).

## Mapping 1 — UV0 regions

### Head (authored UV0)

`meerkat_head_pronograde.glb` already has `TEXCOORD_0`. Used islands sit approximately in **u 0.00–0.70, v 0.60–1.00**. Forehead / brow paint belongs in the high-V, forward-facing part of that island. Ears, trunk/nose, and tail are **not** tattooed under mapping 1.

### Body (GLB requirement)

`epiphant.glb` currently has **no `TEXCOORD_0`** — only POSITION / NORMAL / JOINTS_0 / WEIGHTS_0. The placeholder body mask is authored against this **provisional cylindrical unwrap** of bind-pose local space (must match `skin_material.wgsl`):

```
u = atan2(x, y) / (2π) + 0.5     // spine at u = 0.5, seam on the belly
v = saturate((z - z_min) / z_span)
z_min  = -0.9804133
z_span =  2.0035005
```

Z is the long axis (rear → front / trunk). Ritual regions in this unwrap:

| Region | Unwrap |
|--------|--------|
| Spine | u ≈ 0.50, v 0.18–0.86 |
| Flanks | u ≈ 0.33 and 0.67, v 0.30–0.72 |
| Trunk rings | v ≈ 0.78 / 0.83 / 0.88, around u 0.50 |

**Final GLB work for Liam:** add a single `TEXCOORD_0` to `epiphant.glb` (no `TEXCOORD_1`). Keep existing skinning and joint names. Re-export the same path. Then replace `epiphant_body_uv0_placeholder.png` so strokes sit on the authored islands (spine, flanks, chest/trunk-ring area). Once UV0 exists, the shader samples mesh UV0 and the provisional unwrap is unused.

Do **not** edit the `.blend` in this lane; this folder is the contract for the later art pass.

## Recolor / reuse

Stamp `SkinTattoo { glow, intensity, pulse_rate, pulse_amp }` onto the **body and head part hosts** (`PartNode::with_skin_tattoo` / `MaterialRefRoot`). Ward and Enraged reuse the same recipes and swap `glow` (and optionally intensity). Ears, trunk, and tail stay ordinary hide tints.

Open knobs (config defaults, not approved):

- Glow color: warm gold/amber `srgb(1.0, 0.72, 0.28)`
- Intensity: `4.0`
- Pulse: off (`pulse_rate = 0`)
