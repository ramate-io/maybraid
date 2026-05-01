# 3.1.7.14: Friend's Conifer

This page is subsection **3.1.7.14** of [RFC-183: Chico Vegetation](../../../README.md)


Friend's Conifer is a fuller, more naturally rounded variant of the [Northern Conifer](../11-northern-conifer/README.md#31711-northern-conifer). It keeps the dense conifer ring structure and plane-splay foliage, but changes the projection-length profile so branch length remains nearly consistent through most of the tree before rounding inward near the top.

**Shape**

* Tall, narrow central stalk
* Dense radial branch rings
* Nearly consistent branch length through the lower and middle canopy
* Softly rounded top
* Fuller silhouette than [Liam's Conifer](../02-liam-s-conifer/README.md#3172-liams-conifer)

---

**Stalk**

Use the [Northern Conifer](../11-northern-conifer/README.md#31711-northern-conifer) stalk.

```rust
let stalk_height = H;
let stalk_radius = 0.025 * H;
```

---

**Anchor Rings**

Use the same dense conifer ring structure.

```rust
let z_min = 0.10 * H;
let z_max = 0.98 * H;
let ring_spacing = 0.04 * H;
let anchors_per_ring = 4;
```

---

**Projection Length**

Use a logarithmic rounding profile. The projection length should stay close to its maximum for most of the canopy, then fall off near the top.

Let:

```rust
let u = (z - z_min) / (z_max - z_min);
```

A useful profile is:

$$
\ell(u) = \ell_{\max}\left(1 - \frac{\log(1 + \alpha u^\beta)}{\log(1 + \alpha)}\right)
$$

...with $\beta > 1$ to delay the falloff.

```rust
let max_projection_length = 0.06 * H;
let min_projection_length = 0.015 * H;

let alpha = 8.0;
let beta = 3.0;

let falloff = (1.0 + alpha * u.powf(beta)).ln()
    / (1.0 + alpha).ln();

let projection_length = mix(
    max_projection_length,
    min_projection_length,
    falloff,
);
```

This keeps most branches similar in length, then rounds the upper canopy inward.

---

**Chain Growth**

Use the [Northern Conifer](../11-northern-conifer/README.md#31711-northern-conifer) branch structure, with short, slightly downward-biased projections.

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

Use [Plane Splay](../../02-ball-components/05-plane-splay/README.md#3125-plane-splay) at all ball-stick joints, as in [Northern Conifer](../11-northern-conifer/README.md#31711-northern-conifer).

```rust
fn should_allocate_ball(_ctx: BallSelectionContext) -> bool {
    true
}
```

```rust
let splay_radius = 0.018 * H;
let splay_count = 2..=4;
```

---

**Materials**

* Stick shader: dark conifer bark or cold-region bark
* Leaf shader: dark green, blue-green, snowy green, or alpine needle material

---

**Variants**

* Increase `beta` for a more cylindrical body and sharper top rounding.
* Lower `beta` for a more triangular conifer profile.
* Increase plane-splay density for spruce-like trees.

