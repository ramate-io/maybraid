# 3.1.8.1: Performant Very Low-LOD Canopy

This page is subsection **3.1.8.1** of [RFC-183: Chico Vegetation](../../../README.md)


Use a single primitive to approximate canopy mass:
* upside-down square pyramid
* squashed tetrahedron
* scaled and rotated triangle.

These shapes:

* approximate canopy taper
* are extremely cheap (4–5 faces)
* read well at distance when shaded correctly

```rust
spawn_mesh(upside_down_pyramid(scale));
```

Use slight vertical squash for broader canopies.

---

