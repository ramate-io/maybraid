# Richmond Building Components

This crate contains various scene components for Richmond buildings.

> [!NOTE]
> All components implement [`lod::gen::LodScene`](../../lod/lib/src/gen/presentation.rs) so they can be used in the scene graph and with generation-presentation flows.

## Swept Components

We have not yet defined a sweeping tool. The plan is to make it take linear segments in \(X \in [-1.0, 1.0]\) and extrude/fill them along a path (line or arc).

## Partitions

Partition components are authored in a normalized local space, then transformed into world/cell space by the parent building.

- **Linear Normalization:** linear components are normalized to the following spaces:
  - \(Z = [-0.2, 0.2]\)
  - \(Y = [0.0, 1.0]\)
  - \(X = [-1.0, 1.0]\)
  - Subsegments normalized to \(X = [-1.0, 0.8]\)
- **Angular Normalization:** angular components follow a similar normalization along the arc, but attach to different start and end points at different angles.
  - Thickness is the same swept \(Z = [-0.2, 0.2]\)
  - A 180° arc sweep goes through \(-Z\) from \(X = -1.0\) to \(X = 1.0\)
  - A 90° arc sweep goes through \(-Z\) from \(X = -1.0\) to \(X = 0.0\)
  - A 15° arc sweep goes through \(-Z\) from \(X = -1.0\) to \(X = \cos(15^\circ) - 1.0\), \(Z = -\sin(15^\circ)\)
- **Header Components:** header components are used for smaller vertical spaces. They are normalized to:
  - \(Z = [-0.2, 0.2]\)
  - \(Y = [0.0, 0.3]\)
  - \(X = [-1.0, 1.0]\)

A common approach to building door frames is to use a header component with various 15° arc sweeps to create the frame.

## Floors, Roofs, Stairs, and Doors

These modules hold reusable floor/roof fillers, circulation geometry, and door kits. Like partitions, they implement `LodScene` and are meant to be placed by building authors inside cell write bounds.

Floors and roofs are typically an **arc filler** plus a **struct filler**. Prefer rough stonework; wood appears occasionally (interior halfspaces, perch decking, door leaves).
