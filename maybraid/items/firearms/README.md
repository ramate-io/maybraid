# Maybraid Firearms

Firearm recipes assembled from [`firearms-components`](../firearms-components/).

A [`FirearmKit`](src/kit.rs) is a required [`BodyMesh`](src/parts.rs) plus optional barrel / trigger-box / grip / stock. Named [`FirearmConcept`](src/concepts.rs) values are presets of that kit. Mix parts with `kit --trigger-box paddle --grip bump-handle`. Scale kit bones with `scale barrel --length 1.5 --thickness 0.8`.

[`FirearmWeaponsPlugin`](src/projectiles.rs) fires emissive shots from the receiver `barrel` bone. Put a [`Weapon`](src/projectiles.rs) on the [`FirearmRoot`](src/lib.rs). Auto-fire is the default; add [`FireOnTrigger`](src/projectiles.rs) to require [`TriggerFire`](src/projectiles.rs) (right trigger / click in the firing range):

| Load | Shape | Motion |
|------|--------|--------|
| Bolt | Capsule (length, radius, speed) | No gravity; despawn when path, through-solid, or age is exhausted |
| Bullet | Same capsule | Gravity on; same budgets |
| Laser | Beam along bone +Y | Grows from the muzzle, wraps after max time (no contact physics) |

Muzzle is the barrel tail (`bone-local +Y` of rest length 1). Runtime rest (after the armature’s glTF +90° X) has bore along +Z and grip down; [`aim_plus_x`](src/pose.rs) yaws that onto world +X. Bolts and bullets are query-only; they sweep [`Fixed`](../../lod/avian/src/layers.rs) and charge [`Flight::through`](src/projectiles.rs) with optional [`PenetrationCost`](src/projectiles.rs).

Authoring (bone-space meshes, slots, armature tree): [`maybraid/art/items/guns/README.md`](../../art/items/guns/README.md).

```bash
cargo run -p items-playground
cargo run -p firing-range-playground
```

Blender sources: [`maybraid/art/items/guns/`](../../art/items/guns/). Runtime GLBs: `maybraid/assets/items/guns/`.
