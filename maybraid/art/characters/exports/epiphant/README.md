# Epiphant assembled mesh (design export)

Wavefront export of the **default preview** Epiphant (`EpiphantConfig::default_preview()`): component GLBs assembled with the same fixed-assembly path as runtime (`spawn_fixed_character_assembly`), in **rest / bind pose**, **Y-up**, **metres** (game units).

## Files

| File | Description |
|------|-------------|
| `epiphant_default_preview.obj` | Combined mesh; one `g` / `o` group per component (body, head, features, tail). |
| `epiphant_default_preview.mtl` | Diffuse (`Kd`) tints from character recipe palette — **no image textures**. |
| `epiphant_default_preview.validation.json` | ECS reference bounds + socket bone positions at export time. |
| `previews/*.png` | matplotlib re-import sanity renders (not Procreate). |

## Orientation and scale

- **Up:** +Y (Bevy / glTF convention).
- **Forward:** creature faces **+Z** (quadruped assembly heading).
- **Scale:** 1 unit = 1 metre.

Rest-pose axis-aligned bounds (2026-10-15 export, after socket/pose fix):

- Combined: min (−1.103, −1.362, −1.301) m → max (1.038, 0.844, 1.708) m.

## What was wrong (PR #1322 review) and how it was fixed

The first export treated **every `Mesh3d` under the character root**, including rig GLB armature geometry, and could run **before** socket fulfill, skin prune, and bind-pose propagation finished. That produced oversized / misaligned parts in Blender (meshes “dumped” at bind pose without final assembly).

The exporter now:

1. Waits until fixed assembly is settled (no pending kits, sockets fulfilled, skin remap + duplicate-armature prune done, part meshes loaded).
2. Exports **only meshes under [`CharacterPart`](../../../../world/layers/mobs/characters/lib/src/rig.rs) hosts** (runtime-visible parts, not rig-only GLBs).
3. Runs in **`PostUpdate` after `TransformSystems::Propagate` and character/rig fulfill + pose**, matching rendered joint transforms for CPU skinning.

## Provenance

| Group | Source GLB (under `maybraid/assets/`) |
|-------|----------------------------------------|
| `body__epiphant` | `characters/bodies/quadruped/epiphant.glb` |
| `head__meerkat` | pronograde meerkat head |
| `tail__cat-tail` | `characters/tails/cat_tail.glb` |
| `ear_*__epiphant` | `characters/ears/epiphant_ear_left.glb` (right mirrored) |
| `nose__trunkish` | `characters/noses/trunkish_nose.glb` |
| `eye_*__standard` | shared eye mesh |

Assembly graph: `maybraid/world/layers/mobs/characters/lib/src/species/epiphant/recipe.rs`.

## Regenerating

From the repo root (Nix dev shell recommended). On headless Linux without a GPU, use Lavapipe:

```bash
VK_ICD_FILENAMES=/usr/share/vulkan/icd.d/lvp_icd.json \
  WGPU_BACKEND=vulkan \
  cargo run -p character-mesh-export -- \
  --species epiphant \
  --output maybraid/art/characters/exports/epiphant/epiphant_default_preview.obj
```

Validate OBJ vs ECS reference:

```bash
python3 scripts/character-mesh-validate/main.py \
  maybraid/art/characters/exports/epiphant/epiphant_default_preview.obj \
  maybraid/art/characters/exports/epiphant/epiphant_default_preview.validation.json
```

See [`mesh-export/README.md`](../../../../world/layers/mobs/characters/mesh-export/README.md).

## Validation snapshot (2026-10-15, post-fix)

### Measured (automated)

| Metric | Value |
|--------|------:|
| OBJ groups | 8 |
| Vertices (`v` lines) | 9 087 |
| Faces (triangles) | 3 178 |
| UVs / normals | yes / yes |
| Image textures | none (`Kd` in `.mtl` only) |
| OBJ size | ~772 KiB |
| ECS ↔ OBJ combined bbox corner delta | **0 mm** (trimesh re-import vs `validation.json`) |
| Per-group ECS bounds | in `epiphant_default_preview.validation.json` |
| Socket bone positions (ECS, character space) | same JSON (`sockets[]`) |

Example socket positions (m, character root space): body `head_socket` (−0.078, −0.045, 0.991); head rig `root` matches that point; eyes/ears/nose on head sockets (full list in JSON).

### Unverified

- Import/paint in **Procreate** (not run in CI/agent).
- Side-by-side **in-game runtime screenshot** (no windowed capture in headless export job; previews are matplotlib from the OBJ).

## Git LFS

Under 25 MB; ships as a normal git blob unless release tooling later mandates LFS for large exports.
