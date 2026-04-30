# 3.1.7.12: Common High Bush

This page is subsection **3.1.7.12** of [RFC-183: Chico Vegetation](../../../README.md)


The Common High Bush is a trunkless or near-trunkless shrub form built from upward-biased radial shoots. It is useful as a bush, small tree, understory plant, hedge element, or filler vegetation in most biomes.

**Shape**

* No dominant central trunk
* Seven to ten upward radial shoots
* Rounded or vase-like shrub silhouette
* Dense foliage near outer and terminal nodes
* Works with many bark and leaf shaders

**Anchors**

Use the [High-bushes and Shoots](../../03-01-06-well-known-component-constructions/03-01-06-03-high-bushes-and-shoots/README.md#3163-high-bushes-and-shoots) construction from a ground or near-ground anchor.

```rust
let shoot_count = 7..=10;
let anchor = ground_position + Vec3::Y * (0.02 * H);
```

Distribute shoots radially:

```rust
for i in 0..shoot_count {
    let theta = TAU * i as f32 / shoot_count as f32;
    let radial = Vec3::new(theta.cos(), 0.0, theta.sin());

    let dir = normalize(radial * 0.45 + Vec3::Y * 0.75);

    grow_chain(anchor, dir);
}
```

**Chain Growth**

Use short to moderate ball-stick chains with upward bias.

```rust
BallStickChain {
    segments: 3..=5,
    child_count: 1..=2,
    angle_tolerance: radians(12.0),
}
```

Keep branches readable but not too sparse:

```rust
HysteresisConfig {
    bias_ray: dir,
    bias_strength: high,
    angle_tolerance: radians(12.0),
    child_count: 1..=2,
    length_range: 0.08 * H..0.16 * H,
    radius_range: 0.012 * H..0.025 * H,
}
```

**Ball Selection**

Allocate foliage on terminal and upper nodes, with moderate interior fill for bush density.

```rust
fn should_allocate_ball(ctx: BallSelectionContext) -> bool {
    ctx.is_terminal
        || ctx.height_fraction > 0.45
        || ctx.branch_order > 1
}
```

Use [Plane Splay](../../03-01-02-ball-components/03-01-02-05-plane-splay/README.md#3125-plane-splay), [Noisy Ball](../../03-01-02-ball-components/03-01-02-02-noisy-ball/README.md#3122-noisy-ball), or [Tufts](../../03-01-02-ball-components/03-01-02-06-tufts/README.md#3126-tufts) depending on style.

```rust
let leaf_radius = 0.05 * H;
```

**Materials**

* Stick shader: shrub bark, green woody stems, dry brush, or stylized bark
* Leaf shader: broadleaf, dry chaparral, jungle green, flowering, or ornamental foliage

**Variants**

* Use tufts for scrub or dry brush.
* Use plane splays for leafy bushes.
* Add [Fruiting Bodies](../../03-01-06-well-known-component-constructions/03-01-06-07-fruiting-bodies/README.md#3167-fruiting-bodies) for berry bushes.
* Reduce height and increase shoot count for hedge-like forms.

