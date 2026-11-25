# Bikanwarda character assembly (`bikanwarda-character`)

Boss-scale Epiphant assembly for Bikanwarda ([#1134](https://github.com/ramate-io/maybraid/issues/1134)): **6.7 m** encounter standing height via [`BikanwardaSpec::uniform_scale()`], sockets, collider fit, visual bounds, and Bray / sleep animation envelopes.

No NPC/world health bars and no `combat-hud` dependency — gameplay hooks are spawn markers + physics hulls only.

## Painted default-preview visual

Liam painted Color / Roughness / Metallic maps on the [#1589](https://github.com/ramate-io/maybraid/pull/1589) OBJ
(`maybraid/art/characters/exports/epiphant/epiphant_default_preview.obj`): one `o` /
`{part}_mat` per component, chart-unwrapped UVs. That OBJ is the live part meshes
CPU-skinned once, so on its own it cannot deform.

`character-mesh-export --rebind-painted` (see the
[art README](../../../../../art/characters/exports/epiphant/README.md)) matches every
painted triangle back to the runtime part triangle it came from and writes
`maybraid/assets/characters/epiphant/default/epiphant_default_preview.glb`: one mesh
per component in that part's own local space, with the painted UVs and, for skinned
parts, the part GLB's own joints and weights.

Runtime maps (git LFS is not used for `maybraid/assets`):

- `{slot}-Color.png` — sRGB albedo
- `{slot}-MetallicRoughness.png` — linear packed map, **G = roughness, B = metallic**

[`BikanwardaActor`] uses the stock Epiphant part nodes (same skins, sockets, mirrored
right ear / eye) and stamps `{part}_mat` on each. [`BikanwardaCharacterPlugin`] then
swaps each part's primary mesh for its painted mesh (`#Mesh{i}/Primitive0`) once the
part's skin is remapped, keeping the entity's `SkinnedMesh`, and binds
[`StandardMaterial`] by slot name. A painted mesh is only bound if its bounds match
the mesh it replaces and it has joints exactly when the part is skinned; otherwise
the part keeps its stock mesh and logs a warning.

### Per component

| Component | Deforms with | Painted maps |
| --- | --- | --- |
| body | quadruped body rig (skinned) | `body_mat` |
| head | pronograde head rig (skinned) | `head_mat` |
| ear_left / ear_right | head rig `ear_socket.L/R` (rigid, right is mirrored) | `ear_left_mat` / `ear_right_mat` |
| eye_left / eye_right | head rig `eye_socket.L/R` (rigid, right is mirrored) | `eye_left_mat` / `eye_right_mat` |
| nose | head rig `nose_socket` (rigid) | `nose_mat` |
| tail | body rig `tailbone` (rigid) | `tail_mat` |

Ears, eyes, nose and tail are rigid meshes on sockets for every Epiphant; the part
GLBs have no skin. Colliders, chain anchors and the Bray origin (`nose_socket`) are
unchanged. Ordinary player Epiphants keep the palette assembly.

**Preview stills** (Lavapipe + `character-concepts-playground`):

```bash
CROZON_PREVIEW_ENCOUNTER_SCALE=$(cargo run -q --example scale -p bikanwarda-character) \
CROZON_PREVIEW_CAMERA_DISTANCE=0.42 \
CROZON_PREVIEW_CAPTURE_PNG=/tmp/bikanwarda-close.png \
  cargo run -p character-concepts-playground -- epiphant preview --animation still
```

Close / mid cameras: `CROZON_PREVIEW_CAMERA_DISTANCE=0.42` or `1.0`. GPU stills:
`CROZON_PREVIEW_CAPTURE_PNG` (lavapipe `scrot` stays black). Encounter-scale
Epiphant preview spawns [`BikanwardaActorRecipe`].

**Tests:**

```bash
export CXX=g++-13 CC=gcc-13 LIBRARY_PATH=/usr/lib/gcc/x86_64-linux-gnu/13
cargo test -p bikanwarda-character --all-targets
```
