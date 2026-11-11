# Epiphant default-preview OBJ

Wavefront export of `EpiphantConfig::default_preview()`, assembled the same way as
runtime (`spawn_fixed_character_assembly`). Rest / bind pose, Y-up, metres.

Each independently paintable component is its own `o` object, `*_mat` material, and
`*_color.png` (1024×1024, filled with that part's palette colour). MTL `Kd` is white
so the image supplies the colour.

| File | Role |
| --- | --- |
| `epiphant_default_preview.obj` | One `o` per part: `body`, `head`, `ear_left`, `ear_right`, `eye_left`, `eye_right`, `nose`, `tail` |
| `epiphant_default_preview.mtl` | `newmtl {part}_mat` with `Kd 1 1 1`, `d 1`, `map_Kd {part}_color.png` |
| `{part}_color.png` | Unique 1024×1024 albedo for that part (not shared, even if colours match) |

UVs are a chart unwrap **per component** in 0..1 with island padding. Left/right ears
are different images; each ear's front and back occupy distinct UV space in that image.
Degenerate geometry may have tiny UVs; every nondegenerate triangle has nonzero UV area.

## Rebuild

```bash
nix develop --command cargo run -p character-mesh-export --release -- \
  --species epiphant \
  --output maybraid/art/characters/exports/epiphant/epiphant_default_preview.obj
```

See [`mesh-export/README.md`](../../../../world/layers/mobs/characters/mesh-export/README.md).

## Import in Procreate (manual)

Actual Procreate import is the acceptance check and is **not** verified in CI.

1. Put the OBJ, MTL, and PNGs in **one local On My iPad folder** (not iCloud).
   Extract a zip first if you were given one — do not import the zip.
2. Gallery → Import often greys `.obj` out. Official import is drag-and-drop of the
   **folder** ([handbook](https://help.procreate.com/procreate/handbook/3d-painting/import)).
3. Split View Files + Procreate. Long-press the folder, drag onto the Gallery until a
   green plus appears, drop. That creates a fresh 3D artwork.
4. Open the model. Confirm a separate texture set per component.
5. Disable **Paint through mesh**. Paint the head: body and both ears must not change.
   Paint the left ear: the right ear must not change.

If the file is greyed out but still opens when tapped, that matches
[community reports](https://folio.procreate.com/help-support/59141) and is not a
failure.
