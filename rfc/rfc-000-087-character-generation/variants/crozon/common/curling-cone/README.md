# Curling cone

**Figure (TBD):** add a reference image at `./assets/curling-cone.jpeg` when you have a sketch.

A **cone** (or **frustum**) whose **axis follows a curved path**—typically a **planar arc** or **helix**—so the tip **curls** back toward the root or sweeps out in a spiral. Cross-sections stay **circular** (or elliptical) orthogonal to the local spine tangent. Use for **ram**/**antelope** horns, **shell** spirals at low poly, or **rolled** ear tips where a straight [Cylinder](../cylinder/README.md) or broad [Bow](../bow/README.md) spine is the wrong abstraction.

**Semantics.** Define a **spine curve** (arc, helix, or spline—implementation-specific) from **root** to **tip**. Along the spine, **radius** tapers from `base_radius` at the root to `tip_radius` (≥ `0` for a point). **Binormal** frame twists can carry `twist_angle` for faceted horns. **Opening** direction at the root matches the attachment socket on the skull or [head-shape](../../head-shape/README.md).

## Parameters

**API note:** **Suggestive.** Spine may be analytic (circular arc + pitch) or sampled; taper may be linear or ease curve.

- `base_radius`, `tip_radius`: cone frustum at root and tip (orthogonal to spine).
- `arc_angle` (or `turns` + `pitch` for helix): how far the spine bends or winds (e.g. 180° = half loop, 540° = ram curl).
- `spine_length`: arc length or chord budget along the path.
- `taper_power`: optional exponent on linear taper (thick base, needle tip vs blunt tip).
- `twist`: roll of the cross-section frame about the spine (degrees/radians—document in code).

**Optional:** faceted **N-gon** (regular polygon) approximation instead of a smooth cone; cap mesh at root for clean boolean into a skull boss.

## Crozon feature uses

- [Horn](../../horn/curling-cone/README.md)
- [Ear](../../ear/curling-cone/README.md)
