# 3.1.2.1: Icosahedron

This page is subsection **3.1.2.1** of [RFC-183: Chico Vegetation](../../../README.md)


A low-poly convex canopy primitive used primarily at far range.

**Construction**

* Static indexed mesh: 12 vertices, 20 faces
* Can be precomputed or reused via asset handle

In Bevy:

```rust
let mesh = Mesh::from(shape::Icosahedron {
    radius,
    subdivisions: 0,
});
```

**Usage**

* far LOD canopy fill
* silhouette preservation
* cheap instancing across large forests

**Notes**

* One-sided opaque shading is sufficient at distance
* Icospheres (`subdivisions > 0`) may be used for moderate LOD
* Can replace [Noisy Balls](../03-01-02-02-noisy-ball/README.md#3122-noisy-ball) in [Plane Splays](../03-01-02-05-plane-splay/README.md#3125-plane-splay)

---

