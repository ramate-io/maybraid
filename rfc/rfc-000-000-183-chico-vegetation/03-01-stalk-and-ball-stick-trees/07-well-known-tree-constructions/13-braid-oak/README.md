# 3.1.7.13: Braid Oak

This page is subsection **3.1.7.13** of [RFC-183: Chico Vegetation](../../../README.md)


The Braid Oak is a gnarled, expressive broadleaf tree with interweaving branch structure. It builds on the [Storybook Tree](../01-storybook-tree/README.md#3171-storybook-tree) but introduces strong directional variation and curvature, producing a braided, organic canopy with rich silhouette complexity.

**Shape**

* Moderate-height, sturdy stalk
* Lower branches droop and spread outward
* Mid-branches level out
* Upper branches rise and interweave
* Overall canopy feels braided or layered rather than radial

---

**Stalk**

Use a slightly thicker and more expressive stalk than Storybook.

```rust
let stalk_height = 0.75 * H;
let stalk_radius = 0.045 * H;
```

Prefer [Crook Cylinder](../../01-stick-and-stalk-components/02-crook-cylinder/README.md#3112-crook-cylinder) for the stalk to introduce subtle curvature and age.

---

**Anchor Rings**

Use Storybook-style rings.

```rust
let z_min = 0.15 * H;
let z_max = stalk_height;
let ring_spacing = 0.08 * H;
let anchors_per_ring = 6;
```

Anchors should originate near the stalk centroid.

---

**Projection Length**

Use a standard Storybook profile or slightly increased spread.

```rust
let min_projection_length = 0.15 * H;
let max_projection_length = 0.60 * H;
```

---

**Chain Growth**

Use moderate branching, but apply **height-dependent bias**:

Let:

```rust
let u = height_fraction;
```

Bias transitions from downward to upward:

```rust
let vertical_bias = mix(-0.35, 0.45, u);
let bias_ray = normalize(radial + Vec3::Y * vertical_bias);
```

* Lower branches droop and spread
* Mid0branches become more horizontal
* Upper branches rise and interweave

Use [Crook Cylinder](../../01-stick-and-stalk-components/02-crook-cylinder/README.md#3112-crook-cylinder) for all segments:

```rust
CrookCylinder {
    bend_x: small_to_medium,
    bend_z: small_to_medium,
    noise_amplitude: medium,
}
```

Increase angular variance slightly to encourage overlap and braiding:

```rust
angle_tolerance: radians(18.0),
child_count: 2..=3,
segments: 3..=6,
```

---

**Ball Selection**

Allocate foliage across mid-to-outer canopy, not strictly terminal.

```rust
fn should_allocate_ball(ctx: BallSelectionContext) -> bool {
    ctx.is_terminal
        || ctx.branch_order > 1
        || ctx.height_fraction > 0.35
}
```

Use a mix of:

* [Plane Splay](../../02-ball-components/05-plane-splay/README.md#3125-plane-splay) for outer canopy
* [Noisy Ball](../../02-ball-components/02-noisy-ball/README.md#3122-noisy-ball) for interior mass

```rust
let leaf_radius = 0.085 * H;
```

---

**Materials**

* Stick shader: dark, aged bark with high variation
* Leaf shader: broadleaf greens, autumn tones, or stylized variants

---

**Variants**

* Increase crook amplitude for older, more twisted oaks
* Add [Jungle Growths](../../06-well-known-component-constructions/04-jungle-growths/README.md#3164-jungle-growths) for overgrown variants
* Add [Fruiting Bodies](../../06-well-known-component-constructions/07-fruiting-bodies/README.md#3167-fruiting-bodies) for acorns or stylized fruit
* Reduce upward bias at top for flatter oak canopies

