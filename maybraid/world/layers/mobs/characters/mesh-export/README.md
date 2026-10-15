# character-mesh-export

Headless exporter: spawn a character through the real fixed-assembly path, CPU-skin it, and write a Wavefront OBJ/MTL for DCC import (Procreate, Blender).

```bash
cargo run -p character-mesh-export -- \
  --species epiphant \
  --output maybraid/art/characters/exports/epiphant/epiphant_default_preview.obj
```

On headless Linux without a GPU, point Vulkan at Lavapipe:

```bash
VK_ICD_FILENAMES=/usr/share/vulkan/icd.d/lvp_icd.json WGPU_BACKEND=vulkan \
  cargo run -p character-mesh-export -- \
  --output maybraid/art/characters/exports/epiphant/epiphant_default_preview.obj
```

One `o` object per part. Face corners use independent `v` / `vt` / `vn` streams. Every part has a non-overlapping UV island in 0..1 (missing UVs are planar-unwrapped, then all parts are packed into atlas tiles). Export fails if a group has zero faces or an out-of-range index.
