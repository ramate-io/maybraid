# RFC-N: Procedural Terrain Generation

## 1: Motivation

This RFC specifies how we think about **procedural terrain** for Maybraid: stamps, noise, semantics, and chaining, in a way that scales to demos and downstream systems (watershed, vegetation, LOD). It tracks the open proposal [maybraid#55 — Procedural terrain generation proposal](https://github.com/ramate-io/maybraid/issues/55).

## 2: Prior Art

### 2.1: Theory

*(To be expanded: heightfields, erosion models, hydrology graphs, procedural texturing.)*

### 2.2: Practice

*(To be expanded: game-engine terrain stacks, clipmaps, virtual texturing.)*

### 2.3: Already in Maybraid

The codebase already implements a pipeline in the [`terrain`](../../procedures/terrain/) crate (`procedures/terrain`) where **authoring** is **2.5D**: elevation comes from a height function `h(x, z)` plus stacked modulations. That height is then embedded in a **true 3D signed distance field** ([`TerrainSdf`](../../procedures/terrain/src/lib.rs) implements the [`Sdf`](../../util/sdf/src/lib.rs) trait from the `sdf` crate), so the surface is not locked to a single-valued heightmap mesh forever—you can treat terrain as **volumetric** and combine it with other SDFs (tunnels, caves, overhangs) using the usual CSG / min–max composition rules. The demo wires this through Bevy via [`TerrainPlugin`](../../procedures/terrain/src/plugin.rs) and [`demos/naturescapes/src/terrain.rs`](../../demos/naturescapes/src/terrain.rs); caves are a **capability of the representation**, not yet a shipped stamp set.

**Crate layout and dependencies** ([`procedures/terrain/Cargo.toml`](../../procedures/terrain/Cargo.toml)):

- **Bevy** for ECS and rendering integration.
- **`noise`** (Perlin) for base height and optional boundary perturbation.
- **`sdf`** — terrain is exposed as [`TerrainSdf`](../../procedures/terrain/src/lib.rs): a **signed distance field** for `y = height(x,z)` with a bedrock slab, implementing `Sdf` and mesh identity for the render pipeline.
- **`chunk`** + **`render-item`** — **LOD cascades** (`Cascade`, resolution maps) and `TerrainRenderItem` so terrain meshes are built and cached per chunk.
- **`comproc`** — shared procedural utilities.

**Base height (maps to [Section 3.2](#32-noise-base)):** [`TerrainSdf::height_at`](../../procedures/terrain/src/lib.rs) builds elevation from **four octaves** of Perlin (amplitude halving, frequency doubling), then applies a **contrast exponent** on the summed noise and scales by `height_scale`. This is the global **noise base** before any stamps.

**Stamps as elevation modulations (maps to Sections [3.5](#35-stamp-generation)–[3.6](#36-stamp-semantics)):** The stack [`ElevationModulation`](../../procedures/terrain/src/lib.rs) is applied in order via `add_elevation_modulation` / `height_at_with_all_modulations`. Each modulation sees the current height and returns a new height at `(x,z)`. Today this is primarily **geometric + noise-driven shaping**, not a separate semantic bitmask layer—but the hook is the right place to attach **tags** (e.g. “riverbed”) when we need spawning rules.

**Regions (stamp footprints):** [`region`](../../procedures/terrain/src/region.rs) defines **2D signed-distance regions** (`Region2D`: rounded rect, circle, convex polygon) with optional [`RegionNoise`](../../procedures/terrain/src/region.rs) to **perturb boundaries** using the same FBM-style sampling as the core terrain noise. Implementations include:

- [`RegionAffineModulation`](../../procedures/terrain/src/region/affine.rs) — affine tilt / lift inside a region (used for broad valleys).
- [`RegionRoundingModulation`](../../procedures/terrain/src/region/rounding.rs) — quantize height steps inside a region (e.g. road-like terraces).
- [`RegionGradingModulation`](../../procedures/terrain/src/region/grading.rs) — **blend toward a target grade** between two endpoints and elevations (see [Section 3.7.3](#373-fractal-paths)).

**Fractal / multiscale expansion (partial map to [Section 3.4](#34-fractal-stamping) and [3.7.1](#371-common-noise-chains)):** [`BranchingPlan`](../../procedures/terrain/src/region/branching.rs) takes a seed `RegionAffineModulation` and expands it to many child regions using **Perlin-seeded** `branch_region` calls over **depth × breadth**, producing a forest of similar stamps—useful for valley fingers and other repeated large-scale features without hand-placing each instance.

**Demo wiring:** [`TerrainPlaygroundPlugin::setup_terrain`](../../demos/naturescapes/src/terrain.rs) constructs a `TerrainSdf`, stacks overlapping valley modulations, runs `BranchingPlan`, adds rounding and grading for a road, then spawns **`Terrain` + `Lod` + `Cascade` + `DispatchRenderItem`**. **Detail passes** (`TerrainDetail` with `RockSpheroid`, `GrassTuft`) sample the same `TerrainSdf` for placement, showing how **macro terrain** and **surface detail** share one height oracle.

**Gaps vs this RFC:** No first-class **cellular PRNG grid** ([Section 3.3](#33-cellular-stamping)), no **FNS** ([Section 3.7.2](#372-fractal-neighborhood-stamps-fns)), and no **semantic layers** exported from stamps ([Section 3.6](#36-stamp-semantics)) yet—the design below is partly aspirational and partly a formalization of what we already ship.

## 3: Design

### 3.1: Core Concepts

- **Height oracle:** A callable `h(x, z)` (and derived slope / Laplacian if needed) that is the **single source of truth** for elevation queries. In code today this is [`TerrainSdf`](../../procedures/terrain/src/lib.rs); later it may be a trait object or graph of passes.
- **Stamp:** A **localized operator** on the height oracle: it has a **footprint** (inside/outside test or soft falloff in the plane) and **parameters** (strength, grade endpoints, noise, and so on). Multiple stamps **compose** in a defined order (current code: ordered `ElevationModulation` list).
- **Fractal stamping:** Using **continuous noise** (single or multi-octave) to decide **where** a stamp applies or **how** strong it is, so features naturally span many cells without a rigid grid.
- **Cellular stamping:** Using a **discrete cell key** and **PRNG** to choose stamp presence or type; ideal when **cross-cell correlation is unnecessary**.
- **Semantics:** Data carried alongside deformation (biome tags, hydrology masks, spawn weights). Not yet first-class in `terrain`; [Section 3.6](#36-stamp-semantics) describes the target.

### 3.2: Noise Base

We start from a simple Perlin noise base and make several improvements.

**In Maybraid today:** the base is **multi-octave Perlin** (four octaves, standard persistence) plus a **global exponent** on the summed sample to exaggerate or flatten relief, then **`height_scale`**. That matches the “simple Perlin + improvements” story; future work can swap in **ridged fractal**, **domain warp**, or **erosion-like** post-filters without changing the stamp API if they remain functions of `(x, z)` into base height.

### 3.3: Cellular Stamping

For a fixed cell size, we determine whether the cell has a certain kind of stamp applied with a PRNG. This is good for stamps that don't need to preserve multi-cell structure as the PRNG can be basic. For features that should span multiple cells—for example, rivers, ridges, and valley chains—we can use fractal (noise) approaches as described in [Section 3.4](#34-fractal-stamping).

### 3.4: Fractal Stamping

We recommend the following noise functions by stamp type (initial guidance; tune per art direction):

| Stamp family | Suggested noise role | Notes |
|--------------|----------------------|--------|
| Large landform (basin, plateau) | Low-frequency FBM / Perlin octaves | Correlates across many cells; seed from world coordinates. |
| Ridge / cliff line | Ridged noise or abs(Perlin) variants | Sharp crests; may need directional domain warp. |
| Valley / channel | Anisotropic or curve-guided noise + FBM detail | Combine with path-based grade ([Section 3.7.3](#373-fractal-paths)) for readable rivers. |
| Scatter (boulders, small dips) | Mid-frequency FBM + threshold | Can mix with cellular placement for variety. |
| Boundary wobble | FBM on **2D region SDF** | Matches current [`RegionNoise`](../../procedures/terrain/src/region.rs) usage. |

**Branching expansions** (e.g. [`BranchingPlan`](../../procedures/terrain/src/region/branching.rs)) are one way to get **self-similar** stamp layouts driven by the same noise family.

### 3.5: Stamp Generation

Stamps themselves can be subject to noisy variation: **parameter jitter** (width, depth, rotation), **footprint noise** (wiggly edges via `RegionNoise`), and **internal breakdown** (child stamps from branching). The implementation should keep **determinism** (seed from chunk or world cell) so LOD and streaming agree.

### 3.6: Stamp Semantics

Some stamps can carry semantic meaning that is reused in later generation layers. For example, a riverbed stamp can both lower the elevation to match a consistent grade and mark its extents as a region in which fish can spawn.

**Design target:** separate **geometry** (height delta) from **labels** (tags, masks, optional graph edges) emitted in the same evaluation pass, so gameplay and ecology systems query semantics without re-deriving them from height alone. The current `ElevationModulation` trait only adjusts height; extending it (or parallel components) is the natural integration point.

### 3.7: Stamp Chains

One common requirement will be to chain related stamps—for example, a riverbed, then a waterfall, then a riverbed again.

#### 3.7.1: Common Noise Chains

A simple approach to achieve global agreement on these chains is to generate stamps we intend to chain from the **same noise function** (or same low-dimensional field). We can do this **discretely** (map noise value bands to stamp types) or **continuously** (interpolate stamp parameters along the scalar field).

#### 3.7.2: Fractal Neighborhood Stamps (FNS)

One particularly useful chaining approach is what we call **Fractal Neighborhood Stamps (FNS)**. Instead of taking one seed value, FNS consults **neighboring samples** (grid or graph neighbors) of the same underlying field so local connectivity (tunnels, notches, aligned ridges) emerges without a global solver. FNS does **not** by itself enforce **large-scale constraints** such as monotonic grade along a kilometer of river; pair it with path or optimization passes when needed.

#### 3.7.3: Fractal Paths

Sometimes we need a chain of stamps that adapts terrain **consistently along a path**. To achieve this we use **path-parameterized modulation**: define a curve or polyline in the plane, sample **arc length** `s` (or projection `t`), and drive height or stamp blend weights with **1D noise / splines / analytic grade** along `s`. The existing [`RegionGradingModulation`](../../procedures/terrain/src/region/grading.rs) is a concrete example: it fixes **start/end elevations** sampled from the current oracle, then grades within a **region footprint**, so the path respects global height at its endpoints.

#### 3.7.4: Higher-order Patterns and the Power of Large Extents

At times the patterns above can feel too **local**: a consistently graded river is hard if you only see **one cell** at a time—**even harder** if the water must **meet** elevations imposed by distant terrain.

So we still want **higher-level regimes** that **plan** whole features (drainage basins, mountain ranges) and then **refine** with smaller stamps. At the same time, we cannot expect every global constraint to be solved by a single pass of local rules; we accept **multi-resolution** planning (coarse graph or field) plus **fine stamps** for detail.

It is often helpful to treat **large chained features as stamps themselves**: a single stamp spans many **streamed cells**; when a cell is loaded, it runs **bespoke routines** scoped to that stamp’s parameterization (internal cellular grids, nested noise, and so on).

**Composing** several such **macro-stamp** layers matches how we already **stack** `ElevationModulation` on `TerrainSdf`, and aligns with **BVH / spatial LOD**: macro stamps occupy coarse nodes; children refine. Generation and **culling** should eventually share the same hierarchical structure where possible.

### 3.8: Jersey Stamps

We propose the following stamps be released under the Jersey edition of Maybraid terrain.

## 4: Milestones

