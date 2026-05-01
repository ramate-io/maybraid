# 3.1.2.3: Octagonal Plane

This page is subsection **3.1.2.3** of [RFC-183: Chico Vegetation](../../../README.md)


A low triangle-count planar element used within splays.

**Construction**

* 8-sided polygon in local plane
* UVs centered for radial leaf textures

```rust
let positions = regular_ngon(8, radius);
```

**Usage**

* canopy layering in [Plane Splay](../05-plane-splay/README.md#3125-plane-splay)
* mid-detail foliage clusters

**Notes**

* Billboarded or slightly tilted
* Double-sided material recommended

---

