# RFC-N: Procedural Terrain Generation

## 1: Motivation

Maybraid relies heavily on procedural generation, including for terrain. Because its world is arbitrarily large it also requires globally consistent terrain. We propose strategies focused on use of fractal noise, stamps, semantics, and chaining, in a way that scales to demos and downstream systems (watershed, vegetation, LOD). It tracks the open proposal [maybraid#55 — Procedural terrain generation proposal](https://github.com/ramate-io/maybraid/issues/55).

## 2: Prior Art

### 2.1: Theory

#### 2.1.1: Gradient noise (Perlin and relatives)

[Perlin noise](https://en.wikipedia.org/wiki/Perlin_noise) and successor **gradient-domain** functions (see Ken Perlin’s [noise reference page](https://cs.nyu.edu/~perlin/noise/); **Improving Noise**, ACM TOG 2002, is not linked here—common PDF mirrors and DOI checks often time out or return errors in automated link validation) are the default **continuous random field** for height: cheap to sample, smooth between neighbors, and easy to tile into **infinite worlds** (Minecraft-style overlays, space games, survival sandboxes).

**In procedurally generated games**, gradient noise typically drives **base elevation**, **wind patterns**, or **blend weights** between materials; designers stack a few octaves and tune frequency, so play spaces read as hills and flats rather than white noise.

**Maybraid:** Strong fit for the **noise base** and for **stamp parameters** (Section 3): we want **deterministic**, **streaming-safe** height without authoring every hill. Low-poly art direction does not reduce the need for coherent macro shape; it reduces reliance on **high-frequency** detail in the base field.

#### 2.1.2: Fractal sums and spectral control (fBm-style terrain)

Summing noise at multiple scales produces **fractional Brownian motion (fBm)**-like relief; see [fractional Brownian motion](https://en.wikipedia.org/wiki/Fractional_Brownian_motion) and the intuition of **fractal terrain** in Mandelbrot’s [The Fractal Geometry of Nature](https://en.wikipedia.org/wiki/The_Fractal_Geometry_of_Nature).

**In games**, multi-octave sums are the standard way to get **foothills → ridges → micro-roughness** from a single seed; many titles add **ridging**, **warp**, or **curve remapping** on top of the same stack.

**Maybraid:** Aligns directly with a **stampable** pipeline: the global base can stay fBm-like while **stamps** inject valleys, roads, and hero features. We should keep **spectral** choices (how many octaves, how much high frequency) compatible with **chunk LOD**, so distant terrain does not shimmer or diverge from near terrain.

#### 2.1.3: Implicit surfaces, marching cubes, and CSG

[Metaballs](https://en.wikipedia.org/wiki/Metaballs), [marching cubes](https://en.wikipedia.org/wiki/Marching_cubes), and [constructive solid geometry](https://en.wikipedia.org/wiki/Constructive_solid_geometry) treat terrain as **volumes** defined by scalar fields—enabling **caves**, **overhangs**, and **tunnels** that a strict **single-valued heightmap** cannot represent.

**In games**, voxels (e.g. Minecraft-like) and some **SDF** pipelines use this for **diggable** worlds or cinematic rocks; full **isosurface** extraction every frame is often reserved for mods, tools, or hybrid meshes (surface + volume holes).

**Maybraid:** **Conceptual fit** is good: the motivation mentions **subterranean** exploration and downstream systems; a **3D SDF or hybrid** representation (Section 2.3 exploratory direction) keeps **caves** in scope without committing every ship target to marching cubes. **Cost and tooling** are the tension: low-poly + Bevy favors **simple meshes**; volumetric composition may stay **optional** or **localized** (stamps that carve holes) rather than global voxel worlds.

#### 2.1.4: Hydraulic and thermal erosion

Iterated **sediment transport** and **slope-collapse** models reshape noise into **drainage-like** networks; classic references appear in Musgrave’s line of work and in [*Texturing and Modeling*](https://www.sciencedirect.com/book/9781558608481/texturing-and-modeling) (Ebert et al.); see also surveys of **interactive terrain erosion** (e.g. [terrain deformation / erosion](https://en.wikipedia.org/wiki/Terrain_deformation)).

**In games**, erosion appears both as **offline** bake (acceptable load times) and as **GPU** or **chunk** passes for **live** worlds; it sells **valleys, talus, and river attachment** better than raw noise alone.

**Maybraid:** **Partial fit.** Hydrology-flavored **look** supports discovery and **watershed**-adjacent gameplay, but **heavy** iterative simulation on every chunk may fight **demo deadlines** and **deterministic streaming**. Likely pattern: **lightweight** erosion-like **stamps** or **few** global passes, plus **semantic** water networks where we need correctness more than full physics.

#### 2.1.5: River networks and stream order

[Stream order](https://en.wikipedia.org/wiki/Stream_order) (Strahler, Horton) formalizes **tree-shaped drainage**: tributaries merge into larger channels. Procedural tools often build a **graph or tree on the plane**, then **carve** or **stamp** terrain to match.

**In games**, explicit river graphs power **navigable water**, **quests along rivers**, and **consistent flow direction**; without a graph, rivers from noise alone often **fail to close** or **flow uphill** locally.

**Maybraid:** Strong fit for **macro-stamps** and **stamp chains** (Section 3.7): we want **large features** that stay coherent across **streamed** cells. The open question is **how much** graph solving we do up front vs. **lazy** refinement when chunks load—both are compatible with the RFC if semantics and height stay coupled.

#### 2.1.6: Scatter placement (Poisson disk, Worley / cellular noise)

[Poisson disk sampling](https://en.wikipedia.org/wiki/Supersampling#Poisson_disc) yields **even, non-grid** spacing for props; [Worley / cellular noise](https://en.wikipedia.org/wiki/Worley_noise) builds **Voronoi-like** regions useful for **biome patches**, **stone fields**, or **cellular** variation that still looks organic.

**In games**, these drive **trees, rocks, grass clumps**, and **loot** placement on top of terrain; they are orthogonal to **how** height is formed but essential for **readable** open worlds.

**Maybraid:** Strong fit: vegetation and **detail** layers already sit beside macro terrain in our demos. Worley-style fields can also back **cellular stamping** (Section 3.3) when we want **patchy** stamp types without a rigid grid. The main constraint is the same as everywhere: **reproducible** seeds per chunk and agreement with **LOD** (don’t spawn detail only at one LOD).

### 2.2: Practice

#### 2.2.1: Large-scale terrain LOD (geometry clipmaps)

[Geometry clipmaps](https://en.wikipedia.org/wiki/Clipmap) (Losasso & Hoppe, [author project page](https://hhoppe.com/proj/geomclipmap/) and [paper PDF](https://hhoppe.com/geomclipmap.pdf)) use **nested regular grids** centered on the camera, so **high resolution** sits near the player and **coarser** rings fill the horizon, with **streaming** and **stitching** rules that avoid cracks.

**In procedurally generated games**, clipmaps and close cousins (**chunked heightfields**, **CDLOD**, planet **quadtrees**) are how open-world and flight titles keep **frame cost bounded** while the world is **infinite or huge**. Generation runs **per chunk or per ring**, keyed by world coordinates and LOD level.

**Maybraid:** Strong fit for **discovery-scale** outdoor spaces: we need **deterministic** height per chunk and stable **transitions** between LODs, so procedural stamps do not **pop** or **shift** when rings update. Exact choice (clipmap vs irregular chunk cascade) is an **engine** decision; the RFC assumes **some** hierarchical LOD story shared with **culling** (Section 3.7.4).

#### 2.2.2: Texture resolution at scale (virtual texturing)

[Virtual texturing](https://en.wikipedia.org/wiki/MegaTexture#Virtual_texturing) (see also **MegaTexture** / clipmap-style streaming on the same page) and **sparse** / **paged** albedo-normal workflows (often discussed under **virtual texture** or **megatexture**-class rendering) break the terrain **material** into **tiles** loaded by **visibility** and **mip** need, instead of one giant bitmap.

**In games**, this matters when **ground detail** must hold up at **player scale** across **kilometers**—common in AAA open worlds and some stylized titles with rich surface breakup.

**Maybraid:** **Partial fit.** Low-poly shading reduces pressure for **8K-class** splats, but **biomes**, **paths**, and **stamp semantics** may still want **sparse** overlays (decals, splat IDs, or VT) as worlds grow. Cost is **pipeline complexity** (streaming, compression, tooling); we may stay **lightweight** (few layers, baked low-res) until a demo explicitly needs **near-field** texture density.

#### 2.2.3: Engine terrain stacks (Unreal, Unity, and analogs)

[Unreal Landscape](https://dev.epicgames.com/documentation/en-us/unreal-engine/landscape-technical-guide) and [Unity Terrain](https://docs.unity3d.com/Manual/terrain-UsingTerrains.html) embody the mainstream **heightfield + paint layers + splat masks** workflow: artists **sculpt** and **blend** materials; code can **feed** heights and masks procedurally.

**In games**, these stacks are the **authoring hub** even when **height** comes from noise or Houdini: teams export or **drive** landscapes from tools, then layer **foliage**, **physics**, and **streaming** on the same grid.

**Maybraid:** **Workflow reference, not a mandate.** We are on **Bevy**, not Unreal/Unity terrain, but the **separation of concerns** carries over: **height oracle** + **material classification** + **decals** + **detail spawning**. Procedural **stamps** (Section 3) play the role of **brushes** and **layers**, with **deterministic** parameters instead of hand painting.

#### 2.2.4: GPU-oriented synthesis (noise, erosion-style passes on GPU)

[GPU Gems 3 — “Generating Complex Procedural Terrains Using the GPU”](https://developer.nvidia.com/gpugems/gpugems3/part-i-geometry/chapter-1-generating-complex-procedural-terrains-using-gpu) exemplifies **real-time** composition: **noise layers**, **blends**, and **erosion-like** iterations on **height textures** or **vertex buffers** under a **millisecond** budget.

**In games**, GPU height passes appear in **planet generators**, **editor previews**, and **runtime** deformation (craters, trails); CPU generation remains common when **determinism** and **simple** tooling matter more than peak throughput.

**Maybraid:** **Optional accelerator.** Our **stamp** and **oracle** model can run **CPU-side** first (easier to debug, match across LOD). GPU stages become attractive when we need **wide** batches (whole rings) or **interactive** edits, as long as **bit-identical** or **well-defined** fallbacks exist for **streaming** and **replay**. Not required for **low-poly** credibility; useful when **iteration speed** or **scale** dominates.

### 2.3: Exploratory code (non-normative)

There is **exploratory** terrain work in-tree ([`procedures/terrain`](../../procedures/terrain/), demo wiring in [`demos/naturescapes/src/terrain.rs`](../../demos/naturescapes/src/terrain.rs)). It experiments with a **2.5D height oracle** plus stacked **region-shaped modulations**, and with embedding that surface in a **3D signed distance field**, so volumetric composition (e.g. caves) remains conceivable at the representation level.

That code is **not** the specification for this RFC: it may be replaced, re-scoped, or discarded as the design in [Section 3](#3-design) stabilizes. When this document and the implementation diverge, **this document wins** until explicitly revised.

## 3: Design

The following sections state **normative intent** for Maybraid procedural terrain: vocabulary, composition patterns, and constraints. They do **not** prescribe a particular crate layout or the exploratory code in [Section 2.3](#23-exploratory-code-non-normative).

### 3.1: Core Concepts

- **Height oracle:** A well-defined way to evaluate **elevation** (and optionally **derivatives**) at horizontal coordinates `(x, z)` for a given world seed or LOD context. Callers that need ground height should use this oracle (or an approved cache), not reimplement ad hoc noise.
- **Stamp:** A **local operator** on the oracle: a **footprint** in the plane (hard mask, smooth falloff, or signed-distance blend) plus **parameters** (strength, orientation, path anchors, noise seeds). Stamps **compose** under a documented **policy** (sequential stack, DAG, priority buckets—choose per pipeline and document).
- **Fractal stamping:** Drive stamp **presence, type, or parameters** from **continuous** noise fields, so structure is **spatially correlated** across many samples. Suited to ridges, basins, and features that should not break on a rigid cell grid.
- **Cellular stamping:** Drive stamps from a **discrete cell key** and a **deterministic PRNG** when **local independence** is acceptable.
- **Semantics:** Optional **non-geometric** outputs co-emitted with a stamp (tags, spawn masks, hydrology hints). Semantics should be **queryable** without inferring them only from final height ([Section 3.6](#36-stamp-semantics)).

### 3.2: Noise Base

The **global base** before stamps is typically a **multi-octave** sum of smooth noise (fBm-style), optionally followed by **spectral shaping** (ridged variants, **domain warp**) or **iterative erosion-like** passes ([Section 2.1](#21-theory)). The base must be **deterministic** from world coordinates and seed, so streaming and LOD stay consistent.

### 3.3: Cellular Stamping

For a fixed **cell size**, decide whether a cell receives a stamp (and which type) using a **PRNG keyed by cell coordinates**. Use this when stamps **do not** need correlated structure across neighbors. For features that **must** span many cells (rivers, ranges, valley trains), prefer **fractal** (noise-driven) or **higher-order** planning ([Section 3.7.4](#374-higher-order-patterns-and-the-power-of-large-extents)).

### 3.4: Fractal Stamping

Use **low-, mid-, and high-frequency** noise (or derived fields) to control **where** stamps apply and **how strong** they are. Initial guidance by stamp family:

| Stamp family | Suggested noise role | Notes |
|--------------|----------------------|--------|
| Large landform (basin, plateau) | Low-frequency fBm / gradient noise | Strong spatial correlation; stable under LOD. |
| Ridge / cliff line | Ridged or absolute-value variants of smooth noise | Often needs **directional** bias or domain warp. |
| Valley / channel | Curve- or graph-guided field + detail noise | Pair with **path-consistent grade** ([Section 3.7.3](#373-fractal-paths)) where water should read correctly. |
| Scatter (boulders, small dips) | Mid-frequency noise + thresholds | Combine with cellular rules for variety. |
| Wobbly footprint | Noise **modulating a 2D distance field** | Perturbs stamp boundaries without hand-authored splines. |

**Recursive / multiscale** placement (one stamp spawning families of related stamps) can reuse the **same noise family** at shifted seeds or scales for **self-similar** layouts.

### 3.5: Stamp Generation

Stamps may **jitter** parameters, **noise-distort** footprints, or **subdivide** into child stamps. Randomness must be **reproducible** from `(seed, cell id, stamp id)` (or equivalent), so **streaming, replay, and LOD** agree.

### 3.6: Stamp Semantics

Some stamps should both **deform** the surface and **declare meaning**—for example, a riverbed that enforces grade **and** marks volume for aquatic spawns.

**Target:** keep **geometry** and **semantic payloads** in one evaluation episode but **logically separate** (height delta vs. tag set, mask, or graph edge), so downstream systems consume semantics without fragile height inversion.

### 3.7: Stamp Chains

Chains express **ordered or adjacent** stamp types (riverbed → waterfall → riverbed, ridge → saddle → ridge).

#### 3.7.1: Common Noise Chains

Drive chained stamps from a **shared low-dimensional field** (single or few noise images). **Discrete:** map value bands to stamp types. **Continuous:** interpolate parameters along isolines or along a scalar progression.

#### 3.7.2: Fractal Neighborhood Stamps (FNS)

**FNS** consults **neighboring samples** (grid or graph) of a field when deciding local stamp behavior, so **connectivity** (notches, aligned gaps, tunnel mouths) emerges without a full global solve. FNS does **not** replace **global** constraints (e.g. monotonic grade along a long reach); combine with paths or planners when needed.

#### 3.7.3: Fractal Paths

For **consistent deformation along a path**, parameterize a curve in the plane (`s` = arc length or similar) and drive height or blend weights with **1D noise, splines, or analytic grade**. **Endpoint constraints** (heights fixed by the oracle or by a planner at junctions) should be explicit, so paths **meet** the rest of the landscape.

#### 3.7.4: Higher-order Patterns and the Power of Large Extents

Purely **local** rules struggle with **global** consistency (e.g. a river that must meet distant pour points). Accept **multi-resolution** workflows: **coarse** plans (drainage graphs, mountain envelopes) **constrain** **fine** stamps.

**Macro-stamps** span many future streamed regions; loading a cell runs **local** routines **parameterized** by the macro stamp (nested grids, inner noise, inner chains). **Composing** macro layers aligns with **spatial hierarchies** (BVH-style LOD, coarse-to-fine generation): coarse nodes carry intent; leaves carry detail. **Culling** and **generation** should share hierarchy where practical.

### 3.8: Jersey Stamps

We propose the following stamps be released under the Jersey edition of Maybraid terrain.

## 4: Milestones

