# RFC-183: Chico Vegetation

## Table of Contents

- [1: Summary](#1-summary)
- [2: Prior Art](#2-prior-art)
- [3: Design](#3-design)
  - [3.1: Stalk and Ball-stick Trees](#31-stalk-and-ball-stick-trees)
  - [3.2: L-system Trees](#32-l-system-trees)
  - [3.3: Ground Cover](#33-ground-cover)
  - [3.4: Cellular Groves](#34-cellular-groves)
  - [3.5: Cellular Forests](#35-cellular-forests)
  - [3.6: Elder Trees](#36-elder-trees)
- [4: Milestones](#4-milestones)

## 1: Summary

We propose the Chico vegetation system in response to [#61](https://github.com/ramate-io/maybraid/issues/61). Chico defines a layered, deterministic vegetation pipeline that evaluates from large forest cells down to individual plant constructions while keeping placement coherent with terrain, biome identity, and level-of-detail constraints.

The system is built around a few reusable pieces: ball-stick tree constructions for individual plants, world-space stick and leaf shaders for stable within-species variation, ground-cover primitives for low vegetation, cellular groves for local planting recipes, cellular forests for large-scale layering, and elder trees for rare urban landmarks. Forests select layerings with Hopscotch, layers select groves with Bucket Throw, and groves select individual variants with first-fit placement constraints.

The goal is not botanical simulation for its own sake. The goal is an authorable procedural vegetation system that can produce recognizable forest, scrub, grassland, orchard, jungle, and landmark-tree identities while remaining chunk-stable, scalable, and practical for runtime generation.

## 2: Prior Art

Industry vegetation systems tend to combine authored species assets with procedural distribution, terrain masks, and aggressive LOD. [SpeedTree](https://docs8.speedtree.com/modeler/doku.php?id=modeling_approach) is the most visible middleware example: it uses procedural generators for branches, fronds, leaves, and variation, while still giving artists direct control over the final plant models. [GPU Gems 3: Next-Generation SpeedTree Rendering](https://developer.nvidia.com/gpugems/GPUGems3/gpugems3_ch04.html) also illustrates the rendering side of this problem, especially leaf lighting, silhouettes, billboards, and distant tree representation.

Open-world games show the importance of procedural placement at terrain scale. Ubisoft's [Far Cry 5 procedural world generation](https://www.youtube.com/watch?v=JBp8zvLVsgg) pipeline used biome recipes and deterministic content generation to keep vegetation coherent as terrain changed during production. Ubisoft's [Ghost Recon Wildlands vegetation generation](https://80.lv/articles/vegetation-generation-in-ghost-recon-wildlands) similarly combined terrain materials, slope, density, spacing, exclusion masks, cascading tree-to-bush placement, and location-sensitive LOD to populate a very large world.

Academically, [*The Algorithmic Beauty of Plants*](http://www.springer.com/la/book/9780387946764) and related [Algorithmic Botany](http://www.algorithmicbotany.org/papers/) work establish L-systems as a foundation for procedural plant structure. [Realistic Modeling and Rendering of Plant Ecosystems](http://www.graphics.stanford.edu/papers/ecosys/) extends the problem to ecosystem-scale placement, combining terrain editing, procedural plant models, plant distribution, and geometric simplification for large natural scenes. Chico borrows the broad lesson from these systems, but favors spatially grounded, chunk-friendly constructions over a full L-system-first design.

## 3: Design

At runtime, Chico vegetation evaluates from coarse forest cells down to individual vegetation constructions:

```mermaid
graph TD
    Terrain["Terrain / world seed"]
    ForestGrid["Forest cell grid<br/>all cells active"]
    Hopscotch["Hopscotch selection<br/>choose forest layering"]
    ForestParams["Forest parameterization<br/>sample grove biases"]
    Layers["Forest layers<br/>ground cover, tufts, understory,<br/>lower canopy, upper canopy"]
    LayerThrow["Bucket Throw per layer<br/>choose grove or None"]
    GroveGrid["Grove cell grid<br/>all cells active"]
    GroveThrow["Bucket Throw / first-fit<br/>choose grove variant"]
    Placement["Placement constraints<br/>elevation, steepness, placement noise"]
    TreeCells["Individual vegetation cells<br/>trees, bushes, tufts, ground cover"]

    Terrain --> ForestGrid
    ForestGrid --> Hopscotch
    Hopscotch --> ForestParams
    ForestParams --> Layers
    Layers --> LayerThrow
    LayerThrow --> GroveGrid
    GroveGrid --> GroveThrow
    GroveThrow --> Placement
    Placement --> TreeCells
```

### 3.1: Stalk and Ball-stick Trees


Stalk and ball-stick trees form the core geometric system for Chico vegetation. The design refines the current system which uses [an ad hoc stalk with radial projection](https://github.com/ramate-io/maybraid/blob/cebdaf75f0ce2d837ddc818a9a2658abb3d738dd/procedures/vegetation/src/tree.rs#L171), a [`BallStick`](https://github.com/ramate-io/maybraid/blob/cebdaf75f0ce2d837ddc818a9a2658abb3d738dd/procedures/comproc/src/complex/chain/ball_stick/builder.rs) complex for the canopy, a [noisy cylinder](https://github.com/ramate-io/maybraid/blob/cebdaf75f0ce2d837ddc818a9a2658abb3d738dd/procedures/vegetation/src/tree/meshes/trunk/segment.rs) for trunk and branch segments, and a [planar canopy](https://github.com/ramate-io/maybraid/blob/9c38f45cfd697a392e6114bbc6e67b50005b7f65/procedures/vegetation/src/tree/meshes/canopy/ball.rs).

The goal is to formalize this into a composable, parameterized system:

* **Stalks** define primary vertical structure
* **Sticks** define trunk and branch segments
* **Balls and planes** define canopy and foliage
* **Ball-stick chains** unify tree construction
* **World-space stick and leaf shaders** provide stable color variation within a species

Tree types are then expressed as parameterized constructions over these primitives rather than bespoke implementations, with shader-side palettes providing individual variation without changing the underlying geometry.

---

Subsections:

- [3.1.1: Stick and Stalk Components](./03-01-stalk-and-ball-stick-trees/01-stick-and-stalk-components/README.md)
- [3.1.2: Ball Components](./03-01-stalk-and-ball-stick-trees/02-ball-components/README.md)
- [3.1.3: Ball-stick Anchors](./03-01-stalk-and-ball-stick-trees/03-ball-stick-anchors/README.md)
- [3.1.4: Ball-stick Chains](./03-01-stalk-and-ball-stick-trees/04-ball-stick-chains/README.md)
- [3.1.5: Ball Selection](./03-01-stalk-and-ball-stick-trees/05-ball-selection/README.md)
- [3.1.6: Well-known Component Constructions](./03-01-stalk-and-ball-stick-trees/06-well-known-component-constructions/README.md)
- [3.1.7: Well-known Tree Constructions](./03-01-stalk-and-ball-stick-trees/07-well-known-tree-constructions/README.md)
- [3.1.8: Tree LOD Tricks](./03-01-stalk-and-ball-stick-trees/08-tree-lod-tricks/README.md)
- [3.1.9: Stick Shading](./03-01-stalk-and-ball-stick-trees/09-stick-shading/README.md)
- [3.1.10: Leaf Shading](./03-01-stalk-and-ball-stick-trees/10-leaf-shading/README.md)

---

### 3.2: L-system Trees


L-systems are a well-established method for generating botanical structures and offer a natural way to express recursive growth, branching grammars, and species variation. They are a strong candidate for future expansion of the vegetation system.

Details:

- [3.2: L-system Trees](./03-02-l-system-trees/README.md)

---

### 3.3: Ground Cover


Ground cover is the lowest layer of vegetation detail: it fills terrain with grasses, moss, scrub, and low plant matter. It should stay **dense but inexpensive**, **spatially stable**, **driven by terrain** (elevation, slope, biome), and **composable** with higher-level vegetation. The design pairs **bump outs** (continuous SDF height variation, aligned with [RFC-170 bump outs](https://github.com/ramate-io/maybraid/tree/main/rfc/rfc-000-000-170-terrain-detail#34-bump-outs)) with **tufts** (sparse discrete clumps for breakup and silhouette).

Subsections:

- [3.3.1: Bump Outs](./03-03-ground-cover/01-bump-outs/README.md)
- [3.3.2: Tufts](./03-03-ground-cover/02-tufts/README.md)

---

### 3.4: Cellular Groves


Cellular Groves are the primary allocation unit for vegetation types. A grove defines a **locally coherent planting context**: it determines *what* can be planted, *how often*, and *under what constraints*. Groves unify a set of compatible vegetation types and expose parameter ranges that are instantiated by the parent [Forest](./03-05-cellular-forests/README.md#35-cellular-forests).

At a high level:

* **Parameterization** defines the statistical and environmental character of the grove
* **Selection and Placement** determines where and what is actually planted

---

Subsections:

- [3.4.1: Parameterization](./03-04-cellular-groves/01-parameterization/README.md)
- [3.4.2: Selection and Placement](./03-04-cellular-groves/02-selection-and-placement/README.md)
- [3.4.3: Well-known Ground Cover Groves](./03-04-cellular-groves/03-well-known-ground-cover-groves/README.md)
- [3.4.4: Well-known Tufts Groves](./03-04-cellular-groves/04-well-known-tufts-groves/README.md)
- [3.4.5: Well-known Understory Groves](./03-04-cellular-groves/05-well-known-understory-groves/README.md)
- [3.4.6: Well-known Lower Canopy Groves](./03-04-cellular-groves/06-well-known-lower-canopy-groves/README.md)
- [3.4.7: Well-known Upper Canopy Groves](./03-04-cellular-groves/07-well-known-upper-canopy-groves/README.md)
- [3.4.8: Grove LOD Tricks](./03-04-cellular-groves/08-grove-lod-tricks/README.md)

---

### 3.5: Cellular Forests


Cellular Forests are the top-level allocation system for Chico vegetation. They select coherent forest layerings over large forest cells, pass forest-scale parameter biases down to groves, and instantiate compatible grove layers inside each selected cell. A reasonable starting scale is `1600m x 1600m` forest cells with a(n) `8 x 8` grid of `200m x 200m` grove cells inside each forest cell.

Subsections:

- [3.5.1: Parameterization](./03-05-cellular-forests/01-parameterization/README.md)
- [3.5.2: Selection and Construction](./03-05-cellular-forests/02-selection/README.md)
- [3.5.3: Forest Layers](./03-05-cellular-forests/03-forest-layers/README.md)
- [3.5.4: Well-known Layerings](./03-05-cellular-forests/04-well-known-layerings/README.md)
- [3.5.5: Chico Vegetation](./03-05-cellular-forests/05-chico-vegetation/README.md)
- [3.5.6: Forest LOD Tricks](./03-05-cellular-forests/06-lod-tricks/README.md)

---

### 3.6: Elder Trees

Elder Trees are massive ball-stick tree constructions intended to pair tightly with urbanization. They use a separate allocation grid from forests and act as living landmarks for platforms, paths, shrines, homes, bridges, and other built features.

Details:

- [3.6: Elder Trees](./03-06-elder-trees/README.md)

## 4: Milestones

> [!NOTE]
> These milestones are implementation planning hooks, not dated commitments. Most map directly to Section 3 subsections; later milestones cover integration with [RFC-142: Gimme](../rfc-000-000-142-gimme/README.md), [RFC-127: Marazion Watersheds](../rfc-000-000-127-marazion-watersheds/README.md), and [RFC-154: Generalized LOD](../rfc-000-000-154-generalized-lod/README.md).

### 4.1: Ball-stick Primitive API

Implement the reusable tree-construction primitives from [3.1.1](./03-01-stalk-and-ball-stick-trees/01-stick-and-stalk-components/README.md) through [3.1.5](./03-01-stalk-and-ball-stick-trees/05-ball-selection/README.md): stalks, sticks, balls, planes, anchors, chains, and ball selection.

### 4.2: Proto Tree Constructions

Implement flexible `Proto*` tree construction types and constrain them into the well-known component and tree constructions in [3.1.6](./03-01-stalk-and-ball-stick-trees/06-well-known-component-constructions/README.md) and [3.1.7](./03-01-stalk-and-ball-stick-trees/07-well-known-tree-constructions/README.md).

### 4.3: Tree Shading Pipeline

Implement world-space [stick shading](./03-01-stalk-and-ball-stick-trees/09-stick-shading/README.md) and [leaf shading](./03-01-stalk-and-ball-stick-trees/10-leaf-shading/README.md), including species palettes, stable individual variation, flecks, season, longitude, and altitude terms.

### 4.4: Tree LOD Tricks

Implement the [tree LOD tricks](./03-01-stalk-and-ball-stick-trees/08-tree-lod-tricks/README.md): silhouette-preserving canopy and trunk simplification, branch dropout, rotation/skew variation, vertical gradients, and material simplification.

### 4.5: Ball-stick Playground

Build a developer playground for inspecting ball-stick chains, component mixes, tree variants, shader palettes, and LOD transitions in isolation before connecting them to terrain or grove allocation.

### 4.6: Ground Cover Primitives

Implement [ground cover](./03-03-ground-cover/README.md), including bump outs, tufts, flecking, grassy mounds, and low-cost terrain-hugging variation suitable for later grove and forest layering.

### 4.7: Grove Core Selection

Implement [cellular grove parameterization](./03-04-cellular-groves/01-parameterization/README.md) and [selection and placement](./03-04-cellular-groves/02-selection-and-placement/README.md): Bucket Throw, explicit `None` outcomes, position selection, per-variant constraints, and first-fit fallback.

### 4.8: Proto Grove Types

Build flexible `ProtoGrove` and `ProtoGroveCell` tooling that can express scale, density, offsets, placement constraints, palette mixes, and ordered bucket distributions before the full well-known grove library is locked down.

### 4.9: Well-known Grove Library

Implement the well-known grove families from [3.4.3](./03-04-cellular-groves/03-well-known-ground-cover-groves/README.md) through [3.4.7](./03-04-cellular-groves/07-well-known-upper-canopy-groves/README.md), starting with a thin subset and expanding toward the full ground-cover, tufts, understory, lower-canopy, and upper-canopy catalog.

### 4.10: Grove Playground

Build a developer playground for sampling one grove against a terrain model, inspecting bucket selection, `None` outcomes, first-fit fallback, placement constraints, palette mixes, and grove LOD behavior.

### 4.11: Grove LOD

Implement [grove LOD tricks](./03-04-cellular-groves/08-grove-lod-tricks/README.md), especially reduced instance counts with increased horizontal scale, so distant groves preserve footprint and treeline impression.

### 4.12: Forest Cell Core

Implement [cellular forest parameterization](./03-05-cellular-forests/01-parameterization/README.md) and [selection and construction](./03-05-cellular-forests/02-selection/README.md): forest cells, Hopscotch selection, forest-level grove biases, layer selection, grove grid construction, and deterministic salts.

### 4.13: Forest Layers and Layerings

Implement [forest layers](./03-05-cellular-forests/03-forest-layers/README.md) and the initial [well-known layerings](./03-05-cellular-forests/04-well-known-layerings/README.md), including `None` semantics per layer and ground-cover flip/flop behavior.

### 4.14: Chico Hopscotch Graph

Implement the [Chico Vegetation](./03-05-cellular-forests/05-chico-vegetation/README.md) Hopscotch graph over well-known layerings, with node weights, adjacency weights, loop-backs, and deterministic traversal.

### 4.15: Forest LOD

Implement [forest LOD tricks](./03-05-cellular-forests/06-lod-tricks/README.md): selective layer dropout, canopy preservation, ground-cover impressions, per-forest dropout policy, and integration with generalized LOD requirements.

### 4.16: Forest Cell Playground

Build a developer playground for one forest cell: visualize selected layering, per-layer grove choices, grove grids, forest bias values, `None` outcomes, and LOD dropout masks.

### 4.17: Gimme-backed Vegetation Queries

Integrate vegetation generation with [Gimme](../rfc-000-000-142-gimme/README.md) by querying terrain and generated vegetation through AaBb regions on the spatial index. Start with terrain-height and steepness queries; later add typed materialized views for vegetation, water, urban features, and blockers.

### 4.18: Surface Water Avoidance

Integrate with [Marazion Watersheds](../rfc-000-000-127-marazion-watersheds/README.md) by rejecting or adjusting candidate tree and grove placements whose AaBb intersects a common `SurfaceWater` type in the spatial index. This may land after the first terrain-only grove implementation.

### 4.19: Vegetation Loading via Generalized LOD

Implement a vegetation `ChunkTracker` for [Generalized LOD](../rfc-000-000-154-generalized-lod/README.md). Visible chunks should request or materialize vegetation through Gimme; hidden and removed chunks should preserve deterministic regeneration and avoid double-spawn or orphan entity behavior.

### 4.20: Elder Trees

Implement [Elder Trees](./03-06-elder-trees/README.md) on a separate allocation grid from cellular forests, using ball-stick constructions, ordinary tree/grove LOD techniques, and explicit urban attachment affordances.

### 4.21: Tree Playground

Build a developer playground focused on complete well-known tree constructions: tree recipes, scale envelopes, palettes, fruiting bodies, flecks, and per-tree LOD swaps.

### 4.22: Vegetation Playground

Build an end-to-end developer playground that runs terrain queries, forest-cell selection, grove selection, individual placement, shading, and LOD in one inspectable scene.

### 4.23: End-to-End Gimme Draft Generation

Run vegetation generation through Gimme draft-style regional writes, validating deterministic IDs, region materialization, persistence boundaries, and repeatable get-or-generate behavior across chunk loads.

### 4.24: Cross-system Validation

Validate vegetation against terrain, water, urbanization, and elder-tree allocation: no surface-water tree placements where disabled, no major forest/elder conflicts, stable chunk boundaries, and readable biome identity under low LOD.

