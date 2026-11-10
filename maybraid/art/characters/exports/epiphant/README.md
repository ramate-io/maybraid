# Epiphant default-preview OBJ

Wavefront export of `EpiphantConfig::default_preview()`, assembled the same way as
runtime (`spawn_fixed_character_assembly`). Rest / bind pose, Y-up, metres.

| File | Role |
| --- | --- |
| `epiphant_default_preview.obj` | One `o` object per part (body, head, ears, eyes, nose, tail) |
| `epiphant_default_preview.mtl` | Per-part materials with relative `map_Kd` / `map_metallic` / `map_roughness` |
| `epiphant_default_preview_*_color.png` | 2048×2048 albedo filled with the part's palette `Kd` |
| `epiphant_default_preview_metallic.png` | Shared 2048×2048 metallic (black — organic) |
| `epiphant_default_preview_roughness.png` | Shared 2048×2048 roughness (mid grey) |

Each part is a texture set: independent chart unwrap in 0..1 with island padding,
so mirrored left/right surfaces are separately paintable. Degenerate geometry may
have tiny UVs; every nondegenerate triangle has nonzero UV area.

## Texture resolution

**2048×2048** for every map. Procreate documents 2K vs 4K painting resolution and
requires every image in a material to share a size
([3D Painting help](https://help.procreate.com/articles/Z0mATv-3d-painting)).
2048 is the size that fits consumer iPads; 4K is for Pro-class devices. Solid-colour
PNGs stay small on disk; Procreate unpacks them at import.

## Rebuild

```bash
nix develop --command cargo run -p character-mesh-export --release -- \
  --species epiphant \
  --output maybraid/art/characters/exports/epiphant/epiphant_default_preview.obj
```

See [`mesh-export/README.md`](../../../../world/layers/mobs/characters/mesh-export/README.md).

## Import in Procreate (manual)

Actual Procreate import is the acceptance check and is **not** verified in CI.

1. Copy the OBJ, MTL, and PNGs into one local **On My iPad** folder (not iCloud).
   Extract a zip first if you were given one — do not import the zip.
2. Gallery → Import often greys `.obj` out. That is a Files UTI issue; official
   import is drag-and-drop
   ([handbook](https://help.procreate.com/procreate/handbook/3d-painting/import)).
3. Split View Files + Procreate. Long-press the **folder**, drag onto the Gallery
   until a green plus appears, drop.
4. Open the model, tap a mesh, paint. Confirm left/right ears and eyes paint
   independently, and that no triangle is an unpaintable collapsed island.

If the file is greyed out but still opens when tapped, that matches
[community reports](https://folio.procreate.com/help-support/59141) and is not a
failure.
