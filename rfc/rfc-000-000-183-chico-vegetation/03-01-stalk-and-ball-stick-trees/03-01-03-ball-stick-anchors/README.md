# 3.1.3: Ball-stick Anchors

This page is subsection **3.1.3** of [RFC-183: Chico Vegetation](../../README.md)


Ball-stick anchors define where a branch, canopy chain, or foliage complex begins. The current system does this implicitly through [radial projection from an ad hoc stalk](https://github.com/ramate-io/maybraid/blob/cebdaf75f0ce2d837ddc818a9a2658abb3d738dd/procedures/vegetation/src/tree.rs#L171), then passes the resulting start point and ray into a [`BallStick`](https://github.com/ramate-io/maybraid/blob/cebdaf75f0ce2d837ddc818a9a2658abb3d738dd/procedures/comproc/src/complex/chain/ball_stick/builder.rs) construction.

The refinement is to treat anchoring as a named step. An anchor records the start position, initial growth direction, bias direction, and local scale for a branch or canopy chain.

```rust
pub struct BallStickAnchor {
    pub position: Vec3,
    pub initial_ray: Vec3,
    pub bias_ray: Vec3,
    pub radius: f32,
}
```

Anchors are usually placed at or near the **stalk radial centroid** rather than directly on the visible stalk surface. This reduces the chance that branches appear detached or improperly projected. The branch mesh can still emerge visually from the stalk surface because the first stick segment projects outward from the centroid.

Common anchor routines include:

* stalk-height sampling for ordinary branches
* radial rings around the stalk for canopies
* crown-only rings for palms
* node-derived anchors for secondary growth
* downward-biased anchors for banyan descenders
* ground anchors for bushes, shoots, and trunkless palms

Tree recipes should compose these routines rather than hard-code anchor placement. For example:

```rust
let anchors = stalk_rings(...)
    .with_height_bias(...)
    .with_radial_count(...)
    .with_direction_rule(...);
```

This makes constructions such as conifers, palms, banyans, and vase trees variations over anchor selection rather than separate systems.

---

