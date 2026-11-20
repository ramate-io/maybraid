# Bikanwarda character assembly (`bikanwarda-character`)

Boss-scale Epiphant assembly for Bikanwarda ([#1134](https://github.com/ramate-io/maybraid/issues/1134)): **6.7 m** encounter standing height via [`BikanwardaSpec::uniform_scale()`], sockets, collider fit, visual bounds, and Bray / sleep animation envelopes.

No NPC/world health bars and no `combat-hud` dependency — gameplay hooks are spawn markers + physics hulls only.

## Painted default-preview visual

Liam painted Color / Roughness / Metallic maps on the [#1589](https://github.com/ramate-io/maybraid/pull/1589) OBJ
(`maybraid/art/characters/exports/epiphant/epiphant_default_preview.obj`): one `o` /
`{part}_mat` per component, chart-unwrapped UVs. Bevy does not load OBJ. A
format-conversion script writes a GLB that keeps those names and UVs:

```bash
python3 maybraid/art/characters/exports/epiphant/obj_to_glb.py
```

Output: `maybraid/assets/characters/epiphant/default/epiphant_default_preview.glb`
(one glTF scene per part: Scene0 body … Scene7 tail). No new crate; no Blender
re-authoring. V is flipped for glTF’s top-left TEXCOORD origin.

Runtime maps (git LFS is not used for `maybraid/assets`):

- `{slot}-Color.png` — sRGB albedo
- `{slot}-MetallicRoughness.png` — linear packed map, **G = roughness, B = metallic**

[`BikanwardaActor`] loads those scenes as unsocketed, unskinned [`PartNode`]s and
stamps `{part}_mat`. [`BikanwardaCharacterPlugin`] binds [`StandardMaterial`] by
slot name.

### Animation / gait tradeoff

The #1589 export is a **static rest-pose** mesh (CPU-skinned once at export, then
OBJ). It is not a skinned GLB. Binding it as the visible body means **the painted
mesh does not deform** with Bray, sleep, or locomotion clips. That is not faked.

What still runs on the existing rig:

- Quadruped + pronograde head **rigs** (clips, bone maps)
- Motor collider / hurtboxes from [`BikanwardaSpec::uniform_scale()`] (6.7 m)
- Chain anchors and Bray origin (`nose_socket` on the head rig)

The textured mesh is a visual over that rig. Ordinary player Epiphants keep the
skinned palette assembly.

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
