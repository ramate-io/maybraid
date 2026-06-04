# High-bush shoots

Trunkless radial shoot construction at a ground anchor ([#225](https://github.com/ramate-io/maybraid/issues/225), [RFC §3.1.6.3](https://github.com/ramate-io/maybraid/tree/main/rfc/rfc-000-000-183-chico-vegetation/03-01-stalk-and-ball-stick-trees/06-well-known-component-constructions/03-high-bushes-and-shoots/README.md)).

[`HighBushShoots`] builds one shared ball-stick graph from [`HighBushShootsShape`], spawns woody [`ChicoStick`](../../stick-components) segments, and allocates [`PlaneSplay`](../../ball-components) or [`SucculentTuft`](../../ball-components) foliage using the Common High Bush ball-selection rule ([#233](https://github.com/ramate-io/maybraid/issues/233)).

## Usage

```rust
HighBushShoots {
    shape: HighBushShootsShape { height: 12.0, foliage_style: HighBushFoliageStyle::Tuft, .. },
    stick_material: bark.clone(),
    leaf_material: leaf.clone(),
    ..
}
// RenderItem::spawn_render_items(...)
```

## Modules

| File | Role |
|------|------|
| [`config.rs`](config.rs) | `HighBushShootsShape`, `HighBushFoliageStyle` |
| [`preset.rs`](preset.rs) | Common High Bush RFC constants + `apply_common_high_bush_preset` |
| [`canopy.rs`](canopy.rs) | Ball / tuft selection and render rules |
| [`stick.rs`](stick.rs) | Stick render rule |
| [`assembly.rs`](assembly.rs) | `HighBushShoots` + `RenderItem` |
