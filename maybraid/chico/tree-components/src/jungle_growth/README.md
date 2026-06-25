# Jungle growths

Secondary foliage at **one canopy anchor** ([#226](https://github.com/ramate-io/maybraid/issues/226), [RFC-183 §3.1.6.4](https://github.com/ramate-io/maybraid/tree/main/rfc/rfc-000-000-183-chico-vegetation/03-01-stalk-and-ball-stick-trees/06-well-known-component-constructions/04-jungle-growths/README.md)).

Each [`JungleGrowth`] instance spawns:

1. A **scaled [`ChicoBall`](../../../ball-components/)** — inner dirt/wood mass (`inner-ball-scale` × node radius), [`SkippedBodyMeshMaterial`].
2. A **[`FrondCrown`](../../../ball-components/src/frond.rs)** — outward arching shoots anchored at the inner-ball apex, draping over the mass.
3. A **[`BuddhaHandTuft`](../../../ball-components)** — upward fingers buried below the crown to conceal the anchor (fixed offset, not configurable).

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

Body, frond crown, and Buddha's-hand spawn as **children** of one assembly root (local lifts/scales), so the cluster stays composed when the anchor `Transform` moves.

## Modules

| File | Role |
|------|------|
| [`config.rs`](config.rs) | `JungleGrowthShape` — scales, seed, frond + Buddha's-hand defaults |
| [`assembly.rs`](assembly.rs) | `JungleGrowth` mesh builders + `RenderItem` spawn |

## Follow-up

- Wire into [`SopesBanyan`](../../../sbs-trees/src/sopes_banyan.rs) behind a dense-variant flag
