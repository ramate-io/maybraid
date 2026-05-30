# Jungle growths

Secondary foliage at **one canopy anchor** ([#226](https://github.com/ramate-io/maybraid/issues/226), [RFC-183 §3.1.6.4](https://github.com/ramate-io/maybraid/tree/main/rfc/rfc-000-000-183-chico-vegetation/03-01-stalk-and-ball-stick-trees/06-well-known-component-constructions/04-jungle-growths/README.md)).

Each [`JungleGrowth`] instance spawns:

1. A **scaled darker [`ChicoBall`](../../ball-components)** — inner dirt/wood mass (`inner-ball-scale` × node radius), [`SkippedBodyMeshMaterial`].
2. A **[`WeepingTuft`](../../ball-components)** — protruding wet/overgrown foliage, [`SkippedFoliageMeshMaterial`].

**Node selection** (which anchors receive growth) is owned by the composing tree recipe, not this module.

## Usage

```rust
JungleGrowth {
    shape: JungleGrowthShape { seed: mix_seed(node_idx, node.position), .. },
    body_material: bark.clone(),
    foliage_material: darker_leaf.clone(),
    ..
}
.spawn_at(commands, cascade_chunk, Transform {
    translation: node.position,
    rotation: Quat::from_rotation_arc(Vec3::Y, outward_bias),
    scale: Vec3::splat(node.radius),
});
```

## Modules

| File | Role |
|------|------|
| [`config.rs`](config.rs) | `JungleGrowthShape` — scales, seed, embedded tuft shape |
| [`assembly.rs`](assembly.rs) | `JungleGrowth` mesh builders + `RenderItem` spawn |

## Follow-up

- Wire into [`SopesBanyan`](../../sbs-trees/src/sopes_banyan.rs) behind a dense-variant flag
- Playground toggle / clap on tree commands
