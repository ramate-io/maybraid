# Maybraid Firearms

Firearm recipes assembled from [`firearms-components`](../firearms-components/).

A [`FirearmConcept`](src/concepts.rs) emits a shared receiver [`RigNode`] plus body / barrel / grip [`PartNode`]s. Kit pieces socket onto `body` / `barrel` / `grip` / `stock`. There is no stock mesh yet.

```bash
cargo run -p items-playground
```

Blender sources: [`maybraid/art/items/guns/`](../../art/items/guns/). Runtime GLBs: `maybraid/assets/items/guns/`.
