# Maybraid Firearms

Firearm recipes assembled from [`firearms-components`](../firearms-components/).

A [`FirearmConcept`](src/concepts.rs) emits body / barrel / grip [`PartNode`]s, the same way a character species emits rigs and parts. Kit pieces socket onto a receiver armature (`barrel`, `grip`) when one is present; until then they parent under the firearm root at authored pose.

```bash
# later: a playground will `cargo run -p firearms-playground`
```

Blender sources: [`maybraid/art/items/guns/`](../../art/items/guns/). Runtime GLBs: `maybraid/assets/items/guns/`.
