# RFC-N: Procedural Terrain Generation

## 1: Motivation

Maybraid aims for **large, lightly authored** worlds. Terrain must stay **globally consistent** under **streaming** and **LOD**: the same horizontal coordinates should yield the same **elevation** (and compatible macro shape) whenever a chunk loads or a coarser ring replaces a finer one.

We treat terrain as a **noise base** accessed through one **height oracle**, then **deformed by stamps**—**fractal stamping** where landforms must correlate across many samples (valleys, ranges, **hydrology**), **cellular stamping** where **per-cell** independence is enough (scatter, small relief). **Stamp semantics** expose **non-geometric** facts to gameplay and simulation; **stamp chains** order stamp types along a shared spine (again with **rivers and reaches** as the lead example). [Section 2](#2-prior-art) surveys theory and engine practice against that vocabulary; [Section 3](#3-design) says how Maybraid should implement the stack. Open context: [maybraid#55 — Procedural terrain generation proposal](https://github.com/ramate-io/maybraid/issues/55).

## 2: Prior Art

What follows maps textbook methods and commercial patterns onto the same terms as [Section 1](#1-motivation): **noise base**, **height oracle**, **stamps** (fractal vs cellular), **stamp semantics**, **stamp chains**, **deterministic** evaluation, **streaming**, and **LOD**. Each **Maybraid** note is a fit judgment for that target stack, not a claim that the engine already implements it end to end.

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

**Maybraid:** **Strong fit.** The **SDF engine is 3D from the outset**—**caves**, **overhangs**, and **tunnels** are not an afterthought bolted onto a heightmap; they live in the same signed-distance story as exterior form. Pipelines can use **function analysis** on the implicit definitions—special cases, bounds, cheaper evaluators—where that applies, to **accelerate queries** rather than assuming "real 3D" always means brute-force voxels or full marching-cubes cost on every code path. Exploratory terrain hooks are in [Section 2.3](#23-exploratory-code-non-normative). **Scope tension** is still real: **Bevy** and low-poly art tend to favor **simple extracted meshes** for much of what ships in a demo; how much of the world is **SDF-primary** versus **height-oracle-first** is a **product and budget** choice, not a hard limit of the representation.

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

**W.l.o.g.,** the same **coarse-then-finer** recipe is not limited to one split: you can **nest it arbitrarily**—a layer that was "fine" relative to its parent can itself become the **coarse** scaffold for another inner pass—so **higher-order** terrain structure and **fractal** refinement of stamping are the same mechanism **applied recursively**, subject to the usual **determinism** and **semantic ID** rules ([Section 3.5](#35-stamp-generation)).

### 3.8: Jersey Stamps

**Jersey** is the working name for the **first curated bundle** of terrain stamp families aimed at demos and vertical slices. Names below are **product-facing** groupings; one family may compile to several internal stamp types. Together they exercise **fractal** and **chained** patterns from [Sections 3.4](#34-fractal-stamping)–[3.7](#37-stamp-chains), plus **localized volume** work aligned with the SDF discussion in [Section 2.1.3](#213-implicit-surfaces-marching-cubes-and-csg). **Hydrology-aware** work appears both as **lake-and-stream chains** (**Jersey Pocket Waters**, **Jersey Basin Waters**) and as **separate Jersey lines** for **canyons** and **hydrology-related landform complexes**, so authoring and docs stay simple even when the underlying math overlaps.

**Look and layering:** Every Jersey family is meant to run **on top of** the **noise base** ([Section 3.2](#32-noise-base)), not on a flat synthetic plane. Stamps should lean on **noise** for **placement, strength, footprint warp, and micro-breakup** ([Section 3.4](#34-fractal-stamping), [Section 3.5](#35-stamp-generation)), so the result stays **rough and natural-looking**—strong landforms read clearly, but they still **inherit the fractal grain** of the base instead of looking like smooth CAD inserts.

#### 3.8.1: Jersey Valley Basins (Unchained)

**Purpose:** Single-region **valley** depressions with parameterized **cross-section** (V-shaped, U-shaped, or asymmetric), **width**, **axis curvature**, and **bank falloff**. Placement and strength should be **fractal-driven** ([Section 3.4](#34-fractal-stamping)), so long valleys do not tile like independent cellular tiles.

**Semantics (recommended):** **bank mask** for foliage and splats; optional tags distinguishing **dry arroyo** profiles from **spillway-ready** floors that a later hydrology stamp can occupy.

**LOD:** fix **macro axis and depth** early; treat **micro bank breakup** as a high-frequency layer that can weaken at distance without moving the thalweg.

#### 3.8.2: Jersey Plateau Caps (Unchained)

**Purpose:** **Tablelands** and **mesa-style caps**: raised interior with gentle **tilt**, **escarpment** strength at the rim, and controlled **corner behavior**. Footprints can be **convex polygons**, smooth **blobs**, or **noise-warped** boundaries using the “wobbly footprint” idea from [Section 3.4](#34-fractal-stamping).

**Semantics (recommended):** **surface class** (e.g. exposed cap rock vs soil mantle) for materials and props.

#### 3.8.3: Jersey Rugged Massifs (Unchained)

**Purpose:** **Ridged**, **serrated**, or **cliff-banded** high terrain—peaks, **arêtes**, and broken crests—using the same spectral toolkit as the ridge row in [Section 3.4](#34-fractal-stamping). Often stacked **after** coarse envelope stamps, so **crests** inherit watershed-scale context.

**Semantics (optional):** **exposure** or **rockiness** masks for scree and cliff props.

#### 3.8.4: Jersey Pocket Waters (Small Hydrology Chains)

**Purpose:** **Stamp chains** for **small** closed systems: a **pond or tarn** body, optional **outlet lip**, a short **run or riffle**, and a documented **termination** (another sink, marsh hint, or hand-off tag). All instances along the chain share one **drainage ID** ([Section 3.7](#37-stamp-chains), [Section 3.6](#36-stamp-semantics)).

**Semantics (required for water gameplay):** **water-surface target** where applicable; **reach graph** (typically a handful of edges); **flow direction** on the run; **bank** or **littoral** masks.

#### 3.8.5: Jersey Basin Waters (Large Hydrology Chains)

**Purpose:** **Macro hydrology** bundles: **lake or reservoir-scale** water bodies, **branched outlet systems**, and **tributary stubs** or **confluence nodes**, parameterized from a **coarse drainage graph** ([Section 3.7.4](#374-higher-order-patterns-and-the-power-of-large-extents)). Designed so **macro reaches** stay stable as chunks stream.

**Semantics (required):** **reach IDs**, **junction records**, **pour-point** or **outlet** targets for downstream systems; optional **seasonal** or **regulated** level hints.

#### 3.8.6: Jersey Valley Trains (Chained Valleys)

**Purpose:** **Ordered** valley stamps along a **shared horizontal spine** (headwater to base level): for example **upper gorge**, **middle glide**, **lower widened floor**. Segment heights obey **endpoint constraints** from a **macro planner** or oracle ([Section 3.7.3](#373-fractal-paths)).

**Semantics (recommended):** per-segment tags indicating **active channel** vs **floodplain-only**, so hydrology overlays know where to bind running water.

#### 3.8.7: Jersey Canyons (Confined Incision)

**Purpose:** **Morphology-first** stamps for **confined** terrain: **slots**, **narrows**, **gorge walls**, and **vertical relief** along a **spine** (dry or wet). This is **hydrology-adjacent**—many canyons host **ephemeral or perennial** channels—but Jersey treats **canyons** as their own product line so tooling emphasizes **wall height**, **confinement ratio**, **bench** shelves, and **overhang** risk rather than only **water-surface** targets.

**Variants:** **unchained** (single enclosed reach of incision) or **chained** segments (**upper slot**, **wider box canyon**, **exit ramp**) along one **centerline**, using [Section 3.7.3](#373-fractal-paths) endpoint discipline.

**Semantics (recommended):** **wall** or **cliff** masks; **floor** vs **ledge** classification; optional **thalweg** or **dry-channel** spine for downstream **Pocket** or **Basin** water stamps to bind without re-deriving confinement from height alone.

#### 3.8.8: Jersey Hydrology Complexes (Multi-Part Landforms)

**Purpose:** **Packaged** stamp groups that describe a **single named geomorphic system** made of several interacting pieces—still **grounded in drainage logic**, but **not** sold as the same SKU as **Pocket Waters** or **Basin Waters** (those focus on **graph-like** lake–stream–pour-point networks). Examples: **alluvial fan head + incised distributaries + toe**; **sink–polje–resurgence**-style stepped flats; **plunge–pool + rapid ladder + glide pool** along one **macro reach**; **terrace stair** with **paired** cutbanks.

**Authoring:** either a **macro footprint** or a **graph seed** drives child stamps arranged as a **chain** (sequential) or **DAG** (parallel), with a shared **complex ID**, so consumers see **one** logical feature.

**Semantics (required):** **complex type** tag; **constituent** roles (fan apex, main stem, overflow sill, etc.); **reach** or **segment edges** where water could attach; optional **seasonal** routing hints.

#### 3.8.9: Jersey Karst Pockets (Small Caves, Unchained)

**Purpose:** **Localized** cavities: **sinkhole mouths**, short **alcoves**, or **rubble-choked** pockets. Implementation may be **SDF-local** (native 3D, [Section 2.1.3](#213-implicit-surfaces-marching-cubes-and-csg)) or a **height-oracle** dip plus **volumetric tag** when caves are represented lightly.

**Semantics (recommended):** **cavity mask**, **entrance** curve or portal disk, **navigation** class (passable, crawl-only, hazard).

#### 3.8.10: Jersey Cave Networks (Chained Caves)

**Purpose:** **Chains** of **passage** stamps along a **3D spine** (mouth, slot, chamber, sump or daylight exit), analogous to hydrology chains but in **tunnel parameter** space. Reuses chain discipline from [Section 3.7](#37-stamp-chains): shared **tunnel graph ID**, deterministic **sub-stamp** keys ([Section 3.5](#35-stamp-generation)).

**Semantics (recommended):** **branch nodes**, **air vs flooded** segments, **graph edges** for audio, lighting, and spawn zoning.

#### 3.8.11: Jersey Rolling Ground (Unchained)

**Purpose:** **Gentle** swell and swale on **valley floors**, **plateau interiors**, or **piedmont** surfaces without opening new primary drainages. Use **mid-frequency** fractal modulation chosen, so it **does not fight** larger valley or plateau stamps.

**Semantics (optional):** **pasture / agriculture suitability** or generic **detail** mask for scatter rules.

## 4: Milestones

Milestones below are **planning hooks**, not dated commitments. Except for [Section 4.3](#43-jersey-stamp-milestones), they are **suggestive**: this RFC does not lock engine layout, crate boundaries, or BVH APIs—teams should adapt wording to whatever design documents and codebases they adopt.

### 4.1: Noise and stamping abstraction (suggestive)

- **Height oracle contract:** callable **elevation** (and optional **derivatives**) from `(x, z)` plus **seed / LOD or chunk context**, with tests that the same query is stable across reloads ([Section 3.1](#31-core-concepts), [Section 3.2](#32-noise-base)).
- **Noise base pipeline:** documented **build order** (global base before stamps), **fBm-style** stack plus optional spectral shaping hooks, and a path to **swap** noise implementations without changing callers.
- **Stamp core:** **footprint** types (hard mask, falloff, SDF blend), **parameter** bundle, and a **documented composition policy** (stack, DAG, or buckets) with deterministic overlap resolution ([Section 3.1](#31-core-concepts)).
- **Fractal vs cellular:** at least one **fractal-driven** placement path and one **cell PRNG** path, both feeding the same stamp evaluator ([Section 3.3](#33-cellular-stamping), [Section 3.4](#34-fractal-stamping)).
- **Reproducibility:** randomness keyed by an agreed tuple (e.g. seed, region, stamp ID, sub-stamp index) verified in **streaming** and **replay** scenarios ([Section 3.5](#35-stamp-generation)).
- **Semantics v0:** stamps can attach **queryable** payloads (tags, masks, sparse graph edges); at least one **hydrology-shaped** example end-to-end ([Section 3.6](#36-stamp-semantics)).

### 4.2: MVP BVH implementation (suggestive)

- **Spatial index v0:** a **bounding-volume hierarchy** (or equivalent) over **terrain chunks**, **stamp macro regions**, or **both**, sufficient for **frustum / distance** rejection in a demo scene.
- **Single-writer rule:** document how **generation** and **culling** agree on **node bounds** and **versioning** when terrain updates (even if updates are rare in the MVP).
- **Coarse LOD linkage:** MVP may use **one or two** discrete LODs; milestones should still record **which oracle parameters** change per level, so Jersey stamps can be tested against **pop** and **shift** behavior ([Section 3.7.4](#374-higher-order-patterns-and-the-power-of-large-extents)).
- **Debug visibility:** draw or log **BVH nodes** (optional overlay) to validate **hierarchy depth** and **overlap** against stamp footprints.

### 4.3: Jersey stamp milestones

Each milestone below maps to **one** Jersey family in [Section 3.8](#38-jersey-stamps). **Done when** means: behavior matches that family’s **Purpose** and **Semantics** bullets, respects **Look and layering** (noise-on-base), and stays **deterministic** under [Section 4.1](#41-noise-and-stamping-abstraction-suggestive).

**Suggested sequencing:** **4.3.1**–**4.3.3** and **4.3.11** can land early. **4.3.4**–**4.3.8** need **chain + semantics** maturity. **4.3.9**–**4.3.10** need the chosen **SDF / volume** path.

#### 4.3.1: Milestone — Jersey Valley Basins (Unchained)

**Spec:** [Section 3.8.1](#381-jersey-valley-basins-unchained).

**Done when:** Fractal-driven **valley** depression with parameterized **cross-section**, **width**, **axis**, and **bank falloff**; **bank** (and optional arroyo vs spillway-ready) **semantics**; **thalweg** stable across **LOD** reloads.

#### 4.3.2: Milestone — Jersey Plateau Caps (Unchained)

**Spec:** [Section 3.8.2](#382-jersey-plateau-caps-unchained).

**Done when:** **Tableland** interior with rim **escarpment** and controlled corners; **noise-warped**, blob, or polygon **footprints**; **surface class** semantic for materials or props.

#### 4.3.3: Milestone — Jersey Rugged Massifs (Unchained)

**Spec:** [Section 3.8.3](#383-jersey-rugged-massifs-unchained).

**Done when:** **Ridged / cliff-banded** high terrain consistent with [Section 3.4](#34-fractal-stamping) ridge-style noise; stacks sensibly **after** coarse envelopes; optional **exposure** or **rockiness** mask.

#### 4.3.4: Milestone — Jersey Pocket Waters (Small Hydrology Chains)

**Spec:** [Section 3.8.4](#384-jersey-pocket-waters-small-hydrology-chains).

**Done when:** **Chain** of small hydrology stamps (pond or tarn, outlet, short run, termination) sharing one **drainage ID**; **water-surface** targets where needed; **reach graph**, **flow direction**, **bank** or **littoral** masks per §3.8.4.

#### 4.3.5: Milestone — Jersey Basin Waters (Large Hydrology Chains)

**Spec:** [Section 3.8.5](#385-jersey-basin-waters-large-hydrology-chains).

**Done when:** **Macro** lake or reservoir body with **branched** outlets and tributary or confluence nodes driven from a **coarse drainage graph**; **reach IDs**, **junction** records, and **pour-point** targets **stable** as chunks stream.

#### 4.3.6: Milestone — Jersey Valley Trains (Chained Valleys)

**Spec:** [Section 3.8.6](#386-jersey-valley-trains-chained-valleys).

**Done when:** **Ordered** valley stamps on one **horizontal spine** with **endpoint** heights from **macro planner** or oracle ([Section 3.7.3](#373-fractal-paths)); per-segment **active channel** vs **floodplain-only** tags for hydrology overlays.

#### 4.3.7: Milestone — Jersey Canyons (Confined Incision)

**Spec:** [Section 3.8.7](#387-jersey-canyons-confined-incision).

**Done when:** **Confined incision** (unchained or **chained** gorge segments) with **wall height** and **confinement** tooling; **wall** / **cliff**, **floor** / **ledge** semantics; optional **thalweg** or **dry-channel** spine for binding **Pocket** or **Basin** water later.

#### 4.3.8: Milestone — Jersey Hydrology Complexes (Multi-Part Landforms)

**Spec:** [Section 3.8.8](#388-jersey-hydrology-complexes-multi-part-landforms).

**Done when:** **Multi-part** stamp group under one **complex ID**; **chain** or **DAG** child arrangement; **complex type**, **constituent roles**, and **reach** or **segment** edges; optional **seasonal** routing hints per §3.8.8.

#### 4.3.9: Milestone — Jersey Karst Pockets (Small Caves, Unchained)

**Spec:** [Section 3.8.9](#389-jersey-karst-pockets-small-caves-unchained).

**Done when:** **Localized** cavity or sink **entrance** via **SDF-local** carve and/or **height-oracle** dip plus **volumetric tag**; **cavity mask**, **entrance** geometry, **navigation** class semantic.

#### 4.3.10: Milestone — Jersey Cave Networks (Chained Caves)

**Spec:** [Section 3.8.10](#3810-jersey-cave-networks-chained-caves).

**Done when:** **Chained** passage stamps on a **3D spine** with shared **tunnel graph ID**; **branch**, **air vs flooded**, and **graph edge** semantics for downstream systems ([Section 3.5](#35-stamp-generation) sub-stamp keys).

#### 4.3.11: Milestone — Jersey Rolling Ground (Unchained)

**Spec:** [Section 3.8.11](#3811-jersey-rolling-ground-unchained).

**Done when:** **Mid-frequency** swell and swale that does not overpower **valley** or **plateau** stamps; optional **pasture / agriculture suitability** or generic **detail** mask.

### 4.4: Full BVH and streaming (suggestive)

- **Shared hierarchy:** **culling** and **terrain generation** consume the **same** BVH (or strictly synchronized mirrors), so invisible regions do not schedule expensive stamp work ([Section 3.7.4](#374-higher-order-patterns-and-the-power-of-large-extents)).
- **Streaming correctness:** chunk **load / unload** does not change **deterministic** height or **semantic IDs** for regions that remain loaded; document **handshake** between **macro-stamps** and **fine** local passes when new neighbors appear.
- **Multi-resolution BVH:** **coarse-to-fine** nodes aligned with **LOD** rings or clipmap-style bands; **semantic IDs** stable from coarse to fine.
- **Scale stress:** targets for **node count**, **refit** cost, and **worst-case** depth on **discovery-scale** worlds (numbers left to engine RFCs).
- **Failure modes:** defined behavior when **planner data** arrives late (fallback height, **degraded** semantics, or **explicit** “not ready” query), without silent graph corruption.
