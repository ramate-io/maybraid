# Temperate Conifer

This file is part of [RFC-183: Chico Vegetation](../../../README.md).

**Construction type:** tree construction (see section 3.1.7 in the main RFC).


The Temperate Conifer is a sparse, fronded variant of [Friend's Conifer](../../../README.md#31714-friends-conifer). It keeps the rounded conifer profile but replaces plane-splay foliage with [Fronds](../../../README.md#3127-fronds), giving the canopy a lighter, more articulated texture.

**Shape**

* Tall, narrow central stalk
* Rounded conifer silhouette
* Sparse frond-based foliage
* Open branch visibility
* Works well when scaled down into strange bushes

---

**Stalk**

Use the [Friend's Conifer](../../../README.md#31714-friends-conifer) stalk.

```rust
let stalk_height = H;
let stalk_radius = 0.025 * H;
```

---

**Anchor Rings**

Use the same conifer ring structure.

```rust
let z_min = 0.10 * H;
let z_max = 0.98 * H;
let ring_spacing = 0.04 * H;
let anchors_per_ring = 4;
```

---

**Projection Length**

Use the same logarithmic rounding profile from [Friend's Conifer](../../../README.md#31714-friends-conifer), preserving the almost cylindrical body and rounded top.

```rust
let max_projection_length = 0.06 * H;
let min_projection_length = 0.015 * H;
let alpha = 8.0;
let beta = 3.0;
```

---

**Chain Growth**

Use the same short, slightly downward-biased conifer branch structure.

```rust
BallStickChain {
    segments: 3,
    segment_lengths: [
        0.70 * projection_length,
        0.15 * projection_length,
        0.15 * projection_length,
    ],
    child_count: 1..=2,
    angle_tolerance: radians(8.0),
}
```

```rust
let bias_ray = rotate_down(radial, radians(2.0));
```

---

**Ball Selection**

Allocate foliage at all ball-stick joints, but use [Fronds](../../../README.md#3127-fronds) instead of [Plane Splay](../../../README.md#3125-plane-splay).

```rust
fn should_allocate_ball(_ctx: BallSelectionContext) -> bool {
    true
}
```

Fronds should be short and narrow, oriented along or slightly below the branch direction.

```rust
FrondConfig {
    segments: 5..=8,
    length: 0.035 * H..0.07 * H,
    width: 0.012 * H,
    droop: low_to_medium,
    twist: mild,
    leaflet_count: 6..=10,
}
```

Use fewer fronds per joint than a palm crown:

```rust
let fronds_per_joint = 1..=2;
```

---

**Materials**

* Stick shader: dry conifer bark or semi-arid bark
* Leaf shader: muted green, dusty green, or tropical dry foliage

---

**Variants**

* Scale down for strange bushes or ornamental shrubs.
* Increase frond length for tropical or semi-arid variants.
* Reduce frond count for sparse dryland silhouettes.
