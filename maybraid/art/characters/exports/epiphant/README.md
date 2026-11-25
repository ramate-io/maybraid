# Epiphant default-preview OBJ

Wavefront export of `EpiphantConfig::default_preview()`, assembled the same way as
runtime (`spawn_fixed_character_assembly`), CPU-skinned into character root space.
Y-up, metres. `--freeze-clock` holds animation time at 0 so the baked pose is
repeatable; without it the pose is whatever idle-clip frame load time reached.

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
  --species epiphant --freeze-clock \
  --output maybraid/art/characters/exports/epiphant/epiphant_default_preview.obj
```

See [`mesh-export/README.md`](../../../../world/layers/mobs/characters/mesh-export/README.md).

## Runtime GLB (Bevy): painted maps on the skinned parts

This OBJ is static. To render Liam's painted maps on the animated Epiphant, re-bind
the painted OBJ onto the live part meshes:

```bash
VK_ICD_FILENAMES=/usr/share/vulkan/icd.d/lvp_icd.json WGPU_BACKEND=vulkan \
  cargo run -p character-mesh-export --release -- \
  --rebind-painted maybraid/art/characters/exports/epiphant/epiphant_default_preview.obj \
  --glb-output maybraid/assets/characters/epiphant/default/epiphant_default_preview.glb
```

The exporter assembles the character again (clock frozen, the same pose as this
OBJ) and matches each painted triangle to the part triangle it came from by baked
corner positions and normals. It writes one glTF mesh per component (`Mesh0` body,
`Mesh1` head, `Mesh2` ear_left, `Mesh3` ear_right, `Mesh4` eye_left, `Mesh5`
eye_right, `Mesh6` nose, `Mesh7` tail) in that part mesh's own local space:
painted `TEXCOORD_0` (V flipped for glTF), the part's own positions and normals,
and for skinned parts (body, head) the part GLB's own `JOINTS_0` / `WEIGHTS_0`. No
weights are painted or invented. It fails if any corner is more than 1 cm off, any
normal disagrees, or a triangle cannot take its own source triangle.

Re-run it whenever the OBJ is repainted with a new unwrap or the part GLBs change.
Bikanwarda keeps the ordinary skinned / socketed Epiphant parts and swaps each part's
mesh for its painted mesh at runtime (`bikanwarda-character` `textures.rs`).

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
