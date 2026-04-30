# 3.3.1: Bump Outs

This page is subsection **3.3.1** of [RFC-183: Chico Vegetation](../../README.md).

Ground cover primarily relies on a similar [bump out](https://github.com/ramate-io/maybraid/tree/main/rfc/rfc-000-000-170-terrain-detail#34-bump-outs) method to RFC-170. These modify the underlying terrain SDF to introduce small-scale height variation representing grass beds, moss, soft soil, or low vegetation mats.

Construction follows:

* define a cell or region
* sample noise to determine coverage
* apply a bounded vertical displacement to the terrain SDF

```rust
let mask = noise(world_position * scale);

if mask > threshold {
    let height = amplitude * smooth(mask);
    sdf += height;
}
```

Key characteristics:

* **continuous**: no discrete meshes required
* **cheap**: operates in terrain generation phase
* **stable**: tied to world-space coordinates
* **biome-driven**: parameters vary with terrain conditions

Detail is primarily expressed through [Leaf Shaders](../../03-01-stalk-and-ball-stick-trees/03-01-10-leaf-shading/README.md):

* color variation (greens, yellows, browns)
* seasonal effects (drying, snow cover)
* flecking (flowers, debris, moss variation)

Bump outs provide the **base visual mass** of ground vegetation.
