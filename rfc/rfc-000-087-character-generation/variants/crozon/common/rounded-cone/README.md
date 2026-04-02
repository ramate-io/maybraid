# Rounded cone

**Figure (TBD):** add a reference image at `./assets/rounded-cone.jpeg` when you have a sketch.

A cone-family primitive with a **rounded terminal cap** and **non-linear taper** along its length. Compared with a classic cone or frustum, this keeps directional narrowing while avoiding a needle tip and straight-line profile.

**Semantics.** In local space, `height` runs along the taper axis, `base_radius` defines the broad end, and `tip_radius` defines the rounded end before cap blending. The side profile follows a non-linear taper curve (for example ease-in/ease-out or spline), and the tip transitions into a rounded cap with curvature continuity.

## Parameters

**API note:** **Suggestive.** Represent as a radius function `r(t)` over normalized axis `t in [0, 1]`, plus a terminal cap blend segment.

- `height`: extent along the taper axis.
- `base_radius`: radius at the broad/base end.
- `tip_radius`: radius at the narrow end before terminal cap rounding.
- `taper_curve`: non-linear taper control (curve/spline/easing selector).
- `cap_roundness`: how strongly the terminal end rounds (0 = hard cutoff, 1 = full smooth cap).
- `axis_bend`: optional centerline curvature for gentle limb/head arcs.

## Crozon feature uses

- [Head shape](../../head-shape/rounded-cone/README.md) (angled-downward cranial wedge with soft tip)
- [Lower limb](../../lower-limb/rounded-cone/README.md) (organic shin/forelimb taper without sharp apex)
- [Horn](../../horn/rounded-cone/README.md) (tapered horn profile with soft terminal cap)
- [Ear](../../ear/rounded-cone/README.md) (cartilage-fin ear taper with rounded tip)
