# 3.1.8.15: Material Simplification

This page is subsection **3.1.8.15** of [RFC-183: Chico Vegetation](../../../README.md)


Reduce shader complexity at lower LODs:

* remove normal maps
* reduce texture lookups
* flatten roughness variation

This reduces GPU cost and improves batching while maintaining overall color and silhouette.

---

These techniques combine to produce large, varied forests at low cost while preserving convincing silhouettes and biome identity.

