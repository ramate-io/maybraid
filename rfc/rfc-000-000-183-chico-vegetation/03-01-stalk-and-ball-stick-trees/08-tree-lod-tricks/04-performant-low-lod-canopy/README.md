# 3.1.8.4: Performant Low-LOD Canopy

This page is subsection **3.1.8.4** of [RFC-183: Chico Vegetation](../../../README.md)


Use stretched icosahedra to approximate canopy shape:

* one vertical icosahedron for tall forms
* one horizontal (squashed) icosahedron for wide forms
* combine two for vase-like or complex shapes

```rust
spawn_mesh(icosahedron(scale));
```

This preserves:

* rounded silhouette
* better shading than pyramids
* low triangle count

---

