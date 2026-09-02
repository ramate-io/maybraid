# Item art

Blender sources for handheld items. Runtime GLBs mirror this layout under `maybraid/assets/items/`. Firearm paths are registered in [`firearms-components` `assets.rs`](../../items/firearms-components/src/assets.rs).

Firearm kit authoring (bone-space meshes, slots, hand landmarks) is in [`guns/README.md`](guns/README.md).

| Folder | Role |
|--------|------|
| [`guns/rigs/`](guns/rigs/) | Shared firearm receiver armature |
| [`guns/bodies/`](guns/bodies/) | Receiver / body meshes |
| [`guns/barrels/`](guns/barrels/) | Barrel meshes |
| [`guns/trigger_boxes/`](guns/trigger_boxes/) | Fire-control box meshes |
| [`guns/grips/`](guns/grips/) | Grip meshes |
| [`guns/stocks/`](guns/stocks/) | Stock meshes (none yet) |
| [`guns/concepts/`](guns/concepts/) | Baked one-mesh kits |
| [`melee/blades/`](melee/blades/) | Blade meshes |
| [`melee/guards/`](melee/guards/) | Guard meshes |
| [`melee/handles/`](melee/handles/) | Handle meshes |
