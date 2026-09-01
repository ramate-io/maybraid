# Maybraid Firearms

Firearm recipes assembled from [`firearms-components`](../firearms-components/).

A [`FirearmKit`](src/kit.rs) is a required [`BodyMesh`](src/parts.rs) plus optional barrel / trigger-box / grip / stock. Named [`FirearmConcept`](src/concepts.rs) values are presets of that kit. Mix parts with `kit --trigger-box paddle --grip bump-handle`. Scale kit bones with `scale barrel --length 1.5 --thickness 0.8`.

Authoring (bone-space meshes, slots, armature tree): [`maybraid/art/items/guns/README.md`](../../art/items/guns/README.md).

```bash
cargo run -p items-playground
```

Blender sources: [`maybraid/art/items/guns/`](../../art/items/guns/). Runtime GLBs: `maybraid/assets/items/guns/`.
