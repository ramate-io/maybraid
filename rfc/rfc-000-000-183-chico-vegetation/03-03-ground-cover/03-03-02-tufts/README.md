# 3.3.2: Tufts

This page is subsection **3.3.2** of [RFC-183: Chico Vegetation](../../README.md).

Extra volumetric detail is provided by tufts. These add discrete geometry to break up the flatness of bump-out-only surfaces.

Tufts are:

* SDF-based or mesh-based clumps
* sparsely distributed over bump-out regions
* oriented by terrain normal
* scaled and rotated deterministically

```rust
if noise(seed) > placement_threshold {
    spawn_tuft(
        position = terrain_position,
        direction = terrain_normal,
        scale = tuft_scale,
    );
}
```

As detailed in the [Tufts layer](../../03-05-cellular-forests/03-05-02-forest-layers/03-05-02-02-tufts-layer/README.md) of cellular forests, tufts should be handled as a **separate layer** from bump outs:

* bump outs define coverage and base density
* tufts provide localized vertical structure

This separation allows:

* independent LOD control
* independent density tuning
* better performance scaling

**Placement considerations**

* bias placement toward flatter regions or slight slopes
* avoid excessive clustering unless biome requires it
* reduce density near large vegetation or obstacles

**Usage**

* grasses and scrub
* jungle undergrowth
* dry brush
* moss clumps
* small flowering plants

Together, bump outs and tufts provide a scalable and performant ground cover system that integrates cleanly with terrain and vegetation layers.
