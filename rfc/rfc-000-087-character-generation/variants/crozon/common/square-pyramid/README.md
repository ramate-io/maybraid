# Square pyramid

**Figure (TBD):** add a reference image at `./assets/square-pyramid.jpeg` when you have a sketch.

A **square pyramid**: square **base**, four triangular faces, one **apex**. This is not a regular **tetrahedron** (four triangular faces only). Use anywhere a form should **taper** from a broad base to a point or small cap.

**Semantics.** In local space, **`height`** is apex-to-base distance along the pyramid axis. **Base** dimensions (`base_width`, `base_depth`, or `base_size` if square) lie in the base plane. **Top** / **bottom** can label base vs apex according to embedding; for limbs, a common choice is **base** = proximal / thick end and **apex** = distal / narrow end (document per species). For **nose / snout**, the same taper often reads as a **beak**, **horn-snout**, or **wedge muzzle**: attach the **base** at the nasal bridge or upper jaw socket and point the **apex** forward/down along the facial forward axis.

## Parameters

**API note:** **Suggestive.** Apex truncation (frustum), rounded apex, and non-square bases may appear later as extra fields or variants.

- `height`: apex-to-base distance along the pyramid axis.
- `base_width`, `base_depth`: base edge lengths (or full spans—match generator convention).
- `apex_blunt`: optional truncated apex or bevel (`0` = sharp, `> 0` = flat or rounded cap).
- `twist`: optional rotation of the base about the axis relative to a parent frame (document units in code).

**Optional:** facet shading via vertex normals; geodesic-style subdivision via a separate LOD parameter.

## Crozon feature uses

- [Lower limb](../../lower-limb/square-pyramid/README.md)
- [Nose and snout](../../nose-snout/square-pyramid/README.md) (beak, wedge snout, hard taper)
