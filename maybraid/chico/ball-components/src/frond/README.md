# Frond crown

Mesh-based arching frond chains ([#218](https://github.com/ramate-io/maybraid/issues/218), [RFC-183 §3.1.2.7](https://github.com/ramate-io/maybraid/tree/main/rfc/rfc-000-000-183-chico-vegetation/03-01-stalk-and-ball-stick-trees/02-ball-components/07-fronds/README.md)).

## Variants

| Component | Module | Description |
|-----------|--------|-------------|
| [`FrondCrown`] | root | Segmented rachis + lateral leaflet **pairs** at each spine sample |
| [`ModerateLodFrondCrown`] | [`moderate_lod/`](moderate_lod.rs) | Shoot tube + dense lateral cards (~30–50 m) |

Shared: [`config.rs`](config.rs), [`spine.rs`](spine.rs), [`crown.rs`](crown.rs).

[`FrondCrownShape`](../frond.rs) uses `emission_lift_radians` + `downward_tilt_radians` for outward pitch (spread wobbles azimuth only) and `arch_lift` + `droop` on the spine for up-and-over strands (date palm defaults in `chico-sbs-trees`).

## Playground

```bash
cargo run -p chico-sbs-trees-playground -- render frond-crown --translate 0,2,0
cargo run -p chico-sbs-trees-playground -- render moderate-lod-frond-crown --translate 0,2,0
```

[`FrondCrown`]: ../frond.rs
[`ModerateLodFrondCrown`]: moderate_lod.rs

## See also

- [Tufts](../tuft.rs)
- [Buddha's-hand tuft](../tuft/buddha_hand/README.md)
