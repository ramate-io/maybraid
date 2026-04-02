# Rounded cuboid

**Figure (TBD):** add a reference image at `./assets/rounded-cuboid.jpeg` when you have a sketch.

A **rectangular prism** with rounded edges and corners. In geometric terms this is a **filleted cuboid** (or chamfered if you use planar bevels instead of smooth rounds). It keeps the box-like mass while softening silhouettes and avoiding hard crate reads.

**Semantics.** In local space, `height` is the long axis, while `width` and `depth` define the rectangular cross-section. A single rounding control can apply to all twelve edges and eight corners, or you can split side/corner controls later for art direction.

## Parameters

**API note:** **Suggestive.** Implement as Minkowski sum (cuboid + sphere radius) for smooth fillets, or as explicit edge loops for low-poly control.

- `height`: extent along the primary axis.
- `width`, `depth`: cross-section spans orthogonal to `height`.
- `edge_radius`: fillet radius for edges and corners.
- `corner_profile`: optional profile selector (`round`, `bevel`, hybrid).
- `axis_taper`: optional linear taper along `height` while retaining rounded corners.

## Crozon feature uses

- [Torso](../../torso/rounded-cuboid/README.md) (soft block torso with shoulder-to-waist structure)
- [Head shape](../../head-shape/rounded-cuboid/README.md) (stylized cranium with softened planar reads)
