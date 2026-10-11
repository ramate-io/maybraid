# Maybraid VFX

Real-time VFX: GPU particles, flipbooks, mesh lobes, sustained field meshes, and short-lived composite hosts.

Shaders and [`MaterialRef`](../../material-ref) recipes stay in their domain crates. This crate is the effect graph. Burst delivery is the fiery explosion from [#945](https://github.com/ramate-io/maybraid/issues/945): reusable `flash`, `fireball`, `smoke`, and `sparks` layers, composed as `fiery_explosion`. Sustained Bikanwarda looks from [#1122](https://github.com/ramate-io/maybraid/issues/1122) and [#1123](https://github.com/ramate-io/maybraid/issues/1123) are `god_beam` and `sonic_wave`.

Near fire and smoke use overlapping mesh lobes with a crate-owned stylized material. Driven effects use a separate [`FieldMaterial`](src/field_material.rs) so they do not collide with `firearms::EnergyMaterial`. Flipbook cards remain the distant / wisp layer. Spawn through `VfxLibrary` and `Commands::spawn_vfx`. Scale is spatial and applied once at init. Intensity scales particle count, emission, and field gain. Playback speeds the authored envelope without changing those ratios. `ExplosionPalette` recolors burst mesh, particles, and light; driven effects use `FieldPalette` (or map an explosion palette) and `tint` multiplies either. An omitted seed is generated; an explicit seed reproduces sampled variation. Shared Hanabi assets are never mutated per instance.

## Driven presentation API

Boss attacks own timing, hazards, and damage. This crate only plays looks. Drive an effect with [`VfxDrive`](src/drive.rs) and later call `commands.cancel_vfx(handle)` or `commands.despawn_vfx(handle)`.

Sonic rings **do not simulate travel**. Each ring renders a [`SonicWaveState`](src/drive.rs) snapshot (origin, direction, current position, radius, thickness, wave index, range) written onto [`VfxWaveStates`](src/drive.rs). The playground has a tiny driver that advances those snapshots. [#1119](https://github.com/ramate-io/maybraid/issues/1119) (or a presenter in `attacks`) should do the same from `AttackPresentationEvent`. This crate does not depend on `attacks` and does not mirror its types.

### Mapping `#1147` events (for the attack presenter)

[#1118](https://github.com/ramate-io/maybraid/issues/1118) / [PR #1147](https://github.com/ramate-io/maybraid/pull/1147) emit `AttackPresentationEvent`. Suggested mapping, owned by the presenter:

| `#1147` payload | VFX |
| --- | --- |
| `Hazard` + `Cylinder { radius, height }` + pose | `god_beam`: `size = 2 * radius` (diameter), `reach = height`, transform at pose origin |
| `Hazard` + `Wavefront { radius, speed, length }` + pose | write `SonicWaveState`: `origin` = emission socket at `Spawned`, `direction` = flattened pose forward, `position` = supplied pose origin each `Updated` (current volume center), `radius` = `Wavefront.radius` (damage disc, **not** travel growth), `range` = `length`, `index` = hazard index. `speed` is for the presenter only. |
| `Telegraph`, `Stage` | ignore (no double column / no extra rings on windup) |
| `Ended` / `Despawned` | `cancel_vfx` or `despawn_vfx` on the bound root |

Tag the spawned VFX root with `attacks::AttackPresentationBinding` so gameplay despawn on cancel / death / sleep / retirement still works. A three-ring Bray is one `sonic_wave` spawn with `count = 3`; the presenter writes three `SonicWaveState` rows.

| Field | God Beam | Sonic wave |
| --- | --- | --- |
| `size` | column **diameter** (m). B04's 7 m beam is `7.0` | travel **range** (m) |
| `reach` | column height (m) | fan cone (rad); `0` = one path |
| `speed` | unused | travel m/s (presenter / playground driver) |
| `count` | `1` | sequential rings |
| `radius` | unused | constant ring radius (m) |
| `thickness` | unused | ring depth along travel (m) |
| `pitch` | unused | `None` keeps travel horizontal |
| `interval` | unused | seconds between sequential rings |
| `lifetime` | active seconds (default 10) | travel seconds; `None` = range / speed |
| `far_point` | optional sky origin | unused |
| `fade_in` / `fade_out` | arrival / departure, also used by cancel | fade at range / cancel |

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

```text
cargo test -p maybraid-vfx
cargo test -p vfx-playground
cargo run -p vfx-playground -- god-beam --width 7 --lifetime 10
cargo run -p vfx-playground -- sonic-wave --range 20 --speed 22 --count 3 --radius 0.55
```
