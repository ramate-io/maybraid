# 3.1.7.1: Storybook Tree

This page is subsection **3.1.7.1** of [RFC-183: Chico Vegetation](../../../README.md)


The Storybook Tree is the default broadleaf silhouette: a narrow central stalk with a rounded canopy assembled from moderately dense radial ball-stick projections. It is useful for deciduous forests, orchards, parks, and general-purpose background trees.

**Shape**

* Tall, fairly narrow stalk
* Rounded canopy beginning low on the upper trunk
* Lower branches longer than upper branches
* Moderate branching and soft radial spread

**Stalk**

Let $H$ be total tree height including canopy.

```rust
let stalk_height = 0.80 * H;
let stalk_radius = 0.035 * H;
```

Use a [Noisy Cylinder](../../01-stick-and-stalk-components/01-noisy-cylinder/README.md#3111-noisy-cylinder) for the stalk. Noise should be visible but not dominant.

```rust
NoisyCylinder {
    base_radius: stalk_radius,
    top_radius: stalk_radius * 0.55,
    noise_amplitude: 0.08 * stalk_radius,
    noise_frequency: medium,
}
```

**Anchor Rings**

Radial projections begin around **30%** along the stalk (clear trunk below the canopy belt) and continue toward the top of the stalk. Ring anchors are perturbed in position and direction like Sope's Banyan ([`StalkPerturbation`](../../../../../../maybraid/chico/sbs-geometry/src/anchors/stalk_perturbation.rs)); chain growth uses elevated noise frequency on each limb.

```rust
let z_min_frac = 0.30; // along stalk, 0 = base, 1 = tip
let z_max_frac = 1.0;
let ring_spacing = 0.10; // fraction of stalk height between rings
let anchors_per_ring = 6;
```

Each ring places anchors roughly every $60^\circ$:

```rust
for z in steps(z_min, z_max, ring_spacing) {
    for i in 0..anchors_per_ring {
        let theta = TAU * i as f32 / anchors_per_ring as f32;
        let radial = Vec3::new(theta.cos(), 0.0, theta.sin());

        anchor(position = stalk_centroid(z), initial_ray = radial);
    }
}
```

Anchors should originate near the stalk radial centroid to avoid detached-looking branches.

**Projection Length**

Branch reach should form a **rounded canopy belt**: shorter near the lower trunk and near the tip, longest around mid-height. Let:

$$
u = \frac{z - z_{\min}}{z_{\max} - z_{\min}}
$$

Use a dome (bell) profile that is low at both ends of the ring band:

$$
\ell(u) = \ell_{\min} + (\ell_{\max} - \ell_{\min}) \sin(\pi u)
$$

with:

```rust
let max_projection_length = 0.50 * H;
let end_fraction = 0.40; // ℓ_min = end_fraction * ℓ_max at u = 0 and u = 1
```

This produces a round canopy that arches outward in the middle without extreme short/long contrast between the lowest and highest rings.

**Chain Growth**

Each radial projection grows as a short ball-stick chain:

```rust
BallStickChain {
    segments: 3..=5,
    child_count: 1..=3, // mean near 2
    angle_tolerance: radians(26.0),
    bias_ray: radial,
    bias_strength: moderate,
}
```

The bias should be mostly horizontal, with slight upward variance for higher branches and slight downward variance for lower branches if a fuller canopy is desired.

**Ball Selection**

At high detail, allocate [Plane Splay](../../02-ball-components/05-plane-splay/README.md#3125-plane-splay) primarily on the outer canopy:

```rust
fn should_allocate_ball(ctx: BallSelectionContext) -> bool {
    ctx.is_terminal || ctx.distance_from_anchor > 0.65 * ctx.max_projection_length
}
```

Use a splay radius of roughly:

```rust
let leaf_radius = 0.09 * H;
```

Interior nodes should usually avoid foliage allocation unless a dense canopy is desired.

**Materials**

* Stick shader: bark or stylized trunk material
* Leaf shader: broadleaf, deciduous, orchard, or fantasy foliage
* Optional [Fruiting Bodies](../../06-well-known-component-constructions/07-fruiting-bodies/README.md#3167-fruiting-bodies) for orchard or magical variants

**Variants**

* Denser rings and larger leaf splays produce orchard trees.
* Smaller projection length and darker materials produce compact forest trees.
* Higher angular variance and [Crook Cylinder](../../01-stick-and-stalk-components/02-crook-cylinder/README.md#3112-crook-cylinder) segments produce older or more whimsical silhouettes.

