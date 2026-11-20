# Bikanwarda character assembly (`bikanwarda-character`)

Boss-scale Epiphant assembly for Bikanwarda ([#1134](https://github.com/ramate-io/maybraid/issues/1134)): **6.7 m** encounter standing height via [`BikanwardaSpec::uniform_scale()`], sockets, collider fit, visual bounds, and Bray / sleep animation envelopes.

No NPC/world health bars and no `combat-hud` dependency — gameplay hooks are spawn markers + physics hulls only.

## Epiphant default maps

Liam’s `epiphant_default_preview` Color / Roughness / Metallic PNGs live at
`maybraid/assets/characters/epiphant/default/` as:

- `{slot}-Color.png` — sRGB albedo (`ImageLoaderSettings.is_srgb = true`)
- `{slot}-MetallicRoughness.png` — linear packed map, **G = roughness, B = metallic**

Slots: `body_mat`, `head_mat`, `ear_left_mat`, `ear_right_mat`, `eye_left_mat`,
`eye_right_mat`, `nose_mat`, `tail_mat`. [`BikanwardaActor`] stamps those names on
[`PartNode`] materials; [`BikanwardaCharacterPlugin`] loads [`StandardMaterial`] by
slot name. Git LFS is not used for `maybraid/assets` (same as the existing GLBs).

The maps were painted for a specific UV layout. **The matching GLB is not in this
repo.** Do not treat the current assembly as that mesh:

| Part | Current asset | UVs | Material slots |
|------|----------------|-----|----------------|
| Body | `characters/bodies/quadruped/epiphant.glb` (`BézierCurve`, skinned) | none | none |
| Head | `characters/heads/meerkat_head_pronograde.glb` (`MeerkatHead`) | `TEXCOORD_0` | none (`head_mat` missing) |
| Ear L/R | `characters/ears/epiphant_ear_left.glb` (right = reflected left) | none | none |
| Nose | `characters/noses/trunkish_nose.glb` | none | none |
| Tail | `characters/tails/cat_tail.glb` | `TEXCOORD_0` | none (`tail_mat` missing) |
| Eyes | `characters/eyes/humanoid_eye_left.glb` | `TEXCOORD_0` | none (`eye_*_mat` missing) |

Until a matching export lands, the maps bind by slot name so they apply
automatically; they cannot read as painted on the current hulls.

### Matching GLB requirements

Do not author a second encounter-scale export. Keep ordinary Epiphant authored
size; encounter still uses [`BikanwardaSpec::uniform_scale()`] to 6.7 m.

- **Material slot names** exactly: `body_mat`, `head_mat`, `ear_left_mat`,
  `ear_right_mat`, `eye_left_mat`, `eye_right_mat`, `nose_mat`, `tail_mat`.
- **UVs:** every painted primitive needs `TEXCOORD_0` authored to those 2048×2048
  maps. Do not unwrap a new layout.
- **Part / mesh names** should match the slots (body, head, ears, eyes, nose, tail).
  Right ear must be its own mesh with `ear_right_mat` UVs — not a UV-mirrored left
  ear — so the painted right map lands.
- **Sockets / anchors** (keep names): `head_socket`, `ear_socket.L` / `.R`,
  `eye_socket.L` / `.R`, `nose_socket`, `tailbone`, `mouth_emit`. Chain anchors stay
  on [`BikanwardaAnchors`] (motor-local, pre-root-scale).
- **Origin / axes:** +Y up, +Z forward, origin at the quadruped motor root (same
  as today’s `epiphant.glb` / quadruped rig). Body AABB today is about
  `[-0.74, -1.09, -0.98]` … `[0.73, 0.52, 1.02]` before recipe normalization.
- **Skin:** body skinned to the quadruped armature (`back_ridge` root). Head
  skinned to the pronograde head rig.

**Preview stills** (Lavapipe + `character-concepts-playground`):

```bash
./maybraid/world/layers/mobs/characters/bikanwarda/previews/1134/capture.sh
```

Close / mid cameras: `CROZON_PREVIEW_CAMERA_DISTANCE=0.42` or `1.0`. GPU stills:
`CROZON_PREVIEW_CAPTURE_PNG=/tmp/bikanwarda.png` (lavapipe `scrot` stays black).

**Tests:**

```bash
export CXX=g++-13 CC=gcc-13 LIBRARY_PATH=/usr/lib/gcc/x86_64-linux-gnu/13
cargo test -p bikanwarda-character --all-targets
```
