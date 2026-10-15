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

**GPU compare (2026-10-15, Lavapipe + xvfb, after capture-rig warmup):** see [`compare/`](compare/) — `runtime_*` (live `CharacterHostsPlugin` draw) vs `obj_reimport_*` (same camera, OBJ loaded in Bevy). Mask IoU (RGB vs clear-color; Bevy PNGs are opaque):

| View | IoU runtime vs OBJ re-import | Runtime mask H×W (px) | OBJ re-import H×W (px) | H/W ratio |
|------|-----------------------------:|----------------------:|-----------------------:|----------:|
| front | **0.996** | 258×256 | 258×256 | 1.00 |
| side | **1.000** | 263×329 | 263×329 | 1.00 |
| three_quarter | **1.000** | 264×272 | 264×272 | 1.00 |

Side-by-side: `compare/compare_{front,side,three_quarter}.png`. Numbers: `compare/silhouette_compare.json`.

**Root cause of the earlier front-only-head shot:** the capture camera was spawned on the first screenshot frame, so clothing/face shader pipelines had not specialized. Side/TQ (later frames) already showed the assembled character. Spawning the rig at compare init and settling ~90 frames with that camera present makes all three runtime views match the OBJ.

**Exporter vertices:** no bake-math change. CPU diagnose bbox Δ = 0; GPU silhouettes now agree. Liam-style “parts not socketed” is not reproduced against the real renderer once the first frame is valid.

**Unverified:** Procreate / Blender GUI import (automated check is Bevy GPU re-import at 1 m). Matplotlib OBJ IoU (~0.43–0.55) uses a different camera model — not the gate.

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
| GPU mask IoU (runtime vs OBJ re-import) | **0.996 / 1.000 / 1.000** (front / side / three-quarter) |

### Part / proportion audit

`parts_audit.json` (from `scripts/character-mesh-export/audit_parts.py`): all eight recipe groups present; head-to-body extent ratio ≈ **0.54**; tail center z ≈ **−1.00** m (rear of body); head center above body center on Y.

### Unverified

- Import/paint in **Procreate** or **Blender** (automated check is Bevy GPU re-import only).

## Git LFS

Under 25 MB; ships as a normal git blob unless release tooling later mandates LFS for large exports.
