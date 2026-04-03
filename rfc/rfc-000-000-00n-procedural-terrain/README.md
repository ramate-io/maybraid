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

**In games**, multi-octave sums are the standard way to get **foothills, ridges, and micro-roughness** from a single seed; many titles add **ridging**, **warp**, or **curve remapping** on top of the same stack.

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

This section specifies **how Maybraid should assemble procedural terrain going forward**: contracts between a noise base, stamps, stamp semantics, and chains. Nothing here asserts that a given subsystem already exists or matches exploratory code in [Section 2.3](#23-exploratory-code-non-normative); implementers should treat this as the target architecture and migrate toward it. It does **not** mandate crate layout.

### 3.1: Core Concepts

- **Height oracle:** Maybraid should expose **one** elevation API for horizontal coordinates `(x, z)` (and optionally **slopes or normals**) keyed by **world seed** and **LOD or chunk context**. Gameplay, physics, foliage, and procedural stamps that need ground height should call this oracle or a **documented cache** derived from it, instead of sampling noise ad hoc, so every system agrees after streaming and LOD changes.
- **Stamp:** A stamp is a **local operator**: a **footprint** in the plane (hard mask, smooth falloff, or signed-distance blend) plus **parameters** (strength, orientation, path anchors, noise seeds). Pipelines should pick and document a **composition policy** (ordered stack, DAG with priorities, or buckets), so evaluation order is not ambiguous when several stamps overlap.
- **Fractal stamping:** Use **continuous** noise (or derived fields) to drive whether a stamp applies, **which** stamp type runs, or **numeric parameters**, so landforms stay **spatially correlated** across many samples. Prefer this for **drainage valleys, mountain fronts, and ridge lines** that would look tiled or broken under independent cell dice.
- **Cellular stamping:** Use a **fixed cell grid** and a **PRNG keyed by cell coordinates** when **per-cell independence** is acceptable (scatter rocks, small hollows). Do **not** rely on cellular dice alone for **long reaches** that must line up (see hydrology under [Section 3.7](#37-stamp-chains)).
- **Stamp semantics:** Stamps that affect gameplay or simulation should emit **structured, non-geometric data** alongside height (tags, masks, graph hooks). **Hydrology is the reference example** throughout this RFC: a channel stamp should be able to expose **reach identity, flow direction, bank masks, and adjacency** to the next stamp downstream without forcing callers to infer water from height alone ([Section 3.6](#36-stamp-semantics)).

### 3.2: Noise Base

**Build order:** evaluate a **global base** first, then apply stamps. The base should normally be a **multi-octave** smooth-noise stack (fBm-style), optionally followed by **spectral shaping** (ridged noise, **domain warp**) or **lightweight erosion-like** passes as described in [Section 2.1](#21-theory). **Determinism:** the base **must** be a pure function of **world coordinates, seed, and LOD parameters**, so the same query after a chunk reload or at a coarser ring returns the same elevation and downstream hydrology stamps do not fight a shifting substrate.

### 3.3: Cellular Stamping

**Procedure:** fix a **cell size**; for each cell, hash `(cell_i, cell_j, world_seed)` into a PRNG; decide **boolean presence** and **stamp type** from that stream. **When to use:** props, potholes, and other features that do not need to **align across cell borders**. **When to avoid:** main-stem rivers, continuous ridges, or any feature whose **centerline or banks** must meet across streamed boundaries; those require **fractal** fields and/or **planned graphs** ([Section 3.7.4](#374-higher-order-patterns-and-the-power-of-large-extents)).

### 3.4: Fractal Stamping

Use **low-, mid-, and high-frequency** noise to choose **stamp influence masks** and **strength**. Tie frequency bands to physical scale, so LOD can drop high bands without changing where major valleys sit.

| Stamp family | Suggested noise role | Notes |
|--------------|----------------------|--------|
| Large landform (basin, plateau) | Low-frequency fBm / gradient noise | Establishes **watershed-scale** bowls and barriers before channel stamps run. |
| Ridge / cliff line | Ridged or absolute-value variants of smooth noise | Often needs **directional** bias or domain warp, so divides stay coherent. |
| Valley / channel | Curve- or graph-guided field plus detail noise | **Hydrology:** pair with **path-consistent grade** ([Section 3.7.3](#373-fractal-paths)), so pools and runs read as downhill. |
| Scatter (boulders, small dips) | Mid-frequency noise plus thresholds | Mix with cellular rules where independence is fine. |
| Wobbly footprint | Noise **modulating a 2D distance field** | Irregular bank lines without hand-authored splines. |

**Recursive / multiscale placement:** allow a stamp to **spawn child stamps** (e.g. main channel spawns **bars, cutbanks, and confluence pockets**) by reusing the **same noise family** at **offset seeds or scales**, so detail stays **self-similar** but still deterministic.

### 3.5: Stamp Generation

Stamps may **jitter** parameters, **noise-distort** footprints, or **subdivide** into children. **Reproducibility:** any randomness must be keyed by **`(world_seed, cell_or_region_id, stamp_id, sub_stamp_index)`** (or an equivalent tuple you document), so **streaming, replay, and LOD** all see the same geometry and the same **semantic payloads** (e.g. the same reach ID before and after a chunk boundary).

### 3.6: Stamp Semantics

**Goal:** stamps that carve **channels, lakeshores, or engineered grades** should both **move height** and **publish facts** downstream systems need. **Hydrology-first examples:**

- A **riverbed** stamp should be able to emit **wet mask**, **thalweg polyline or raster spine**, **flow direction**, and **optional graph edges** (“this reach continues to stamp instance *k*” or “confluence with reach *m*”).
- A **waterfall or grade break** stamp should record **drop height**, **overflow lip geometry**, and **upstream/downstream reach IDs**, so audio, particles, and fish spawning do not re-derive topology from triangles.
- **Consumers** (spawning, buoyancy, quest triggers, future flow solvers) should read these fields from a **query API**, not by thresholding final height (“blue below *z*”) unless you explicitly document that as a fallback.

Keep **geometry deltas** and **semantic records** in the **same evaluation pass** but **separate in data** (height change vs. tag set, masks, edges), so changing how height is blended does not silently erase **which cells belong to which reach**.

### 3.7: Stamp Chains

Chains are **ordered sequences** of stamp types applied along a **shared spatial or logical spine**. **Hydrology is the primary motivating pattern:** e.g. **meandering low-gradient bed**, then **falls or cascade**, then **bedrock slot**, then **low-gradient bed** again, all sharing one **centerline or drainage ID**. Non-hydrology chains (e.g. **ridge line, saddle, ridge line**) use the same machinery.

#### 3.7.1: Common Noise Chains

Drive several stamp types from **one low-dimensional field** (one or a few noise images or 1D curves along arc length). **Discrete:** map value bands to types (**pool, riffle, glide, fall**) along a reach. **Continuous:** interpolate **depth, width, or roughness** along **isolines** of that field or along **monotone distance-from-source**. The field should be **shared**, so transitions do not reset unrelated random state at segment boundaries.

#### 3.7.2: Fractal Neighborhood Stamps (FNS)

When choosing parameters for a stamp in cell *C*, **read neighboring samples** of the same underlying field (grid neighbors or graph neighbors along the channel). Use that to align **bank height, undercut notches, and confluence mouths**, so they **line up across cells** without a full global solver. **FNS does not replace** **global** constraints: if the **main stem must lose elevation monotonically from headwater to pour point**, still enforce that with a **path planner or stored graph** from [Section 3.7.4](#374-higher-order-patterns-and-the-power-of-large-extents); FNS only **dresses** local connectivity.

#### 3.7.3: Fractal Paths

Parameterize the **thalweg or road** as a plane curve with coordinate `s` (arc length). Drive **bed elevation or blend weights** with **1D noise, splines, or analytic grade** along `s`. **Endpoints:** fix heights at **junctions, lakes, and pour points** using the oracle or a **macro planner**, and document those constraints, **so** the path **meets** surrounding terrain without jumps. **Hydrology:** this is how you keep **pools and riffles** coherent while still hitting **required water-surface targets** at dams, confluences, and outlets.

#### 3.7.4: Higher-order Patterns and the Power of Large Extents

**Problem:** purely **local** per-chunk rules cannot guarantee that a **river meets a distant lake outlet** or that **parallel tributaries** drain to the same trunk. **Approach:** adopt **multi-resolution** workflows—**coarse** artifacts first, **fine** stamps inside them.

- **Coarse layer:** build a **drainage graph** (or skeleton field): trunk, tributaries, pour points, and **target elevations** at key nodes. This layer can be **sparse** and **computed infrequently**.
- **Fine layer:** when a chunk loads, run **local** stamp routines **parameterized** by the **macro reach** that crosses that chunk (inner noise, inner chains from [Section 3.7.1](#371-common-noise-chains) through [Section 3.7.3](#373-fractal-paths)). **Macro-stamps** therefore **apply across many chunks as they stream over time**; cell work is **interpolation and detail**, not re-deciding where the main stem goes.
- **Engine alignment:** **culling** and **generation** should share **the same spatial hierarchy** (BVH-style LOD, coarse-to-fine) where practical, so invisible work is not scheduled, and **semantic IDs** stay stable from coarse to fine.

### 3.8: Jersey Stamps

We propose the following stamps be released under the Jersey edition of Maybraid terrain.

## 4: Milestones

