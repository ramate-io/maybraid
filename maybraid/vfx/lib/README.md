# Maybraid VFX

Real-time VFX: GPU particles, flipbooks, mesh lobes, sustained field meshes, and short-lived composite hosts.

Shaders and [`MaterialRef`](../../material-ref) recipes stay in their domain crates. This crate is the effect graph. Burst delivery is the fiery explosion from [#945](https://github.com/ramate-io/maybraid/issues/945): reusable `flash`, `fireball`, `smoke`, and `sparks` layers, composed as `fiery_explosion`. Sustained Bikanwarda looks from [#1122](https://github.com/ramate-io/maybraid/issues/1122) and [#1123](https://github.com/ramate-io/maybraid/issues/1123) are `god_beam` and `sonic_wave`.

Near fire and smoke use overlapping mesh lobes with a crate-owned stylized material. Driven effects use a separate [`FieldMaterial`](src/field_material.rs) so they do not collide with `firearms::EnergyMaterial`. Flipbook cards remain the distant / wisp layer. Spawn through `VfxLibrary` and `Commands::spawn_vfx`. Scale is spatial and applied once at init. Intensity scales particle count, emission, and field gain. Playback speeds the authored envelope without changing those ratios. `ExplosionPalette` recolors burst mesh, particles, and light; driven effects use `FieldPalette` (or map an explosion palette) and `tint` multiplies either. An omitted seed is generated; an explicit seed reproduces sampled variation. Shared Hanabi assets are never mutated per instance.

## Driven presentation API

Boss attacks own timing, hazards, and damage. This crate only plays looks. Drive an effect with [`VfxDrive`](src/drive.rs) and later call `commands.cancel_vfx(handle)` or `commands.despawn_vfx(handle)`.

[#1118](https://github.com/ramate-io/maybraid/issues/1118) / [PR #1147](https://github.com/ramate-io/maybraid/pull/1147) emit `AttackPresentationEvent` from `attacks`. This crate does **not** depend on that unmerged crate. [`attack_drive`](src/attack_drive.rs) mirrors the payloads so [#1119](https://github.com/ramate-io/maybraid/issues/1119) can plug the looks in:

1. Copy `instance` / `kind` / `phase` / `pose` / `shape` into `AttackDriveEvent`.
2. Call `AttackDriveBindings::apply`.
3. On a newly returned root, insert `attacks::AttackPresentationBinding { instance, kind }` so cancel / death / sleep / retirement despawn the same entity.

`HazardShape::Cylinder` → `god_beam` (`size` = diameter = `2 * radius`, `reach` = `height`). `HazardShape::Wavefront` → `sonic_wave` (`size` = `length`, `speed` = `speed`, one ring per hazard). Telegraph and Stage are ignored so windup does not double-play the column. Sonic-wave expansion is owned by the effect; Updated pose translation is not applied.

| Field | God Beam | Sonic wave |
| --- | --- | --- |
| `size` | column **diameter** (m). B04's 7 m beam is `7.0` | travel range (m) |
| `reach` | column height (m) | yaw spread (rad) |
| `speed` | unused | wavefront m/s |
| `count` | `1` | ring count |
| `lifetime` | active seconds (default 10) | travel seconds; `None` = range / speed |
| `far_point` | optional sky origin | unused |
| `fade_in` / `fade_out` | arrival / departure, also used by cancel | same |

```rust
let handle = commands.spawn_vfx(
    &library.god_beam,
    VfxSpawn {
        transform: Transform::from_translation(ground),
        drive: VfxDrive::god_beam()
            .with_size(7.0)
            .with_lifetime(10.0)
            .with_far_point(sky),
        seed: Some(seed),
        ..Default::default()
    },
);
commands.cancel_vfx(handle);
```

Sonic waves use the same spawn. Set `speed`, `size`, and `count` to the authoritative B04 wavefront; the VFX does not emit hits.

```text
cargo test -p maybraid-vfx
cargo test -p vfx-playground
cargo run -p vfx-playground -- god-beam --width 7 --lifetime 10
cargo run -p vfx-playground -- sonic-wave --range 28 --speed 22
```

