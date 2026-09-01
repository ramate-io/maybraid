# Maybraid Firearms

Firearm recipes assembled from [`firearms-components`](../firearms-components/).

A [`FirearmKit`](src/kit.rs) is a required [`BodyMesh`](src/parts.rs) plus optional barrel / trigger-box / grip / stock. Named [`FirearmConcept`](src/concepts.rs) values are presets of that kit (Bullpup fills barrel + grip; the others are body-only). Mix parts in the playground with `kit --barrel laznard`.

Authoring (bone-space meshes, slots, armature tree): [`maybraid/art/items/guns/README.md`](../../art/items/guns/README.md).

```bash
cargo run -p items-playground
```

Blender sources: [`maybraid/art/items/guns/`](../../art/items/guns/). Runtime GLBs: `maybraid/assets/items/guns/`.
