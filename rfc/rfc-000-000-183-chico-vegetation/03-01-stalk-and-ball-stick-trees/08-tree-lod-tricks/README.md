# 3.1.8: Tree LOD Tricks

This page is subsection **3.1.8** of [RFC-183: Chico Vegetation](../../README.md)


This section outlines simple, high-impact techniques for reducing geometry and draw cost while preserving silhouette and visual variety across distance. The general principle is:

* preserve **silhouette first**
* preserve **mass second**
* drop **structure (branches) early**

Where possible, prefer **fewer meshes, fewer draw calls, and simpler topology** over geometric fidelity.

---

Subsections:

- [3.1.8.1: Performant Very Low-LOD Canopy](./01-performant-very-low-lod-canopy/README.md)
- [3.1.8.2: Performant Very Low-LOD Trunks](./02-performant-very-low-lod-trunks/README.md)
- [3.1.8.3: Performant Very Low-LOD Branches](./03-performant-very-low-lod-branches/README.md)
- [3.1.8.4: Performant Low-LOD Canopy](./04-performant-low-lod-canopy/README.md)
- [3.1.8.5: Performant Low-LOD Trunks](./05-performant-low-lod-trunks/README.md)
- [3.1.8.6: Performant Low-LOD Branches](./06-performant-low-lod-branches/README.md)
- [3.1.8.7: Performant Moderate-LOD Canopy](./07-performant-moderate-lod-canopy/README.md)
- [3.1.8.8: Performant Moderate-LOD Trunks](./08-performant-moderate-lod-trunks/README.md)
- [3.1.8.9: Performant Moderate-LOD Branches](./09-performant-moderate-lod-branches/README.md)
- [3.1.8.10: Varied Low-LOD Canopy](./10-varied-low-lod-canopy/README.md)
- [3.1.8.11: Varied Moderate-LOD Canopy](./11-varied-moderate-lod-canopy/README.md)
- [3.1.8.12: Silhouette-Preserving Scaling](./12-silhouette-preserving-scaling/README.md)
- [3.1.8.13: Random Rotation and Skew](./13-random-rotation-and-skew/README.md)
- [3.1.8.14: Vertical Color Gradient](./14-vertical-color-gradient/README.md)
- [3.1.8.15: Material Simplification](./15-material-simplification/README.md)

