# Drumstick

![Drumstick](./assets/drumstick.jpeg)

The drumstick lower limb mesh or multi-mesh is a simple design found in nature. It is roughly a cylinder, but with a bulbous region.

**Semantics.** In local space, `height` runs along the limb segment (long axis). `width` and `depth` are the two cross-section extents orthogonal to that axis (ellipse or box); keep the same convention as the generator (half-extents vs full width) everywhere. **Top** is the **proximal** end of this asset (toward the body/trunk joint); **bottom** is distal.

Treat the segment as a **bulge** run (bulbous region) plus a **shaft** (non-bulge) run along `height`. Axial bulge length can be given as `drop` (from the proximal reference along the limb) or implied by `bulge_to_shaft_ratio` together with `height`; if both are supplied, the generator should enforce a single consistent split (e.g. derive one from the other).

## Parameters

**API note:** Names and shapes here are **suggestive**—they document intent until a generator or schema fixes the real types. In particular, bulge taper will likely need a **polynomial** (or coefficient list) over normalized position along the bulge, not a single scalar; the scalar below is a placeholder for “how much pinch.”

- `height`: total extent along the limb axis.
- `depth`: cross-section depth (orthogonal to `height`).
- `width`: cross-section width (orthogonal to `height`); interpret as the maximum across the bulge unless tapering pulls it down toward the bulge ends.
- `drop`: distance from the proximal end (**top**) to the distal end of the bulbous region (where the bulge ends toward the distal side)—i.e. axial length of the bulge when measured from that reference.
- `bulge_to_shaft_ratio`: ratio of axial bulge length to axial non-bulge (shaft) length (bulge length divided by shaft length). For a simple two-part segment, bulge length = `height * bulge_to_shaft_ratio / (1 + bulge_to_shaft_ratio)`.
- `bulge_width_taper`: placeholder for bulge **width** falloff from maximum toward the proximal and distal ends of the bulge (`0` ≈ constant; `1` ≈ full pinch to shaft width at the ends). Expect this to be replaced or generalized by a polynomial (or similar) taper curve later. Use the same curve for `depth` when the profile is isotropic; otherwise document a separate taper if you split them later.
- `offset`: optional shift along the limb axis of the bulbous region (e.g. moves where the bulb begins relative to the proximal end).

**Optional:** geodesic refinement or noise-driven bumps for surface detail—extras on top of the base silhouette, not required for the primitive shape.

## As a Lower Limb

The drumstick is a natural lower-limb candidate. It mimics real-world lower-limbs and can be placed on the skeleton such that the distal tips anchor to skeleton joints. 