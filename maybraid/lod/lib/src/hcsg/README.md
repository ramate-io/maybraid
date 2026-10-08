# HCSG generation and presentation

This is the design for the shared HCSG runtime. It has two aims:

1. **One generation path and one presentation path.** Each layer writes `GenerationScheme`s and scene implementations. The shared runtime provides two generic systems:
   - `generation<B, T>` keeps HCSG storage warm for type `T` within the bounds supplied by `B`.
   - `presentation<B, T>` does the same and also marshals the published values into Bevy entities.

   The runtime owns generation requests, storage access, the lifecycle of presentation hosts, and scene reconciliation. Layers no longer carry their own presenters, producers or window plumbing.
2. **No generation inside the frame.** Generation runs on a worker thread. The frame only publishes requested bounds and reads values that have already been published.

[#1005](https://github.com/ramate-io/maybraid/issues/1005) introduced `HcsgStorage`. [#1007](https://github.com/ramate-io/maybraid/issues/1007) moves the layers onto it. This document describes where both end up. Signatures here are illustrative.

## Overview

```text
 Bevy frame                         │  Generation worker (one thread)
                                    │
 generation::<B, T>                 │
   reads bounds from B              │
   replaces its subscription ──────►│  (fire and forget)
   (later) evicts from HcsgStorage  │
                                    │
 presentation::<B, T>               │
   reads bounds from B              │
   replaces its subscription ──────►│  Demand ── Condvar wakes the worker
   reads newly published ids  ◄─────│    discover T in bounds
   spawns / retires hosts           │    generate missing values (recursively)
   (HcsgNode<T> components)         │    publish to HcsgStorage
                                    │    append ids to the subscription
 scene follow-on systems            │
   LodScene / VisualLodScene        │
   on HcsgNode<T>                   │
```

There are four parts, each with one job:

| Part | Owns |
|---|---|
| [Storage](#storage) | Completed, immutable values and their spatial index. |
| [Demand](#demand) | Subscriptions, deduplication of work, and waking the worker. |
| [Generation system](#the-generation-system) | Which values stay in HCSG for one bounds source and one type. |
| [Presentation](#presentation) | The active set of Bevy hosts for one bounds source and one type. |

Generation and presentation request work the same way. They differ in what they retire from: generation retires values from HCSG, while presentation only retires entities from Bevy.

## Contract for generated values

A stored value is a **pure function of its key** `(TypeId, Id)` and of the session roots described below. Once published, a value never changes. Everything else follows from this rule:

- Values are shared as `Arc<T>`. Mutable gameplay state belongs in ECS components, never in stored values.
- Nothing published is ever corrected, so there is no general invalidation. Values stop being requested and can later be evicted, but they are never wrong.
- A scheme reads only through its generation context. It never reads ECS resources or other mutable state.

### Intent belongs in types

Different game modes ask for different things by asking for different **types**, not by mutating shared inputs. A mode that needs a different discovery or composition wraps a shared core:

```rust
/// Shared core, unaware of the mode: cached across modes.
impl GenerationScheme for Terrain { /* ... */ }

/// Mode-specific: the same core, composed differently for this mode.
struct Discovery<T>(PhantomData<T>);
impl<T: GenerationScheme> GenerationScheme for Discovery<T> { /* delegates, then adds */ }
```

Only types whose inputs really differ are wrapped. Shared lower types, such as Durham terrain, stay unaware of the mode, so their values carry over when the mode changes. Wrapper types, associated types and blanket impls express what is actually going on: a shared core behaving slightly differently in a given context.

### Session roots

Some values are chosen at runtime, such as a seed, a map or authored placements. These cannot be type parameters, so they enter as **seeded roots**:

1. **Initialization phase.** A mode seeds its roots when its session starts, before it subscribes anything.
2. **Immutable for the session.** Roots never change while the session runs.
3. **Mode exit** clears the stores of the mode's own wrapped types and bumps the [epoch](#sessions-and-epochs). The shared core stays cached.

Changing a root means starting a new session. Development-only knobs, such as playground reseeds, do the same thing: clear the affected stores and start again.

### The presented type is total

A layer presents one type, and that type exists for every cell in its bounds. For example, urbanization presents padded terrain for every ground cell. Where no pad reaches, the padded cell just wraps the ground cell. Durham's own terrain is not presented underneath it. This removes the need for any replacement machinery, such as hiding raw terrain under padded terrain.

## Storage

`HcsgStorage` keeps one typed store per generated type behind an erased registry. The whole store is downcast, while individual values stay typed. The effective key is `(TypeId, Id)`.

```rust
#[derive(Clone, Resource)]
pub struct HcsgStorage(Arc<Registry>);

struct Registry {
    stores: RwLock<HashMap<TypeId, Arc<dyn ErasedStore>>>,
    base_scales: RwLock<HashMap<TypeId, DVec3>>,
    next_version: AtomicU64, // one counter for every store
}

struct TypedStore<T> {
    nodes: RwLock<NodeStore<Arc<T>>>, // entries, gimme index, membership revision
}

pub struct StoredEntry<T> {
    pub value: T, // Arc<T> here
    pub bounds: Aabb3d,
    pub version: Version,
}
```

`TypedStore` reuses today's `NodeStore` (entries plus the gimme index) behind one lock. `HcsgValue` (renamed from today's `HcsgNode` trait) bounds what can be stored: `Send + Sync + 'static`.

### Locking

- The registry lock is held only long enough to clone the store's `Arc`. The guard is released before the store is touched.
- A store's lock is held only for lookup and publication. It is **never** held during generation or during recursive dependency calls.
- A value and its spatial entry are published together, under one write.
- The main thread uses `try_read` only (`try_entry`, `try_overlapping`, `try_membership_revision`). A failed read returns `Err(Busy)`, which means "nothing new this frame", never "empty".
- A poisoned lock is read through. Values are immutable and published whole, so the store is still consistent.

## Generation

### Schemes

Schemes receive a context instead of `&mut HcsgStorage`:

```rust
pub trait GenerationScheme: HcsgValue + Sized {
    /// Ids that originate in `region`. May generate dependencies through `cx`.
    fn original_ids_for(cx: &mut GenerationContext, region: Aabb3d) -> Vec<OriginalId>;

    /// The value for `id` and its bounds, or `None` where nothing exists.
    fn build_with_id(cx: &mut GenerationContext, id: Id) -> Option<(Self, Aabb3d)>;
}
```

The context hands out owned references, so dependencies stay usable after storage locks are released:

```rust
impl GenerationContext {
    pub fn get<T: HcsgValue>(&self, id: Id) -> Option<Arc<T>>;
    pub fn get_or_generate<T: GenerationScheme>(&mut self, id: Id) -> Option<Arc<T>>;
    pub fn overlapping<T: HcsgValue>(&self, region: Aabb3d) -> Vec<Id>;
}
```

`get_or_generate` looks the value up, and if it is missing, builds it (recursively resolving that value's dependencies) and publishes it before returning. The context tracks which `(TypeId, Id)` pairs it is currently generating, and treats a repeat as a cycle (`None`).

The worker builds its context with a staleness check (`GenerationContext::with_stale`). Once the subscription is cancelled, the context generates nothing more and drops whatever it just built instead of publishing it.

Schemes are synchronous. Large collections stay ordinary synchronous computations on the worker. Async generation can come later without changing the presentation boundary, because a partially constructed value stays private until it is complete.

### Demand

The demand layer sits between presentation and storage. It is the only shared state besides storage:

```rust
#[derive(Clone, Resource)]
pub struct HcsgDemand(Arc<DemandInner>);

struct DemandShared {
    next_id: AtomicU64,
    state: Mutex<DemandState>,
    wake: Condvar, // work arrived, or shutdown
    idle: Condvar, // the worker finished a subscription or ran dry
}

struct DemandState {
    subscriptions: HashMap<SubscriptionId, Subscription>,
    epoch: u64,
    working: bool,
    shutdown: bool,
}

struct Subscription {
    bounds: Aabb3d,
    focus: Option<Vec3>,
    discover: Discover, // fn pointers monomorphized per T
    generate: Generate,
    published: Vec<Id>, // appended as each value becomes available
    done: bool,
    cancelled: Arc<AtomicBool>, // set on replace, unsubscribe, or epoch end
}
```

The presentation side has three operations:

```rust
impl HcsgDemand {
    /// Removes `previous` (if any) and inserts a fresh subscription, under one lock.
    /// Notifies the worker.
    pub fn subscribe<T: GenerationScheme>(
        &self,
        previous: Option<SubscriptionId>,
        bounds: Aabb3d,
        focus: Option<Vec3>,
    ) -> SubscriptionId;

    /// Ids published for `id` from `cursor` on. `Err(Busy)` if the lock is held;
    /// `Ok(None)` if the subscription no longer exists.
    pub fn try_read_published(
        &self,
        id: SubscriptionId,
        cursor: usize,
    ) -> Result<Option<Vec<Id>>, Busy>;

    pub fn unsubscribe(&self, id: SubscriptionId);
}
```

Subscription ids come from the `AtomicU64`. A presentation system holds exactly one subscription id at a time. Replacing it is a single locked operation, so subscriptions never leak.

### Worker

There is one dedicated worker thread:

```text
loop:
  lock demand; wait on Condvar until some subscription is not done
  pick the newest such subscription; copy (id, bounds, focus, fns, cancelled); unlock
  fill(storage, demand, job):
    ids = T::original_ids_for(cx, bounds), nearest to focus first
    for each id:
      if cancelled: stop
      get_or_generate::<T>(id); nothing is published once cancelled
      if the value exists: append the id to the subscription's `published`
  mark the subscription done (if it still exists)
```

- **Every** discovered id whose value exists is appended to `published`, including values published earlier by another subscription. That's how presentation learns about values it didn't cause.
- **Cancellation is cheap.** Replacing, unsubscribing or ending the epoch removes the subscription and sets its `cancelled` flag. The worker checks the flag without taking the demand lock, and stops at the next id or dependency.
- **Deduplication.** With one worker, the published-value check in `get_or_generate` is enough: nothing else generates concurrently. The context's own generating set guards recursion. A cross-worker in-progress set waits for [more workers](#later).
- **A panicking scheme** is caught and logged. Its subscription is marked done with whatever was published so far, and the worker carries on.
- **Bounds are coalesced.** Each generation or presentation system has one live subscription, holding its latest bounds. A camera sweep replaces subscriptions instead of queuing work behind them.

### Bounds sources

`B` supplies the requested region for both systems. It might follow a camera, a gameplay region or an explicit warming region:

```rust
pub trait HcsgBounds: Send + Sync + 'static {
    type Param: SystemParam;
    /// Region to fill (and, for presentation, spawn in).
    fn inner(param: &SystemParamItem<Self::Param>) -> Option<Aabb3d>;
    /// Things are retired only once they leave this region (hysteresis).
    fn outer(param: &SystemParamItem<Self::Param>) -> Option<Aabb3d>;
    fn focus(param: &SystemParamItem<Self::Param>) -> Option<Vec3> { None }
}
```

Bounds sources that should share hosts must combine their demand into one `B`.

### The generation system

`generation<B, T>` is presentation without the marshalling. It works the same way on the request side, but it has nothing to reconcile, so it is fire and forget at both ends:

```rust
pub fn generation<B: HcsgBounds, T: GenerationScheme>(
    bounds: StaticSystemParam<B::Param>,
    demand: Res<HcsgDemand>,
    mut state: Local<Generated<T>>,
) { /* ... */ }

struct Generated<T> {
    subscription: Option<SubscriptionId>,
    requested: Option<Aabb3d>,
    _t: PhantomData<fn() -> T>,
}
```

- **Request:** when `B::inner` changes, it replaces its subscription, exactly as presentation does. That is all it does on this side. It never reads `published`, keeps no cursor, holds no hosts and creates no entities.
- **Retire (later):** it evicts values of `T` from `HcsgStorage` once their bounds leave `B::outer`. Generation is the only system that retires from HCSG. Presentation never evicts stored values; it only retires its Bevy hosts.

Use it to keep values warm where nothing is presented yet, such as ahead of the camera or under a gameplay region. A type that is presented needs no separate generation system, because presentation requests its own generation.

## Presentation

### The host component

```rust
#[derive(Component)]
pub struct HcsgNode<T: HcsgValue> {
    pub id: Id,
    pub version: GenerationVersion,
    pub value: Arc<T>,
}

impl<T: HcsgValue> Clone for HcsgNode<T> {
    fn clone(&self) -> Self {
        Self { id: self.id, version: self.version, value: Arc::clone(&self.value) }
    }
}
```

`T` does not need to be a Bevy component or `Clone`, and host construction requires no `Default`. A host is built from a completed record.

### The presentation system

```rust
pub fn presentation<B: HcsgBounds, T: GenerationScheme>(
    bounds: StaticSystemParam<B::Param>,
    storage: Res<HcsgStorage>,
    demand: Res<HcsgDemand>,
    mut state: Local<Presented<T>>,
    mut commands: Commands,
) { /* ... */ }

struct Presented<T> {
    subscription: Option<SubscriptionId>,
    requested: Option<Aabb3d>,
    cursor: usize,
    hosts: HashMap<Id, Entity>,
    _t: PhantomData<fn() -> T>,
}
```

Each frame:

1. **Request.** If `B::inner` changed, replace the subscription (`subscribe` with the previous id) and reset the cursor. The presentation system never calls discovery itself.
2. **Read.** `try_read_published(subscription, cursor)`. On `Err(Busy)`, nothing changes this frame. On `Ok(None)`, the subscription is gone (for example after an epoch change), so subscribe again.
3. **Spawn.** For each new id not already in `hosts`, clone the entry's `Arc` under a short `try_read` of the store, then spawn a host carrying `HcsgNode<T>`. If the store is busy, the id is retried next frame and the cursor is not advanced past it.
4. **Retire.** Despawn any host whose entry bounds no longer intersect `B::outer`, along with its owned scene subtree. Retirement depends only on bounds, never on what discovery returned.

Notes:
- When the subscription is replaced, existing hosts stay. Ids the new subscription publishes that are already in `hosts` are skipped.
- An id that leaves the bounds while it is still being generated is a presentation-side concern. Its subscription is gone, so the worker stops, and anything it already published is simply never spawned.
- Retiring a host does not evict its stored value. Presentation retires only from Bevy; eviction from HCSG belongs to [`generation<B, T>`](#the-generation-system).
- ECS changes are issued only after the `Arc` handles have been cloned and every storage guard has been released.

### Scenes

Scene traits are implemented on the wrapper and forward to the value:

```rust
impl<T: LodScene + HcsgValue> LodScene for HcsgNode<T> {
    // Forward scene selection and construction to self.value.
}

impl<T: VisualLodScene + HcsgValue> VisualLodScene for HcsgNode<T> {
    // Forward visual selection and construction to self.value.
}
```

- `LodScene` builds the semantic ECS subtree. `VisualLodScene` builds visual representations, including packed geometry held outside archetype storage.
- Host construction inserts `HcsgNode<T>` itself. It never forwards to an inner host-construction method that inserts or clones `T`.
- Follow-on systems query `HcsgNode<T>` directly.
- Within a session values never change, so a version change only happens across sessions. Asynchronous scene work still checks the host's version before attaching its result. A change of LOD alone reuses the same value.

### Registration and ordering

```rust
app.add_plugins(PresentationPlugin::<WorldBounds, Terrain>::default());

app.register_lod_scene::<HcsgNode<Terrain>>();
app.register_visual_lod_scene::<HcsgNode<Terrain>>();

// Only where values should stay warm without being presented:
app.add_plugins(GenerationPlugin::<AheadOfCamera, Terrain>::default());
```

- Presentation needs no separately registered generation system. It requests its own generation.
- Scene capabilities are registered according to the traits `T` implements.
- Host reconciliation runs before scene follow-on systems, with deferred entity changes applied between them.

## Sessions and epochs

The demand layer holds an `epoch`:

- **Mode exit** bumps the epoch, removes the mode's subscriptions, and clears the mode's wrapped stores.
- `advance_epoch` removes every subscription and sets its `cancelled` flag, so the worker stops and never publishes a value generated under an older epoch.
- **Mode enter** runs its initialization phase (seeding roots) before its generation and presentation systems subscribe.

Nothing else is invalidated. Recording references between values (`get_or_generate` noting the `(TypeId, Id)` pairs it touched) is a later feature for eviction and garbage collection, not for correctness.

## Testing

Tests keep the real worker thread and wait for it explicitly:

```rust
impl HcsgDemand {
    /// Blocks until no subscription has outstanding work, or `timeout` passes
    /// (`false`). Needs a running worker.
    pub fn wait_idle(&self, timeout: Duration) -> bool;
}
```

A typical test seeds its roots, runs `app.update()` once so presentation subscribes, calls `wait_idle(…)`, then runs `app.update()` again to spawn hosts and asserts on them. The timeout turns a stuck worker into a failed assertion instead of a hung test.

## Layer responsibilities

A layer provides:

- its generation types: schemes, plus wrappers for mode-specific intent;
- the stores it registers, with their base scales;
- its bounds sources;
- scene implementations for the types it presents;
- gameplay systems, which query `HcsgNode<T>` or read storage without blocking.

The shared runtime provides everything else.

## What this replaces

- `gen::runtime` (`LodGeneratePlugin<T, S>`): the last step of [#1007](https://github.com/ramate-io/maybraid/issues/1007).
- `GenerateOn<P, T>`, `GenerationProducer<P>`, `GenerateQueue`, `CurrentBounds` and the per-window generation budgets.
- `Seed::<R>().invalidates::<G>().restarts::<P>()` and `clear_group`, replaced by session roots, epochs, and clearing a mode's stores on exit.
- Bespoke presenters and presenter state: Durham terrain and water, Chico forest and bump-outs, Barking, Maputo, Richmond hosts and padded terrain, and mobs.
- Replacement machinery between presented types, such as `sync_raw_terrain_replacements` and `TerrainSuperseded`.
- Synchronous generation in Bevy systems (the layer `prepare` hooks, reads through `UrbanizationModel::Read`) and the mutable authored roots used by the training ground.

## Migration

The new API lives in `lod::hcsg::shared`, alongside the frame-synchronous `lod::hcsg` API, until the last layer moves over. Values are pure functions of their keys, so while a layer is mid-move both storages may hold the same value. That costs duplicate work, never wrong results. Old and new plugins coexist, and each app switches once its layers are ready.

0. **Shared storage, context, demand and worker** (done). `HcsgStorage` with `Arc` values and `Busy`, `HcsgValue`, `GenerationScheme` and `GenerationContext`, `HcsgDemand` and `HcsgWorker`, tested in isolation.
1. **Adapter** (done). `GenerationContext` implements the legacy `SpatialIndex<T>`, so every legacy scheme generic over `S` (Durham, cells) is a `GenerationScheme` unchanged and runs on the worker. Native schemes can depend on those legacy types. Legacy `get(&self) -> Option<&T>` borrows from an append-only cache (`elsa::FrozenMap`) of the `Arc`s the context has read. Legacy `descendants` run only from legacy entry points.
2. **Generation and presentation systems.** `HcsgBounds`, `HcsgNode<T>`, `generation<B, T>`, `presentation<B, T>`, scene forwarding and the plugins, tested on fixtures.
3. **Durham.** Present terrain and water through `presentation<B, T>` next to the old presenters; seed Durham's roots into both storages; switch `durham-playground`.
4. **Richmond.** Rewrite its four schemes natively; present total padded terrain and built developments; delete the terrain replacement machinery.
5. **Chico, Barking, Maputo.** Rewrite each onto the context, deleting its bespoke index and presenter; then delete `gen::runtime`.
6. **Native Durham and cells.** Rewrite their generic schemes on the context API, dropping the `S` capability bounds and dependency clones.
7. **Modes and removal.** Build the Discovery game mode on the new runtime. Delete the adapter (and `elsa`), the producer and queue machinery, `Seed`, the old storage and the training ground, which is then rebuilt from scratch.

## Later

- Retirement on the generation side: `generation<B, T>` evicts values outside `B::outer`. Recorded references between values decide whether a dependency can go too, so nothing still reachable from a retained value is evicted.
- More than one worker. This needs an in-progress `(TypeId, Id)` set in the demand state, so two workers never build the same value.
- Async generation for large collections.
