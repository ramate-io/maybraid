# character-mesh-export

Headless exporter: spawn a character through the real fixed-assembly path, CPU-skin it, chart-unwrap UVs, and write a Wavefront OBJ with **one material and one color PNG per independently paintable component**.

GLB parts often arrive with unique vertices per corner, so charts grow across **geometric** edges (quantized positions), not index adjacency. Islands stay connected surfaces (torso, legs, ear flaps), not one triangle per island. Each component occupies its own 0..1 tile; left and right ears are separate images, and an ear's front/back charts pack into that ear's image without overlapping.

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

`--texture-resolution` defaults to **1024** (square, per component). Materials use white `Kd` so the PNG supplies the colour (`map_Kd {component}_color.png`). Metallic/roughness maps are not written.

## Procreate notes

Procreate imports USDZ and OBJ, and **requires a UV map**
([Import handbook](https://help.procreate.com/procreate/handbook/3d-painting/import)).
A model with MTL + textures must be imported by **dragging the folder** that contains
the OBJ, MTL, and maps. Gallery → Import often greys `.obj` out because of Files UTIs
([forum: greyed out](https://folio.procreate.com/help-support/59141)).

Distinct material *names* are not enough: Procreate's texture sets come from image maps.
Painting one component must not mark another at the same UV coordinates, so each part
gets its own `map_Kd` file even when the initial colours match.

OBJ record order: `mtllib`, then per object `o`, `v`/`vt`/`vn`, `s off`, `usemtl`, faces.
`g` is omitted so Procreate does not invent extra layer groups
([forum](https://folio.procreate.com/help-support/65413)).

These structural checks do **not** prove Procreate will isolate texture sets. That has
to be verified on an iPad with Paint through mesh disabled.
