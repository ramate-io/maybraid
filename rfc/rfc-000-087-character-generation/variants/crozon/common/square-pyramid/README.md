# Square pyramid

**Figure (TBD):** add a reference image at `./assets/square-pyramid.jpeg` when you have a sketch.

A **square pyramid**: square **base**, four triangular faces, one **apex**. This is not a regular **tetrahedron** (four triangular faces only). Use anywhere a form should **taper** from a broad base to a point or small cap.

**Semantics.** In local space, **`height`** is apex-to-base distance along the pyramid axis. **Base** dimensions (`base_width`, `base_depth`, or `base_size` if square) lie in the base plane. **Top** / **bottom** can label base vs apex according to embedding; for limbs, a common choice is **base** = proximal / thick end and **apex** = distal / narrow end (document per species). For **nose / snout**, the same taper often reads as a **beak**, **horn-snout**, or **wedge muzzle**: attach the **base** at the nasal bridge or upper jaw socket and point the **apex** forward/down along the facial forward axis.

**Flattening and orientation (cross-feature).** The same primitive becomes a **low wedge** when **`height` is small** compared to `base_width` / `base_depth` (a **flattened** pyramid—still mathematically a pyramid, not a frustum unless you add `apex_blunt`). Parent **rotation** then carries most of the silhouette intent:

- **Feet (hand–foot):** orient the solid **side-on**, so the **base plane** lies in the **sagittal** or **frontal** plane of the foot (pick per species) and the **shallow ridge** of the triangle reads as **instep / hoof wall / toe wedge** from the profile camera. The **apex** can aim **distal** along the foot or **lateral** for a splayed digit pad.
- **Head:** use a **flattish** pyramid (moderately low `height` vs base) as a **crown**, **forehead**, or **skull cap** wedge; pitch the frame, so the **apex sits slightly lower** than the base centerline (**nose-down** tilt) to avoid a “Party hat” read—think **sloped brow** or **helmet** roof meeting the face cylinder.
- **Ear:** a small flattened pyramid reads as a **triangular fin** or **alert ear** from the side; **twist** the base so the **ridge** aligns with the **helix** line on the head; often paired with a [dish](../dish/README.md) concha.

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
- [Horn](../../horn/square-pyramid/README.md)
- [Head shape](../../head-shape/square-pyramid/README.md) (flat crown / brow wedge, pitched down)
- [Hand and foot](../../hand-foot/square-pyramid/README.md) (flattened, side-on foot wedge)
- [Ear](../../ear/square-pyramid/README.md) (triangular fin ear)
