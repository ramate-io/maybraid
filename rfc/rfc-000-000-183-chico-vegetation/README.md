# RFC-183: Chico Vegetation

## Table of Contents

## 1: Summary

We propose the Chico vegetation system in response to [#61](https://github.com/ramate-io/maybraid/issues/61).

## 2: Prior Art

## 3: Design

### 3.1: Stalk and Ball-stick Trees


Stalk and ball-stick trees form the core geometric system for Chico vegetation. The design refines the current system which uses [an ad hoc stalk with radial projection](https://github.com/ramate-io/maybraid/blob/cebdaf75f0ce2d837ddc818a9a2658abb3d738dd/procedures/vegetation/src/tree.rs#L171), a [`BallStick`](https://github.com/ramate-io/maybraid/blob/cebdaf75f0ce2d837ddc818a9a2658abb3d738dd/procedures/comproc/src/complex/chain/ball_stick/builder.rs) complex for the canopy, a [noisy cylinder](https://github.com/ramate-io/maybraid/blob/cebdaf75f0ce2d837ddc818a9a2658abb3d738dd/procedures/vegetation/src/tree/meshes/trunk/segment.rs) for trunk and branch segments, and a [planar canopy](https://github.com/ramate-io/maybraid/blob/9c38f45cfd697a392e6114bbc6e67b50005b7f65/procedures/vegetation/src/tree/meshes/canopy/ball.rs).

The goal is to formalize this into a composable, parameterized system:

* **Stalks** define primary vertical structure
* **Sticks** define trunk and branch segments
* **Balls and planes** define canopy and foliage
* **Ball-stick chains** unify tree construction

Tree types are then expressed as parameterized constructions over these primitives rather than bespoke implementations.

---

Subsections:

- [3.1.1: Stick and Stalk Components](./03-01-stalk-and-ball-stick-trees/03-01-01-stick-and-stalk-components/README.md)
- [3.1.2: Ball Components](./03-01-stalk-and-ball-stick-trees/03-01-02-ball-components/README.md)
- [3.1.3: Ball-stick Anchors](./03-01-stalk-and-ball-stick-trees/03-01-03-ball-stick-anchors/README.md)
- [3.1.4: Ball-stick Chains](./03-01-stalk-and-ball-stick-trees/03-01-04-ball-stick-chains/README.md)
- [3.1.5: Ball Selection](./03-01-stalk-and-ball-stick-trees/03-01-05-ball-selection/README.md)
- [3.1.6: Well-known Component Constructions](./03-01-stalk-and-ball-stick-trees/03-01-06-well-known-component-constructions/README.md)
- [3.1.7: Well-known Tree Constructions](./03-01-stalk-and-ball-stick-trees/03-01-07-well-known-tree-constructions/README.md)
- [3.1.8: Tree LOD Tricks](./03-01-stalk-and-ball-stick-trees/03-01-08-tree-lod-tricks/README.md)
- [3.1.9: Stick Shading](./03-01-stalk-and-ball-stick-trees/03-01-09-stick-shading/README.md)
- [3.1.10: Leaf Shading](./03-01-stalk-and-ball-stick-trees/03-01-10-leaf-shading/README.md)

---

### 3.2: L-system Trees


L-systems are a well-established method for generating botanical structures and offer a natural way to express recursive growth, branching grammars, and species variation. They are a strong candidate for future expansion of the vegetation system.

Subsections:

- [3.2: L-system Trees](./03-02-l-system-trees/README.md)

---

### 3.3: Ground Cover


Ground cover is the lowest layer of vegetation detail: it fills terrain with grasses, moss, scrub, and low plant matter. It should stay **dense but inexpensive**, **spatially stable**, **driven by terrain** (elevation, slope, biome), and **composable** with higher-level vegetation. The design pairs **bump outs** (continuous SDF height variation, aligned with [RFC-170 bump outs](https://github.com/ramate-io/maybraid/tree/main/rfc/rfc-000-000-170-terrain-detail#34-bump-outs)) with **tufts** (sparse discrete clumps for breakup and silhouette).

Subsections:

- [3.3.1: Bump Outs](./03-03-ground-cover/03-03-01-bump-outs/README.md)
- [3.3.2: Tufts](./03-03-ground-cover/03-03-02-tufts/README.md)

---

### 3.4: Cellular Groves


Cellular Groves are the primary allocation unit for vegetation types. A grove defines a **locally coherent planting context**: it determines *what* can be planted, *how often*, and *under what constraints*. Groves unify a set of compatible vegetation types and expose parameter ranges that are instantiated by the parent [Forest](./03-05-cellular-forests/README.md#35-cellular-forests).

At a high level:

* **Parameterization** defines the statistical and environmental character of the grove
* **Selection and Placement** determines where and what is actually planted

---

Subsections:

- [3.4.1: Parameterization](./03-04-cellular-groves/03-04-01-parameterization/README.md)
- [3.4.2: Selection and Placement](./03-04-cellular-groves/03-04-02-selection-and-placement/README.md)
- [3.4.3: Well-known Ground Cover Groves](./03-04-cellular-groves/03-04-03-well-known-ground-cover-groves/README.md)
- [3.4.4: Well-known Tufts Groves](./03-04-cellular-groves/03-04-04-well-known-tufts-groves/README.md)
- [3.4.5: Well-known Understory Groves](./03-04-cellular-groves/03-04-05-well-known-understory-groves/README.md)
- [3.4.6: Well-known Lower Canopy Groves](./03-04-cellular-groves/03-04-06-well-known-lower-canopy-groves/README.md)
- [3.4.7: Well-known Upper Canopy Groves](./03-04-cellular-groves/03-04-07-well-known-upper-canopy-groves/README.md)

---

### 3.5: Cellular Forests


General name for top-level grove allocation system. Split into several layers of groves. 

Subsections:


---

### 3.6: Elder Trees


Subsections:

- [3.6: Elder Trees](./03-06-elder-trees/README.md)

## 4: Milestone

