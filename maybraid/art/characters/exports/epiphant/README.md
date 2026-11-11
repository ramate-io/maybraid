# Epiphant default-preview OBJ

Wavefront export of `EpiphantConfig::default_preview()`, assembled the same way as
runtime (`spawn_fixed_character_assembly`). Rest / bind pose, Y-up, metres.

Mesh only: one OBJ with `vt` UVs and `vn` normals. No MTL, no texture images.

| File | Role |
| --- | --- |
| `epiphant_default_preview.obj` | One `o` object per part (body, head, ears, eyes, nose, tail) |

Each part has an independent chart unwrap in 0..1 with island padding, so
mirrored left/right surfaces are separately paintable. Degenerate geometry may
have tiny UVs; every nondegenerate triangle has nonzero UV area.

## Rebuild

```bash
nix develop --command cargo run -p character-mesh-export --release -- \
  --species epiphant \
  --output maybraid/art/characters/exports/epiphant/epiphant_default_preview.obj
```

See [`mesh-export/README.md`](../../../../world/layers/mobs/characters/mesh-export/README.md).

## Import in Procreate (manual)

Actual Procreate import is the acceptance check and is **not** verified in CI.

1. Copy `epiphant_default_preview.obj` onto the iPad (On My iPad, not iCloud).
   Extract a zip first if you were given one — do not import the zip.
2. Gallery → Import often greys `.obj` out. That is a Files UTI issue; official
   import is drag-and-drop of the lone OBJ
   ([handbook](https://help.procreate.com/procreate/handbook/3d-painting/import)).
3. Split View Files + Procreate. Drag the `.obj` onto the Gallery until a green
   plus appears, drop.
4. Open the model, tap a mesh, paint. Confirm left/right ears and eyes paint
   independently, and that no triangle is an unpaintable collapsed island.

If the file is greyed out but still opens when tapped, that matches
[community reports](https://folio.procreate.com/help-support/59141) and is not a
failure.
