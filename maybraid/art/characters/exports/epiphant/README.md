# Epiphant assembled mesh (design export)

Wavefront export of the **default preview** Epiphant (`EpiphantConfig::default_preview()`): component GLBs assembled with the same fixed-assembly path as runtime (`spawn_fixed_character_assembly`), in **rest / bind pose**, **Y-up**, **metres** (game units).

## Files

| File | Description |
|------|-------------|
| `epiphant_default_preview.obj` | Separate `o` objects; **first is `root` (body)**, then head/tail/features. Every object has UVs. |
| `epiphant_default_preview.mtl` | Relative `mtllib`; `Kd` plus `map_Kd albedo.png`. |
| `albedo.png` | 1×1 placeholder color map (folder-import with the OBJ). |
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

**Blender “scattered dots” on head/tail (2026-10-15):** not missing faces in the writer — `f` lines existed — but **`vt` indices reused the global `v` index**. Body/ears/nose write no `vt` (`f v//vn`). Head/tail write `f v/vt/vn`. After UV-less groups, `vt` 7951…9087 referenced a file with only **1665** texcoords, so Blender dropped those faces and left vertices. Bevy re-import ignored `vt` and still drew triangles.

Fix: independent `v` / `vt` / `vn` bases. Head sample is now `f 7153/529/7153`; tail `f 8859/1437/8859`. Validator rejects zero faces or out-of-range indices.

| Group | `v` | `vt` | `vn` | `f` | form | OOB | winding vs `vn` |
|-------|----:|-----:|-----:|----:|------|----:|----------------:|
| body__epiphant | 5148 | 0 | 5148 | 1808 | v//vn | 0 | 1808/0 agree |
| head__meerkat | 956 | 956 | 956 | **324** | v/vt/vn | 0 | 324/0 |
| tail__cat-tail | 229 | 229 | 229 | **122** | v/vt/vn | 0 | 122/0 |
| ear_left / ear_right | 738 | 0 | 738 | 246 | v//vn | 0 | 246/0 |
| eye_left / eye_right | 240 | 240 | 240 | 80 | v/vt/vn | 0 | 80/0 |
| nose__trunkish | 798 | 0 | 798 | 272 | v//vn | 0 | 272/0 |

**Procreate “no mesh” (docs/forums, not run here):** Savage [Handbook](https://help.procreate.com/procreate/handbook/3d-painting/import) requires a UV map on any paintable OBJ. Official missing-UV copy is “This model has missing UVs is currently unsupported.” Forum reports of **“model contains no valid meshes” / “no meshes”** ([r/ProCreate](https://www.reddit.com/r/ProCreate/comments/qkw8i6/model_import_failing/)) match OBJ exports **without UVs** (or only `f v//vn`). The previous file’s first object was the body with **no `vt`** — iOS ModelIO can treat that as an empty mesh / no root. We do **not** merge parts (breaks authoring). Layout now: `o root` (body) first, then separate part objects, every face `v/vt/vn`, relative `mtllib`, no `g` lines.

**Unverified:** Procreate on iPad (cannot be run in this environment). Blender GUI re-open. Matplotlib OBJ IoU (~0.43–0.55) uses a different camera model — not the gate.

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
