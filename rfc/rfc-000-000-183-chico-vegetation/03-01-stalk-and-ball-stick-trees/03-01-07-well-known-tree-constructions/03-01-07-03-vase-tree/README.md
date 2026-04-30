# 3.1.7.3: Vase Tree

This page is subsection **3.1.7.3** of [RFC-183: Chico Vegetation](../../../README.md)


The Vase Tree is a broad, upward-opening tree form. It starts from the [Storybook Tree](../03-01-07-01-storybook-tree/README.md#3171-storybook-tree) construction but inverts the canopy profile, so radial projections grow wider toward the top. This gives a head-trained, vase-like silhouette useful for ornamental trees, mystical forests, bushes, and urban plantings.

**Shape**

* Narrow to moderate stalk
* Canopy opens upward and outward
* Upper branches are longer than lower branches
* Lower branches are strongly upward-biased
* Bias relaxes closer to horizontal near the top

**Stalk**

Use the same stalk construction as [Storybook Tree](../03-01-07-01-storybook-tree/README.md#3171-storybook-tree), optionally shortened slightly for bush or ornamental variants.

```rust
let stalk_height = 0.75 * H;
let stalk_radius = 0.035 * H;
```

Use a [Noisy Cylinder](../../03-01-01-stick-and-stalk-components/03-01-01-01-noisy-cylinder/README.md#3111-noisy-cylinder) or [Crook Cylinder](../../03-01-01-stick-and-stalk-components/03-01-01-02-crook-cylinder/README.md#3112-crook-cylinder) depending on desired stylization.

**Anchor Rings**

Use Storybook-style radial rings, but favor upper canopy density.

```rust
let z_min = 0.20 * H;
let z_max = stalk_height;
let ring_spacing = 0.08 * H;
let anchors_per_ring = 6;
```

Anchors should originate near the stalk radial centroid.

Yes — you’re right. A normal sigmoid gives more of a **chalice** profile.

For the vase, you want something closer to an **inverse sigmoid radius profile**: fast widening near the bottom, slower widening through the middle, then renewed flare near the rim.

A clean construction is to use the **logit-like inverse sigmoid shape**, but keep it bounded:

```rust
fn vase_profile(u: f32, eps: f32) -> f32 {
    let u = u.clamp(eps, 1.0 - eps);

    let x = (u / (1.0 - u)).ln();

    // remap from [-a, a] into [0, 1]
    let a = ((1.0 - eps) / eps).ln();
    (x + a) / (2.0 * a)
}
```

Then:

```rust
let cup = vase_profile(u, 0.08);

let projection_length = mix(
    min_projection_length,
    max_projection_length,
    cup,
);
```

Proposal wording:

---

**Projection Length**

Use a bounded inverse-sigmoid profile over height. This gives the vase or calyx shape: rapid widening near the base of the crown, slower widening through the middle, and renewed flare near the rim.

Let:

$$
u = \frac{z - z_{\min}}{z_{\max} - z_{\min}}
$$

Use a clamped inverse sigmoid:

$$
v(u) =
\frac{
\log\left(\frac{u}{1-u}\right) + a
}{
2a
}
$$

where:

$$
a = \log\left(\frac{1-\epsilon}{\epsilon}\right)
$$

...and $u$ is clamped to $[\epsilon, 1-\epsilon]$.

Then:

$$
\ell(u) = \ell_{\min} + (\ell_{\max} - \ell_{\min})v(u)
$$

```rust
fn vase_profile(u: f32, eps: f32) -> f32 {
    let u = u.clamp(eps, 1.0 - eps);
    let a = ((1.0 - eps) / eps).ln();

    ((u / (1.0 - u)).ln() + a) / (2.0 * a)
}

let projection_length = mix(
    min_projection_length,
    max_projection_length,
    vase_profile(u, 0.08),
);
```

This produces the desired “flower cup” profile rather than the squared-off chalice profile of a direct sigmoid.

> [!NOTE]
> You can play with this inverse sigmoid shape at the Desmos plot [here](https://www.desmos.com/calculator/vvytytkb8u).

**Chain Growth**

Use a Storybook-like chain with moderate branching.

```rust
BallStickChain {
    segments: 3..=5,
    child_count: 1..=3,
    angle_tolerance: radians(15.0),
}
```

Bias should start strongly upward and approach horizontal as height increases.

```rust
let vertical_angle = mix(
    radians(45.0),
    radians(5.0),
    u,
);

let bias_ray = rotate_up(radial, vertical_angle);
```

This opens the canopy like a vase: lower branches climb sharply, while upper branches spread outward.

**Ball Selection**

Allocate foliage mostly on upper and outer nodes.

```rust
fn should_allocate_ball(ctx: BallSelectionContext) -> bool {
    ctx.is_terminal
        || ctx.height_fraction > 0.60
        || ctx.distance_from_anchor > 0.60 * ctx.max_projection_length
}
```

Use [Plane Splay](../../03-01-02-ball-components/03-01-02-05-plane-splay/README.md#3125-plane-splay) at high detail and [Noisy Ball](../../03-01-02-ball-components/03-01-02-02-noisy-ball/README.md#3122-noisy-ball) or icospheres at lower detail.

```rust
let leaf_radius = 0.08 * H;
```

**Materials**

* Stick shader: deciduous bark, ornamental bark, or stylized dark bark
* Leaf shader: broadleaf, flowering, magical, or urban ornamental foliage
* Optional [Fruiting Bodies](../../03-01-06-well-known-component-constructions/03-01-06-07-fruiting-bodies/README.md#3167-fruiting-bodies) for orchard-like variants

**Variants**

* Shorter stalk and denser upper branches produce a bush form.
* Higher upward bias produces a flame-like ornamental tree.
* Crook cylinders add a trained or sculpted garden appearance.

