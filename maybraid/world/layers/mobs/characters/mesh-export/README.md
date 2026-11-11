# character-mesh-export

Headless exporter: spawn a character through the real fixed-assembly path, CPU-skin it, chart-unwrap UVs, and write a **mesh-only** Wavefront OBJ for DCC import (Procreate, Blender). No MTL and no texture images.

GLB parts often arrive with unique vertices per corner, so charts grow across **geometric** edges (quantized positions), not index adjacency. Islands stay connected surfaces (torso, legs, ear flaps), not one triangle per island.

```bash
nix develop --command cargo run -p character-mesh-export --release -- \
  --species epiphant \
  --output maybraid/art/characters/exports/epiphant/epiphant_default_preview.obj
```

On headless Linux without a GPU, point Vulkan at Lavapipe:

```bash
VK_ICD_FILENAMES=/usr/share/vulkan/icd.d/lvp_icd.json WGPU_BACKEND=vulkan \
  nix develop --command cargo run -p character-mesh-export --release -- \
  --output maybraid/art/characters/exports/epiphant/epiphant_default_preview.obj \
  --zip /tmp/epiphant_default_preview_procreate.zip
```

## Procreate notes

Procreate imports USDZ and OBJ, and **requires a UV map**
([Import handbook](https://help.procreate.com/procreate/handbook/3d-painting/import)).
A lone `.obj` can be dragged onto the Gallery. Gallery → Import often greys `.obj`
out because of Files UTIs
([forum: greyed out](https://folio.procreate.com/help-support/59141)).

This exporter does **not** write `mtllib` / `usemtl` or a companion `.mtl`. The
file is the mesh: `o` per part, `v` / `vt` / `vn`, `s off`, faces.

OBJ record order matches Blender's working export: `o`, then `v`/`vt`/`vn`, then
`s off`, faces. `g` is omitted so Procreate does not invent extra layer groups
([forum](https://folio.procreate.com/help-support/65413)).

These structural checks do **not** prove Procreate will import the file. That has
to be verified on an iPad.
