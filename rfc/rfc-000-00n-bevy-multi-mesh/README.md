# RFC-N: Bevy Multi-mesh

## Motivation

> [!NOTE]
> Below are some relevant references to this concept which preceded this proposal:
>
> - [#86](https://github.com/ramate-io/maybraid/issues/86)

In order to modularly compose mesh generation and animation systems, we will benefit from a multi-mesh system. This system should be responsible for inserting relevant `bevy::Transforms` or a similar object, s.t., we can assign higher order transformations and have them apply consistently down the multi-mesh structure.

This will eventually need to play nicely with animation systems. For example, an animation system may angle the legs on a character a particular way according to some kinematics. This higher-order multi-mesh system may then be used to produce the effect that the whole character multi-mesh should be moved by a certain amount following an explosion. Should the legs keep their angle? How can we make it easy for the implementer to decide? Can we wrap the multi-mesh behavior behind a trait? If we do, how do we still give some baseline behavior out of the box?

## Prior art

### In general

Industry practice almost always factors **pose** into a **tree of transforms** (a [scene graph](https://en.wikipedia.org/wiki/Scene_graph)): each node has a local matrix; world matrices multiply down branches. That model is independent of engine; it is the usual way to combine “move the whole character” with “move a limb relative to the torso.”

**Rigid multi-mesh (separate draw calls, no vertex skinning)**  
Several meshes parented under a common root (vehicle + wheels, robot built from parts). Each part is rigid; motion is entirely from transform hierarchy.

| Pros | Cons |
| --- | --- |
| Simple mental model; matches ECS “entity per part” | Seams at joints unless geometry is built to hide them |
| Easy LOD per part | Many draw calls vs one merged mesh |
| No skin weights or bone data in the asset | No continuous bending (elbow is a hinge between meshes, not a smooth skin) |

**Skeletal (skinned) mesh**  
One (or few) meshes bound to a **skeleton**: animated **joint** transforms deform vertices using **skin weights** (each vertex blends several joints—see [linear blend skinning](https://developer.nvidia.com/gpugems/gpugems/part-i-natural-effects/chapter-4-animation-dawn-demo) in GPU Gems). Game docs often call the runtime piece a *skinned mesh* and refer to bones and bind poses ([Unity: Skinned Mesh Renderer](https://docs.unity3d.com/6000.0/Documentation/Manual/class-SkinnedMeshRenderer.html)).

| Pros | Cons |
| --- | --- |
| Smooth deformation at joints; industry default for characters | Heavier assets (weights, bone indices); more GPU work |
| One body mesh with many animations | Pipeline complexity (DCC export, retargeting, etc.) |

**What people mean by a “skeletal pipeline”**  
End-to-end flow: **authoring** (rig + skin weights in a DCC tool) → **export** (e.g. [glTF `skins` and `joints`](https://registry.khronos.org/glTF/specs/2.0/glTF-2.0.html#skins)—inverse bind matrices, joint hierarchy) → **runtime** (update joint transforms per frame from animation, IK, or gameplay) → **skinning** (apply weighted joint matrices to vertices in the vertex shader). *Animation* in that pipeline usually means sampling clips / blending; *kinematics* may drive the same joint transforms instead of clips.

**Example uses**  
Rigid hierarchy: doors, vehicles, mechanical props, stylized “block” characters. Skinned: humanoids, creatures, anything that must bend smoothly. Hybrid: skinned body + rigid attached props (weapon, backpack) parented to a hand joint.

### Within `bevy`

**Where this is written up**  
Bevy documents behavior primarily in **API docs** ([`Transform`](https://docs.rs/bevy/latest/bevy/prelude/struct.Transform.html), [`GlobalTransform`](https://docs.rs/bevy/latest/bevy/prelude/struct.GlobalTransform.html)), the **[`bevy_animation`](https://docs.rs/bevy/latest/bevy/animation/index.html)** crate (e.g. [`AnimationPlayer`](https://docs.rs/bevy/latest/bevy/animation/struct.AnimationPlayer.html), [`animate_targets`](https://docs.rs/bevy/latest/bevy/animation/fn.animate_targets.html)), and **examples** on [bevy.org](https://bevy.org/) (e.g. [animated transform](https://bevy.org/examples/animation/animated-transform/), [transform](https://bevy.org/examples/transforms/transform)). Engine design discussions also appear in [Bevy’s GitHub](https://github.com/bevyengine/bevy) (issues, PRs); there is no single long-form “multi-mesh” design doc comparable to a commercial manual.

**What Bevy already covers**

- **Hierarchy and propagation**: `ChildOf` / `Children`; each frame, child `Transform` composes into `GlobalTransform`. This is the built-in answer for rigid assemblies and for moving a whole subtree together.
- **Loaded scenes**: `bevy_gltf` spawns glTF scenes as entities (meshes + materials + hierarchy). Skinned assets use the same joint tree concept as the [glTF skin spec](https://registry.khronos.org/glTF/specs/2.0/glTF-2.0.html#skins).
- **Skeletal animation**: `bevy_animation` evaluates clips and drives targets (docs describe [`AnimationClip`](https://docs.rs/bevy/latest/bevy/animation/struct.AnimationClip.html), [`AnimationTargetId`](https://docs.rs/bevy/latest/bevy/animation/struct.AnimationTargetId.html), blending via [`AnimationGraph`](https://docs.rs/bevy/latest/bevy/prelude/struct.AnimationGraph.html)). That covers the **animation half** of a skeletal pipeline when data comes from assets; **authoring** and **export** remain external (Blender, etc.).

**Gaps relative to this RFC**  
Bevy does not prescribe a single **procedural** multi-mesh abstraction for code-generated assemblies: you still decide how to spawn entities and whether to parent them. Procedural generators that emit many meshes without parents do not automatically get “one logical object” semantics—that is the space this RFC addresses.

#### `bevy` API (summary)

- **Entity hierarchy** (`ChildOf` / `Children`): child `Transform` is local to its parent; `GlobalTransform` is derived. Moving an ancestor moves the whole subtree—natural composition of whole-body vs part-local motion.
- **Scene and glTF loading**: imported scenes are typically a tree of entities with meshes, materials, and transforms; skinned assets include joint hierarchies per glTF.
- **`bevy_animation`**: curves target entities (e.g. bones); [`animate_targets`](https://docs.rs/bevy/latest/bevy/animation/fn.animate_targets.html) applies evaluated animation to those targets.

#### `bevy` community

- **Physics (Rapier, Avian, …)**: compound colliders and child bodies are usually parented, so physics transforms stay consistent with rendering—useful precedent for “one object, many pieces.”
- **Examples / tutorials**: parented props and vehicles are common; they illustrate hierarchy without extra engine types.

## Approaches Considered

- **Parented entities only**: rely on Bevy’s transform propagation; gameplay and animation write `Transform` at the right nodes. Minimal custom API; conventions document who owns which depth.
- **Trait / policy object**: abstract “apply command to assembly,” so types can customize root vs part routing; more flexible, more API surface.
- **Flat meshes + sync systems**: keep siblings and copy offsets each frame—simple for legacy spawners, easier to get wrong under animation.

## Proposed Design

TBD as we narrow scope against this repo’s generators and command layer. Baseline expectation: align with **parented entities** unless a concrete need forces a heavier abstraction.
