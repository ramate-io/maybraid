# Maybraid Firearms

Firearm recipes assembled from [`firearms-components`](../firearms-components/).

A [`FirearmKit`](src/kit.rs) is a required [`BodyMesh`](src/parts.rs) plus optional barrel / trigger-box / grip / stock. Named [`FirearmConcept`](src/concepts.rs) values are presets of that kit. Mix parts with `kit --trigger-box paddle --grip bump-handle`. Scale kit bones with `scale barrel --length 1.5 --thickness 0.8`.

[`FirearmWeaponsPlugin`](src/projectiles.rs) auto-fires emissive shots from the receiver `barrel` bone. Put a [`Weapon`](src/projectiles.rs) on the [`FirearmRoot`](src/lib.rs):

| Load | Shape | Motion |
|------|--------|--------|
| Bolt | Capsule (length, radius, speed) | No gravity, despawn at max range |
| Bullet | Same capsule | Gravity on, despawn at max range |
| Laser | Beam along bone +Y | Grows from the muzzle, wraps after max time |

Muzzle is the barrel tail (`bone-local +Y` of rest length 1). Kits author along bone +Y; rotate the host if the range should fire world +X.

Authoring (bone-space meshes, slots, armature tree): [`maybraid/art/items/guns/README.md`](../../art/items/guns/README.md).

```bash
cargo run -p items-playground
cargo run -p firing-range-playground
```

Blender sources: [`maybraid/art/items/guns/`](../../art/items/guns/). Runtime GLBs: `maybraid/assets/items/guns/`.
