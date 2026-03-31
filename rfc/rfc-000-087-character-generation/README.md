# RFC-87: Character Generation

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

- **Template + parameter variation**: start from a hand-authored base topology and vary scale, proportions, feature toggles, and material palettes. This is common in games because it is stable, art-directable, and easy to constrain (for a production example, see [MetaHuman](https://www.metahuman.com/)).
- **Part library assembly (modular kits)**: generate characters by composing reusable parts (head, torso, limbs, ears, horns, tails), then enforce compatibility constraints. This aligns well with our multi-mesh-first approach (see [MB-Lab](https://github.com/animate1978/MB-Lab)).
- **Rule/grammar-driven generation**: encode allowed combinations and dependencies as rules (for example, species trait implies a family of limb and skull shapes). This is often used to keep outputs coherent while preserving variety (see [Shape Grammars](https://en.wikipedia.org/wiki/Shape_grammar) and [L-systems](https://en.wikipedia.org/wiki/L-system)).
- **Morph/blend-space variation**: blend between shape keys or latent feature sliders to produce smooth families of forms. This is powerful for continuity, but can be heavier than needed for low-poly, rigid-part pipelines (see [SMPL](https://smpl.is.tue.mpg.de/)).
- **Skeleton-aware proceduralization**: generate or adjust meshes with explicit awareness of target rig constraints (joint limits, attachment sockets, gait assumptions), so downstream animation remains viable (see [Mixamo auto-rig](https://www.mixamo.com/#/) and [SIGGRAPH 2010: Example-Based Facial Rigging](https://www.hao-li.com/publications/papers/siggraph2010EBFR.pdf)).

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

### 3.1: Species Desiderata

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
> Concept art for the multi-meshes is linked by section to subdirectories under this folder. 

#### 3.2.1: Crozon Head Shape Variant Multi-meshes
- Concept set: [`crozon-head-shape-variants/README.md`](./crozon-head-shape-variants/README.md)

#### 3.2.2: Crozon Ear Variant Multi-meshes
- Concept set: [`crozon-ear-variants/README.md`](./crozon-ear-variants/README.md)

#### 3.2.3: Crozon Horn Variant Multi-meshes
- Concept set: [`crozon-horn-variants/README.md`](./crozon-horn-variants/README.md)

#### 3.2.4: Crozon Eye Variant Multi-meshes
- Concept set: [`crozon-eye-variants/README.md`](./crozon-eye-variants/README.md)

#### 3.2.5: Crozon Nose and Snout Variant Multi-meshes
- Concept set: [`crozon-nose-snout-variants/README.md`](./crozon-nose-snout-variants/README.md)

#### 3.2.6: Crozon Mouth Variant Multi-meshes
- Concept set: [`crozon-mouth-variants/README.md`](./crozon-mouth-variants/README.md)

#### 3.2.7: Crozon Head Assembly Variant Multi-meshes
- Concept set: [`crozon-head-assembly-variants/README.md`](./crozon-head-assembly-variants/README.md)

#### 3.2.8: Crozon Neck Variant Multi-meshes
- Concept set: [`crozon-neck-variants/README.md`](./crozon-neck-variants/README.md)

#### 3.2.9: Crozon Lower Limb Variant Multi-meshes
- Concept set: [`crozon-lower-limb-variants/README.md`](./crozon-lower-limb-variants/README.md)

#### 3.2.10: Crozon Upper Limb Variant Multi-meshes
- Concept set: [`crozon-upper-limb-variants/README.md`](./crozon-upper-limb-variants/README.md)

#### 3.2.11: Crozon Hand and Foot Variant Multi-meshes
- Concept set: [`crozon-hand-foot-variants/README.md`](./crozon-hand-foot-variants/README.md)

#### 3.2.12: Crozon Torso Variant Multi-meshes
- Concept set: [`crozon-torso-variants/README.md`](./crozon-torso-variants/README.md)

#### 3.2.13: Crozon Tail Variant Multi-meshes
- Concept set: [`crozon-tail-variants/README.md`](./crozon-tail-variants/README.md)

### 3.3: Animations

Initial Animations will be delivered under the title Malo. 

## 4: Milestones

> [!NOTE]
> The milestones below are not intended to cover the entire duration of the project. This is intended with minimal speculation.

### 4.1: Crozon Head Shape Variant Multi-meshes

### 4.2: Crozon Ear Variant Multi-meshes

### 4.3: Crozon Horn Variant Multi-meshes

### 4.4: Crozon Eye Variant Multi-meshes

### 4.5: Crozon Nose and Snout Variant Multi-meshes

### 4.6: Crozon Mouth Variant Multi-meshes

### 4.7: Crozon Head Assembly Variant Multi-meshes

### 4.8: Crozon Neck Variant Multi-meshes

### 4.9: Crozon Lower Limb Variant Multi-meshes

### 4.10: Crozon Upper Limb Variant Multi-meshes

### 4.11: Crozon Hand and Foot Variant Multi-meshes

### 4.12: Crozon Torso Variant Multi-meshes

### 4.13: Crozon Tail Variant Multi-meshes

### 4.14: Biped Skeleton

### 4.15: Quadruped Skeleton

### 4.16: Develop Multi-mesh API

Develop the multi-mesh API for rigging skeletons as described in [RFC-88](../rfc-000-088-bevy-multi-mesh/README.md)

### 4.17: Crozon Species Assembly

### 4.18: Malo Gait Animation for Bipeds

### 4.19: Malo Gait Animation for Quadrupeds

### 4.20: Malo Carrying and Grasping Animation for Bipeds

### 4.21: Malo Expressions Animations