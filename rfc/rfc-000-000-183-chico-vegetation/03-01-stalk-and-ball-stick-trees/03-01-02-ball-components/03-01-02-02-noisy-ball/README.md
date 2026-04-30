# 3.1.2.2: Noisy Ball

This page is subsection **3.1.2.2** of [RFC-183: Chico Vegetation](../../../README.md)


An SDF-backed spherical canopy element with surface perturbation.

**Construction**

$$
d(\mathbf{p}) = |\mathbf{p}| - r + \text{noise}(\mathbf{p})
$$

```rust
fn distance(p: Vec3) -> f32 {
    let n = perlin(p * freq + seed) * amp;
    p.length() - radius + n
}
```

Mesh generation proceeds via marching cubes or dual contouring.

**Usage**

* mid-range canopy fill
* base layer for higher-detail canopy, e.g. [Plane Splay](../03-01-02-05-plane-splay/README.md#3125-plane-splay)

**Notes**

* One-sided shading at range
* Two-sided shading up close
* Can be replaced by icosahedra at low LOD

---

