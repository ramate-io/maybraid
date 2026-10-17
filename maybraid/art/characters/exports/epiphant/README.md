# Epiphant default-preview OBJ

Wavefront export of `EpiphantConfig::default_preview()`, assembled the same way as runtime (`spawn_fixed_character_assembly`).

- `epiphant_default_preview.obj` — one `o` object per part (body, head, ears, eyes, nose, tail)
- `epiphant_default_preview.mtl` — palette `Kd` only (no image textures)

Rest / bind pose, Y-up, metres. Each part has its own non-overlapping UV island in 0..1. Face indices use independent `v` / `vt` / `vn` streams.

```bash
cargo run -p character-mesh-export -- \
  --species epiphant \
  --output maybraid/art/characters/exports/epiphant/epiphant_default_preview.obj
```

See [`mesh-export/README.md`](../../../../world/layers/mobs/characters/mesh-export/README.md).
