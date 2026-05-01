# 3.1.8.2: Performant Very Low-LOD Trunks

This page is subsection **3.1.8.2** of [RFC-183: Chico Vegetation](../../../README.md)


Use a stretched tetrahedron or square pyramid.

```rust
spawn_mesh(stretched_pyramid(height, radius));
```

These give:

* strong vertical read
* minimal geometry
* acceptable silhouette at long range

---

