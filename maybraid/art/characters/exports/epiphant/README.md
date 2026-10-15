# Epiphant assembled mesh (design export)

Wavefront export of the **default preview** Epiphant (`EpiphantConfig::default_preview()`): component GLBs assembled with the same fixed-assembly path as runtime (`spawn_fixed_character_assembly`), in **rest / bind pose**, **Y-up**, **metres** (game units).

## Files

| File | Description |
|------|-------------|
| `epiphant_default_preview.obj` | Combined mesh; one `g` / `o` group per component (body, head, features, tail). |
| `epiphant_default_preview.mtl` | Diffuse (`Kd`) tints from character recipe palette — **no image textures**. |
| `previews/*.png` | Quick matplotlib re-import sanity renders (not Procreate). |

## Orientation and scale

- **Up:** +Y (Bevy / glTF convention).
- **Forward:** creature faces **+Z** (quadruped assembly heading).
- **Scale:** 1 unit = 1 metre. Rest-pose axis-aligned bounds (2026-10-15 export): roughly 2.44 × 2.22 × 2.05 m (X × Y × Z).

## Provenance

| Group | Source GLB (under `maybraid/assets/`) |
|-------|----------------------------------------|
| `body__epiphant` | `characters/bodies/quadruped/epiphant.glb` |
| `head__meerkat` | pronograde meerkat head (`characters/heads/…`) |
| `tail__cat-tail` | `characters/tails/cat_tail.glb` |
| `ear_*__epiphant` | `characters/ears/epiphant_ear_left.glb` (right mirrored) |
| `nose__trunkish` | `characters/noses/trunkish_nose.glb` |
| `eye_*__standard` | shared eye mesh |

Assembly graph: `maybraid/world/layers/mobs/characters/lib/src/species/epiphant/recipe.rs`.

## Regenerating

From the repo root (Nix dev shell recommended; provides Blender and consistent Bevy deps). On headless Linux without a GPU, use Lavapipe:

```bash
VK_ICD_FILENAMES=/usr/share/vulkan/icd.d/lvp_icd.json \
  WGPU_BACKEND=vulkan \
  cargo run -p character-mesh-export -- \
  --species epiphant \
  --output maybraid/art/characters/exports/epiphant/epiphant_default_preview.obj
```

See [`mesh-export/README.md`](../../../../world/layers/mobs/characters/mesh-export/README.md) for tool details.

## Validation snapshot (2026-10-15)

| Metric | Value |
|--------|------:|
| OBJ groups | 8 |
| Vertices (`v` lines) | 9 087 |
| Faces (triangles) | 3 178 |
| UVs | yes (`vt`) |
| Normals | yes (`vn`) |
| Image textures | none (vertex colours approximated via `Kd` in `.mtl`) |
| OBJ size | ~778 KiB |
| MTL size | ~447 B |

Re-import check: `trimesh` load, bbox matches tool output; previews in `previews/`. **Not verified in Procreate** on this export run.

## Git LFS

The OBJ is under 25 MB; the repo has no checked-in Git LFS policy for art OBJs, so this ships as a normal git blob unless release tooling later mandates LFS for large exports.
