---
name: Hydro composition modulation
overview: Authored plans emit hydro nodes into WatershedDepressionComplex, which modulates Terrain and blends at sample time (spatial broadphase only). No Jersey grade reuse; no precomputed footprint-intersection bake.
todos:
  - id: hydro-types
    content: Add HydroPrimitive / HydroElevation / FootprintIndex; WatershedDepressionComplex holds nodes and blends at sample time
    status: completed
  - id: union-policy
    content: Sample-time min bed + smoothmin W + rim/apron from phi_union with one ComplexApronParams
    status: completed
  - id: compile-wire
    content: Prepare complex (build broadphase index) for Durham ComposedElevationOp; shared fill eval
    status: completed
  - id: migrate-streams
    content: Decompose StreamsGraph/solo Stream polylines into segment HydroPrimitives; retire StreamBandComposer path
    status: completed
  - id: contract-tests
    content: Add union-bed, no-internal-rim, index==bruteforce, reach XZ profile; keep StreamsGraph freeboard/pillar contracts
    status: completed
isProject: false
---

# Hydro composition modulation (union-first complex)

## Pipeline

```text
Authored plan (Stream / StreamsGraph / …)
    --emits nodes-->  WatershedDepressionComplex  (builder / holder)
    --modulates-->    Terrain
    --sampled by-->   mesh-building backend
```

The complex **is** the terrain modulation (marazion-owned). It is not flattened into a list of Jersey ops before sampling.

**Do not start from Jersey graded polylines.** Authoring may still produce polylines (hysteresis); the complex stores **decomposed segment nodes** with marazion-owned footprints and local elevation fields.

Default scope: substrate + **StreamsGraph** (+ solo Stream on the same path). Lake/Bog stay on legacy emit until a radial-bowl follow-up.

## Sample-time blend vs pre-sample intersection bake

Two conceivable versions:

1. **Chosen (v1):** Nodes live in the complex behind a spatial index. Each terrain/fill sample through the complex does continuous blending (`min` bed, `smoothmin` \(W\), `min` \(\phi\)) over the small candidate set from the broadphase. Compact support already makes faraway nodes arithmetic no-ops.
2. **Rejected:** At a pre-sample step, compute exact footprint-intersection sets and store them so sample time skips “who overlaps?”. More machinery, little win. **Not what we build.**

`HydroSample` was a confused stand-in for (2) or for a sample-time DTO. **Neither exists** — no precomputed intersection graph, no intermediate sample struct.

```mermaid
flowchart LR
  Authored[Authored plan] --> Nodes[Emit HydroPrimitives]
  Nodes --> Complex[WatershedDepressionComplex]
  Complex --> Terrain[Terrain modulation list]
  Terrain --> Mesh[Mesh backend samples height/fill]
  Mesh -->|"each sample"| Blend["Broadphase then continuous min/smoothmin"]
```

## Core types (marazion)

New module [`maybraid/durham/marazion/src/hydro.rs`](maybraid/durham/marazion/src/hydro.rs).

```rust
struct HydroPrimitive {
    footprint: HydroFootprint,   // phi <= 0 interior
    elevation: HydroElevation, // bed/surface over footprint
    influence_pad: f32,
}

enum HydroFootprint {
    ReachSegment { a: Vec2, b: Vec2, half_width: f32 },
    Ellipse { center: Vec2, radii: Vec2, rotation: f32 },
}

enum HydroElevation {
    /// Local Z along travel, local X across channel.
    ReachProfile {
        surface_a: f32,
        surface_b: f32,
        center_depth: f32, // depth(X) = center_depth * P(|X|/half_width)
    },
    /// Lake: bowl in ellipse-normalized u.
    RadialBowl {
        surface: f32,
        center_depth: f32,
    },
}
```

`WatershedDepressionComplex` holds primitives + one `ComplexApronParams`, builds a **broadphase** `FootprintIndex` when prepared for terrain, and evaluates height/fill by blending at sample time.

### Local frames

**Reach:** \(Z \in [0,1]\) along segment; \(X\) = signed perpendicular / `half_width`.

\[
W(Z) = (1-Z)\,W_a + Z\,W_b
\qquad
z_{\mathrm{bed}} = W(Z) - D_0\,P(|X|)
\]

**Lake:** flat \(W\); \(z_{\mathrm{bed}} = W - D_0\,P(u)\) on the ellipse. No inset API in v1.

### Polyline decomposition

```text
path p0..pn  ->  one HydroPrimitive per segment (p_i, p_{i+1})
```

### Each terrain/fill sample

1. Broadphase bucket → candidate node IDs.
2. Continuous fold: \(\phi=\min\phi_i\), \(z_{\mathrm{bed}}=\min z_{\mathrm{bed},i}\), \(W=\mathrm{smoothmin}\,W_i\).
3. Height: depress to bed inside; one complex rim/apron from \(\phi\).

Rim/apron bands use shared `ComplexApronParams` only — internal banks vanish where \(\phi_{\mathrm{union}}<0\).

## Spatial index (broadphase only)

```rust
struct FootprintIndex {
    origin: Vec2,
    cell: f32,
    buckets: HashMap<(i32, i32), SmallVec<[u16; 8]>>,
}
```

Built when the complex is prepared for terrain (today’s `compile()` lifecycle). Limits candidates per sample; does **not** bake exact intersections.

## Durham wiring

```rust
enum ComposedElevationOp {
    Jersey(JerseyModulation),
    Watershed(WatershedDepressionComplex), // or a prepared view thereof
}
```

Cell-domain identity baked into the complex (leaf AABB).

## Authoring migration

| Plan | Change |
|------|--------|
| [`streams_graph.rs`](maybraid/durham/marazion/src/streams_graph.rs) | Emit segment `HydroPrimitive`s + one apron param set into the complex. |
| Solo [`stream`](maybraid/durham/marazion/src/stream.rs) | Same segment decomposition. |
| [`complex.rs`](maybraid/durham/marazion/src/complex.rs) | Complex is the modulation: primitives + broadphase + sample-time blend. |
| [`compose.rs`](maybraid/durham/marazion/src/compose.rs) | Retire for StreamsGraph. |

## Tests

1. Reach profile: bed vs \(|X|\), pitch vs \(Z\); \(W\) independent of \(X\).
2. Union bed: overlapping segments → min bed.
3. No internal rim at confluence interior.
4. Index candidates ⊇ brute-force contributors (correctness under broadphase).
5. StreamsGraph freeboard / rim-cap contracts.

## Explicit non-goals

- Pre-sample exact footprint-intersection baking (version 2)
- Jersey `node_blend` / polyline grading for hydro
- Lake/Bog wiring this slice (`RadialBowl` type only)
- Junction morph objects, routing pathfinding, shoreline inset API

## Key files

- New: [`hydro.rs`](maybraid/durham/marazion/src/hydro.rs)
- Edit: [`complex.rs`](maybraid/durham/marazion/src/complex.rs), [`streams_graph.rs`](maybraid/durham/marazion/src/streams_graph.rs), [`stream.rs`](maybraid/durham/marazion/src/stream.rs), [`fill.rs`](maybraid/durham/marazion/src/fill.rs), [`terrain.rs`](maybraid/durham/models/src/terrain.rs)
- Spec: [`NODE_BLEND.md`](maybraid/durham/marazion/src/NODE_BLEND.md) §§1–3, 5 (broadphase-only index reading)
