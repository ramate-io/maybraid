# 3.1.7.7: Rory's Head-trained

This page is subsection **3.1.7.7** of [RFC-183: Chico Vegetation](../../../README.md)


Rory's Head-trained is a top-heavy, trained tree form: a simple stalk with a thin, mostly horizontal canopy near the top. It is useful for arid trees, grape-vine-like bushes, ornamental plantings, and non-coniferous groves.

**Shape**

* Standard vertical stalk
* Single high canopy ring
* Thin horizontal spread
* Moderate branching
* Minimal lower foliage

**Stalk**

Use a standard [Noisy Cylinder](../../01-stick-and-stalk-components/01-noisy-cylinder/README.md#3111-noisy-cylinder).

```rust
let stalk_height = 0.90 * H;
let stalk_radius = 0.025 * H;
```

Keep the trunk relatively clean and readable.

```rust
NoisyCylinder {
    base_radius: stalk_radius,
    top_radius: stalk_radius * 0.55,
    noise_amplitude: 0.06 * stalk_radius,
    noise_frequency: medium,
}
```

**Anchor Ring**

Begin radial projections at $90%$ or more of total height. Use one ring layer.

```rust
let z = 0.90 * H;
let anchors_per_ring = 6..=8;
```

Anchors should originate near the stalk radial centroid.

```rust
for i in 0..anchors_per_ring {
    let theta = TAU * i as f32 / anchors_per_ring as f32;
    let radial = Vec3::new(theta.cos(), 0.0, theta.sin());

    anchor(
        position = stalk_centroid(z),
        initial_ray = radial,
        bias_ray = radial,
    );
}
```

**Projection Length**

Use moderate projection lengths similar to [Storybook Tree](../01-storybook-tree/README.md#3171-storybook-tree), but keep the canopy flatter.

```rust
let projection_length = 0.35 * H..0.55 * H;
```

For bush or grape-vine variants, reduce height and keep spread relatively wide:

```rust
let projection_length = 0.60 * H;
```

**Chain Growth**

Use moderate branching and segment length values.

```rust
BallStickChain {
    segments: 3..=5,
    child_count: 1..=3,
    angle_tolerance: radians(10.0),
}
```

Bias projections nearly horizontal:

```rust
let bias_ray = normalize(radial + Vec3::Y * 0.02);
```

Keep vertical variance small to maintain the trained canopy plane.

```rust
HysteresisConfig {
    bias_ray,
    bias_strength: high,
    angle_tolerance: radians(10.0),
    child_count: 1..=3,
}
```

**Ball Selection**

Allocate canopy primarily on terminal and outer nodes, preserving the thin horizontal profile.

```rust
fn should_allocate_ball(ctx: BallSelectionContext) -> bool {
    ctx.is_terminal
        || ctx.distance_from_anchor > 0.65 * ctx.max_projection_length
}
```

Use compact [Plane Splay](../../02-ball-components/05-plane-splay/README.md#3125-plane-splay), [Noisy Ball](../../02-ball-components/02-noisy-ball/README.md#3122-noisy-ball), or [Tufts](../../02-ball-components/06-tufts/README.md#3126-tufts) depending on species.

```rust
let leaf_radius = 0.06 * H;
```

Avoid dense interior allocation, since this tree should read as a trained crown rather than a full rounded canopy.

**Materials**

* Stick shader: dry bark, vineyard wood, or ornamental bark
* Leaf shader: broadleaf, vine, arid green, or cultivated foliage
* Optional [Fruiting Bodies](../../06-well-known-component-constructions/07-fruiting-bodies/README.md#3167-fruiting-bodies) for orchard or grape-like variants

**Variants**

* Shorten stalk and increase spread for bush or grape-vine forms.
* Add fruiting bodies for cultivated groves.
* Use sparse tufts for arid scrub variants.

