# 3.1.7.6: Sope's Banyan

This page is subsection **3.1.7.6** of [RFC-183: Chico Vegetation](../../../README.md)


![Sope's Banyan](./assets/sopes-banyan.png)

Sope's Banyan is a banyan variant with a tall, vase-like crown. It begins from the [Honu Banyan](../05-honu-banyan/README.md#3175-honu-banyan) construction, but moves the canopy lower and biases branch growth upward, closer to the [Penmarch Torch](../04-penmarch-torch/README.md#3174-penmarch-torch). The result is a mystical, vertically rising banyan form suited to jungle, riparian, and elder-tree contexts.

**Shape**

* Thick banyan trunk
* Canopy begins around mid-height
* Wide but upward-projecting branch structure
* Periodic downward descenders
* Tall, vase-like silhouette

**Stalk**

Use the [Banyan Trunk](../../06-well-known-component-constructions/05-banyan-trunk/README.md#3165-banyan-trunk) construction.

```rust
let stalk_height = 0.75 * H;
let stalk_radius = 0.075 * H;
```

Use high-noise bark and strong trunk mass, as in [Honu Banyan](../05-honu-banyan/README.md#3175-honu-banyan).

**Anchor Rings**

Radial projections begin much lower than Honu Banyan, around $40%$ of total height.

```rust
let z_min = 0.40 * H;
let z_max = 0.90 * H;
let ring_spacing = 0.08 * H;
let anchors_per_ring = 6..=8;
```

Use several rings to build the rising crown:

```rust
let ring_count = 5..=7;
```

Anchors should originate near the stalk radial centroid, so the large upward limbs read as emerging from the trunk mass.

**Projection Length**

Use a vase-like widening profile. Let:

$$
u = \frac{z - z_{\min}}{z_{\max} - z_{\min}}
$$

Projection length follows a bounded vase profile (inverse sigmoid / logit over normalized height):

```rust
let min_projection_length = 0.25 * H;
let max_projection_length = 0.70 * H;

fn vase_profile(u: f32, eps: f32) -> f32 {
    let u = u.clamp(eps, 1.0 - eps);
    let a = ((1.0 - eps) / eps).ln();
    ((u / (1.0 - u)).ln() + a) / (2.0 * a)
}

let length = mix(
    min_projection_length,
    max_projection_length,
    vase_profile(u, 0.08),
);
```

This keeps the lower crown compact, opens quickly into a cup, then keeps widening toward the rim (more vase-like than a simple `sqrt(u)` ramp).

**Chain Growth**

Use long banyan-like chains with upward torch bias.

```rust
BallStickChain {
    segments: 5..=8,
    child_count: 1..=3,
    angle_tolerance: radians(12.0),
}
```

Bias rises with height, as in [Penmarch Torch](../04-penmarch-torch/README.md#3174-penmarch-torch):

```rust
let vertical_angle = mix(
    radians(25.0),
    radians(70.0),
    u,
);

let canopy_bias = rotate_up(radial, vertical_angle);
```

Descenders still occur every third to fourth segment:

```rust
fn hysteresis_for(ctx: ChainContext) -> HysteresisConfig {
    if ctx.segment_index % 4 == 0 {
        descender_config()
    } else {
        HysteresisConfig {
            bias_ray: canopy_bias,
            bias_strength: high,
            angle_tolerance: radians(12.0),
            child_count: 1..=3,
            length_range: medium..long,
            radius_range: medium..thin,
        }
    }
}
```

Descenders should remain strongly downward-biased, but may be slightly less frequent than in Honu Banyan if the crown should read more vertical than tangled.

**Ball Selection**

Allocate foliage broadly throughout the rising crown.

```rust
fn should_allocate_ball(ctx: BallSelectionContext) -> bool {
    ctx.height_fraction > 0.45
        || ctx.is_terminal
        || ctx.branch_order > 1
}
```

Use [Noisy Ball](../../02-ball-components/02-noisy-ball/README.md#3122-noisy-ball), [Plane Splay](../../02-ball-components/05-plane-splay/README.md#3125-plane-splay), and optional [Jungle Growths](../../06-well-known-component-constructions/04-jungle-growths/README.md#3164-jungle-growths) for dense variants.

```rust
let leaf_radius = 0.09 * H;
```

Descenders should receive sparse foliage, except where a denser mystical canopy is desired.

**Materials**

* Stick shader: dark banyan bark, wet bark, or high-contrast fantasy bark
* Leaf shader: dense jungle green, deep riparian green, or saturated mystical foliage
* Optional darker inner canopy balls for depth

**Variants**

* Increase total height and reduce descender frequency for an elder-tree silhouette.
* Add [Fruiting Bodies](../../06-well-known-component-constructions/07-fruiting-bodies/README.md#3167-fruiting-bodies) for mystical or ancient variants.
* Use [Crook Cylinder](../../01-stick-and-stalk-components/02-crook-cylinder/README.md#3112-crook-cylinder) on major limbs for a more twisted appearance.


