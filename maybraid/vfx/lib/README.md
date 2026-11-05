# Maybraid VFX

Real-time VFX: GPU particles, flipbooks, mesh lobes, sustained field meshes, and short-lived composite hosts.

Shaders and [`MaterialRef`](../../material-ref) recipes stay in their domain crates. This crate is the effect graph. Burst delivery is the fiery explosion from [#945](https://github.com/ramate-io/maybraid/issues/945): reusable `flash`, `fireball`, `smoke`, and `sparks` layers, composed as `fiery_explosion`. Sustained Bikanwarda looks from [#1122](https://github.com/ramate-io/maybraid/issues/1122) and [#1123](https://github.com/ramate-io/maybraid/issues/1123) are `god_beam` and `sonic_wave`.

Near fire and smoke use overlapping mesh lobes with a crate-owned stylized material. Driven effects use a separate [`FieldMaterial`](src/field_material.rs) so they do not collide with `firearms::EnergyMaterial`. Flipbook cards remain the distant / wisp layer. Spawn through `VfxLibrary` and `Commands::spawn_vfx`. Scale is spatial and applied once at init. Intensity scales particle count, emission, and field gain. Playback speeds the authored envelope without changing those ratios. `ExplosionPalette` recolors burst mesh, particles, and light; driven effects use `FieldPalette` (or map an explosion palette) and `tint` multiplies either. An omitted seed is generated; an explicit seed reproduces sampled variation. Shared Hanabi assets are never mutated per instance.

## Driven presentation API

Boss attacks own timing, hazards, and damage. This crate only plays looks. Drive an effect with [`VfxDrive`](src/drive.rs) and later call `commands.cancel_vfx(handle)` or `commands.despawn_vfx(handle)`.

Sonic rings **do not simulate travel**. Each ring renders a [`SonicWaveState`](src/drive.rs) snapshot written onto [`VfxWaveStates`](src/drive.rs). The playground has a tiny driver that advances those snapshots. [#1119](https://github.com/ramate-io/maybraid/issues/1119) (or a presenter in `attacks`) should write the same component from [`WavefrontPresentation`](https://github.com/ramate-io/maybraid/pull/1147). This crate does not depend on `attacks` and does not mirror its types.

### Mapping `#1147` `WavefrontPresentation` (for the attack presenter)

[PR #1147](https://github.com/ramate-io/maybraid/pull/1147) publishes the wave contract on `cursor/reusable-boss-attack-runtime-14d4`. Discrete rings leave the mouth at constant size, travel horizontally (optional pitch), and hit once per wave. Copy fields with [`SonicWaveState::follow`](src/drive.rs) — a thin adapter over the generic API, not a second schema.

| `WavefrontPresentation` | VFX |
| --- | --- |
| `origin` | `SonicWaveState.origin` (mouth at emit) |
| `direction` | `SonicWaveState.direction` — use as supplied; do not re-flatten |
| `position` | `SonicWaveState.position` — ring center; the mesh sits here |
| `radius` | `SonicWaveState.radius` — visual hoop matches the hit disc |
| `thickness` | `SonicWaveState.thickness` — visual depth matches `2 * half_depth` |
| `wave_index` | `SonicWaveState.index` |
| `range` | `SonicWaveState.range` and spawn `VfxDrive.size` |
| `speed` | spawn `VfxDrive.speed` (fade / playground driver only) |

The gameplay volume is a filled disc or short capsule. Drawing a ring is fine for now.

| `#1147` event | VFX |
| --- | --- |
| `Hazard` + `Cylinder { radius, height }` + pose | `god_beam`: `size = 2 * radius` (diameter), `reach = height`, transform at pose origin |
| `Hazard` + `wave: Some(WavefrontPresentation)` | `SonicWaveState::follow(origin, direction, position, radius, thickness, wave_index, range)` into `VfxWaveStates` |
| `Telegraph`, `Stage` | ignore |
| `Ended` / `Despawned` | `cancel_vfx` or `despawn_vfx` on the bound root |

Tag the spawned VFX root with `attacks::AttackPresentationBinding { instance, kind: Hazard { index, wave } }` so gameplay despawn on cancel / death / sleep / retirement still works. A three-ring Bray is one `sonic_wave` spawn with `count = 3`; the presenter writes one `SonicWaveState` per live `wave_index`.

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
