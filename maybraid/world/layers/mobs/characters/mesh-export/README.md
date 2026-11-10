# character-mesh-export

Headless exporter: spawn a character through the real fixed-assembly path, CPU-skin it, chart-unwrap UVs, and write a Wavefront OBJ/MTL plus PBR maps for DCC import (Procreate, Blender).

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
A lone `.obj` can be dragged in; a model with MTL + textures must be imported by
**dragging the folder** that contains the OBJ, MTL, and maps — Gallery → Import
often greys `.obj` out because of Files UTIs
([3D Painting help](https://help.procreate.com/articles/Z0mATv-3d-painting),
[forum: greyed out](https://folio.procreate.com/help-support/59141),
[forum: need obj+mtl+image](https://folio.procreate.com/help-support/49896)).

MTL texture keys Procreate documents:

- `map_Kd` colour
- `map_metallic`
- `map_roughness`

All maps are **2048×2048** (Procreate's documented 2K painting size; 4K is for
Pro iPads). Paths are relative to the MTL.

OBJ record order matches Blender's working export: `o`, then `v`/`vt`/`vn`, then
`s off`, `usemtl`, faces. `g` is omitted so Procreate does not invent extra layer
groups ([forum](https://folio.procreate.com/help-support/65413)). Faces are grouped
by material ([Blender T96511](https://archive.blender.org/developer/maniphest/0096/0096511/index.html)).
Apple Model I/O (used on iOS) resolves MTL and texture paths relative to the
referencing file
([kUTType3dObject](https://developer.apple.com/documentation/uniformtypeidentifiers/uttype-swift.struct/3dobject)).

These structural checks do **not** prove Procreate will import the file. That has
to be verified on an iPad.
