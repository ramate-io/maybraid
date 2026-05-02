# 3.1.7.13: Jungle Storybook Tree

This page is subsection **3.1.7.13** of [RFC-183: Chico Vegetation](../../../README.md)


The Jungle Storybook Tree is a dense, overgrown variant of the [Storybook Tree](../01-storybook-tree/README.md#3171-storybook-tree). Rather than simply adding [Jungle Growths](../../06-well-known-component-constructions/04-jungle-growths/README.md#3164-jungle-growths), this construction increases canopy density, introduces layered foliage, and adds secondary growth behaviors to achieve a humid, entangled jungle appearance.

**Shape**

* Retains general Storybook silhouette
* Denser, more layered canopy
* Reduced interior visibility
* Secondary growth protruding from branches
* Slight downward weight in canopy mass

---

**Base Construction**

Start from the [Storybook Tree](../01-storybook-tree/README.md#3171-storybook-tree), but adjust:

* increase anchor density slightly
* reduce angular symmetry
* increase branching slightly

```rust
let anchors_per_ring = 6..=8;
let child_count = 2..=3;
```

---

**Projection Length**

Slightly compress the canopy vertically and expand it laterally.

```rust
let max_projection_length = 0.65 * H;
let min_projection_length = 0.15 * H;
```

Optionally bias the profile toward a flatter crown:

```rust
let projection_length = mix(min_projection_length, max_projection_length, sigmoid(u, 10.0, 0.4));
```

---

**Chain Growth**

Increase branching and introduce mild downward drift to give weight to the canopy.

```rust
HysteresisConfig {
    bias_ray: normalize(radial + Vec3::Y * 0.15),
    bias_strength: medium,
    angle_tolerance: radians(18.0),
    child_count: 2..=3,
}
```

Occasionally introduce slight downward perturbations:

```rust
if noise(seed, segment_index) < 0.2 {
    bias_ray += -Vec3::Y * 0.25;
}
```

---

**Ball Selection**

Unlike the base Storybook Tree, allocate foliage throughout the canopy, not just outer layers.

```rust
fn should_allocate_ball(ctx: BallSelectionContext) -> bool {
    ctx.height_fraction > 0.40
        || ctx.is_terminal
        || ctx.branch_order > 1
}
```

Use a mix of components:

* [Plane Splay](../../02-ball-components/05-plane-splay/README.md#3125-plane-splay) for outer canopy
* [Noisy Ball](../../02-ball-components/02-noisy-ball/README.md#3122-noisy-ball) for inner mass
* occasional [Tufts](../../02-ball-components/06-tufts/README.md#3126-tufts) for irregular protrusions

```rust
let leaf_radius = 0.09 * H;
```

---

**Jungle Growths**

Apply [Jungle Growths](../../06-well-known-component-constructions/04-jungle-growths/README.md#3164-jungle-growths) at selected canopy nodes:

```rust
if noise(seed, node_id) < 0.4 {
    spawn_jungle_growth(node);
}
```

This adds:

* darker secondary balls
* tufts
* localized density and visual noise

---

**Secondary Effects**

Introduce layered variation:

* slightly darker interior foliage
* higher saturation in outer canopy
* irregular clustering

Optional additions:

* sparse [Fruiting Bodies](../../06-well-known-component-constructions/07-fruiting-bodies/README.md#3167-fruiting-bodies)
* occasional short descender-like branches

---

**Materials**

* Stick shader: darker, higher-contrast bark
* Leaf shader: saturated greens, wet foliage tones
* Inner canopy: slightly darker or desaturated

---

**Variants**

* Increase jungle growth density for rainforest canopy
* Add short descenders for proto-banyan hybrid
* Increase downward bias for heavier, humid appearance
* Mix in [Plane Splay](../../02-ball-components/05-plane-splay/README.md#3125-plane-splay) and [Tufts](../../02-ball-components/06-tufts/README.md#3126-tufts) for layered foliage complexity

