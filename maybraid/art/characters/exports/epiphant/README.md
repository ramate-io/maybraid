# Epiphant assembled mesh (design export)

Wavefront export of the **default preview** Epiphant (`EpiphantConfig::default_preview()`): component GLBs assembled with the same fixed-assembly path as runtime (`spawn_fixed_character_assembly`), in **rest / bind pose**, **Y-up**, **metres** (game units).

## Files

| File | Description |
|------|-------------|
| `epiphant_default_preview.obj` | Combined mesh; one `g` / `o` group per component (body, head, features, tail). |
| `epiphant_default_preview.mtl` | Diffuse (`Kd`) tints from character recipe palette — **no image textures**. |
| `epiphant_default_preview.validation.json` | ECS bounds + socket bone positions at export time (same code path as OBJ). |
| `epiphant_default_preview.diagnose.json` | Per-part transform chains, socket snapshot, mesh entity globals, reference vs OBJ divergence. |
| `previews/*.png` | matplotlib re-import sanity renders (not Procreate). |

## Orientation and scale

- **Up:** +Y (Bevy / glTF convention).
- **Forward:** creature faces **+Z** (quadruped assembly heading).
- **Scale:** 1 unit = 1 metre.

Rest-pose axis-aligned bounds (2026-10-15 export, post skinning/mesh-selection fix):

- Combined: min (−1.097, −1.362, −1.302) m → max (1.038, 0.845, 1.739) m.

## Root cause (PR #1322 review, evidence-based)

| Issue | What happened | Fix |
|-------|----------------|-----|
| **False “0 mm delta” validation** | `validation.json` was generated from the same `collect_groups` / CPU skinning path as the OBJ writer, so `character-mesh-validate` only proved the OBJ matched itself—not an independent runtime render. | Documented; added `diagnose.json` with per-part chains and `first_divergence_step`. |
| **Bind-local vertices in OBJ** | CPU skinning left vertices with no live joint weight at **raw mesh bind positions** (character space without `GlobalTransform`), while rigid features used the mesh entity world matrix. That diverges from `screen-capture` / rigid export for any zero-weight influences. | `skin.rs` now uses `Mat4::from(joint.affine()) * inverse_bind` and falls back to `mesh_world.transform_point(bind)` when live weight ≤ ε (matches `screen-capture`). |
| **Multiple `Mesh3d` per part** | Export walked every mesh under a `CharacterPart` host; duplicate / helper GLB nodes could be merged into a group. | One **primary** mesh per part: prefer `SkinnedMesh`, else largest vertex count (`primary_mesh_under`). |

**Measured (diagnose, 2026-10-15):** all eight parts report `first_divergence_step: "none (OBJ matches runtime reference skinning)"` and per-group bbox Δ = 0 vs the reference path. Socket alignment: head rig `root` equals body `head_socket` (−0.078, −0.045, 0.991) m in character space (see `validation.json` / `diagnose.json`).

**Note:** Source GLBs name the render mesh `BézierCurve` for body/ears/tail; that is the authored mesh name in Blender exports, not a stray curve helper.

**Unverified:** in-game GPU screenshot at the same camera as OBJ previews (Lavapipe export has no shaded character pass); Procreate import.

## Regenerating

From the repo root (Nix dev shell recommended). On headless Linux without a GPU, use Lavapipe:

```bash
VK_ICD_FILENAMES=/usr/share/vulkan/icd.d/lvp_icd.json \
  WGPU_BACKEND=vulkan \
  cargo run -p character-mesh-export -- \
  --species epiphant \
  --output maybraid/art/characters/exports/epiphant/epiphant_default_preview.obj
```

Validate OBJ vs ECS reference (same-frame):

```bash
python3 scripts/character-mesh-validate/main.py \
  maybraid/art/characters/exports/epiphant/epiphant_default_preview.obj \
  maybraid/art/characters/exports/epiphant/epiphant_default_preview.validation.json
```

OBJ preview PNGs:

```bash
python3 scripts/character-mesh-export/render_previews.py \
  maybraid/art/characters/exports/epiphant/epiphant_default_preview.obj \
  maybraid/art/characters/exports/epiphant/previews
```

See [`mesh-export/README.md`](../../../../world/layers/mobs/characters/mesh-export/README.md).

## Validation snapshot (2026-10-15)

### Measured (automated)

| Metric | Value |
|--------|------:|
| OBJ groups | 8 |
| Vertices (`v` lines) | 9 087 |
| Faces (triangles) | 3 178 |
| UVs / normals | yes / yes |
| Image textures | none (`Kd` in `.mtl` only) |
| ECS ↔ OBJ per-group bbox Δ | **0** m (`character-mesh-validate`) |
| `diagnose.json` reference vs OBJ | **no divergence** on all parts |

### Unverified

- Import/paint in **Procreate**.
- Side-by-side **in-game runtime screenshot** vs OBJ at matched camera (previews are matplotlib point clouds from the OBJ).

## Git LFS

Under 25 MB; ships as a normal git blob unless release tooling later mandates LFS for large exports.
