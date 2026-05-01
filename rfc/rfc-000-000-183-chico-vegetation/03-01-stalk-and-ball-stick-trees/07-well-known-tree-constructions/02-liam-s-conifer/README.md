# 3.1.7.2: Liam's Conifer

This page is subsection **3.1.7.2** of [RFC-183: Chico Vegetation](../../../README.md)


Liam's Conifer is a sparse, dry conifer silhouette: a narrow vertical stalk with many short, lightly downward-biased radial projections. It is useful for drier conifer stands, semi-arid forests, and lighter woodland edges.

**Shape**

* Tall, narrow central stalk
* Short radial projections
* Sparse branching
* Tuft-based canopy at most ball-stick joints
* Slight downward branch bias

**Stalk**

Let $H$ be total tree height.

```rust
let stalk_height = H;
let stalk_radius = 0.025 * H;
```

Use a [Noisy Cylinder](../../01-stick-and-stalk-components/01-noisy-cylinder/README.md#3111-noisy-cylinder) with modest taper and low to medium noise.

```rust
NoisyCylinder {
    base_radius: stalk_radius,
    top_radius: stalk_radius * 0.35,
    noise_amplitude: 0.06 * stalk_radius,
    noise_frequency: medium,
}
```

**Anchor Rings**

Radial projections begin at roughly $10%$ of height and continue nearly to the top.

```rust
let z_min = 0.10 * H;
let z_max = 0.98 * H;
let ring_spacing = 0.04 * H;
let anchors_per_ring = 4;
```

Each ring places anchors roughly every $90^\circ$:

```rust
for z in steps(z_min, z_max, ring_spacing) {
    for i in 0..anchors_per_ring {
        let theta = TAU * i as f32 / anchors_per_ring as f32;
        let radial = Vec3::new(theta.cos(), 0.0, theta.sin());

        anchor(position = stalk_centroid(z), initial_ray = radial);
    }
}
```

**Projection Length**

Upper projections shrink linearly relative to lower projections.

Let:

$$
u = \frac{z - z_{\min}}{z_{\max} - z_{\min}}
$$

Then:

$$
\ell(u) = \ell_{\max}(1 - u)
$$

with:

```rust
let max_projection_length = 0.05 * H;
```

Optionally clamp to preserve a small top silhouette:

```rust
let length = max(0.20 * max_projection_length, max_projection_length * (1.0 - u));
```

**Chain Growth**

Each projection uses a long first segment followed by two short segments.

```rust
BallStickChain {
    segments: 3,
    segment_lengths: [
        0.70 * projection_length,
        0.15 * projection_length,
        0.15 * projection_length,
    ],
    child_count: 1..=2, // mean close to 1
    angle_tolerance: radians(8.0),
}
```

Bias the projection slightly downward:

```rust
let downward_bias = rotate_down(radial, radians(2.0));
```

Use tight hysteresis, so branches remain sparse and readable.

```rust
HysteresisConfig {
    bias_ray: downward_bias,
    bias_strength: high,
    angle_tolerance: radians(8.0),
    child_count: 1..=2,
}
```

**Ball Selection**

Allocate [Tufts](../../02-ball-components/06-tufts/README.md#3126-tufts) at all ball-stick joints.

```rust
fn should_allocate_ball(_ctx: BallSelectionContext) -> bool {
    true
}
```

Use two to three tufts per joint:

```rust
let tuft_count = 2..=3;
let tuft_scale = 0.02 * H;
```

Tufts should follow the branch direction with mild upward spread to avoid a purely flat silhouette.

**Materials**

* Stick shader: lighter bark or dry trunk tones
* Leaf shader: pale green, dusty green, or dry conifer tones

**Variants**

* Increasing ring density produces fuller conifers.
* Replacing tufts with [Plane Splay](../../02-ball-components/05-plane-splay/README.md#3125-plane-splay) produces a northern conifer variant.
* Increasing downward bias gives a drooping alpine silhouette.

