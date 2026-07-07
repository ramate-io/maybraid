# RFC-87: Character Generation

## Table of contents

- [1: Background](#1-background)
- [2: Prior Art](#2-prior-art)
- [3: Proposed Design](#3-proposed-design)
  - [3.1: Species](#31-species)
    - [3.1.1: Crozon Species Design](#311-crozon-species-design)
    - [3.1.2: Crozon Species Diversity](#312-crozon-species-diversity)
  - [3.2: Multi-meshes](#32-multi-meshes)
    - [3.2.1: Crozon Head Shape Variant Multi-meshes](#321-crozon-head-shape-variant-multi-meshes)
    - [3.2.2: Crozon Ear Variant Multi-meshes](#322-crozon-ear-variant-multi-meshes)
    - [3.2.3: Crozon Horn Variant Multi-meshes](#323-crozon-horn-variant-multi-meshes)
    - [3.2.4: Crozon Eye Variant Multi-meshes](#324-crozon-eye-variant-multi-meshes)
    - [3.2.5: Crozon Nose and Snout Variant Multi-meshes](#325-crozon-nose-and-snout-variant-multi-meshes)
    - [3.2.6: Crozon Mouth Variant Multi-meshes](#326-crozon-mouth-variant-multi-meshes)
    - [3.2.7: Crozon Neck Variant Multi-meshes](#327-crozon-neck-variant-multi-meshes)
    - [3.2.8: Crozon Lower Limb Variant Multi-meshes](#328-crozon-lower-limb-variant-multi-meshes)
    - [3.2.9: Crozon Upper Limb Variant Multi-meshes](#329-crozon-upper-limb-variant-multi-meshes)
    - [3.2.10: Crozon Hand and Foot Variant Multi-meshes](#3210-crozon-hand-and-foot-variant-multi-meshes)
    - [3.2.11: Crozon Torso Variant Multi-meshes](#3211-crozon-torso-variant-multi-meshes)
    - [3.2.12: Crozon Tail Variant Multi-meshes](#3212-crozon-tail-variant-multi-meshes)
  - [3.3: Animations](#33-animations)
    - [3.3.1: Malo Biped Walk](#331-malo-biped-walk)
    - [3.3.2: Malo Biped Run](#332-malo-biped-run)
    - [3.3.3: Malo Biped Leap](#333-malo-biped-leap)
    - [3.3.4: Malo Quadruped Walk](#334-malo-quadruped-walk)
    - [3.3.5: Malo Quadruped Run](#335-malo-quadruped-run)
    - [3.3.6: Malo Quadruped Leap](#336-malo-quadruped-leap)
    - [3.3.7: Malo Biped IK Carry](#337-malo-biped-ik-carry)
    - [3.3.8: Malo Biped Fly](#338-malo-biped-fly)
    - [3.3.9: Malo Facial Expressions](#339-malo-facial-expressions)
- [4: Milestones](#4-milestones)
  - [4.1: Crozon Head Shape Variant Multi-meshes](#41-crozon-head-shape-variant-multi-meshes)
  - [4.2: Crozon Ear Variant Multi-meshes](#42-crozon-ear-variant-multi-meshes)
  - [4.3: Crozon Horn Variant Multi-meshes](#43-crozon-horn-variant-multi-meshes)
  - [4.4: Crozon Eye Variant Multi-meshes](#44-crozon-eye-variant-multi-meshes)
  - [4.5: Crozon Nose and Snout Variant Multi-meshes](#45-crozon-nose-and-snout-variant-multi-meshes)
  - [4.6: Crozon Mouth Variant Multi-meshes](#46-crozon-mouth-variant-multi-meshes)
  - [4.7: Crozon Head Assembly Variant Multi-meshes](#47-crozon-head-assembly-variant-multi-meshes)
  - [4.8: Crozon Neck Variant Multi-meshes](#48-crozon-neck-variant-multi-meshes)
  - [4.9: Crozon Lower Limb Variant Multi-meshes](#49-crozon-lower-limb-variant-multi-meshes)
  - [4.10: Crozon Upper Limb Variant Multi-meshes](#410-crozon-upper-limb-variant-multi-meshes)
  - [4.11: Crozon Hand and Foot Variant Multi-meshes](#411-crozon-hand-and-foot-variant-multi-meshes)
  - [4.12: Crozon Torso Variant Multi-meshes](#412-crozon-torso-variant-multi-meshes)
  - [4.13: Crozon Tail Variant Multi-meshes](#413-crozon-tail-variant-multi-meshes)
  - [4.14: Biped Skeleton](#414-biped-skeleton)
  - [4.15: Quadruped Skeleton](#415-quadruped-skeleton)
  - [4.16: Develop Multi-mesh API](#416-develop-multi-mesh-api)
  - [4.17: Crozon Species Assembly](#417-crozon-species-assembly)
  - [4.18: Malo Gait Animation for Bipeds](#418-malo-gait-animation-for-bipeds)
  - [4.19: Malo Gait Animation for Quadrupeds](#419-malo-gait-animation-for-quadrupeds)
  - [4.20: Malo Carrying and Grasping Animation for Bipeds](#420-malo-carrying-and-grasping-animation-for-bipeds)
  - [4.21: Malo Expressions Animations](#421-malo-expressions-animations)

## 1: Background

Maybraid intends to rely on procedural generation, including for the generation of characters. We propose a character generation system and roadmap to this end. 

The system proposed is a basic assembly of multi-meshes with species types controlling high-order patterns. Generally speaking, a species will restrict features to a set of 

In early development, we prioritize simple bi- and quadrupedal designs. We describe how to work to these designs from basic topology: spheroids, cylinders, and simple polygonal volumes. 

Following from our low-poly look, we intend to specify these designs with minimal skinning. 

Animations of the characters will also be subject to slight variation via procedural generation. 

## 2: Prior Art

> [!NOTE]
> Because Maybraid is developed without a dedicated art team, our goal may differ from some studios. Instead of working backwards from lore-based concepts or complete character impressions, we are trying somewhat to work up from components.
>
> We want to design compelling components that we assemble into multi-meshes. 
>
> We allow our artistic direction to emerge with constraints. 

Procedural character generation usually combines a few layers, rather than one monolithic algorithm:

- **Template + parameter variation**: start from a hand-authored base topology and vary scale, proportions, feature toggles, and material palettes. This is common in games because it is stable, art-directable, and easy to constrain (for a production example, see `MetaHuman`).
- **Part library assembly (modular kits)**: generate characters by composing reusable parts (head, torso, limbs, ears, horns, tails), then enforce compatibility constraints. This aligns well with our multi-mesh-first approach (see [MB-Lab](https://github.com/animate1978/MB-Lab)).
- **Rule/grammar-driven generation**: encode allowed combinations and dependencies as rules (for example, species trait implies a family of limb and skull shapes). This is often used to keep outputs coherent while preserving variety (see [Shape Grammars](https://en.wikipedia.org/wiki/Shape_grammar) and [L-systems](https://en.wikipedia.org/wiki/L-system)).
- **Morph/blend-space variation**: blend between shape keys or latent feature sliders to produce smooth families of forms. This is powerful for continuity, but can be heavier than needed for low-poly, rigid-part pipelines (see [SMPL](https://smpl.is.tue.mpg.de/)).
- **Skeleton-aware proceduralization**: generate or adjust meshes with explicit awareness of target rig constraints (joint limits, attachment sockets, gait assumptions), so downstream animation remains viable (see [Mixamo auto-rig](https://www.mixamo.com/) and [SIGGRAPH 2010: Example-Based Facial Rigging](https://www.hao-li.com/publications/papers/siggraph2010EBFR.pdf)).

For low-poly styles, teams commonly favor **modular assembly + constrained variation** over heavy skinning. Intersections, silhouette readability, and discrete part transitions are often acceptable or stylistically desirable, as long as rig anchors and motion envelopes are consistent.

Typical production pattern is:

1. Define a **canonical anatomy schema** (named slots/anchors and species constraints).
2. Author **variant libraries** per slot.
3. Add **compatibility rules** and weighted sampling.
4. Validate against **rig/animation constraints**.
5. Expose generation as deterministic routines (seeded) for reproducibility.

Representative references and talks:

- [No Man's Sky - Procedural Generation Toolkit (GDC)](https://www.youtube.com/watch?v=sCRzxEEcO2Y)
- [Spore Creature Creator (overview)](https://en.wikipedia.org/wiki/Spore_Creature_Creator)
- [SMPL: A Skinned Multi-Person Linear Model](https://smpl.is.tue.mpg.de/)
- [MB-Lab (Blender procedural human add-on)](https://github.com/animate1978/MB-Lab)

## 3: Proposed Design

### 3.1: Species

#### 3.1.1: Crozon Species Design

A species consists of the following:

1. A set of features.
2. A mapping of features to allowed seed-able multi-meshes.
3. A mapping of features to allowed parameters: pattern and dimension.

An individual in a species narrows this space:

1. Selects all features in the set
2. In a pre-determined order, chooses a single seeded multi-mesh per feature.
3. Chooses parameters for pattern and dimension. 

Drawing from an L-Grammar, when an individual selects a given multi-mesh for a feature, it may remove options or otherwise impose constraints on other mappings. For example, after choosing a certain pattern for the head, it may impose this pattern on the rest of the body. 

Most species will choose to encode symmetrical components as one feature. For example, left and right limbs will use the same multi-mesh-pattern-dimension mapping--producing the separate limbs only at assembly time. 

As a first construction pass, this is a strong baseline. Three design risks follow from the same ingredients (ordered features, propagating constraints, and bilateral assembly). Treat them as requirements, not footnotes.

**Feature order and sampling bias.** A fixed global order interacts with constraint propagation: early choices can shrink later option sets unevenly, so marginal distributions over full individuals need not match naive per-feature weights. Mitigations: (1) sample meshes and parameters with weights conditioned on current state $\sigma$, not only on the slot; (2) derive the visit order from the seed (e.g. a species-defined partial order with a topological shuffle, or random permutation among compatible layers) when order is not semantically fixed; (3) validate empirically by histogramming accepted individuals and adjusting weights or order priors until targets are met.

**Dead-ends, retries, and backtracking.** When choices restrict later slots, greedy commitment can reach a state with no valid completion. The implementation should not assume the first walk succeeds. Prefer **forward checking** after each commit: compute $\mathrm{Remaining}(f,\sigma)$ for not-yet-assigned features and reject a candidate early if any future slot becomes empty. When the generator still fails, use a bounded **retry** with deterministic salt $s' = s \oplus k$, or **backtrack** (undo last assignment and try the next candidate). Keep retry/backtrack policies explicit in the species spec so determinism and worst-case cost are clear.

**Symmetry as explicit policy.** Encoding left and right as one logical feature and instantiating mirrored geometry at assembly is efficient, but “symmetry” still needs a first-class rule: **perfect mirror** (same mesh, mirrored transform), **near-mirror** (shared pattern with independent small jitter on scale or attach for organic asymmetry), or **asymmetric override** (independent choices per side when the species allows). Record that policy per bilateral pair (or globally per species) so tooling, QA, and constraint code do not implicitly assume perfect mirroring.

Formally, model a species as follows. Let $F = (f_1, \ldots, f_n)$ be a **finite, ordered** sequence of features. For each $f \in F$, let $\mathcal{M}_f$ be the set of allowed multi-meshes and $\mathcal{P}_f$ the domain of allowed parameters (pattern and dimension). A species defines a constraint predicate or relation $C$ over tuples $(f, m, p)$ with $m \in \mathcal{M}_f$ and $p \in \mathcal{P}_f$.

An individual with seed $s \in \mathbb{N}$ (or a bitvector) instantiates deterministic maps on a mutable state $\sigma \in \Sigma$:

$$
\mathrm{chooseMesh}_s : F \times \Sigma \to \bigcup_{f \in F} \mathcal{M}_f, \qquad
\mathrm{chooseParams}_s : F \times \mathcal{M}_f \times \Sigma \to \mathcal{P}_f,
$$

$$
\sigma' = \mathrm{apply}_C(f, m, p, \sigma).
$$

At each step $i$, after committing $(f_i, m_i, p_i)$, require non-emptiness of remaining feasible choices for the next feature:

$$
\mathrm{Remaining}(f_{i+1}, \sigma_i) \neq \varnothing.
$$

If the invariant fails, retry with a deterministic salt $s' = s \oplus k$ for $k \in \mathbb{N}$, or backtrack.

```rust
// Pseudocode: types intentionally simplified.
type Seed = u64;

#[derive(Clone, Copy, Hash, Eq, PartialEq, Debug)]
enum Feature {
    HeadShape,
    Ear,
    Horn,
    Eye,
    NoseSnout,
    Mouth,
    Neck,
    LowerLimb,
    UpperLimb,
    HandFoot,
    Torso,
    Tail,
}

#[derive(Clone, Debug)]
struct SpeciesSpec {
    order: Vec<Feature>,
    meshes_by_feature: HashMap<Feature, Vec<MeshId>>,
    params_by_feature: HashMap<Feature, ParamDomain>,
    constraints: Vec<ConstraintRule>,
    symmetry: SymmetryPolicy,
}

#[derive(Clone, Debug, Default)]
struct BuildState {
    chosen: HashMap<Feature, (MeshId, Params)>,
    forbidden: HashMap<Feature, HashSet<MeshId>>,
}

fn generate_individual(spec: &SpeciesSpec, seed: Seed) -> Result<BuildState, GenError> {
    for retry in 0..MAX_RETRIES {
        let mut rng = SeedRng::new(seed ^ retry as u64);
        let mut state = BuildState::default();
        let mut ok = true;

        for &feature in &spec.order {
            let mesh_opts = allowed_meshes(spec, feature, &state);
            if mesh_opts.is_empty() {
                ok = false;
                break;
            }
            let mesh = sample_weighted(&mesh_opts, &mut rng);
            let params = sample_params(spec, feature, mesh, &state, &mut rng)?;

            state.chosen.insert(feature, (mesh, params));
            apply_rules(spec, feature, &mut state);

            if violates_future_viability(spec, &state) {
                ok = false;
                break;
            }
        }

        if ok {
            apply_symmetry(spec.symmetry, &mut state);
            return Ok(state);
        }
    }
    Err(GenError::NoValidAssembly)
}
```


#### 3.1.2: Crozon Species Diversity

> [!NOTE]
> In RFC-87, we do not provide strict species definitions. We leave this for future RFCs. We do provide desiderata for the species and concept art for various limb parts. 

The initial offering of species is called Crozon. Subspecies should follow from procedural generation extension of basic routines. However, we expect the following distinct species routines in Crozon:

- Four humanoid species
- Six grazing species, building for quadruped skeleton
- Three large land predators, building for quadruped skeleton
- Three small land animals, building for quadruped skeleton
- One very large "noble" land animal, building for either quadruped or biped skeleton
- Six medium flying creatures, building for bipedal skeleton
- One very large "noble" flying creature, building for biped skeleton. 
- Three medium-size swimming creatures, building for the quadruped skeleton.
- Three small swimming creatures, building for the quadruped skeleton.
- One very large "noble" swimming creature, building for the quadruped skeleton. 

### 3.2: Multi-meshes

> [!TIP]
> Concept art for the multi-meshes is linked by section to [`variants/crozon/`](./variants/crozon/). **Shared shape primitives** (cylinder, drumstick, bow, etc.) live under [`variants/crozon/common/`](./variants/crozon/common/README.md); each **feature** folder (lower limb, horn, …) links to those definitions and adds embedding notes for that slot.

#### 3.2.1: Crozon Head Shape Variant Multi-meshes
- Concept set: [`head-shape`](./variants/crozon/head-shape/README.md) (primitives: [`common`](./variants/crozon/common/README.md))

Head shape variants model the macro **cranium / face block** as one or a few solids chosen from shared primitives. The usual pattern is a **parent** head mass with **optional child** meshes for mouth, nose–snout, and other features parented in place. Embedding notes describe orientation and scale, so the silhouette reads correctly for the species without over-specifying anatomy.

#### 3.2.2: Crozon Ear Variant Multi-meshes
- Concept set: [`ear`](./variants/crozon/ear/README.md) (primitives: [`common`](./variants/crozon/common/README.md))

Ears are treated as **composable multi-mesh** kits: for example lobe, cartilage fold, and optional fur or detail strips as separate meshes or instances. Each variant row ties a common primitive to **ear-specific** placement and pairing rules (left/right, alertness). The goal is reusable ear recipes that still read clearly at low poly counts.

#### 3.2.3: Crozon Horn Variant Multi-meshes
- Concept set: [`horn`](./variants/crozon/horn/README.md) (primitives: [`common`](./variants/crozon/common/README.md))

Horns are typically **one multi-mesh per horn** or **paired instances** of the same spec for symmetry. Variants differ by taper, curvature, and cross-section (cone-like, cylindrical, bowed, and so on) while sharing the same attachment semantics at the skull. Species rules decide count, mirroring, and seed jitter rather than duplicating primitive definitions.

#### 3.2.4: Crozon Eye Variant Multi-meshes
- Concept set: [`eye`](./variants/crozon/eye/README.md) (primitives: [`common`](./variants/crozon/common/README.md))

Eyes are almost always **multi-mesh**: a **globe**, a **socket** or rim volume, and optional **lid** or related strips. Primitives supply the underlying solids; feature stubs describe how those pieces stack and align for blink, gaze, and stylized reads. The assembly stays small enough to rig and instance consistently across species.

#### 3.2.5: Crozon Nose and Snout Variant Multi-meshes
- Concept set: [`nose-snout`](./variants/crozon/nose-snout/README.md) (primitives: [`common`](./variants/crozon/common/README.md))

Nose and snout kits reuse the same **common** primitives as elsewhere but document **face-forward** embedding: bridge, muzzle length, and taper relative to the head block. A variant is often a single primary snout mesh plus optional add-ons where species need them. The emphasis is on readable silhouette and clean seams to head shape and mouth.

#### 3.2.6: Crozon Mouth Variant Multi-meshes
- Concept set: [`mouth`](./variants/crozon/mouth/README.md) (primitives: [`common`](./variants/crozon/common/README.md))

Mouth variants combine an **opening volume** with **lip**, **beak**, or transition geometry, and may add **teeth** or other rows as separate meshes. Chains and bowed primitives support curved jaw lines and stylized muzzles without a single monolithic mouth mesh. Child meshes are typically parented so jaw motion and attachment to head shape stay predictable.

#### 3.2.7: Crozon Neck Variant Multi-meshes
- Concept set: [`neck`](./variants/crozon/neck/README.md) (primitives: [`common`](./variants/crozon/common/README.md))

Neck volumes bridge **head** and **torso** using cylinders, bows, or pearl chains with explicit **attachment** and **bow-plane** notes. Multi-mesh use is lighter than for hands or eyes but still allows stacked segments when the silhouette needs a curved or articulated column. Embedding focuses on length, thickness profile, and where the neck meets clavicle and skull sockets.

#### 3.2.8: Crozon Lower Limb Variant Multi-meshes
- Concept set: [`lower-limb`](./variants/crozon/lower-limb/README.md) (primitives: [`common`](./variants/crozon/common/README.md))

Lower limbs are specified as **shared primitives** plus **limb-only** embedding from hip to ankle (or equivalent), including taper, bow, and multi-segment options like drum prongs or pearl chains. One variant recipe can cover a full leg by composing length-scaled instances along the skeleton. Concept art may live on the common primitive or on the limb stub for pose-on-leg context.

#### 3.2.9: Crozon Upper Limb Variant Multi-meshes
- Concept set: [`upper-limb`](./variants/crozon/upper-limb/README.md) (primitives: [`common`](./variants/crozon/common/README.md))

Upper limbs follow the same primitive vocabulary as lower limbs but with **shoulder → elbow → wrist** embedding and typically **shorter** axial scales. Multi-mesh composition mirrors the lower limb (drumstick, cylinder, bow, and future promoted primitives) with arm-specific orientation defaults. The pattern keeps arm and leg pipelines aligned for tooling and species parameterization.

#### 3.2.10: Crozon Hand and Foot Variant Multi-meshes
- Concept set: [`hand-foot`](./variants/crozon/hand-foot/README.md) (primitives: [`common`](./variants/crozon/common/README.md))

Hands and feet are **multi-mesh by design**: a **palm or sole** pad, **per-digit** extrusions (often one primitive chain per digit), and optional **wrist or ankle** cuffs. Instancing the same digit spec several times still counts as one variant recipe; species rules carry count, layout, and symmetry. The docs recommend a small ordered recipe (pad, optional wedge, digits) so assembly stays repeatable.

#### 3.2.11: Crozon Torso Variant Multi-meshes
- Concept set: [`torso`](./variants/crozon/torso/README.md) (primitives: [`common`](./variants/crozon/common/README.md))

Torso variants describe the **primary trunk mass** from shoulders to pelvis using spheroids, drumsticks, tapered or rounded cuboids, and related solids. Each row links the common definition to **torso-only** notes for chest–waist taper, bulge placement, and attachment to neck and limbs. Additional primitives are added to this table when they are wired into torso assembly rules.

#### 3.2.12: Crozon Tail Variant Multi-meshes
- Concept set: [`tail`](./variants/crozon/tail/README.md) (primitives: [`common`](./variants/crozon/common/README.md))

Tails are modeled as **chains** or **tapered extrusions** along a **caudal** axis: pearl chains, bows, cylinders, or stacked segments with decreasing radius. Multi-mesh setups parent segments along a spline or spine so taper and curl read clearly in motion. The same primitive family can express both thin whips and thicker tapered tails by parameter and segment count.

### 3.3: Animations

> [!NOTE]
> The specifics of these animations are left to future RFCs; this section states **design intent** and **scope** for Malo only (curves, constraints, and engine integration).

**Malo** is the first animation pass for Crozon characters: **looping gaits**, **short locomotion bursts**, **IK-driven interaction**, and **minimal facial motion** on top of the biped and quadruped skeletons in [Section 4](#4-milestones). Clips should **retarget** or **parameterize** across species where proportions differ, rather than duplicating bespoke animation per mesh. Root motion vs in-place variants should be explicit per clip family so gameplay and cinematics can choose consistently.

#### 3.3.1: Malo Biped Walk

A **cycle** with clear **contact** and **passing** poses, **weight shift** over the supporting foot, and subtle **counter-rotation** on the pelvis and shoulders. Prefer a neutral, forward-facing stride that reads on stylized Crozon bipeds without extreme personality keys. Deliver both **in-place** and **root-motion** versions if downstream systems require both.

#### 3.3.2: Malo Biped Run

A faster **cadence** than walk, with a short **flight phase** (both feet off the ground) where species leg length allows, and slightly larger **vertical displacement** than walk for readability at speed. Keep spine pitch and arm swing in a tunable range, so the same clip family can be scaled per archetype. Document foot slip tolerance and any mandatory speed range for blending.

#### 3.3.3: Malo Biped Leap

A **short hop** or **forward leap**: **anticipation**, **takeoff**, **airborne** hold, and **landing** with recovery frames; landing should absorb impact in hips and knees without popping the root. Support **directional** variants (forward vs vertical emphasis) only if they share the same timing structure for blend trees. Root motion should match horizontal travel unless explicitly marked as in-place.

#### 3.3.4: Malo Quadruped Walk

A **quadruped walk cycle** with believable **footfall order** (e.g. walk or amble pattern—pick one canonical pattern for Malo and document it). Include **spine** and **tail** follow-through only at the level of a simple secondary motion pass; avoid species-specific quirks in the baseline clip. Same **in-place** vs **root-motion** policy as biped walk.

#### 3.3.5: Malo Quadruped Run

A **gallop- or canter-like** simplified run: clear **suspension** phases where appropriate, faster stride than walk, and stable spine orientation so the read survives low-poly silhouettes. Parameters or alternate timing slots can distinguish **large** vs **small** quadrupeds without separate art direction per species. Align foot timing with the chosen quadruped skeleton ([Section 4.15](#415-quadruped-skeleton)).

#### 3.3.6: Malo Quadruped Leap

A **forward or upward leap** from four limbs: **compression** before push, **extension**, short **airborne** pose, and **four-foot** or staggered landing as appropriate to the skeleton. Keep landing recovery short enough to chain into walk or run blends. Mark whether the clip assumes **symmetric** push or allows front/rear asymmetry as a later extension.

#### 3.3.7: Malo Biped IK Carry

**Inverse kinematics** layer (or IK-adjusted animation) so **hands** reach **grasp targets** in world or attachment space, with **torso** and **shoulder** compensation to avoid hyperextension. Include a **neutral carry** pose and transitions into/out of idle or walk; **grasp** shapes should match generic held props before hero-specific props. Coordinate with the multi-mesh / rigging story in [RFC-88](../rfc-000-088-bevy-multi-mesh/README.md) for socket naming and hand bones.

#### 3.3.8: Malo Biped Fly

**Winged biped** flight: a **wingbeat cycle** (upstroke/downstroke), optional **glide** or **soar** hold with slower secondary motion, and clear **body pitch** relative to velocity. Distinguish **hover** vs **forward flight** only if both can share blend infrastructure; otherwise ship one canonical forward-flight loop first. Feathered vs membrane wings may share the same timing with different amplitudes.

#### 3.3.9: Malo Facial Expressions

**Facial motion** for **smile**, **grimace**, **talking** (jaw and mouth corners), and **gaze / peering** (eyes or eye sockets), implemented as **blend shapes**, **bone-driven** rigs, or a documented mix. Keep the set **minimal** for Malo: enough to sell emotion and lip sync at low fidelity, without a full FACS pass. Expression clips should blend cleanly with full-body locomotion without fighting neck or spine overrides.

## 4: Milestones

> [!NOTE]
> The milestones below are not intended to cover the entire duration of the project. This is intended with minimal speculation. Species- or type-specific delivery is usually **thin wrappers** (defaults, attachment, allowed primitive mixes) on the reusable multi-mesh definitions in [Section 3.2](#32-multi-meshes) and [`variants/crozon/common/`](./variants/crozon/common/README.md).

### 4.1: Crozon Head Shape Variant Multi-meshes

Implement [3.2.1: Crozon Head Shape Variant Multi-meshes](#321-crozon-head-shape-variant-multi-meshes).

### 4.2: Crozon Ear Variant Multi-meshes

Implement [3.2.2: Crozon Ear Variant Multi-meshes](#322-crozon-ear-variant-multi-meshes).

### 4.3: Crozon Horn Variant Multi-meshes

Implement [3.2.3: Crozon Horn Variant Multi-meshes](#323-crozon-horn-variant-multi-meshes).

### 4.4: Crozon Eye Variant Multi-meshes

Implement [3.2.4: Crozon Eye Variant Multi-meshes](#324-crozon-eye-variant-multi-meshes).

### 4.5: Crozon Nose and Snout Variant Multi-meshes

Implement [3.2.5: Crozon Nose and Snout Variant Multi-meshes](#325-crozon-nose-and-snout-variant-multi-meshes).

### 4.6: Crozon Mouth Variant Multi-meshes

Implement [3.2.6: Crozon Mouth Variant Multi-meshes](#326-crozon-mouth-variant-multi-meshes).

### 4.7: Crozon Head Assembly Variant Multi-meshes

Implement composed head assemblies per [Section 3.2: Multi-meshes](#32-multi-meshes) ([3.2.1](#321-crozon-head-shape-variant-multi-meshes)–[3.2.6](#326-crozon-mouth-variant-multi-meshes)).

### 4.8: Crozon Neck Variant Multi-meshes

Implement [3.2.7: Crozon Neck Variant Multi-meshes](#327-crozon-neck-variant-multi-meshes).

### 4.9: Crozon Lower Limb Variant Multi-meshes

Implement [3.2.8: Crozon Lower Limb Variant Multi-meshes](#328-crozon-lower-limb-variant-multi-meshes).

### 4.10: Crozon Upper Limb Variant Multi-meshes

Implement [3.2.9: Crozon Upper Limb Variant Multi-meshes](#329-crozon-upper-limb-variant-multi-meshes).

### 4.11: Crozon Hand and Foot Variant Multi-meshes

Implement [3.2.10: Crozon Hand and Foot Variant Multi-meshes](#3210-crozon-hand-and-foot-variant-multi-meshes).

### 4.12: Crozon Torso Variant Multi-meshes

Implement [3.2.11: Crozon Torso Variant Multi-meshes](#3211-crozon-torso-variant-multi-meshes).

### 4.13: Crozon Tail Variant Multi-meshes

Implement [3.2.12: Crozon Tail Variant Multi-meshes](#3212-crozon-tail-variant-multi-meshes).

### 4.14: Biped Skeleton

Implement the **canonical biped skeleton** (joint hierarchy, rest pose, limits, and skin/bind conventions) used by Crozon **bipedal** species, including humanoids and **flying bipeds** from [Section 3.1.2: Crozon Species Diversity](#312-crozon-species-diversity). Naming and attachment sockets should match the rigging assumptions in [RFC-88](../rfc-000-088-bevy-multi-mesh/README.md) so assembled multi-meshes from [Section 3.2](#32-multi-meshes) snap to a single stable rig. This milestone unblocks [Section 4.18: Malo Gait Animation for Bipeds](#418-malo-gait-animation-for-bipeds), [Section 4.20: Malo Carrying and Grasping Animation for Bipeds](#420-malo-carrying-and-grasping-animation-for-bipeds), and [Section 4.21: Malo Expressions Animations](#421-malo-expressions-animations).

### 4.15: Quadruped Skeleton

Implement the **canonical quadruped skeleton** for Crozon **quadruped** routines (grazing, predators, small land animals, swimmers, and related buckets) per [Section 3.1.2: Crozon Species Diversity](#312-crozon-species-diversity), with spine and limb chains suited to the limb and tail variants in [Section 3.2](#32-multi-meshes). Keep joint layout and socket naming consistent with [RFC-88](../rfc-000-088-bevy-multi-mesh/README.md) so the same assembly pipeline as bipeds applies. This milestone unblocks [Section 4.19: Malo Gait Animation for Quadrupeds](#419-malo-gait-animation-for-quadrupeds).

### 4.16: Develop Multi-mesh API

Develop the multi-mesh API for rigging skeletons as described in [RFC-88](../rfc-000-088-bevy-multi-mesh/README.md). Treat this as the **contract** between assembled Crozon parts ([Section 4.1](#41-crozon-head-shape-variant-multi-meshes)–[4.13](#413-crozon-tail-variant-multi-meshes)) and the biped / quadruped skeletons ([Section 4.14](#414-biped-skeleton), [Section 4.15](#415-quadruped-skeleton)); it should land before or in lockstep with [Section 4.17: Crozon Species Assembly](#417-crozon-species-assembly).

### 4.17: Crozon Species Assembly

Implement **deterministic Crozon species assembly**: given a species spec in the sense of [Section 3.1.1: Crozon Species Design](#311-crozon-species-design) (feature order, allowed meshes and parameters, constraints, symmetry policy), produce a full character instance parented to the correct skeleton ([Section 4.14](#414-biped-skeleton) or [Section 4.15](#415-quadruped-skeleton)) via [Section 4.16: Develop Multi-mesh API](#416-develop-multi-mesh-api). Validate forward-checking / retry behavior against empty option sets, so generation remains reproducible from seed. This milestone integrates the per-feature mesh milestones [Section 4.1](#41-crozon-head-shape-variant-multi-meshes)–[Section 4.13](#413-crozon-tail-variant-multi-meshes) and is the default **validation target** for Malo animation milestones.

### 4.18: Malo Gait Animation for Bipeds

Implement biped **walk**, **run**, and **leap** per [3.3.1: Malo Biped Walk](#331-malo-biped-walk), [3.3.2: Malo Biped Run](#332-malo-biped-run), and [3.3.3: Malo Biped Leap](#333-malo-biped-leap), on the [biped skeleton](#414-biped-skeleton) with Crozon species assembly as the validation target. When flyer species are in scope for the same release, add **winged flight** per [3.3.8: Malo Biped Fly](#338-malo-biped-fly) in the same milestone or as an immediate follow-on, so locomotion coverage matches Crozon’s flying buckets.

### 4.19: Malo Gait Animation for Quadrupeds

Implement quadruped **walk**, **run**, and **leap** per [3.3.4: Malo Quadruped Walk](#334-malo-quadruped-walk), [3.3.5: Malo Quadruped Run](#335-malo-quadruped-run), and [3.3.6: Malo Quadruped Leap](#336-malo-quadruped-leap), on the [quadruped skeleton](#415-quadruped-skeleton) with the same retargeting and root-motion conventions as biped Malo clips.

### 4.20: Malo Carrying and Grasping Animation for Bipeds

Implement biped **carry and grasp** IK animation per [3.3.7: Malo Biped IK Carry](#337-malo-biped-ik-carry), including reach, hold, and transitions that work with generic grasp targets and the rigging assumptions in [RFC-88](../rfc-000-088-bevy-multi-mesh/README.md).

### 4.21: Malo Expressions Animations

Implement the Malo **facial expression** set per [3.3.9: Malo Facial Expressions](#339-malo-facial-expressions), blend-compatible with full-body states and scoped to the minimal smile / grimace / speech / gaze reads described there.