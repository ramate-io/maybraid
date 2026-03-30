# RFC-N: Bevy Multi-mesh

## Motivation

> [!NOTE]
> Below are some relevant references to this concept which preceded this proposal:
>
> - [#86](https://github.com/ramate-io/maybraid/issues/86)

We want to **modularly compose** mesh generation, gameplay motion, and future animation so that:

- Many meshes can belong to **one logical assembly** (`MultiMesh`) with **dynamic attachment** (`MultiMesh` on `MultiMesh`).
- **Higher-level** motion (e.g. explosion knockback on the assembly) can combine with **part-local** state (e.g. leg angle from kinematics) without a single writer clobbering another.
- We stay **low-poly / rigid** in look: **smooth skinning is not the main tool**; intersections and part boundaries carry most of the visual burden.

We **do not** drive this by writing every consumer’s final [`Transform`](https://docs.rs/bevy/latest/bevy/prelude/struct.Transform.html) directly from a central system. Instead, assemblies emit **suggested** rigid transforms; each child **reconciles** suggestions with its own state (position vs rotation vs scale policies).

Open questions from the original draft (trait vs default behavior, “do legs keep their angle?”) are addressed by **reconciliation policy on the child** and by **layered contributions** in a mailbox rather than last-write-wins on one component.

## Prior art

### In general

Industry practice almost always factors **pose** into a **tree of transforms** (a [scene graph](https://en.wikipedia.org/wiki/Scene_graph)): each node has a local matrix; world matrices multiply down branches.

**Rigid multi-mesh** (our primary target): several meshes under a common logical assembly; motion is rigid transforms, not vertex skinning.

| Pros | Cons |
| --- | --- |
| Simple; matches ECS “entity per part” | Seams at joints unless art hides them |
| Easy LOD per part | More draw calls than one merged mesh |

**Skeletal (skinned) mesh** (context only): joints + weights + GPU skinning—see [glTF `skins`](https://registry.khronos.org/glTF/specs/2.0/glTF-2.0.html#skins), [GPU Gems — Dawn animation](https://developer.nvidia.com/gpugems/gpugems/part-i-natural-effects/chapter-4-animation-dawn-demo), [Unity: Skinned Mesh Renderer](https://docs.unity3d.com/6000.0/Documentation/Manual/class-SkinnedMeshRenderer.html).

> [!NOTE]
> **Tangent — local frame vs “ray only”**  
> A useful parent anchor is an **oriented plane / orthonormal frame**: origin + two in-plane axes; the third axis is the cross product (handedness fixed in code). Children store **local coordinates** in that frame; when the parent frame rotates, children move consistently. A bare **ray + uniform scale** under-specifies **roll** in the perpendicular plane unless the child supplies a reference direction or stores a full local pose.

### Within `bevy`

**Docs and examples**: [`Transform`](https://docs.rs/bevy/latest/bevy/prelude/struct.Transform.html), [`GlobalTransform`](https://docs.rs/bevy/latest/bevy/prelude/struct.GlobalTransform.html), [`bevy_animation`](https://docs.rs/bevy/latest/bevy/animation/index.html), [transform](https://bevy.org/examples/transforms/transform/) and [animated transform](https://bevy.org/examples/animation/animated-transform/) examples, [Bevy GitHub](https://github.com/bevyengine/bevy).

**Built-in**: `ChildOf` / `Children`, transform propagation, glTF scenes + optional skinned rigs.

**Gap**: No standard **procedural `MultiMesh`** contract or **multi-writer pose suggestion** path; this RFC defines that.

## Approaches Considered

- **Parented `Transform` only**: minimal; works when a single hierarchy and clear ownership suffice. Weak when **multiple sources** suggest motion for the same part in one tick.
- **Staging resource (`Vec` / queue) → drain**: best when **many systems** must append without `&mut` conflicts on the same entity; see tangent below.
- **Spawned “command entities”** per suggestion: flexible schedule control, **higher per-message cost** than buffers or observers.
- **Trait-heavy policy on parent**: flexible but couples parent to child enumeration; nesting gets awkward.

**Chosen direction**: **suggestive transforms** carried in a per-receiver **mailbox**, with **[`EntityEvent`](https://docs.rs/bevy/latest/bevy/ecs/event/trait.EntityEvent.html)** (and optionally custom **relationships** / propagation) for routing; **observers** append into the mailbox; **ordered systems** run reconciliation and downstream propagation. Reuse the mailbox idea as a **generic pattern** for other aggregated effects in a dedicated crate.

> [!TIP]
> **Tangent — `trigger` vs `Mailbox` concurrency**  
> Bevy’s observer path uses [`Commands::trigger`](https://docs.rs/bevy/latest/bevy/prelude/struct.Commands.html) / [`World::trigger`](https://docs.rs/bevy/latest/bevy/prelude/struct.World.html): triggers run **immediately** for that call—observers execute synchronously, not via the older **`Events<T>` + `EventWriter`** double-buffer pattern. **Per-entity `Mailbox`**: parallel systems still cannot take conflicting `&mut Mailbox` on the **same** entity; use **schedule order** or a **single staging `Resource`** (e.g. `Vec<(Entity, Msg)>`) that **one** drain system empties into mailboxes, then **piped** reconcile systems.

> [!TIP]
> **Tangent — spawn-per-message vs triggers / buffers**  
> Spawning short-lived “command entities” per suggestion adds **allocator and archetype** cost. **`trigger` + observer → `Mailbox::push`** (or a **staging `Resource` queue** then drain) is usually **lighter per message**; keep spawn markers only where you want **extreme** decoupling from producer systems.

> [!TIP]
> **Tangent — when observers run**  
>
> Observers run **synchronously** when the event is **triggered** (e.g. from a system). **System piping** (`chain`, `before` / `after`) orders **which system triggers** and **which system drains/reconciles**. Explicit **observer-to-observer** ordering is **not** a first-class API today; see [bevy#14890](https://github.com/bevyengine/bevy/issues/14890). For us, **ordering inside one system** or **one aggregate observer** is sufficient.

## Proposed Design

### 1. Core types (illustrative)

Mailbox entries carry **provenance** and a **depth** hint, so receivers can merge without losing hierarchy information when multiple `MultiMesh` levels contribute in one frame.

```rust
use bevy::prelude::*;

/// Who suggested this transform (logical source entity).
pub type FromEntity = Entity;

/// Distance or generation hint along the MultiMesh graph for merge policy
/// (exact definition TBD: e.g. hops from emitter, or stable rank in an assembly).
pub type Depth = u32;

#[derive(Clone, Debug)]
pub struct TransformSuggestion {
    pub from: FromEntity,
    pub depth: Depth,
    pub transform: Transform,
}

/// Drained each frame (or phase) by a reconcile system; producers only push.
#[derive(Component, Default)]
pub struct Mailbox {
    pub pending: Vec<TransformSuggestion>,
}

impl Mailbox {
    pub fn push(&mut self, suggestion: TransformSuggestion) {
        self.pending.push(suggestion);
    }

    /// Single ordered system calls this after producers for the phase have finished.
    pub fn drain(&mut self) -> Vec<TransformSuggestion> {
        std::mem::take(&mut self.pending)
    }
}
```

> [!NOTE]
> **Tangent — epochs / eviction**  
> Optional `tick: u32` (or epoch) on each entry supports **staleness** rules on drain: drop contributions from an old propagation wave or superseded pass without keyed lookup if we **always drain**.

### 2. Why not write `Transform` directly?

[`Transform`](https://docs.rs/bevy/latest/bevy/prelude/struct.Transform.html) remains the **authoritative** pose for rendering after reconciliation. **Suggestions** are a separate channel, so children can implement policies such as: adopt translation, preserve local rotation from animation, ignore scale, etc.

### 3. `EntityEvent`, observers, and aggregation

Define a(n) **`EntityEvent`**, so triggers can be **routed to a target entity** (see [`EntityEvent`](https://docs.rs/bevy/latest/bevy/ecs/event/trait.EntityEvent.html)). The payload carries one `TransformSuggestion`; repeated triggers in the same frame **append** to the mailbox.

[`On<E>`](https://docs.rs/bevy/latest/bevy/ecs/observer/struct.On.html) derefs to `E`; observers are normal systems and may use `Query`, `Commands`, etc.

```rust
use bevy::prelude::*; // `EntityEvent`, `On`, `Event` — see Bevy 0.18 prelude / `bevy::ecs::*`

#[derive(EntityEvent, Event, Clone, Debug)]
struct MultiMeshTransformSuggested {
    entity: Entity,
    suggestion: TransformSuggestion,
}

fn plugin(app: &mut App) {
    app.add_observer(append_suggestion_to_mailbox);
}

/// Global observer: on each trigger, push into the *target* entity’s `Mailbox`.
fn append_suggestion_to_mailbox(
    mut on: On<MultiMeshTransformSuggested>,
    mut q: Query<&mut Mailbox>,
) {
    let target = on.entity;
    if let Ok(mut mailbox) = q.get_mut(target) {
        mailbox.push(on.suggestion.clone());
    }
}

/// From an ordered system (placed in a `.chain()` where you need it):
fn emit_suggestion_to(mut commands: Commands, target: Entity, suggestion: TransformSuggestion) {
    commands.trigger(MultiMeshTransformSuggested {
        entity: target,
        suggestion,
    });
}
```

Use [Observers](https://bevy.org/examples/ecs-entity-component-system/observers/) for **entity-targeted** delivery; the observer **aggregates** by pushing into `Mailbox`. **Propagation** along a custom `MultiMesh` graph can use `#[entity_event(propagate = …)]` with a **relationship** component (see examples in the [`EntityEvent` docs](https://docs.rs/bevy/latest/bevy/ecs/event/trait.EntityEvent.html)).

### 4. Schedule shape (system piping)

Typical **single-frame** pipeline (names illustrative):

1. **Producers** — gameplay, animation, parent `MultiMesh` pass: `trigger` suggestions (or append to a **staging `Resource`** if many parallel writers must avoid `&mut` clashes).
2. **Optional: drain staging queue → mailboxes** — single system if using that resource for parallelism.
3. **Reconcile** — `Query<(Entity, &mut Mailbox, /* child policy components */)>`: `drain()`, merge by `(depth, from, …)` policy, write **final** `Transform` or intermediate components.
4. **Propagate** — walk `MultiMesh` children (explicit traversal or relationship walk), emit **next** round of `MultiMeshTransformSuggested` if needed.

```rust
// Illustrative plugin ordering — exact sets TBD.
app.add_systems(
    Update,
    (
        emit_multimesh_suggestions,
        drain_staging_queue_into_mailboxes, // optional
        reconcile_mailboxes_into_transforms,
        propagate_multimesh_to_children,
    )
        .chain(),
);
```

> [!TIP]
> **Tangent — nested `MultiMesh` and overwrite**  
> A single `Transform` slot on a child would **lose** contributions when both an ancestor and an intermediate `MultiMesh` write in one tick. The **mailbox + `(FromEntity, Depth)`** (and a defined **merge order**) preserves provenance, so reconciliation can compose rather than overwrite.

### 5. Generality

`Mailbox<T>` (or a small family of mailboxes) with the same **push / drain / reconcile** pattern applies to other **multi-source** effects (forces, damage, animation hints). **Multi-mesh transforms** are the first consumer.

### 6. Physics (optional lane)

A **force** or **placement** API can coexist: impulses go through physics; **kinematic** assembly motion goes through **mailbox suggestions**. Keeping both avoids forcing all motion through the physics solver.

## References

- [`maybraid`#86 — Multi-mesh manipulation](https://github.com/ramate-io/maybraid/issues/86)
- [Bevy `EntityEvent`](https://docs.rs/bevy/latest/bevy/ecs/event/trait.EntityEvent.html)
- [Bevy observers example](https://bevy.org/examples/ecs-entity-component-system/observers/)
- [Observer ordering discussion](https://github.com/bevyengine/bevy/issues/14890)
