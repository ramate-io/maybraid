# character-mesh-export

Headless-friendly exporter that spawns a character through the real **fixed assembly** pipeline (`CharacterHostsPlugin` + `spawn_fixed_character_assembly`), waits for GLB load and skin remap, CPU-skins meshes, and writes a Wavefront OBJ/MTL bundle for DCC tools (e.g. Procreate 3D paint).

## Usage

```bash
cargo run -p character-mesh-export -- \
  --species epiphant \
  --output maybraid/art/characters/exports/epiphant/epiphant_default_preview.obj
```

### GPU runtime vs OBJ silhouette compare (headless)

Writes `runtime_*.png`, `obj_reimport_*.png`, `cameras.json` under `--compare-dir` (xvfb + Lavapipe). The capture camera is spawned at job init and settled before the first shot so character shader pipelines are specialized (a first-frame-only camera produced a head-only front view).

```bash
VK_ICD_FILENAMES=/usr/share/vulkan/icd.d/lvp_icd.json WGPU_BACKEND=vulkan \
  xvfb-run -a -s "-screen 0 640x640x24" \
  cargo run -p character-mesh-export --release -- \
  --output maybraid/art/characters/exports/epiphant/epiphant_default_preview.obj \
  --compare-dir maybraid/art/characters/exports/epiphant/compare

python3 scripts/character-mesh-export/compare_silhouettes.py \
  maybraid/art/characters/exports/epiphant/compare \
  maybraid/art/characters/exports/epiphant/epiphant_default_preview.obj
```

### Headless Linux (no GPU)

Bevy still initializes a Vulkan adapter; Lavapipe works:

```bash
VK_ICD_FILENAMES=/usr/share/vulkan/icd.d/lvp_icd.json WGPU_BACKEND=vulkan \
  cargo run -p character-mesh-export -- --output /path/to/out.obj
```

## Output conventions

- Rest/bind pose, character root at origin.
- Y-up, metres.
- One OBJ group per component (`body__…`, `head__…`, etc.).
- Materials: palette `Kd` only (no texture maps).
- Waits for socket fulfill + skin remap/prune; exports only `CharacterPart` meshes.
- Schedules export in `PostUpdate` after transforms and rig pose propagate.
- Writes `<output>.validation.json` (ECS bounds + socket positions) and `<output>.diagnose.json` (per-part transform chains + divergence vs reference skinning).
- Validate with `scripts/character-mesh-validate/main.py` (OBJ vs validation JSON) and inspect `diagnose.json` for assembly evidence.

## Scope

Does not modify game assets or gameplay crates. Only this binary crate and checked-in export artifacts under `maybraid/art/characters/exports/`.
